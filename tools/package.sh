#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mode="${1:-debug}"
case "$mode" in debug|release) ;; *) echo 'Usage: tools/package.sh [debug|release]' >&2; exit 1;; esac
export MACOSX_DEPLOYMENT_TARGET=14.0
sh tools/icons.sh
if [ "$mode" = release ]; then cargo build --release --locked; else cargo build --locked; fi
version=$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["package"]["version"])')
if [ "$mode" = release ]; then app="dist/release/RPCRad.app"; else app="dist/RPCRad.app"; fi
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/protos"
cp "${CARGO_TARGET_DIR:-target}/$mode/rpcrad" "$app/Contents/MacOS/RPCRad"
cp tools/bin/grpcurl "$app/Contents/MacOS/grpcurl"
cp examples/protos/*.proto "$app/Contents/Resources/protos/"
cp assets/RPCRad.icns "$app/Contents/Resources/RPCRad.icns"
cp THIRD_PARTY_NOTICES.md "$app/Contents/Resources/"
python3 - "$app" "$version" <<'PY'
import plistlib, sys
from pathlib import Path
app, version = sys.argv[1:]
values = dict(CFBundleName="RPCRad", CFBundleDisplayName="RPCRad", CFBundleIdentifier="dev.rpcrad.desktop",
    CFBundleExecutable="RPCRad", CFBundleIconFile="RPCRad.icns", CFBundlePackageType="APPL",
    CFBundleShortVersionString=version, CFBundleVersion=version, NSHighResolutionCapable=True,
    LSMinimumSystemVersion="14.0", NSHumanReadableCopyright="Copyright © 2026 eyotang")
Path(app,"Contents/Info.plist").write_bytes(plistlib.dumps(values))
PY
if [ "$mode" = release ]; then python3 tools/collect-licenses.py "$app/Contents/Resources/licenses"; fi
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"
touch "$app"
printf 'Built %s (%s)\n' "$app" "$version"
