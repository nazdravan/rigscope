#!/usr/bin/env bash
# dist/THIRD-PARTY.txt: the licences of every crate built into rigscope, with the text of each,
# egui's fonts included. Every packaging script needs it (copy_docs in env.sh).
# Needs cargo-about: cargo install cargo-about --locked --features cli --version <CARGO_ABOUT_VERSION from env.sh>
set -euo pipefail
# shellcheck source=packaging/env.sh
. "$(dirname "$0")/env.sh"
cd "$ROOT"

if ! have=$(cargo about --version 2>/dev/null) || [ "$have" != "cargo-about $CARGO_ABOUT_VERSION" ]; then
    echo "need cargo-about $CARGO_ABOUT_VERSION (now: ${have:-missing}):" >&2
    echo "  cargo install cargo-about --locked --features cli --version $CARGO_ABOUT_VERSION" >&2
    exit 1
fi
mkdir -p dist
# --fail: a crate under an unknown or unaccepted licence (about.toml) stops the generation.
cargo about generate --locked --fail -c about.toml packaging/about.hbs -o dist/THIRD-PARTY.txt

# egui's fonts come from [epaint_default_fonts.clarify] (about.toml); without it their texts are missing.
for text in "UBUNTU FONT LICENCE" "BITSTREAM VERA LICENSE" "Copyright (c) 2014 John Slegers" \
    "SIL OPEN FONT LICENSE"; do
    if ! grep -qF "$text" dist/THIRD-PARTY.txt; then
        echo "dist/THIRD-PARTY.txt has no '$text': check [epaint_default_fonts.clarify] in about.toml" >&2
        exit 1
    fi
done
echo "done: dist/THIRD-PARTY.txt ($(grep -c '^  - ' dist/THIRD-PARTY.txt) crates)"
