# Getting started

[Home](index.md) · [REPL](repl.md) · [Python](python.md)

## Install

Requires Python ≥3.10.

```bash
pip install miniapl
```

This installs the `miniapl` command and Python package.

## First calculations

Run `miniapl` to open the [REPL](repl.md). Type an expression and press Enter. `⍝` introduces a comment; examples here use it to show the result.

APL evaluates right-to-left. Parentheses change grouping.

```apl
2×3+4                 ⍝ 14
(2×3)+4               ⍝ 10
```

Spaces form vectors. Functions work on every element.

```apl
10+1 2 3              ⍝ 11 12 13
```

`⍳` generates indices, `←` assigns a name, and `+/` sums.

```apl
v←⍳5 ⋄ v              ⍝ 1 2 3 4 5
+/⍳5                  ⍝ 15
```

To enter `⍳5`, type backtick, `iota`, then `5`. The digit accepts the glyph and enters the argument. [Symbol entry](repl.md#typing-glyphs) also supports abbreviations and Tab completion.

## Numbers and arrays

Bare numbers are approximate. Use `x` for exact integers and `r` for fractions.

```apl
1÷3                   ⍝ 0.3333333333333333
1x÷3x                 ⍝ 1r3
```

`j` separates real and imaginary parts.

```apl
1j2×1j¯2              ⍝ 5
```

`⍴` reshapes a vector; `+/` sums each row.

```apl
+/2 3⍴⍳6              ⍝ 6 15
```

Indices start at 1. Comparisons use tolerance `1E¯14`; `0.3=0.1+0.2` is true. See [language rules](rules.md) and the [glyph index](index.md#glyph-reference).
