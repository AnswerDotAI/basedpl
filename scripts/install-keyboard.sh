#!/bin/bash
# Copy a regional keyboard bundle and say how to enable it.
set -euo pipefail

code="${1:-us}"
if [[ $# -gt 1 || ! "$code" =~ ^[a-z]{2}$ ]]; then
    echo "Usage: $0 [two-letter layout code]"
    exit 1
fi
name="BasedPL-$code"
src="$(cd "$(dirname "$0")/.." && pwd)/editors/macos/$name.bundle"
dest="$HOME/Library/Keyboard Layouts"
if [[ ! -d "$src" ]]; then
    echo "No keyboard bundle for '$code': $src"
    exit 1
fi

mkdir -p "$dest"
cp -R "$src" "$dest/"

echo "Installed $name. In Keyboard settings → Text Input → Edit, add $name, then select it from the input menu."
echo "If $name isn't listed, or the old layout stays active, log out and back in."
