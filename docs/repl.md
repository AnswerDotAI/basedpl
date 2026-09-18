# REPL

[Home](index.md) · [Command line](cli.md)

Run `miniapl`. Expressions display their results; assignments retain names for later lines. An error shows its source location and returns to the prompt.

```text
      v←⍳5
      +/v
15
```

## Typing glyphs

Type a backtick followed by a name. Tab replaces it with the glyph. A non-letter accepts the glyph and enters that character too.

| Input | Result |
|---|---|
| `` `io `` then Tab | `⍳` |
| `` `iota5 `` | `⍳5` |
| `` 2`times3 `` | `2×3` |
| `` v`assign `` then Space | `v← ` |
| `` `scan `` then Tab | `\` |

Names are case-insensitive. Exact names win, then prefixes, then abbreviations retaining the first letter and later letters in order. `lar` matches `left-arrow`; `grup` matches `grade-up`. Type letters only: `` `lar- `` becomes `←-`.

Matches appear as you type. Tab twice lists ambiguous choices; backtick then Tab twice lists the catalogue. Ambiguous input stays as typed. Enter accepts a unique match and submits the line.

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

Nested arrays have boxes, axis arrows and type markers. Enter a function name to see its tree.

| Command | Effect |
|---|---|
| `]box off` | Plain display |
| `]box on` | Boxed display |
| `]box on -trains=def` | Function expressions |
| `]box on -trains=tree` | Function trees |
| `]box on -fns=off` | Leave output inside functions unboxed |
| `]box ?` | Show settings |
| `]Display (1 2)(3 4)` | Draw one array without changing settings |

Boxing controls how results are drawn. Batch, Python and JSON sessions start with plain display.

`⎕←` explicitly prints a value, including an assignment's result.

```text
      ⎕←v←⍳3
1 2 3
```
