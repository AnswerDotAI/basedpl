

# `⍨` — Self / Commute

Monadic operator. Keys: `Alt-1 Backtick`. Hanzi: `己/换` (`己/換`)

`f⍨Y` is `Y f Y`.

``` bpl
×⍨3                ⍝ 9
```

`X f⍨ Y` is `Y f X`.

``` bpl
2-⍨10              ⍝ 8
```

A subject operand makes a constant function: `A⍨Y` and `X A⍨Y` return
`A`.

``` bpl
42⍨0               ⍝ 42
```

## Inverse

`X f⍨⁻Y` gives the `Z` for which `Z f X` is `Y`. `+⍨⁻Y` halves `Y`,
`×⍨⁻Y` takes its square root, and `⌊⍨⁻Y`, `⌈⍨⁻Y` and `⊢⍨⁻Y` give back
`Y`. For a subject `k`, `k⍨⁻Y` gives back `Y`, which must match `k`.

``` bpl
+⍨⁻10   ⍝ 5
×⍨⁻9    ⍝ 3
```
