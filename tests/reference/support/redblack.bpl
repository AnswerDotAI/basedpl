⍝ The redblack operations that tree cases use. Each case runs in a fresh session, so it loads this file.
•load "lib/tree.bpl"
put←'∪' redblack ⋄ get←'⍎' redblack ⋄ rem←'~' redblack ⋄ fmt←'⍕' redblack ⋄ chk←'?' redblack ⋄ vec←'∊' redblack ⋄ tree←0⍃(put foldl)
