#!/bin/bash
export PATH="$HOME/.cargo/bin:$PATH"

cargo fmt -- --check || exit 1
cargo clippy -- -D warnings || exit 1
