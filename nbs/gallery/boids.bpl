⍝ Boids
⍝ Each bird steers by three rules: keep apart from very close neighbours, match the heading of nearby ones, and move towards their
⍝ centre. Positions and velocities are complex numbers. One outer product gives every pair's offset. A matrix product with the
⍝ neighbour matrix averages their velocities and positions.
options←["background":0.96;"fade":0.3]
n←150
p←500×∨⁻¹¿[n 2]⍴0
v←2×○ π 2×¿n⍴0
frame←{
  r←|d←p-⊗p
  count←+/near←(r<40)∧r>0
  [mv mp]←⍉(near+.×⍉[v⋄p])÷1⌈count
  v+←0.03×mv-v + 0.002×(count>0)×mp-p + 1.5×+/(r<20)×d÷1⌈r*2
  turn←○0.3×¯0.5+¿n⍴0
  speed←|v
  v×←turn×(1.5⌈3⌊speed)÷speed
  p⊢←∨⁻¹ 500|∨p+v
  corners←,⍤2 ∨p+(v÷|v)×⊗[11 5×○2.5 5×○¯2.5]
  heading←⊢/∧v
  colour←0.45+0.35×2○heading+⊗0 2.1 4.2
  ["triangles":corners,colour,0.9]
}
