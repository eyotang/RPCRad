#!/usr/bin/env python3
"""Build the bundled gRPC tools on the current OS."""
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]

def main():
    binary = ROOT / 'tools/bin'
    binary.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.update(GOBIN=str(binary), GOPATH=str(ROOT/'.cache/go'), GOCACHE=str(ROOT/'.cache/go-build'))
    # Keep the supported Go 1.26 line: grpcurl 1.9.3 passes nil to json.NewDecoder,
    # which panics with Go 1.27's default JSON v2 implementation.
    env['GOTOOLCHAIN'] = 'go' + (ROOT/'.go-version').read_text(encoding='utf-8').strip()
    subprocess.run(['go', 'install', 'github.com/fullstorydev/grpcurl/cmd/grpcurl@v1.9.3'], env=env, cwd=ROOT, check=True)
    suffix = '.exe' if os.name == 'nt' else ''
    subprocess.run(['go', 'build', '-mod=readonly', '-o', str(binary/f'rpcrad-demo{suffix}'), '.'], cwd=ROOT/'tools/demo', env=env, check=True)

if __name__ == '__main__': main()
