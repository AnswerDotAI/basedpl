•prefs ["kinds":"static" "int":32]

⍝⍝ Static kinds with 32-bit integers

⍝ — The int pref sets the width at which integers compute and wrap
[2147483647ₓ+1ₓ (•storage [1 2]ₓ).kind]
¯2147483648ₓ "int32"

⍝ — Under static kinds with the int pref at 32, folds, scans and inner products wrap at 32 bits, as arithmetic does. ⌈.+ wraps each sum before comparing it
a←[2147483647 5]ₓ ⋄ [+/a +\a -/[¯2147483648 1]ₓ ×/a,2ₓ a+.×[1 1]ₓ a⌈.+[1 1]ₓ (•storage +\a).kind]
[¯2147483644ₓ [2147483647 ¯2147483644]ₓ 2147483647ₓ ¯10ₓ ¯2147483644ₓ 6ₓ "int32"]

⍝ — Decode, polynomial evaluation, the determinant and the permanent of integers wrap at 32 bits, as repeated + and × do
m←2 2⍴[100000 0 0 100000]ₓ ⋄ [2ₓ⊥40⍴1ₓ [0 100000]ₓ⌻100000ₓ -.×m +.×m]
[¯1 1410065408 1410065408 1410065408]ₓ

⍝ — Matrix products of integers wrap at 32 bits, as repeated + and × do, even where the exact sums pass 64 bits
c←6 6⍴2000000000ₓ+⍳36ₓ ⋄ [(c+.×c)≡c{+/⍺×⍵}⍤1⍤1 2⍉c;(•storage c+.×c).kind]
[$t "int32"]

⍝ — Under the int pref at 32, a block stage wraps ordinary integers at 32 bits, as the interpreter does. Each replay of the program that `∂` builds runs a block stage on the body `f v`, whose values the gradient gives as floats
f←{⍵+⍵×2ₓ} ⋄ v←2147483000ₓ+⍳20ₓ ⋄ F←{+/⍵×(f v)} ⋄ o←20⍴0 ⋄ e←•explain "[F∂o ; F∂o ; F∂o]" ⋄ w←f v ⋄ (e.value≡1×[w;w;w]),(∧/0<1↓e.programs.blocks),(⊂(•storage w).kind),⊂¯3↑w
[$t $t "int32" [2147481755 2147481758 2147481761]ₓ]

⍝ — Under the int pref at 32, a fixed integer of 32 bits displays with plain ₓ, and one of 64 bits keeps its suffix
[⍕[1 2]ₓ₄+1ₓ ; ⍕[1 2 3]ₓ₈+1ₓ]   ⍝ "[2 3]ₓ" "[2 3 4]ₓ₈"
