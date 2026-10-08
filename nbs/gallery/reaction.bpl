⍝ Reaction-diffusion
⍝ In the Gray-Scott model, chemical b feeds on chemical a. Both chemicals spread out, and fresh a flows in everywhere. Each run
⍝ places one to four small squares of b at random, and shows their centres above the picture: paste them over the `centres` line
⍝ to keep a pattern you like. From the squares, a pattern grows until it fills the 250 × 250 grid as a labyrinth. Each frame runs
⍝ 60 more steps of the simulation as you watch, and the animation stops after 400 frames.
n←250
centres←40+¿[1+¿4 2]⍴170
⎕←"centres←",⍎⁻¹ centres
seed←∨⌿{(8>|⍵₀-⍳n)∧⊗8>|⍵₁-⍳n}⍤1 centres
lap←{1⌽⍵ + ¯1⌽⍵ + 1⊖⍵ + ¯1⊖⍵ - 4×⍵}
[feed kill]←0.0545 0.062
step←{
  [a b]←⍵
  r←a×b×b
  [a+(0.16×lap a)-r-feed×1-a;b+(0.08×lap b)+r-(feed+kill)×b]
}
ab←[1-0.5×seed;(0.25×seed)+0.02×¿[n n]⍴0]
[dark light]←[0.02 0.03 0.1⋄1 0.75 0.35]
frame←{
  ⍵≥400?;
  ab⊢←step⍣60 ab
  t←0⌈1⌊3×1⊃ab
  picture←2⍉[1-t⋄t] +.× [dark⋄light]
  ["image":picture]
}
