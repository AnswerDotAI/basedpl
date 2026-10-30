

# `⁝` — Token separator

Syntax. Keys: `Alt-:`. Hanzi: `隔`

`⁝` separates tokens without ending their run. It prevents adjacent
names from merging and stops a literal strand continuing. It has no
value and makes no function call.

``` bpl
h←- ⋄ m←5 ⋄ h⁝m             ⍝ ¯5
≥⍣3⁝5                       ⍝ 8
```

`h⁝m` is `h(m)`, and `≥⍣3⁝5` is `(≥⍣3)5`. A space also groups runs, as
[Syntax](../syntax.qmd#runs) describes. `⁝` leaves the grouping
unchanged. Parentheses still group compound operands and trains within a
run. [Composing
functions](../composition.qmd#separating-an-operand-from-its-argument)
gives examples.
