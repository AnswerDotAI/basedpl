

# [`⊤`](https://answerdotai.github.io/basedpl/glyphs/down-tack.html) — Binary encode / Encode

Function. Keys: `Alt-n`. Hanzi: `编` (`編`). Ranks:
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html)
monadic, `1 0` dyadic

`⊤Y` encodes nonnegative whole numbers in binary, choosing enough digits
for the largest value. Each number’s digits lie along a new last axis.
Zero needs zero digits.

``` bpl
⊤5                ⍝ 1 0 1
⊤[2 5]ₓ           ⍝ [0 1 0◇1 0 1]ₓ
```

`B⊤Y` represents numbers in bases `B`. For a zero base, the digit is the
remaining quotient.

``` bpl
2 2 2 2⊤10         ⍝ 1 0 1 0
[0 60 60]ₓ⊤3661ₓ   ⍝ [1 1 1]ₓ
```

Unit `B`: residues. Vector `B`: a new last axis of digits. The result’s
shape is `ρY , ρB`. Exact arguments give exact results.

``` bpl
9⊤15               ⍝ 6
9⊤⊂15              ⍝ ⊂6
10⊤12 34           ⍝ 2 4
2 2 2⊤3 5          ⍝ [0 1 1◇1 0 1]
```

See [Decode](up-tack.qmd).

## Inverse

`⊤⁻` is
[`⊥`](https://answerdotai.github.io/basedpl/glyphs/up-tack.html).

``` bpl
⊤⁻1 0 1       ⍝ 5
24 60⊤⁻1 30   ⍝ 90
```
