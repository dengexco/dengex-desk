#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Set PATH to your toolchain, or source scripts/env.sh for workspace-local tools.
bash scripts/build-macos.sh
mode=release
cargo_profile=release
tauri_args=(build --bundles app)
if [[ "${1:-}" == --debug ]]; then
  mode=debug
  cargo_profile=dev
  tauri_args+=(--debug)
fi
cargo build --locked --profile "$cargo_profile" -p dengex-transport
mkdir -p apps/desktop/src-tauri/resources .artifacts/releases
cp "target/$mode/dx-probe" apps/desktop/src-tauri/resources/dx-probe
cp .artifacts/native/dx-macos-probe apps/desktop/src-tauri/resources/dx-macos-probe
cp .artifacts/native/dx-device-identity apps/desktop/src-tauri/resources/dx-device-identity
npm ci
npm run desktop -- "${tauri_args[@]}"
bundle="target/$mode/bundle/macos/dengeX Remote Lab.app"
codesign --force --sign - --timestamp=none "$bundle/Contents/Resources/resources/dx-probe"
codesign --force --sign - --timestamp=none "$bundle/Contents/Resources/resources/dx-macos-probe"
codesign --force --sign - --timestamp=none "$bundle/Contents/Resources/resources/dx-device-identity"
codesign --force --sign - --timestamp=none --options runtime "$bundle"
codesign --verify --deep --strict "$bundle"
archive=".artifacts/releases/dengeX-Remote-Lab-macOS-$(uname -m).zip"
ditto -c -k --sequesterRsrc --keepParent "$bundle" "$archive"
shasum -a 256 "$archive" > .artifacts/releases/SHA256SUMS.txt
printf '%s\n' 'Development application only. Ad-hoc signature, no Developer ID / notarization.'
