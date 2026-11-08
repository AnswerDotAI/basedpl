⍝⍝ setup
n←1024ₓ
v←1+0.001×⍳n
w←v×1+5e¯15
⍝⍝ copies
⍝ The operands differ by less than Mathish's relative tolerance, not by exact equality.
⍝ These are low-priority cost canaries, not optimization targets.
v=w   ⍝ n⍴$t
⍝⍝: - - - instructions linux 23.104

v<w   ⍝ n⍴$f
⍝⍝: - - - instructions linux 23.205

