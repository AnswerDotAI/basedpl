⍝ The avl operations that tree cases use. Each case runs in a fresh session, so it loads this file.
•load "lib/tree.bpl"
put←'∪' avl ⋄ get←'⍎' avl ⋄ rem←'~' avl ⋄ fmt←'⍕' avl ⋄ chk←'?' avl ⋄ vec←'∊' avl ⋄ tree←0⍃(put foldl)
