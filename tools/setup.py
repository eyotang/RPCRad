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
    # Use the same supported runtime for grpcurl and fixtures on every build host.
    env['GOTOOLCHAIN'] = 'go' + (ROOT/'.go-version').read_text(encoding='utf-8').strip()
    subprocess.run(['go', 'install', 'github.com/fullstorydev/grpcurl/cmd/grpcurl@v1.9.3'], env=env, cwd=ROOT, check=True)
    suffix = '.exe' if os.name == 'nt' else ''
    subprocess.run(['go', 'build', '-mod=readonly', '-o', str(binary/f'rpcrad-demo{suffix}'), '.'], cwd=ROOT/'tools/demo', env=env, check=True)

if __name__ == '__main__': main()
