# shellcheck shell=bash
# Variables shared by the packaging scripts (source with `. packaging/env.sh`).
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP=rigscope
# The only place of the version: [workspace.package] version in Cargo.toml.
VERSION="$(sed -n '/^\[workspace\.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p;}' "$ROOT/Cargo.toml" | head -1)"
OUT="$ROOT/dist/release"
export ROOT APP VERSION OUT
