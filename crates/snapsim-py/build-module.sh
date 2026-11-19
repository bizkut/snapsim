#!/bin/sh
set -eu
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/../.."
target="${CARGO_TARGET_DIR:-target}"
cargo build --release -p snapsim-py
case "$(uname -s)" in
  Darwin) library="$target/release/libsnapsim_py.dylib" ;;
  Linux) library="$target/release/libsnapsim_py.so" ;;
  *) echo 'Unsupported platform' >&2; exit 1 ;;
esac
cp "$library" "$target/release/snapsim_py.so"
printf '%s\n' "$(cd "$target/release" && pwd)/snapsim_py.so"
