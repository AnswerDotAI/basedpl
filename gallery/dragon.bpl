⍝ Dragon curve
⍝ Folding a strip of paper in half again and again leaves a sequence of left and right turns. Each fold keeps the turns so far,
⍝ adds one turn, then repeats the old turns backwards and flipped. Two scans make the path. The first sums the turns into
⍝ headings. The second sums unit steps into positions.
options←["background":0.05;"fade":0]
turns←{⍵,1,~⌽⍵}⍣13⍬
heading←+\0,≤⊽turns
p←∨0,+\○(π 0.5×heading)
[lo hi]←[⌊⌿p⋄⌈⌿p]
scale←440÷⌈/hi-lo
centre←lo+hi ÷ 2
lines←,⍤2 ↕250+scale×p-⍤1 centre
t←⍳n ÷ n←≢lines
colour←0.5+0.5×2○ π 2×t+⊗0 0.33 0.67
segments←lines,1.2,colour,0.9
frame←{["lines":24↑(24×⍵)↓segments]}
