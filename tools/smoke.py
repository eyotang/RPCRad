#!/usr/bin/env python3
"""Exercise packaged gRPC/signing and optionally two actual native UI frames."""
import argparse
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]

def run(app, gui, data):
    env = os.environ.copy()
    env['RPCRAD_DATA_DIR'] = str(data)
    data.mkdir(parents=True, exist_ok=True)
    # An existing file prevents tests from importing the developer's legacy workspace.
    (data/'workspace.json').write_text('{}')
    for arg in ['--smoke-test', '--plugin-smoke-test']:
        result = subprocess.run([str(app), '--language=en', arg], cwd=data, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=45)
        print(result.stdout, end='')
        if result.returncode:
            raise RuntimeError(f'{arg}: {result.returncode}\n{result.stderr}')
        if 'PASS:' not in result.stdout:
            raise RuntimeError(f'{arg}: no success marker')
    if gui:
        for code, label in [('zh-CN', '简体中文'), ('en', 'English'), (None, 'English')]:
            marker = data/'rendered.txt'
            marker.unlink(missing_ok=True)
            env['RPCRAD_UI_SMOKE_MARKER'] = str(marker)
            command = [str(app), '--ui-smoke-test']
            if code:
                command.append(f'--language={code}')
            result = subprocess.run(command, cwd=data, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=40)
            if result.returncode or not marker.exists():
                raise RuntimeError(f'Native UI failed: {result.returncode}\n{result.stdout}\n{result.stderr}')
            if marker.read_text(encoding='utf-8') != f'native frames rendered: {label}':
                raise RuntimeError('Native UI used the wrong language')
            print(f'PASS: native UI rendered {label}; switching and persistence verified')

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--app', required=True, type=Path)
    parser.add_argument('--gui', action='store_true')
    parser.add_argument('--packaged', action='store_true')
    args = parser.parse_args()
    app = args.app.resolve()
    for port in (50051, 50052):
        with socket.socket() as s:
            if s.connect_ex(('127.0.0.1', port)) == 0:
                raise RuntimeError(f'Test port {port} is already in use; nothing was stopped')
    processes = []
    with tempfile.TemporaryDirectory(prefix='rpcrad smoke 中文 ') as temporary:
        data = Path(temporary)
        try:
            fixture = ROOT/'tools/bin'/('rpcrad-demo.exe' if os.name == 'nt' else 'rpcrad-demo')
            for port in (50051, 50052):
                command = [str(fixture), '-addr', f'127.0.0.1:{port}']
                if port == 50052: command.append('-verify-demo-signature')
                processes.append(subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
                for _ in range(100):
                    if processes[-1].poll() is not None: raise RuntimeError('Test server exited')
                    try:
                        with socket.create_connection(('127.0.0.1', port), timeout=.1): break
                    except OSError: time.sleep(.05)
                else: raise RuntimeError('Test server did not start')
            if args.packaged:
                mac = app.parent.name == 'MacOS'
                source = app.parents[2] if mac else app.parent
                copied = data/'Relocated App'
                shutil.copytree(source, copied)
                app = copied/app.relative_to(source)
                resources = app.parent.parent/'Resources' if mac else app.parent/'resources'
                assert (resources/'protos/user.proto').is_file(), 'Missing bundled Proto'
                assert (app.parent/('grpcurl.exe' if os.name == 'nt' else 'grpcurl')).is_file(), 'Missing grpcurl'
            run(app, args.gui, data/'workspace')
        finally:
            for proc in processes:
                proc.terminate()
            for proc in processes:
                try: proc.wait(timeout=5)
                except subprocess.TimeoutExpired: proc.kill(); proc.wait()

if __name__ == '__main__': main()
