⍝⍝ setup
n←257ₓ
cells←{⍵⍴1.0}¨(1ₓ+⍳n-1ₓ),1ₓ
⍝⍝ shapes
⍝ One repeated length among distinct cells should decline cohorts instead of batching every singleton.
{+/⍵}¨cells
⍝⍝: -	-	-	instructions macos 34935.339 linux 26474.222	allocated macos 1622.163 linux 1622.163
