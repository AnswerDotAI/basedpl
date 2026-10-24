

# `∨` — Real-imag / Or

Function. Keys: `Alt-9`. Hanzi: `析/或`. Ranks: `0` monadic, `0 0`
dyadic

`∨Y` gives real/imaginary pairs along a new last axis. Y is a numeric
array. A real item’s imaginary part is a zero of the item’s own kind: a
number of its type, or `$f` for a Boolean.

``` bpl
∨3j4              ⍝ 3 4
∨3j4 1j2          ⍝ [3 4⋄1 2]
∨1r3              ⍝ 1r3 0ₓ
∨1=1              ⍝ $t $f
```

`X∨Y` gives GCD; on Booleans, OR. Pervasive.

``` bpl
0 0 1 1∨0 1 0 1   ⍝ 0 1 1 1
12ₓ∨18ₓ            ⍝ 6ₓ
∨/0 0 1            ⍝ 1
```

Includes rational and complex GCD. Inputs are finite. Empty reduction
identity: zero, or `$f` on Booleans.

For approximate real inputs, GCD reconstructs the ratio of their
magnitudes using continued fractions. It chooses the first convergent
equal to that ratio within the [comparison
tolerance](../system/prefs.qmd). Integer convergents are computed
exactly. The result is the larger magnitude divided by the convergent’s
numerator. Exact inputs use exact GCD. Complex inputs use
Gaussian-integer Euclidean iteration.

## Inverse

`∨⁻¹Y` rebuilds complex numbers from the real and imaginary pairs on the
last axis of `Y`.

``` bpl
∨⁻¹3 4   ⍝ 3j4
```
