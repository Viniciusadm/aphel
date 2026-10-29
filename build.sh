#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "$0")" && pwd)"

cargo run -q --manifest-path "$root_dir/back/Cargo.toml" -p xtask -- modules sync
npm --prefix "$root_dir/web" ci
npm --prefix "$root_dir/web" run build
cargo build --release --manifest-path "$root_dir/back/Cargo.toml" -p aphel
