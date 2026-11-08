•prefs ["mathish":$f "seed":0]
⍝⍝ setup
•prefs ["seed":0] ⋄ n←100000 ⋄ v←¿n⍴0 ⋄ w←101
⍝⍝ moving
⍝ Width 101 uses rustymath's prefix/suffix sliding sum (about three additions/output), then bulk division: O(n), without an n×w window buffer.
⍝ Their shared ceiling awaits quiet measurement of the manual form; slow wrappers do not set the budget.
(+/w↕v)÷w
⍝⍝: -	-	-

{(+/⍵)÷≢⍵}⍤1 w↕v   ⍝ (+/w↕v)÷w
⍝⍝: -	-	-

(+/÷≢)⍤1 w↕v   ⍝ (+/w↕v)÷w
⍝⍝: -	-	-
