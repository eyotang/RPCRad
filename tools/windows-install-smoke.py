"""Install the real Windows setup package in a disposable directory and exercise it."""
from pathlib import Path
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[1]
installers = list((root/'release-assets').glob('v*/*-setup.exe'))
if len(installers) != 1:
    raise RuntimeError('Expected exactly one Windows installer')

with tempfile.TemporaryDirectory(prefix='RPCRad setup 中文 ') as temporary:
    destination = Path(temporary)/'Installed App'
    subprocess.run([str(installers[0]), '/VERYSILENT', '/SUPPRESSMSGBOXES',
                    '/NORESTART', '/SP-', f'/DIR={destination}'], check=True, timeout=120)
    uninstaller = destination/'unins000.exe'
    try:
        if not uninstaller.is_file():
            raise RuntimeError('Installer did not create its uninstall program')
        subprocess.run([sys.executable, str(root/'tools/smoke.py'), '--app',
                        str(destination/'RPCRad.exe'), '--packaged', '--gui'], check=True)
    finally:
        if uninstaller.is_file():
            subprocess.run([str(uninstaller), '/VERYSILENT', '/SUPPRESSMSGBOXES',
                            '/NORESTART'], check=True, timeout=120)
    print('PASS: Windows installer, installed application, and uninstaller')
