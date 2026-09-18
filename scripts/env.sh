#!/usr/bin/env bash
# Source from the repository root. Tools remain local to this checkout.
export CARGO_HOME="$PWD/.tools/cargo"
export RUSTUP_HOME="$PWD/.tools/rustup"
export PATH="$CARGO_HOME/bin:$PWD/.tools/go/bin:$PATH"
# The installed workspace stable toolchain is 1.98.1. Avoid a duplicate copy
# of the exact same compiler under a second rustup toolchain name.
export RUSTUP_TOOLCHAIN=stable
