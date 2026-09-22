# `:` — Unkey / Key / Guard

`:Y` removes axis keys. `K:Y` labels leading axes. `K:[axes]Y` labels selected axes. See [axis keys](../keyed.md).

```apl
:('aa' 'bb':1 2)    ⍝ 1 2
'bb'⊃'aa' 'bb':1 2 ⍝ 2
```

`N:[0]Y` names axes. Each entry is a string, or the axis's own index for unnamed. `:[0]Y` removes axis names. Both retain position keys and share the element buffer.

```apl
M←'city' 'month':[0]2 3⍴⍳6
⍳[0]M              ⍝ 'city' 'month'
⍳[0]('city' ⋄ 2):[0]M ⍝ ('city' ⋄ 2x)
⍳[0](:[0]M)        ⍝ 1x 2x
```

A keyed `K` supplies axis names and position keys together.

```apl
axes←'city' 'month':('London' 'Paris' ⋄ 'Jan' 'Feb' 'Mar')
M←axes:2 3⍴⍳6
M['Paris';'Feb']    ⍝ 5
```

Inside a dfn, group functional uses: `(K:Y)` or `(:Y)`.

`condition:result` returns `result` when the condition is true. Otherwise execution continues. Only the chosen result evaluates.

```apl
{⍵<0:-⍵ ⋄ ⍵}¯4     ⍝ 4
{⍵=0:0 ⋄ ÷⍵}0      ⍝ 0
```

Condition: Boolean singleton. An empty selected result returns no value. See [error guards](error-guard.md).
