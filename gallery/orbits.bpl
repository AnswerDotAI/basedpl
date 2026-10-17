⍝ Orbits
⍝ Dots circle the centre, the inner ones faster, as planets do. A partial fade leaves trails.
options←["background":0;"fade":0.08;"blend":"add"]
n←240
t←⍳n ÷ n
r←30+210×t
start←π t×n×3-√5
speed←0.06×(30÷r)*1.5
[warm cool]←[1 0.6 0.2⋄0.3 0.5 1]
glow←0.9 0
colour←⍉[1-t⋄t] +.× [warm⋄cool]
style←colour ,⍤1 glow
frame←{
  z←250ⱼ250+r×○start+⍵×speed
  ["points":(∨z),3,style]
}
