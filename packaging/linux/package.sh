#!/usr/bin/env bash
# Archive for Linux x86_64: dist/release/rigscope-<version>-linux-x86_64.tar.gz
set -euo pipefail
. "$(dirname "$0")/../env.sh"
cd "$ROOT"

cargo build --release --locked -p rigscope

name=$APP-$VERSION-linux-x86_64
stage=$ROOT/dist/stage/$name
rm -rf "$stage" "$OUT/$name.tar.gz"
mkdir -p "$stage" "$OUT"
install -m 755 target/release/rigscope "$stage/rigscope"
cp README.md README.ro.md "$stage/"
copy_docs "$stage"
tar -C "$ROOT/dist/stage" -czf "$OUT/$name.tar.gz" "$name"
echo "Done: $OUT/$name.tar.gz"
