#!/bin/bash
# Install the macOS layout; requires Xcode Command Line Tools for Swift.
set -euo pipefail

src="$(cd "$(dirname "$0")/../editors/macos" && pwd)"
dest="$HOME/Library/Keyboard Layouts"

mkdir -p "$dest"
cp "$src/bAsedPL.keylayout" "$src/bAsedPL.icns" "$dest/"

xcrun swift - "$dest/bAsedPL.keylayout" <<'SWIFT'
import Carbon
import Foundation

let url = URL(fileURLWithPath: CommandLine.arguments[1])
let status = TISRegisterInputSource(url as CFURL)
if status != noErr {
    fputs("Layout copied, but registration failed: OSStatus \(status)\n", stderr)
    exit(1)
}
SWIFT

echo "Installed bAsedPL. In Keyboard settings → Text Input → Edit, add bAsedPL."
echo "Then select bAsedPL from the input menu."
open "x-apple.systempreferences:com.apple.Keyboard-Settings.extension"
