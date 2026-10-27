⍝ Mandelbrot, 32-bit
⍝ The 64-bit Mandelbrot's zoom, computed with 32-bit floats. These hold about 7 significant digits, so this version zooms to about
⍝ 1,000 times, after 233 frames, before it zooms back out.
centre←¯0.7436439ⱼ0.1318259
g←≤⊽ ⍳250 ÷ 250
grid←, 0ⱼ1×⌽g +⊗ g
step←{
  [z c i I V s]←⍵
  z←c+z×z
  live←~e←256<|z
  I,←e#i
  V,←s+1-2⍟⍟|e#z
  [z c i]←live↣#¨[z c i]
  [z c i I V ≥s]
}
frame←{
  zoom←1.03*233-|233-466|⍵
  c←centre+grid×1.5÷zoom
  limit←⌊200+250×10⍟zoom
  [_ _ _ I V _]←step⍣limit [0×c;c;⍳≢c;⍬;⍬;0]
  v←≢c ⍴ ¯1
  v.[I]←V
  hue←0.35×√0⌈v
  colour←v≥0 × 0.5+0.5×2○hue+⊗0.6 1.6 2.6
  ["image":[250 250 3]⍴colour]
}
