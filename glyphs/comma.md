

# [`,`](https://answerdotai.github.io/basedpl/glyphs/comma.html) — Ravel / Catenate

Function. Hanzi: `连` (`連`). Ranks:
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html)
monadic, `∞ ∞` dyadic

`,Y` makes a vector of the items. With [axes](quad-colon.qmd), `,⍠K Y`
ravels consecutive axes and `,⍠⍬ Y` appends an axis of length 1.

``` bpl
,[1 2◇3 4]       ⍝ 1 2 3 4
ρ,7                ⍝ [1]ₓ
ρ,₁ ₂ 2 3 4ρι24ₓ    ⍝ [2 12]ₓ
```

Ravel keeps the name and keys of each axis that it leaves unchanged. It
drops the names and keys of the axes that it merges. A vector therefore
keeps its keys. To remove them, use [`:`](colon.qmd), as in `:v`:

``` bpl
v←"aa" "bb":1 2 ◇ ,v              ⍝ ["aa":1 "bb":2]
:v                                ⍝ 1 2
ρ,₁ ₂ ["a":2 "b":3 "c":4]ρι24     ⍝ ["a":2 12]ₓ
```

`X,Y` joins on the last axis. `X,⍠K Y` joins on axis `K`. Other axes
must agree. A unit is repeated to fit, and an argument of one less rank
adds one position along the axis.

``` bpl
1 2,3 4            ⍝ 1 2 3 4
```

There is no laminate. `[x◇y]` has `x` and `y` as its major cells, along
a new leading axis. `x,⁰y` pairs the items of `x` and `y` along a new
last axis.

``` bpl
x←1 2 ◇ y←3 4
[x◇y]            ⍝ [1 2◇3 4]
x,⁰y             ⍝ [1 3◇2 4]
```

## Inverse

`,⁻Y` gives back `Y` when it’s a vector. `X,⁻Y` removes the fixed `X`
from the front of `Y`. `(,↢X)⁻Y` removes `X` from the end of `Y`.

``` bpl
,⁻1 2 3           ⍝ 1 2 3
1 2,⁻1 2 3 4      ⍝ 3 4
,↢3 4 ⁻ 1 2 3 4   ⍝ 1 2
```
