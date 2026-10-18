"""Release boundary tests with real Minisign signatures and temporary Git repos."""

import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True).strip()

    def test_release_notes_select_previous_tag_and_append_installation_notes(self):
        self.git("init", "-b", "main")
        (self.root / "docs").mkdir()
        instructions = "## Install\nKeep your vault backup.\n"
        (self.root / "docs/release-notes.md").write_text(instructions, encoding="utf-8")
        output = self.root / "notes.md"
        changelog = "## What's Changed\n* Fix by @contributor in #1\n\nFull Changelog: compare\n"
        for tag, previous in (("v0.2.6", None), ("v0.3.0", "v0.2.6")):
            with self.subTest(tag=tag):
                self.git("-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "--allow-empty", "-m", tag)
                self.git("tag", tag)
                with patch.object(release, "ROOT", self.root), patch.object(
                    release.subprocess, "check_output", return_value=changelog
                ) as api:
                    release.write_release_notes(output, "PeeeBrain/latch", tag)
                expected = ["gh", "api", "repos/PeeeBrain/latch/releases/generate-notes", "-f", f"tag_name={tag}"]
                if previous:
                    expected.extend(["-f", f"previous_tag_name={previous}"])
                api.assert_called_once_with(expected + ["--jq", ".body"], text=True)
                self.assertEqual(output.read_text(encoding="utf-8"), changelog.rstrip() + "\n\n" + instructions)

    def test_release_notes_api_failure_does_not_write_publishable_notes(self):
        self.git("init", "-b", "main")
        self.git("-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "--allow-empty", "-m", "initial")
        self.git("tag", "v0.3.0")
        output = self.root / "notes.md"
        with patch.object(release, "ROOT", self.root), patch.object(
            release.subprocess, "check_output", side_effect=subprocess.CalledProcessError(1, "gh")
        ), self.assertRaises(subprocess.CalledProcessError):
            release.write_release_notes(output, "PeeeBrain/latch", "v0.3.0")
        self.assertFalse(output.exists())

    def test_release_requires_matching_tag_at_checkout_on_main(self):
        self.git("init", "-b", "main")
        self.git("-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "--allow-empty", "-m", "main")
        self.git("update-ref", "refs/remotes/origin/main", "HEAD")
        self.git("tag", "v0.3.0")
        previous = Path.cwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, previous)
        release.validate("v0.3.0", "0.3.0")
        with self.assertRaises(ValueError):
            release.validate("v0.2.6", "0.3.0")
        self.git("-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "--allow-empty", "-m", "unmerged")
        with self.assertRaises(ValueError):
            release.validate("v0.3.0", "0.3.0")
        self.git("tag", "-f", "v0.3.0")
        with self.assertRaises(subprocess.CalledProcessError):
            release.validate("v0.3.0", "0.3.0")

    def test_stage_rejects_missing_or_ambiguous_packages_and_missing_signatures(self):
        source = self.root / "source"
        source.mkdir()
        destination = self.root / "downloads"
        with self.assertRaises(ValueError):
            release.stage(source, destination, "windows-x86_64", "0.3.0")
        package = source / "latch-desktop_0.3.0_x64-setup.exe"
        package.write_bytes(b"installer")
        with self.assertRaises(FileNotFoundError):
            release.stage(source, destination, "windows-x86_64", "0.3.0")
        package.with_name(package.name + ".sig").write_text("signature")
        release.stage(source, destination, "windows-x86_64", "0.3.0")
        self.assertEqual((destination / "Latch_0.3.0_windows-x86_64-setup.exe").read_bytes(), b"installer")
        (source / "other-setup.exe").write_bytes(b"other")
        with self.assertRaises(ValueError):
            release.stage(source, destination, "windows-x86_64", "0.3.0")

    def test_manifest_verifies_signatures_and_serves_complete_native_targets(self):
        key = self.root / "key"
        public = self.root / "key.pub"
        subprocess.run(["minisign", "-G", "-W", "-s", str(key), "-p", str(public)], check=True, capture_output=True)
        public_key = base64.b64encode(public.read_bytes()).decode()
        assets = self.root / "assets"
        for platform in release.PLATFORMS:
            source = self.root / platform
            source.mkdir()
            for _, pattern, _ in release.FORMATS[platform.split("-")[0]]:
                package = source / pattern.replace("*", "latch")
                package.write_bytes(f"package for {platform}: {pattern}".encode())
                signature = package.with_name(package.name + ".sig")
                subprocess.run(["minisign", "-Sm", str(package), "-s", str(key), "-x", str(signature)], check=True, capture_output=True)
                signature.write_bytes(base64.b64encode(signature.read_bytes()))
            release.stage(source, assets, platform, "0.3.0")

        release.write_manifest(assets, "PeeeBrain/latch", "0.3.0", public_key)
        manifest = json.loads((assets / "native-latest.json").read_text())
        self.assertEqual(manifest["version"], "0.3.0")
        self.assertEqual(set(manifest["platforms"]), {"windows-x86_64", "macos-aarch64", "linux-x86_64"})
        self.assertEqual({platform: item["format"] for platform, item in manifest["platforms"].items()}, {
            "windows-x86_64": "nsis", "macos-aarch64": "app", "linux-x86_64": "appimage",
        })
        for item in manifest["platforms"].values():
            name = item["url"].split("/")[-1]
            self.assertEqual(item["url"], f"https://github.com/PeeeBrain/latch/releases/download/v0.3.0/{name}")
            self.assertEqual(item["signature"], (assets / (name + ".sig")).read_text())
        for line in (assets / "SHA256SUMS").read_text().splitlines():
            digest, name = line.split("  ")
            self.assertEqual(digest, hashlib.sha256((assets / name).read_bytes()).hexdigest())

        # A failed assembly must not leave a feed that publication could use.
        (assets / "native-latest.json").unlink()
        package = assets / "Latch_0.3.0_windows-x86_64-setup.exe"
        original = package.read_bytes()
        package.write_bytes(b"tampered")
        with self.assertRaises(subprocess.CalledProcessError):
            release.write_manifest(assets, "PeeeBrain/latch", "0.3.0", public_key)
        self.assertFalse((assets / "native-latest.json").exists())
        package.write_bytes(original)
        other_key = self.root / "other-key"
        other_public = self.root / "other-key.pub"
        subprocess.run(["minisign", "-G", "-W", "-s", str(other_key), "-p", str(other_public)], check=True, capture_output=True)
        with self.assertRaises(subprocess.CalledProcessError):
            release.write_manifest(assets, "PeeeBrain/latch", "0.3.0", base64.b64encode(other_public.read_bytes()).decode())
        package.unlink()
        with self.assertRaises(FileNotFoundError):
            release.write_manifest(assets, "PeeeBrain/latch", "0.3.0", public_key)

    def test_cargo_version_must_be_stable_and_match_between_crates(self):
        for crate in ("latch-core", "latch-desktop"):
            directory = self.root / "crates" / crate
            directory.mkdir(parents=True)
            (directory / "Cargo.toml").write_text('[package]\nversion = "0.3.0"\n')
        with patch.object(release, "ROOT", self.root):
            self.assertEqual(release.version(), "0.3.0")
            desktop = self.root / "crates/latch-desktop/Cargo.toml"
            desktop.write_text('[package]\nversion = "0.3.1"\n')
            with self.assertRaises(ValueError):
                release.version()
            for crate in ("latch-core", "latch-desktop"):
                (self.root / "crates" / crate / "Cargo.toml").write_text('[package]\nversion = "0.3.0-beta.1"\n')
            with self.assertRaises(ValueError):
                release.version()


if __name__ == "__main__":
    unittest.main()
