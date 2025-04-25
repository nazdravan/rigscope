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
install -m 755 packaging/linux/install-helper.sh "$stage/install-helper.sh"
install -m 644 packaging/linux/org.rigscope.dump-dmi.policy "$stage/"
cp README.md README.ro.md LICENSE-MIT LICENSE-APACHE "$stage/"
cp assets/fonts/OFL.txt "$stage/THIRD-PARTY.txt"
tar -C "$ROOT/dist/stage" -czf "$OUT/$name.tar.gz" "$name"
echo "Done: $OUT/$name.tar.gz"
