⍝ The splay operations that tree cases use, and helpers that search and check splay trees. Each case runs in a fresh session, so it loads this file.
_tree←•load "../../../lib/tree.bpl"
_array←•load "../../../lib/array.bpl"
_power←•load "../../../lib/power.bpl"
_t←_tree.splay
put←'∪' _t ◇ get←'⍎' _t ◇ rem←'~' _t ◇ fmt←'⍕' _t ◇ chk←'?' _t ◇ vec←'∊' _t ◇ dep←'≡' _t ◇ tree←0↣(put _array.foldl)
try←{⊃ fmt\¨ (get/)_power.traj α,⊂tree(ω)}  ⍝ Formatted trees after each search of tree ω for keys α.
check←{([$t α]≡2↑chk(ω))∧(ια)≡↑¨vec(ω)}  ⍝ Whether tree ω is valid and holds keys ια.
