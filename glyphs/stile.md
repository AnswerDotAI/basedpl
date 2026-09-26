

# `|` — Magnitude / Residue

`|Y` gives magnitude. Pervasive.

``` apl
|¯3               ⍝ 3
|3j4              ⍝ 5
```

`X|Y` gives residue modulo `X`. Real residue has the modulus’s sign or
is zero. Pervasive.

``` apl
3|¯1 0 1 4        ⍝ 2 0 1 1
0|7               ⍝ 7
```

Complex residue uses complex floor.
