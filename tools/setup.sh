#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p tools/bin
export GOBIN="$PWD/tools/bin"
export GOPATH="$PWD/.cache/go"
export GOCACHE="$PWD/.cache/go-build"
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@v1.9.3
cd tools/demo
go build -mod=mod -o ../bin/rpcrad-demo .
