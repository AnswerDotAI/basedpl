# Glyph keyboard

[REPL](repl.md#typing-glyphs) · [Glyph reference](index.md#glyph-reference)

Hold **Alt** for the middle column, **Alt-Shift** for the right. Keys refer to a US layout. Ordinary ASCII characters use their normal keys.

| Key | Alt | Alt-Shift |
|---|---|---|
| Backtick | `⋄` | `⌺` |
| 1 | `¨` | |
| 2 | `¯` | |
| 3 | `√` | `⍒` |
| 4 | `≤` | `⍋` |
| 5 | `⇄` | `⌽` |
| 6 | `≥` | `⍉` |
| 7 | `⌝` | `⊖` |
| 8 | `≠` | `⍟` |
| 9 | `∨` | `⍱` |
| 0 | `∧` | `⍲` |
| - | `×` | `∞` |
| = | `÷` | `⌹` |
| q | `•` | |
| w | `⍵` | `⍹` |
| e | `∊` | `⍷` |
| r | `⍴` | `ℙ` |
| t | `˘` | `⍨` |
| y | `◶` | |
| u | `∘` | `↕` |
| i | `⍳` | `⍸` |
| o | `○` | |
| p | `π` | `⍣` |
| [ | | `⍞` |
| ] | `⎕` | `⍬` |
| Backslash | `⊢` | `⊣` |
| a | `⍺` | `⍶` |
| s | `⌈` | |
| d | `⌊` | |
| f | `∂` | `⍛` |
| g | `∇` | `⍢` |
| h | `←` | `∆` |
| j | `↓` | `⍤` |
| k | `↑` | `⌸` |
| l | `→` | `⌷` |
| ; | `⍎` | `≡` |
| ' | `⍕` | `≢` |
| z | `⊂` | `⊆` |
| x | `⊃` | |
| c | `∩` | `⨸` |
| v | `∪` | `⍥` |
| b | `⊥` | |
| n | `⊤` | |
| m | `⌾` | `⊛` |
| , | `⍝` | `⍪` |
| . | `⍀` | `⍙` |
| / | `⌿` | `⍠` |

`∆` and `⍙` are identifier characters.

The layout retains Dyalog positions where practical, with hjkl arrows and spare keys for bAsedPL's additions. Alt-[ and Alt-Shift-o are left vacant because their terminal encodings start control sequences.

## Editor integration

The shared table is `python/basedpl/keyboard.json`, shipped in the Python package and embedded in the Rust REPL. Each key is the US character **after Shift, before Alt**: `a` maps to `⍺`, `A` to `⍶`, `_` to `∞`.

```python
import json
from importlib.resources import files

keymap = json.loads(files('basedpl').joinpath('keyboard.json').read_text())
assert keymap['h'] == '←' and keymap['A'] == '⍶'
```

Monaco/CodeMirror adapters can import the same JSON. Derive the US character from `KeyboardEvent.code` and Shift: macOS Option may already have changed `event.key`. Track `AltLeft`/`AltRight` keydown/up to select one Option key, and clear modifier state on blur. For a mapped chord, prevent the browser's default action and insert the glyph through the editor's normal edit transaction. Leave composition events to the input method.
