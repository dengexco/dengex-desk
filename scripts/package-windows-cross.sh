#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/windows-build-env.sh
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C target-feature=+crt-static"
npm run build
cargo xwin build --locked --target x86_64-pc-windows-msvc -p dengex-desktop --features tauri/custom-protocol
stage=.artifacts/releases/windows-x64
mkdir -p "$stage/resources"
cp target/x86_64-pc-windows-msvc/debug/dengex-desktop.exe "$stage/dengeX Remote.exe"
cp apps/desktop/src-tauri/resources/pilot-server.json "$stage/resources/pilot-server.json"
cp docs/windows-pilot.tr.txt "$stage/BENI-OKU.txt"
python3 - <<'PY'
from pathlib import Path
import zipfile,hashlib
root=Path('.artifacts/releases'); stage=root/'windows-x64'; archive=root/'dengeX-Remote-Windows-x64.zip'
with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED) as z:
 for p in stage.rglob('*'):
  if p.is_file():z.write(p,'dengeX Remote/'+str(p.relative_to(stage)))
(root/'Windows-SHA256.txt').write_text(hashlib.sha256(archive.read_bytes()).hexdigest()+'  '+archive.name+'\n')
print(archive)
PY
