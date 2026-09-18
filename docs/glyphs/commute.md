# `⍨` — Self / Commute / Constant

`f⍨Y` is `Y f Y`.

```apl
×⍨3                ⍝ 9
```

`X f⍨Y` is `Y f X`.

```apl
2-⍨10              ⍝ 8
```

An array operand makes a constant function: `A⍨Y` and `X A⍨Y` return `A`.

```apl
42⍨0               ⍝ 42
```
