

# `⊢` — Right

`⊢Y` and `X⊢Y` return `Y`.

``` apl
⊢1 2               ⍝ 1 2
1 2⊢3 4            ⍝ 3 4
```

Both arguments evaluate. `⊢` also separates a literal operand from a
literal argument. Without it, `1+⍣3 0` would read `3 0` as one literal
run.

``` apl
1+⍣3⊢0             ⍝ 3
```
