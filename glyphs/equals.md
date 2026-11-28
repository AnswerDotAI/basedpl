

# [`=`](https://answerdotai.github.io/basedpl/glyphs/equals.html) — Classify / Equal

Function. Hanzi: `等`. Ranks:
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html)
monadic, `0 0` dyadic

`=Y` classifies the major cells of `Y`. The result has a row for each
distinct cell, in order of first occurrence, and a column for each cell
of `Y`. An item is `$t` where its column’s cell equals its row’s cell.

``` bpl
="aba"            ⍝ [$t $f $t◇$f $t $f]
```

`X=Y` tests equality, pervasively. NaN equals nothing, as IEEE 754
defines.

``` bpl
1 2 3=2            ⍝ $f $t $f
0.3=0.1+0.2        ⍝ $t
"abc"='b'          ⍝ $f $t $f
```

See [tolerance](../evaluation.qmd#equality-and-ordering) and whole-array
[Match `≡`](equal-underbar.qmd).

## Inverse

On Booleans,
[`=`](https://answerdotai.github.io/basedpl/glyphs/equals.html) is its
own inverse with either argument fixed.

``` bpl
1 0=⁻1 1   ⍝ $t $f
```
