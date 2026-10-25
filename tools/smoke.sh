#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
app="${1:-target/debug/rpcrad}"
mkdir -p target/smoke
if lsof -nP -iTCP:50051 -sTCP:LISTEN >/dev/null 2>&1 || lsof -nP -iTCP:50052 -sTCP:LISTEN >/dev/null 2>&1; then
  echo 'Smoke tests require free local ports 50051 and 50052.' >&2; exit 1
fi
tools/bin/rpcrad-demo > target/smoke/demo.log 2>&1 &
demo_pid=$!
tools/bin/rpcrad-demo -addr 127.0.0.1:50052 -verify-demo-signature > target/smoke/signed.log 2>&1 &
signed_pid=$!
trap 'kill "$demo_pid" "$signed_pid" 2>/dev/null || true' EXIT INT TERM
python3 - <<'PY'
import socket, time
for port in (50051,50052):
    for attempt in range(100):
        try:
            with socket.create_connection(("127.0.0.1",port),timeout=.1): break
        except OSError: time.sleep(.05)
    else: raise RuntimeError(f"Local fixture did not start: {port}")
PY
"$app" --smoke-test
"$app" --plugin-smoke-test
