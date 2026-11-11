⍝⍝ setup
n←1024ₓ
v←1+0.001×ιn
w←v×1+5e¯15
⍝⍝ copies
⍝ The operands differ by less than Mathish's relative tolerance, not by exact equality.
⍝ These are low-priority cost canaries, not optimization targets.
v=w   ⍝ nρ$t
⍝⍝: -	-	-	instructions macos 38.772 linux 23.104	allocated macos 2.27

⍝ mathish_less —
v<w   ⍝ nρ$f
⍝⍝: -	-	-	instructions macos 38.985 linux 23.205	allocated macos 2.27

