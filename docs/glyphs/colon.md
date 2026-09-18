# `:` — Guard

`condition:result` returns `result` when the condition is true. Otherwise execution continues. Only the chosen result evaluates.

```apl
{⍵<0:-⍵ ⋄ ⍵}¯4     ⍝ 4
{⍵=0:0 ⋄ ÷⍵}0      ⍝ 0
```

Condition: Boolean singleton. An empty selected result returns no value. See [error guards](error-guard.md).
