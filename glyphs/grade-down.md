

# `⍒` — Grade down

Keys: `Alt-Backslash g`. Ranks: `∞` monadic, `∞ ∞` dyadic

`⍒Y` gives indices that sort major cells descending. Stable, with the
same [ordering](grade-up.qmd) as Grade up.

``` bpl
⍒30 10 20          ⍝ [0 2 1]ₓ
⍒2 1 2 1           ⍝ [0 2 1 3]ₓ
```

`C⍒Y` uses character collation `C`.

``` bpl
"cba"⍒"abc"        ⍝ [0 1 2]ₓ
```

See [`>`](greater.qmd) to sort.

## Inverse

`⍒⁻¹P` gives an array whose descending grade is the permutation `P`. Any
other `P` is a `DOMAIN` error.

``` bpl
⍒⁻¹2 0 1   ⍝ [1 0 2]ₓ
```
