#!/usr/bin/env python3
"""Run Weston inside Xvfb for a virtual seat; the application uses native Wayland."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

with tempfile.TemporaryDirectory(prefix='rpcrad-wayland-') as temporary:
    runtime=Path(temporary); runtime.chmod(0o700)
    env=os.environ.copy(); env.update(XDG_RUNTIME_DIR=str(runtime), WAYLAND_DISPLAY='wayland-rpcrad')
    if not env.get('DISPLAY'):
        raise RuntimeError('Run under xvfb-run so Weston can expose a virtual input seat')
    app_env=env.copy(); app_env.pop('DISPLAY',None)
    help_text=subprocess.check_output(['weston','--help'],text=True,stderr=subprocess.STDOUT)
    renderer='--renderer=pixman' if '--renderer' in help_text else '--use-pixman'
    log=runtime/'weston.log'
    with log.open('w') as stream:
        compositor=subprocess.Popen(['weston','--backend=x11-backend.so',renderer,'--socket=wayland-rpcrad','--idle-time=0'],env=env,stdout=stream,stderr=subprocess.STDOUT)
        try:
            for _ in range(100):
                if compositor.poll() is not None: raise RuntimeError(log.read_text())
                if (runtime/'wayland-rpcrad').exists(): break
                time.sleep(.1)
            else: raise RuntimeError(log.read_text())
            subprocess.run(['dbus-run-session','--',sys.executable,'tools/smoke.py','--app','/opt/rpcrad/rpcrad','--packaged','--gui'],env=app_env,check=True)
        finally:
            compositor.terminate()
            try: compositor.wait(timeout=5)
            except subprocess.TimeoutExpired: compositor.kill(); compositor.wait()
