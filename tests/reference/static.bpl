•prefs ["kinds":"static"]

⍝⍝ setup
kind←{(•storage ⍵).kind} ⋄ replayed←{∧/(0<⍶ ⍵.programs)≥⍵.programs.event≡¨⊂"replayed"} ⋄ compiled←⊢.compiled replayed ⋄ staged←⊢.blocks replayed ⋄ ok←{∧/,(⍺≠⍺)∨⍺=⍵}

⍝⍝ Static kinds

⍝ — Under static kinds, a result's kind follows from its arguments' kinds: real arguments stay real, complex results stay complex, ⌊ and ÷ give floats, and integers compute at the int width. New floats take 32 bits
{(•storage ⍵).kind}¨[√¯4 9;1j0;0j1×0j1;⌊2.5 3.5;7ₓ÷2ₓ;[1 2]ₓ;○[0]ₓ₁;○0⍴3ₓ]
"float32" "complex64" "complex64" "float32" "float32" "int32" "complex64" "complex64"

⍝ — Static kinds give NaN outside the real domain and wrap integers that don't fit. Exact fractions stay exact
[√¯4 9;⌊2.5 ∞;7ₓ÷2ₓ;2147483647ₓ+1ₓ;2ₓ*100ₓ;1r3+1r6]
[[$n 3] [2 ∞] 3.5 ¯2147483648ₓ 0ₓ 1ᵣ2]

⍝ — Under static kinds, `⍟1+x` computes log1p, and gives NaN below ¯1, as `⍟` does outside the real domain
⍟1+1E¯17 ¯3   ⍝ 1ₑ¯17 $n

⍝ — Under static kinds with 64-bit floats, a weak atom takes the width of the floats beside it, as under dynamic kinds. An integer beside a float takes the float's width. Booleans add as integers
•prefs ["float":64] ⋄ {(•storage ⍵).kind}¨[(32•float 1 2)×2;(32•float 1 2)+0j1;[1 2]ₓ+0.5;[1 2]ₓ+32•float 0.5;$t $f+$t $t]
"float32" "complex64" "float64" "float32" "int32"

⍝ — With the int pref at 64, folds, scans and integer products wrap at 64 bits, where dynamic kinds widen to exact integers of any size. `static-int32.bpl` checks them at 32 bits
•prefs ["int":64] ⋄ [+/[9223372036854775807 1]ₓ +\[9223372036854775807 1]ₓ ×/[4294967296 4294967296]ₓ [9223372036854775807 1]ₓ+.×[1 1]ₓ]
[¯9223372036854775808ₓ [9223372036854775807 ¯9223372036854775808]ₓ 0ₓ ¯9223372036854775808ₓ]

⍝ — With the int pref at 64, decode wraps a value that leaves 64 bits, as repeated `+` and `×` would, and a fixed width wraps at its own width
•prefs ["int":64] ⋄ [10ₓ⊥20⍴9ₓ;31ₓ₄⊥[2 32]⍴(⍳10ₓ)ₓ₄;¯7ₓ⊥30⍴[3 ¯5]ₓ]
[7766279631452241919ₓ;[¯1617581680 ¯1829123632]ₓ₄;2220363030220159130ₓ]

⍝ — Under static kinds, ÷ divides integers as floats on every route: on arrays, in folds and in scans, lanes of one item included
{(•storage ⍵).kind}¨[[6 8]ₓ÷[3 2]ₓ;÷/[6 3]ₓ;÷\[6 3]ₓ;÷/,5ₓ;÷\,5ₓ;÷/[2 1]⍴5ₓ 7ₓ]
"float32" "float32" "float32" "float32" "float32" "float32"

⍝ — A write whose items have another kind gives the whole array the compact kind that holds both, through `@`, indexed assignment and modified assignment. A Boolean written among bytes keeps bytes. Characters among numbers leave the array mixed
x←[0 200 253 255]ₓ₁
a←(255-)@[252<]x
b←x
b.[[0]]←3.5
c←x
c.[[1]]+←0.5
d←x
d.[[0]]←$t
e←x
e.[[0]]←'a'
[a b c d ; kind¨[a;b;c;d;e]]
⍝ =>
[[0 200 2 0] [3.5 200 253 255] [0 200.5 253 255] [1 200 253 255]ₓ₁ ; "float32" "float32" "float32" "uint8" "mixed"]

⍝ — A function gives an atom the kind that it gives a one-item array, under both kinds settings. Each arithmetic function and comparison, with one argument or two, takes fixed integers, floats, Booleans and a weak number beside each. The first seven results come from static kinds, the last seven from dynamic kinds
kd←{(•storage ⍵).kind} ⋄ sd←{((kd ⍺ ⍶ ⍵) ≡ kd ⍺ ⍶ [⍵]) ∧ (kd ⍵ ⍶ ⍺) ≡ kd [⍵] ⍶ ⍺} ⋄ md←{(kd ⍶ ⍵) ≡ kd ⍶ [⍵]} ⋄ same←{∧/[⍺ +sd ⍵ ; ⍺ -sd ⍵ ; ⍺ ×sd ⍵ ; ⍺ ÷sd ⍵ ; ⍺ *sd ⍵ ; ⍺ ⍟sd ⍵ ; ⍺ |sd ⍵ ; ⍺ ⌈sd ⍵ ; ⍺ ⌊sd ⍵ ; ⍺ !sd ⍵ ; ⍺ ∧sd ⍵ ; ⍺ ∨sd ⍵ ; ⍺ =sd ⍵ ; ⍺ ≠sd ⍵ ; ⍺ <sd ⍵ ; ⍺ ≤sd ⍵ ; ⍺ >sd ⍵ ; ⍺ ≥sd ⍵ ; ⍺ √sd ⍵ ; -md ⍵ ; ×md ⍵ ; ÷md ⍵ ; *md ⍵ ; ⍟md ⍵ ; |md ⍵ ; ⌈md ⍵ ; ⌊md ⍵ ; !md ⍵ ; ○md ⍵ ; √md ⍵ ; ⊼md ⍵ ; ⊽md ⍵ ; ≤md ⍵ ; ≥md ⍵]} ⋄ sk←[3ₓ₂ same 4ₓ₂ ; 3ₓ₂ same 4ₓ₄ ; 3ₓ same 4ₓ₁ ; 1.5 same 4ₓ₂ ; $t same $t ; 1.5 same (32•float 2.5) ; 3ₓ₂ same (32•float 2.5)] ⋄ •prefs ["kinds":"dynamic"] ⋄ dk←[3ₓ₂ same 4ₓ₂ ; 3ₓ₂ same 4ₓ₄ ; 3ₓ same 4ₓ₁ ; 1.5 same 4ₓ₂ ; $t same $t ; 1.5 same (32•float 2.5) ; 3ₓ₂ same (32•float 2.5)] ⋄ •prefs ["kinds":"static"] ⋄ sk,dk
$t $t $t $t $t $t $t $t $t $t $t $t $t $t

⍝ — A comparison between integers and a whole float gives what comparing them with the integer it stands for gives
j←[¯3 0 3 1000]ₓ ⋄ j=3   ⍝ $f $f $t $f

⍝ — A bare float takes 32 bits, which read 16777217 as 16777216, so a comparison with 16777216 still finds them equal
(16777216+⍳3ₓ)=16777216   ⍝ $t $t $f

⍝ — A residue of 64-bit integers by a whole float is the residue at 32 bits, which read 16777219 as 16777220. Narrower fixed integers give the residues and scans by the integer the float stands for, as floats
[3|[16777219 5]ₓ ; 3|[¯7 ¯1 0 1 7]ₓ₂ ; 7|\[3 10 4 ¯5]ₓ₂]
⍝ =>
[2 2 ; 2 2 0 1 1 ; 3 1 0 ¯5]

⍝ — With the int pref at 64, a 32-bit residue whose quotient passes 2^24 is the exact remainder of the floats, from 0 towards the divisor, where 9007199254740993 reads as 2^53
•prefs ["int":64] ⋄ [3|[9007199254740993]ₓ ; ¯3|[9007199254740993]ₓ ; 3|9007199254740993ₓ]
⍝ =>
[[2] ; [¯1] ; 2]

⍝ — Under static kinds, a function whose results are approximate gives floats for integer arguments, at the width ÷ gives them: ⌹ inverts and solves integer matrices as floats, polar gives an integer's magnitude and angle as floats, and the derivatives of `!` and Lambert W give an atom the floats they give an array. Exact fractions stay exact
[{(•storage ⍵).kind}¨[⌹2 2⍴[2 1 1 3]ₓ;[1 2]ₓ⌹2 2⍴[2 1 1 3]ₓ;(⌹⍤2)2 2 2⍴[2 1 1 3]ₓ;∧[3 ¯4]ₓ;∧3ₓ;{2ₓ!⍵}∂5ₓ;{2ₓ!⍵}∂[5 6]ₓ;{!⍵}∂3ₓ;{!⍵}∂[3]ₓ;(×↢*)⁻¹ 3ₓ;(×↢*)⁻¹ [3]ₓ];⌹1r3]
["float32" "float32" "float32" "float32" "float32" "float32" "float32" "float32" "float32" "float32" "float32";3ₓ]

⍝ — Under static kinds, `∂` computes an integer argument as floats of the width `÷` gives it: the derivative of `-⍵` at `3ₓ` is the float ¯1
[{-⍵}∂3ₓ ; {+/-⍵}∂[2 3]ₓ ; {⍵*3ₓ}∂2ₓ ; •float {⍵}∂3ₓ ; •float {⍵}∂$t]
[¯1 ; ¯1 ¯1 ; 12 ; 32ₓ ; 32ₓ]

⍝ — Under static kinds, folds by - and ÷ trace too, a lane with a zero item included, and give what ∂ gives by calling - or ÷ on each item. A fold by a comparison or by ∨ has no tangent. No trace stops
x←1 2 3 4.5 ⋄ y←0 4 1 8 ⋄ stops←{≢(•explain ⍵).stopped.reason}¨"{-/⍵}∂x" "{÷/⍵}∂y" "{</⍵}∂x" "{∨/⍵}∂x" ⋄ [{-/⍵}∂x ; {÷/⍵}∂2 4 1 8 ; {÷/⍵}∂y ; {</⍵}∂1 2 3 ; {∨/⍵}∂1 2 3.5 ; stops]
[1 ¯1 1 ¯1 ; 0.03125 ¯0.015625 0.0625 ¯0.0078125 ; 0.03125 0 0 0 ; 0 0 0 ; 0 0 0 ; [0 0 0 0]ₓ]

⍝ — Under static kinds, roll gives integers for integer bounds, so a bound of 0, which rolls a float, is a DOMAIN error
¿[0 3]ₓ
⍝ error: DOMAIN ERROR

⍝ — Under static kinds, a float bound of 0 still rolls a float
(•storage ¿0 0).kind
"float32"

⍝ — Under static kinds with the int pref at 64, a call on ordinary integers defers, because its kind follows from its arguments' kinds. Its group wraps at the `int` width, as the interpreter does
•prefs ["int":64] ⋄ jv←4611686018427387900ₓ+⍳300000ₓ ⋄ r←jv+jv×2ₓ ⋄ e←•explain "jv+jv×2ₓ" ⋄ [(•storage r).pending ; (•storage r).kind ; (1000↑r)≡(1000↑jv)+(1000↑jv)×2ₓ ; e.fused.glyphs ; ∧/0>1000↑r]
[$t "int64" $t ["+×+"] $t]

⍝ — Under static kinds, a fixed mark after a pending float chain joins the chain's group as one more call. The group stores the narrow result alone, with the interpreter's values
x←¿300000⍴0 ⋄ y←⌊ₓ₁ x×255 ⋄ e←•explain "⌊ₓ₁ x×255" ⋄ [(•storage y).pending ; (1000↑y)≡⌊ₓ₁ (1000↑x)×255 ; e.fused.glyphs ; (•storage e.value).kind]
[$t $t ["×⌊"] "uint8"]

⍝ — Under static kinds, floor and ceiling give integers at a fixed width or at the `int` width unchanged. No group computes them
um←(300000⍴⍳200ₓ)ₓ₁ ⋄ jv←300000⍴⍳200ₓ ⋄ g←∊(•explain "2×⌈um").fused.glyphs,(•explain "1+⌊jv").fused.glyphs ⋄ [~∨/g∊"⌊⌈" ; (⌊um)≡um ; (⌈jv)≡jv]
[$t $t $t]

⍝ — `∂` of a dfn under static kinds runs from a program that holds the whole call, its value and its derivative together. The first call with given argument types and shapes builds the program by tracing the call, later calls replay it, and both give what `∂` of a train gives, which runs without a program. Each of the three calls runs from a program
f←{+/⍺+⍵×⍵} ⋄ i←+/∘(⊣+⊢×⊢) ⋄ e←•explain "[3 f∂ 1.5 2.5 ; 3 f∂ 1.5 2.5 ; 3ₓ f∂ [1 2]ₓ]" ⋄ [e.value ; e.value≡[3 i∂ 1.5 2.5 ; 3 i∂ 1.5 2.5 ; 3ₓ i∂ [1 2]ₓ] ; ≢e.programs.event]
[[3 5 ; 3 5 ; 2 4] ; $t ; 3ₓ]

⍝ — A program's logical functions on Booleans run on the Boolean table's kernels, as the interpreter's do. The mask that `∨` and `~` give is the gradient of `⍵×` it. Both calls run from the program
g←{+/⍵×(⍵>0.5)∨~⍵>0.25} ⋄ v←0.2 0.4 0.6 ⋄ e←•explain "[g∂v ; g∂v]" ⋄ e.value,≢e.programs.event
[[1 0 1] [1 0 1] 2ₓ]

⍝ — A fold of ordinary integers runs in the program that `∂` builds, whatever the integers' values: under static kinds its type is the `int` width. All four calls run from programs
g←{+/⍵×-/n} ⋄ h←{+/⍵×+/m} ⋄ n←[1 2 3 4]ₓ ⋄ m←[100 200]ₓ ⋄ v←1.5 2.5 ⋄ e←•explain "[g∂v ; g∂v ; h∂v ; h∂v]" ⋄ e.value,≢e.programs.event
[[¯2 ¯2] [¯2 ¯2] [300 300] [300 300] 4ₓ]

⍝ — A scan of Booleans by `+` runs in the program: under static kinds its sums take the `int` width. All four calls run from programs
g←{+/⍵×+\b} ⋄ h←{+/,⍵×+⍀m} ⋄ b←0≠[1 0 1 1]ₓ ⋄ m←0≠[2 3]⍴[1 0 1 1 0 1]ₓ ⋄ v←0.5+⍳4 ⋄ x←[2 3]⍴0.5+⍳6 ⋄ e←•explain "[g∂v ; g∂v ; h∂x ; h∂x]" ⋄ e.value,≢e.programs.event
[[1 1 2 3] [1 1 2 3] [1 0 1⋄2 0 2] [1 0 1⋄2 0 2] 4ₓ]

⍝ — A seeded fold runs in the program, with its seed as the fold's left argument. A 64-bit seed sums bytes at 64 bits, and a weak float seed takes the width of the items' floats. All four calls run from programs
f←{+/⍵×0ₓ₈+/u} ⋄ g←{+/⍵×0.5+/w} ⋄ u←[200 100 50]ₓ₁ ⋄ w←1.5 2.5 ⋄ v←1 2 ⋄ e←•explain "[f∂v ; f∂v ; g∂v ; g∂v]" ⋄ e.value,≢e.programs.event
[[350 350] [350 350] [4.5 4.5] [4.5 4.5] 4ₓ]

⍝ — A seeded fold of a pending array joins its block stage where the stage's compiled part folds at the result's type: by `⌈` or `⌊` with a wider seed, by `+` into the items' width, and by `+` as a count of Booleans with an integer seed. Each total takes the seed as its last item
v←(⍳2*18)ₓ₁ ⋄ e←•explain "[5ₓ₁+/v×2ₓ₁ ; 300ₓ₂⌈/v×2ₓ₁ ; 100ₓ₂⌈/v×2ₓ₁ ; ¯5ₓ₂⌊/v×2ₓ₁ ; 7ₓ₄+/v>100ₓ₁]" ⋄ e.value,⊂e.fused.glyphs
[5ₓ₂ 300ₓ₂ 254ₓ₂ ¯5ₓ₂ 262050ₓ₄ ["×+/" "×⌈/" "×⌈/" "×⌊/" ">+/"]]

⍝ fill-free-name — A fill read from a free name follows the name: a call after the name changes reads the new fill
f←{+/⍵×+/(5↑⍠["fill":z] ⍵)} ⋄ z←10 ⋄ r_1←f∂1 2 3 ⋄ z←20 ⋄ [r_1 ; f∂1 2 3]
[32 32 32 ; 52 52 52]

⍝ — A program's replay reads its dfn's free names again. A name that holds a value of the same type and shape gives its new value, and a name whose value changes type or shape, or a function redefined, makes a new trace
g←{⍵+1} ⋄ f←{+/⍵×(g ⍵×m)} ⋄ m←2 ⋄ a←f∂1.5 2.5 ⋄ m←3 ⋄ b←f∂1.5 2.5 ⋄ m←[1 2]ₓ ⋄ c←f∂1.5 2.5 ⋄ g←{⍵+10} ⋄ [a;b;c;f∂1.5 2.5]
[7 11 ; 10 16 ; 4 11 ; 13 20]

⍝ — A free name that a dfn reads as positions holds its value in the program: every call after the first, while the name holds the same value, replays. A new value, even of the same type and shape, makes a new trace. After three new traces in a row that never replayed, the argument signature runs without a program, with the stopped-trace reason "a free value that changes on every call"
x←0.5×[3 2]⍴⍳6 ⋄ r←[0 0 1]ₓ ⋄ f←{+/,⍵.[r]} ⋄ e←•explain "[f∂x ; f∂x]" ⋄ c←{+/+/⍵.[r]} ⋄ r←[2 2 2]ₓ ⋄ g←c∂x ⋄ r←[1 1 1]ₓ ⋄ h←c∂x ⋄ r←[0 1 2]ₓ ⋄ i←c∂x ⋄ r←[2 1 0]ₓ ⋄ k←•explain "c∂x" ⋄ [e.value ; (≢e.programs.event),∧/1↓e.programs.event≡¨⊂"replayed" ; g ; h ; i ; k.value ; k.stopped.reason]
[[[2 2⋄1 1⋄0 0] [2 2⋄1 1⋄0 0]] [2 1]ₓ [0 0⋄0 0⋄3 3] [0 0⋄3 3⋄0 0] [1 1⋄1 1⋄1 1] [1 1⋄1 1⋄1 1] ["a free value that changes on every call"]]

⍝ — A free name that a program holds as it was, such as a nested array, must hold an identical value at a replay. A value that `=` calls equal but that differs in sign, such as `¯0` for `0`, makes a new trace
n←[(1 2) 0] ⋄ f←{+/⍵÷1⊃n} ⋄ a←f∂1 2 ⋄ n←[(1 2) ¯0] ⋄ [a ; f∂1 2]
[∞ ∞ ; ¯∞ ¯∞]

⍝ — A trace stops at what a program can't hold, such as an effect, and `∂` runs without a program. `•explain` gives the stop a `stopped` row
f←{+/⍵×≢(•storage ⍵).kind} ⋄ e←•explain "f∂1.5 2.5" ⋄ e.value (e.stopped.reason)
[[7 7] ; ,⊂"an effect"]

⍝ — In a training step, `loss∂⍵` builds its program on the first call, which holds the loss and its gradient, and every later call replays it, with no trace stopped. It gives the gradient's closed form to within the last bits that a fold's order can change
X←0.1×[6 3]⍴⍳18 ⋄ y←0.5×⍳6 ⋄ loss←{e←(X+.×⍵)-y ⋄ (+/e×e)÷≢y} ⋄ step←{⍵-0.01×loss∂⍵} ⋄ e←•explain "(step⍣10)3⍴0" ⋄ by←{e←(X+.×⍵)-y ⋄ ⍵-0.01×(2×(⍉X)+.×e)÷≢y} ⋄ w←(by⍣10)3⍴0 ⋄ ((⌈/|e.value-w)≤1e¯5×⌈/|w),(∧/1↓e.programs.event≡¨⊂"replayed"),(≢e.programs.event),≢e.stopped.reason
[1 1 10 0]ₓ

⍝ — A gradient that reads its argument at repeated positions adds the cotangent back at them in one accumulation step of its program, into zeros that the replay makes. Both calls run from programs, and no trace stops
v←0.5×⍳10 ⋄ g←{+/(⍵.[[3 5 3]])*2} ⋄ e←•explain "[g∂v ; g∂v]" ⋄ e.value,(≢e.programs.event),≢e.stopped.reason
[[0 0 0 6 0 5 0 0 0 0] [0 0 0 6 0 5 0 0 0 0] 2ₓ 0ₓ]

⍝ — `+.×⍤2` on stacks of matrices runs in the program as a stacked product, and so does each product that its derivative's transpose makes. Both calls run from programs, no trace stops, and both give the closed form to within the last bits that a product's order can change
x←0.1×2 5 3⍴⍳30 ⋄ w←0.01×2 3 4⍴⍳24 ⋄ f←{+/,(⍵(+.×⍤2)w)*2} ⋄ e←•explain "[f∂x ; f∂x]" ⋄ c←2×(x(+.×⍤2)w)(+.×⍤2)0 2 1⍉w ⋄ (∧/{(⌈/,|⍵-c)≤1e¯5×⌈/,|c}¨e.value),(≢e.programs.event),≢e.stopped.reason
[1 2 0]ₓ

⍝ — A program's first reuse compiles each block stage of elementwise steps on arrays of one shape, and later calls run its compiled parts. A compiled part gives the values that its steps give one at a time. This case and the block-stage cases after it run a body on a free name in the program that `∂` builds: the gradient of `{+/⍵×(f v)}` is `f v` [compiled]
f←{(⍵-0.5)×⍵+0.5} ⋄ v←0.25×⍳8 ⋄ F←{+/⍵×(f v)} ⋄ o←0×v ⋄ e←•explain "[F∂o ; F∂o ; F∂o]" ⋄ w←f v ⋄ (e.value≡[w;w;w]),⌈/e.programs.compiled
[1 1]ₓ

⍝ — A run with a step that doesn't lower to machine operations, such as `*`, still compiles: the steps that lower around it, the comparison `⍵>0.25` among them, become compiled parts, and `*` runs on its precompiled kernel inside the same stage. A comparison's Booleans stay Booleans until a later step reads them as floats [compiled]
f←{(*⍵-0.5)×⍵>0.25} ⋄ v←0.25×⍳8 ⋄ F←{+/⍵×(f v)} ⋄ o←0×v ⋄ e←•explain "[F∂o ; F∂o ; F∂o]" ⋄ w←f v ⋄ (e.value≡[w;w;w]),(⌈/e.programs.blocks),⌈/e.programs.compiled
[1 1 1]ₓ

⍝ — A run of steps that lower compiles where it holds at least two steps or the whole stage. Here `⍵-0.5`, `×` and the gradient's `×` lower, but each stands alone beside a `*`, which doesn't, so the stage compiles nothing and runs each step on its precompiled kernel [compiled]
f←{*(*⍵-0.5)×*⍵} ⋄ v←0.25×⍳8 ⋄ F←{+/⍵×(f v)} ⋄ o←0×v ⋄ e←•explain "[F∂o ; F∂o ; F∂o]" ⋄ w←f v ⋄ (e.value≡[w;w;w]),(⌈/e.programs.blocks),⌈/e.programs.compiled
[1 1 0]ₓ

⍝ — A block stage stores each value that later steps read and folds a value of one axis. It reads an array of another shape through the map that agreement gives it, as `r`'s stage reads the sum of each row that its fold gave. Each gives the interpreter's values
g←{d←*⍵-0.5 ⋄ d÷⌈/d} ⋄ s←{d←⍵×⍵>0.5 ⋄ d-+/d} ⋄ h←{2×*⍵×w} ⋄ r←{*⍵-(+/⍵)÷8} ⋄ w←0.5 1.5 ⋄ v←0.25×⍳8 ⋄ m←[2 4]⍴v ⋄ G←{+/⍵×(g v)} ⋄ S←{+/⍵×(s v)} ⋄ H←{+/,⍵×(h m)} ⋄ R←{+/,⍵×(r m)} ⋄ o←0×v ⋄ p←0×m ⋄ e←•explain "[G∂o ; G∂o ; S∂o ; S∂o ; H∂p ; H∂p ; R∂p ; R∂p]" ⋄ (e.value≡[g v ; g v ; s v ; s v ; h m ; h m ; r m ; r m]),staged e
$t $t

⍝ — With the int pref at 64, a block stage computes integers as the interpreter does: ordinary integers wrap at the `int` width, and fixed integers wrap at their own width. Each replay runs a block stage and gives the interpreter's values, as floats because they're a gradient
•prefs ["int":64] ⋄ f←{⍵+⍵×2ₓ} ⋄ g←{(⍵×2ₓ)+1ₓ} ⋄ v←4611686018427387900ₓ+⍳8ₓ ⋄ u←(13×⍳20ₓ)ₓ₁ ⋄ F←{+/⍵×(f v)} ⋄ G←{+/⍵×(g u)} ⋄ o←8⍴0 ⋄ p←20⍴0 ⋄ e←•explain "[F∂o ; F∂o ; F∂o ; G∂p ; G∂p ; G∂p]" ⋄ (e.value≡1×[f v ; f v ; f v ; g u ; g u ; g u]),staged e
$t $t

⍝ — A block stage holds Booleans as values. Logical functions on Booleans give Booleans, and so does `∧` of two comparisons. A comparison that a float step reads gives it 1 and 0. Each replay runs a block stage and gives the interpreter's values, as floats because they're a gradient
f←{~⍺∧⍵} ⋄ g←{(⍺>0.5)∧⍵>0.5} ⋄ h←{(⍵>3)×⍵} ⋄ a←0≠20⍴[1 0 1 1]ₓ ⋄ b←0≠20⍴[1 1 0 0]ₓ ⋄ x←0.1×⍳20 ⋄ y←⌽x ⋄ F←{+/⍵×(a f b)} ⋄ G←{+/⍵×(x g y)} ⋄ H←{+/⍵×(h x)} ⋄ o←20⍴0 ⋄ e←•explain "[F∂o ; F∂o ; F∂o ; G∂o ; G∂o ; G∂o ; H∂o ; H∂o ; H∂o]" ⋄ (e.value≡1×[a f b ; a f b ; a f b ; x g y ; x g y ; x g y ; h x ; h x ; h x]),staged e
$t $t

⍝ — A size mark in a dfn becomes a conversion step in its program. The step converts as the interpreter does. A fixed mark takes a float's floor and clamps it to the type's range: NaN gives 0, and an infinity gives the nearer end of the range. Each of the nine calls runs from a program, and each replay runs a block stage
f←{⌊ₓ₁ ⍵×255} ⋄ g←{⌊ₓ₄ ⍵×2.5} ⋄ k←{÷ₚ ⍵+1} ⋄ v←0.5 0.25 1.5 ¯0.1 2.9999999999999996 ¯3.5 300.5 $n ∞ ¯∞ ⋄ F←{+/⍵×(f v)} ⋄ G←{+/⍵×(g v)} ⋄ K←{+/⍵×(k v)} ⋄ o←10⍴0 ⋄ e←•explain "[F∂o ; F∂o ; F∂o ; G∂o ; G∂o ; G∂o ; K∂o ; K∂o ; K∂o]" ⋄ (e.value≡1×[f v ; f v ; f v ; g v ; g v ; g v ; k v ; k v ; k v]),(staged e),(≢e.programs.event),⊂(f v)
[$t $t 9ₓ [127 63 255 0 255 0 255 0 255 0]ₓ₁]

⍝ — A product of conversions to wider fixed integers reads the narrow arrays through the conversions, which a program leaves pending for it. Each of the three calls runs from a program and gives the interpreter's result
f←{(⍵ₓ₄)+.×⍵ₓ₄} ⋄ u←(30 30⍴37ₓ×⍳900ₓ)ₓ₁ ⋄ F←{+/,⍵×(f u)} ⋄ o←30 30⍴0 ⋄ e←•explain "[F∂o ; F∂o ; F∂o]" ⋄ w←1×f u ⋄ (e.value≡[w;w;w]),≢e.programs.event
[1 3]ₓ

⍝ — In a compiled part, as in the interpreter, `x⌈y` and `x⌊y` give NaN or the other argument beside NaN, and a zero of either sign for two zeros [compiled]
v←$n 0 ¯0,997⍴1 ⋄ hi←{(--⍵)⌈¯0} ⋄ lo←{(--⍵)⌊0} ⋄ H←{+/⍵×(hi v)} ⋄ L←{+/⍵×(lo v)} ⋄ o←1000⍴0 ⋄ e←•explain "[H∂o ; H∂o ; L∂o ; L∂o]" ⋄ ((⊃3↑¨e.value) ok 0),compiled e
$t $t

⍝ — A compiled part folds negative zeros [compiled]
z←-1000⍴0 ⋄ s←{+/--⍵} ⋄ S←{⍵×(s z)} ⋄ e←•explain "[S∂0 ; S∂0 ; S∂0]" ⋄ e.value,compiled e
0 0 0 1

⍝ — A compiled part folds sums and products of f32 and f64 in registers, within 10⁻⁴ of the interpreter's totals, relative. The later calls of each program run compiled [compiled]
f←{+/(⍵-0.5)×⍵-0.5} ⋄ p←{×/1+1e¯4×⍵-0.5} ⋄ v←0.001×⍳5000 ⋄ w←64•float v ⋄ Fw←{⍵×(f w)} ⋄ Pv←{⍵×(p v)} ⋄ Pw←{⍵×(p w)} ⋄ e←•explain "[Fw∂0 ; Fw∂0 ; Fw∂0 ; Pv∂0 ; Pv∂0 ; Pv∂0 ; Pw∂0 ; Pw∂0 ; Pw∂0]" ⋄ x←[f w ; f w ; f w ; p v ; p v ; p v ; p w ; p w ; p w] ⋄ (∧/(|e.value-x)≤1e¯4×|x),compiled e
$t $t

⍝ — A compiled part folds floats by ⌈ and ⌊ in registers, from ¯∞ and ∞, with the greatest and least items. Beside NaN it gives NaN or the greatest or least of the other items: here the greatest item comes first and NaNs follow it [compiled]
h←{⌈/--⍵} ⋄ l←{⌊/--⍵} ⋄ v←-0.5+⍳5000 ⋄ n←v ⋄ n.[[32 4000]]←$n ⋄ Hv←{⍵×(h v)} ⋄ Lv←{⍵×(l v)} ⋄ Hn←{⍵×(h n)} ⋄ Ln←{⍵×(l n)} ⋄ e←•explain "[Hv∂0 ; Hv∂0 ; Hv∂0 ; Lv∂0 ; Lv∂0 ; Lv∂0 ; Hn∂0 ; Hn∂0 ; Ln∂0 ; Ln∂0]" ⋄ r←e.value ⋄ (∧/(6↑r)=¯0.5 ¯0.5 ¯0.5 ¯4999.5 ¯4999.5 ¯4999.5),(∧/((6↓r)≠6↓r)∨(6↓r)=¯0.5 ¯0.5 ¯4999.5 ¯4999.5),compiled e
$t $t $t

⍝ — A block stage folds fixed integers as the interpreter does: `+` wraps at the items' width, and `⌈` and `⌊` give the greatest and least items. Each fold joins its stage, as a count of a comparison's Booleans does, and gives the interpreter's values, compiled or not [compiled]
s←{+/⍵×3ₓ₁} ⋄ hi←{⌈/⍵+1ₓ₁} ⋄ lo←{⌊/⍵+1ₓ₁} ⋄ c←{+/⍵>100ₓ₁} ⋄ u←(⍳5000ₓ)ₓ₁ ⋄ S←{⍵×(s u)} ⋄ Hi←{⍵×(hi u)} ⋄ Lo←{⍵×(lo u)} ⋄ C←{⍵×(c u)} ⋄ z←32•float 0 ⋄ e←•explain "[S∂z ; S∂z ; S∂z ; Hi∂z ; Hi∂z ; Hi∂z ; Lo∂z ; Lo∂z ; Lo∂z ; C∂z ; C∂z ; C∂z]" ⋄ w←1×[s u ; s u ; s u ; hi u ; hi u ; hi u ; lo u ; lo u ; lo u ; c u ; c u ; c u] ⋄ (e.value≡w),(⌈/e.programs.compiled),⊂∪e.fused.glyphs
[$t 1ₓ ["×+/" "+⌈/" "+⌊/" ">+/"]]

⍝ — A compiled part counts a comparison's Booleans in registers, with the interpreter's counts beside NaN, the infinities and items equal to the comparison's other argument, for f32 and f64 items, and for rows that blocks cross [compiled]
c←{[+/⍵>0.5 ; +/⍵≤0.5 ; +/⍵=0.5 ; +/⍵≠0.5]} ⋄ r←{+/⍵>0.5} ⋄ v←$n ∞ ¯∞ 0.5 0.5 ¯0.5,0.0002×⍳4994 ⋄ f←32•float v ⋄ d←64•float v ⋄ m←2 5000⍴d,d ⋄ Cf←{+/⍵×(c f)} ⋄ Cd←{+/⍵×(c d)} ⋄ Rm←{+/⍵×(r m)} ⋄ o←4⍴0 ⋄ p←2⍴0 ⋄ e←•explain "[Cf∂o ; Cf∂o ; Cf∂o ; Cd∂o ; Cd∂o ; Cd∂o ; Rm∂p ; Rm∂p ; Rm∂p]" ⋄ (e.value≡1×[c f ; c f ; c f ; c d ; c d ; c d ; r m ; r m ; r m]),compiled e
$t $t

⍝ — A comparison of integers computes at the narrowest width that holds both operands, compiled or not, and counts as comparisons of the same values as floats do: two u8 arrays at u8, a u8 array beside 300 or ¯1, which need i16, at i16, and an i16 array at i16 [compiled]
u←(⍳5000ₓ)ₓ₁ ⋄ v←(7ₓ×⍳5000ₓ)ₓ₁ ⋄ i←(¯2500ₓ+⍳5000ₓ)ₓ₂ ⋄ c←{[+/⍵>v ; +/⍵>300 ; +/⍵≥¯1 ; +/⍵<¯1ₓ ; +/⍵≤127]} ⋄ d←{[+/⍵>127 ; +/⍵≤¯300ₓ ; +/⍵=0ₓ₁]} ⋄ Cu←{+/⍵×(c u)} ⋄ Di←{+/⍵×(d i)} ⋄ o←5⍴0 ⋄ p←3⍴0 ⋄ e←•explain "[Cu∂o ; Cu∂o ; Cu∂o ; Di∂p ; Di∂p ; Di∂p]" ⋄ x←1×u ⋄ y←1×v ⋄ j←1×i ⋄ f←[+/x>y ; +/x>300 ; +/x≥¯1 ; +/x<¯1 ; +/x≤127] ⋄ g←[+/j>127 ; +/j≤¯300 ; +/j=0] ⋄ (e.value≡1×[f;f;f;g;g;g]),compiled e
$t $t

⍝ — A stage whose steps don't all lower compiles each run of steps that do, and runs the others on their precompiled kernels: here `3|⍵` runs precompiled, and the steps after it run as one compiled part, with the interpreter's values [compiled]
k←{((⍵-0.5)×⍵+0.5)+3|⍵} ⋄ v←0.001×⍳5000 ⋄ K←{+/⍵×(k v)} ⋄ o←0×v ⋄ e←•explain "[K∂o ; K∂o ; K∂o]" ⋄ w←k v ⋄ (e.value≡[w;w;w]),compiled e
$t $t

⍝ — An ordinary integer is a fixed integer of the int width. A fixed integer of that width displays with plain ₓ, a weak literal beside bytes takes their width, plain ₓ gives the int width, and integer ÷ₓ wraps where its quotient doesn't fit
u←[100 200]ₓ₁ ⋄ [⍕[1 2]ₓ₄+1ₓ ; u+1ₓ ; (uₓ)+200ₓ ; ¯2147483648ₓ÷ₓ¯1ₓ]   ⍝ ["[2 3]ₓ" [101 201]ₓ₁ [300 400]ₓ ¯2147483648ₓ]

⍝ — •csv and •json read integers at the int width, whatever their values, as every new integer does
nl←•ucs 10 ⋄ (kind (•csv "a",nl,"7").a) (kind (•json "{""a"":[1,2]}").a)   ⍝ "int32" "int32"

⍝ — A column value that doesn't fit the int width is a DOMAIN error, which names the column and the types option
•csv 'a',(•ucs 10),"3000000000"
⍝ error: DOMAIN ERROR

⍝ — An array that a JSON field holds reads the same way
•json "{""a"":[3000000000]}"
⍝ error: DOMAIN ERROR

⍝ — A dfn on arrays with axis names or keys runs in the program, and its results keep the labels that the interpreter gives them. A fold by an axis name runs there too
d←[3 4]⍴0.1×⍳12 ⋄ dn←["r":3 "c":4]⍴d ⋄ dk←["a" "b" "c" "d"]:⍤1 d ⋄ g←{+⌿(⍵-0.5)×⍵+0.5} ⋄ f←{+/⍠"r" ⍵} ⋄ Gn←{+/⍵×(g dn)} ⋄ Gk←{+/⍵×(g dk)} ⋄ Fn←{+/⍵×(f dn)} ⋄ zn←0×(g dn) ⋄ zk←0×(g dk) ⋄ zf←0×(f dn) ⋄ e←•explain "[Gn∂zn ; Gn∂zn ; Gk∂zk ; Gk∂zk ; Fn∂zf ; Fn∂zf]" ⋄ x←[g dn ; g dn ; g dk ; g dk ; f dn ; f dn] ⋄ [6ₓ=≢e.programs.event ; (⍕¨e.value)≡⍕¨x]
$t $t

⍝ — Block stages run a labelled program's steps through its layouts' maps. Where two arrays' keys differ, the program pairs them by the union of the keys, as the interpreter does, and each missing item reads as 0
d←[3 4]⍴0.1×⍳12 ⋄ dn←["r":3 "c":4]⍴d ⋄ dk←["a" "b" "c" "d"]:⍤1 d ⋄ dq←["a" "b" "c" "e"]:⍤1 d ⋄ g←{(⍵-0.5)×⍵+0.5} ⋄ h←{⍺+⍵×2} ⋄ Gk←{+/,⍵×(g dk)} ⋄ Gn←{+/,⍵×(g dn)} ⋄ Hq←{+/,⍵×(dk h dq)} ⋄ zk←0×dk ⋄ zn←0×dn ⋄ zq←0×(dk h dq) ⋄ e←•explain "[Gk∂zk ; Gk∂zk ; Gn∂zn ; Gn∂zn]" ⋄ q←•explain "[Hq∂zq ; Hq∂zq]" ⋄ [staged e ; 2ₓ=≢q.programs.event ; (⍕¨q.value)≡2⍴⊂⍕dk+dq×2]
$t $t $t

⍝ — A step that reads another step of its group through a table or a transpose ends the group, and the next stage reads the stored step through that map, as with permuted keys or named axes in another order
d←[3 4]⍴0.1×⍳12 ⋄ dk←["a" "b" "c" "d"]:⍤1 d ⋄ dp←["d" "c" "b" "a"]:⍤1 d ⋄ dn←["r":3 "c":4]⍴d ⋄ dt←["c":4 "r":3]⍴⍉d ⋄ h←{(⍺+⍵×2)×3} ⋄ Hp←{+/,⍵×(dk h dp)} ⋄ Ht←{+/,⍵×(dn h dt)} ⋄ zp←0×(dk h dp) ⋄ zt←0×(dn h dt) ⋄ p←•explain "[Hp∂zp ; Hp∂zp]" ⋄ t←•explain "[Ht∂zt ; Ht∂zt]" ⋄ [staged p ; staged t ; (⍕¨p.value)≡2⍴⊂⍕(dk+dp×2)×3 ; (⍕¨t.value)≡2⍴⊂⍕(dn+dt×2)×3]
$t $t $t $t

⍝ — A fold along a window axis of at most 8 positions runs as steps of a block stage, which read the windows' source once for each position: through padding by any edge mode, along two window axes, and from a value that an earlier stage stores. Maxima and minima pass over NaN as the interpreter's folds do
f←{+/¯3↕⍵} ⋄ g←{⌈/["size":3 "edge":"wrap"]↕⍵} ⋄ h←{⌊/⌊/3 3↕⍵} ⋄ k←{2×+/3↕⍵+1} ⋄ p←{⌈/₂ ₃[[2 2]⋄[2 2]]↕⍵} ⋄ v←0.25×⍳12 ⋄ w←1.5 $n 2 0.5 $n $n 3 4 2.5 $n 1 6 ⋄ m←[3 4]⍴w ⋄ F←{+/,⍵×(f v)} ⋄ G←{+/,⍵×(g w)} ⋄ H←{+/,⍵×(h m)} ⋄ K←{+/,⍵×(k v)} ⋄ P←{+/,⍵×(p m)} ⋄ zf←0×(f v) ⋄ zg←0×(g w) ⋄ zh←0×(h m) ⋄ zk←0×(k v) ⋄ zp←0×(p m) ⋄ e←•explain "[F∂zf ; F∂zf ; G∂zg ; G∂zg ; H∂zh ; H∂zh ; K∂zk ; K∂zk ; P∂zp ; P∂zp]" ⋄ (e.value≡[f v ; f v ; g w ; g w ; h m ; h m ; k v ; k v ; p m ; p m]),staged e
$t $t

⍝ — A program's key holds its arguments' labels, so arguments with other keys run from another program, with their own keys in the result
d←[3 4]⍴0.1×⍳12 ⋄ dk←["a" "b" "c" "d"]:⍤1 d ⋄ dq←["a" "b" "c" "e"]:⍤1 d ⋄ g←{+/,⍵×⍵} ⋄ e←•explain "[g∂dk ; g∂dq ; g∂dk]" ⋄ [3ₓ=≢e.programs.event ; (⍕¨e.value)≡⍕¨[2×dk ; 2×dq ; 2×dk]]
$t $t
