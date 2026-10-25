#!/usr/bin/env python3
"""Native packages and an all-platform, checksummed GitHub update manifest."""
import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
import zipfile
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = 'eyotang/RPCRad'
VERSION_PATTERN = r'(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)'
PLATFORMS = ('macos-arm64', 'macos-x64', 'windows-x64', 'linux-x64')

def version(root=ROOT):
    value = tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
    if not re.fullmatch(VERSION_PATTERN, value): raise ValueError('Release version must be stable x.y.z')
    return value

def validate_tag(tag, value):
    if tag != f'v{value}': raise ValueError('Release tag must exactly match Cargo.toml')

def native_platform():
    system, machine = platform.system(), platform.machine().lower()
    if system == 'Darwin': return 'macos-arm64' if machine == 'arm64' else 'macos-x64'
    if machine not in ('amd64', 'x86_64'): raise RuntimeError('This release supports x64 Windows/Linux')
    return {'Windows': 'windows-x64', 'Linux': 'linux-x64'}[system]

def packages(value, target):
    stem = f'RPCRad-{value}'
    return {
        'macos-arm64': {f'{stem}-macos-arm64.dmg': 'darwin-aarch64'},
        'macos-x64': {f'{stem}-macos-x64.dmg': 'darwin-x86_64'},
        'windows-x64': {f'{stem}-windows-x64-setup.exe': 'windows-x86_64', f'{stem}-windows-x64.zip': 'windows-x86_64-portable'},
        'linux-x64': {f'{stem}-linux-x64.tar.gz': 'linux-x86_64', f'rpcrad_{value}_amd64.deb': 'ubuntu-x86_64', f'rpcrad-{value}-1-x86_64.pkg.tar.zst': 'arch-x86_64'},
    }[target]

def asset_names(value, targets=PLATFORMS):
    return [item for target in targets for name in packages(value, target) for item in (name, name+'.sha256')]

def checksum(path):
    with path.open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()

def validate_assets(assets, value, targets=PLATFORMS):
    expected = set(asset_names(value, targets))
    actual = {p.name for p in assets.iterdir()} - {'latest.json'}
    if actual != expected: raise ValueError(f'Unexpected or missing release assets: {actual ^ expected}')
    for target in targets:
        for name in packages(value, target):
            path = assets/name
            if path.is_symlink() or not path.is_file() or not path.stat().st_size: raise ValueError('Invalid release asset')
            if (assets/(name+'.sha256')).read_text().strip() != f'{checksum(path)}  {name}': raise ValueError(f'SHA-256 does not match {name}')

def manifest(assets, value, notes):
    validate_assets(assets, value)
    return dict(version=value, pub_date=datetime.now(timezone.utc).isoformat(timespec='seconds').replace('+00:00','Z'), notes=notes,
        platforms={}, # Reserved for a future verified automatic installer; all current clients open the release page.
        downloads={key: dict(url=f'https://github.com/{REPOSITORY}/releases/download/v{value}/{name}', sha256=checksum(assets/name))
                   for target in PLATFORMS for name, key in packages(value, target).items()})

def run(*args, **kwargs):
    subprocess.run([str(a) for a in args], cwd=ROOT, check=True, **kwargs)

def stage_bundle(target, value, skip_build):
    if target.startswith('macos-'):
        if skip_build: raise ValueError('--skip-build is only for non-macOS staging')
        run('sh', 'tools/package.sh', 'release')
        return ROOT/'dist/release/RPCRad.app'
    if not skip_build: run('cargo', 'build', '--release', '--locked')
    bundle = ROOT/'dist'/target/'RPCRad'
    if bundle.exists(): shutil.rmtree(bundle)
    resources = bundle/'resources'
    resources.mkdir(parents=True)
    windows = target == 'windows-x64'
    suffix = '.exe' if windows else ''
    compiled = Path(os.environ.get('CARGO_TARGET_DIR', ROOT/'target'))/'release'/f'rpcrad{suffix}'
    shutil.copy2(compiled, bundle/('RPCRad.exe' if windows else 'rpcrad'))
    shutil.copy2(ROOT/'tools/bin'/f'grpcurl{suffix}', bundle/f'grpcurl{suffix}')
    shutil.copytree(ROOT/'examples/protos', resources/'protos')
    shutil.copy2(ROOT/'assets/RPCRad-256.png', resources/'RPCRad.png')
    if windows: shutil.copy2(ROOT/'assets/RPCRad.ico', resources/'RPCRad.ico')
    shutil.copy2(ROOT/'THIRD_PARTY_NOTICES.md', resources/'THIRD_PARTY_NOTICES.md')
    run(sys.executable, 'tools/collect-licenses.py', resources/'licenses')
    revision = subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    (resources/'build-info.json').write_text(json.dumps(dict(version=value, target=target, revision=revision),indent=2)+'\n')
    return bundle

def mac_package(bundle, destination, value):
    with tempfile.TemporaryDirectory(prefix='rpcrad-dmg-') as temporary:
        stage = Path(temporary)
        run('ditto', bundle, stage/'RPCRad.app')
        (stage/'Applications').symlink_to('/Applications', target_is_directory=True)
        run('hdiutil','create','-volname',f'RPCRad {value}','-srcfolder',stage,'-ov','-format','UDZO',destination)
    run('hdiutil','verify',destination)

def windows_packages(bundle, assets, value):
    names = list(packages(value,'windows-x64'))
    with zipfile.ZipFile(assets/names[1], 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(bundle.rglob('*')):
            if path.is_file(): archive.write(path, Path('RPCRad')/path.relative_to(bundle))
    compiler = shutil.which('ISCC') or str(Path(os.environ.get('ProgramFiles(x86)',r'C:\Program Files (x86)'))/'Inno Setup 6/ISCC.exe')
    script = ROOT/'target/rpcrad-installer.iss'
    script.write_text(f'''[Setup]
AppId={{{{9DBD82E8-89EB-41FB-9D1B-D51A660B87AD}}
AppName=RPCRad
AppVersion={value}
DefaultDirName={{localappdata}}\\Programs\\RPCRad
DefaultGroupName=RPCRad
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={assets}
OutputBaseFilename={Path(names[0]).stem}
SetupIconFile={ROOT/'assets/RPCRad.ico'}
UninstallDisplayIcon={{app}}\\RPCRad.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
[Files]
Source: "{bundle}\\*"; DestDir: "{{app}}"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{{group}}\\RPCRad"; Filename: "{{app}}\\RPCRad.exe"
[Run]
Filename: "{{app}}\\RPCRad.exe"; Description: "Launch RPCRad"; Flags: nowait postinstall skipifsilent
''',encoding='utf-8-sig')
    run(compiler,script)

def linux_packages(bundle, assets, value):
    names = list(packages(value,'linux-x64'))
    with tarfile.open(assets/names[0], 'w:gz') as archive: archive.add(bundle, arcname='RPCRad')
    with tempfile.TemporaryDirectory(prefix='rpcrad-linux-') as temporary:
        stage = Path(temporary)
        shutil.copytree(bundle, stage/'opt/rpcrad')
        desktop = stage/'usr/share/applications/dev.rpcrad.desktop.desktop'
        desktop.parent.mkdir(parents=True)
        desktop.write_text('[Desktop Entry]\nType=Application\nName=RPCRad\nComment=Local gRPC workbench\nExec=/opt/rpcrad/rpcrad\nIcon=rpcrad\nTerminal=false\nCategories=Development;Network;\nStartupWMClass=dev.rpcrad.desktop\n')
        icon = stage/'usr/share/icons/hicolor/256x256/apps/rpcrad.png'
        icon.parent.mkdir(parents=True); shutil.copy2(ROOT/'assets/RPCRad-256.png',icon)
        launcher = stage/'usr/bin/rpcrad'; launcher.parent.mkdir(parents=True)
        launcher.symlink_to('/opt/rpcrad/rpcrad')
        control = stage/'DEBIAN/control'; control.parent.mkdir()
        control.write_text(f'''Package: rpcrad
Version: {value}
Section: devel
Priority: optional
Architecture: amd64
Maintainer: eyotang <bensetang@126.com>
Depends: libc6 (>= 2.39), libgcc-s1, libstdc++6, libfontconfig1, libfreetype6, libxkbcommon0, libxkbcommon-x11-0, libxcb1, libx11-6, libwayland-client0, libvulkan1, libasound2t64, libssl3t64, libzstd1, xdg-utils, xdg-desktop-portal
Recommends: fonts-noto-core, fonts-noto-cjk, mesa-vulkan-drivers, xdg-desktop-portal-gtk
Description: Native local gRPC workbench with request signing plugins
''')
        run('dpkg-deb','--root-owner-group','--build',stage,assets/names[1])
        shutil.rmtree(control.parent)
        size = sum(p.stat().st_size for p in stage.rglob('*') if p.is_file() and not p.is_symlink())
        deps = ['glibc','gcc-libs','fontconfig','freetype2','libxkbcommon','libxkbcommon-x11','libxcb','libx11','wayland','vulkan-icd-loader','alsa-lib','openssl','zstd','xdg-utils','xdg-desktop-portal']
        (stage/'.PKGINFO').write_text(f'pkgname = rpcrad\npkgbase = rpcrad\npkgver = {value}-1\npkgdesc = Native local gRPC workbench\nurl = https://github.com/{REPOSITORY}\nbuilddate = {int(time.time())}\npackager = RPCRad\nsize = {size}\narch = x86_64\nlicense = custom\n'+''.join(f'depend = {dep}\n' for dep in deps)+'optdepend = noto-fonts-cjk: Chinese text\noptdepend = vulkan-driver: Vulkan graphics\noptdepend = xdg-desktop-portal-gtk: file picker\n')
        run('tar','--zstd','--numeric-owner','--owner=0','--group=0','-cf',assets/names[2],'-C',stage,'.PKGINFO','opt','usr')

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tag',required=True)
    parser.add_argument('--platform',choices=PLATFORMS)
    parser.add_argument('--manifest-only',action='store_true')
    parser.add_argument('--skip-build',action='store_true')
    args = parser.parse_args()
    value = version(); validate_tag(args.tag,value)
    assets = ROOT/'release-assets'/args.tag; assets.mkdir(parents=True,exist_ok=True)
    if args.manifest_only:
        notes=(ROOT/'docs/releases'/f'{args.tag}.md').read_text()
        (assets/'latest.json').write_text(json.dumps(manifest(assets,value,notes),ensure_ascii=False,indent=2)+'\n')
        print(f'Validated complete multi-platform release: {assets}')
        return
    target = args.platform or native_platform()
    if target != native_platform(): raise RuntimeError('Build packages on their native OS/architecture')
    bundle=stage_bundle(target,value,args.skip_build)
    if target.startswith('macos-'): mac_package(bundle,assets/next(iter(packages(value,target))),value)
    elif target=='windows-x64': windows_packages(bundle,assets,value)
    else: linux_packages(bundle,assets,value)
    for name in packages(value,target): (assets/(name+'.sha256')).write_text(f'{checksum(assets/name)}  {name}\n')
    validate_assets(assets,value,(target,))
    print(f'Validated {target} packages: {assets}')

if __name__=='__main__': main()
