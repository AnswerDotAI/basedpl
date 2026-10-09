#!/bin/bash
# Install SAX2B and a regional keyboard bundle.
set -euo pipefail

code="${1:-us}"
if [[ $# -gt 1 || ! "$code" =~ ^[a-z]{2}$ ]]; then
    echo "Usage: $0 [two-letter layout code]"
    exit 1
fi
name="BasedPL-$code"
root="$(cd "$(dirname "$0")/.." && pwd)"
src="$root/editors/macos/$name.bundle"
dest="$HOME/Library/Keyboard Layouts"
if [[ ! -d "$src" ]]; then
    echo "No keyboard bundle for '$code': $src"
    exit 1
fi

mkdir -p "$dest" "$HOME/Library/Fonts"
cp -R "$src" "$dest/"
cp "$root/nbs/fonts/SAX2B.ttf" "$HOME/Library/Fonts/"

echo "Installed SAX2B and $name. In Keyboard settings → Text Input → Edit, add $name, then select it from the input menu."
echo "If $name isn't listed, or the old layout stays active, log out and back in."
