

# `⌊` — Floor / Minimum

Function. Keys: `Alt-d`. Hanzi: `下`. Ranks: `0` monadic, `0 0` dyadic

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

`X⌊Y` takes the real minimum, and for characters the one with the lower
code point. A number is less than any character, as
[`⍋`](delta-stile.qmd) orders them. Pervasive. Beside NaN it gives NaN
or the other argument. Which one can vary by platform. A reduction,
scan, window or inner product by `⌊`, or a write by `⌊←` to one position
more than once, gives NaN or the least of the items that aren’t NaN, `∞`
where there are none, and never another number. Empty reduction
identity: `∞`.

``` bpl
3⌊1 5             ⍝ 1 3
"abc"⌊"bab"       ⍝ "aab"
3⌊'a'             ⍝ 3
```
