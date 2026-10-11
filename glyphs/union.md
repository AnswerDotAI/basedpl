

# `∪` — Unique / Union

Keys: `Alt-v`. Ranks: `∞` monadic, `∞ ∞` dyadic

`∪Y` keeps the first of each matching major cell. Uses tolerant
matching.

``` bpl
∪3 1 3 2 1         ⍝ 3 1 2
```

[`∩`](intersection.qmd) and [`⍷`](find.qmd) give the duplicates and
positions of the same distinct cells. `≢¨⍷Y` gives how many times each
one occurs.

`X∪Y` appends the major cells of `Y` absent from `X`. Order and
repetitions are retained.

``` bpl
1 2∪2 3            ⍝ 1 2 3
1 1∪2 2            ⍝ 1 1 2 2
[1 2⋄3 4]∪[3 4⋄5 6] ⍝ [1 2⋄3 4⋄5 6]
```

## Errors

- `RANK`: `X` is a unit
- `LENGTH`: the major cells of `X` and `Y` differ in shape
