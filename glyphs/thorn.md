

# `⍕` — Format / Format spec

Function. Keys: `Alt-o n`. Hanzi: `文`. Ranks: `∞` monadic, `1 ∞` dyadic

`⍕Y` returns character text. Matrix rows remain rows. Numbers that share
a marked kind write as display shows them: as their literal laid out in
rows, with one mark after the brackets. Each plane of a higher rank is
its own bracketed block.

``` bpl
⍕1 2 3             ⍝ "1 2 3"
⍕2 2⍴⍳4ₓ           ⍝ 2 6⍴"[0 1   2 3]ₓ"
```

`D⍕Y` gives `D` decimal places. A negative `D` gives exponent notation
with `|D|` significant digits.

``` bpl
2⍕3.125 10         ⍝ "3.13 10.00"
¯2⍕1234.5          ⍝ "1.2ₑ3"
```

An empty `D`, `⍬` or `''`, gives `⍕Y`‘s text without the markers of
numbers’ kinds, such as `ₓ`.

``` bpl
⍕[1 2 3]ₓ          ⍝ "[1 2 3]ₓ"
⍬⍕[1 2 3]ₓ         ⍝ "1 2 3"
''⍕2 2⍴⍳4ₓ         ⍝ 2 3⍴"0 12 3"
```

`W D⍕Y` also sets the field width `W`. Give one pair per column to set
each column separately. Width 0 fits each column to its longest number,
with one space between columns. Overflowing fields become `*`.

``` bpl
6 2⍕3.125          ⍝ "  3.13"
```

Formatting is independent of REPL boxing.
