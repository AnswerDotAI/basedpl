

# `⍉` — Transpose

Keys: `Alt-o Backslash`. Ranks: `∞` monadic, `1 ∞` dyadic

`⍉Y` reverses axis order.

``` bpl
⍉2 3⍴⍳6            ⍝ [0 3⋄1 4⋄2 5]
```

In `A⍉Y`, each item of `A` names the result axis for the corresponding
leading axis of `Y`. The axes of `Y` that `A` doesn’t reach fill the
unnamed result axes in order, as in BQN. Together, the named and filled
axes run from 0 with no gaps.

``` bpl
1 0⍉2 3⍴⍳6         ⍝ [0 3⋄1 4⋄2 5]
⍴1⍉2 3 4⍴⍳24       ⍝ [3 2 4]ₓ
```

Repeated labels select diagonals, taking the smallest participating
dimension.

``` bpl
0 0⍉3 3⍴⍳9         ⍝ 0 4 8
```

A left argument of higher rank transposes `Y` once for each row, and
stacks the results.

``` bpl
[0 1⋄1 0]⍉2 3⍴⍳6     ⍝ [[0 1 2⋄3 4 5]⋄[0 3⋄1 4⋄2 5]]
```

## Inverse

`⍉⁻¹Y` is `⍉Y`. `X⍉⁻¹Y` puts the axes of `Y` back in their order before
`X⍉`.

``` bpl
2 0 1⍉⁻¹2 0 1⍉2 3 4⍴⍳24   ⍝ 2 3 4⍴⍳24
⍴2⍉⁻¹2 3 4⍴⍳24            ⍝ [4 2 3]ₓ
```
