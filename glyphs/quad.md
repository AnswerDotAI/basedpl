

# [`⎕`](https://answerdotai.github.io/basedpl/glyphs/quad.html) — IO

Syntax. Keys: `Alt-q`. Hanzi: `出/入`

`⎕←Y` prints `Y` and returns `Y`.

``` bpl
≥⎕←2              ⍝ 3
```

The example prints `2` before returning `3`. Explicit output is separate
from implicit result display and is captured by the evaluation API.

## Input

Reading [`⎕`](https://answerdotai.github.io/basedpl/glyphs/quad.html)
gives the next line of standard input as a string, without its line
ending. End of input:
[`⍬`](https://answerdotai.github.io/basedpl/glyphs/zilde.html) (empty
line: `""`). `⍎⎕` evaluates the line, as Dyalog’s
[`⎕`](https://answerdotai.github.io/basedpl/glyphs/quad.html) does.
`•nget "-"` gives the rest of standard input as one string.

``` bash
printf '3\n4\n' | bpl -e '+/⍎¨[⎕ ⎕]'       # 7
```

`"-" •nput text` writes `text` to standard output with no line ending
added. When
[`⎕`](https://answerdotai.github.io/basedpl/glyphs/quad.html) is read,
the text written since the last line ending is its prompt. A terminal
shows the prompt and reads the answer on the same line, and a notebook
shows the prompt beside its text box.

``` bash
printf 'bob\n' | bpl -e '"-" •nput "Name? " ◇ "Hi ",⎕'   # Name? Hi bob
```

`bpl -e`, script files and the interactive REPL read standard input. In
a notebook,
[`⎕`](https://answerdotai.github.io/basedpl/glyphs/quad.html) asks for a
line in a text box, but `•nget "-"` raises an `IO` error. The Python API
reads each line with Python’s `input()`, and `•nget "-"` reads
`sys.stdin`. `bpl -` and piped REPL input read the program itself from
standard input. In those two modes and in `bpl --worker`, reading
[`⎕`](https://answerdotai.github.io/basedpl/glyphs/quad.html) or
`•nget "-"` raises an `IO` error.

Dyalog’s `⎕A` and `⎕D` are BPL’s [`$a` and
`$d`](../system/constants.qmd). `⎕C` and `⎕UCS` are
[`•c`](../system/c.qmd) and [`•ucs`](../system/ucs.qmd).
