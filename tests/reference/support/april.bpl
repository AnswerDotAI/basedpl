⍝ Helpers from April's dfns demos, for reference cases. Each case runs in a fresh session, so a case that uses one loads this file.
⍝ try and check use tree operations that a tree support file, such as splay.bpl, defines: fmt, get, tree, chk and vec.
lvec←{"fooling around", ⍵, "with barrels", ⍵, "in alleys"} •ucs 10  ⍝ Lines separated by newlines.
try←{⊃ fmt\¨ (get/)traj ⍺,⊂tree(⍵)}  ⍝ Formatted trees after each search of tree ⍵ for keys ⍺.
check←{([1 ⍺]≡2↑chk(⍵))∧(⍳⍺)≡↑¨vec(⍵)}  ⍝ Whether tree ⍵ is valid and holds keys ⍳⍺.
