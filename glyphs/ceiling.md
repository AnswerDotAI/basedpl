

# `⌈` — Ceiling / Maximum

Keys: `Alt-s`. Ranks: `0` monadic, `0 0` dyadic

`⌈Y` rounds up to an exact integer. Pervasive; tolerant near integers.
An infinity or NaN stays a float. Complex ceiling is `-⌊-Y`.

``` bpl
⌈1.2 ¯1.2         ⍝ [2 ¯1]ₓ
⌈3r2              ⍝ 2ₓ
```

`X⌈Y` takes the real maximum. Pervasive. Beside NaN it returns the other
argument, as Rust’s `f64::max` does. Empty reduction identity: `¯∞`.

``` bpl
3⌈1 5             ⍝ 3 5
```
