⍝ Clifford attractor
⍝ Each run draws four random parameters for the map, and draws again until the points spread out instead of settling on a
⍝ point or a loop. The parameters show above the picture: paste them over the `params` line to keep a drawing you like. Every
⍝ frame moves 3,000 points one step of the map, all at once as array operations. Each point adds a faint dot to the picture, and
⍝ additive blending makes the places the points visit most often glow brightest. The picture stops growing after 1,200 frames.
options←["background":0;"fade":0;"blend":"add"]
map←{[a b c d]←⍺ ⋄ [x y]←⍵
  [(1○a×y)+c×2○a×x
   (1○b×x)+d×2○b×y]}
start←{¯2+4×¿[2 ⍵]⍴0}
spread←{xy←⍵ map⍣300 start 400 ⋄ ≢∪⍉⌊20×1+xy÷1+|2↓⍵}
⎕←"params←",⍎⁻¹ params←{⌊0.5+100×¯2+4×¿4⍴0 ÷ 100}⍣{150≤spread ⍺}⍬
xy←params map⍣100 start 3000
[lo hi]←[⌊/xy⋄⌈/xy]
scale←460÷⌈/hi-lo
centre←lo+hi ÷ 2
glow←2⍴0.012×(≢∪⍉⌊scale×xy÷10)÷830
frame←{
  ⍵≥1200?;
  new←params map xy
  hue←⊢/∧∨⁻¹⍉new-xy
  colour←0.55+0.45×2○hue+⊗0 2.1 4.2 ,⍤1 glow
  xy⊢←new
  pos←⍉250+scale×xy-centre
  ["points":pos,0.8,colour]
}
