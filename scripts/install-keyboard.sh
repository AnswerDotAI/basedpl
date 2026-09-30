#!/bin/bash
# Install the macOS layout; requires Xcode Command Line Tools for Swift.
set -euo pipefail

src="$(cd "$(dirname "$0")/../editors/macos" && pwd)"
dest="$HOME/Library/Keyboard Layouts"

mkdir -p "$dest"
cp "$src/BasedPL.keylayout" "$src/BasedPL.icns" "$dest/"

xcrun swift - "$dest/BasedPL.keylayout" <<'SWIFT'
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
open "x-apple.systempreferences:com.apple.Keyboard-Settings.extension"
