#!/usr/bin/env bash
# AppImage for Linux x86_64: dist/release/rigscope-<version>-linux-x86_64.AppImage
# One file: make it executable and run it. appimagetool is downloaded to .cache/ on first use.
# An AppImage cannot install the root-owned helper; see install-helper.sh or use the .deb.
set -euo pipefail
. "$(dirname "$0")/../env.sh"
cd "$ROOT"

TOOL_URL=https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
tool=$ROOT/.cache/appimagetool-x86_64.AppImage
if [ ! -x "$tool" ]; then
    mkdir -p "$ROOT/.cache"
    curl -fsSL -o "$tool.part" "$TOOL_URL"
    chmod +x "$tool.part"
    mv "$tool.part" "$tool"
fi

cargo build --release --locked -p rigscope

name=$APP-$VERSION-linux-x86_64
appdir=$ROOT/dist/stage/rigscope.AppDir
rm -rf "$appdir" "$OUT/$name.AppImage"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications" \
    "$appdir/usr/share/icons/hicolor/256x256/apps" "$appdir/usr/share/doc/rigscope" "$OUT"
install -m 755 target/release/rigscope "$appdir/usr/bin/rigscope"
cp packaging/linux/rigscope.desktop "$appdir/usr/share/applications/"
cp assets/icon.png "$appdir/usr/share/icons/hicolor/256x256/apps/rigscope.png"
cp README.md LICENSE-MIT LICENSE-APACHE "$appdir/usr/share/doc/rigscope/"
cp assets/fonts/OFL.txt "$appdir/usr/share/doc/rigscope/THIRD-PARTY.txt"
# appimagetool wants the .desktop file and the icon in the AppDir root too.
ln -s usr/share/applications/rigscope.desktop "$appdir/rigscope.desktop"
ln -s usr/share/icons/hicolor/256x256/apps/rigscope.png "$appdir/rigscope.png"
ln -s rigscope.png "$appdir/.DirIcon"
ln -s usr/bin/rigscope "$appdir/AppRun"

# No FUSE (containers, CI): appimagetool unpacks itself and runs from the folder.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 VERSION=$VERSION \
    "$tool" --no-appstream "$appdir" "$OUT/$name.AppImage"
echo "Done: $OUT/$name.AppImage"
