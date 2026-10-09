⍝ Mandelbrot
⍝ Each frame zooms 3% further into Seahorse Valley and computes every pixel afresh at 250 × 250. After 400 frames, at about
⍝ 30,000 times, it zooms back out the same way, then starts again. Only the points that haven't escaped yet keep iterating.
centre←¯0.7436438870371587ⱼ0.1318259042053120
g←≤2×(⍳250)÷250
grid←,(0ⱼ1×⌽g)+⊗g
step←{
  [z c i I V s]←⍵
  z←c+z×z
  live←~e←65536<z×+z
  I,←e#i
  V,←s+1-2⍟⍟|e#z
  [z c i]←live↣#¨[z c i]
  [z c i I V ≥s]
}
frame←{
  zoom←1.03*400-|400-800|⍵
  c←centre+grid×1.5÷zoom
  limit←⌊200+250×10⍟zoom
  [_ _ _ I V _]←step⍣limit [0×c;c;⍳≢c;⍬;⍬;0]
  v←(≢c)⍴¯1
  v.[I]←V
  hue←0.35×√0⌈v
  colour←(v≥0)×0.5+0.5×2○hue+⊗0.6 1.6 2.6
  ["image":[250 250 3]⍴colour]
}
