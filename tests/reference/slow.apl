⍝ april/libraries/dfns/tree/demo.lisp:899 — Splay tree validity after searches
•load 'lib/tree.apl'
put←'∪' splay ⋄ get←'⍎' splay ⋄ rem←'~' splay ⋄ chk←'?' splay ⋄ vec←'∊' splay ⋄ dep←'≡' splay
check←{(1 256≡2↑chk ⍵)∧(⍳256)≡↑¨vec ⍵}
tt←0 put foldl 256?256 ⋄ revt←{↑⌽⍵ get ⍺}
tt←tt revt foldl 256?256
check tt
⍝ =>
1

⍝ april/libraries/dfns/tree/demo.lisp:901 — Splay tree search depths and retrieved keys
•load 'lib/tree.apl'
put←'∪' splay ⋄ get←'⍎' splay ⋄ rem←'~' splay ⋄ chk←'?' splay ⋄ vec←'∊' splay ⋄ dep←'≡' splay
check←{(1 256≡2↑chk ⍵)∧(⍳256)≡↑¨vec ⍵}
tt←0 put foldl 256?256 ⋄ revt←{↑⌽⍵ get ⍺}
tt←tt revt foldl 256?256
keys←16?256 ⋄ d←keys dep¨⊂tt
(check tt)∧(∧/d≥1)∧(∧/d≤256)∧keys≡↑¨keys get¨⊂tt
⍝ =>
1

⍝ april/libraries/dfns/tree/demo.lisp:903 — Splay tree repeated searches
•load 'lib/tree.apl'
put←'∪' splay ⋄ get←'⍎' splay ⋄ rem←'~' splay ⋄ chk←'?' splay ⋄ vec←'∊' splay ⋄ dep←'≡' splay
check←{(1 256≡2↑chk ⍵)∧(⍳256)≡↑¨vec ⍵}
tt←0 put foldl 256?256 ⋄ revt←{↑⌽⍵ get ⍺}
tt←tt revt foldl 256?256
keys←16?256 ⋄ tt←tt revt foldl 16/keys ⋄ d←keys dep¨⊂tt
(check tt)∧(∧/d≥1)∧(∧/d≤256)∧keys≡↑¨keys get¨⊂tt
⍝ =>
1

⍝ april/libraries/dfns/tree/demo.lisp:904 — Splay tree validity after repeated searches
•load 'lib/tree.apl'
put←'∪' splay ⋄ get←'⍎' splay ⋄ rem←'~' splay ⋄ chk←'?' splay ⋄ vec←'∊' splay ⋄ dep←'≡' splay
check←{(1 256≡2↑chk ⍵)∧(⍳256)≡↑¨vec ⍵}
tt←0 put foldl 256?256 ⋄ revt←{↑⌽⍵ get ⍺}
tt←tt revt foldl 256?256
keys←16?256 ⋄ tt←tt revt foldl 16/keys
check tt
⍝ =>
1

⍝ april/libraries/dfns/tree/demo.lisp:905 — Splay tree remove all keys
•load 'lib/tree.apl'
put←'∪' splay ⋄ get←'⍎' splay ⋄ rem←'~' splay ⋄ chk←'?' splay ⋄ vec←'∊' splay ⋄ dep←'≡' splay
check←{(1 256≡2↑chk ⍵)∧(⍳256)≡↑¨vec ⍵}
tt←0 put foldl 256?256 ⋄ revt←{↑⌽⍵ get ⍺}
tt←tt revt foldl 256?256
keys←16?256 ⋄ tt←tt revt foldl 16/keys
tt rem foldl 256?256
⍝ =>
0

