#!/bin/bash
# Copy the repository's keyboard layout bundle over the installed one, register it, and open Keyboard settings. Registration needs Xcode Command Line Tools for Swift.
set -euo pipefail

src="$(cd "$(dirname "$0")/.." && pwd)/editors/macos/BasedPL.bundle"
dest="$HOME/Library/Keyboard Layouts"

mkdir -p "$dest"
rm -rf "$dest/BasedPL.bundle"
cp -R "$src" "$dest/"

xcrun swift - "$dest/BasedPL.bundle" <<'SWIFT'
import Carbon
import Foundation

let url = URL(fileURLWithPath: CommandLine.arguments[1])
let status = TISRegisterInputSource(url as CFURL)
if status != noErr {
    fputs("Layout copied, but registration failed: OSStatus \(status)\n", stderr)
    exit(1)
}
SWIFT

echo "Installed BasedPL. In Keyboard settings → Text Input → Edit, add BasedPL."
echo "Then select BasedPL from the input menu."
