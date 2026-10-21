if exists('b:current_syntax')
  finish
endif

syntax match bplName /\v%(_\d+|_\S|_|[^\s\x00-\x1f\x7f-\x9f0-9←⎕•⍺⍵⍶⍹∇⍢⍝⋄\?¯∞⍬\+\-×÷⌈⌊\|\*⍟○π√!∧∨⊼⊽\~=≠<≤>≥⍳↦:⍴≢≡,⍪⊂⊃⊆∊∪∩⍋⍒↑↓⌽⊖⍉⊤⊥⍎⍕⌷⌹\#↕⍭⨸⌻⍸⍷⊢⊣¿¨ᵇ⍨↣⍤∘⍠⍥↢\.⊗⌸⍣⇄⊘@∂⍚⌺\/⌿\\⍀⁰¹²³⁴⁵⁶⁷⁸⁹⁻ᵀᵘₓ₀₁₂₃₄₅₆₇₈₉₋\(\)\[\]\{\}'";\$_ₑⱼᵣ])+/
syntax match bplSystem /\v[•][A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*/
syntax match bplString /\v\$[ad]/
syntax match bplFunction /[+×÷⌈⌊|*⍟○π√!∧∨⊼⊽~=≠<≤>≥⍳↦:⍴≢≡,⍪⊂⊃⊆∊∪∩⍋⍒↑↓⌽⊖⍉⊤⊥⍎⍕⌷⌹#↕⍭⨸⌻⍸⍷⊢⊣¿-]/
syntax match bplMonadicOperator #[¨ᵇ⍨⊗⌸⇄∂/⌿\\⍀⁰¹²³⁴⁵⁶⁷⁸⁹⁻ᵀᵘₓ]#
syntax match bplDyadicOperator /[↣⍤∘⍠⍥↢.⍣⊘@⍚⌺]/
syntax match bplArgument /[⍺⍵⍶⍹∇⍢]/
syntax match bplKeyword /[←⎕⋄?₀₁₂₃₄₅₆₇₈₉₋]/
syntax match bplNumber /\v[¯]?[∞]|[¯]?(\d+([.]\d+)?|[.]\d+)([eEₑ][¯]?\d+)?([jJⱼ][¯]?(\d+([.]\d+)?|[.]\d+)([eEₑ][¯]?\d+)?|ₓ|[rᵣ][¯]?\d+)?|[ₑⱼᵣ]|[⍬]ₓ?/
syntax match bplNumber /\v([\])])@<=ₓ/
syntax match bplNumber /\v\$[tfn]/
syntax match bplCharacter /'.\?'/
syntax region bplString oneline start=/"/ skip=/""/ end=/"/
syntax match bplComment /\v⍝.*$/
syntax match bplComment /\%^#!.*/

highlight default link bplName Identifier
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
