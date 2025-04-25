#!/usr/bin/env bash
# For the tar.gz and the AppImage: installs the root-owned helper and the polkit policy, so that
# "Read more (root)" can ask for a password. The .deb does this by itself.
#   sudo ./install-helper.sh [path/to/rigscope]
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
bin="${1:-$here/rigscope}"
policy="$here/org.rigscope.dump-dmi.policy"
[ -x "$bin" ] || { echo "no executable at $bin" >&2; exit 1; }
[ -f "$policy" ] || { echo "no policy file at $policy" >&2; exit 1; }
install -D -m 755 -o root -g root "$bin" /usr/libexec/rigscope/rigscope-helper
install -D -m 644 -o root -g root "$policy" /usr/share/polkit-1/actions/org.rigscope.dump-dmi.policy
echo "Installed /usr/libexec/rigscope/rigscope-helper and the polkit policy."
