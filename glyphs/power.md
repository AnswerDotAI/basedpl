

# `⍣` — Iterate / Invert / Fixed point

`f⍣N Y` applies `f` `N` times. `N=0` returns `Y`. In dyadic calls the
left argument stays fixed.

``` apl
(2×⍣3)1            ⍝ 8
2(+⍣3)1            ⍝ 7
```

Negative counts apply the known inverse.

``` apl
(2×⍣¯1)10          ⍝ 5
10(-\⍣¯1)9 7 4     ⍝ 1 2 3
```

Inverses propagate through Atop, dyadic After and Before, bound
arguments, left sections, Each, rank, dyadic Commute and scans.
Primitive inverses include arithmetic bindings, exp/log, circle, axis
permutations, encode/decode, enclosure/mix/split and Where. Square roots
and logs select branches.

`f⍣g Y` iterates until `new g old` is true, returning `new`.

``` apl
1+⍣{⍺>4}0          ⍝ 5
```

A list of counts gives the state for each count, assembled with fill.
Counts `⍳N` give the first `N` states, starting with `Y`. The counts
need a group, because an operator takes one item to its right.

``` apl
2×⍣(⍳5) 1              ⍝ 1 2 4 8 16
1+⍣(-⍳3) 5             ⍝ 5 4 3
(1+⍣3 ¯2 0 3)10        ⍝ 13 8 10 13
{⍵,1}⍣(⍳3) ,2          ⍝ [2 0 0 ⋄ 2 1 0 ⋄ 2 1 1]
⍴({1÷0}⍣(0 2⍴0))"ab"   ⍝ 0ₓ 2ₓ 2ₓ
```

Positive steps run first, then inverse steps from `Y`; repeated counts
reuse states. Empty counts use `Y`’s cell shape/fill directly.

## Fixed point

A count of `∞` runs until the state stops changing, the same test as
`⍣≡`. `¯∞` runs the inverse until it converges. `∞` can sit in a list of
counts. A literal argument needs a group after `∞`, because the literal
run would take it in.

``` apl
n←{0.5×⍵+2÷⍵}
n⍣∞ (1)                ⍝ 1.414213562373095
n⍣(0 ∞)1               ⍝ 1 1.414213562373095
```

## History

The until form keeps every state when its predicate is in a one-item
list: `f⍣[g;]`. The states include `Y` and the state where `g` holds, so
it always takes at least one step.

``` apl
1+⍣[{⍺=3};]0           ⍝ 0 1 2 3
⊢⍣[≡;]4                ⍝ 4 4
n←{0.5×⍵+2÷⍵} ⋄ ≢n⍣[≡;]1   ⍝ 7ₓ
```

See [Inverse pair](inverse-pair.qmd).

## Errors

- `DOMAIN`: a count that is neither an integer nor infinite, non-Boolean
  predicate, unknown inverse
