

# Constants and `$` names

Names that start with `$` are built in. All of them are constants except
`$e`, which holds the error a guard caught.

## `$t` and `$f` — True and false

`$t` is true and `$f` is false. Comparisons give them. Wherever a number
is expected, `$t` is 1 and `$f` is 0.
[Booleans](../numbers.qmd#booleans) has more.

``` bpl
3>1 5                ⍝ $t $f
$t+$t                ⍝ 2ₓ
```

## `$n` — NaN

`$n` is the float NaN, the result of an undefined operation such as
`0÷0`. Under `=` it equals nothing, but `≡` treats it as one value.
[Infinities and NaN](../numbers.qmd#infinities-and-nan) has more.

``` bpl
0÷0                  ⍝ $n
$n=$n                ⍝ $f
```

## `$a` — Alphabet

`$a` is the uppercase Latin alphabet.

``` bpl
3↑$a               ⍝ "ABC"
$a⍳"CAB"           ⍝ [2 0 1]ₓ
```

## `$d` — Digits

`$d` is `"0123456789"`.

``` bpl
"a2b"∊$d           ⍝ $f $t $f
```

## `$m` — Machine epsilon

`$m` is the gap between 1 and the next float at the session’s width:
`2*¯52` for 64-bit floats, `2*¯23` for 32-bit floats and `2*¯10` for
16-bit floats. A width mark gives another width, as in `$mₛ`.

``` bpl
$m=2*¯52             ⍝ $t
$mₛ=2*¯23            ⍝ $t
```

A matrix’s rank is the number of its singular values above a cutoff.
NumPy’s cutoff is `$m` times the larger of the matrix’s two lengths
times the largest singular value:

``` bpl
m←[1 2⋄2 4⋄3 6]
s←(["kind":"svd" "vectors":$f]•decompose m).s
+/s>$m×(⌈/⍴m)×⌈/s     ⍝ 1ₓ
```

## `$e` — Caught error

`$e` is the error that an [error guard](../glyphs/error-guard.qmd)’s
handler caught, as a record. The error guard page lists its fields.
Anywhere else, `$e` is a `VALUE` error.
