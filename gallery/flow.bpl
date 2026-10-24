⍝ Flow field
⍝ A sum of three sine waves gives every point of the canvas a direction. Each run draws the waves' lengths and phases at random,
⍝ and shows them above the drawing: paste them over the `waves` and `phases` lines to keep a drawing you like. Particles drift
⍝ along the field and draw a short faint line at each step. Their paths build up into a drawing. A particle that leaves the canvas
⍝ starts again somewhere new, as do a random few each step. The drawing is finished after 900 frames.
options←["background":0.97;"fade":0]
place←{500×∨⁻¹¿[⍵ 2]⍴0}
n←1500
p←place n
⎕←"waves←",⍎⁻¹ waves←50+¿3⍴150
⎕←"phases←",⍎⁻¹ phases←100÷⍨¿3⍴628
angle←{[x y]←⍉∨⍵ ⋄ [u v w]←phases+[x y x+y]÷waves ⋄ π 1○u + 2○v + 1○w}
style←0.6 0.1 0.15 0.3 0.12
frame←{
  ⍵≥900?;
  q←p+0.8×○ angle p
  lines←∨p , ∨q ,⍤1 style
  outside←∨/c≠0⌈500⌊c←∨q
  restart←outside∨0.005>¿n⍴0
  p⊢←q×~restart + restart×place n
  ["lines":lines]
}
