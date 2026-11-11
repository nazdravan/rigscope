#!/usr/bin/env bash
# .deb for Ubuntu / Debian / Mint: dist/release/rigscope-<version>-linux-x86_64.deb
# Install with `sudo apt install ./rigscope-*.deb`.
# Needs dpkg-deb (package dpkg).
set -euo pipefail
. "$(dirname "$0")/../env.sh"
cd "$ROOT"

cargo build --release --locked -p rigscope

name=$APP-$VERSION-linux-x86_64
root=$ROOT/dist/stage/deb
rm -rf "$root" "$OUT/$name.deb"
mkdir -p "$root/DEBIAN" "$root/usr/bin" \
    "$root/usr/share/applications" "$root/usr/share/icons/hicolor/256x256/apps" \
    "$root/usr/share/doc/rigscope" "$OUT"
install -m 755 target/release/rigscope "$root/usr/bin/rigscope"
install -m 644 packaging/linux/rigscope.desktop "$root/usr/share/applications/rigscope.desktop"
install -m 644 assets/icon.png "$root/usr/share/icons/hicolor/256x256/apps/rigscope.png"
install -m 644 README.md "$root/usr/share/doc/rigscope/README.md"
copy_docs "$root/usr/share/doc/rigscope"
{
    echo "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/"
    echo "Upstream-Name: rigscope"
    echo "Source: https://github.com/nazdravan/rigscope"
    echo "Comment: The Rust crates and egui's fonts (Ubuntu Light, Hack, Noto Emoji,"
    echo " emoji-icon-font) built into usr/bin/rigscope, with the full text of each licence,"
    echo " are listed in /usr/share/doc/rigscope/THIRD-PARTY.txt."
    echo
    echo "Files: *"
    echo "Copyright: $(sed -n 's/^Copyright (c) //p' LICENSE-MIT)"
    echo "License: MIT or Apache-2.0"
    echo
    echo "Files: assets/fonts/*"
    echo "Copyright: $(sed -n '1s/^Copyright //p' assets/fonts/OFL.txt)"
    echo "License: OFL-1.1"
    echo "Comment: Noto Sans, built into usr/bin/rigscope."
    echo
    echo "License: MIT"
    sed 's/^/ /; s/^ $/ ./' LICENSE-MIT
    echo
    echo "License: Apache-2.0"
    echo " On Debian systems, the full text is in /usr/share/common-licenses/Apache-2.0."
    echo
    echo "License: OFL-1.1"
    sed 's/^/ /; s/^ $/ ./' assets/fonts/OFL.txt
} >"$root/usr/share/doc/rigscope/copyright"

# Debian writes pre-releases with ~: 0.2.0-rc.1 -> 0.2.0~rc.1.
debver=${VERSION/-/\~}
size=$(du -sk "$root/usr" | cut -f1)
cat >"$root/DEBIAN/control" <<CONTROL
Package: rigscope
Version: $debver
Architecture: amd64
Maintainer: nazdravan <nazdravan@users.noreply.github.com>
Installed-Size: $size
Depends: libc6 (>= 2.35), libgl1, libxkbcommon0, pkexec | policykit-1
Recommends: smartmontools, hwdata, libxkbcommon-x11-0, libwayland-client0
Section: utils
Priority: optional
Homepage: https://github.com/nazdravan/rigscope
Description: detailed hardware configuration viewer
 Shows every RAM slot, the CPU, disks with their SMART health, graphics,
 network, battery and sensors in one window, in English or Romanian. Reads
 /proc and /sys as a normal user; the SMBIOS table and SMART are read after
 a polkit password prompt, by a fixed script that only prints them.
CONTROL

dpkg-deb --root-owner-group -Zxz --build "$root" "$OUT/$name.deb"
echo "Done: $OUT/$name.deb"
