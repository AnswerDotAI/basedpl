

# `:` — Unkey / Key / Guard

`:Y` removes axis keys. `K:Y` labels leading axes. `K:⍠A Y` labels the
axes `A`. Brackets with any `key:value` item build one keyed vector.
Their other items are entries without keys. See [axis
keys](../keyed.qmd).

``` apl
:("aa" "bb":1 2)    ⍝ 1 2
"bb"⊃"aa" "bb":1 2 ⍝ 2
["aa":1 "bb":2]≡"aa" "bb":1 2 ⍝ 1ₓ
```

In `K`, a position’s own number leaves that position without a key. Axis
names are the keys of the shape. `(N:⍴M)⍴M` renames the axes. See
[shape](rho.qmd).

``` apl
"aa" 1:10 20        ⍝ ["aa":10 20]
M←["city":2 "month":3]⍴⍳6
⍴(:⍴M)⍴M            ⍝ 2ₓ 3ₓ
```

A keyed `K` supplies axis names and position keys together.

``` apl
axes←["city":["NY" "LA"] "month":["Jan" "Feb" "Mar"]]
M←axes:[1 2 3 ⋄ 4 5 6]
"LA" "Feb"⌷M       ⍝ 5
```

Inside a dfn, group functional uses: `(K:Y)` or `(:Y)`.

`condition:result` returns `result` when the condition is true.
Otherwise execution continues. Only the chosen result evaluates.

``` apl
{⍵<0:-⍵ ⋄ ⍵}¯4     ⍝ 4
{⍵=0:0 ⋄ ÷⍵}0      ⍝ 0
```

Condition: Boolean singleton. An empty selected result returns no value.
See [error guards](error-guard.qmd).
