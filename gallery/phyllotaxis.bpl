⍝ Phyllotaxis
⍝ Each seed turns by the golden angle from the one before, as in a sunflower head.
[circle rect]←•element "circle" "rect"
i←⍳700
t←i÷700
golden←π 3-√5
z←50ⱼ50+1.8×(√i)×○i×golden
size←0.4+1.1×t
hue←300×t
seed←{["cx":⍵₀;"cy":⍵₁;"r":⍵₂;"fill":"hsl(",(⍕⍵₃),",80%,60%)"] circle ""}
seeds←(⊂seed)⍤1 (∨z),⍉[size⋄hue]
back←["width":100;"height":100;"fill":"#111"] rect ""
["width":500;"height":500] •svg [back],seeds
