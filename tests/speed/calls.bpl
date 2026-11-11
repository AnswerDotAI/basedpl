⍝⍝ setup
n←65536 size 1024
v←¿nρ0
⍝ A dfn that writes one item of its argument in place where it holds the argument alone.
step←{[[0]]⌷ω ←0 ◇ ω}
⍝⍝ calls

⍝ A train whose tine is a dfn that writes one item of its argument, in the spelling that the style page prefers to `1+(step ω-0.5)`. The train gives its argument to `step`, which writes in place.
⍝ Expected: 4.6 copies, what `{1+(step ω-0.5)} v` costs on macOS: two passes for `-` and `+`, and `step`'s call, which writes its item in place.
⍝⍝⍝ allows forced "←": an indexed write computes its pending target whole before it writes
{1+step ω-0.5} v
⍝⍝: 9.2	35	9.4	instructions macos 15.067 linux 10.071	allocated macos 16.983 linux 16.983
