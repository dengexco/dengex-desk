#!/usr/bin/env bash
# Source from the repository root. Tools remain local to this checkout.
export CARGO_HOME="$PWD/.tools/cargo"
export RUSTUP_HOME="$PWD/.tools/rustup"
export PATH="$CARGO_HOME/bin:$PWD/.tools/go/bin:$PATH"
