⍝ The splay operations that tree cases use. Each case runs in a fresh session, so it loads this file.
•load "lib/tree.bpl"
put←'∪' splay ⋄ get←'⍎' splay ⋄ rem←'~' splay ⋄ fmt←'⍕' splay ⋄ chk←'?' splay ⋄ vec←'∊' splay ⋄ dep←'≡' splay ⋄ tree←0⍃(put foldl)
