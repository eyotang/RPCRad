#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
exec python3 tools/smoke.py --app "${1:-target/debug/rpcrad}"
