

# `:` — Unkey / Keyed

Function. Hanzi: `键/卸` (`鍵/卸`). Ranks: `∞` monadic, `1 ∞` dyadic

As a separate [index](squad.qmd) selector, `:` takes one whole axis:
`[: 1]⌷M` selects column 1.

`:Y` removes the axis names and position keys of `Y`. `:⍠A Y` removes
them from the axes `A` only. `K:Y` labels leading axes. `K:⍠A Y` labels
the axes `A`. Keys on one axis must differ: `"a" "a":1 2` is a DOMAIN
error. A bracket list with any `key:value` item is one keyed vector. The
list’s other items are entries without keys. See [axis
keys](../keyed.ipynb).

``` bpl
:"aa" "bb":1 2    ⍝ 1 2
"bb"⊃"aa" "bb":1 2 ⍝ 2
["aa":1 "bb":2]≡"aa" "bb":1 2 ⍝ $t
```

In `K`, a position’s own number leaves that position without a key. Axis
names are the keys of the shape. `N:ρM ρ M` renames the axes. See
[shape](rho.qmd).

``` bpl
"aa" 1:10 20        ⍝ ["aa":10 20]
M←["city":2 "month":3]ρι6ₓ
ρ "rows" "cols":ρM ρ M   ⍝ ["rows":2 "cols":3]ₓ
```

A keyed `K` supplies axis names and position keys together.

``` bpl
axes←["city":"NY" "LA";"month":"Jan" "Feb" "Mar"]
M←axes:[1 2 3◇4 5 6]
"LA" "Feb"⌷M       ⍝ 5
```

`:` removes both kinds of label. To remove one kind, reshape. `ρY ρ ,Y`
removes the position keys and keeps the axis names. `:ρY ρ Y` removes
the names and keeps the keys, since a [reshape](rho.qmd) to the same
shape keeps the position keys:

``` bpl
M←["city":"NY" "LA";"month":"Jan" "Feb" "Mar"]:[1 2 3◇4 5 6]
ρ:M                 ⍝ [2 3]ₓ
ρ:₀M                ⍝ [2 "month":3]ₓ
ι₀ ρM ρ ,M          ⍝ ["city":2]ρ[0 1]ₓ
ι₀ :ρM ρ M          ⍝ "NY" "LA"
```

## Inverse

`K:⁻Y` removes the labels that `K:` attaches: the keys `K`, which `Y`
must have exactly, and the names of the axes that a keyed `K` labels.
Names and keys that `K:` doesn’t attach stay. Where a single key makes
`K:Y` a record of one entry, `K:⁻` gives back that entry’s value.

``` bpl
"a" "b":⁻["a":1 "b":2]   ⍝ 1 2
"a":⁻["a":5]             ⍝ 5
axes←["city":"NY" "LA";"month":"Jan" "Feb" "Mar"]
ρaxes:⁻axes:[1 2 3◇4 5 6]   ⍝ [2 3]ₓ
L←"a" "b" ◇ Y←["r":2]ρ1 2
ρL:⁻L:Y                     ⍝ ["r":2]ₓ
```
