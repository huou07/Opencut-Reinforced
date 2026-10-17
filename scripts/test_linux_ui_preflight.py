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
