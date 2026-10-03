

# `⍕` — Format

Keys: `Alt-n t`

`⍕Y` returns character text. Matrix rows remain rows.

``` bpl
⍕1 2 3             ⍝ "1 2 3"
```

`D⍕Y` gives `D` decimal places. Negative `D` selects exponent notation.

``` bpl
2⍕3.125            ⍝ " 3.13"
```

`W D⍕Y` also sets the field width `W`. Give one pair per column to set
each column separately. Width 0 chooses a width. Overflowing fields
become `*`.

``` bpl
6 2⍕3.125          ⍝ "  3.13"
```

Formatting is independent of REPL boxing.
