# shellcheck shell=bash
# Variables shared by the packaging scripts (source with `. packaging/env.sh`).
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP=rigscope
# The only place of the version: [workspace.package] version in Cargo.toml.
VERSION="$(sed -n '/^\[workspace\.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p;}' "$ROOT/Cargo.toml" | head -1)"
OUT="$ROOT/dist/release"
export ROOT APP VERSION OUT

# The cargo-about version packaging/licences.sh generates dist/THIRD-PARTY.txt with (also in CI).
export CARGO_ABOUT_VERSION=0.9.3

# copy_docs DEST: the licences that go into every package, into the folder DEST. The packaging
# scripts copy them only through this function (the Windows zip in release.yml copies the same
# files). Noto Sans's licence travels as OFL-NotoSans.txt.
copy_docs() {
    local dest=$1
    if [ ! -f "$ROOT/dist/THIRD-PARTY.txt" ]; then
        echo "dist/THIRD-PARTY.txt is missing: run packaging/licences.sh" >&2
        return 1
    fi
    mkdir -p "$dest"
    cp "$ROOT/LICENSE-MIT" "$ROOT/LICENSE-APACHE" "$ROOT/NOTICE" "$ROOT/dist/THIRD-PARTY.txt" "$dest/"
    cp "$ROOT/assets/fonts/OFL.txt" "$dest/OFL-NotoSans.txt"
}
