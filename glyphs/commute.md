

# `⍨` — Self / Commute / Constant

Keys: `Alt-1 Backtick`

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

With one argument fixed, `f⍨⁻¹` inverts `f` with that argument on the
other side. `+⍨⁻¹Y` halves `Y`, `×⍨⁻¹Y` takes its square root, and
`⌊⍨⁻¹Y`, `⌈⍨⁻¹Y` and `⊢⍨⁻¹Y` give back `Y`. For an array `k`, `k⍨⁻¹Y`
gives back `Y`, which must match `k`.

``` bpl
+⍨⁻¹10   ⍝ 5
×⍨⁻¹9    ⍝ 3
```
