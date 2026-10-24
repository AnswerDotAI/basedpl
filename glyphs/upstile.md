

# `⌈` — Ceiling / Maximum

Function. Keys: `Alt-s`. Hanzi: `上`. Ranks: `0` monadic, `0 0` dyadic

`⌈Y` rounds up to an exact integer. Pervasive; tolerant near integers.
An infinity or NaN stays a float. Complex ceiling is `-⌊-Y`.

``` bpl
⌈1.2 ¯1.2         ⍝ [2 ¯1]ₓ
⌈3r2              ⍝ 2ₓ
```

`X⌈Y` takes the real maximum, and for characters the one with the higher
code point. A character is greater than any number, as
[`⍋`](delta-stile.qmd) orders them. Pervasive. Beside NaN it gives NaN
or the other argument. Which one can vary by platform. A reduction,
scan, window or inner product by `⌈`, or a write by `⌈←` to one position
more than once, gives NaN or the greatest of the items that aren’t NaN,
and never another number. Empty reduction identity: `¯∞`.

``` bpl
3⌈1 5             ⍝ 3 5
"abc"⌈"bab"       ⍝ "bbc"
3⌈'a'             ⍝ 'a'
```
