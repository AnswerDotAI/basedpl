

# `π` — Pi

Keys: `Alt-p`. Ranks: `0` monadic, `0 0` dyadic

`πY`: πY. Pervasive.

``` bpl
π1                ⍝ 3.141592653589793
π2                ⍝ 6.283185307179586
```

`XπY`: Xπ/Y. Angles in radians.

``` bpl
1π2               ⍝ 1.5707963267948966
2π3               ⍝ 2.0943951023931953
```

Use [`○`](circle.qmd) to turn an angle into a unit-circle point.

## Inverse

`π⁻¹Y` divides `Y` by π. `Xπ⁻¹Y` is `XπY`. With the right argument
fixed, `(π↢X)⁻¹Y` is `Y×X÷π1`.

``` bpl
π⁻¹π2      ⍝ 2
2π⁻¹4      ⍝ π0.5
(π↢2)⁻¹4   ⍝ 8÷π1
```
