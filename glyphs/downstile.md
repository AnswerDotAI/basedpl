

# `⌊` — Floor / Minimum

Function. Keys: `Alt-d`. Ranks: `0` monadic, `0 0` dyadic

`⌊Y` rounds down to an exact integer. Pervasive; tolerant near integers.
An infinity or NaN stays a float.

``` bpl
⌊1.8 ¯1.8         ⍝ [1 ¯2]ₓ
⌊3r2              ⍝ 1ₓ
```

Complex floor uses APL’s Gaussian-integer rule.

``` bpl
⌊1.5j0.5          ⍝ 1j1
```

`X⌊Y` takes the real minimum. Pervasive. Beside NaN it gives `Y`’s item.
Beside NaN, `⌊/` and `⌊\` give NaN or the least other item, depending on
the processor. Empty reduction identity: `∞`.

``` bpl
3⌊1 5             ⍝ 1 3
3⌊1 $n 5          ⍝ 1 $n 3
```
