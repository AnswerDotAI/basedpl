# `↕` — Full windows

`S↕Y`: full windows of sizes `S` on leading axes. Trailing axes stay inside each window.

```apl
3↕⍳5                ⍝ 3 3⍴1 2 3 2 3 4 3 4 5
2 2↕2 3⍴⍳6          ⍝ 1 2 2 2⍴1 2 4 5 2 3 5 6
2↕3 2⍴⍳6            ⍝ 2 2 2⍴1 2 3 4 3 4 5 6
```

Shape: position-frame, sizes, trailing axes. Position counts: `0⌈1+((≢S)↑⍴Y)-S`.

```apl
4↕'ab'               ⍝ 0 4⍴' '
0↕'ab'               ⍝ 3 0⍴' '
⍬↕2 3⍴⍳6            ⍝ 2 3⍴⍳6
```

Oversized windows give empty frames. Zero sizes give `n+1` empty windows; `⍬` leaves `Y` unchanged. For padded neighbourhoods, see [Stencil](stencil.md).

RANK: sizes not scalar/vector, or too many axes. DOMAIN: negative or non-integral size.
