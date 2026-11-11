#!/usr/bin/env bash
# Universal (arm64 + x86_64) macOS disk image: dist/release/rigscope-<version>-macos-universal.dmg
# On macOS the app reads memory, processor, board, system, disks, graphics and network
# (system_profiler, sysctl); battery and sensors are not read yet. The bundle is ad-hoc signed (not notarized): open it with right click -> Open.
set -euo pipefail
. "$(dirname "$0")/../env.sh"
cd "$ROOT"

for t in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --locked -p rigscope --target "$t"
done

name=$APP-$VERSION-macos-universal
app=$ROOT/dist/stage/rigscope.app
dmgdir=$ROOT/dist/stage/dmg
rm -rf "$app" "$dmgdir" "$OUT/$name.dmg"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$dmgdir" "$OUT"
lipo -create -output "$app/Contents/MacOS/rigscope" \
    target/aarch64-apple-darwin/release/rigscope target/x86_64-apple-darwin/release/rigscope
sed "s/@VERSION@/${VERSION%%-*}/g" packaging/macos/Info.plist >"$app/Contents/Info.plist"
cp assets/rigscope.icns "$app/Contents/Resources/rigscope.icns"
copy_docs "$app/Contents/Resources"
codesign --force --deep --sign - "$app"

cp -R "$app" "$dmgdir/"
ln -s /Applications "$dmgdir/Applications"
hdiutil create -volname "rigscope $VERSION" -srcfolder "$dmgdir" -ov -format UDZO "$OUT/$name.dmg"
echo "Done: $OUT/$name.dmg"
