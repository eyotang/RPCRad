#!/bin/sh
set -eu
cd "$(dirname "$0")"
mkdir -p .rpcrad
if ! /usr/sbin/lsof -nP -iTCP:50051 -sTCP:LISTEN >/dev/null 2>&1; then
  nohup "$PWD/tools/bin/rpcrad-demo" -protos "$PWD/examples/protos" > .rpcrad/demo.log 2>&1 &
  echo "本地示例服务已启动：localhost:50051"
fi
open "$PWD/dist/RPCRad.app"
