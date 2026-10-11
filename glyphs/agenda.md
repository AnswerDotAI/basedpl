

# `⍚` — Agenda

Keys: `Alt-Minus Backtick`

`selector⍚cases` derives a function that selects a branch on each call.
`cases` is a nonempty vector of functions. An array among them acts as a
constant function. The selector can be a unit index or a function
returning one. Indices are integral and count from 0, and negative
indices count from the end. A Boolean selector picks case 0 for `$f` and
case 1 for `$t`.

``` bpl
cases←[- ⊢] ⋄ abs←0≤ ⍚ cases ⋄ abs¯3          ⍝ 3
cases←[- ⊢] ⋄ abs←0≤ ⍚ cases ⋄ abs3           ⍝ 3
mul←1⍚[+ ×] ⋄ 2 mul 3                         ⍝ 6
mul←¯1⍚[+ ×] ⋄ 2 mul 3                        ⍝ 6
choose←>⍚[- ÷] ⋄ 12 choose 3                  ⍝ 4
nonneg←0≤ ⍚ [0 ⊢] ⋄ nonneg¨ ¯3 4               ⍝ 0 4
```

A function selector runs once with the original arguments. Only the
selected branch runs, with those same arguments. Index arrays do not map
over branches or construct trains.

## Errors

- `RANK`: an index that is not a unit
- `INDEX`: out-of-range index
- `DOMAIN`: nonnumeric or nonintegral index
