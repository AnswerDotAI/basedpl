# Windows keyboard

BasedPL-us is a native US keyboard layout for 64-bit apps on x64 and ARM64 Windows. Normal US typing stays unchanged. Right Alt (AltGr) replaces Option in the [BPL keyboard map](../../nbs/keyboard.qmd). Use Shift+AltGr for the shifted layer. No helper runs in the background.

## Install

Download `basedpl-keyboard-us-windows.zip` from the [latest release](https://github.com/AnswerDotAI/basedpl/releases/latest) and extract it. Open 64-bit PowerShell as administrator in the extracted folder, then run:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The script selects the DLL for your system, registers BasedPL-us and installs SAX2B. It does not replace another layout. Add **BasedPL-us** under **English (United States)** in **Settings > Time & language > Language & region > Language options > Keyboards**, then select it with **Win+Space**. Sign out and in if it is not listed.

For example, AltGr+i types `⍳`. AltGr+o followed by `m` types `⊗`. Press a dead-key chord twice, or follow it with Space, to type its own glyph.

Shift+AltGr+1–9 types `¹²³⁴⁵⁶⁷⁸⁹`. Shift+AltGr+Minus types `⁻`. Shift+AltGr+0 types `⍬`.

Windows dead keys emit one UTF-16 character. Type a script minus and digit separately: AltGr+6, `-`, then AltGr+6, `1` gives `⁻¹`. Use AltGr+5 in the same way for `₋₁`. Other BPL keyboards retain their existing negative-script sequences.

Remove the layout from your keyboard list before updating it. If Windows reports that a DLL is in use, sign out and in before rerunning the installer.

## Build

`basedpl.editors.write()` generates `layout.h` from `python/basedpl/layout.json` during `scripts/prep.py`. `keyboard.c` supplies the standard US scan-code and modifier tables.

Install Microsoft's [C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with **C++ core build tools**, **MSVC x64/x86**, **MSVC ARM64**, **MSVC ARM64EC** and a **Windows SDK**. From the repository, run:

```powershell
./editors/windows/build.ps1
```

The build uses the Windows SDK's `kbd.h` and Microsoft's standard keyboard-layout tables. It writes `dist/basedpl-keyboard-us-windows.zip` with an x64 DLL, an [ARM64X DLL](https://learn.microsoft.com/en-us/windows/arm/arm64x-build) for native and x64-emulated apps on ARM64 Windows, the installer, SAX2B and the source files. No MSKLC compiler is used.

`tests/windows_keyboard.c` checks native key translation, including Unicode dead keys. Compile it with MSVC and `user32.lib`, then run it in a logged-in Windows session with the installed layout's registry ID as its argument.
