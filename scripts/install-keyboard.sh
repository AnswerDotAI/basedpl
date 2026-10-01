#!/bin/bash
# Copy the repository's keyboard layout bundle over the installed one, and say how to enable it.
set -euo pipefail

src="$(cd "$(dirname "$0")/.." && pwd)/editors/macos/BasedPL.bundle"
dest="$HOME/Library/Keyboard Layouts"

mkdir -p "$dest"
rm -rf "$dest/BasedPL.bundle"
cp -R "$src" "$dest/"

echo "Installed BasedPL. In Keyboard settings → Text Input → Edit, add BasedPL, then select it from the input menu."
echo "If BasedPL isn't listed, or the old layout stays active, log out and back in."
