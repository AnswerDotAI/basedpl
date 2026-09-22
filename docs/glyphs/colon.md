# `:` — Unkey / Key / Guard

`:Y` removes axis keys. `K:Y` labels leading axes. `K:[axes]Y` labels selected axes. See [axis keys](../keyed.md).

```apl
:('aa' 'bb':1 2)    ⍝ 1 2
'bb'⊃'aa' 'bb':1 2 ⍝ 2
```

Inside a dfn, group functional uses: `(K:Y)` or `(:Y)`.

`condition:result` returns `result` when the condition is true. Otherwise execution continues. Only the chosen result evaluates.

```apl
{⍵<0:-⍵ ⋄ ⍵}¯4     ⍝ 4
{⍵=0:0 ⋄ ÷⍵}0      ⍝ 0
```

Condition: Boolean singleton. An empty selected result returns no value. See [error guards](error-guard.md).
