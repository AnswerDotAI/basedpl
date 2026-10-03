⍝ The redblack operations that tree cases use. Each case runs in a fresh session, so it loads this file.
_tree←•load "../../../lib/tree.bpl"
_array←•load "../../../lib/array.bpl"
_t←_tree.redblack
put←'∪' _t ⋄ get←'⍎' _t ⋄ rem←'~' _t ⋄ fmt←'⍕' _t ⋄ chk←'?' _t ⋄ vec←'∊' _t ⋄ tree←0↣(put _array.foldl)
