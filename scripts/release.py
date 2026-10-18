"""Validate tagged releases and prepare signed Cargo Packager downloads."""

import argparse
import base64
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parent.parent
PLATFORMS = ("windows-x86_64", "macos-aarch64", "macos-x86_64", "linux-x86_64")
# The first format is the one supported by cargo-packager-updater on that OS.
FORMATS = {
    "windows": (("nsis", "*-setup.exe", "-setup.exe"),),
    "macos": (("app", "*.app.tar.gz", ".app.tar.gz"), ("dmg", "*.dmg", ".dmg")),
    "linux": (("appimage", "*.AppImage", ".AppImage"), ("deb", "*.deb", ".deb")),
}


def version():
    versions = {
        tomllib.loads((ROOT / "crates" / crate / "Cargo.toml").read_text())["package"]["version"]
        for crate in ("latch-core", "latch-desktop")
    }
    if len(versions) != 1:
        raise ValueError("Core and desktop Cargo versions must match")
    value = versions.pop()
    if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", value):
        raise ValueError("Stable releases require a major.minor.patch Cargo version")
    return value


def validate(tag, release_version):
    if tag != f"v{release_version}":
        raise ValueError(f"Expected tag v{release_version}, got {tag}")
    commit = subprocess.check_output(
        ["git", "rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}"], text=True
    ).strip()
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    if commit != head:
        raise ValueError("Checkout must be the tagged commit")
    subprocess.run(["git", "merge-base", "--is-ancestor", commit, "origin/main"], check=True)


def asset_name(release_version, platform, suffix):
    return f"Latch_{release_version}_{platform}{suffix}"


def stage(source, destination, platform, release_version):
    destination.mkdir(parents=True, exist_ok=True)
    for _, pattern, suffix in FORMATS[platform.split("-")[0]]:
        # Only final outputs, never Packager's intermediate copies.
        matches = list(source.glob(pattern))
        if len(matches) != 1:
            raise ValueError(f"Expected one {pattern} in {source}, found {len(matches)}")
        package = matches[0]
        signature = package.with_name(package.name + ".sig")
        if not package.stat().st_size or not signature.read_text().strip():
            raise ValueError(f"Empty package or signature: {package}")
        name = asset_name(release_version, platform, suffix)
        shutil.copyfile(package, destination / name)
        shutil.copyfile(signature, destination / (name + ".sig"))


def write_manifest(assets, repository, release_version, public_key):
    if not re.fullmatch(r"[\w.-]+/[\w.-]+", repository):
        raise ValueError("Expected a GitHub owner/repository")
    platforms = {}
    checksums = []
    with tempfile.TemporaryDirectory() as directory:
        raw_key = Path(directory) / "updater.pub"
        raw_key.write_bytes(base64.b64decode(public_key.strip(), validate=True))
        for platform in PLATFORMS:
            for index, (format_name, _, suffix) in enumerate(FORMATS[platform.split("-")[0]]):
                name = asset_name(release_version, platform, suffix)
                package = assets / name
                signature = (assets / (name + ".sig")).read_text().strip()
                if not package.stat().st_size:
                    raise ValueError(f"Empty package: {name}")
                raw_signature = Path(directory) / "package.sig"
                raw_signature.write_bytes(base64.b64decode(signature, validate=True))
                # Verify every download against the exact key embedded in the app.
                subprocess.run(
                    ["minisign", "-Vm", str(package), "-p", str(raw_key), "-x", str(raw_signature)],
                    check=True,
                )
                with package.open("rb") as payload:
                    digest = hashlib.file_digest(payload, "sha256").hexdigest()
                checksums.append(f"{digest}  {name}\n")
                if index == 0:
                    platforms[platform] = {
                        "url": f"https://github.com/{repository}/releases/download/v{release_version}/{name}",
                        "signature": signature,
                        "format": format_name,
                    }
    manifest = {
        "version": release_version,
        "notes": "See the GitHub release for installation and migration instructions.",
        "pub_date": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "platforms": platforms,
    }
    (assets / "native-latest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (assets / "SHA256SUMS").write_text("".join(checksums))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    validation = commands.add_parser("validate")
    validation.add_argument("tag")
    staging = commands.add_parser("stage")
    staging.add_argument("source", type=Path)
    staging.add_argument("destination", type=Path)
    staging.add_argument("--platform", choices=PLATFORMS, required=True)
    manifest = commands.add_parser("manifest")
    manifest.add_argument("assets", type=Path)
    manifest.add_argument("--repository", required=True)
    args = parser.parse_args()
    release_version = version()
    if args.command == "validate":
        validate(args.tag, release_version)
    elif args.command == "stage":
        stage(args.source, args.destination, args.platform, release_version)
    else:
        write_manifest(
            args.assets, args.repository, release_version,
            (ROOT / "crates/latch-desktop/updater.pub").read_text(),
        )


if __name__ == "__main__":
    main()
