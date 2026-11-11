•prefs ["kinds":"static"]

⍝⍝ setup
kind←{(•storage ω).kind} ◇ compiled←{∧/(∨/ω.routes.route≡¨⊂"compiled")≥ω.programs.event≡¨⊂"replayed"} ◇ staged←{∧/(0<≢ω.fused.items)≥ω.programs.event≡¨⊂"replayed"} ◇ ok←{∧/,(α≠α)∨α=ω}

⍝⍝ Static kinds

⍝ — Under static kinds, a result's kind follows from its arguments' kinds: real arguments stay real, complex results stay complex, ⌊ gives floats and integer ÷ floors, and integers compute at the int width. New floats take 32 bits
{(•storage ω).kind}¨[√¯4 9;1j0;0j1×0j1;⌊2.5 3.5;7ₓ÷2ₓ;[1 2]ₓ;○[0]ₓ₁;○0ρ3ₓ]
"float32" "complex64" "complex64" "float32" "int32" "int32" "complex64" "complex64"

⍝ — Static kinds give NaN outside the real domain and wrap integers that don't fit. Exact fractions stay exact
[√¯4 9;⌊2.5 ∞;7ₓ÷2ₓ;2147483647ₓ+1ₓ;2ₓ*100ₓ;1r3+1r6]
[[$n 3] [2 ∞] 3ₓ ¯2147483648ₓ 0ₓ 1ᵣ2]

⍝ — `√ₐ` and `⊛ₐ` give the real part where `√` and `⊛` give NaN, as floats of the arguments' width
[√ₐ¯4 9;⊛ₐ¯1 ¯2;kind √ₐ¯4 9]
[[0 3] [0 0.6931472] "float32"]

⍝ — `*ₐ` gives the real part of the principal power where `*` gives NaN. ÷3 rounds up at 32 bits, so `¯8*ₐ÷3` lies just below 1
[¯8*÷3;¯8 ¯4*ₐ[÷3;0.5];kind ¯8*ₐ÷3]
[$n [0.99999994 0] "float32"]

⍝ — `○ₐ` and `○⁻ₐ` give the real part where `○` and `○⁻` give NaN, as floats of the arguments' width
[2○⁻ₐ2;1○⁻ₐ¯2 2;7○⁻ₐ 2;8○ₐ 2;kind 2○⁻ₐ2]
[0 [¯1.5707964 1.5707964] 0.54930615 0 "float32"]

⍝ — A count that doesn't fit the int width saturates to its maximum, as a conversion does. An empty array can have an axis that long
≢[3000000000 0]ρ0   ⍝ 2147483647ₓ

⍝ — So does a length that monadic ρ gives
ρ[3000000000 0]ρ0   ⍝ [2147483647 0]ₓ

⍝ — Under static kinds, `⊛1+x` computes log1p, and gives NaN below ¯1, as `⊛` does outside the real domain
x←1E¯17 ¯3 ◇ ⊛1+x   ⍝ 1ₑ¯17 $n

⍝ — Under static kinds with 64-bit floats, a weak atom takes the width of the floats beside it, as under dynamic kinds. An integer beside a float takes the float's width. Booleans add as integers
•prefs ["float":64] ◇ {(•storage ω).kind}¨[(32•float 1 2)×2;(32•float 1 2)+0j1;[1 2]ₓ+0.5;[1 2]ₓ+32•float 0.5;$t $f+$t $t]
"float32" "complex64" "float64" "float32" "int32"

⍝ — With the int pref at 64, folds, scans and integer products wrap at 64 bits, where dynamic kinds widen to exact integers of any size. `static-int32.bpl` checks them at 32 bits
•prefs ["int":64] ◇ [+/[9223372036854775807 1]ₓ +\[9223372036854775807 1]ₓ ×/[4294967296 4294967296]ₓ [9223372036854775807 1]ₓ+.×[1 1]ₓ]
[¯9223372036854775808ₓ [9223372036854775807 ¯9223372036854775808]ₓ 0ₓ ¯9223372036854775808ₓ]

⍝ — With the int pref at 64, decode wraps a value that leaves 64 bits, as repeated `+` and `×` would, and a fixed width wraps at its own width
•prefs ["int":64] ◇ [10ₓ⊥20ρ9ₓ;31ₓ₄⊥[2 32]ρ(ι10ₓ)ₓ₄;¯7ₓ⊥30ρ[3 ¯5]ₓ]
[7766279631452241919ₓ;[¯1617581680 ¯1829123632]ₓ₄;2220363030220159130ₓ]

⍝ — Under static kinds, integer ÷ floors at the int width on every route: arrays, folds and scans, including one-item lanes
{(•storage ω).kind}¨[[6 8]ₓ÷[3 2]ₓ;÷/[6 3]ₓ;÷\[6 3]ₓ;÷/,5ₓ;÷\,5ₓ;÷/[2 1]ρ5ₓ 7ₓ]
"int32" "int32" "int32" "int32" "int32" "int32"

⍝ — Under static kinds, as under dynamic kinds, an index outside its axis, or one that isn't a whole number, reads the fill through `.[]`, `⌷` and `⊃`
v←[10 20 30]ₓ ◇ 0=[v.[5];5⌷v;[5]⊃v;v.[1.5];v.[¯9]]
$t $t $t $t $t

⍝ — Under static kinds, as under dynamic kinds, a write writes nothing at an invalid index: assignment and modified assignment change only the valid positions
v←[10 20 30]ₓ ◇ v.[5]←1ₓ ◇ w←[10 20 30]ₓ ◇ w.[[0 5]]+←1ₓ ◇ [v;w]≡[[10 20 30]ₓ;[11 20 30]ₓ]
$t

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

⍝ — Results computed apart join to one compact kind, as a write's items do: the cells of a map, through Each batched or one cell at a time, the states that `⍣` keeps for a list of counts, and the cases of a compacting select, through `⍚` and a guard. Characters among numbers leave the result mixed
[{(0<ω)⍚[(1<) (2×)]ω}¨¯1.5 2 3 ; (0<)⍚[(1<) (2×)] ¯1.5 2 3 ; {ω>0?ω;$f}¨¯1.5 2 3 ; {ω×0.5}⍣(ι3) 1ₓ ; kind {ω>0?ω;'a'}¨¯1 2]   ⍝ [[0 4 6] [0 4 6] [0 2 3] [1 0.5 0.25] "mixed"]

⍝ — A function gives an atom the kind that it gives a one-item array, under both kinds settings. Each arithmetic function and comparison, with one argument or two, takes fixed integers, floats, Booleans and a weak number beside each. The first seven results come from static kinds, the last seven from dynamic kinds
kd←{(•storage ω).kind} ◇ sd←{((kd α ⍶ ω) ≡ kd α ⍶ [ω]) ∧ (kd ω ⍶ α) ≡ kd [ω] ⍶ α} ◇ md←{(kd ⍶ ω) ≡ kd ⍶ [ω]} ◇ same←{∧/[α +sd ω ; α -sd ω ; α ×sd ω ; α ÷sd ω ; α *sd ω ; α ⊛sd ω ; α |sd ω ; α ⌈sd ω ; α ⌊sd ω ; α !sd ω ; α ∧sd ω ; α ∨sd ω ; α =sd ω ; α ≠sd ω ; α <sd ω ; α ≤sd ω ; α >sd ω ; α ≥sd ω ; α √sd ω ; -md ω ; ×md ω ; ÷md ω ; *md ω ; ⊛md ω ; |md ω ; ⌈md ω ; ⌊md ω ; !md ω ; ○md ω ; √md ω ; ⊼md ω ; ⊽md ω ; ≤md ω ; ≥md ω]} ◇ sk←[3ₓ₂ same 4ₓ₂ ; 3ₓ₂ same 4ₓ₄ ; 3ₓ same 4ₓ₁ ; 1.5 same 4ₓ₂ ; $t same $t ; 1.5 same (32•float 2.5) ; 3ₓ₂ same (32•float 2.5)] ◇ •prefs ["kinds":"dynamic"] ◇ dk←[3ₓ₂ same 4ₓ₂ ; 3ₓ₂ same 4ₓ₄ ; 3ₓ same 4ₓ₁ ; 1.5 same 4ₓ₂ ; $t same $t ; 1.5 same (32•float 2.5) ; 3ₓ₂ same (32•float 2.5)] ◇ •prefs ["kinds":"static"] ◇ sk,dk
$t $t $t $t $t $t $t $t $t $t $t $t $t $t

⍝ — A comparison between integers and a whole float gives what comparing them with the integer it stands for gives
j←[¯3 0 3 1000]ₓ ◇ j=3   ⍝ $f $f $t $f

⍝ — A bare float takes 32 bits, which read 16777217 as 16777216, so a comparison with 16777216 still finds them equal
(16777216+ι3ₓ)=16777216   ⍝ $t $t $f

⍝ — A residue of 64-bit integers by a whole float is the residue at 32 bits, which read 16777219 as 16777220. Narrower fixed integers give the residues and scans by the integer the float stands for, as floats
[3|[16777219 5]ₓ ; 3|[¯7 ¯1 0 1 7]ₓ₂ ; 7|\[3 10 4 ¯5]ₓ₂]
⍝ =>
[2 2 ; 2 2 0 1 1 ; 3 1 0 ¯5]

⍝ — With the int pref at 64, a 32-bit residue whose quotient passes 2^24 is the exact remainder of the floats, from 0 towards the divisor, where 9007199254740993 reads as 2^53
•prefs ["int":64] ◇ [3|[9007199254740993]ₓ ; ¯3|[9007199254740993]ₓ ; 3|9007199254740993ₓ]
⍝ =>
[[2] ; [¯1] ; 2]

⍝ — Under static kinds, a function whose results are approximate gives floats for integer arguments, at the width ÷ gives them: ⌹ inverts and solves integer matrices as floats, polar gives an integer's magnitude and angle as floats, and the derivatives of `!` and Lambert W give an atom the floats they give an array. Exact fractions stay exact
[{(•storage ω).kind}¨[⌹2 2ρ[2 1 1 3]ₓ;[1 2]ₓ⌹2 2ρ[2 1 1 3]ₓ;(⌹⍤2)2 2 2ρ[2 1 1 3]ₓ;∧[3 ¯4]ₓ;∧3ₓ;{2ₓ!ω}∂5ₓ;{2ₓ!ω}∂[5 6]ₓ;{!ω}∂3ₓ;{!ω}∂[3]ₓ;(×↢*)⁻ 3ₓ;(×↢*)⁻ [3]ₓ];⌹1r3]
["float32" "float32" "float32" "float32" "float32" "float32" "float32" "float32" "float32" "float32" "float32";3ᵣ]

⍝ — Dyadic `○` and `○⁻` give an integer argument the kind that they give a float, at every code from 0 to 8, under both kinds settings: none of their formulas has an integer form
c←0↦8 ◇ k←{x←ω ◇ ({kind ω○x}¨c),{kind ω○⁻x}¨c} ◇ s←(k⁝0ₓ)≡k⁝0 ◇ •prefs ["kinds":"dynamic"] ◇ d←(k⁝0ₓ)≡k⁝0 ◇ •prefs ["kinds":"static"] ◇ s d
$t $t

⍝ — Under static kinds, `∂` computes an integer argument as floats of the width `÷` gives it: the derivative of `-ω` at `3ₓ` is the float ¯1
[{-ω}∂3ₓ ; {+/-ω}∂[2 3]ₓ ; {ω*3ₓ}∂2ₓ ; •float {ω}∂3ₓ ; •float {ω}∂$t]
[¯1 ; ¯1 ¯1 ; 12 ; 32ₓ ; 32ₓ]

⍝ — Under static kinds, folds by - and ÷ trace too, a lane with a zero item included, and give what ∂ gives by calling - or ÷ on each item. A fold by a comparison or by ∨ has no tangent. No trace stops
x←1 2 3 4.5 ◇ y←0 4 1 8 ◇ stops←{≢(•explain ω).stopped.reason}¨"{-/ω}∂x" "{÷/ω}∂y" "{</ω}∂x" "{∨/ω}∂x" ◇ [{-/ω}∂x ; {÷/ω}∂2 4 1 8 ; {÷/ω}∂y ; {</ω}∂1 2 3 ; {∨/ω}∂1 2 3.5 ; stops]
[1 ¯1 1 ¯1 ; 0.03125 ¯0.015625 0.0625 ¯0.0078125 ; 0.03125 0 0 0 ; 0 0 0 ; 0 0 0 ; [0 0 0 0]ₓ]

⍝ — Under static kinds, roll gives integers for integer bounds, so a bound of 0, which rolls a float, is a DOMAIN error
¿[0 3]ₓ
⍝ error: DOMAIN ERROR

⍝ — Under static kinds, a float bound of 0 still rolls a float
(•storage ¿0 0).kind
"float32"

⍝ — Under static kinds with the int pref at 64, a call on ordinary integers defers, because its kind follows from its arguments' kinds. Its group wraps at the `int` width, as the interpreter does
•prefs ["int":64] ◇ jv←4611686018427387900ₓ+ι300000ₓ ◇ r←jv+jv×2ₓ ◇ e←•explain "jv+jv×2ₓ" ◇ [(•storage r).pending ; (•storage r).kind ; (1000↑r)≡(1000↑jv)+(1000↑jv)×2ₓ ; e.fused.glyphs ; ∧/0>1000↑r]
[$t "int64" $t ["+×+"] $t]

⍝ — Under static kinds, a fixed mark after a pending float chain joins the chain's group as one more call. The group stores the narrow result alone, with the interpreter's values
x←¿300000ρ0 ◇ y←⌊ₓ₁ x×255 ◇ e←•explain "⌊ₓ₁ x×255" ◇ [(•storage y).pending ; (1000↑y)≡⌊ₓ₁ (1000↑x)×255 ; e.fused.glyphs ; (•storage e.value).kind]
[$t $t ["×⌊"] "uint8"]

⍝ — Under static kinds, floor and ceiling give integers at a fixed width or at the `int` width unchanged. No group computes them
um←(300000ρι200ₓ)ₓ₁ ◇ jv←300000ρι200ₓ ◇ g←∊(•explain "2×⌈um").fused.glyphs,(•explain "1+⌊jv").fused.glyphs ◇ [~∨/g∊"⌊⌈" ; (⌊um)≡um ; (⌈jv)≡jv]
[$t $t $t]

⍝ — `∂` of a dfn under static kinds runs from a program that holds the whole call, its value and its derivative together. The first call with given argument types and shapes builds the program by tracing the call, later calls replay it, and both give what `∂` of a train gives, which runs without a program. Each of the three calls runs from a program
f←{+/α+ω×ω} ◇ i←+/∘(⊣+⊢×⊢) ◇ e←•explain "[3 f∂ 1.5 2.5 ; 3 f∂ 1.5 2.5 ; 3ₓ f∂ [1 2]ₓ]" ◇ [e.value ; e.value≡[3 i∂ 1.5 2.5 ; 3 i∂ 1.5 2.5 ; 3ₓ i∂ [1 2]ₓ] ; ≢e.programs.event]
[[3 5 ; 3 5 ; 2 4] ; $t ; 3ₓ]

⍝ — A program's logical functions on Booleans run on the Boolean table's kernels, as the interpreter's do. The mask that `∨` and `~` give is the gradient of `ω×` it. Both calls run from the program
g←{+/ω×(ω>0.5)∨~ω>0.25} ◇ v←0.2 0.4 0.6 ◇ e←•explain "[g∂v ; g∂v]" ◇ e.value,≢e.programs.event
[[1 0 1] [1 0 1] 2ₓ]

⍝ — A fold of ordinary integers runs in the program that `∂` builds, whatever the integers' values: under static kinds its type is the `int` width. All four calls run from programs
g←{+/ω×-/n} ◇ h←{+/ω×+/m} ◇ n←[1 2 3 4]ₓ ◇ m←[100 200]ₓ ◇ v←1.5 2.5 ◇ e←•explain "[g∂v ; g∂v ; h∂v ; h∂v]" ◇ e.value,≢e.programs.event
[[¯2 ¯2] [¯2 ¯2] [300 300] [300 300] 4ₓ]

⍝ — A scan of Booleans by `+` runs in the program: under static kinds its sums take the `int` width. All four calls run from programs
g←{+/ω×+\b} ◇ h←{+/,ω×+⍀m} ◇ b←0≠[1 0 1 1]ₓ ◇ m←0≠[2 3]ρ[1 0 1 1 0 1]ₓ ◇ v←0.5+ι4 ◇ x←[2 3]ρ0.5+ι6 ◇ e←•explain "[g∂v ; g∂v ; h∂x ; h∂x]" ◇ e.value,≢e.programs.event
[[1 1 2 3] [1 1 2 3] [1 0 1◇2 0 2] [1 0 1◇2 0 2] 4ₓ]

⍝ — A seeded fold runs in the program, with its seed as the fold's left argument. A 64-bit seed sums bytes at 64 bits, and a weak float seed takes the width of the items' floats. All four calls run from programs
f←{+/ω×0ₓ₈+/u} ◇ g←{+/ω×0.5+/w} ◇ u←[200 100 50]ₓ₁ ◇ w←1.5 2.5 ◇ v←1 2 ◇ e←•explain "[f∂v ; f∂v ; g∂v ; g∂v]" ◇ e.value,≢e.programs.event
[[350 350] [350 350] [4.5 4.5] [4.5 4.5] 4ₓ]

⍝ — A seeded fold of a pending array joins its block stage where the stage's compiled part folds at the result's type: by `⌈` or `⌊` with a wider seed, by `+` into the items' width, and by `+` as a count of Booleans with an integer seed. Each total takes the seed as its last item
v←(ι2*18)ₓ₁ ◇ e←•explain "[5ₓ₁+/v×2ₓ₁ ; 300ₓ₂⌈/v×2ₓ₁ ; 100ₓ₂⌈/v×2ₓ₁ ; ¯5ₓ₂⌊/v×2ₓ₁ ; 7ₓ₄+/v>100ₓ₁]" ◇ e.value,⊂e.fused.glyphs
[5ₓ₂ 300ₓ₂ 254ₓ₂ ¯5ₓ₂ 262050ₓ₄ ["×+/" "×⌈/" "×⌈/" "×⌊/" ">+/"]]

⍝ — A seeded fold along one axis of a keyed pending array keeps the other axis's name and keys, in its block stage as in the interpreter
k←"a" "b":["r":2]ρ0 1 ◇ m←(k+ι["c":2*17])ₓ₁ ◇ 5ₓ₁+/m×2ₓ₁
["r":2]ρ["a":5 "b":3]ₓ₁

⍝ fill-free-name — A fill read from a free name follows the name: a call after the name changes reads the new fill
f←{+/ω×+/(5↑⍠["fill":z] ω)} ◇ z←10 ◇ r_1←f∂1 2 3 ◇ z←20 ◇ [r_1 ; f∂1 2 3]
[32 32 32 ; 52 52 52]

⍝ — A program's replay reads its dfn's free names again. A name that holds a value of the same type and shape gives its new value, and a name whose value changes type or shape, or a function redefined, makes a new trace
g←{ω+1} ◇ f←{+/ω×(g ω×m)} ◇ m←2 ◇ a←f∂1.5 2.5 ◇ m←3 ◇ b←f∂1.5 2.5 ◇ m←[1 2]ₓ ◇ c←f∂1.5 2.5 ◇ g←{ω+10} ◇ [a;b;c;f∂1.5 2.5]
[7 11 ; 10 16 ; 4 11 ; 13 20]

⍝ — A free name that a dfn reads as positions is data, as gather indices are: a replay selects at the name's current positions, so new positions replay rather than trace, and each gives its own values
x←0.5×[3 2]ρι6 ◇ c←{+/+/ω.[r]} ◇ r←[2 2 2]ₓ ◇ g←c∂x ◇ r←[1 1 1]ₓ ◇ h←c∂x ◇ r←[0 1 2]ₓ ◇ e←•explain "c∂x" ◇ [g ; h ; e.value ; e.programs.event]
[[0 0◇0 0◇3 3] [0 0◇3 3◇0 0] [1 1◇1 1◇1 1] ["replayed"]]

⍝ — A free count observation guards a recording by value. Repeated changes without reuse can stop further recording for that argument signature; ordinary differentiation still gives the right value, and •explain reports why it fell back.
x←0.5×ι20 ◇ n←0 ◇ f←{+/n↑ω} ◇ t←{n⊢←ω ◇ f∂x} ◇ j←t¨1↦18 ◇ n←19 ◇ k←•explain "f∂x" ◇ [k.value ; k.stopped.reason]
[1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 0 ; ["a free value that changes on every call"]]

⍝ — A free name that a program holds as it was, such as a nested array, must hold an identical value at a replay. A value that `=` calls equal but that differs in sign, such as `¯0` for `0`, makes a new trace
n←[(1 2) 0] ◇ f←{+/ω÷1⊃n} ◇ a←f∂1 2 ◇ n←[(1 2) ¯0] ◇ [a ; f∂1 2]
[∞ ∞ ; ¯∞ ¯∞]

⍝ — A trace stops at what a program can't hold, such as an effect, and `∂` runs without a program. `•explain` gives the stop a `stopped` row
f←{+/ω×≢(•storage ω).kind} ◇ e←•explain "f∂1.5 2.5" ◇ e.value (e.stopped.reason)
[[7 7] ; ,⊂"an effect"]

⍝ — In a training step, `loss∂ω` builds its program on the first call, which holds the loss and its gradient, and every later call replays it, with no trace stopped. It gives the gradient's closed form to within the last bits that a fold's order can change
X←0.1×[6 3]ρι18 ◇ y←0.5×ι6 ◇ loss←{e←(X+.×ω)-y ◇ (+/e×e)÷≢y} ◇ step←{ω-0.01×loss∂ω} ◇ e←•explain "(step⍣10)3ρ0" ◇ by←{e←(X+.×ω)-y ◇ ω-0.01×(2×(⍉X)+.×e)÷≢y} ◇ w←(by⍣10)3ρ0 ◇ ((⌈/|e.value-w)≤1e¯5×⌈/|w),(∧/1↓e.programs.event≡¨⊂"replayed"),(≢e.programs.event),≢e.stopped.reason
[1 1 10 0]ₓ

⍝ — A gradient that reads its argument at repeated positions adds the cotangent back at them in one accumulation step of its program, into zeros that the replay makes. Both calls run from programs, and no trace stops
v←0.5×ι10 ◇ g←{+/(ω.[[3 5 3]])*2} ◇ e←•explain "[g∂v ; g∂v]" ◇ e.value,(≢e.programs.event),≢e.stopped.reason
[[0 0 0 6 0 5 0 0 0 0] [0 0 0 6 0 5 0 0 0 0] 2ₓ 0ₓ]

⍝ — `+.×⍤2` on stacks of matrices runs in the program as a stacked product, and so does each product that its derivative's transpose makes. Both calls run from programs, no trace stops, and both give the closed form to within the last bits that a product's order can change
x←0.1×2 5 3ρι30 ◇ w←0.01×2 3 4ρι24 ◇ f←{+/,(ω(+.×⍤2)w)*2} ◇ e←•explain "[f∂x ; f∂x]" ◇ c←2×(x(+.×⍤2)w)(+.×⍤2)0 2 1⍉w ◇ (∧/{(⌈/,|ω-c)≤1e¯5×⌈/,|c}¨e.value),(≢e.programs.event),≢e.stopped.reason
[1 2 0]ₓ

⍝ — With compilation requested, a block stage gives the values that its elementwise steps give one at a time. This case and the block-stage cases after it run a body on a free name in the program that `∂` builds: the gradient of `{+/ω×(f v)}` is `f v` [compiled]
f←{(ω-0.5)×ω+0.5} ◇ v←0.25×ι8 ◇ F←{+/ω×(f v)} ◇ o←0×v ◇ e←•explain "[F∂o ; F∂o ; F∂o]" ◇ w←f v ◇ (e.value≡[w;w;w]),1ₓ×compiled e
[1 1]ₓ

⍝ — A run with a step that doesn't lower to machine operations, such as `*`, still compiles: the steps that lower around it, the comparison `ω>0.25` among them, become compiled parts, and `*` runs on its precompiled kernel inside the same stage. A comparison's Booleans stay Booleans until a later step reads them as floats [compiled]
f←{(*ω-0.5)×ω>0.25} ◇ v←0.25×ι8 ◇ F←{+/ω×(f v)} ◇ o←0×v ◇ e←•explain "[F∂o ; F∂o ; F∂o]" ◇ w←f v ◇ (e.value≡[w;w;w]),(1ₓ×staged e),1ₓ×compiled e
[1 1 1]ₓ

⍝ — A run of steps that lower compiles where it holds at least two steps or the whole stage. Here `ω-0.5`, `×` and the gradient's `×` lower, but each stands alone beside a `*`, which doesn't, so the stage compiles nothing and runs each step on its precompiled kernel [compiled]
f←{*(*ω-0.5)×*ω} ◇ v←0.25×ι8 ◇ F←{+/ω×(f v)} ◇ o←0×v ◇ e←•explain "[F∂o ; F∂o ; F∂o]" ◇ w←f v ◇ (e.value≡[w;w;w]),(1ₓ×staged e),1ₓ×compiled e
[1 1 0]ₓ

⍝ — A block stage stores each value that later steps read and folds a value of one axis. It reads an array of another shape through the map that agreement gives it, as `r`'s stage reads the sum of each row that its fold gave. Each gives the interpreter's values
g←{d←*ω-0.5 ◇ d÷⌈/d} ◇ s←{d←ω×ω>0.5 ◇ d-+/d} ◇ h←{2×*ω×w} ◇ r←{*ω-(+/ω)÷8} ◇ w←0.5 1.5 ◇ v←0.25×ι8 ◇ m←[2 4]ρv ◇ G←{+/ω×(g v)} ◇ S←{+/ω×(s v)} ◇ H←{+/,ω×(h m)} ◇ R←{+/,ω×(r m)} ◇ o←0×v ◇ p←0×m ◇ e←•explain "[G∂o ; G∂o ; S∂o ; S∂o ; H∂p ; H∂p ; R∂p ; R∂p]" ◇ (e.value≡[g v ; g v ; s v ; s v ; h m ; h m ; r m ; r m]),staged e
$t $t

⍝ — With the int pref at 64, a block stage computes integers as the interpreter does: ordinary integers wrap at the `int` width, and fixed integers wrap at their own width. Each replay runs a block stage and gives the interpreter's values, as floats because they're a gradient
•prefs ["int":64] ◇ f←{ω+ω×2ₓ} ◇ g←{(ω×2ₓ)+1ₓ} ◇ v←4611686018427387900ₓ+ι8ₓ ◇ u←(13×ι20ₓ)ₓ₁ ◇ F←{+/ω×(f v)} ◇ G←{+/ω×(g u)} ◇ o←8ρ0 ◇ p←20ρ0 ◇ e←•explain "[F∂o ; F∂o ; F∂o ; G∂p ; G∂p ; G∂p]" ◇ (e.value≡1×[f v ; f v ; f v ; g u ; g u ; g u]),staged e
$t $t

⍝ — A block stage holds Booleans as values. Logical functions on Booleans give Booleans, and so does `∧` of two comparisons. A comparison that a float step reads gives it 1 and 0. Each replay runs a block stage and gives the interpreter's values, as floats because they're a gradient
f←{~α∧ω} ◇ g←{(α>0.5)∧ω>0.5} ◇ h←{(ω>3)×ω} ◇ a←0≠20ρ[1 0 1 1]ₓ ◇ b←0≠20ρ[1 1 0 0]ₓ ◇ x←0.1×ι20 ◇ y←⌽x ◇ F←{+/ω×(a f b)} ◇ G←{+/ω×(x g y)} ◇ H←{+/ω×(h x)} ◇ o←20ρ0 ◇ e←•explain "[F∂o ; F∂o ; F∂o ; G∂o ; G∂o ; G∂o ; H∂o ; H∂o ; H∂o]" ◇ (e.value≡1×[a f b ; a f b ; a f b ; x g y ; x g y ; x g y ; h x ; h x ; h x]),staged e
$t $t

⍝ — A size mark in a dfn becomes a conversion step in its program. The step converts as the interpreter does. A fixed mark takes a float's floor and clamps it to the type's range: NaN gives 0, and an infinity gives the nearer end of the range. Each of the nine calls runs from a program, and each replay runs a block stage
f←{⌊ₓ₁ ω×255} ◇ g←{⌊ₓ₄ ω×2.5} ◇ k←{÷ₚ ω+1} ◇ v←0.5 0.25 1.5 ¯0.1 2.9999999999999996 ¯3.5 300.5 $n ∞ ¯∞ ◇ F←{+/ω×(f v)} ◇ G←{+/ω×(g v)} ◇ K←{+/ω×(k v)} ◇ o←10ρ0 ◇ e←•explain "[F∂o ; F∂o ; F∂o ; G∂o ; G∂o ; G∂o ; K∂o ; K∂o ; K∂o]" ◇ (e.value≡1×[f v ; f v ; f v ; g v ; g v ; g v ; k v ; k v ; k v]),(staged e),(≢e.programs.event),⊂(f v)
[$t $t 9ₓ [127 63 255 0 255 0 255 0 255 0]ₓ₁]

⍝ — A product of conversions to wider fixed integers reads the narrow arrays through the conversions, which a program leaves pending for it. Each of the three calls runs from a program and gives the interpreter's result
f←{(ωₓ₄)+.×ωₓ₄} ◇ u←(30 30ρ37ₓ×ι900ₓ)ₓ₁ ◇ F←{+/,ω×(f u)} ◇ o←30 30ρ0 ◇ e←•explain "[F∂o ; F∂o ; F∂o]" ◇ w←1×f u ◇ (e.value≡[w;w;w]),≢e.programs.event
[1 3]ₓ

⍝ — In a compiled part, as in the interpreter, `x⌈y` and `x⌊y` give NaN or the other argument beside NaN, and a zero of either sign for two zeros [compiled]
v←$n 0 ¯0,997ρ1 ◇ hi←{(--ω)⌈¯0} ◇ lo←{(--ω)⌊0} ◇ H←{+/ω×(hi v)} ◇ L←{+/ω×(lo v)} ◇ o←1000ρ0 ◇ e←•explain "[H∂o ; H∂o ; L∂o ; L∂o]" ◇ ((⊃3↑¨e.value) ok 0),compiled e
$t $t

⍝ — A compiled part folds negative zeros [compiled]
z←-1000ρ0 ◇ s←{+/--ω} ◇ S←{ω×(s z)} ◇ e←•explain "[S∂0 ; S∂0 ; S∂0]" ◇ e.value,compiled e
0 0 0 1

⍝ — A compiled part folds sums and products of f32 and f64 in registers, within 10⁻⁴ of the interpreter's totals, relative. The later calls of each program run compiled [compiled]
f←{+/(ω-0.5)×ω-0.5} ◇ p←{×/1+1e¯4×ω-0.5} ◇ v←0.001×ι5000 ◇ w←64•float v ◇ Fw←{ω×(f w)} ◇ Pv←{ω×(p v)} ◇ Pw←{ω×(p w)} ◇ e←•explain "[Fw∂0 ; Fw∂0 ; Fw∂0 ; Pv∂0 ; Pv∂0 ; Pv∂0 ; Pw∂0 ; Pw∂0 ; Pw∂0]" ◇ x←[f w ; f w ; f w ; p v ; p v ; p v ; p w ; p w ; p w] ◇ (∧/(|e.value-x)≤1e¯4×|x),compiled e
$t $t

⍝ — Recorded folds by ⌈ and ⌊ start from ¯∞ and ∞ and give the greatest and least items. Beside NaN they give NaN or the greatest or least remaining item: here the greatest item comes first and NaNs follow it
h←{⌈/--ω} ◇ l←{⌊/--ω} ◇ v←-0.5+ι5000 ◇ n←v ◇ n.[[32 4000]]←$n ◇ Hv←{ω×(h v)} ◇ Lv←{ω×(l v)} ◇ Hn←{ω×(h n)} ◇ Ln←{ω×(l n)} ◇ e←•explain "[Hv∂0 ; Hv∂0 ; Hv∂0 ; Lv∂0 ; Lv∂0 ; Lv∂0 ; Hn∂0 ; Hn∂0 ; Ln∂0 ; Ln∂0]" ◇ r←e.value ◇ (∧/(6↑r)=¯0.5 ¯0.5 ¯0.5 ¯4999.5 ¯4999.5 ¯4999.5),(∧/((6↓r)≠6↓r)∨(6↓r)=¯0.5 ¯0.5 ¯4999.5 ¯4999.5)
$t $t

⍝ — A block stage folds fixed integers as the interpreter does: `+` wraps at the items' width, and `⌈` and `⌊` give the greatest and least items. Each fold joins its stage, as a count of a comparison's Booleans does, and gives the interpreter's values, compiled or not
s←{+/ω×3ₓ₁} ◇ hi←{⌈/ω+1ₓ₁} ◇ lo←{⌊/ω+1ₓ₁} ◇ c←{+/ω>100ₓ₁} ◇ u←(ι5000ₓ)ₓ₁ ◇ S←{ω×(s u)} ◇ Hi←{ω×(hi u)} ◇ Lo←{ω×(lo u)} ◇ C←{ω×(c u)} ◇ z←32•float 0 ◇ e←•explain "[S∂z ; S∂z ; S∂z ; Hi∂z ; Hi∂z ; Hi∂z ; Lo∂z ; Lo∂z ; Lo∂z ; C∂z ; C∂z ; C∂z]" ◇ w←1×[s u ; s u ; s u ; hi u ; hi u ; hi u ; lo u ; lo u ; lo u ; c u ; c u ; c u] ◇ (e.value≡w),⊂∪e.fused.glyphs
[$t ["×+/" "+⌈/" "+⌊/" ">+/"]]

⍝ — A compiled part counts a comparison's Booleans in registers, with the interpreter's counts beside NaN, the infinities and items equal to the comparison's other argument, for f32 and f64 items, and for rows that blocks cross [compiled]
c←{[+/ω>0.5 ; +/ω≤0.5 ; +/ω=0.5 ; +/ω≠0.5]} ◇ r←{+/ω>0.5} ◇ v←$n ∞ ¯∞ 0.5 0.5 ¯0.5,0.0002×ι4994 ◇ f←32•float v ◇ d←64•float v ◇ m←2 5000ρd,d ◇ Cf←{+/ω×(c f)} ◇ Cd←{+/ω×(c d)} ◇ Rm←{+/ω×(r m)} ◇ o←4ρ0 ◇ p←2ρ0 ◇ e←•explain "[Cf∂o ; Cf∂o ; Cf∂o ; Cd∂o ; Cd∂o ; Cd∂o ; Rm∂p ; Rm∂p ; Rm∂p]" ◇ (e.value≡1×[c f ; c f ; c f ; c d ; c d ; c d ; r m ; r m ; r m]),compiled e
$t $t

⍝ — A comparison of integers computes at the narrowest width that holds both operands, compiled or not, and counts as comparisons of the same values as floats do: two u8 arrays at u8, a u8 array beside 300 or ¯1, which need i16, at i16, and an i16 array at i16 [compiled]
u←(ι5000ₓ)ₓ₁ ◇ v←(7ₓ×ι5000ₓ)ₓ₁ ◇ i←(¯2500ₓ+ι5000ₓ)ₓ₂ ◇ c←{[+/ω>v ; +/ω>300 ; +/ω≥¯1 ; +/ω<¯1ₓ ; +/ω≤127]} ◇ d←{[+/ω>127 ; +/ω≤¯300ₓ ; +/ω=0ₓ₁]} ◇ Cu←{+/ω×(c u)} ◇ Di←{+/ω×(d i)} ◇ o←5ρ0 ◇ p←3ρ0 ◇ e←•explain "[Cu∂o ; Cu∂o ; Cu∂o ; Di∂p ; Di∂p ; Di∂p]" ◇ x←1×u ◇ y←1×v ◇ j←1×i ◇ f←[+/x>y ; +/x>300 ; +/x≥¯1 ; +/x<¯1 ; +/x≤127] ◇ g←[+/j>127 ; +/j≤¯300 ; +/j=0] ◇ (e.value≡1×[f;f;f;g;g;g]),compiled e
$t $t

⍝ — A stage whose steps don't all lower compiles each run of steps that do, and runs the others on their precompiled kernels: here `3|ω` runs precompiled, and the steps after it run as one compiled part, with the interpreter's values [compiled]
k←{((ω-0.5)×ω+0.5)+3|ω} ◇ v←0.001×ι5000 ◇ K←{+/ω×(k v)} ◇ o←0×v ◇ e←•explain "[K∂o ; K∂o ; K∂o]" ◇ w←k v ◇ (e.value≡[w;w;w]),compiled e
$t $t

⍝ — An ordinary integer is a fixed integer of the int width. A fixed integer of that width displays with plain ₓ, a weak literal beside bytes takes their width, plain ₓ gives the int width, and integer ÷ₓ wraps where its quotient doesn't fit
u←[100 200]ₓ₁ ◇ [⍕[1 2]ₓ₄+1ₓ ; u+1ₓ ; (uₓ)+200ₓ ; ¯2147483648ₓ÷ₓ¯1ₓ]   ⍝ ["[2 3]ₓ" [101 201]ₓ₁ [300 400]ₓ ¯2147483648ₓ]

⍝ — Weak literal minima do not specialize opaque inputs, and tacit and dfn additions share promotion
f←1ₓ₁+ ◇ g←{1ₓ₁+ω} ◇ u←1ₓ ◇ k←[kind f u;kind g u] ◇ u←256ₓ
[kind 1ₓ₁+1ₓ ; kind 1ₓ₁+256ₓ ; kind [1ₓ 2ₓ] ; k ; f u ; g u]
⍝ =>
["uint8";"int16";"int32";["int32";"int32"];257ₓ;257ₓ]

⍝ — •csv and •json read integers at the int width, whatever their values, as every new integer does
nl←•ucs 10 ◇ (kind (•csv "a",nl,"7").a) (kind (•json "{""a"":[1,2]}").a)   ⍝ "int32" "int32"

⍝ — A column value that doesn't fit the int width is a DOMAIN error, which names the column and the types option
•csv 'a',(•ucs 10),"3000000000"
⍝ error: DOMAIN ERROR

⍝ — An array that a JSON field holds reads the same way
•json "{""a"":[3000000000]}"
⍝ error: DOMAIN ERROR

⍝ — A dfn on arrays with axis names or keys runs in the program, and its results keep the labels that the interpreter gives them. A fold by an axis name runs there too
d←[3 4]ρ0.1×ι12 ◇ dn←["r":3 "c":4]ρd ◇ dk←["a" "b" "c" "d"]:⍤1 d ◇ g←{+⌿(ω-0.5)×ω+0.5} ◇ f←{+/⍠"r" ω} ◇ Gn←{+/ω×(g dn)} ◇ Gk←{+/ω×(g dk)} ◇ Fn←{+/ω×(f dn)} ◇ zn←0×(g dn) ◇ zk←0×(g dk) ◇ zf←0×(f dn) ◇ e←•explain "[Gn∂zn ; Gn∂zn ; Gk∂zk ; Gk∂zk ; Fn∂zf ; Fn∂zf]" ◇ x←[g dn ; g dn ; g dk ; g dk ; f dn ; f dn] ◇ [6ₓ=≢e.programs.event ; (⍕¨e.value)≡⍕¨x]
$t $t

⍝ — Block stages run a labelled program's steps through its layouts' maps. Where two arrays' keys differ, the program pairs them by the union of the keys, as the interpreter does, and each missing item reads as 0
d←[3 4]ρ0.1×ι12 ◇ dn←["r":3 "c":4]ρd ◇ dk←["a" "b" "c" "d"]:⍤1 d ◇ dq←["a" "b" "c" "e"]:⍤1 d ◇ g←{(ω-0.5)×ω+0.5} ◇ h←{α+ω×2} ◇ Gk←{+/,ω×(g dk)} ◇ Gn←{+/,ω×(g dn)} ◇ Hq←{+/,ω×(dk h dq)} ◇ zk←0×dk ◇ zn←0×dn ◇ zq←0×(dk h dq) ◇ e←•explain "[Gk∂zk ; Gk∂zk ; Gn∂zn ; Gn∂zn]" ◇ q←•explain "[Hq∂zq ; Hq∂zq]" ◇ [staged e ; 2ₓ=≢q.programs.event ; (⍕¨q.value)≡2ρ⊂⍕dk+dq×2]
$t $t $t

⍝ — A step that reads another step of its group through a table or a transpose ends the group, and the next stage reads the stored step through that map, as with permuted keys or named axes in another order
d←[3 4]ρ0.1×ι12 ◇ dk←["a" "b" "c" "d"]:⍤1 d ◇ dp←["d" "c" "b" "a"]:⍤1 d ◇ dn←["r":3 "c":4]ρd ◇ dt←["c":4 "r":3]ρ⍉d ◇ h←{(α+ω×2)×3} ◇ Hp←{+/,ω×(dk h dp)} ◇ Ht←{+/,ω×(dn h dt)} ◇ zp←0×(dk h dp) ◇ zt←0×(dn h dt) ◇ p←•explain "[Hp∂zp ; Hp∂zp]" ◇ t←•explain "[Ht∂zt ; Ht∂zt]" ◇ [staged p ; staged t ; (⍕¨p.value)≡2ρ⊂⍕(dk+dp×2)×3 ; (⍕¨t.value)≡2ρ⊂⍕(dn+dt×2)×3]
$t $t $t $t

⍝ — A fold along a window axis of at most 8 positions runs as steps of a block stage, which read the windows' source once for each position: through padding by any edge mode, along two window axes, and from a value that an earlier stage stores. Maxima and minima pass over NaN as the interpreter's folds do
f←{+/¯3↕ω} ◇ g←{⌈/["size":3 "edge":"wrap"]↕ω} ◇ h←{⌊/⌊/3 3↕ω} ◇ k←{2×+/3↕ω+1} ◇ p←{⌈/₂ ₃[[2 2]◇[2 2]]↕ω} ◇ v←0.25×ι12 ◇ w←1.5 $n 2 0.5 $n $n 3 4 2.5 $n 1 6 ◇ m←[3 4]ρw ◇ F←{+/,ω×(f v)} ◇ G←{+/,ω×(g w)} ◇ H←{+/,ω×(h m)} ◇ K←{+/,ω×(k v)} ◇ P←{+/,ω×(p m)} ◇ zf←0×(f v) ◇ zg←0×(g w) ◇ zh←0×(h m) ◇ zk←0×(k v) ◇ zp←0×(p m) ◇ e←•explain "[F∂zf ; F∂zf ; G∂zg ; G∂zg ; H∂zh ; H∂zh ; K∂zk ; K∂zk ; P∂zp ; P∂zp]" ◇ (e.value≡[f v ; f v ; g w ; g w ; h m ; h m ; k v ; k v ; p m ; p m]),staged e
$t $t

⍝ — A program's key holds its arguments' labels, so arguments with other keys run from another program, with their own keys in the result
d←[3 4]ρ0.1×ι12 ◇ dk←["a" "b" "c" "d"]:⍤1 d ◇ dq←["a" "b" "c" "e"]:⍤1 d ◇ g←{+/,ω×ω} ◇ e←•explain "[g∂dk ; g∂dq ; g∂dk]" ◇ [3ₓ=≢e.programs.event ; (⍕¨e.value)≡⍕¨[2×dk ; 2×dq ; 2×dk]]
$t $t

⍝⍝ Constants in derivative graphs

⍝ — Under static kinds, ∂ of a sum with a literal term, a repeated argument or a literal factor traces with no stops
x←1 2 3 4.5
stops←{≢(•explain ω).stopped.reason}¨"{+/ω+1}∂x" "{+/ω+ω}∂x" "{+/ω×2}∂x"
[{+/ω+1}∂x ; {+/ω+ω}∂x ; {+/ω×2}∂x ; stops]
⍝ =>
[1 1 1 1 ; 2 2 2 2 ; 2 2 2 2 ; [0 0 0]ₓ]

⍝ — Under static kinds, ∂ of a fold by - or ÷ on an empty vector gives an empty gradient
e←0ρ0.5
[{-/ω}∂e ; {÷/ω}∂e]
⍝ =>
[0ρ0 ; 0ρ0]

⍝ structural-operator-captures — Axis names and rank vectors held in names remain call-time inputs, even after a body has been recorded
d←["r":2 "c":3]ρι6 ◇ axis←"r" ◇ f←{+/⍠axis ω} ◇ a←f d ◇ axis←"c" ◇ b←f d ◇ m←2 3ρι6 ◇ ranks←0 1 ◇ g←{≢⍤ranks ω} ◇ c←g m ◇ ranks←1 0 ◇ e←g m ◇ [a≡+⌿d ; b≡+/d ; c≡(≢⍤[0 1])m ; e≡(≢⍤[1 0])m]
4ρ$t

⍝ capture-reuse-keeps-recording — Reusing an older program between changes to an observed count keeps recording useful; the final count's derivative can still replay
x←0.5×ι5 ◇ n←1 ◇ f←{+/n↑ω} ◇ t←{n⊢←ω ◇ f∂x} ◇ j←t¨1 2 1 3 1 4 ◇ e←•explain "f∂x" ◇ [e.value ; e.programs.event ; ≢e.stopped.reason]
[1 1 1 1 0 ; ["replayed"] ; 0ₓ]
