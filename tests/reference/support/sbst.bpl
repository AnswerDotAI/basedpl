⍝ The sbst operations that tree cases use. Each case runs in a fresh session, so it loads this file.
•load "lib/tree.bpl"
put←'∪' sbst ⋄ get←'⍎' sbst ⋄ rem←'~' sbst ⋄ fmt←'⍕' sbst ⋄ chk←'?' sbst ⋄ vec←'∊' sbst ⋄ bal←'=' sbst ⋄ tree←0⍃(put foldl)
