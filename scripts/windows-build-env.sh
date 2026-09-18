#!/usr/bin/env bash
# Source from repository root. Workspace-local cross tools; no global changes.
source scripts/env.sh
export PATH="$PWD/.tools/windows-bin:$PWD/.tools/llvm/llvm/23.1.1/bin:$PWD/.tools/xwin/bin:$PATH"
export DYLD_LIBRARY_PATH="$PWD/.tools/rustup/toolchains/stable-aarch64-apple-darwin/lib"
export XWIN_CACHE_DIR="$PWD/.tools/xwin-cache"
export XWIN_ARCH=x86_64
export XWIN_ACCEPT_LICENSE=1
