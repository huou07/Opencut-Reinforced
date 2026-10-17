#!/usr/bin/env python3
"""Regression checks for the bounded Linux preflight cache preparation."""
import contextlib
import importlib.util
import io
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


class InterruptSafetyTests(unittest.TestCase):
    def _manifest(self, temporary: str) -> tuple[Path, bytes]:
        manifest = Path(temporary) / "rust-toolchain.toml"
        original = (
            b'[toolchain]\nchannel = "1.98.0"\n'
            b'targets = [\n    "x86_64-unknown-linux-gnu",\n'
            b'    "aarch64-apple-darwin",\n]\n'
        )
        manifest.write_bytes(original)
        return manifest, original

    def test_scoped_rewrite_keeps_a_backup_then_restores_exact_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            manifest, original = self._manifest(temporary)
            backup = Path(temporary) / "backup" / "rust-toolchain.toml.original"
            with preflight.linux_toolchain_target(
                manifest, "x86_64-unknown-linux-gnu", backup=backup
            ):
                self.assertTrue(backup.is_file())
                self.assertEqual(backup.read_bytes(), original)
                self.assertNotEqual(manifest.read_bytes(), original)
            self.assertEqual(manifest.read_bytes(), original)
            self.assertFalse(backup.exists())

    def test_a_killed_run_is_repaired_from_the_backup(self):
        with tempfile.TemporaryDirectory() as temporary:
            manifest, original = self._manifest(temporary)
            # What a SIGKILLed run leaves behind: a scoped manifest and its backup.
            manifest.write_bytes(b'[toolchain]\nchannel = "1.98.0"\ntargets = ["x86_64-unknown-linux-gnu"]\n')
            backup = Path(temporary) / "rust-toolchain.toml.original"
            backup.write_bytes(original)

            message = preflight.recover_interrupted_manifest(manifest, backup)

            self.assertIsNotNone(message)
            self.assertEqual(manifest.read_bytes(), original)
            self.assertFalse(backup.exists())

    def test_recovery_without_a_backup_changes_nothing(self):
        with tempfile.TemporaryDirectory() as temporary:
            manifest, original = self._manifest(temporary)
            backup = Path(temporary) / "absent.original"
            self.assertIsNone(preflight.recover_interrupted_manifest(manifest, backup))
            self.assertEqual(manifest.read_bytes(), original)

    def test_recovery_clears_a_stale_backup_for_an_already_restored_manifest(self):
        with tempfile.TemporaryDirectory() as temporary:
            manifest, original = self._manifest(temporary)
            backup = Path(temporary) / "rust-toolchain.toml.original"
            backup.write_bytes(original)
            self.assertIsNone(preflight.recover_interrupted_manifest(manifest, backup))
            self.assertFalse(backup.exists())
            self.assertEqual(manifest.read_bytes(), original)

    def _plan(self, temporary: str, status: str,
              required_tools: tuple[str, ...] = (),
              flutter_root: Path | None = None) -> tuple[int, str, Path]:
        app = Path(temporary) / "or-app"
        app.mkdir(exist_ok=True)
        out = io.StringIO()
        with patch.object(preflight, "APP", app), patch.object(
            preflight, "ROOT", app
        ), patch.object(preflight, "GIB", 1), patch.object(
            preflight, "working_tree_status", return_value=status
        ), patch.object(
            preflight, "git_output", return_value="0" * 40
        ), patch.object(
            preflight, "available_memory", return_value=100
        ), patch.object(
            preflight, "linux_rust_target", return_value="x86_64-unknown-linux-gnu"
        ), patch.object(
            preflight, "cache_is_writable", return_value=True
        ), patch.object(
            # Tool availability and the Flutter root are host properties like the
            # caches above. The CI hygiene runner has no Flutter SDK, so a test
            # that left them real failed there while passing on a workstation.
            preflight, "REQUIRED_TOOLS", required_tools
        ), patch.object(
            preflight, "flutter_root_from", return_value=flutter_root or app
        ), contextlib.redirect_stdout(out):
            code = preflight.main(["--plan"])
        return code, out.getvalue(), app

    def test_plan_refuses_a_dirty_tree_and_changes_nothing(self):
        with tempfile.TemporaryDirectory() as temporary:
            code, output, app = self._plan(temporary, " M scripts/example.py\n")
            self.assertEqual(code, 3)
            self.assertIn("BLOCKED", output)
            self.assertIn("working tree: DIRTY", output)
            self.assertFalse((app / "build").exists())

    def test_plan_reports_ready_on_a_clean_tree(self):
        with tempfile.TemporaryDirectory() as temporary:
            code, output, app = self._plan(temporary, "")
            self.assertEqual(code, 0)
            self.assertIn("PLAN: ready", output)
            self.assertFalse((app / "build").exists())

    def test_plan_refuses_when_a_required_tool_is_missing(self):
        with tempfile.TemporaryDirectory() as temporary:
            code, output, app = self._plan(
                temporary, "", required_tools=("definitely-not-installed-h07p",))
            self.assertEqual(code, 3)
            self.assertIn("missing required tools", output)
            self.assertFalse((app / "build").exists())


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
