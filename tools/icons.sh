#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
source_icon="assets/rpcrad-logo-v1.png"
iconset="target/app-icons/RPCRad.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$source_icon" --out "$iconset/icon_${size}x${size}.png" >/dev/null
  retina=$((size * 2))
  sips -z "$retina" "$retina" "$source_icon" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o assets/RPCRad.icns
sips -z 128 128 "$source_icon" --out assets/app-logo.png >/dev/null
