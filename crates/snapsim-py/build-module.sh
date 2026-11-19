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
# Rename instead of overwriting in place: a running server keeps its mapped copy.
cp "$library" "$target/release/snapsim_py.so.tmp"
mv -f "$target/release/snapsim_py.so.tmp" "$target/release/snapsim_py.so"
printf '%s\n' "$(cd "$target/release" && pwd)/snapsim_py.so"
