if exists('b:current_syntax')
  finish
endif

syntax match bplName /\v[A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*(\.[A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*)*/
syntax match bplSystem /\v[•系][A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*/
syntax match bplString /\v\$[ad]/
syntax match bplFunction /[+加减減×乘号號÷除⌈上⌊下|绝余絕餘*幂冪⍟对對○圆圓π周√根!阶拣階揀∧极且極∨析或⊼方否⊽双莫雙~非无無=等≠别初別<小≤限退>大≥达进達進⍳位↦至:键卸鍵⍴形≢数异數異≡同深,连連⍪接矩⊂包⊃选合選⊆巢段∊属展屬∪并独併獨∩交重⍋升⍒降↑取↓去拆⌽转轉⊖纵縱⍉翻⊤编編⊥译譯⍎行⍕文⌷索⌹解逆#复複↕窗⍭素⨸因⌻零值⍸处间處間⍷寻群尋⊢右⊣左¿掷抽擲-]/
syntax match bplMonadicOperator #[¨遍⍨己换換⊗表⌸组組⇄优優∂导梯導/总總⌿汇匯\\累⍀续續⁰¹²³⁴⁵⁶⁷⁸⁹⁻ᵀᵘ]#
syntax match bplDyadicOperator /[↣先⍤秩∘承⍠轴軸⍥经經↢后後.内內⍣次止⊘兼@于於⍚择擇⌺邻鄰]/
syntax match bplArgument /[⍺甲⍵乙⍶丙⍹丁∇递遞⍢循]/
syntax match bplKeyword /[←是⎕出入⋄句?若₀₁₂₃₄₅₆₇₈₉₋]/
syntax match bplNumber /\v[¯负負]?[∞永]|[¯负負]?(\d+([.内內]\d+)?|[.内內]\d+)([eEₑ][¯负負]?\d+)?([jJⱼ][¯负負]?(\d+([.内內]\d+)?|[.内內]\d+)([eEₑ][¯负負]?\d+)?|ₓ|[rᵣ][¯负負]?\d+)?|[⍬空]ₓ?/
syntax match bplNumber /\v([\])])@<=ₓ/
syntax match bplNumber /\v\$[tfn]/
syntax match bplCharacter /'.\?'/
syntax region bplString oneline start=/"/ skip=/""/ end=/"/
syntax match bplComment /\v[⍝注].*$/
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
