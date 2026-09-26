

# REPL

[Home](index.ipynb) · [Command line](cli.qmd)

Run `bapl`. Expressions display their results; assignments retain names
for later lines. An error shows its source location and returns to the
prompt.

``` text
      v←⍳5
      +/v
10
```

## Typing glyphs

Hold Alt and press a key from the [keyboard layout](keyboard.qmd):
**Alt-h/j/k/l** gives `← ↓ ↑ →`, **Alt–** gives `×`, **Alt-=** gives
`÷`. Alt-Shift-a/w gives the operands `⍶ ⍹`. Chords insert literal
characters, including inside strings and comments.

On macOS, configure your terminal to send left Option as Alt. In
Ghostty:

``` text
macos-option-as-alt = left
```

Right Option keeps normal Mac character entry. iTerm2 offers **Left
Option → Esc+**; Terminal.app calls its setting **Use Option as Meta
key**. Glyph chords take precedence over Alt-based editing shortcuts;
cursor keys and Ctrl shortcuts remain available.

### Named entry

Type a backtick followed by a name. Tab replaces it with the glyph. A
non-letter accepts the glyph and enters that character too.

Use `` `bullet `` then Tab for the system prefix `•`; `` `quad `` gives
the output glyph `⎕`.

Before `⊸` is Alt-u, or `` `before `` then Tab. After `⟜` is Alt-t, or
`` `after `` then Tab. Use `` `pi `` for `π` and `` `sqrt `` for `√`.

<table>
<thead>
<tr>
<th>Input</th>
<th>Result</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>`io</code> then Tab</td>
<td><code>⍳</code></td>
</tr>
<tr>
<td><code>`iota5</code></td>
<td><code>⍳5</code></td>
</tr>
<tr>
<td><code>2`times3</code></td>
<td><code>2×3</code></td>
</tr>
<tr>
<td><code>v`assign</code> then Space</td>
<td><code>v←</code></td>
</tr>
<tr>
<td><code>`scan</code> then Tab</td>
<td><code>\</code></td>
</tr>
</tbody>
</table>

Each glyph has one name, the one the [glyph reference](glyphs.qmd) uses.
The REPL also matches search words such as `reshape` for `⍴`. It always
shows the name: `` `resh `` lists `⍴ rho r`.

Names are case-insensitive. Exact matches win, then prefixes, then
abbreviations formed from prefixes of successive hyphen-separated parts.
At each level a name beats a search word. A name that is a prefix of
every other match wins: `` `om `` gives `⍵`, and `` `omu `` gives `⍹`.
`grup` matches `grade-up`. `lar` matches the search word `left-arrow`,
not `logarithm`, and lists `← assign h`. Type letters only: `` `lar- ``
becomes `←-`.

Matches appear as you type. Each listed name ends with its Alt key: `a`
for Alt-a, `Sa` for Alt-Shift-a. Tab twice lists ambiguous choices;
backtick then Tab twice lists the catalogue. Ambiguous input stays as
typed. Enter accepts a unique match and submits the line.

Expansion applies only to typed REPL input, outside strings and
comments. Pasted APL, source files, Python and process requests use
actual glyphs.

## Editing and multiline input

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Key</th>
<th>Action</th>
</tr>
</thead>
<tbody>
<tr>
<td>Left / Right</td>
<td>Move within input</td>
</tr>
<tr>
<td>Up / Down</td>
<td>Recall this session’s history</td>
</tr>
<tr>
<td>Ctrl-C</td>
<td>Cancel the current input, including unfinished multiline input</td>
</tr>
<tr>
<td>Ctrl-D on an empty line</td>
<td>Exit</td>
</tr>
</tbody>
</table>

An unclosed delimiter gives a continuation prompt. Closing it evaluates
the complete input once.

``` text
      double←{
    · ⍵×2
    · }
      double 3
6
```

Inside brackets and parentheses, a line break counts as a space. Rows of
an array need `⋄`.

## Array and function display

Interactive sessions start with:

``` text
]box on -style=max -trains=tree -fns=on
```

Arrays with axes have boxes, axis arrows and type markers. Enter a
function name to see its tree.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Command</th>
<th>Effect</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>]box off</code></td>
<td>Plain display</td>
</tr>
<tr>
<td><code>]box on</code></td>
<td>Boxed display</td>
</tr>
<tr>
<td><code>]box on -trains=def</code></td>
<td>Function expressions</td>
</tr>
<tr>
<td><code>]box on -trains=tree</code></td>
<td>Function trees</td>
</tr>
<tr>
<td><code>]box on -fns=off</code></td>
<td>Leave output inside functions unboxed</td>
</tr>
<tr>
<td><code>]box ?</code></td>
<td>Show settings</td>
</tr>
<tr>
<td><code>]box reset</code></td>
<td>Restore the starting display settings</td>
</tr>
<tr>
<td><code>]help +</code></td>
<td>Help for a name or glyph</td>
</tr>
<tr>
<td><code>]help mean -source</code></td>
<td>Definition source</td>
</tr>
<tr>
<td><code>]Display [[1 2] [3 4]]</code></td>
<td>Draw one array without changing settings</td>
</tr>
<tr>
<td><code>]clear</code></td>
<td>Remove every name and restore the starting display settings</td>
</tr>
</tbody>
</table>

Boxing controls how results are drawn. Batch, Python and JSON sessions
start with plain display.

Both display modes show scalars as source: `⊂4ₓ`, `⊂⊂4ₓ` or `⊂1 2`.

`⎕←` explicitly prints a value, including an assignment’s result.

``` text
      ⎕←v←⍳3
0 1 2
```
