if exists('b:current_syntax')
  finish
endif

syntax match bplName /\v[A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*(\.[A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*)*/
syntax match bplSystem /\v•[A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*/
syntax match bplNumber /\v¯?∞|¯?(\d+(\.\d+)?|\.\d+)([eEₑ]¯?\d+)?([jJⱼ]¯?(\d+(\.\d+)?|\.\d+)([eEₑ]¯?\d+)?|ₓ|[rᵣ]¯?\d+)?|⍬ₓ?/
syntax match bplNumber /\v([\])])@<=ₓ/
syntax match bplNumber /\v\$[tfn]/
syntax match bplString /\v\$[ad]/
syntax match bplFunction /[+×÷⌈⌊|*⍟○π√!∧∨⊼⊽~=≠<≤>≥⍳⍈:⍴≢≡,⍪⊂⊃⊆∊∪∩⍋⍒↑↓⌽⊖⍉⊤⊥⍎⍕⌷⌹#↕⍭⨸⌻⍸⍷⊢⊣¿-]/
syntax match bplMonadicOperator #[¨⍨⊗⌸∂/⌿\\⍀⁰¹²³⁴⁵⁶⁷⁸⁹⁻ᵀᵘ]#
syntax match bplDyadicOperator /[↣⍤∘⍠⍥↢.⍣⇄⊘⌾⍚@⌺]/
syntax match bplArgument /[⍺⍵⍶⍹∇⍢]/
syntax match bplKeyword /[←⎕⋄?₀₁₂₃₄₅₆₇₈₉₋]/
syntax match bplCharacter /'.\?'/
syntax region bplString oneline start=/"/ skip=/""/ end=/"/
syntax match bplComment /⍝.*$/
syntax match bplComment /\%^#!.*/

highlight default link bplSystem PreProc
highlight default link bplNumber Number
highlight default link bplFunction Function
highlight default link bplMonadicOperator Operator
highlight default link bplDyadicOperator Type
highlight default link bplArgument Special
highlight default link bplKeyword Statement
highlight default link bplCharacter Character
highlight default link bplString String
highlight default link bplComment Comment

let b:current_syntax = 'bpl'
