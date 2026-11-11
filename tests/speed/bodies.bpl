•prefs ["mathish":$f "kinds":"static" "float":64]
⍝⍝ setup
width←4096 size 32 ◇ count←32 size 4 ◇ n←width×count
x←0.0001×ιwidth ◇ cells←countρ⊂x ◇ seed←0.25+x ◇ blend←{(0.9×α)+(0.1×ω)}
⍝⍝ bodies
⍝ Expected: 2 copies per logical input item: 1.5 for one fused body's input/carry reads and result write, plus 0.5 for body calls and assembly. macOS uses the standard twice-expected budget; Linux/wasm await calibration.
⍝ One seeded right fold with an array carry and a three-operation dfn, not a primitive-equivalent reduction.
seed(blend/)cells
⍝⍝: 4	-	-	instructions macos 44.139	allocated macos 74.022

⍝ The left scan keeps each array-valued state; the completed operator includes all of its output.
seed(blend\)cells
⍝⍝: 4	-	-	instructions macos 44.770	allocated macos 74.066
