"""Include attribution and upstream license texts for distributed Rust and Go dependencies."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

def copy_license(source, destination):
    # Go's module cache is read-only; generated bundle files must stay replaceable.
    destination = Path(destination)
    destination.unlink(missing_ok=True)
    return shutil.copyfile(source, destination)

root = Path(__file__).resolve().parents[1]
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version=1"], cwd=root))
notices = []
for package in metadata["packages"]:
    if package["source"] is None:
        continue
    folder = Path(package["manifest_path"]).parent
    notices.append({k: package[k] for k in ["name", "version", "license", "repository"]})
    target = out / f'{package["name"]}-{package["version"]}'
    candidates = [p for p in folder.iterdir() if p.name.upper().startswith(("LICENSE", "COPYING", "NOTICE"))]
    if package.get("license_file"):
        candidates.append(folder / package["license_file"])
    for path in candidates:
        if path.is_file():
            target.mkdir(exist_ok=True)
            copy_license(path, target/path.name)
        elif path.is_dir():
            shutil.copytree(path, target/path.name, dirs_exist_ok=True, copy_function=copy_license)
(out/"rust-dependencies.json").write_text(json.dumps(notices, ensure_ascii=False, indent=2)+"\n", encoding="utf-8")
env = os.environ.copy()
env["GOPATH"] = str(root/".cache/go")
env["GOCACHE"] = str(root/".cache/go-build")
# Read the exact modules linked into grpcurl; avoid unrelated upstream test dependencies.
info = subprocess.check_output(["go", "version", "-m", str(root/"tools/bin"/("grpcurl.exe" if os.name == "nt" else "grpcurl"))], env=env).decode()
def escaped(value):
    return "".join("!" + c.lower() if c.isupper() else c for c in value)
for line in info.splitlines():
    fields = line.split()
    if len(fields) < 3 or fields[0] not in {"mod", "dep"}:
        continue
    name, version = fields[1:3]
    folder = root/".cache/go/pkg/mod"/escaped(f"{name}@{version}")
    if not folder.is_dir():
        raise RuntimeError(f"Missing dependency source for attribution: {name}@{version}; run tools/setup.sh")
    for path in folder.iterdir():
        if path.is_file() and path.name.upper().startswith(("LICENSE", "COPYING", "NOTICE")):
            target = out/"go"/name/version
            target.mkdir(parents=True, exist_ok=True)
            copy_license(path, target/path.name)
copy_license(root/"THIRD_PARTY_NOTICES.md",out/"THIRD_PARTY_NOTICES.md")
