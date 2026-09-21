if exists('b:current_syntax')
  finish
endif

syntax match aplGlyph #[←→⍳⍴≢≡+×÷⌈⌊|*⍟○π√!∧∨⍲⍱~=≠<≤>≥⎕•⍞⍺⍵⍶⍹∇⍢⋄¯∞⍬,⍪⊂⊃⊆∊∪∩⍋⍒↑↓⌽⊖⍉⊤⊥⍎⍕⌷⌹¨/⌿\\⍀⍤∘⌝⍨⍥⍛⍣⍸⍷⊢⊣⌸@⌺⍠?⇄⌾↕ℙ⨸⊛∂˘◶-]#
syntax region aplString oneline start=/'/ skip=/''/ end=/'/
syntax match aplComment /⍝.*$/

highlight default link aplGlyph Operator
highlight default link aplString String
highlight default link aplComment Comment

let b:current_syntax = 'apl'
