$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
function Run([scriptblock]$Command) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "Build command failed ($LASTEXITCODE)" }
}
Run { npm ci }
Run { npm run build }
# Public packages always start without a private or temporary endpoint.
Copy-Item apps/desktop/src-tauri/resources/pilot-server.example.json apps/desktop/src-tauri/resources/pilot-server.json -Force
$env:RUSTFLAGS = '-C target-feature=+crt-static'
Run { cargo build --locked --release -p dengex-desktop --lib --features tauri/custom-protocol }
$stage = '.artifacts/releases/windows-x64'
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force "$stage/resources" | Out-Null
Copy-Item target/release/dengex_desktop.dll $stage
Run { python scripts/build-windows-launcher.py "$stage/dengex_desktop.dll" "$stage/dengeX Remote.exe" }
Run { python scripts/check-windows-dll.py "$stage/dengeX Remote.exe" }
Copy-Item apps/desktop/src-tauri/resources/pilot-server.example.json "$stage/resources/pilot-server.json"
Copy-Item LICENSE "$stage/LICENSE.txt"
Copy-Item docs/quickstart.md "$stage/START-HERE.md"
Run { python scripts/collect-notices.py }
Copy-Item .artifacts/THIRD-PARTY-NOTICES.txt "$stage/THIRD-PARTY-NOTICES.txt"
Compress-Archive -Path "$stage/*" -DestinationPath '.artifacts/releases/dengeX-Remote-Windows-x64.zip' -Force
