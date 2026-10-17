#!/usr/bin/env python3
"""Regression checks for the bounded Linux preflight cache preparation."""
import importlib.util
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("run_linux_ui_preflight.py")
spec = importlib.util.spec_from_file_location("linux_ui_preflight", SCRIPT)
preflight = importlib.util.module_from_spec(spec)
spec.loader.exec_module(preflight)


class CargoCachePreparationTests(unittest.TestCase):
    def test_cache_copy_skips_paths_the_worker_cannot_read(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed = root / "owner-cargo"
            source = seed / "registry" / "src"
            source.mkdir(parents=True)
            (source / "readable.dat").write_text("public cached source")
            blocked = source / "blocked.dat"
            blocked.write_text("unavailable cached entry")
            app = root / "or-app"
            app.mkdir()
            environment = {"CARGO_HOME": str(seed)}
            real_access = os.access

            def worker_access(path, mode):
                candidate = Path(path)
                if candidate == seed and mode == os.W_OK:
                    return False
                if candidate == blocked and mode & os.R_OK:
                    return False
                return real_access(path, mode)

            with patch.object(preflight, "APP", app), patch.object(
                preflight.os, "access", side_effect=worker_access
            ):
                preflight.writable_cargo_cache(environment)

            copied = app / ".dart_tool" / "aeh-cargo"
            self.assertEqual(
                (copied / "registry" / "src" / "readable.dat").read_text(),
                "public cached source",
            )
            self.assertFalse((copied / "registry" / "src" / "blocked.dat").exists())
            self.assertEqual(environment["CARGO_HOME"], str(copied))
            self.assertEqual(environment["CARGO_NET_OFFLINE"], "true")


    def test_pub_cache_copy_skips_unreadable_public_packages(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            seed = root / "owner-pub-cache"
            source = seed / "hosted" / "pub.dev"
            source.mkdir(parents=True)
            readable = source / "readable-package"
            readable.mkdir()
            (readable / "package.dart").write_text("public cached package")
            blocked = source / "blocked-package"
            blocked.mkdir()
            (blocked / "package.dart").write_text("unavailable cached package")
            app = root / "or-app"
            (app / ".dart_tool").mkdir(parents=True)
            environment = {"HOME": str(root), "PUB_CACHE": str(seed)}
            real_access = os.access

            def worker_access(path, mode):
                candidate = Path(path)
                if candidate == source and mode == os.W_OK:
                    return False
                if candidate == blocked and mode & os.R_OK:
                    return False
                return real_access(path, mode)

            with patch.object(preflight, "APP", app), patch.object(
                preflight.os, "access", side_effect=worker_access
            ):
                preflight.writable_pub_cache(environment)

            copied = app / ".dart_tool" / "aeh-pub-cache"
            self.assertEqual(
                (copied / "hosted" / "pub.dev" / "readable-package" / "package.dart").read_text(),
                "public cached package",
            )
            self.assertFalse((copied / "hosted" / "pub.dev" / "blocked-package").exists())
            self.assertEqual(environment["PUB_CACHE"], str(copied))


class CacheWriteProbeTests(unittest.TestCase):
    def test_probe_rejects_a_cache_the_sandbox_denies_despite_access(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            with patch.object(preflight.os, "access", return_value=True), patch.object(
                preflight.tempfile,
                "NamedTemporaryFile",
                side_effect=PermissionError(13, "sandbox denied"),
            ):
                self.assertFalse(preflight.cache_is_writable(directory))

    def test_probe_survives_a_directory_that_denies_access(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            with patch.object(preflight.os, "access", return_value=False):
                self.assertFalse(preflight.cache_is_writable(directory))
            self.assertFalse(any(directory.iterdir()))


class FlutterOverlayTests(unittest.TestCase):
    def _synthetic_sdk(self, root: Path) -> Path:
        sdk = root / "flutter"
        (sdk / "bin" / "internal").mkdir(parents=True)
        (sdk / "bin" / "internal" / "shared.sh").write_text("# shared\n")
        (sdk / "bin" / "flutter").write_text("#!/usr/bin/env bash\n")
        (sdk / "bin" / "flutter").chmod(0o755)
        (sdk / "bin" / "cache" / "artifacts" / "engine" / "linux-x64").mkdir(parents=True)
        (sdk / "bin" / "cache" / "artifacts" / "engine" / "linux-x64" / "flutter_tester").write_text("engine")
        (sdk / "bin" / "cache" / "engine.stamp").write_text("shared-stamp\n")
        (sdk / "packages").mkdir()
        (sdk / "version").write_text("3.47.7\n")
        return sdk

    def _read_only_access(self, sdk: Path):
        real_access = os.access

        def worker_access(path, mode):
            if Path(path) == sdk / "bin" / "cache" and mode == os.W_OK:
                return False
            return real_access(path, mode)

        return worker_access

    def test_overlay_copies_small_files_and_links_read_only_artifacts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sdk = self._synthetic_sdk(root)
            target = root / "overlay"
            preflight.materialize_flutter_overlay(sdk, target)

            self.assertTrue((target / "bin" / "flutter").is_file())
            self.assertFalse((target / "bin" / "flutter").is_symlink())
            self.assertTrue((target / "bin" / "cache" / "engine.stamp").is_file())
            self.assertFalse((target / "bin" / "cache" / "engine.stamp").is_symlink())
            self.assertTrue((target / "bin" / "cache" / "artifacts").is_symlink())
            self.assertEqual(
                (target / "bin" / "cache" / "artifacts").resolve(),
                sdk / "bin" / "cache" / "artifacts",
            )
            self.assertTrue((target / "packages").is_symlink())
            self.assertEqual((target / "version").read_text(), "3.47.7\n")

    def test_read_only_shared_cache_is_mirrored_and_reused(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sdk = self._synthetic_sdk(root)
            app = root / "or-app"
            app.mkdir()
            environment = {"PATH": str(sdk / "bin")}

            with patch.object(preflight, "APP", app), patch.object(
                preflight.os, "access", side_effect=self._read_only_access(sdk)
            ):
                preflight.writable_flutter_root(environment)
                overlay = app / ".dart_tool" / preflight.FLUTTER_OVERLAY_NAME
                self.assertTrue((overlay / "bin" / "flutter").is_file())
                self.assertTrue((overlay / "bin" / "cache" / "engine.stamp").is_file())
                self.assertTrue(environment["PATH"].startswith(f"{overlay / 'bin'}{os.pathsep}"))
                first = overlay.stat().st_ino
                preflight.writable_flutter_root(environment)
                self.assertEqual(overlay.stat().st_ino, first)

            # The shared SDK is never written: its stamp still holds the original bytes.
            self.assertEqual((sdk / "bin" / "cache" / "engine.stamp").read_text(), "shared-stamp\n")

    def test_writable_shared_cache_is_used_directly(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sdk = self._synthetic_sdk(root)
            app = root / "or-app"
            app.mkdir()
            environment = {"PATH": str(sdk / "bin")}

            with patch.object(preflight, "APP", app):
                preflight.writable_flutter_root(environment)

            self.assertEqual(environment["PATH"], str(sdk / "bin"))
            self.assertFalse((app / ".dart_tool").exists())

    def test_overlay_owned_by_another_worker_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sdk = self._synthetic_sdk(root)
            app = root / "or-app"
            overlay = app / ".dart_tool" / preflight.FLUTTER_OVERLAY_NAME
            overlay.mkdir(parents=True)
            environment = {"PATH": str(sdk / "bin")}

            with patch.object(preflight, "APP", app), patch.object(
                preflight.os, "access", side_effect=self._read_only_access(sdk)
            ), patch.object(preflight.os, "getuid", return_value=os.getuid() + 1):
                with self.assertRaises(RuntimeError):
                    preflight.writable_flutter_root(environment)


class ToolchainScopeTests(unittest.TestCase):
    def test_linux_target_is_temporary_and_restores_exact_manifest(self):
        with tempfile.TemporaryDirectory() as temporary:
            manifest = Path(temporary) / "rust-toolchain.toml"
            original = (
                '[toolchain]\nchannel = "1.98.0"\n'
                'targets = [\n    "x86_64-unknown-linux-gnu",\n'
                '    "aarch64-apple-darwin",\n]\n'
            )
            manifest.write_text(original)
            with preflight.linux_toolchain_target(manifest, "x86_64-unknown-linux-gnu"):
                targets = preflight.tomllib.loads(manifest.read_text())["toolchain"]["targets"]
                self.assertEqual(targets, ["x86_64-unknown-linux-gnu"])
            self.assertEqual(manifest.read_text(), original)

    def test_missing_linux_target_is_rejected_without_changing_manifest(self):
        with tempfile.TemporaryDirectory() as temporary:
            manifest = Path(temporary) / "rust-toolchain.toml"
            original = '[toolchain]\nchannel = "1.98.0"\ntargets = ["aarch64-apple-darwin"]\n'
            manifest.write_text(original)
            with self.assertRaises(RuntimeError):
                with preflight.linux_toolchain_target(manifest, "x86_64-unknown-linux-gnu"):
                    pass
            self.assertEqual(manifest.read_text(), original)


if __name__ == "__main__":
    unittest.main()
