

# `⍨` — Self / Commute / Constant

Keys: `Alt-1 ~`

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
