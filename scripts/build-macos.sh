#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .artifacts/native
swiftc -swift-version 5 -O -target "$(uname -m)-apple-macosx14.0" \
  native/platform-macos/Sources/Probe.swift \
  -framework AppKit -framework ScreenCaptureKit -framework VideoToolbox \
  -framework CoreMedia -framework CoreImage -framework ApplicationServices \
  -o .artifacts/native/dx-macos-probe
codesign --force --sign - .artifacts/native/dx-macos-probe
swiftc -swift-version 5 -O -target "$(uname -m)-apple-macosx14.0" \
  native/platform-macos/Sources/DeviceIdentity.swift \
  -framework Foundation -framework Security -framework CryptoKit \
  -o .artifacts/native/dx-device-identity
codesign --force --sign - .artifacts/native/dx-device-identity
echo 'Built ad-hoc signed macOS development helper (not notarized).'
