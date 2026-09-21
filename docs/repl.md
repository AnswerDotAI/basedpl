# REPL

[Home](index.md) · [Command line](cli.md)

Run `bapl`. Expressions display their results; assignments retain names for later lines. An error shows its source location and returns to the prompt.

```text
      v←⍳5
      +/v
15
```

## Typing glyphs

Hold Alt and press a key from the [keyboard layout](keyboard.md): **Alt-h/j/k/l** gives `← ↓ ↑ →`, **Alt--** gives `×`, **Alt-=** gives `÷`. Alt-Shift-a/w gives the operands `⍶ ⍹`. Chords insert literal characters, including inside strings and comments.

On macOS, configure your terminal to send left Option as Alt. In Ghostty:

```text
macos-option-as-alt = left
```

Right Option keeps normal Mac character entry. iTerm2 offers **Left Option → Esc+**; Terminal.app calls its setting **Use Option as Meta key**. Glyph chords take precedence over Alt-based editing shortcuts; cursor keys and Ctrl shortcuts remain available.

### Named entry

Type a backtick followed by a name. Tab replaces it with the glyph. A non-letter accepts the glyph and enters that character too.

Use `` `bullet`` then Tab for the system prefix `•`; `` `quad`` gives the output glyph `⎕`.

Strand `˘` is Alt-t, or right Option–Shift–period on a US Mac keyboard, or `` `strand`` then Tab. Use `` `pi`` for `π` and `` `sqrt`` for `√`.

| Input | Result |
|---|---|
| `` `io `` then Tab | `⍳` |
| `` `iota5 `` | `⍳5` |
| `` 2`times3 `` | `2×3` |
| `` v`assign `` then Space | `v← ` |
| `` `scan `` then Tab | `\` |

Each glyph has one name, the one the [glyph reference](index.md#glyph-reference) uses. The REPL also matches search words such as `reshape` for `⍴`. It always shows the name: `` `resh `` lists `⍴ rho r`.

Names are case-insensitive. Exact matches win, then prefixes, then abbreviations retaining the first letter and later letters in order. At each level a name beats a search word. A name that is a prefix of every other match wins: `` `om `` gives `⍵`, and `` `omu `` gives `⍹`. `grup` matches `grade-up`. `lar` matches the search word `left-arrow` and lists `← assign h`. Type letters only: `` `lar- `` becomes `←-`.

Matches appear as you type. Each listed name ends with its Alt key: `a` for Alt-a, `Sa` for Alt-Shift-a. Tab twice lists ambiguous choices; backtick then Tab twice lists the catalogue. Ambiguous input stays as typed. Enter accepts a unique match and submits the line.

Expansion applies only to typed REPL input, outside strings and comments. Pasted APL, source files, Python and process requests use actual glyphs.

## Editing and multiline input

| Key | Action |
|---|---|
| Left / Right | Move within input |
| Up / Down | Recall this session's history |
| Ctrl-C | Cancel the current input, including unfinished multiline input |
| Ctrl-D on an empty line | Exit |

An unclosed delimiter gives a continuation prompt. Closing it evaluates the complete input once.

```text
      double←{
    · ⍵×2
    · }
      double 3
6
```

Inside array literals, newlines separate items/rows, like `⋄`.

## Array and function display

Interactive sessions start with:

```text
]box on -style=max -trains=tree -fns=on
```

Arrays with axes have boxes, axis arrows and type markers. Enter a function name to see its tree.

| Command | Effect |
|---|---|
| `]box off` | Plain display |
| `]box on` | Boxed display |
| `]box on -trains=def` | Function expressions |
| `]box on -trains=tree` | Function trees |
| `]box on -fns=off` | Leave output inside functions unboxed |
| `]box ?` | Show settings |
| `]Display (1 2⋄ 3 4)` | Draw one array without changing settings |

Boxing controls how results are drawn. Batch, Python and JSON sessions start with plain display.

Both display modes show scalars as `⊂4x`, `⊂⊂4x` or `⊂(1 2)`.

`⎕←` explicitly prints a value, including an assignment's result.

```text
      ⎕←v←⍳3
1 2 3
```
