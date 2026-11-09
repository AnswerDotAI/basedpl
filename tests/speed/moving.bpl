•prefs ["mathish":$f "seed":0]
⍝⍝ setup
•prefs ["seed":0] ⋄ n←100000 ⋄ v←¿n⍴0 ⋄ w←101
⍝⍝ moving
⍝ Width 101 uses rustymath's prefix/suffix sliding sum (about three additions/output), then bulk division: O(n), without an n×w window buffer.
⍝ Expected: 6 copy-equivalents for HGW reads/writes, buffer initialization, serial dependencies and bulk division; the shared Linux budget is twice that. macOS/wasm await measurement.
(+/w↕v)÷w
⍝⍝: -	12	-

{(+/⍵)÷≢⍵}⍤1 w↕v   ⍝ (+/w↕v)÷w
⍝⍝: -	12	-

(+/÷≢)⍤1 w↕v   ⍝ (+/w↕v)÷w
⍝⍝: -	12	-
