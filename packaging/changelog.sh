#!/usr/bin/env bash
# The notes of one version from CHANGELOG.md (for the GitHub release).
#   packaging/changelog.sh            the version in Cargo.toml
#   packaging/changelog.sh 0.2.0
# Fails when CHANGELOG.md has no section for the version.
set -euo pipefail
. "$(dirname "$0")/env.sh"
cd "$ROOT"

v="${1:-$VERSION}"
v="${v#v}"
notes="$(awk -v head="## [$v]" '
    index($0, head) == 1 { found = 1; next }
    found && /^## / { exit }
    found && /^\[[^]]+\]: / { exit }
    found { print }
' CHANGELOG.md | sed -e '/./,$!d' -e :a -e '/^\n*$/{$d;N;ba' -e '}')"

if [ -z "$notes" ]; then
    echo "CHANGELOG.md has no section \"## [$v]\"." >&2
    exit 1
fi
printf '%s\n' "$notes"
