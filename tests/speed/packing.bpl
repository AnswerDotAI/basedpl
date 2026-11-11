⍝⍝ setup
n←10000
•prefs ["batch":$f] ⋄ cells←{⍵+1ₓ}¨n⍴⊂⍳8ₓ ⋄ •prefs ["batch":$t]
⍝⍝ cells
⍝ Count-only: 10,000 tiny independent buffers exposed expensive join planning. Their combined 80,000 items exceed the deferral size, but each cell is cheap to pack.
{+/⍵}¨cells
⍝⍝: -	-	-	instructions macos 555.891 linux 574.41	allocated macos 17.456
