# Windows keyboard

BasedPL-us is a native US keyboard layout for x64 Windows 10 and 11. Normal US typing stays unchanged. Right Alt (AltGr) replaces Option in the [BPL keyboard map](../../nbs/keyboard.qmd). Use Shift+AltGr for the shifted layer. No helper runs in the background.

## Install

Download `basedpl-keyboard-us-windows-x64.zip` from the [latest release](https://github.com/AnswerDotAI/basedpl/releases/latest) and extract it. Open 64-bit PowerShell as administrator in the extracted folder, then run:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The script copies the native and WOW64 layout DLLs into Windows and registers BasedPL-us without replacing another layout. Add **BasedPL-us** under **English (United States)** in **Settings > Time & language > Language & region > Language options > Keyboards**. Double-click `SAX2B.ttf` and choose **Install**. Sign out and in, then select BasedPL-us with **Win+Space**.

For example, AltGr+i types `⍳`. AltGr+o followed by `m` types `⊗`. Press a dead-key chord twice, or follow it with Space, to type its own glyph.

Shift+AltGr+1–9 types `¹²³⁴⁵⁶⁷⁸⁹`. Shift+AltGr+Minus types `⁻`. Shift+AltGr+0 types `⍬`.

Windows dead keys emit one UTF-16 character. Type a script minus and digit separately: AltGr+6, `-`, then AltGr+6, `1` gives `⁻¹`. Use AltGr+5 in the same way for `₋₁`. Other BPL keyboards retain their existing negative-script sequences.

This first Windows build still needs a Windows typing check, including normal punctuation, dead keys, negative scripts and a 32-bit application. Remove the layout from your keyboard list before updating it. If Windows reports that a DLL is in use, sign out and in before rerunning the installer.

## Build

`basedpl.editors.write()` generates `BasedPL-us.klc` from `python/basedpl/layout.json` during `scripts/prep.py`. Only the Windows generator changes the negative-script sequences. The shared map is unchanged.

On Windows, install [Microsoft Keyboard Layout Creator 1.4](https://www.microsoft.com/en-us/download/details.aspx?id=102134), then run from the repository:

```powershell
./editors/windows/build.ps1 -Compiler 'C:\Program Files (x86)\Microsoft Keyboard Layout Creator 1.4\bin\i386\kbdutool.exe'
```

The compiler's `-u -m` and `-u -o` modes build the native x64 and WOW64 DLLs. The script writes the distribution ZIP to `dist/`. CI extracts the same official compiler without installing the MSKLC GUI. ARM Windows is not supported by this build.
