#!/usr/bin/env python3
"""Build and validate Metra-style macOS release assets and latest.json."""
import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
import tempfile
import tomllib
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = "eyotang/RPCRad"
VERSION_PATTERN = r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"

def version(root=ROOT):
    value = tomllib.loads((root/"Cargo.toml").read_text())["package"]["version"]
    if not re.fullmatch(VERSION_PATTERN, value):
        raise ValueError("Release version must be stable x.y.z")
    return value

def validate_tag(tag, value):
    if tag != f"v{value}":
        raise ValueError("Release tag must exactly match Cargo.toml")

def asset_names(value):
    dmg = f"RPCRad-{value}-macos-arm64.dmg"
    return [dmg, dmg+".sha256"]

def checksum(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def manifest(assets, value, notes):
    names = asset_names(value)
    actual = {p.name for p in assets.iterdir()}
    if actual - {"latest.json"} != set(names):
        raise ValueError("Unexpected or missing release assets")
    for name in names:
        if not (assets/name).is_file() or not (assets/name).stat().st_size:
            raise ValueError("Empty release asset")
    digest = checksum(assets/names[0])
    if (assets/names[1]).read_text().strip() != f"{digest}  {names[0]}":
        raise ValueError("SHA-256 does not match the package")
    return {
        "version": value,
        "pub_date": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
        "notes": notes,
        # Like Metra, macOS uses the release page; platforms is reserved for signed installers.
        "platforms": {},
        "downloads": {"darwin-aarch64": {
            "url": f"https://github.com/{REPOSITORY}/releases/download/v{value}/{names[0]}",
            "sha256": digest,
        }},
    }

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--manifest-only", action="store_true")
    args = parser.parse_args()
    value = version(); validate_tag(args.tag,value)
    assets = ROOT/"release-assets"/args.tag
    assets.mkdir(parents=True, exist_ok=True)
    if not args.manifest_only:
        if platform.system() != "Darwin" or platform.machine() != "arm64":
            raise RuntimeError("This release target requires an Apple Silicon Mac")
        subprocess.run(["sh", "tools/package.sh", "release"], cwd=ROOT, check=True)
        dmg = assets/asset_names(value)[0]
        with tempfile.TemporaryDirectory(prefix="rpcrad-dmg-") as temporary:
            stage = Path(temporary)
            subprocess.run(["ditto", str(ROOT/"dist/release/RPCRad.app"), str(stage/"RPCRad.app")], check=True)
            (stage/"Applications").symlink_to("/Applications", target_is_directory=True)
            subprocess.run(["hdiutil", "create", "-volname", f"RPCRad {value}", "-srcfolder", str(stage), "-ov", "-format", "UDZO", str(dmg)], check=True)
        subprocess.run(["hdiutil", "verify", str(dmg)], check=True)
        (assets/(dmg.name+".sha256")).write_text(f"{checksum(dmg)}  {dmg.name}\n")
    notes = (ROOT/"docs/releases"/f"{args.tag}.md").read_text()
    result = manifest(assets,value,notes)
    (assets/"latest.json").write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n")
    print(f"Validated release assets: {assets}")

if __name__ == "__main__":
    main()
