if exists('b:current_syntax')
  finish
endif

syntax match aplName /\v[A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*(\.[A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*)*/
syntax match aplSystem /\v•[A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*/
syntax match aplNumber /\v¯?∞|¯?(\d+(\.\d+)?|\.\d+)([eE]¯?\d+)?([jJ]¯?(\d+(\.\d+)?|\.\d+)([eE]¯?\d+)?|[xₓ]|r¯?\d+)?|⍬/
syntax match aplFunction #[?⊣⊢|⌊⌈*⍟○π√!∨∧⍲⍱~+×÷=≠<≤>≥⍳:≡⍸∊∪∩⍷⍋⍒⌷⊤⊥⌹⍎⍕⍴≢,⊂⊆⊃↑↓⍪⌽⊖⍉↕ℙ⨸⊛-]#
syntax match aplOperator #[¨⍨⊸⍤⍠⍥⟜.⌝⌸⍣⇄⌾∂◶@⌺/⌿\\⍀]#
syntax match aplArgument /[⍺⍵⍶⍹∇⍢]/
syntax match aplKeyword /[←→⋄⎕]/
syntax match aplCharacter /'.'/
syntax region aplString oneline start=/"/ skip=/""/ end=/"/
syntax match aplComment /⍝.*$/

highlight default link aplSystem PreProc
highlight default link aplNumber Number
highlight default link aplFunction Function
highlight default link aplOperator Operator
highlight default link aplArgument Special
highlight default link aplKeyword Statement
highlight default link aplCharacter Character
highlight default link aplString String
highlight default link aplComment Comment

let b:current_syntax = 'apl'
