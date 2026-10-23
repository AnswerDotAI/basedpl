⍝⍝ Bitwise lifting

⍝ bitwise-defined — Boolean-valued dfns supply their truth table
6{~⍺∧⍵}ᵇ3   ⍝ ¯3ₓ

⍝ bitwise-implication — A true zero-bit case extends the result's sign bits
6≤ᵇ3   ⍝ ¯5ₓ

⍝ bitwise-constant — A true constant fills every bit, not just the low bit
{1}ᵇ6   ⍝ ¯1ₓ

⍝ bitwise-big — Complement preserves arbitrary integer precision
~ᵇ1208925819614629174706176ₓ   ⍝ ¯1208925819614629174706177ₓ

⍝ bitwise-map — A bitwise call keeps native batching of mapped arguments
6{⍺∧ᵇ⍵}¨3 5   ⍝ [2 4]ₓ

⍝ bitwise-fold — A fold by a lifted primitive applies its table from the right, as other folds do
[∧ᵇ/[12 10 14]ₓ; <ᵇ/[1 2 7]ₓ]   ⍝ [8 4]ₓ

⍝ bitwise-scan — A scan by a lifted primitive accumulates from the left
≠ᵇ\[5 3 6]ₓ   ⍝ [5 6 0]ₓ

⍝ bitwise-boolean-fold — Folds and scans of Booleans by a lifted primitive give Booleans
[∨ᵇ/$f $f $t; ∨ᵇ\$f $t $f]   ⍝ [$t; $f $t $t]

⍝ bitwise-xnor-fold — An XNOR fold of bytes gives negative totals. Booleans fold by the logical function, unless a number seed turns the fold to the integer one
[=ᵇ/[200 3]ₓ; =ᵇ\[200 3 7]ₓ; =ᵇ/$t $t; 5=ᵇ/$t $t]   ⍝ [¯204ₓ; [200 ¯204 204]ₓ; $t; 5ₓ]

⍝ bitwise-whole — Fractional input is not an integer bit pattern
~ᵇ1.5
⍝ error: DOMAIN ERROR

⍝ bitwise-table — Every truth-table entry must be Boolean-valued
6+ᵇ3
⍝ error: DOMAIN ERROR

⍝⍝ Selection after ordering

⍝ selected-order — Bound take and ranked selection compose with stable sort and grade
v←8 2 4 2 6 1 7 5 9 3
[((2↑)∘<)v ((¯2↑)∘>)v ((2↑)∘⍋)v ((¯2↑)∘⍒)v ((2⌷)∘<)v]
⍝ =>
[[1 2] [2 1] [5 1]ₓ [3 5]ₓ 2]

⍝⍝ Vector-axis reduction

⍝ reduce-axes — Selected axes form one ravelled cell; remaining axes form the result frame
n←2 2 3⍴⍳6ₓ
[+/⍠1 2 n;+⌿⍠0 2 n;+/⍠2 1 n]
⍝ =>
[15 15;6 24;15 15]ₓ

⍝ reduce-axes-order — Axis order determines the ravel, not a sequence of reductions
m←2 2⍴⍳4ₓ ⋄ [-/⍠0 1 m;-/⍠1 0 m]   ⍝ [¯2 ¯4]ₓ

⍝ reduce-axes-binding — Glyphs, named operators and already-derived reductions use the same axes
r←/
sum←+/
n←2 2 3⍴⍳6ₓ
[+r⍠1 2 n;sum⍠1 2 n;+/⍠1 2 n]
⍝ =>
[15 15;15 15;15 15]ₓ

⍝ reduce-axes-empty — Empty cells use identities, empty frames retain prototypes, no axes means singleton cells
[+/⍠1 2 (2 0 3⍴0);+/⍠1 2 (0 2 3⍴0);+/⍠⍬ (2 2⍴⍳4ₓ)]
[0 0;⍬;2 2⍴⍳4ₓ]

⍝ reduce-axes-seed — Each cell receives the whole seed
10 20+/⍠1 2 (2 2 3⍴⍳6ₓ)   ⍝ [25 35⋄25 35]

⍝ reduce-rank-seed — A number on the left of a fold under Rank seeds the fold of every cell, whether the function folds in any order or in its own order
[10ₓ+/⍤1 [1 2 3⋄4 5 6]ₓ;10ₓ-/⍤1 [1 2 3⋄4 5 6]ₓ;0.5⌈/⍤1 [0 0.25⋄1 0.75]]   ⍝ [[16 25]ₓ;[¯8 ¯5]ₓ;0.5 1]

⍝ reduce-axes-keys — Unselected axes retain their labels
+/⍠1 2 ("aa" "bb":2 2 3⍴⍳6ₓ)   ⍝ ("aa" "bb":15 15)ₓ

⍝ reduce-view-keys — A fold that reads a pending view of a computed array keeps the names of the axes it doesn't fold, as a fold of a stored array does
v←262144⍴1 ⋄ ⍴+/["row":2 "col":131072]⍴2×v   ⍝ ["row":2]ₓ

⍝ reduce-view-axis-keys — So does a fold along an axis given by `⍠`
v←262144⍴1 ⋄ ⍴+/⍠0⊢["row":2 "col":131072]⍴2×v   ⍝ ["col":131072]ₓ

⍝ reduce-axes-calls — Generic reductions retain right association and the order of the cells
f←{⎕←⍺ ⋄ ⍺-⍵} ⋄ f/⍠1 2 (2 2 2⍴⍳8ₓ)
[¯2 ¯2]ₓ
⍝ ⎕: 2ₓ\n1ₓ\n0ₓ\n6ₓ\n5ₓ\n4ₓ

⍝ reduce-axes-prototype — An empty frame invokes the reduction on its prototype cell
f←{⎕←⍺ ⋄ ⍺+⍵} ⋄ f/⍠1 2 (0 2 2⍴0)
⍬
⍝ ⎕: 0\n0\n0

⍝ reduce-axes-repeat — Axes are distinct
+/⍠1 1 (2 2⍴⍳4ₓ)
⍝ error: DOMAIN ERROR

⍝ reduce-axes-rank — An axis specification is a unit or vector
+/⍠[1 2⋄] 2 2⍴⍳4ₓ
⍝ error: RANK ERROR

⍝⍝ Axis keys

⍝ axis-description-single — A one-position description labels the existing axis without enclosing its values
["row":"only"]:,7   ⍝ ["row":1]⍴"only":⍠0,7

⍝ axis-description-collision — Axis descriptions cannot duplicate a surviving axis name
["city":["Jan" "Feb" "Mar"]]:⍠1 ["city":2 3]⍴0
⍝ error: DOMAIN ERROR

⍝ axis-shape — The shape is keyed by axis names, and reshaping by a keyed shape names the axes
M←["city":2 "month":3]⍴⍳6ₓ
[⍴M "month"⌷⍴M ⍳⍠0⍴M ⍴(:⍴M)⍴M ⍴("a" "b":⍴M)⍴M]
⍝ =>
[["city":2 "month":3] 3 ["city" "month"] [2 3] ["a":2 "b":3]]ₓ

⍝ axis-shape-gaps — An unnamed axis is an unnamed entry of the shape
A←["city":2 3]⍴1
B←["product":4 3]⍴2
[⍴A+B ⍳⍠0⍴A+B]
⍝ =>
[["city":2 3 "product":4] ["city" 1 "product"]]ₓ

⍝ axis-gap-agreement — Named entries match by name, and unnamed entries match in order
[["a":1 2]+[10 "a":20] ["a":1 2]+["b":10 20]]
⍝ =>
[["a":21 12] ["a":1 22 "b":10]]

⍝ —
["a":1 2]+["a":1 2 3]
⍝ error: LENGTH ERROR

⍝ axis-gap-json — A JSON object needs a key for every entry
•json⁻¹ ["a":1 2]
⍝ error: DOMAIN ERROR

⍝ axis-colon — Colon binds as a function, including in a dfn, and literal separators keep their meanings
f←:
a←"n" f 5
g←{⍵<0?-⍵;+/:"n":⍵}
[(:a);≢["a":1 "b":2];g ¯3;g 3]
⍝ =>
[[5] 2ₓ 3 3]

⍝ axis-pair-literal — A literal with key:value items builds one keyed vector, other items are unnamed entries, and display reads back
x←3
r←["one":[1 2] "two":x]
[r≡"one" "two":[[1 2] 3];r≡⍎⍕r;["a":1 2]≡("a":1),2;[["a":1] ["b":2]]≡[["a":1]],[["b":2]]]
⍝ =>
$t $t $t $t

⍝ axis-page-header — Each page of a keyed array of rank 3 or more is headed by the index that selects it
⎕←["pp":["x1" "x2"] "rr":["r1" "r2"] "cc":["c1" "c2"]]:2 2 2⍴⍳8ₓ
0
⍝ =>
0ₚ
⍝ ⎕: "x1"⌷\n ₓ c1 c2\nr1  0  1\nr2  2  3\n\n"x2"⌷\n ₓ c1 c2\nr1  4  5\nr2  6  7

⍝ axis-colon-order — Construction uses ordinary right-to-left evaluation
:(⎕←'a'):(⎕←7)
,7
⍝ ⎕: 7\n'a'

⍝ axis-construct — Attach keys to axes without nesting the matrix; Iota with an axis inspects them
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
[⍴M;⍳⍠0 M;⍳⍠1 M;⍳⍠0 1 (:M);:M]
⍝ =>
[[2 2]ₓ ["alice" "bob"] ["price" "qty"] [0 1;0 1]ₓ [10 2⋄20 4]]

⍝ axis-selection — Atomic row selection retains column keys; one string is one selector
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
["bob"⌷M;[: "price"]⌷M;"alice" "qty"⌷M;[,⊂"alice"]⌷M]
⍝ =>
[["price":20 "qty":4] ["alice":10 "bob":20] 2 ([["alice"];"price" "qty"]:[10 2⋄])]

⍝ axis-insertion — Appending both axes fills every new coordinate with the original prototype
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
M.["cara" "tax"]←3
M
⍝ =>
["alice" "bob" "cara";"price" "qty" "tax"]:[10 2 0⋄20 4 0⋄0 0 3]

⍝ axis-agreement — Independently union axes, including coordinates absent from both inputs
A←"alice":⍠0 ("price":⍠1 [[10]⋄])
B←"bob":⍠0 ("qty":⍠1 [[2]⋄])
A+B
⍝ =>
["alice" "bob";"price" "qty"]:[10 0⋄0 2]

⍝ axis-singleton — Broadcasting expands positions; enclosure keeps a single named value
[("base":5)+10 20 30;("base":5)+⊂10 20 30]
[15 25 35;["base":[15 25 35]]]

⍝ axis-iota — Iota uses the dimension values, and a keyed shape names the result's axes, as it does for reshape
⍳("rows" "cols":[2 3]ₓ)   ⍝ ("rows" "cols":[2 3]ₓ)⍴⍳[2 3]ₓ

⍝ named-index-ends — A named selector counts a negative position back from the end, as an unnamed one does; arithmetic on it wraps or clamps
x←10×⍳5ₓ ⋄ i←⍳["i":5] ⋄ [x.[5|i+1] x.[i-1] x.[0⌈i-1]]   ⍝ [["i":5]⍴10 20 30 40 0;["i":5]⍴40 0 10 20 30;["i":5]⍴0 0 10 20 30]

⍝ named-selector-pairs — Named selectors with one name read one position along both; different names combine outer
m←[0 1 2⋄3 4 5⋄6 7 8] ⋄ i←⍳["i":3] ⋄ j←⍳["j":3] ⋄ [m.[i i];+/⍠"i" m.[i i];⍴m.[i j];⍴[0 2;i]⌷m]   ⍝ [["i":3]⍴0 4 8;12;["i":3 "j":3]ₓ;[2 "i":3]ₓ]

⍝ named-selector-rank — A matrix of positions pairs its named axis with an earlier selector's
m←[0 1 2⋄3 4 5⋄6 7 8] ⋄ d←⍳["d":3] ⋄ g←["d":3 "q":2]⍴[0 2⋄1 0⋄2 1] ⋄ m.[d g]   ⍝ ["d":3 "q":2]⍴[0 2⋄4 3⋄8 7]

⍝ named-selector-keys — A named vector keeps the keys at the positions it selects
x←"a" "b" "c":10 20 30 ⋄ i←⍳["i":3] ⋄ x.[2-i]   ⍝ ["i":3]⍴["c":30 "b":20 "a":10]

⍝ named-assignment-outside — Assignment through a named selector can't reach outside its axis
x←10ₓ×⍳5ₓ ⋄ i←⍳["i":5] ⋄ x.[i+1]←0
⍝ error: INDEX ERROR

⍝ nan-positions — A NaN position reads the fill in every selection form, `•edge` gives NaN for a position outside its axis, and a write through a NaN position writes nothing
x←10 20 30 40 ⋄ m←[1 2⋄3 4] ⋄ n←[1 2;3 4] ⋄ [x.[[1 $n]];[[1 $n]]⌷m;[$n]⊃x;n.[[[1 $n] [0 1]]];-@[1 $n] x;x.[(4•edge (⍳4ₓ)-1ₓ)]]   ⍝ [20 0;[3 4⋄0 0];0;0 2;10 ¯20 30 40;0 10 20 30]

⍝ nan-exact-positions — Exact positions beside a NaN decode as other positions do, in selection and in `•edge`, and `•edge`'s fill keeps every position exact under a narrow float pref
x←10 20 30 40 ⋄ •prefs ["float":16] ⋄ e←4096ₓ•edge 2049ₓ ⋄ •prefs ["float":64] ⋄ [x.[[1ₓ $n]];4ₓ•edge [1ₓ $n];e]   ⍝ [[20 0] [1 $n] 2049]

⍝ nan-writes — A write through a NaN position writes nothing, at a position, an item or a path level alike, and a modifier never runs for it. An atom keeps its value, and an empty array keeps its storage
y←10 20 30 40 ⋄ z←y ⋄ p←[1 2;3 4] ⋄ q←p ⋄ r←p ⋄ a←5 ⋄ e←⍬ ⋄ y.[[1 $n]]←5 6 ⋄ z.[$n]←5 ⋄ ([1 $n]⊃p)←9 ⋄ [0 $n]⊃q +←9 ⋄ r.[0].[$n] {⍺+⍵}←1 ⋄ $n⊃a ←9 ⋄ e.[[$n]]←⊂3 4 ⋄ [y z p q r a e]   ⍝ [10 5 30 40;10 20 30 40;(1 2⋄3 4);(1 2⋄3 4);(1 2⋄3 4);5;⍬]

⍝ named-selector-disagree — Named selectors that share a name must agree in length
m←3 3⍴⍳9ₓ ⋄ m.[⍳["d":3] ⍳["d":2]]
⍝ error: LENGTH ERROR

⍝ axis-named-index — A named index as the axis operand names its axis; Iota with an axis keeps the axis name
a←["k":3 "n":2]⍴⍳6ₓ ⋄ k←⍳["k":3] ⋄ [+/⍠k a;⍴⍳⍠"n" a]   ⍝ [["n":2]⍴[6 9]ₓ;["n":2]ₓ]

⍝ fresh-index — `•index n` is `⍳n` along a fresh name: `_`, an optional stem, `·` and a random part. Indices from two calls have different names and build a grid
[b c]←"b"•index 2 2 ⋄ [∧/b=⍳2ₓ;≢⍴b+c;3↑↑⍳⍠0⊢⍴b;≢¨•index 2 3 4]   ⍝ [$t 2ₓ "_b·" [2 3 4]ₓ]

⍝ match-names — Match pairs axes as arithmetic does: the same name or no name matches, different names don't, and keyed axes match by key in any order, or match an unkeyed axis. Search matches cells by the same rule
m←["r":2 "c":3]⍴⍳6ₓ ⋄ [(⍳["i":3])≡⍳3ₓ;(⍳["i":3])≡⍳["j":3];["a":1 "b":2]≡["b":2 "a":1];["a":1 "b":2]≡1 2;m⍳⍳3ₓ;m⍳⍳["d":3]]   ⍝ [$t $f $t $t 0ₓ 2ₓ]

⍝ named-selector-address — A selector with a name that no other selector has selects along the axis with that name, wherever it stands; other selectors take the free axes in order
x←["i":3 "j":4]⍴⍳12ₓ ⋄ i←⍳["i":3] ⋄ j←⍳["j":4] ⋄ [x≡x.[j i] x≡[j i]⌷x x.[[1 2]]]   ⍝ [$t $t ["i":2 "j":4]⍴[4 5 6 7⋄8 9 10 11]ₓ]

⍝ named-coordinates — A named selector of coordinates claims every axis it covers, as an unnamed one does, and pairs in `⌷` with an index array that shares its name
a←["r":2 "c":3 "z":4]⍴⍳24ₓ ⋄ p←["r":2]⍴[[0 1] [1 2]] ⋄ b←3 3 3⍴⍳27ₓ ⋄ c←["p":2]⍴[[0 1] [1 2]] ⋄ j←["p":2]⍴0 1 ⋄ [[p 1]⌷a [c j]⌷b]   ⍝ [["r":2]⍴[5 21]ₓ;["p":2]⍴[3 16]ₓ]

⍝ named-selector-no-axis — A named selector with no axis to select along is an error
z←["i":3 "k":2]⍴⍳6ₓ ⋄ z.[⍳["j":2]]
⍝ error: INDEX ERROR

⍝ named-selector-shared — Selectors that share a name read the free axes in order, as unnamed index arrays do: a row number for each column, and a diagonal
x←["i":3 "j":4]⍴⍳12ₓ ⋄ i←⍳["i":3] ⋄ j←⍳["j":4] ⋄ k←["j":4]⍴2 0 1 2 ⋄ [x.[k j] x.[i i]]   ⍝ [["j":4]⍴[8 1 6 11]ₓ;["i":3]⍴[0 5 10]ₓ]

⍝ grade-keeps-name — Grade's positions keep the name of the axis they run along, through ranks and partial grades too, so a grade along `i` sorts each column
x←["i":3 "j":4]⍴3 1 4 1 5 9 2 6 5 3 5 8 ⋄ v←["i":12]⍴⌽⍳12ₓ ⋄ [x.[(⍋⍠"i" x)];⍋⍋ v;3↑⍋ v]   ⍝ [["i":3 "j":4]⍴[3 1 2 1⋄5 3 4 6⋄5 9 5 8];["i":12]⍴⌽⍳12ₓ;["i":3]⍴[11 10 9]ₓ]

⍝ named-selector-along — `⌷⍠` places a named selector on an axis with another name, which the result renames. An axis that shares a name with an index array pairs with it, reading one item from each row
m←["r":3 "c":3]⍴⍳9ₓ ⋄ p←["r":3]⍴2 0 1 ⋄ [[p]⌷⍠"c" m;⍴[⍳["q":2]]⌷⍠"c" m]   ⍝ [["r":3]⍴[2 3 7]ₓ ["r":3 "q":2]ₓ]

⍝ hidden-names — A `_` name meets a visible name, or no name, as if that axis had no name, and the result keeps the visible name
h←•index 3 ⋄ v←⍳["v":3] ⋄ [⍴h+v ⍴v+h h≡⍳3ₓ h≡⍳["v":3]]   ⍝ [["v":3]ₓ;["v":3]ₓ;$t;$t]

⍝ axis-structure — Transpose moves labels; reduction removes only the reduced axis
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
[+/M +⌿M ⍉M ⌽M 1↑M]
⍝ =>
[["alice":12 "bob":24] ["price":30 "qty":6] (["price" "qty";"alice" "bob"]:[10 20⋄2 4]) (["alice" "bob";"qty" "price"]:[2 10⋄4 20]) (["alice";"price" "qty"]:[10 2⋄])]

⍝ axis-reshape — Shape-changing reshape drops keys and may cycle values
4⍴"aa" "bb":1 2   ⍝ 1 2 1 2

⍝ axis-replicate — An unkeyed axis may repeat while another axis retains its keys
2#⍠¯1 ["aa" "bb"]:[1 2⋄3 4]   ⍝ "aa" "bb":[1 1 2 2⋄3 3 4 4]

⍝ axis-repeat — A labelled axis cannot repeat a position
2#("aa" "bb":[1 2⋄3 4])
⍝ error: DOMAIN ERROR

⍝ axis-search — Outer labels do not restrict value search; found positions return labels
A←"left":7
B←"right":7
[A∊B A⍳B A∩B A∪B A,B]
⍝ =>
[["left":$t] ["right":"left"] ["left":7] ["left":7] ["left":7 "right":7]]

⍝ axis-indices — Grade, interval index and index-of return keys, retaining numeric sentinels
V←"low" "mid" "high":10 20 30
[⍒V;V⍳20 99;V⍸5 15 25 40;[⍒V]⌷V]
⍝ =>
[["high" "mid" "low"] ["mid" 3]ₓ [0 "low" "mid" "high"]ₓ ["high":30 "mid":20 "low":10]]

⍝ axis-match — Match aligns keys independently on both axes
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
M≡⌽⊖M   ⍝ $t

⍝ axis-outer — Outer product keeps each argument's labelled axes
("aa" "bb":1 2)+⊗("xx" "yy":10 20)
["aa" "bb";"xx" "yy"]:[11 21⋄12 22]

⍝ axis-rank — Frame keys survive assembly; cell axes need the same labels in every result
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
[+/⍤1 M;⊢⍤1 M;⌽⍤1 M;{10=↑⍵?⌽⍵;⍵}⍤1 M]
⍝ =>
[["alice":12 "bob":24] (["alice" "bob";"price" "qty"]:[10 2⋄20 4]) (["alice" "bob";"qty" "price"]:[2 10⋄4 20]) ("alice" "bob":[2 10⋄20 4])]

⍝ axis-rank-union — Rank aligns frame keys before applying the cell function
A←"alice" "bob":[1 2⋄3 4]
B←"bob" "cara":[10 20⋄30 40]
A +⍤1 B
⍝ =>
"alice" "bob" "cara":[1 2⋄13 24⋄30 40]

⍝ axis-windows — Windows have no keys on their windowed axes; stencil centres keep their axis labels
V←"aa" "bb" "cc":1 2 3
[2↕V;3↕V;{+/⍵}⌺3 V;{(⍳⍠0 ⍵)≡⍳3ₓ}⌺3 V]
⍝ =>
[[1 2⋄2 3];[1 2 3⋄];["aa":3 "bb":6 "cc":5];["aa":$t "bb":$t "cc":$t]]

⍝ axis-coordinates — Pick, Squad and coordinate indexing resolve each axis independently
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
["bob" "qty"⊃M;"bob" "qty"⌷M;"alice"⊃M;[(⊂"alice" "price")]⌷M;↑⍸M=20]
⍝ =>
[4 4 ["price":10 "qty":2] ⊂10 ["bob" "price"]]

⍝ axis-explicit — Explicit axes for a pervasive function align the labels on those axes
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
M+⍠1 ("qty" "tax":10 2)
⍝ =>
["alice" "bob";"price" "qty" "tax"]:[10 12 2⋄20 14 2]

⍝ axis-assembly — Added axes are unkeyed; unchanged axes retain labels
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
[⊃⊂⍠1 M;{[⍵⋄]}⍤1 M;⍪"aa" "bb":1 2]
⍝ =>
[(["alice" "bob";"price" "qty"]:[10 2⋄20 4]) (["alice" "bob";"price" "qty"]:⍠0 2 (2 1 2⍴10 2 20 4)) ("aa" "bb":[[1]⋄[2]])]

⍝ axis-permuted-agreement — Explicit axes carry keys with their mapped dimensions
X←["xx" "yy" "zz";"aa" "bb"]:3 2⍴⍳6ₓ
(2 3 1⍴0)+⍠1 0 X
⍝ =>
["aa" "bb";"xx" "yy" "zz"]:⍠0 1 (2 3 1⍴0 2 4 1 3 5)

⍝ axis-compact-union — Compact integer and float paths use the same missing-position fill
X←"aa" "bb":[2 3]ₓ
Y←"bb" "cc":[5 7]ₓ
[X+Y X×Y X<Y (X+0)+Y X+¨Y]
⍝ =>
[("aa" "bb" "cc":[2 8 7]ₓ) ("aa" "bb" "cc":[0 15 0]ₓ) ("aa" "bb" "cc":[$f $t $t]) ("aa" "bb" "cc":2 8 7) ("aa" "bb" "cc":[2 8 7]ₓ)]

⍝ axis-partition — Partition cells retain sliced labels; group axes are new and unkeyed
M←["alice" "bob";"xx" "yy" "zz"]:2 3⍴⍳6ₓ
[1 2⊆M;1 0 1⊂⍠1 M]
⍝ =>
((([["alice"];"xx" "yy" "zz"]:[0 1 2⋄]) ([["bob"];"xx" "yy" "zz"]:[3 4 5⋄])⋄(["alice" "bob";"xx" "yy"]:[0 1⋄3 4]) (["alice" "bob";["zz"]]:[[2]⋄[5]])))ₓ

⍝ axis-selective-write — Keyed RHS follows selected labels; Pick of a row updates its elements
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
("alice"⊃M)←"qty" "price":7 8
(⌽M)←["bob" "alice";"price" "qty"]:[30 3⋄40 4]
M
⍝ =>
["alice" "bob";"price" "qty"]:[40 4⋄30 3]

⍝ axis-catenate-alignment — Catenate aligns non-joined keys and concatenates joined labels
A←["aa" "bb";"xx" "yy"]:[1 2⋄3 4]
B←["bb" "aa";["zz"]]:[[5]⋄[6]]
A,B
⍝ =>
["aa" "bb";"xx" "yy" "zz"]:[1 2 6⋄3 4 5]

⍝ axis-empty-shapes — Empty axes preserve other dimensions and their labels
M←"xx" "yy":⍠1 (0 2⍴0)
[⍳⍠1 M;⍉M;+/M;⊢⍤1 M;(0⍴0)⌽⍠1 M]
⍝ =>
[["xx" "yy"] ("xx" "yy":2 0⍴0) (0⍴0) ("xx" "yy":⍠1 (0 2⍴0)) (0 2⍴0)]

⍝ axis-numeric-frames — Number-theory and radix results retain their argument frames
V←"aa" "bb":1 2
[⍭V ⊤V 2⊥⊤V ("days" "hours":0 24)⊤("aa" "bb":25 50)]
⍝ =>
[["aa":3 "bb":5]ₓ (["aa" "bb";0 1]:[0 1⋄1 0]) ["aa":1 "bb":2] (["aa" "bb";"days" "hours"]:[1 1⋄2 2])]

⍝ axis-product-frame — Inner product retains uncontracted axes
A←"aa" "bb":[1 2⋄3 4]
B←"xx" "yy":⍠1 [5 6⋄7 8]
A+.×B
⍝ =>
["aa" "bb";"xx" "yy"]:[19 22⋄43 50]

⍝ axis-native-rank — Native numerical cell functions align labelled frames like Rank
P←"aa" "bb":[1 2⋄10 3]
X←"bb" "aa":4 5
[P⌻X ("aa" "bb":3 3)⍭("bb" "aa":10 9)]
⍝ =>
[["aa":11 "bb":22] ["aa":6 "bb":4]ₓ]

⍝ axis-matrix-layout — Inversion swaps axes; a solution retains the coefficient column axis
M←["r1" "r2";"xx" "yy"]:[2 0⋄0 4]ₓ
[⌹M [4 12]ₓ⌹M]
⍝ =>
[(["xx" "yy";"r1" "r2"]:[1r2 0ₓ⋄0ₓ 1r4]) ["xx":2 "yy":3]ₓ]

⍝ axis-power-frame — Array-valued iteration counts supply the result frame
N←"initial" "once" "twice":0 1 2
2×⍣N 3   ⍝ "initial" "once" "twice":3 6 12

⍝ axis-contract — Contracted axes pair names, retaining the left contraction order
A←"hi" "lo":1 2
B←"lo" "hi":10 20
M←["r1" "r2";"xx" "yy"]:[2 0⋄0 4]ₓ
[A+.×B ("hi" "lo":10 10)⊥("lo" "hi":2 1) ("r2" "r1":[12 4]ₓ)⌹M]
⍝ =>
[40 12 ["xx":2 "yy":3]ₓ]

⍝ axis-complex-parts — Complex decomposition adds an unkeyed axis after the original axes
∨"first" "second":3j4 5j12   ⍝ "first" "second":[3 4⋄5 12]

⍝ axis-gradient — the Jacobian keeps the keys of the result and of the argument
f←[1 2 3]ₓ↣⌻
g←⊂[1 2 0⋄1 0 2]ₓ ↣⌻
[f∂ ("aa" "bb":[1 2]ₓ);g∂⊂"xx" "yy":[3 4]ₓ]
⍝ =>
[(["aa" "bb";"aa" "bb"]:[8 0⋄0 14]ₓ) ⊂["xx":6 "yy":8]ₓ]

⍝ axis-contract-missing — Labelled contractions require the same key set, including singletons
("aa":1)+.×("bb":2)
⍝ error: LENGTH ERROR

⍝ axis-decode-missing — Decode cannot invent a radix position
("hi" "lo":10 10)⊥("hi" "other":1 2)
⍝ error: LENGTH ERROR

⍝ axis-inverse-layout — Inverse scan and outer-product inversion retain surviving axis labels
V←"aa" "bb":1 3
B←"row1" "row2":10 20
f←(×↢*)⍨
[(+\)⁻¹ V;B↣(+⊗)⁻¹ ⊖B+⊗V;⍳⍠0 f⁻¹ f V]
⍝ =>
[["aa":1 "bb":2] ["aa":1 "bb":3] ["aa" "bb"]]

⍝ axis-nested-matrix-write — Named insertion uses the same axis extension inside stored matrices
T←"data":["r1" "r2";"xx" "yy"]:[1 2⋄3 4]
T.data.["r3" "zz"]←9
("r4"⊃T.data)←10 11 12
T.data
⍝ =>
["r1" "r2" "r3" "r4";"xx" "yy" "zz"]:[1 2 0⋄3 4 0⋄0 0 9⋄10 11 12]

⍝ axis-selector-once — Preparing named insertion evaluates a computed selector once
T←"data":"aa":1
calls←0
(({calls+←1 ⋄ "bb"}0)⌷T.data)←2
calls   ⍝ 1

⍝ axis-key-groups — Key returns named positions monadically and slices labelled value cells dyadically
V←"aa" "bb" "cc":1 2 1
[{⊂⍵}⌸V;1 2 1{⊂⍵}⌸V]
⍝ =>
[[["aa" "cc"] ["bb"]] [["aa":1 "cc":1] ["bb":2]]]

⍝ axis-solve-missing — Solve cannot invent an equation
("r1" "other":4 12)⌹("r1" "r2":[2 0⋄0 4])
⍝ error: LENGTH ERROR

⍝⍝ Operand glyphs

⍝ — Left and right operands bind without spaces
2{⍶+⍹×⍵}3(4)   ⍝ 14

⍝ — An operator that starts a run takes the whole run before it as its left operand
x←0 1 1 0 1 1 ⋄ ≢⊢ ⌸x   ⍝ [2 4]ₓ

⍝ — A joined run that still starts with an operator joins again
+ / ¨[1 2;3 4 5]   ⍝ 3 12

⍝ — An operator in parentheses, a name that holds one, or a dot path to one starts a run as the operator does without them
x←0 1 1 0 1 1 ⋄ k←⌸ ⋄ m←["k":⌸] ⋄ [≢⊢ (⌸)x;≢⊢ (k)x;≢⊢ (m.k)x]   ⍝ [2 4;2 4;2 4]ₓ

⍝ — A run that ends in a dyadic operator in parentheses, or in a name that holds one, takes the next run as its right operand
j←∘ ⋄ [-(∘) | ¯3;-(j) | ¯3]   ⍝ ¯3 ¯3

⍝ — A dot path after a space reads as the touching form
T←["a":1 "b":2] ⋄ T .b+1   ⍝ 3

⍝ — An operator that starts a bracket item has no run before it
[≢ ⌸0 1 1]
⍝ error: SYNTAX ERROR

⍝⍝ Recorded statements

⍝ — A dfn's statement replays on later calls, and binds again where an operand it read turns from a function into an array
w←{X¨⍵} ⋄ X←{⍵+1} ⋄ a←w 1 2 ⋄ X←5 ⋄ b←w 1 2 ⋄ a b   ⍝ [2 3;5 5]

⍝ — An operator name that changes between calls
r←{+O ⍵} ⋄ O←/ ⋄ a←r 1 2 3 ⋄ O←\ ⋄ b←r 1 2 3 ⋄ a b   ⍝ [6;1 3 6]

⍝ — A name in parentheses that turns from an operator into a function, and back, changes whether its run joins the run before it
k←⌸ ⋄ g←{≢⊢ (k)⍵} ⋄ a←g 1 1 2 ⋄ k←⌽ ⋄ b←g 1 1 2 ⋄ k←⌸ ⋄ a b (g 1 1 2)   ⍝ [[2 1]ₓ 3ₓ [2 1]ₓ]

⍝ — A dot root that turns from a record into a function
s←{T.a ⍵} ⋄ T←["a":5] ⋄ a←s 1 ⋄ b←s 2 ⋄ T←⊢ ⋄ c←s ["a":7] ⋄ a b c   ⍝ [5 1;5 2;7]

⍝ — A dot path whose field turns from an operator into a function binds again, though its root stays a record
m←["k":⌸] ⋄ g←{≢⊢ m.k ⍵} ⋄ a←g 1 1 2 ⋄ m←["k":⌽] ⋄ b←g 1 1 2 ⋄ a b   ⍝ [[2 1]ₓ 3ₓ]

⍝ — A superscript on an array, a subscript and a train replay as they first bound
f←{(⍵₀²)+(+/÷≢)⍵} ⋄ (f 1 3) (f 2 4)   ⍝ 3 7

⍝ — A group that reads no names records the train it gave, beside dfns whose closures change between calls
h←{y←⍵ ⋄ [((+/÷≢)+{y}) 1 2 3;(+/{⍵×y}) 1 2 3]} ⋄ (h 10) (h 20)   ⍝ [12 60;22 120]

⍝ — An operator bound to a primitive operand replays as it first bound
p←{+/⍤1⊢⍵} ⋄ (p [2 2]⍴⍳4ₓ) (p [2 3]⍴⍳6ₓ)   ⍝ [1 5;3 12]ₓ

⍝ — An operator bound to a name binds again on each call
q←{↑,⊖⍤n⊢⍵} ⋄ n←1 ⋄ a←q [2 3]⍴⍳6ₓ ⋄ n←2 ⋄ a (q [2 3]⍴⍳6ₓ)   ⍝ [2 3]ₓ

⍝ — A name that switches from a monadic operator to a superscript binds again where the derive's result changes category
s←⍨ ⋄ h←{2 s ⍵} ⋄ a←h 5 ⋄ a_2←h 5 ⋄ s←² ⋄ b←h 5 ⋄ b_2←h 5 ⋄ a a_2 b b_2   ⍝ [2 2 [4 5] [4 5]]

⍝ — A group whose name changes category binds again inside the group, and the statement carries on where the group's value keeps its category
x←- ⋄ f←{(x ⍵)+1} ⋄ a←f 2 ⋄ x←[- +] ⋄ b←f 2 ⋄ a b   ⍝ [¯1 [¯1 3]]

⍝ — A statement binds again where a group's value changes category
x←- ⋄ f←{(x) ⍵} ⋄ a←f 2 ⋄ x←10 ⋄ b←f 2 ⋄ a b   ⍝ [¯2 [10 2]]

⍝ — A name that changes category with items already on the binder's stacks binds again from that state
f←{⍵ g 1} ⋄ g←+ ⋄ a←f 2 ⋄ g←10 ⋄ b←f 2 ⋄ a b   ⍝ [3 [2 10 1]]

⍝ — Inside a group in a group, a value that changes category binds again in the middle group, and the statement carries on where the middle group's value keeps its category
f←{((⍵⊃[- 10]) 1) 2} ⋄ (f 0) (f 1) (f 0)   ⍝ [¯1 2;[10 1] 2;¯1 2]

⍝ — Where the middle group's value changes category too, the statement binds again
f←{((⍵⊃[⊢ 10])) 5} ⋄ (f 0) (f 1) (f 0)   ⍝ [5 [10 5] 5]

⍝ — A group binds again from its start where what a name holds changes how its run binds, here whether the run ends in a dyadic operator
o←⊢ ⋄ f←{(+o 0) ⍵} ⋄ a←f 5 ⋄ o←⍤ ⋄ b←f 5 ⋄ o←⊢ ⋄ a b (f 5)   ⍝ [[0 5] 5 [0 5]]

⍝ — A statement that assigns replays, and binds again where its target's start moves: here `x` turns from an array, which `x(g)←` modifies, into a function, which applies to `(g)←3`
g←+ ⋄ x←1 ⋄ h←{x(g)←⍵ ⋄ 0} ⋄ a←h 2 ⋄ b←x ⋄ x←- ⋄ c←h 3 ⋄ b (x 5) (g 1 2)   ⍝ [3 ¯5 [1 2]]

⍝ — An assignment whose value changes category binds again with the assignment in progress
h←{x←(⍵⊃[- 5]) ⋄ x 1} ⋄ (h 0) (h 1) (h 0)   ⍝ [¯1 [5 1] ¯1]

⍝ — Chained and destructuring assignments replay
f←{a←b←⍵+1 ⋄ [c d]←a (b×2) ⋄ c-d} ⋄ (f 1),(f 2)   ⍝ ¯2 ¯3

⍝ — A modified assignment with no value replays
f←{x←⍵ ⋄ x(-)← ⋄ x} ⋄ (f 3),(f 4)   ⍝ ¯3 ¯4

⍝ — An assignment inside a group replays
f←{y←⍵ ⋄ z×(z←y+1)} ⋄ (f 3),(f 4)   ⍝ 16 25

⍝ — A last statement that assigns replays, and its call after the assignment returns to the caller's loop
f←{⍵=0?0;f k←⍵-1} ⋄ f 50   ⍝ 0

⍝ — A default argument replays
f←{⍺←⍵ ⋄ ⍺+⍵} ⋄ (f 1),(2 f 3),(f 4)   ⍝ 2 5 8

⍝ — An error guard's kinds replay
k←"INDEX" ⋄ f←{k::¯1 ⋄ ⍵⊃1 2} ⋄ (f 0),(f 5),(f 1)   ⍝ 1 ¯1 2

⍝ — A bracket literal's items replay in order, left to right
f←{[a←⍵+1;a×10]} ⋄ (f 1),(f 2)   ⍝ 2 20 3 30

⍝ — An item that fails stops the items after it, as when the binder evaluates the literal
f←{"VALUE"::n ⋄ [n←1;{}0;n←2]} ⋄ (f 0),(f 0)   ⍝ 1 1

⍝ — A keyed item whose keys don't fit its value fails before the items after it, after a successful call has compiled the literal too
f←{n←0 ⋄ ∞::n ⋄ ["a" "b":⍵;n+←1]} ⋄ a←f 3 ⋄ _←f 1 2 ⋄ a,(f 3)   ⍝ 0 0

⍝ — A literal whose item turns into a function holds the function, and the statement binds again around the literal
h←2 ⋄ f←{[h;⍵]} ⋄ a←f 1 ⋄ h←+ ⋄ b←f 1 ⋄ a b   ⍝ [2 1] [+ 1]

⍝ — A record literal with constant keys replays, and an absent ⍺ drops its key
f←{["a":⍺ "b":⍵]} ⋄ (f 1) (2 f 3) (f 4)   ⍝ ["b":1] ["a":2 "b":3] ["b":4]

⍝ — A literal whose key reads a name stays with `resolve`, and follows the name
k←"x" ⋄ f←{[k:⍵]} ⋄ a←f 1 ⋄ k←"y" ⋄ a (f 2)   ⍝ ["x":1] ["y":2]

⍝ — A run that ends in a dyadic operator replays with its right operand, and binds again where a name changes whether the run ends in one
op←⍤ ⋄ f←{(+/op 1) ⍵} ⋄ a←f [2 2]⍴⍳4 ⋄ op←⍨ ⋄ b←f [2 2]⍴⍳4 ⋄ a b   ⍝ [1 5;2 ([2 2]⍴⍳4)]

⍝ — A run's right operand that turns from an array into a function binds again
r←1 ⋄ f←{+/⍤ r ⍵} ⋄ a←f [2 3]⍴⍳6 ⋄ r←{1} ⋄ a (f [2 3]⍴⍳6)   ⍝ [3 12;3 12]

⍝ — A dfn called with and without ⍺ keeps a recording for each
f←{⍺-⍵} ⋄ (f 3),(5 f 3),(f 3),(1 f 3)   ⍝ ¯3 2 ¯3 ¯2

⍝ — A compiled read of a global follows the global when it's assigned again
g←1 ⋄ f←{⍵+g} ⋄ a←f 0 ⋄ g←10 ⋄ a,f 0   ⍝ 1 10

⍝ — A global deleted and assigned again keeps its entry, and a compiled read finds the new value
g←1 ⋄ f←{⍵+g} ⋄ a←f 0 ⋄ _←•ex "g" ⋄ g←5 ⋄ a,f 0   ⍝ 1 5

⍝ — A compiled read of a deleted global is a VALUE error, as the binder's read is
g←1 ⋄ f←{⍵+g} ⋄ a←f 0 ⋄ _←•ex "g" ⋄ f 0
⍝ error: VALUE ERROR

⍝ — A closure's read of its parent's local goes on to the global once the local is deleted
k←7 ⋄ f←{k←3 ⋄ h←{k} ⋄ a←h 0 ⋄ _←•ex "k" ⋄ a,h 0} ⋄ (f 0),(f 0)   ⍝ 3 7 3 7

⍝ — A name that `⍎` binds in the frame hides the global from a compiled read
x←1 ⋄ f←{a←x ⋄ ⍎"x←2" ⋄ a,x} ⋄ (f 0),(f 0)   ⍝ 1 2 1 2

⍝ — A compiled read in a dfn defined inside a scope reads the scope's key
r←["k":5] ⋄ f←{r.({⍵+k} ⍵)} ⋄ (f 1),(f 2)   ⍝ 6 7

⍝ — A group's train calls its tines in place on later calls, in the order its function would call them: right tine, left tine, then the middle tine
a←{⎕←"a" ⋄ ⍵} ⋄ b←{⎕←"b" ⋄ ⍺+⍵} ⋄ c←{⎕←"c" ⋄ ⍵×2} ⋄ f←{(a b c) ⍵} ⋄ (f 1),(f 2)   ⍝ 3 6
⍝ ⎕: c\na\nb\nc\na\nb

⍝ — A group's train called with a left argument, whose array tine binds the function after it
f←{⍺ (⍵-+) 10} ⋄ (3 f 1),(5 f 2)   ⍝ ¯12 ¯13

⍝ — A group's train whose last tine is a grade is built, so that it keeps its idiom
f←{(⍵↑⍋) 5 3 9 1} ⋄ (f 2),(f 3)   ⍝ [3 1 3 1 0]ₓ

⍝ — A run binds again where its first name turns from a function into an operator, which joins the run to the node before it
s←- ⋄ t←{+ s⍵} ⋄ a←t 1 2 3 ⋄ s←{⍶/⍵} ⋄ b←t 1 2 3 ⋄ s←- ⋄ a b (t 1 2 3)   ⍝ [[¯1 ¯2 ¯3] 6 [¯1 ¯2 ¯3]]

⍝ — A run that ends in a dyadic operator, with a constant right operand, records as the function it gives
f←{(+/⍤ 1) ⍵} ⋄ (f [2 2]⍴⍳4),(f [2 2]⍴4+⍳4)   ⍝ 1 5 9 13

⍝ — A last statement first recorded while an error guard stops its tail call returns its call to the caller's loop on later calls
f←{⍺←0 ⋄ ⍺=1?"DOMAIN"::0 ⋄ 0?⍬ ; ⍵≤0?0 ; 0 f ⍵-1} ⋄ [1 f 1;0 f 100000]   ⍝ 0 0

⍝⍝ Mathematical monads

⍝ — Real and imaginary parts form a trailing pair axis
∨3j4 5 0j¯2   ⍝ [3 4⋄5 0⋄0 ¯2]

⍝ — Exact real parts stay exact
∨1r3   ⍝ 1r3 0ₓ

⍝ — A Boolean's parts stay Boolean, with `$f` imaginary parts
∨ 1 0=1   ⍝ [$t $f⋄$f $f]

⍝ — Magnitude and phase, in radians
∧3j4 0j1   ⍝ [5 0.9272952180016122⋄1 1.5707963267948966]

⍝ — Empty decomposition retains frame and pair axis
⍴∨2 0⍴0ₓ   ⍝ [2 0 2]ₓ

⍝ — Self-classification keeps classes in first-occurrence order
="aba"   ⍝ [$t $f $t⋄$f $t $f]

⍝ — Self-classify major cells
=[1 2⋄3 4⋄1 2]   ⍝ [$t $f $t⋄$f $t $f]

⍝ — An atom has one class and one item
=7   ⍝ [[$t]⋄]

⍝ — No items, no classes
=⍬   ⍝ 0 0⍴$f

⍝ — Zero-width rows are equal
=3 0⍴0   ⍝ 1 3⍴$t

⍝ — Binary encode chooses enough last-axis digits
⊤[2 5]ₓ   ⍝ [0 1 0⋄1 0 1]ₓ

⍝ — Binary decode reads the last digit axis
⊥[0 1 0⋄1 0 1]ₓ   ⍝ [2 5]ₓ

⍝ — Binary zero needs no digits
⊤0ₓ   ⍝ ⍬ₓ

⍝ — Binary encoding works above machine-integer range
⊥⊤18446744073709551615ₓ   ⍝ 18446744073709551615ₓ

⍝ — Float binary encoding must not use tolerant floor
⊤9007199254740991   ⍝ 53⍴1

⍝ — Float mixed-radix digits retain low bits
(16⍴16)⊤¯1+2*53   ⍝ 0 0 1,13⍴15

⍝ — Binary digits require integral values
⊤1.5
⍝ error: DOMAIN ERROR

⍝ — Binary encoding is unsigned
⊤¯1
⍝ error: DOMAIN ERROR

⍝ — Increment/decrement pervade and preserve exactness
≥≤[1ₓ [2 3]ₓ]   ⍝ [1 [2 3]]ₓ

⍝ — Increment promotes on overflow
≥9223372036854775807ₓ   ⍝ 9223372036854775808ₓ

⍝ — Square promotes on overflow
⊼3037000500ₓ   ⍝ 9223372037000250000ₓ

⍝ — Doubling preserves rational values
⊽1r3   ⍝ 2r3

⍝ — Square and double preserve empty exact prototypes
[⊼0⍴0ₓ ⊽0⍴0ₓ]   ⍝ [⍬ ⍬]ₓ

⍝ — Square root extends into complex numbers
√0 9 ¯4 3j4   ⍝ 0 3 0j2 2j1

⍝ — Roots of exact rationals and big integers are approximate too
√1r9 18446744073709551616ₓ   ⍝ 0.3333333333333333 4294967296

⍝ — Roots of exact numbers are approximate, perfect squares included
√[2 4 9]ₓ   ⍝ 1.4142135623730951 2 3

⍝ — Odd integral roots of negative reals use the real branch
3 ¯3√¯8   ⍝ ¯2 ¯0.5

⍝ — Non-integral degree uses principal power
0.5√¯2   ⍝ 4

⍝ — Complex square root uses the principal branch
1E¯14>|(√¯3j¯4)-1j¯2   ⍝ $t

⍝ — Zero degree is undefined
0√2
⍝ error: DOMAIN ERROR

⍝ — Zero has an infinite negative-degree root
¯3√0   ⍝ ∞

⍝ — Pi fractions and multiples
[π2 1π2 2π3]
6.283185307179586 1.5707963267948966 2.0943951023931953

⍝ — Unit-circle points
○[0 1π2 π1]   ⍝ 1 0j1 ¯1

⍝ — Complex angles use exp(i z)
○0j1   ⍝ 0.36787944117144233

⍝ — Principal inverses of root and circle
[√⁻¹3 ○⁻¹○0.5]   ⍝ 9 0.5

⍝ — Bound pi fractions invert either argument
(1↣π)⁻¹1π4   ⍝ 4

⍝⍝ Function arrays

⍝ — Select a function from a vector, then call it
fs←[+ × ÷]
mul←fs₁
2 mul 3   ⍝ 6

⍝ — Primitives, dfns and derived functions share one function vector
fs←[+/ {⍵×⍵} 3↣+]
square←fs₁
square4   ⍝ 16

⍝ — A record entry can hold a function, and the record's text reads back
r←["sum":+/ "max":⌈/] ⋄ [r.sum 1 2 3;(⍎⍕r).max 1 2 3]   ⍝ 6 3

⍝ — Reverse and disclose a function vector
fs←[+ × ÷]
div←↑⌽fs
6 div 3   ⍝ 2

⍝ — A function vector remains one nested item in a list
fs←[+ ×]
gs←[fs ÷]
f←1⊃0⊃gs
2 f 3   ⍝ 6

⍝ — Overtake preserves existing functions
fs←[+ ×]
f←0⊃3↑fs
2 f 3   ⍝ 5

⍝ — Function-vector fill retains the first function
fs←[+ ×]
f←2⊃3↑fs
2 f 3   ⍝ 5

⍝ — Empty function vectors retain their positions in a list
fs←[+ ×]
f←2⊃[0↑fs 0↑fs ÷]
6 f 3   ⍝ 2

⍝ — Arrays and functions can share a mixed vector
fs←[+ ×]
result←(⊂1 2 3),fs₁
back←result₁
2 back 3   ⍝ 6

⍝ — A pick path descends into a nested function vector
x←[[+ ×] [- ÷]]
f←1⊃0⊃x
2 f 3   ⍝ 6

⍝ — A dfn can return a selected function
fs←[+ ×]
f←{⍵₁}fs
2 f 3   ⍝ 6

⍝ — Inverse enclose discloses a callable function
fs←[+ ×]
f←⊂⁻¹ [⊂1]⌷fs
2 f 3   ⍝ 6

⍝ — Each assembles function results back into an array
fs←[+ ×]
f←↑↑¨fs
2 f 3   ⍝ 5

⍝ — Rank assembles function results back into an array
fs←[+ ×]
f←↑↑⍤0 fs
2 f 3   ⍝ 5

⍝ — An empty pick path preserves the scalar array, without disclosing its function
fs←[+ ×]
s←[⊂1]⌷fs
s≡⍬⊃s   ⍝ $t

⍝ — Functions built from matching parts match, with values compared as `≡` compares them, and a dfn matches only itself
([+ +/ ×⍤1 1ₓ↣+]≡[+ +/ ×⍤1 1↣+]) ([{⍵}]≡[{⍵}])   ⍝ $t $f

⍝ — A selected function can use its still-active lexical binding
{a←⍵ ⋄ fs←[{a+⍵} ×] ⋄ f←↑fs ⋄ f3}4   ⍝ 7

⍝ — Passing a function vector to another dfn retains lexical lookup
use←{f←↑⍵ ⋄ f3} ⋄ {a←⍵ ⋄ use [{a+⍵} +]}4   ⍝ 7

⍝ — A helper may return a function while its defining frame remains active
id←{↑⍵} ⋄ {a←⍵ ⋄ f←id [{a+⍵} +] ⋄ f3}4   ⍝ 7

⍝ — Returning a top-level function needs no escaping local frame
fs←[{⍵×⍵} +]
f←{↑⍵}fs
f3   ⍝ 9

⍝ — Formatting a function vector produces character text
fs←[+ ×] ⋄ ⍴⍕fs   ⍝ [6]ₓ

⍝ — One list may contain numbers, callable functions and text
x←[1 + "abc"]
f←x₁
x₀f≢x₂   ⍝ 4

⍝ — Functions have no grade ordering
⍋[+ ×]
⍝ error: DOMAIN ERROR

⍝ — Grade fails only when a comparison reaches a function
⍋[[1 +] [2 -]]   ⍝ [0 1]ₓ

⍝ — Interval index requires ordered data, not functions
[+ ×]⍸[+ ×]
⍝ error: DOMAIN ERROR

⍝ — A returned function vector cannot retain an expired local frame
{x←⍵ ⋄ [{x+⍵} +]}2
⍝ error: DOMAIN ERROR

⍝ — Assignment to a selection cannot export a local function into an outer array
fs←[+ ×] ⋄ {fs₀←0⌷[{⍵} +] ⋄ 0}2
⍝ error: DOMAIN ERROR

⍝ — Even an empty function vector retains its prototype function
{0↑[{⍵} +]}2
⍝ error: DOMAIN ERROR

⍝ — Disclosing before return does not permit an escaping closure
{x←⍵ ⋄ ↑[{x+⍵} +]}2
⍝ error: DOMAIN ERROR

⍝ — Directly returning an escaping closure is also rejected
{x←⍵ ⋄ {x+⍵}}2
⍝ error: DOMAIN ERROR

⍝ — A level of a pick path has at most one position for each axis
[[0 0]]⊃[+ ×]
⍝ error: RANK ERROR

⍝ path-suffix — After pick paths that end at cells, more selectors select within each cell
a←[(2 3⍴⍳6ₓ) 7] ⋄ [[[0 1] [0 0]] [2 0]]⌷a   ⍝ [5 3⋄2 0]ₓ

⍝ path-suffix-assign — The same selection is a target
a←[(2 3⍴⍳6ₓ) 7ₓ]
([[[0 1] [0 0]] 2]⌷a)←50ₓ 20ₓ
a   ⍝ [[0 1 20⋄3 4 50] 7]ₓ

⍝ path-suffix-fill — A suffix that selects a fill, such as `$n`, reads the cell's prototype, in an empty cell too, and a write through it changes nothing
a←[(2 3⍴⍳6ₓ) 7ₓ] ⋄ z←[(2 0⍴0ₓ) 7ₓ] ⋄ b←a ⋄ ([[[0 1] [0 0]] $n]⌷b)←50ₓ 20ₓ ⋄ [[[[0 1] [0 0]] $n]⌷a;[[[0 0] [0 1]] $n]⌷z;b≡a]   ⍝ [[0 0]ₓ;[0 0]ₓ;$t]

⍝ path-suffix-keys — Each cell that a path reaches resolves the suffix with its own keys, for reads and writes. Where the cells' keys differ, the result's axes keep only their lengths
ka←["r":"x" "y";"c":"a" "b"] ⋄ kb←["r":"x" "y";"c":"b" "a"] ⋄ t←[(ka:[1 2⋄3 4]ₓ) (kb:[5 6⋄7 8]ₓ)] ⋄ u←t ⋄ ([[[0 0] [1 0]] "b"]⌷u)←20ₓ 50ₓ ⋄ [[[[0 0] [1 0]] "b"]⌷t;[[[0 0] [1 0]] :]⌷t;[[[0 0] [1 0]] :]⌷u]   ⍝ [[2 5]ₓ;[1 2⋄5 6]ₓ;[1 20⋄50 6]ₓ]

⍝⍝ Calling function arrays

⍝ — In a slot that takes only a function, an array that holds functions calls each one on the whole argument
aggs←["mean":+⌿÷≢ "sum":+⌿] ⋄ aggs∘⊢ 1 2 3   ⍝ ["mean":2 "sum":6]

⍝ — Rank calls the array on each row, and the results keep the array's keys
aggs←["mean":+⌿÷≢ "sum":+⌿] ⋄ aggs⍤1 ⊢ [1 2 3⋄4 5 6]
⊃[["mean":2 "sum":6] ["mean":5 "sum":15]]

⍝ — A dyadic call gives each function both arguments, and other items are their own results
10 [+ 0 -]¨ 1 2   ⍝ [11 0 9;12 0 8]

⍝ — A matrix of functions gives its frame to the results
6 [+ -⋄× ÷]¨ 2   ⍝ [8 4⋄12 3]

⍝ — Results of different lengths pad with fill, as Rank pads them
[⍳ ⊢]∘⊢ 3ₓ   ⍝ [0 1 2⋄3 0 0]ₓ

⍝ — A scalar that holds a function adds no axis to the function's result
{⍵×10}ᵘ∘⊢ 2   ⍝ 20

⍝ — Each calls a scalar that holds a function on each item
{⍵×10}ᵘ¨ 1 2   ⍝ 10 20

⍝ — A function inside a nested item is a constant, not a call
[[+] -]∘⊢ 1   ⍝ 2 1⍴[+ ¯1]

⍝ — An empty array holds no function, and acts as a constant
(0⍴[+ -])¨ 1 2   ⍝ [(0⍴[+ -]) (0⍴[+ -])]

⍝ — The functions run in ravel order
[{⎕←⍵} {⎕←-⍵}]∘⊢ 7   ⍝ 7 ¯7
⍝ ⎕: 7\n¯7

⍝ — Reduce calls the array on each pair, as it calls any function
[+ ×]/ 1 2 3   ⍝ [6 7⋄5 6]

⍝ — An array stops being callable when a write replaces its last function, even after an earlier call [mixed]
x←[+ 0]
_←x¨ 1
x.[0]←5
x¨ 1 2   ⍝ [5 0;5 0]

⍝ — A write that puts a function into an array makes it callable, even after an earlier call found none
x←[0 [1 2]]
_←x¨ 1
x.[0]←-
x¨ 5   ⍝ [¯5 0⋄1 2]

⍝ — Appending a function makes an array callable, even after an earlier call found none
x←[0 [1 2]]
_←x¨ 1
x,←-
x¨ 5   ⍝ [0 0⋄1 2⋄¯5 0]

⍝ — ∂ differentiates through each function of an array
{+/ , [⊢ ×⍨]⍤0 ⊢ ⍵}∂ 1 2   ⍝ 3 5

⍝ — Before binds a left argument to an array that holds functions
10↣[+ -] 3   ⍝ 13 7

⍝ — Commute's operand can be either kind, and an array that holds functions stays a constant there
[+ -]⍨ 5   ⍝ [+ -]

⍝ — A function made from an array reads back from its text
r←["f":[+ -]¨ "g":[+ ×]⍤1] ⋄ (⍎⍕r)≡r   ⍝ $t

⍝ — An array that holds functions calls them when an array follows it
aggs←["mean":+⌿÷≢ "sum":+⌿] ⋄ aggs 1 2 3   ⍝ ["mean":2 "sum":6]

⍝ — An array before it is the left argument
10 [+ -] 3   ⍝ 13 7

⍝ — With nothing after it, an array that holds functions is data, and strands
10 [+ -]   ⍝ [10 [+ -]]

⍝ — A function before it applies to the call's result
⊖ [+ -] 1 2   ⍝ [¯1 ¯2⋄1 2]

⍝ — Before a function, the array is that function's left argument
[+ -] , 10   ⍝ [+ - 10]

⍝ — In a train, the array is data
f←[+ -], ⋄ ≢f 1 2   ⍝ 4ₓ

⍝ — In brackets, each run is its own item, and a semicolon item calls
d←1 2 ⋄ [≢[[+ -] d];[+ -] 1]   ⍝ [2ₓ;1 ¯1]

⍝ — An array that holds functions can be another such array's argument
[≢ ≡] [+ -]   ⍝ [2 1]ₓ

⍝ — A scalar holding a function calls it when an array follows it
(-ᵘ)3   ⍝ ¯3

⍝ — An array after the absent ⍺ of a monadic call calls it, as a function does, and gives an absent result
{≢ [+ -] ⍺} 1
⍝ error: VALUE ERROR

⍝ — A recorded statement resumes the binder when its name's array loses its last function
f←{x ⍵}
x←[+ -]
a←f 1
x←[1 2]
b←f 1
[a;b]   ⍝ [1 ¯1;[1 2] 1]

⍝ — A recorded statement resumes the binder when its name's array gains a function
f←{x ⍵}
x←[1 2]
a←f 1
x←[+ -]
b←f 1
[a;b]   ⍝ [[1 2] 1;1 ¯1]

⍝⍝ Bracket lists

⍝ —
[1 2 3]   ⍝ 1 2 3

⍝ — Semicolons separate items that contain spaces
[1;2 3;4]   ⍝ [1 [2 3] 4]

⍝ — Brackets make an inner list one nested item
[[1 2] 3]   ⍝ [1 2;3]

⍝ — Unspaced expressions become separate items
[1+2 3+4]   ⍝ 3 7

⍝ — Named vectors remain nested rather than being catenated
a←1 2
b←3 4
[a b]   ⍝ [1 2;3 4]

⍝ — A function list can be one item within a list
x←[1 [+ ×] 4]
f←1⊃x₁
x₀ f x₂   ⍝ 4

⍝ — The list forms before reduction applies
+/[1 2 3]   ⍝ 6

⍝ — Bracket items evaluate left to right
a←0 ⋄ [(a←1) a]   ⍝ 1 1

⍝ — Bracket lists support destructuring and modified assignment
[a b]←[1 2]
[a b]+←[3 4]
[a b]   ⍝ 4 6

⍝ — A higher-rank value destructures into its major cells
[a b]←2 3⍴⍳6ₓ
b   ⍝ [3 4 5]ₓ

⍝ — A pattern takes keyed rows by key, in any order
m←"top" "bottom":2 3⍴⍳6ₓ
[bottom: top:]←m
bottom   ⍝ [3 4 5]ₓ

⍝ — A list of names destructures keyed rows by position
m←"top" "bottom":2 3⍴⍳6ₓ
[bottom top]←m
bottom   ⍝ [0 1 2]ₓ

⍝ — A pattern's key is spelled, never a value
top←"bottom" ⋄ m←"top" "bottom":2 3⍴⍳6ₓ
[top:x]←m
x   ⍝ [0 1 2]ₓ

⍝ — A string key serves a key that isn't a name
["first name":n]←["first name":3 "age":4]
n   ⍝ 3

⍝ — A key with no value takes the value of the name it spells
a←1 ⋄ b←2
[a: b:]   ⍝ ["a":1 "b":2]

⍝ — A key with no value is a name
["a":]
⍝ error: SYNTAX ERROR

⍝ — Replicate stored as an element acts as a function after disclosure
fs←[# +]
f←↑fs
1 0 1 f 2 3 4   ⍝ 2 4

⍝ — A defined operator can put its function operand in a returned list
op←{[⍶ +]}
fs←(×op)0
f←↑fs
2 f 3   ⍝ 6

⍝⍝ Strands of arrays

⍝ — Arrays side by side form a strand, as brackets do
a←1 2
b←3 4
a b   ⍝ [1 2;3 4]

⍝ — A longer strand has one item for each array
a←1 2
b←3 4
a b a   ⍝ [1 2;3 4;1 2]

⍝ — An argument beside itself
{⍵⍵}1 0   ⍝ [1 0;1 0]

⍝ — A vector beside a bracket list is a strand, not a selection
v←10 20 30 ⋄ v[2]   ⍝ [10 20 30;[2]]

⍝ — Two bracket lists side by side form a strand
[10 20 30] [2 0]   ⍝ [10 20 30;2 0]

⍝ — A scalar holding a function is one item of a strand when no array follows it
≢3(-ᵘ)   ⍝ 2ₓ

⍝ — Each run is one item
a←1
b←2
a b+1   ⍝ 1 3

⍝ — A parenthesised strand stays one item
a←1 2
b←3 4
a (b b)   ⍝ [1 2;[3 4;3 4]]

⍝ — A strand is not an assignment target
{v←1 2 3 ⋄ i←0 ⋄ (v)i←9 ⋄ v}0
⍝ error: SYNTAX ERROR

⍝⍝ Agenda

⍝ — Agenda chooses negate for a negative argument
cases←[- ⊢]
abs←(0≤)⍚cases
abs¯3   ⍝ 3

⍝ — Agenda chooses identity for a nonnegative argument
cases←[- ⊢]
abs←(0≤)⍚cases
abs3   ⍝ 3

⍝ — A constant selector chooses the second function
mul←1⍚[+ ×] ⋄ 2 mul 3   ⍝ 6

⍝ — A negative selector counts from the end
mul←¯1⍚[+ ×] ⋄ 2 mul 3   ⍝ 6

⍝ — The selector and selected function both receive the original arguments
choose←>⍚[- ÷] ⋄ 12 choose 3   ⍝ 4

⍝ — An agenda branch may itself return a function
choose←0⍚[{↑⍵} ⊢]
f←choose [+ ×]
2 f 3   ⍝ 5

⍝ — A singleton vector is a selector for each cell, and its axis expands
(,0)⍚[- ⊢] 4 5   ⍝ ¯4 ¯5

⍝ — The cases must be a vector, not a function-containing scalar
1⍚ ⍬⍴[+]
⍝ error: RANK ERROR

⍝ — Each axis of a selector needs an axis of the argument to pair with
{1 2}⍚[+ ×] 3
⍝ error: RANK ERROR

⍝ —
1.5⍚[+ ×]
⍝ error: DOMAIN ERROR

⍝ —
1⍚ 0↑[+ ×]
⍝ error: DOMAIN ERROR

⍝ — An array among agenda's options acts as a constant function
1⍚[1 2] 5   ⍝ 2

⍝ —
{'a'}⍚[+ ×] 3
⍝ error: DOMAIN ERROR

⍝ —
2⍚[+ ×]
⍝ error: INDEX ERROR

⍝ —
¯3⍚[+ ×]
⍝ error: INDEX ERROR

⍝ —
{3}⍚[+ ×] 3
⍝ error: INDEX ERROR

⍝ — Agenda evaluates the selector, then only the selected branch
choose←{⎕←9 ⋄ 1}⍚[{⎕←1 ⋄ 1÷0} {⎕←2 ⋄ ⍺-⍵}] ⋄ 10 choose 3
7
⍝ ⎕: 9\n2

⍝ — A selector for each cell runs each case only on the cells that pick it
(0≤)⍚[0 √] ¯4 9 16   ⍝ 0 3 4

⍝ — An index for each cell picks among several cases
k←0 2 1 2 ⋄ k⍚[- ⊢ 10×] 1 2 3 4   ⍝ ¯1 20 3 40

⍝ — With `⊣` and `⊢` as its cases, a selector picks each item from one of the arguments
3 1 4 <⍚[⊣ ⊢] 2 5 1   ⍝ 3 5 4

⍝ — A case runs once, on the list of all the cells that pick it
(0≤)⍚[{⎕←⍵ ⋄ -⍵} √] ¯1 4 ¯9 16
1 2 9 4
⍝ ⎕: ¯1 ¯9

⍝ — A unit result fills each cell of its case
(0≤)⍚[{+/⍵} ⊢] ¯1 4 ¯2   ⍝ ¯3 4 ¯3

⍝ — So does a result of one item, as under `@`
(0≤)⍚[{1↓⍵} ⊢] ¯1 4 ¯2   ⍝ ¯2 4 ¯2

⍝ — Any other result needs one cell for each cell of its case
(0≤)⍚[{2↑⍵} ⊢] ¯1 4 ¯2 ¯3
⍝ error: LENGTH ERROR

⍝ — An array case gives one value for each cell that picks it
(0≤)⍚[[10 20] ⊢] ¯1 4 ¯2   ⍝ 10 4 20

⍝ — A selector of the leading axis picks a case for each row, and a unit fills the whole row
(0<⊣/)⍚[1 ⊢] [¯1 2⋄3 4]   ⍝ [1 1⋄3 4]

⍝ — A result of one item spreads across each of its cells, even where other cases' cells differ in shape
0 1 2⍚[5 (1↓)⍤1 ⊢] [1 2 3⋄4 5 6⋄7 8 9]   ⍝ [5 5 5⋄5 6 0⋄7 8 9]

⍝ agenda:singleton-fill — A one-item list fills a cell even when only one cell picks its case
0 1⍚[[5] ⊢] [1 2 3⋄4 5 6]   ⍝ [5 5 5⋄4 5 6]

⍝ agenda:constant-fill-shape — When every result fills, the argument gives the cell shape
0 0⍚[0 ⊢] [1 2 3⋄4 5 6]   ⍝ [0 0 0⋄0 0 0]

⍝ agenda:result-keys — A case's result keys align with the cells that picked it
x←["a":1 "b":2 "c":3]
p←{["a":0 "b":1 "c":1]} ⋄ f←{⌽⍵+0j1}
p⍚[⊢ f] x
⍝ =>
["a":1 "b":2j1 "c":3j1]

⍝ agenda:cell-names — Conflicting result-cell names drop, as in ordinary assembly
m←["r":2 "c":2]⍴⍳4ₓ
⍴0 1⍚[{["r":1 "a":2]⍴⍵} {["r":1 "b":2]⍴⍵}] m
⍝ =>
["r":2 2]ₓ

⍝ agenda:padded-fill-keys — A no-case fill keeps the keys that padded result cells agree on
r←["a":1 "b":2 "c":3] ⋄ x←[r⋄r⋄r]
0 1 $n⍚[⊢ {[⍵⋄]}] x
⍝ =>
r←["a":1 "b":2 "c":3]
0 1 2⍚[⊢ {[⍵⋄]} 0] [r⋄r⋄r]

⍝ — The selector's axes pair with the argument's by name, and cells of the argument's rank return to its axes
m←["i":2 "j":3]⍴⍳6ₓ ⋄ {4<+⌿⍵}⍚[⊢ -] m   ⍝ ["i":2 "j":3]⍴[0 ¯1 ¯2 3 ¯4 ¯5]ₓ

⍝ — Cells of another rank keep the selector's axes first
m←["i":2 "j":3]⍴⍳6ₓ ⋄ {4<+⌿⍵}⍚[+/ ⌈/] m   ⍝ (["j":3]⍴3 4 5)ₓ

⍝ — An empty frame along a named axis keeps the argument's axes in their places
⍴{4<+⌿⍵}⍚[⊢ -] ["i":2 "j":0]⍴0   ⍝ ["i":2 "j":0]ₓ

⍝ — A keyed selector lines up with the argument by key
k←["a":1 "b":0] ⋄ k⍚[- ⊢] ["b":4 "a":5]   ⍝ ["b":¯4 "a":5]

⍝ — A cell whose key the selector lacks counts as NaN: it picks no case, and gets the result's fill
k←["a":1] ⋄ k⍚[- ⊢] ["b":4 "a":5]   ⍝ ["b":0 "a":5]

⍝ — A key that only the selector has is ignored
["a":1 "c":0]⍚[- 10×] ["b":4 "a":5]   ⍝ ["b":0 "a":50]

⍝ — A left argument lines up with the right argument by position, and a unit goes whole to each case
10 (0≤⊢)⍚[- +] ¯1 2   ⍝ 11 12

⍝ —
10 20 (0≤⊢)⍚[- +] ¯1 2 3
⍝ error: LENGTH ERROR

⍝ — A keyed left argument lines up by key: a key it lacks reads the fill, and a key that only it has is ignored
["b":10 "a":20 "c":30] (0≤⊢)⍚[- +] ["a":1 "b":¯2]   ⍝ ["a":21 "b":12]

⍝ —
["a":20] (0≤⊢)⍚[- +] ["a":1 "b":¯2]   ⍝ ["a":21 "b":2]

⍝ — A left argument's axis of length 1 stretches across the frame, as agreement stretches it
(1 3⍴100 200 300) +@[{1 0 1}] [1 2 3⋄4 5 6⋄7 8 9]   ⍝ [101 202 303⋄4 5 6⋄107 208 309]

⍝ — A case that no cell picks doesn't run
(0≤)⍚[{÷0} √] 4 9   ⍝ 2 3

⍝ — An argument with no cells gives an empty result, with the cells of the case that the selector's fill picks
⍴ (0≤)⍚[0 ⊢] 0 3⍴0   ⍝ [0 3]ₓ

⍝ — A case that fails on no cells gives an empty result, as Each's operand does
{⍵>0}⍚[{÷0} ⊢] ⍬   ⍝ ⍬

⍝ — Selector values decode as positions do, so a negative value counts back from the end
¯1 0⍚[- ⊢] 4 5   ⍝ 4 ¯5

⍝ —
2 0⍚[- ⊢] 4 5
⍝ error: INDEX ERROR

⍝ —
0.5 0⍚[- ⊢] 4 5
⍝ error: DOMAIN ERROR

⍝ — A NaN picks no case, and its cell gets the result's fill
0 1 $n⍚[- ⊢] 4 5 6   ⍝ ¯4 5 0

⍝ — A case of `⊢` leaves its cells where they are, and the other cases' results go around them
(0>)⍚[⊢ -] ¯1 4 ¯2   ⍝ 1 4 2

⍝ — With a case of `⊢`, a NaN cell still gets the fill
{1 $n 0}⍚[⊢ -] 4 5 6   ⍝ ¯4 0 6

⍝ — With a case of `⊢`, a result of another kind keeps its kind
⍕(0>)⍚[⊢ {2ₓ}] ¯1 4 ¯2   ⍝ "2ₓ 4 2ₓ"

⍝ — ⍚ keeps each result's kind, as Each does
⍕(0≤)⍚[0ₓ √] ¯4 9   ⍝ "0ₓ 3"

⍝ — `f@[p] Y` matches `p⍚[⊢ f] Y`
(√@[0≤] ¯4 9) ≡ (0≤)⍚[⊢ √] ¯4 9   ⍝ $t

⍝ — The derivative goes through the case that each cell picks
{+/(⍵>0)⍚[0 √]⍵}∂ 0 1 4   ⍝ 0 0.5 0.25

⍝ agenda:masked-unpicked — Where every case can run on every item, a select runs each case on all the items and keeps what each item picks: `÷` gives ∞ for the 0 that picks `⊢`, and no result keeps it
(0≥)⍚[÷ ⊢] ¯2 0 4   ⍝ ¯2 0 0.25

⍝ agenda:masked-explain — `•explain` names a select as a fast combination
(•explain "(0≥)⍚[÷ ⊢] ¯2 0 4").fast.combination   ⍝ ["masked select"]

⍝ agenda:masked-nan — A NaN or ¯0 item picks its case by the comparison's result, as it does without the select
(0>)⍚[⊢ -] ¯4 $n ¯0 2   ⍝ 4 $n ¯0 2

⍝ agenda:masked-kind — A 64-bit constant beside 32-bit items keeps the gathered path, whose result is 64-bit only where some item picks the constant
(•storage (0>)⍚[⊢ 0.5] 32 •float 1 2 3).kind   ⍝ "float32"

⍝ —
(•storage (0>)⍚[⊢ 0.5] 32 •float ¯1 2 3).kind   ⍝ "float64"

⍝ agenda:masked-narrow — A 32-bit constant case beside 64-bit items keeps the gathered path, whose result keeps the constant's width where every item picks it
(•storage (0<)⍚[⊢ (32 •float 0.5)] 1.5 2.5).kind   ⍝ "float32"

⍝ agenda:masked-sparse — Without fusion, a select costs less than gathering only where enough items pick a case that computes: for `-`, 1 in 10 isn't enough and 3 in 10 is
(•explain "-@[0>] ¯1 2 3 4 5 6 7 8 9 10").fast.combination   ⍝ 0⍴⊂""

⍝ —
(•explain "-@[0>] ¯1 ¯2 ¯3 4 5 6 7 8 9 10").fast.combination   ⍝ ["masked select"]

⍝ agenda:masked-sampled — Above the size where fusion starts, a select estimates the share its pending comparison picks from a sample: 1 item in 100 picked gathers, and half picked stays one fused select that computes nothing early
x←¿262144⍴0 ⋄ e←•explain "-@[0.5<]x" ⋄ s←•explain "-@[0.99<]x" ⋄ [≢e.fast.combination ≢e.forced.function ≢s.fast.combination]
[1 0 0]ₓ

⍝ —
x←¿262144⍴0 ⋄ [(-@[0.99<]x)≡x-2×x×0.99<x (-@[0.5<]x)≡x-2×x×0.5<x]
[$t $t]

⍝ agenda:masked-costly — A costly kernel such as `*` runs on every item in a select, so it needs more items to pick it: 5 in 10 keep the gathered path, and 7 in 10 take the select
(•explain "*@[0>] ¯1 2 ¯3 4 ¯5 6 ¯7 8 ¯9 10").fast.combination   ⍝ 0⍴⊂""

⍝ —
(•explain "*@[0>] ¯1 ¯2 ¯3 ¯4 ¯5 ¯6 ¯7 8 9 10").fast.combination   ⍝ ["masked select"]

⍝ agenda:masked-costly-fused — On a large array, fusion runs a select without counting its mask only for cheap kernels: for `*`, a mask that picks 1 item in 100 is counted, and keeps the gathered path
x←(⍳200000ₓ)÷200000 ⋄ (•explain "*@[0.99<]x").fast.combination   ⍝ 0⍴⊂""

⍝ agenda:masked-derivative — Under ∂, each case still runs only on the items that pick it, so `÷`'s infinite slope at 0 can't make the gradient NaN
{+/(⍵>0)⍚[0 ÷] ⍵}∂ 0 1 4   ⍝ 0 ¯1 ¯0.0625

⍝ agenda:masked-fused — On a large array, the comparison, the cases, the select and a fold of its result run as one fused graph
x←¯0.5+(⍳200000)÷200000 ⋄ (•explain "+/(0>)⍚[⊢ -] x").fused.glyphs   ⍝ "÷+" ">-⍚+/"

⍝⍝ Leading unit axis broadcasting

⍝ — Unit axes broadcast a column against a row; predicates return exact Booleans
[[1]⋄[2]]<[1 2 3⋄]   ⍝ [$f $t $t⋄$f $f $t]

⍝ —
[[1ₓ]⋄[2ₓ]]<[1 2 3⋄]ₓ   ⍝ [$f $t $t⋄$f $f $t]

⍝ — Rank pairs vector cells using broadcast frames
(2 1 2⍴1 2 3 4) +⍤1 [1 3 2]⍴10 20 30 40 50 60
2 3 2⍴11 22 31 42 51 62 13 24 33 44 53 64

⍝ — Unit axes extend independently across three dimensions
(2 1 2⍴1 2 3 4)+1 3 1⍴10 20 30
2 3 2⍴11 12 21 22 31 32 13 14 23 24 33 34

⍝ — An explicit axis aligns the vector with columns rather than rows
(2 3⍴⍳6ₓ)+⍠1 [10 20 30]   ⍝ [10 21 32⋄13 24 35]

⍝ — Axis permutation and unit-axis extension work together
[[10]⋄[20]]+⍠1 0 [[1]⋄[2]⋄[3]]   ⍝ [11 12 13⋄21 22 23]

⍝ — Broadcasting keeps each argument on its own side, in every direction
m←[1 2 3⋄4 5 6]
[m-10 20;10 20-m;m-⍠1 [10 20 30];[10 20 30]-⍠1 m;(1 3⍴1 2 3)-2 1⍴10 20;(2 1⍴10 20)-1 3⍴1 2 3]
⍝ =>
[[¯9 ¯8 ¯7⋄¯16 ¯15 ¯14] [9 8 7⋄16 15 14] [¯9 ¯18 ¯27⋄¯6 ¯15 ¯24] [9 18 27⋄6 15 24] [¯9 ¯8 ¯7⋄¯19 ¯18 ¯17] [9 8 7⋄19 18 17]]

⍝ — Broadcasting recurs inside nested arrays
(⊂[[1]⋄[2]])+⊂[10 20 30⋄]   ⍝ ⊂[11 21 31⋄12 22 32]

⍝ — Broadcast integer overflow promotes to exact big numbers
[9223372036854775807 1⋄]ₓ+[1 2]ₓ
[9223372036854775808 2⋄9223372036854775809 3]ₓ

⍝ — Rank-zero application extends a singleton frame
(,1) +⍤0 [1 2 3]   ⍝ 2 3 4

⍝ — A vector frame aligns with the leading matrix axis
1 2 +⍤0 [2 3]⍴0   ⍝ [1 1 1⋄2 2 2]

⍝ — A pervasive function under nested Rank pairs each level's frames
x←2 3⍴⍳6ₓ ⋄ m←[10 20 30⋄40 50 60⋄70 80 90]
x(-⍤1)⍤2 1⊢m
⍝ =>
3 2 3⍴¯10 ¯19 ¯28 ¯7 ¯16 ¯25 ¯40 ¯49 ¯58 ¯37 ¯46 ¯55 ¯70 ¯79 ¯88 ¯67 ¯76 ¯85

⍝ — Keyed frames align by key under Rank, and a missing key fills
x←"a" "b":[1 2⋄3 4] ⋄ y←"a" "c":[10 20⋄30 40] ⋄ x(+⍤1)y   ⍝ "a" "b" "c":[11 22⋄3 4⋄30 40]

⍝ — Under Rank, a key that one argument lacks reads a cell of that argument's prototype, as overtake fills a new row
x←"a" "b":2 2⍴[1 2;3;4;5 6] ⋄ y←"a" "c":[10 20⋄30 40] ⋄ :2⌷x({⍺}⍤1)y   ⍝ (0 0⋄0 0)

⍝ — A keyed vector cell keeps its keys on the result's cell axis
r←("p" "q":10 20)(+⍤1)2 2⍴⍳4ₓ ⋄ r.[: "q"]   ⍝ 21 23

⍝ — An empty frame keeps its shape under a pervasive function
⍴(0 3⍴0)+⍤1[1 2 3]   ⍝ [0 3]ₓ

⍝ — An empty frame whose call on the prototype cell fails gives an empty result, as in J
⍬ ({⍺√⍵}⍤0) ⍬   ⍝ ⍬

⍝ — That empty result keeps the frame's shape
m←(0 3)⍴0
⍴ m ({⍺√⍵}⍤0) m   ⍝ [0 3]ₓ

⍝ — A pervasive function gives the same empty result, at an outer or an inner level
[⍴(0 2⍴0)(+⍤1)0 3⍴0;⍴(2 0 2⍴0)((+⍤1)⍤2 1)2 3⍴0]   ⍝ [[0]ₓ;[2 0]ₓ]

⍝ — Frames that don't agree still raise an error when one is empty
⍬ ({⍺√⍵}⍤0) ⍳3ₓ
⍝ error: LENGTH ERROR

⍝ — Rank folds and scans along each cell's axis of the whole argument, as a call for each cell does
s←2 3 4⍴⍳24ₓ
[(+/⍤2 s)≡{+/⍵}⍤2 s;(+⌿⍤2 s)≡{+⌿⍵}⍤2 s;(-/⍤1 s)≡{-/⍵}⍤1 s;(+\⍤2 s)≡{+\⍵}⍤2 s;(×⍀⍤2 s)≡{×⍀⍵}⍤2 s]   ⍝ $t $t $t $t $t

⍝ — Rank's structural functions and reshapes act along the cell axes of the whole argument
s←2 3 4⍴⍳24ₓ
[(⊖⍤2 s)≡{⊖⍵}⍤2 s;(,⍤2 s)≡{,⍵}⍤2 s;((1↑)⍤2 s)≡{1↑⍵}⍤2 s;((1 2↑)⍤2 s)≡{1 2↑⍵}⍤2 s;((¯1↓)⍤2 s)≡{¯1↓⍵}⍤2 s;((1 2↣⌷)⍤2 s)≡{1 2⌷⍵}⍤2 s;((1⊖)⍤2 s)≡{1⊖⍵}⍤2 s;((4 3⍴)⍤2 s)≡{4 3⍴⍵}⍤2 s]   ⍝ $t $t $t $t $t $t $t $t

⍝ — Rank's reshape reads each cell's items in order, repeating them where the shape holds more and dropping them where it holds fewer
s←2 3 4⍴⍳24ₓ
[((5 4⍴)⍤2 s)≡{5 4⍴⍵}⍤2 s;((5⍴)⍤2 s)≡{5⍴⍵}⍤2 s;((2 3⍴)⍤1 s)≡{2 3⍴⍵}⍤1 s;((5↣⍴)⍤1⊢2 3⍴⍳6ₓ)≡[0 1 2 0 1⋄3 4 5 3 4]]   ⍝ $t $t $t $t

⍝ — Rank gives every cell's tally and shape at once
s←2 3 4⍴⍳24ₓ
[(≢⍤2 s)≡{≢⍵}⍤2 s;(≢⍤0 s)≡{≢⍵}⍤0 s;(⍴⍤2 s)≡{⍴⍵}⍤2 s;(⍴⍤0 s)≡{⍴⍵}⍤0 s]   ⍝ $t $t $t $t

⍝ — Rank applies each part of an atop by its own rule
s←2 3 4⍴⍳24ₓ
[((+/,)⍤2 s)≡{+/,⍵}⍤2 s;((-/,)⍤2 s)≡{-/,⍵}⍤2 s;((⌽,)⍤2 s)≡{⌽,⍵}⍤2 s;((⊖⊖)⍤2 s)≡{⊖⊖⍵}⍤2 s]   ⍝ $t $t $t $t

⍝ — Rank's whole-array rules keep the keys and names that a call for each cell keeps
n←["i":"a" "b";"j":"p" "q" "r"]:[1 2 3⋄4 5 6] ⋄ m←["r":3 3]⍴⍳9ₓ ⋄ i←⍳["r":3]
[(+/⍤1 n)≡{+/⍵}⍤1 n;(⊖⍤1 n)≡{⊖⍵}⍤1 n;((2↑)⍤1 n)≡{2↑⍵}⍤1 n;(≢⍤1 n)≡{≢⍵}⍤1 n;(⍴⍤1 n)≡{⍴⍵}⍤1 n;((+/,)⍤1 n)≡{+/,⍵}⍤1 n;(([i]⌷)⍤1 m)≡{[i]⌷⍵}⍤1 m]   ⍝ $t $t $t $t $t $t $t

⍝ — Rank's whole-array rules agree with a call for each cell on an empty frame and on windows
e←0 3 4⍴0 ⋄ w←3 3↕4 5⍴⍳20ₓ
[(+/⍤2 e)≡{+/⍵}⍤2 e;(≢⍤2 e)≡{≢⍵}⍤2 e;(⍴⍤2 e)≡{⍴⍵}⍤2 e;((1↑)⍤2 e)≡{1↑⍵}⍤2 e;((+/,)⍤2 e)≡{+/,⍵}⍤2 e;((+/,)⍤2 w)≡{+/,⍵}⍤2 w]   ⍝ $t $t $t $t $t $t

⍝ — Rank's whole-array rules give the gradient a call for each cell gives
m←2 3⍴⍳6ₓ
({+/,(+/⍤1)⍵×⍵}∂ m)≡{+/,({+/⍵}⍤1)⍵×⍵}∂ m   ⍝ $t

⍝ — A dyad applies to the cells of its declared ranks when an argument's rank is higher
(2 1⍴1 2)↑⍳5ₓ   ⍝ [0 0⋄0 1]ₓ

⍝ —
(2 1⍴0 2)⌷"abc"   ⍝ "ac"

⍝ — The results of different shapes pad to a common shape
(2 2⍴2 3 1 2)⍴⍳6ₓ
(2 2 3⍴0 1 2 3 4 5 0 1 0 0 0 0)ₓ

⍝ — A right argument of rank 3 divides by each matrix
y←(2 2 2)⍴1 0 0 1 2 0 0 2
(2 2⍴1 0 0 1)⌹y
⍝ =>
2 2 2⍴1 0 0 1 0.5 0 0 0.5

⍝ — Each and rank 0 align leading axes as pervasive functions do
x←2 3⍴⍳6ₓ ⋄ [x+10 20;x+¨10 20;x+⍤0[10 20]]
3⍴⊂[10 11 12⋄23 24 25]

⍝ — Rank 0 passes a scalar argument to every cell, where Each passes the scalar's item
[1 2 3 {≢⍵}⍤0 ⊂4 5;1 2 3 {≢⍵}¨ ⊂4 5]   ⍝ (1 1 1⋄2 2 2)ₓ

⍝ —
a←[10⋄20]
b←[1 2 3⋄]
[a+b;a+¨b;a+⍤0 b]
⍝ =>
3⍴⊂[11 12 13⋄21 22 23]

⍝ —
a←[10⋄] ⋄ [a+1 2 3;a+¨1 2 3;a+⍤0[1 2 3]]
3⍴⊂[11⋄12⋄13]

⍝ — An empty frame broadcasts too
a←1 0⍴0ₓ
b←2 1⍴0ₓ
[a+b;a+¨b;a+⍤0 b]   ⍝ 3⍴⊂2 0⍴0ₓ

⍝ — Unequal axes longer than one don't broadcast
(2 3⍴0)+1 2 3
⍝ error: LENGTH ERROR

⍝ —
(2 3⍴0)+¨1 2 3
⍝ error: LENGTH ERROR

⍝ —
(2 3⍴0)+⍤0[1 2 3]
⍝ error: LENGTH ERROR

⍝ —
(0 2⍴0)+3 2⍴0
⍝ error: LENGTH ERROR

⍝ —
(0 2⍴0)+¨3 2⍴0
⍝ error: LENGTH ERROR

⍝ —
(0 2⍴0)+⍤0(3 2⍴0)
⍝ error: LENGTH ERROR

⍝ — Broadcast exact integers keep integer storage
x←[10⋄20]ₓ+[1 2 3⋄]ₓ ⋄ [x (•storage x).kind]   ⍝ [[11 12 13⋄21 22 23]ₓ;"integer"]

⍝ — An empty Each calls its operand once on prototypes, and the call's output shows
(1 0⍴0){⎕←9 ⋄ ⍺+⍵}¨(2 1⍴0)
2 0⍴0
⍝ ⎕: 9

⍝ —
(1 0⍴0){⎕←9 ⋄ ⍺+⍵}⍤0(2 1⍴0)
2 0⍴0
⍝ ⎕: 9

⍝⍝ Selective assignment

⍝ — Selection through identity and disclose replaces the whole nested item
a←[1 2;3 4]
(⊢↑a)←7 8 9
a   ⍝ [7 8 9;3 4]

⍝ — Identity selection may replace the whole array with a different shape
a←1 2
(⊣a)←3 4 5
a   ⍝ 3 4 5

⍝ — An empty pick path selects the whole atom for replacement
a←1
(⍬⊃a)←3 4
a   ⍝ 3 4

⍝ — Whole-array replacement also works when the old array is empty
a←⍬
(⍬⊃a)←3 4
a   ⍝ 3 4

⍝ — Modified assignment through an empty pick path can resize the array
a←1 2
(⍬⊃a),←3 4
a   ⍝ 1 2 3 4

⍝ — Whole-item replacement composes with a nested pick
a←[1 2;3 4]
(⍬⊃0⊃a)←5 6 7
a   ⍝ [5 6 7;3 4]

⍝ — Assignment through reverse maps replacements back to original positions
a←1 2
(⍬⊃⌽a)←3 4
a   ⍝ 4 3

⍝ — Ravel selection cannot resize its source through an empty pick path
a←1 2 ⋄ (⍬⊃,a)←3 4 5
⍝ error: LENGTH ERROR

⍝ — Squad selection still requires replacement-shape agreement
a←1 2 ⋄ (⍬⊃⍬⌷a)←2 2⍴3 4
⍝ error: LENGTH ERROR

⍝ — Replace vowels selected by a Boolean mask
a←"HELLO"
(a∊"AEIOU" # a)←'*'
a   ⍝ "H*LL*"

⍝ — Assign through a ravel prefix without changing matrix shape
z←3 4⍴⍳12ₓ
(5↑,z)←0ₓ
,z
⍝ =>
[0 0 0 0 0 5 6 7 8 9 10 11]ₓ

⍝ — Repeated transpose axes select the diagonal
m←3 3⍴⍳9ₓ
(0 0⍉m)←0ₓ
,m   ⍝ [0 1 2 3 0 5 6 7 0]ₓ

⍝ — Selection through enlist updates characters inside nested vectors
a←"Andy" "Karen" "Liam"
(('a'=∊a)#∊a)←'*'
a   ⍝ "Andy" "K*ren" "Li*m"

⍝ — Each selects a prefix in every nested vector
a←"HELLO" "WORLD"
(2↑¨a)←'*'
a   ⍝ "**LLO" "**RLD"

⍝ — Each uses a separate mask for each nested vector
a←"HELLO" "WORLD"
(a='O' #¨ a)←'*'
a   ⍝ "HELL*" "W*RLD"

⍝ — A selection written with spaces finds its array as the touching form does
lv←1 2 3
e←0 1 0
(e # 1⌽lv)←9
lv   ⍝ 1 2 9

⍝ — Replacing one nested item may change its length
a←[1 2;3 4]
(1↑a)←⊂8 9 10
a   ⍝ [8 9 10;3 4]

⍝ — Repeated selection updates the same source element more than once
a←3⍴0ₓ
(5⍴a)+←1ₓ
a   ⍝ [2 2 1]ₓ

⍝ — Reverse composes with an index selection
a←1 2 3
(⌽[[0 2]]⌷a)←8 9
a   ⍝ 9 2 8

⍝ — An empty selection leaves the source unchanged
a←1 2 3
(0↑a)←9
a   ⍝ 1 2 3

⍝ — Empty each-selection preserves the nested prototype shape
a←0⍴⊂2 3⍴⍳6ₓ
(⌽⍠1¨a)←9
⍴↑a   ⍝ [2 3]ₓ

⍝ — Overtake's fill positions do not create new source elements
a←1 2
(3↑a)←4
a   ⍝ 4 4

⍝ —
a←[1 2;3 4]
(↑a)←7 8 9
a₀   ⍝ 7 8 9

⍝ — First selection replaces an atom with a vector
a←1
(↑a)←3 4
a   ⍝ 3 4

⍝ — Disclose-each selects the first element of each nested vector
a←[1 2;3 4]
(↑¨a)←[5 6;7 8]
a   ⍝ [[[5 6] 2] [[7 8] 4]]

⍝ — A bound take function remains assignment-selective
a←1 2 3
(1↣↑a)←9
a   ⍝ 9 2 3

⍝ — A fork in a target reads the target's values in its left tine, as under Under
a←10 20 30 40
((15<)↣# a)←0
a   ⍝ 10 0 0 0

⍝ — A bound pick function can replace an item with a nested vector
a←1 2
(0↣⊃a)←3 4
a   ⍝ [[3 4] 2]

⍝ — Binding an empty pick path retains whole-array replacement
a←1 2
(⍬↣⊃a)←3 4 5
a   ⍝ 3 4 5

⍝ — Bound pick under each updates the selected nested elements
a←[1 2;3 4]
(0↣⊃¨a)←[5 6;7 8]
a   ⍝ [[[5 6] 2] [[7 8] 4]]

⍝ — The first item of an empty array is a fill, so a write to it is dropped
a←⍬ ⋄ (↑a)←3 4 ⋄ a   ⍝ ⍬

⍝ — The selection expression executes only once
a←1 2
((⎕←1)#a)←3
a
⍝ =>
3 3
⍝ ⎕: 1

⍝ — First, pick and a named function select the same item for replacement
a←1 2
(↑a)←3 4
a   ⍝ [[3 4] 2]

⍝ —
a←1 2
(0⊃a)←3 4
a   ⍝ [[3 4] 2]

⍝ —
first←↑
a←1 2
(first a)←3 4
a   ⍝ [[3 4] 2]

⍝ — A replacement must fit what the selection returns
a←1 2 ⋄ (,↑a)←2 2⍴3 4
⍝ error: LENGTH ERROR

⍝ —
a←1 2 ⋄ (↑¨a)←2 2⍴3 4
⍝ error: LENGTH ERROR

⍝ — Arithmetic is not a selection
a←1 2 ⋄ (1+a)←0
⍝ error: DOMAIN ERROR

⍝ dfn-selector-assign — A dfn selects in an assignment as its body does
v←10 20 30
{⍵.[⍸⍵>15]}v ←0
v   ⍝ 10 0 0

⍝ —
v←10 20 30
{2↑⌽⍵}v ←1 2
v   ⍝ 10 2 1

⍝ dfn-selector-strand — A strand of selected items writes each one back
v←10 20 30
{[⍵₀ ⍵₂]}v ←1 2
v   ⍝ 1 20 2

⍝ dfn-selector-foreign — An item that a selector makes rather than selects can't be written
v←10 20 30 ⋄ {[⍵₀ 5]}v ←1 2
⍝ error: DOMAIN ERROR

⍝ assign-ravel — A ravel target writes back into the matrix in order
m←[2 3]⍴⍳6ₓ ⋄ (,m)←⌽⍳6ₓ ⋄ m   ⍝ [5 4 3⋄2 1 0]ₓ

⍝ assign-transpose — A transposed target writes each item back to its own place
m←[2 2]⍴⍳4ₓ ⋄ (⍉m)←[10 20⋄30 40]ₓ ⋄ m   ⍝ [10 30⋄20 40]ₓ

⍝ assign-keys-align — Replacement keys line up with the keys the target selects, in its order
k←["a":1 "b":2 "c":3] ⋄ ([["b" "a"]]⌷k)←["a":10 "b":20] ⋄ k   ⍝ ["a":10 "b":20 "c":3]

⍝ assign-ravel-length — A replacement of another length doesn't fit the target
v←1 2 3 ⋄ (,v)←4 5
⍝ error: LENGTH ERROR

⍝⍝ Modified, selective and destructuring assignment

⍝ — Modified assignment updates an outer lexical binding
a←10
f←{a+←⍵ ⋄ a}
z←f3
z,a   ⍝ 13 13

⍝ — A local array can shadow an outer operator
o←¨ ⋄ {o←3 ⋄ o}0   ⍝ 3

⍝ — Destructuring creates local names even when an outer name is an operator
o←¨ ⋄ {[a o]←3 4 ⋄ [a o]}0   ⍝ 3 4

⍝ — A local function can shadow an outer operator
o←¨ ⋄ {o←{⍵} ⋄ o3}0   ⍝ 3

⍝ — Modified assignment finds the nearest lexical binding
a←10
f←{a←2 ⋄ g←{a+←⍵ ⋄ a} ⋄ z←g ⍵ ⋄ z,a}
z←f3
z,a   ⍝ 5 5 10

⍝ — The caller's local name does not redirect a callee's lexical assignment
a←10
g←{a+←⍵ ⋄ a}
f←{a←2 ⋄ z←g ⍵ ⋄ z,a}
z←f3
z,a   ⍝ 13 2 13

⍝ — Updating an outer array leaves a previously assigned copy unchanged
a←1 2
b←a
f←{a₀←⍵ ⋄ a}
z←f3
z,a,b   ⍝ 3 2 3 2 1 2

⍝ — Writing into an array that shares only another's storage leaves the other unchanged
x←1 2 3
y←,x
y₀←9
x,y   ⍝ 1 2 3 9 2 3

⍝ — Writing inside a nested item that another item shares leaves the other unchanged
a←1 2
n←[a a]
n₀.[1]←9
[n a]   ⍝ ([1 9] [1 2]⋄1 2)

⍝ — Writing an array into one of its own items stores the array as it was
x←1 2 3
x₀←x
x   ⍝ [[1 2 3] 2 3]

⍝ — Appending leaves an array that shares the original unchanged
a←1 2
b←a
a,←3
[a b]   ⍝ [[1 2 3] [1 2]]

⍝ — Appending to an empty compact array takes the kind of the new items, as joining does
r←⍬
r,←$t
s←(•storage r).kind
r,←2ₓ
[s (•storage r).kind r]
⍝ =>
["bool" "integer" [1 2]ₓ]

⍝ — Joining short lanes of an integer array and a float array converts the integers as they're written
a←[5000 2]⍴⌊⍳10000 ⋄ b←a+0.5 ⋄ +/+/a,b
⍝ =>
99995000

⍝ — An empty mixed array takes its prototype from the first appended item
r←0⍴⊂1 2
r,←⊂"abc"
⍴↑0⍴r   ⍝ [3]ₓ

⍝ — Appending keeps keys
r←["a":1]
r,←["b":2]
r   ⍝ ["a":1 "b":2]

⍝ — Appending a key the array already has is an error
r←["a":1] ⋄ r,←["a":2]
⍝ error: DOMAIN ERROR

⍝ — A local dfn can't be appended to an outer array
x←1 2 ⋄ {x,←{⍵} ⋄ 0}0
⍝ error: DOMAIN ERROR

⍝ — Appending rows to a matrix spreads a unit into a row
m←[1 2⋄3 4]
m⍪←5 6
m⍪←0
m   ⍝ [1 2⋄3 4⋄5 6⋄0 0]

⍝ — A new key leaves an array that shares the original unchanged
T←["a":1]
T.b←2
U←T
T.c←3
[T U]
⍝ =>
[["a":1 "b":2 "c":3] ["a":1 "b":2]]

⍝ — A new key inside a nested record, and a new record on the way to a key
T←["a":["x":1]]
T.a.y←2
T.b.c←3
T   ⍝ ["a":["x":1 "y":2] "b":["c":3]]

⍝ — Selective modified assignment can update an outer array
a←1 2
f←{(⌽a)+←⍵ ⋄ a}
z←f3
z,a   ⍝ 4 5 4 5

⍝ — Modified selective assignment writes as plain assignment does: a float leaves the exact integers beside it exact, and a number written into Booleans leaves them mixed
x←[1 2 3]ₓ
x₀+←0.5
b←1 0 1=1
b₀+←1
[x b]   ⍝ [1.5 2ₓ 3ₓ;2 $f $t]

⍝ — Repeated positions accumulate, with a primitive or a dfn
x←0 0 0
y←x
x.[[1 1]]+←1
y.[[1 1]]{⍺+⍵}←1
x,y   ⍝ 0 2 0 0 2 0

⍝ — Repeated positions accumulate at the kind that the function gives. An integer that outgrows its width moves to a wider one, and past 64 bits to a big integer, as `+` gives it
h←[0 0]ₓ
h.[300⍴0]+←1ₓ
w←[9223372036854775807]ₓ
w.[[0 0]]+←1ₓ
h,w   ⍝ 300ₓ 0ₓ 9223372036854775809ₓ

⍝ — 16-bit floats accumulate in 32 bits and round once
g←[2048]ₕ
g.[[0 0 0 0]]+←1ₕ
g   ⍝ [2052]ₕ

⍝ — An exact quotient that isn't whole gives a rational, as `÷` does. The targets before it keep their results
z←[6 7]ₓ
z.[[0 1]]÷←2ₓ
z   ⍝ 3ₓ 7ᵣ2

⍝ — A target at a fill reads the fill, and its write is dropped
y←10 20 30 40
y.[[1 $n]]+←5
y   ⍝ 10 25 30 40

⍝ — A modifier that reassigns or erases its target doesn't change the update
x←1 2 3
x₀{x,←5 ⋄ ⍺+⍵}←10
y←1 2 3
y₀{•ex "y" ⋄ ⍺+⍵}←10
x,y   ⍝ 11 2 3 11 2 3

⍝ — An error in the modifier leaves the target unchanged
x←1 2 3 ⋄ {"DOMAIN"::x ⋄ x.[[0 1]]{⍵=2?÷'a';⍺+⍵}←1 2}0   ⍝ 1 2 3

⍝ — An index error leaves the target unchanged
x←1 2 3 ⋄ {"INDEX"::x ⋄ x.[[0 5]]←9}0   ⍝ 1 2 3

⍝ — A replacement of the wrong length leaves the target unchanged
x←1 2 3 ⋄ {"LENGTH"::x ⋄ x.[[0 1]]←1 2 3}0   ⍝ 1 2 3

⍝ — A rejected local closure leaves the target unchanged
x←1 2 3 ⋄ {"DOMAIN"::x ⋄ x₀←{⍵} ⋄ 0}0   ⍝ 1 2 3

⍝ — A local error guard does not roll back writes to an outer binding
a←10
f←{∞::a ⋄ a+←⍵ ⋄ 1÷'a'}
z←f3
z,a   ⍝ 13 13

⍝ — A handler sees local bindings as they were when the error happened
{a←2 ⋄ ∞::a ⋄ a+←3 ⋄ 1÷'a'}0   ⍝ 5

⍝ —
1+a←3   ⍝ 4

⍝ — Applying a function to an assignment result makes it non-shy
{1+a←3}0   ⍝ 4

⍝ — Modified assignment yields its right argument, not the updated binding
{a←1 ⋄ +a+←3}0   ⍝ 3

⍝ —
a←1+b←2 ⋄ [a b]   ⍝ 3 2

⍝ — Modified assignment to a selection yields the supplied increment
a←1 2 3 ⋄ 2×a₂+←10   ⍝ 20

⍝ — An enclosing expression sees the replacement in selection order
a←1 2 3 ⋄ 1+(⌽a)←4 5 6   ⍝ 5 6 7

⍝ —
[[a b] c]←[[3 4] 5] ⋄ [a b c]   ⍝ 3 4 5

⍝ — A named reverse function remains assignment-selective
a←1 2 3
rev←⌽
(rev a)←4 5 6
a   ⍝ 6 5 4

⍝ — At top level a named function between the target and the arrow performs modified assignment
a←1
f←+
a(f)←3
a   ⍝ 4

⍝ — In a dfn, brackets destructure into local names, whatever the outer binding
{a←1 ⋄ f←+ ⋄ [a f]←3 ⋄ [a f]}0   ⍝ 3 3

⍝ — In a dfn, deriving the modifier marks it as a function rather than a local array
{a←1 ⋄ f←+ ⋄ a(f↢⊢)←3 ⋄ a}0   ⍝ 4

⍝ — In a dfn, a named function directly after an array performs modified assignment, in either spelling
{a←1 ⋄ f←+ ⋄ a(f)←3 ⋄ (a)f←3 ⋄ a}0   ⍝ 7

⍝ — A named rank operand binds before modified assignment
a←1 2
r←0
1+a+⍤r←3 4   ⍝ 4 5

⍝ — Inside the parentheses of a modifier, spaces group as they do in a value
m←[0 1 2⋄3 4 5]
m(⌽@ 1⌷)←
m   ⍝ [0 1 2⋄5 4 3]

⍝ —
a←1 2 3
a(+⍤ 0)←1
a   ⍝ 2 3 4

⍝ — A parenthesised array after the target is still a strand, even with a space inside
a←1 ⋄ b←2 ⋄ a(b+ 1)←3
⍝ error: SYNTAX ERROR

⍝ — A space before the arrow makes the whole run before it the target, as if parenthesised
m←[1 2⋄3 4] ⋄ [[0] [1]]⌷m ←0 ⋄ m   ⍝ [1 0⋄3 4]

⍝ — A space before a modifier does the same for modified assignment
m←[1 2⋄3 4] ⋄ x←3 ⋄ [[0] [1]]⌷m +←10 ⋄ x (-)← ⋄ [m x]   ⍝ [[1 12⋄3 4] ¯3]

⍝ —
a←1
b←2
[a b]+←3 4
[a b]   ⍝ 4 6

⍝ —
a←1
b←2
[a b]+←3
[a b]   ⍝ 4 5

⍝ — With nothing after the arrow, modified assignment applies the function monadically
a←1 2 3
a⌽←
a   ⍝ 3 2 1

⍝ —
a←1 2 3
f←{⍵×10}
a(f)←
a   ⍝ 10 20 30

⍝ — Monadic modified assignment updates the selected positions
v←1 2 3 4
v.[[0 2]]-←
v   ⍝ ¯1 2 ¯3 4

⍝ — Monadic modified assignment applies the function to each destructured name
a←1 2
b←3 4 5
[a b]⌽←
a,b   ⍝ 2 1 5 4 3

⍝ — A plain name with nothing after the arrow is still an error
a←
⍝ error: SYNTAX ERROR

⍝ — Repeated names receive successive modified assignments
a←1
[a a]+←3 4
a   ⍝ 8

⍝ —
a←1 2 3
a₁←9
a   ⍝ 1 9 3

⍝ — Repeated indices accumulate rather than overwriting from the original value
a←1 2 3
r←a.[[1 1]]+←10 20
a   ⍝ 1 32 3

⍝ — The result of modified assignment is the original right argument
a←1 2 3
r←a.[[1 1]]+←10 20
r   ⍝ 10 20

⍝ — Repeated row and column indices multiply the number of updates
a←3 5⍴0
[0 0 2;0 2 2 4]⌷a +←1
,a   ⍝ 2 0 4 0 2 0 0 0 0 0 1 0 2 0 1

⍝ — Assignment to a dot path preserves value semantics for an earlier copy
a←1 2 3
b←a
a.[[0 2]]←8 9
b   ⍝ 1 2 3

⍝ —
a←[1 2]ₓ
a×←2ₓ
a   ⍝ [2 4]ₓ

⍝ — Modified integer assignment promotes on overflow
a←1ₓ
a+←9223372036854775807ₓ
a   ⍝ 9223372036854775808ₓ

⍝ — Left-to-right evaluation lets a later bracket item see an earlier item's update
a←1
f←{a+←1 ⋄ a}
[f0 a]   ⍝ 2 2

⍝ —
[a [b c]]←[1 [2 3]] ⋄ [a b c]   ⍝ 1 2 3

⍝ —
[a b]←1 ⋄ [a b]   ⍝ 1 1

⍝ — Successive dot selections map back to the original array
a←1 2 3
a.[[2 0]].[1]←9
a   ⍝ 9 2 3

⍝ — A pick path updates an element inside the second item
a←[1 2;3 4]
[1 0]⊃a ←9
↑a₁   ⍝ 9

⍝ — Destructuring modification calls the operand from left to right
a←1
b←2
[a b]{⎕←⍺ ⋄ ⍺+⍵}←3 4
[a b]
⍝ =>
4 6
⍝ ⎕: 1\n2

⍝ — The right-hand assignment runs before the parenthesized read
a←0 ⋄ (⎕←a)+a←⎕←3
6
⍝ ⎕: 3\n3

⍝ — A modifying dfn may read the array it is updating
a←1 2
f←{⎕←a ⋄ ⍺+⍵}
a.[[0 1]]f←10 20
a   ⍝ 11 22

⍝⍝ General axis forms

⍝ — Ravel merges adjacent selected axes. Dyalog 20.0.53963.0, IO=1, CT=1e-14, ML=1.
⍴,⍠1 2 (2 3 4⍴⍳24ₓ)   ⍝ [2 12]ₓ

⍝ — Fractional axes are removed, so ravel cannot insert a unit axis between existing axes
⍴,⍠0.5 (2 3⍴⍳6ₓ)
⍝ error: DOMAIN ERROR

⍝ — Empty-axis ravel appends a unit axis
⍴,⍠⍬ (2 3⍴⍳6ₓ)   ⍝ [2 3 1]ₓ

⍝ — Mix places the item axis at the given position
⊃⍠0 [1 2;3 4]   ⍝ [1 3⋄2 4]

⍝ — Mix places the matrix-cell axes at positions zero and two
⊃⍠0 2 [2 3⍴⍳6ₓ;2 3⍴6ₓ+⍳6ₓ]   ⍝ (2 2 3⍴0 1 2 6 7 8 3 4 5 9 10 11)ₓ

⍝ — A single Mix axis outside the result rank is a domain error, as axis vectors are
⊃⍠3 [2 3⍴⍳6ₓ;2 3⍴6ₓ+⍳6ₓ]
⍝ error: DOMAIN ERROR

⍝ — Laminate is removed, so catenate takes no fractional axis
1 2,⍠0.5 [3 4]
⍝ error: DOMAIN ERROR

⍝ — Pervasive-function axis zero aligns the vector with rows
1 2+⍠0 (2 3⍴⍳6ₓ)   ⍝ [1 2 3⋄5 6 7]

⍝ — Take counts follow the specified axis order
2 1↑⍠1 0 (3 4⍴⍳12ₓ)   ⍝ [0 1⋄]ₓ

⍝ — Drop counts follow the specified axis order
1 1↓⍠1 0 (3 4⍴⍳12ₓ)   ⍝ [5 6 7⋄9 10 11]ₓ

⍝ — Pick along an axis selects one position on that axis
0⊃⍠1 (2 3⍴⍳6ₓ)   ⍝ [0 3]ₓ

⍝ — First along an axis takes position 0 on that axis
↑⍠1 (2 3⍴⍳6ₓ)   ⍝ [0 3]ₓ

⍝ — Squad's index vectors correspond to the listed axes
[1 0;0 1]⌷⍠1 0 (2 3⍴⍳6ₓ)   ⍝ [1 0⋄4 3]ₓ

⍝ — Enclosing all axes in reverse order transposes the enclosed cell
↑⊂⍠1 0 (2 3⍴⍳6ₓ)   ⍝ [0 3⋄1 4⋄2 5]ₓ

⍝ — Axis enclosure leaves the unselected frame, including its zero dimension
⍴⊂⍠2 (2 0 4⍴0ₓ)   ⍝ [2 0]ₓ

⍝ — Empty axis-enclosure retains the length-four cell prototype
⍴↑↑⊂⍠2(2 0 4⍴0ₓ)   ⍝ [4]ₓ

⍝ — Ravel axes must be adjacent. Dyalog 20.0.53963.0, IO=1, CT=1e-14, ML=1.
,⍠0 2 (2 3 4⍴⍳24ₓ)
⍝ error: DOMAIN ERROR

⍝ — Enclose axes are distinct
⊂⍠0 0 (2 3⍴⍳6ₓ)
⍝ error: DOMAIN ERROR

⍝ — A pervasive function's axes must match the lower-rank argument
1+⍠0 (2 3⍴⍳6ₓ)
⍝ error: LENGTH ERROR

⍝ — Take counts must match the listed axes
2↑⍠0 1 (3 4⍴⍳12ₓ)
⍝ error: LENGTH ERROR

⍝⍝ The axis operator

⍝ — ⍠ selects axes. Reduce keeps its own axis meaning
+/⍠0 (3 4⍴⍳12ₓ)   ⍝ [12 15 18 21]ₓ

⍝ — A plain number is still a cell rank
+/⍤1 [3 4]⍴⍳12ₓ   ⍝ [6 22 38]ₓ

⍝ — A name can hold the axes
ax←0 ⋄ +/⍠ax 3 4⍴⍳12ₓ   ⍝ [12 15 18 21]ₓ

⍝ — A dfn applies to the cells made of the selected axes
{+/⍵}⍠0 (3 4⍴⍳12ₓ)   ⍝ [12 15 18 21]ₓ

⍝ — A primitive without its own axis meaning follows the general rule
≢⍠0 (3 4⍴⍳12ₓ)   ⍝ [3 3 3 3]ₓ

⍝ — Result cells of the selected rank return to the selected axes
{⌽⍵}⍠0 (3 4⍴⍳12ₓ)   ⍝ ⊖3 4⍴⍳12ₓ

⍝ — Other result cells start at the first selected axis
{,⍵}⍠0 1 (2 3 4⍴⍳24ₓ)   ⍝ ,⍠0 1 (2 3 4⍴⍳24ₓ)

⍝ — An argument without the selected axes is one whole cell
1 0 1 0{⍺#⍵}⍠1 (3 4⍴⍳12ₓ)   ⍝ 1 0 1 0#⍠¯1(3 4⍴⍳12ₓ)

⍝ —
1{⍺⌽⍵}⍠0 (3 4⍴⍳12ₓ)   ⍝ 1⊖3 4⍴⍳12ₓ

⍝ — Both arguments supply cells when both have the selected axes
(2 3⍴⍳6ₓ){⍺,⍵}⍠0 (2 3⍴⍳6ₓ)   ⍝ (2 3⍴⍳6ₓ)⍪2 3⍴⍳6ₓ

⍝ — Names select axes
m←["row":3 "col":4]⍴⍳12ₓ ⋄ ({+/⍵}⍠"row" m)≡{+/⍵}⍠0 m   ⍝ $t

⍝ — Exception: multi-axis reduce flattens each cell
+/⍠1 2 (2 3 4⍴⍳24ₓ)   ⍝ [66 210]ₓ

⍝ — Exception: a count array rotates each cell by its own count
1 2 3 4⌽⍠0 (3 4⍴⍳12ₓ)   ⍝ [4 9 2 7⋄8 1 6 11⋄0 5 10 3]ₓ

⍝⍝ Brackets

⍝ — Brackets round one item make a one-item vector
[[1 2]]   ⍝ ,⊂1 2

⍝ —
[1]≡,1   ⍝ $t

⍝ — Spaces inside brackets separate items
≢[1 2]   ⍝ 2ₓ

⍝ — Semicolons separate items only inside brackets
1;2
⍝ error: SYNTAX ERROR

⍝ — Assignment through ⌷ adds a missing key
T←"aa" "bb":1 2
"cc"⌷T ←3
T   ⍝ "aa" "bb" "cc":1 2 3

⍝ — Adding a key to an axis with no keys gives the new position a key and leaves the others without
v←10 20 30
"x"⌷v ←9
v   ⍝ [10 20 30 "x":9]

⍝ — Dot assignment adds a named position to an unkeyed vector
v←10 20 30
v.x←9
v   ⍝ [10 20 30 "x":9]

⍝ — A matrix with no row keys gains a named row
M←[1 2 3⋄4 5 6]
"r"⌷M ←7 8 9
M   ⍝ 0 1 "r":[1 2 3⋄4 5 6⋄7 8 9]

⍝ — A strand of keys or a bracketed key array also adds missing keys
M←["aa" "bb";"xx" "yy"]:[1 2⋄3 4]
"cc" "zz"⌷M ←9
"cc" "zz"⌷M   ⍝ 9

⍝ —
T←"aa":1
[["cc" "dd"]]⌷T ←3 4
T   ⍝ "aa" "cc" "dd":1 3 4

⍝ — Dot access after a group that contains dot access
inner←"bb" "cc":1 2
x←["aa":inner "zz":0]
(x.aa).bb   ⍝ 1

⍝ —
inner←"bb" "cc":1 2
x←["aa":inner "zz":0]
(x.aa).bb←5
x.aa   ⍝ "bb" "cc":5 2

⍝⍝ Dot indexing

⍝ — A group after the dot evaluates with the array's keys as names, beside the caller's names
T←["a":1 "b":2]
c←100
T.(a+b+c)   ⍝ 103

⍝ — Assignment inside a scope is local to it
T←["a":1]
r←T.(a←5)
[r T.a]   ⍝ 5 1

⍝ — A key holding a function binds as a function
T←["f":{⍵×2} "a":3] ⋄ T.(f a)   ⍝ 6

⍝ — A scope inside a dfn sees the dfn's ⍺
5 {⍵.(a+⍺)} ["a":1]   ⍝ 6

⍝ — In a scope at top level, ⍺ is an undefined name, as it is at top level itself
["a":1].(⍺-a)
⍝ error: VALUE ERROR

⍝ — •ex erases bindings, not a scope's keys: a key still names its item after •ex
T←["a":1] ⋄ T.({_←•ex "a" ⋄ a} 0)   ⍝ 1

⍝ — A key spelled as one of a dfn's own names names nothing: ⍵ comes from the call
f←{["⍵":0].(⍵+1)} ⋄ f 5   ⍝ 6

⍝ — Over a keyed matrix, each key names its major cell, as dot access selects it
T←"a" "b":[1 2 3⋄4 5 6] ⋄ T.(a+b)   ⍝ 5 7 9

⍝ — A scope needs keys on its first axis
m←3 4⍴⍳12ₓ ⋄ m.(1 2)
⍝ error: DOMAIN ERROR

⍝ — Brackets after the dot hold one index item for each axis, so a one-item vector of positions selects rows
m←3 4⍴⍳12ₓ ⋄ m.[[2 0]]   ⍝ [8 9 10 11⋄0 1 2 3]ₓ

⍝ — Each unspaced expression in brackets is one index item
m←3 4⍴⍳12ₓ
k←⍳3ₓ
m.[k~[1] 3]   ⍝ [3 11]ₓ

⍝ — Dot indexing chains from left to right
A←2 3 4⍴⍳24ₓ ⋄ A.[1 : 0].[¯1]   ⍝ 20ₓ

⍝ — Dot indexing binds tightly inside a larger expression
v←10 20 30 ⋄ 9,v.[1],3   ⍝ 9 20 3

⍝ — Dot indexing follows a literal
"abc".[[2 0]]   ⍝ "ca"

⍝ — Dot indexing follows a constant
$a.[[2 0]]   ⍝ "CA"

⍝ — A decimal point needs a digit after it, so a dot after a number is not part of it
2.
⍝ error: SYNTAX ERROR

⍝ — Sorting through the dot: brackets round one item make a one-item vector, which selects along the first axis
v←3 1 2 ⋄ v.[⍋v]   ⍝ 1 2 3

⍝ — Bracket indexing pairs its arrays of positions
m←2 2⍴⍳4ₓ ⋄ m.[0 1;1 0]   ⍝ [1 2]ₓ

⍝ — Assignment to a dot path goes through ⌷
v←10 20 30
v.[[2 0]]←7 8
v   ⍝ 8 20 7

⍝ —
m←2 2⍴⍳4ₓ
m.[: 1]←0ₓ
m   ⍝ [0 0⋄2 0]ₓ

⍝ — Dot assignment adds missing keys
T←"aa":1
T.[["bb" "cc"]]←2 3
T   ⍝ "aa" "bb" "cc":1 2 3

⍝ — An assignment inside the container of a dot assignment is a syntax error, raised before anything runs
T←["a":1] ⋄ (⎕←T).[⎕←"b"]←2
⍝ error: SYNTAX ERROR
⍝ ⎕:

⍝ — Between functions the dot is inner product, even before a group
1 2 3 +.(×⍨) 4 5 6   ⍝ 32

⍝ — After a function, `.[I]` selects from the function's result, with one argument or two. Under writes back through the selection
v←10 20 30 40 ⋄ [⊢.[[1 3]] v;1 2 3 4 +.[[0 2]] v;{⍵×10}@ ⊢.[[1 3]] v]   ⍝ [20 40;11 33;10 200 30 400]

⍝ — After a function, `.name` reads a field of the function's result, and a path goes on from there. A name or path that holds a function gives an inner product
T←["a":1 "x":[10 20 30]] ⋄ ops←["mul":×] ⋄ [⊢.a T;1 +.a T;⊢.x.[[2 0]] T;{⍵×10}@ ⊢.a T;1 2 3 +.ops.mul 4 5 6]   ⍝ [1;2;30 10;["a":10 "x":[10 20 30]];32]

⍝⍝ Format and execute

⍝ — Decimal formatting rounds halfway cases away from zero
2⍕3.125 ¯3.125 2.675 ¯2.675   ⍝ "3.13 ¯3.13 2.68 ¯2.68"

⍝ — Execute evaluates each string of an array of strings, with a left record applying to each
⍎"1+1" "2×3"   ⍝ 2 6

⍝ —
["a":5] ⍎ "a+1" "a×2"   ⍝ 6 10

⍝ — A string that gives no result contributes `⍬`
⍎"" "2"   ⍝ ⍬ 2

⍝ — An empty array of strings gives an empty result whose fill is `⍬`, as `⍎¨` gives
↑⍎ 0⍴⊂"1"   ⍝ ⍬

⍝ — Execute applies to each row of a character matrix
⍎["1+1"⋄"2×3"]   ⍝ 2 6

⍝ — Integer formatting also rounds halves away from zero
0⍕2.5 ¯2.5 1.5 ¯1.5   ⍝ "3 ¯3 2 ¯2"

⍝ — A negative number that rounds to zero keeps its minus sign
2⍕¯0.001 0.001   ⍝ "¯0.00 0.00"

⍝ — A negative precision selects significant digits in exponential notation
¯2⍕3.25 ¯3.25 325 0.0325   ⍝ "3.3ₑ0 ¯3.3ₑ0 3.3ₑ2 3.3ₑ¯2"

⍝ — Matrix formatting aligns exponential fields by column
¯2⍕[3.125 0.002⋄1000 20]   ⍝ ⊃"3.1ₑ0 2.0ₑ¯3" "1.0ₑ3 2.0ₑ1 "

⍝ —
⍕¯1E¯100j¯2E¯99   ⍝ "¯1ₑ¯100ⱼ¯2ₑ¯99"

⍝ — Default formatting switches to exponents beyond these magnitude thresholds
⍕1E¯6 1E¯7 1E16 1E17   ⍝ "0.000001 1ₑ¯7 10000000000000000 1ₑ17"

⍝ — Formatting a number produces a character vector
⍴⍕1   ⍝ [1]ₓ

⍝ — A zero-row matrix retains column widths and separators
⍴⍕0 3⍴0   ⍝ [0 5]ₓ

⍝ — A matrix with no columns formats to zero-width rows
⍴⍕3 0⍴0   ⍝ [3 0]ₓ

⍝ — Numbers that share a marked kind display as their literal laid out in rows, with one mark after the brackets, at any rank. A keyed matrix puts the mark in its corner
⎕←2 2⍴⍳4ₓ
⎕←2 2 2⍴⍳8ₓ
⎕←2 2⍴32•float ⍳4
⎕←"x" "y":⍠1⊢2 2⍴⍳4ₓ
0
⍝ =>
0ₚ
⍝ ⎕: [0 1\n 2 3]ₓ\n[[0 1\n  2 3]\n [4 5\n  6 7]]ₓ\n[0 1\n 2 3]ₛ\nₓ x y\n  0 1\n  2 3

⍝ — ⍕ writes the same form, keeping a higher rank's leading axes with one bracketed block for each plane
[⍕2 2⍴⍳4ₓ;⍕2 2 2⍴⍳8ₓ;⍕32•float 1 2]   ⍝ [["[0 1  "⋄" 2 3]ₓ"] [["[0 1  "⋄" 2 3]ₓ"]⋄["[4 5  "⋄" 6 7]ₓ"]] "[1 2]ₛ"]

⍝ — Fixed-width formatting expands only the trailing dimension
⍴5 2⍕2 3 4⍴⍳24ₓ   ⍝ [2 3 20]ₓ

⍝ — Per-column formats replace overflowing fields with stars
,3 0 6 2⍕[10.1 15⋄1001 22.357⋄101 1110.1]
" 10 15.00*** 22.36101******"

⍝ — Default matrix formatting aligns decimal points within each column
,⍕[1 12.3⋄123 4]   ⍝ "  1 12.3123  4  "

⍝ — Format and execute preserve exact integer and rational domains
⍎⍕1ₓ 1r3   ⍝ 1ₓ 1r3

⍝ —
1⍕1j2
⍝ error: DOMAIN ERROR

⍝ —
¯1 2⍕1
⍝ error: DOMAIN ERROR

⍝ —
⍎1
⍝ error: DOMAIN ERROR

⍝ —
0.5⍕1
⍝ error: DOMAIN ERROR

⍝ —
⍕12.34   ⍝ "12.34"

⍝ —
⍕1ₓ 1r3   ⍝ "1ₓ 1ᵣ3"

⍝ —
⍕⍬   ⍝ ""

⍝ —
4 1⍕1.1 2 ¯4 2.547   ⍝ " 1.1 2.0¯4.0 2.5"

⍝ — Fixed-width fields retain a trailing space when the exponent is short
7 ¯3⍕5 15 155 1555   ⍝ "5.00ₑ0 1.50ₑ1 1.55ₑ2 1.56ₑ3 "

⍝ —
0 2⍕1 2   ⍝ "1.00 2.00"

⍝ —
0 2⍕1r3 2r3   ⍝ "0.33 0.67"

⍝ — Exact big integers format without conversion through float
0 0⍕9223372036854775808ₓ   ⍝ "9223372036854775808"

⍝ — Requested digits beyond float precision are marked with underscores
0 20⍕÷3   ⍝ "0.3333333333333333____"

⍝ — Execute sees the active local binding without changing the global
a←4
f←{a←10 ⋄ ⍎"a+⍵"}
b←f3
[b a]   ⍝ 13 4

⍝ — Dyadic execute evaluates text with the keys of a keyed vector as names, beside the caller's names
T←["a":1 "b":2]
c←100
T⍎"a+b+c"   ⍝ 103

⍝ — Assignments in dyadic execute stay local, and the record is unchanged
T←["a":1]
r←T⍎"a←5 ⋄ a+1"
[r T.a]   ⍝ 6 1

⍝ — The left argument of execute must be a keyed vector
""⍎"1+2"
⍝ error: DOMAIN ERROR

⍝ — Execute can return a primitive function
g←⍎'+' ⋄ 2 g 3   ⍝ 5

⍝ — A final assignment in execute can also return a function
g←⍎"h←+" ⋄ 2 g 3   ⍝ 5

⍝ — A dfn can return the function produced by execute
g←{⍎'+'}0 ⋄ 2 g 3   ⍝ 5

⍝ — Execute can return a primitive operator
r←⍎'/' ⋄ +r 1 2 3   ⍝ 6

⍝ — Execute can return a defined operator
op←⍎"{⍶ ⍵}" ⋄ -op 3   ⍝ ¯3

⍝ — Empty execute produces no value
⍎""   ⍝ {}0

⍝ — A missing execute result cannot be assigned
a←⍎""
⍝ error: VALUE ERROR

⍝ — ⍕ marks each exact number when a big integer keeps the vector mixed
⍕1ₓ 0ₓ ¯2ₓ 9223372036854775808ₓ   ⍝ "1ₓ 0ₓ ¯2ₓ 9223372036854775808ₓ"

⍝ — Rationals display in lowest terms, and floats with every digit needed to read back
⍕¨[1ₓ÷3ₓ 6ₓ÷3ₓ 1ₓ÷3]   ⍝ "1ᵣ3" "2ₓ" "0.3333333333333333"

⍝⍝ Polynomial representations and derivatives

⍝ — Polynomial coefficients are constant-first
[1 2 3]ₓ⌻[0 1 2]ₓ   ⍝ [1 6 17]ₓ

⍝ — Evaluating at an array of points preserves its shape
[1 2 3]ₓ⌻[0 1⋄2 3]ₓ   ⍝ [1 6⋄17 34]ₓ

⍝ — Evaluate the factored form 2(x-1)(x-3)
[2ₓ [1 3]ₓ]⌻[0 1 2 3]ₓ   ⍝ [6 0 ¯2 0]ₓ

⍝ — Convert multiplier and roots to constant-first coefficients
⌻⁻¹[2ₓ [1 3]ₓ]   ⍝ [6 ¯8 2]ₓ

⍝ — Enclosed roots imply a leading coefficient of one
⌻⁻¹⊂[1 3]ₓ   ⍝ [3 ¯4 1]ₓ

⍝ — Convert an exponent table for x⁵-1, filling missing degrees with zero
⌻⁻¹⊂[1 5⋄¯1 0]ₓ   ⍝ [¯1 0 0 0 0 1]ₓ

⍝ — Fractional exponents evaluate numerically even with exact input
(⊂[2ₓ 1r2⋄3ₓ 1r4])⌻16ₓ   ⍝ 14

⍝ — Evaluate a two-variable exponent table at an enclosed coordinate vector
(⊂[¯1 2 1⋄1 1 1⋄2 1 2⋄3 0 2])⌻⊂2.5 ¯1   ⍝ 11.75

⍝ — Empty exact evaluation points retain an exact prototype
[1 2]ₓ⌻0⍴0ₓ   ⍝ ⍬ₓ

⍝ — Each polynomial row pairs with one evaluation point by frame
[1 2 3⋄4 5 6]ₓ⌻[1 2]ₓ   ⍝ [6 38]ₓ

⍝ —
5ₓ⌻2ₓ   ⍝ 5ₓ

⍝ —
[0 0]ₓ⌻2ₓ   ⍝ 0ₓ

⍝ — A multiplier with no roots is a constant polynomial
⌻⁻¹[2ₓ 0⍴0ₓ]   ⍝ [2]ₓ

⍝ — Differentiate the bound polynomial evaluator
f←[1 2 3]ₓ↣⌻ ⋄ f∂2ₓ   ⍝ 14ₓ

⍝ — Repeated differentiation gives the second derivative
f←[1 2 3]ₓ↣⌻ ⋄ f∂∂2ₓ   ⍝ 6ₓ

⍝ — Differentiating beyond the polynomial's degree gives exact zero
f←[1 2 3]ₓ↣⌻ ⋄ f∂∂∂2ₓ   ⍝ 0ₓ

⍝ — A vector of points gives the Jacobian, which is diagonal
f←[1 2 3]ₓ↣⌻ ⋄ f∂ [1 2]ₓ   ⍝ [8 0⋄0 14]ₓ

⍝ — Differentiate directly from the factored representation
f←[2ₓ [1 3]ₓ]↣⌻ ⋄ f∂2ₓ   ⍝ 0ₓ

⍝ — A multivariate gradient retains the coordinate enclosure
f←⊂[1 2 0⋄1 0 2]ₓ ↣⌻ ⋄ f∂⊂[3 4]ₓ   ⍝ ⊂[6 8]ₓ

⍝ — The left argument passes to the function, and ∂ differentiates with respect to the right
3 {⍺×⍵×⍵}∂ 2   ⍝ 12

⍝ — A shared unit coordinate sums the partial derivatives
f←⊂[1 2 0⋄1 0 2]ₓ ↣⌻ ⋄ f∂3ₓ   ⍝ 12ₓ

⍝ — Two polynomials give one derivative each
f←[1 2⋄3 4]ₓ↣⌻ ⋄ f∂ 3ₓ   ⍝ [2 4]ₓ

⍝ — Assigning f∂ attaches a hand-written JVP rule, which the derivative calls with `⍺` as the direction
r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ r∂3   ⍝ 6

⍝ — Inside another function, an attached rule gives the call's derivative
r←{⍵×⍵} ⋄ r∂←{⍺×7} ⋄ {1+r ⍵}∂3   ⍝ 7

⍝ — A function with an attached derivative applies the function itself
r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ r 3   ⍝ 9

⍝ — A rule that passes its direction through gives the argument's kind, not the body's derivative of 0
ste←{⌊⍵} ⋄ ste∂←{⍺} ⋄ {+/ste ⍵}∂ 1.5 2.5 3.5   ⍝ 1 1 1

⍝ — Each keeps an attached rule, whether `∂` is outside the map or inside it
ste←{⌊⍵} ⋄ ste∂←{⍺} ⋄ v←1.5 2.5 3.5 4.5 ⋄ [{+/ste¨⍵}∂ v; {ste∂⍵}¨ v]   ⍝ [1 1 1 1; 1 1 1 1]

⍝ — Rank keeps an attached rule, whether `∂` is outside the map or inside it
ste←{⌊⍵} ⋄ ste∂←{⍺} ⋄ m←[1.5 2.5⋄3.5 4.5] ⋄ [{+/,ste⍤0⊢⍵}∂ m; {ste∂⍵}⍤0 m]   ⍝ [[1 1⋄1 1] [1 1⋄1 1]]

⍝ — A stencil keeps an attached rule, whether `∂` is outside the map or inside it. Outside, each item's gradient counts the windows that hold it
ste←{⌊⍵} ⋄ ste∂←{⍺} ⋄ v←1.5 2.5 3.5 4.5 ⋄ [{+/{+/ste ⍵}⌺3⊢⍵}∂ v; {{+/ste ⍵}∂⍵}⌺3 v]   ⍝ [2 3 3 2; [1 1 1⋄1 1 1⋄1 1 1⋄1 1 1]]

⍝ — The attached derivative replaces the polynomial's own
f←[1 2 3]ₓ↣⌻ ⋄ f∂←{⍺×[0 0 5]ₓ⌻⍵} ⋄ f∂3ₓ   ⍝ 45ₓ

⍝ — A higher order differentiates the attached derivative
f←[1 2 3]ₓ↣⌻ ⋄ f∂←{⍺×[0 0 5]ₓ⌻⍵} ⋄ f∂∂3ₓ   ⍝ 30ₓ

⍝ — The inverse inverts the function, not its derivative
k←3× ⋄ k∂←{3×⍺} ⋄ k⁻¹ 6   ⍝ 2

⍝ — A rule describes a function of one argument, so ∂ through a call with two gives an error
r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ 10 r∂ 3
⍝ error: DOMAIN ERROR

⍝ — The same holds for a call with two arguments inside other code that ∂ differentiates
r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ {10 r ⍵}∂ 3
⍝ error: DOMAIN ERROR

⍝ — A call with two arguments runs as usual where ∂ needs no derivative through it
r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ [10 r 3; {(10 r 3)+⍵}∂ 2]   ⍝ [9; 1]

⍝ — Binding the left argument first gives a function of one argument, which can take a rule
r←{⍵×⍵} ⋄ bound←10↣r ⋄ bound∂←{⍺×2×⍵} ⋄ bound∂3   ⍝ 6

⍝ — A derivative must be a function
f←{⍵×⍵} ⋄ f∂←3
⍝ error: DOMAIN ERROR

⍝ — A dfn differentiates through the primitives it calls
{⍵×⍵}∂3   ⍝ 6

⍝ — Third derivatives nest three levels of ∂
{⍵×⍵×⍵}∂∂∂2   ⍝ 6

⍝ — Nested ∂ gives a mixed second derivative once
{⍵₀×⍵₁}∂∂ 2 3   ⍝ [0 1⋄1 0]

⍝ — Nested ∂ takes rationals, and numbers that mix exact and approximate kinds
[{+/⍵×⍵}∂∂ [1r2 1r3]; {+/⍵×⍵}∂∂ [1ₓ 2]]   ⍝ [[2 0⋄0 2]ₓ; [2 0⋄0 2]]

⍝ — A Hessian–vector product differentiates a gradient along a direction
x←1 2 ⋄ v←1 0 ⋄ rosen←{[x y]←⍵ ⋄ ⊼1-x + 100×⊼y-⊼x} ⋄ {rosen∂ x+⍵×v}∂0   ⍝ 402 ¯400

⍝ — With more than one number, ∂ runs the function once and transposes its record: a gradient
{+/⍵×⍵}∂ 1 2 3   ⍝ 2 4 6

⍝ — Reverse over reverse gives a Hessian
rosen←{[x y]←⍵ ⋄ ⊼1-x + 100×⊼y-⊼x} ⋄ {rosen∂ ⍵}∂ 1 2   ⍝ [402 ¯400⋄¯400 200]

⍝ — Reverse over reverse moves items and folds them: the Hessian of +/⍵×⌽⍵ is 2 on the anti-diagonal
{{+/⍵×⌽⍵}∂ ⍵}∂ 1 2 3   ⍝ [0 0 2⋄0 2 0⋄2 0 0]

⍝ — An outer dual can be the coefficient of an inner record: the gradient of +/⍵×x is x, whose derivative is the identity
{x←⍵ ⋄ {+/⍵×x}∂ ⍵}∂ 1 2   ⍝ [1 0⋄0 1]

⍝ — A Jacobian with no more results than numbers takes one transpose for each result
cart←{[r t]←⍵ ⋄ [r×2○t; r×1○t]} ⋄ (cart∂ 2 0.5) - [2○0.5 ¯2×1○0.5⋄1○0.5 2×2○0.5]   ⍝ [0 0⋄0 0]

⍝ — A Jacobian with more results than numbers evaluates the record once for each number
{(⍵×⍵),+/⍵}∂ 1 2   ⍝ [2 0⋄0 4⋄1 1]

⍝ — A gradient over a nested argument has its structure
p←[1 2;3] ⋄ {+/⍵₀×⍵₁}∂ p   ⍝ [[3 3] 3]

⍝ — A gradient keeps its argument's keys
{+/⍵×⍵}∂ "a" "b":3 4   ⍝ ["a":6 "b":8]

⍝ — Exact arguments give exact gradients
{+/⍵×⍵}∂ [1 2 3]ₓ   ⍝ [2 4 6]ₓ

⍝ — Copies transpose to sums: fill, rotation, replication and windows
[{+/3↑⍵}∂ 1 2;{+/2⌽⍵×⍳3ₓ}∂ 1 2 3;{+/2 0 3#⍵}∂ 1 2 3;{+/,2↕⍵}∂ 1 2 3]   ⍝ [1 1;0 1 2;2 0 3;1 2 1]

⍝ — Each kind of copy transposes to its adjoint: the dot product of a copy with y equals the dot product of its argument with its gradient against y
dot←{f←⍶ ⋄ r←f ⍵ ⋄ y←(⍴r)⍴1+⍳≢,r ⋄ (+/,y×r)=+/,⍵×({+/,y×(f ⍵)}∂ ⍵)} ⋄ x←4 6⍴⍳24ₓ ⋄ ∧/[⌽dot x;(1⌽)dot x;⊖dot x;(2 3↑)dot x;(¯1 ¯2↓)dot x;(5 8↑)dot x;⍉dot x;(0 0⍉)dot x;(2 3↕)dot x;(¯3 ¯3↕)dot x;([[2 2]⋄[2 2]]↕)dot x;(2 0 1 3#)dot x;{⍵,⍵}dot x;{+/2↕⍵}dot x;{⍵+⊗0 0}dot x;{0 0+⊗⍵}dot x;{⌽⍉⍵}dot x;{2 2↕1↓⍵}dot x;{1⌽2↓⍵}dot x]   ⍝ $t

⍝ — Under and selective assignment transpose through what they write
[{+/(10×)@0 2⊢⍵}∂ 1 2 3;{x←⍵ ⋄ x.[1]←0 ⋄ +/x×x}∂ 1 2 3]   ⍝ [10 1 10;2 0 6]

⍝ — Folds, scans and products have rules that transpose
[{+/+\⍵}∂ 1 2 3;{×/⍵}∂ 2 3 4;{⌈/⍵}∂ 3 1 4;{+/(2 2⍴⍵)+.×1 2}∂ 1 2 3 4]   ⍝ [3 2 1;12 8 6;0 0 1;1 2 1 2]

⍝ — The function runs once, so a random draw is the same for every result
j←{⍵×¿0}∂ 1 2 ⋄ (0 0⌷j)=1 1⌷j   ⍝ $t

⍝ — Catenating a number dual keeps its tangent
{x←1 2×⍵ ⋄ +/x,+/x}∂ 1   ⍝ 6

⍝ — ∂ strips its tangents from a value that its function wrote to an outer name
s←0 ⋄ f←{s+←⍵×2 ⋄ ⍵×⍵} ⋄ _←f∂3 ⋄ s×10   ⍝ 60

⍝ — ∂ also strips its tangents from an outer array that its function changed by selective assignment or by appending in place
c←0 0
a←⍬
f←{([[0]]⌷c)←⍵ ⋄ a,←⍵ ⋄ ⍵×2}
_←f∂3
(•storage c).active (•storage a).active   ⍝ [0 0]ₓ

⍝ — A power count reads its primal, so the count takes no derivative
{(⍵×)⍣⍵⊢1}∂ 3   ⍝ 27

⍝ — A unit selector reads its primal to pick a case
{⍵⍚[{⍵×⍵} {⍵×⍵×⍵}] ⍵}∂ 1   ⍝ 3

⍝ — A predicate reads its primal to pick a body, so it takes no derivative
{(0×⍵)?1;⍵×3}∂ 2   ⍝ 3

⍝ — A cell rank reads its primal, so a dual ∞ is ∞
{(+/⍤⍵)[1 2 3⋄4 5 6]}∂ ∞   ⍝ 0 0

⍝ — An index selector reads its primal, so a dual ∞ selects every remaining axis
{+/⍵⌷10 20 30}∂ ∞   ⍝ 0

⍝ — A position reads its primal, so a selection by ⌷, dot selection or pick takes no derivative from it
{⍵×(⍵⌷10 20 30)+((10 20 30).[⍵])+[⍵]⊃10 20 30}∂ 1   ⍝ 60

⍝ — A count reads its primal, so take, drop, rotate, reshape, replicate and windows take no derivative from it
v←1 2 3 4 ⋄ {⍵×+/(⍵↑v),(⍵↓v),(⍵⌽v),(⍵⍴v),(⍵#v),,⍵↕v}∂ 2   ⍝ 58

⍝ — Display shows a dual as its primal, with the primal's width marker
{⎕←⍵ ⋄ ⍵×⍵}∂ 1.5ₛ
3ₛ
⍝ ⎕: 1.5ₛ

⍝ — Display rounds a dual as it rounds its primal
{⎕←⍵+0.2 ⋄ ⍵}∂ 0.1
1
⍝ ⎕: 0.3

⍝ — •storage reads a dual's view flag from its primal, as it reads the dual's kind and pending flag
m←[1 2⋄3 4] ⋄ {⎕←(•storage ⍵).view ⋄ +/⍵}∂ ,m
1 1 1 1
⍝ ⎕: $t

⍝ — An exact argument gives an exact derivative
{⍵×⍵}∂3ₓ   ⍝ 6ₓ

⍝ — A rational argument gives a rational derivative
{⍵×⍵}∂2ᵣ3   ⍝ 4ᵣ3

⍝ — A 32-bit argument gives a 32-bit derivative
{⍵×⍵}∂3ₛ   ⍝ 6ₛ

⍝ — Under with a predicate differentiates through the selection
{+/(2×)@[0<⊢] ⍵}∂ ¯1 2 3   ⍝ 1 2 2

⍝ — Under with positions differentiates through the selection
{+/(2×)@[1 2] ⍵}∂ 5 5 5   ⍝ 1 2 2

⍝ — Selective assignment keeps the tangent of the value it writes
{x←⍵ ⋄ x.[1]←10×x₁ ⋄ +/x}∂ 1 2 3   ⍝ 1 10 1

⍝ — A keyed argument gives a keyed gradient
{+/⍵×⍵}∂ "a" "b":1 2   ⍝ ["a":2 "b":4]

⍝ — A record argument gives a record gradient
p←["a":1 "b":2] ⋄ {(⍵.a)×⍵.b}∂ p   ⍝ ["a":2 "b":1]

⍝ — Arithmetic pairs named axes, and each gradient sums back over the axes its argument lacks
a←["i":2]⍴1 2.0 ⋄ b←["j":3]⍴0.5 1 1.5 ⋄ [{+/,a×⍵}∂b; {+/,⍵×b}∂a; {+/,a+⍵}∂b]   ⍝ [["j":3]⍴3 3 3; ["i":2]⍴3 3; ["j":3]⍴2 2 2]

⍝ — A shared named axis pairs, and the gradient sums over the other argument's own axis
q←["i":2 "d":3]⍴⍳6ₓ ⋄ k←["j":4 "d":3]⍴⍳12ₓ ⋄ {+/, ⍵×k}∂q   ⍝ ["i":2 "d":3]⍴[18 22 26]ₓ

⍝ — Folds along named axes, one or several, and along a middle axis, have rules
x←["i":2 "j":3]⍴[1 5 3 4 2 6]ₓ ⋄ [{+/⍠"i" "j" ⍵×⍵}∂x; {+/⌈/⍠"i" ⍵}∂x; {+/, ⌈/⍠1 ⍵}∂ 2 3 2⍴[7 1 3 9 2 2 8 0 5 4 6 6]ₓ]   ⍝ [["i":2 "j":3]⍴[2 10 6 8 4 12]ₓ; ["i":2 "j":3]⍴[0 1 0 1 0 1]ₓ; 2 3 2⍴[1 0 0 1 0 0 1 0 0 0 0 1]ₓ]

⍝ — Rank and Each with dfn operands transpose through each call
[{+/,{(+/⍵)×⍵}⍤1⊢⍵}∂ [1 2⋄3 4]; {+/{⍵×⍵}¨⍵}∂ 1 2 3]   ⍝ [[6 6⋄14 14]; 2 4 6]

⍝ — A transpose's transpose reads the cotangent transposed, and a gradient over a nested argument keeps its nesting
[{+/,(⍉⍵)×⍵}∂ [1 2⋄3 4]; {+/(⍵₀)×⍵₁}∂ [2;3 4]]   ⍝ [[2 6⋄4 8]; [7; 2 2]]

⍝ — A gradient through a tangent that holds an active item beside a constant one keeps the constant's tangent as zeros
[{+/↑{⍵}¨[⍵ 1]}∂ 1 2 3; {+/∊{⍵}¨[⍵ 1]}∂ 1 2 3]   ⍝ [1 1 1; 1 1 1]

⍝ — A result built only from a partition's empty parts has an empty Jacobian
⍴{+/,2⊂⍵}∂ 1.5 2.5 3.5   ⍝ [0 3]ₓ

⍝ — A dual atom that a ravel or an enclosure leaves inside a plain array stays active beside a dual, in forward and in reverse mode
[{(,⍵)+(,⍵)+,⍵}∂ 2; {+/⊃(⊂,⍵)+(⊂,⍵)+⊂,⍵}∂ 1.5; {+/,,¨⍵}∂ 1 2 3; {+/,⊂¨⍵}∂ 1 2 3]   ⍝ [[3]; 3; 1 3⍴1; 1 1 1]

⍝ — An inner ∂ whose function closes over the outer argument differentiates through ⊥
x←1.5 2.5 3.5 ⋄ {y←⍵ ⋄ {+/,y⊥⍵}∂ 2.5}∂ x   ⍝ 0 3.5 3.5

⍝ — A gradient through an empty selection of the argument is zeros
{+/,∩⍵}∂ 1.5 2.5 3.5   ⍝ 0 0 0

⍝ — Rank over cells whose results are empty gives a gradient of zeros
{m←3 3⍴⍵ ⋄ +/,{0↑⍵}⍤1⊢m}∂ 1.5 2.5 3.5   ⍝ 0 0 0

⍝ — An inner ∂ whose derivative doesn't depend on the outer argument it closes over gives a gradient of zeros
{y←⍵ ⋄ +/,{+/,y,⍵}∂2.5}∂ 1.5 2.5 3.5   ⍝ 0 0 0

⍝ — A 32-bit argument gives a 32-bit gradient through a reversal
{+/⌽⍵×⍵}∂ 32•float 1 2 3   ⍝ [2 4 6]ₛ

⍝ — A gradient with respect to coefficients pairs each polynomial with its point, as ⌻ does
[{+/⍵⌻2}∂ [1 2⋄3 4]; {+/,⍵⌻1 2}∂ [1 2⋄3 4]; {+/,⍵⌻[1 2⋄3 4]}∂ 1 2]   ⍝ [[1 2⋄1 2]; [1 1⋄1 2]; 4 10]

⍝ — Scans by × have zero-safe rules, and scans by ⌈ and ⌊ follow the item that gives each running extreme, the earlier of any ties
[{+/×\⍵}∂ 1 2 3; {+/×\⍵}∂ 1 0 3; {+/⌈\⍵}∂ 1 3 2 3; {+/⌊\⍵}∂ 3 1 2 0; {×\1 2 3×⍵}∂ 1]   ⍝ [9 4 2; 1 4 0; 1 3 0 0; 1 2 0 1; 1 4 18]

⍝ — Scans along a matrix's axes follow the same rules in each lane: a tie keeps the earlier item, and a running product is zero-safe
[{+/,⌈⍀⍵}∂ [1 3⋄2 1⋄2 4]; {+/,×⍀⍵}∂ [1 2⋄3 4]; {+/,×⍀⍵}∂ [0 2⋄3 0]; {+/,⌊\⍵}∂ [3 1 2⋄0 5 0]]   ⍝ [[1 2⋄2 0⋄0 1]; [4 5⋄1 2]; [4 1⋄0 2]; [1 2 0⋄3 0 0]]

⍝ — A large argument takes one pass, with whole-array tangents
x←0.001×⍳200000ₓ ⋄ ({1○x+⍵}∂0)≡2○x   ⍝ $t

⍝ — A sum's tangent is the sum of the tangents
{+/⍵×⍵}∂ 1 2 3   ⍝ 2 4 6

⍝ — A running sum's Jacobian is lower triangular
{+\⍵}∂ 1 2 3   ⍝ [1 0 0⋄1 1 0⋄1 1 1]

⍝ — An inner product's tangent comes from the product rule
m←[1 2⋄3 4] ⋄ {+/,m+.×⍵}∂ 1 1   ⍝ 4 6

⍝ — An outer product's arguments broadcast before the pervasive rule applies
{+/,⍵×⊗⍵}∂ 1 2   ⍝ 6 6

⍝ — The polar-to-Cartesian Jacobian has determinant r
cart←{[r t]←⍵ ⋄ [r×2○t; r×1○t]} ⋄ -.×cart∂ 2 0.5   ⍝ 2

⍝ — A product's tangent at a zero item is the product of the others
{×/⍵}∂ 2 0 4   ⍝ 0 8 0

⍝ — A product along the first axis differentiates each column
{×⌿⍵}∂ [2 3⋄4 5]   ⍝ [2 2 2]⍴4 0 2 0 0 5 0 3

⍝ — A maximum's tangent is the tangent of its first largest item
{⌈/⍵}∂ 1 5 5 2   ⍝ 0 1 0 0

⍝ — A minimum along the first axis takes each column's smallest item
{⌊⌿⍵}∂ [1 5⋄4 2]   ⍝ [2 2 2]⍴1 0 0 0 0 0 0 1

⍝ — An inverse's tangent is -R+.×tA+.×R, exact for an exact argument, and nested ∂ differentiates it again
[{⌹⍵}∂ 2; {⌹⍵}∂ 2ₓ; {⌹⍵}∂∂ 2]   ⍝ [¯0.25; ¯1ᵣ4; 0.25]

⍝ — Monadic -.× and ⌹ differentiate at an invertible matrix
[{-.×⍵}∂ [1 2⋄3 4]; {+/,⌹⍵}∂ [2 1⋄1 3]]   ⍝ [[4 ¯3⋄¯2 1]; -[4 2⋄2 1]÷25]

⍝ — B⌹A is linear in B for any A, including the least-squares solution of a tall A
[{⍵⌹[2 1⋄1 3]}∂ 1 2; {⍵⌹[1 0⋄0 1⋄1 1]}∂ 3 3 3]   ⍝ [[3 ¯1⋄¯1 2]÷5; [2 ¯1 1⋄¯1 2 1]÷3]

⍝ — ⌹ under Rank differentiates each matrix of a stack
{+/,⌹⍤2⊢⍵}∂ [2 2 2]⍴2 1 1 3 4 1 1 2   ⍝ [2 2 2]⍴(-4 2 2 1÷25),-1 3 3 9÷49

⍝ — A matrix whose pivot falls under rustymath's cutoff has no derivative of its inverse
{⌹⍵}∂ [1 2⋄3 6+1e¯15]
⍝ error: UNSUPPORTED

⍝ — Monadic f.g differentiates only as -.×
{+.×⍵}∂ [1 2⋄3 4]
⍝ error: UNSUPPORTED

⍝ — ⊥ differentiates with respect to its digits and its base, a scalar or a mixed radix, and a single digit extends to the base's length
[{10⊥⍵}∂ 1 2 3; {24 60 60⊥⍵}∂ 1 2 3; {⍵⊥1 2 3}∂ 10; {⍵⊥1 2 3}∂ 24 60 60; {⊥⍵}∂ 1 0 1; {10 10 10⊥⍵}∂ 2]   ⍝ [100 10 1; 3600 60 1; 22; 0 60 62; 4 2 1; 111]

⍝ — ⊥ under Rank differentiates each cell, monadic, with a base, or with its base bound
[{+/,⊥⍤1⊢[3 3]⍴⍵}∂ ⍳9; {+/,2⊥⍤1⊢[3 3]⍴⍵}∂ ⍳9; {+/,(2↣⊥)⍤1⊢[3 3]⍴⍵}∂ ⍳9]   ⍝ 3#⊂9⍴4 2 1

⍝ — Mix of dual cells copies each part whole
{+/,⊃[⍵ 2×⍵]}∂ 1 2 3   ⍝ 3 3 3

⍝ — Discrete results have derivative 0: the digits of ⊤ and the draws of ¿
[{+/2 2⊤⍵}∂ 3; {¿⍵}∂ 5]   ⍝ 0 0

⍝ — The bits that ᵇ gives have derivative 0
{⍵∧ᵇ3}∂ 2   ⍝ 0ₓ

⍝ — The items of x↦y move with the start x, so each has derivative 1 with respect to it. The end gives only the count, as monadic ↦ does
[{+/⍵↦5}∂ 2; {+/2↦⍵}∂ 5; {+/↦⍵}∂ 5; {+/⍵₀↦5}∂ 2 3]   ⍝ [4;0;0;4 0]

⍝ — On real numbers, monadic ∨ gives each number and 0, and monadic ∧ its magnitude and angle, whose derivatives are 1 or the sign, beside 0
[{+/,∨⍵}∂ 1 2; {+/,∧⍵}∂ 1 ¯2; {+/∧⍵}∂ ¯3]   ⍝ [1 1;1 ¯1;¯1]

⍝ — A circle code reads its plain value, as a count does, so its derivative is 0
{+/⍵○1}∂ 1 2   ⍝ 0 0

⍝ — !y is Γ(y + 1), whose derivative is Γ(y + 1)ψ(y + 1), and x!y is C(y, x), whose partials are C(y, x) times differences of ψ. At a whole n, ψ(n + 1) is 1 + 1/2 + … + 1/n - γ
[{!⍵}∂ 3; {2!⍵}∂ 5; {⍵!5}∂ 2; {⍵!2×⍵}∂ 2]   ⍝ [6×(11÷6)-0.5772156649015329; 4.5; 10÷3; 7]

⍝ — A whole x makes x!y the polynomial y(y-1)…(y-x+1)÷!x, whose slope stays finite at its roots and at a negative whole y, where the ψ of the general form meet poles
[{2!⍵}∂ 1; {3!⍵}∂ 1; {2!⍵}∂ ¯1; {⍵!1}∂ 2]   ⍝ [0.5; ¯1÷6; ¯1.5; ¯0.5]

⍝ — The binomial's partials keep the float width that x!y has, here 32 bits
x←32 •float 1 2 5 ⋄ k←32 •float 2 ⋄ [(•storage {+/k!⍵}∂ x).kind; {+/k!⍵}∂ x]   ⍝ ["float32"; 32 •float 0.5 1.5 4.5]

⍝ — In reverse mode, ! gives each item's derivative
{+/!⍵}∂ 1 2 3   ⍝ [1 2 6]×[1 1.5 (11÷6)]-0.5772156649015329

⍝ — A second derivative of ! would need ψ's derivative, which ∂ lacks
{{!⍵}∂⍵}∂ 3
⍝ error: UNSUPPORTED

⍝ — In reverse mode, a result drawn only from constants has a gradient of zeros
c←1 2 3 ⋄ {+/,c∩⍵}∂ 1.5 2 3.5   ⍝ 0 0 0

⍝ — Complex arguments are outside the subset
{|⍵}∂3j4
⍝ error: UNSUPPORTED

⍝ — •json⁻¹ writes a dual as its plain value, as ⍕ does, so the length of its text has derivative 0
{≢•json⁻¹ ⍵}∂ 3   ⍝ 0ₓ

⍝ — A system function is outside the subset
{•ucs ⍵}∂3
⍝ error: UNSUPPORTED

⍝ — ∂ needs a numeric result
{"ab"}∂2
⍝ error: DOMAIN ERROR

⍝ — Negative exponents cannot be converted to a coefficient vector
⌻⁻¹⊂[1 ¯1⋄]
⍝ error: DOMAIN ERROR

⍝ — A nested result needs a number argument
{⊂⍵}∂ 1 2
⍝ error: DOMAIN ERROR

⍝ — An infinite coefficient follows IEEE
⌻1 ∞   ⍝ [∞ [¯0]]

⍝ — A vector result gives the Jacobian
1 2↣⌻∂1 2   ⍝ [2 0⋄0 2]

⍝ — A bound function takes one argument under ∂ too
1(1 2↣⌻∂)1 2
⍝ error: SYNTAX ERROR

⍝ — The coordinate count must match the exponent table's variables
(⊂[1 2 3⋄])⌻⊂1 2 3
⍝ error: LENGTH ERROR

⍝ — Converting coefficients to roots and back gives the coefficients again [atol=1e-10]
[⌻⁻¹⌻0 16 ¯12 2;⌻⁻¹⌻1 0 1;⌻⁻¹⌻1 ¯2 1]
[0 16 ¯12 2;1 0 1;1 ¯2 1]

⍝⍝ ∂ through forms that move or order items

⍝ derivative-moves-same — same (`⊢`) moves a tangent with its primal, in forward mode and in reverse mode
[{⊢⍵×1 2 3}∂ 1; {+/1 2 3×⊢⍵}∂ 2.5 1.5 3.5]   ⍝ [1 2 3;1 2 3]

⍝ derivative-moves-same-left — same left (`⊣`) moves a tangent with its primal, in forward mode and in reverse mode
[{⊣⍵×1 2 3}∂ 1; {+/1 2 3×⊣⍵}∂ 2.5 1.5 3.5]   ⍝ [1 2 3;1 2 3]

⍝ derivative-moves-ravel — ravel (`,`) moves a tangent with its primal, in forward mode and in reverse mode
[{,⍵×[1 2⋄3 4]}∂ 1; {+/1 2 3 4×,⍵}∂ [1.5 2.5⋄3.5 4.5]]   ⍝ [[1 2 3 4] [1 2⋄3 4]]

⍝ derivative-moves-table — table (`⍪`) moves a tangent with its primal, in forward mode and in reverse mode
[{⍪⍵×1 2 3}∂ 1; {+/,[1⋄2⋄3]×⍪⍵}∂ 2.5 1.5 3.5]   ⍝ [[[1]⋄[2]⋄[3]] [1 2 3]]

⍝ derivative-moves-enclose — enclose (`⊂`) moves a tangent with its primal, in forward mode and in reverse mode
[{∊⊂⍵×1 2}∂ 1; {+/1 2×∊⊂⍵}∂ 2.5 1.5]   ⍝ [1 2;1 2]

⍝ derivative-moves-mix — mix (`⊃`) moves a tangent with its primal, in forward mode and in reverse mode
[{⊃[⍵×1 2;⍵×3 4]}∂ 1; {+/,[1 2⋄3 4]×⊃⍵}∂ [1.5 2.5;3.5 4.5]]   ⍝ [[1 2⋄3 4] [1 2;3 4]]

⍝ derivative-moves-nest — nest (`⊆`) moves a tangent with its primal, in forward mode and in reverse mode
[{∊⊆⍵×1 2}∂ 1; {+/1 2×∊⊆⍵}∂ 2.5 1.5]   ⍝ [1 2;1 2]

⍝ derivative-moves-enlist — enlist (`∊`) moves a tangent with its primal, in forward mode and in reverse mode
[{∊[⍵×1 2;⍵×3]}∂ 1; {+/1 2 3×∊⍵}∂ [1.5 2.5;3.5]]   ⍝ [1 2 3;[[1 2] 3]]

⍝ derivative-moves-first — first (`↑`) moves a tangent with its primal, in forward mode and in reverse mode
[{↑⍵×[1 2⋄3 4]}∂ 1; {+/,1 2×↑⍵}∂ [1.5 2.5⋄3.5 4.5]]   ⍝ [[1 2] [1 2⋄0 0]]

⍝ derivative-moves-split — split (`↓`) moves a tangent with its primal, in forward mode and in reverse mode
[{∊↓⍵×[1 2⋄3 4]}∂ 1; {+/1 2 3 4×∊↓⍵}∂ [1.5 2.5⋄3.5 4.5]]   ⍝ [[1 2 3 4] [1 2⋄3 4]]

⍝ derivative-moves-reverse — reverse (`⌽`) moves a tangent with its primal, in forward mode and in reverse mode
[{⌽⍵×1 2 3}∂ 1; {+/1 2 3×⌽⍵}∂ 2.5 1.5 3.5]   ⍝ [3 2 1;3 2 1]

⍝ derivative-moves-reverse-first — reverse first (`⊖`) moves a tangent with its primal, in forward mode and in reverse mode
[{⊖⍵×[1 2⋄3 4]}∂ 1; {+/,[1 2⋄3 4]×⊖⍵}∂ [1.5 2.5⋄3.5 4.5]]   ⍝ [[3 4⋄1 2] [3 4⋄1 2]]

⍝ derivative-moves-transpose — transpose (`⍉`) moves a tangent with its primal, in forward mode and in reverse mode
[{⍉⍵×[1 2⋄3 4]}∂ 1; {+/,[1 2⋄3 4]×⍉⍵}∂ [1.5 2.5⋄3.5 4.5]]   ⍝ [[1 3⋄2 4] [1 3⋄2 4]]

⍝ derivative-moves-materialise — materialise (`⌷`) moves a tangent with its primal, in forward mode and in reverse mode
[{⌷⍵×1 2 3}∂ 1; {+/1 2 3×⌷⍵}∂ 2.5 1.5 3.5]   ⍝ [1 2 3;1 2 3]

⍝ derivative-moves-pairs — pairs (`↕`) moves a tangent with its primal, in forward mode and in reverse mode
[{↕⍵×1 2 3}∂ 1; {+/,[1 2⋄3 4]×↕⍵}∂ 2.5 1.5 3.5]   ⍝ [[1 2⋄2 3] [1 5 4]]

⍝ derivative-moves-unkey — unkey (`:`) moves a tangent with its primal, in forward mode and in reverse mode
[{:⍵×"a" "b":1 2}∂ 1; {+/1 2×:⍵}∂ "a" "b":2.5 1.5]   ⍝ [1 2;["a":1 "b":2]]

⍝ derivative-moves-right — right (`⊢`) moves a tangent with its primal, in forward mode and in reverse mode
[{5⊢⍵×1 2}∂ 1; {+/1 2×5⊢⍵}∂ 2.5 1.5]   ⍝ [1 2;1 2]

⍝ derivative-moves-left — left (`⊣`) moves a tangent with its primal, in forward mode and in reverse mode
[{(⍵×1 2)⊣5}∂ 1; {+/1 2×⍵⊣5}∂ 2.5 1.5]   ⍝ [1 2;1 2]

⍝ derivative-moves-catenate — catenate (`,`) moves a tangent with its primal, in forward mode and in reverse mode
[{(⍵×1 2),⍵×3}∂ 1; {+/1 2 3 4×⍵,⍵×2}∂ 2.5 1.5]   ⍝ [1 2 3;7 10]

⍝ derivative-moves-catenate-first — catenate first (`⍪`) moves a tangent with its primal, in forward mode and in reverse mode
[{(⍵×[1 2⋄3 4])⍪⍵×[5 6⋄]}∂ 1; {+/,[1 2⋄3 4]×⍵⍪⍵×2}∂ [1.5 2.5⋄]]   ⍝ [[1 2⋄3 4⋄5 6] [7 10⋄]]

⍝ derivative-moves-reshape — reshape (`⍴`) moves a tangent with its primal, in forward mode and in reverse mode
[{2 2⍴⍵×1 2 3}∂ 1; {+/1 2 3 4×4⍴⍵}∂ 2.5 1.5 3.5]   ⍝ [[1 2⋄3 1] [5 2 3]]

⍝ derivative-moves-partitioned-enclose — partitioned enclose (`⊂`) moves a tangent with its primal, in forward mode and in reverse mode
[{∊1 0 1⊂⍵×1 2 3}∂ 1; {+/1 2 3×∊1 0 1⊂⍵}∂ 2.5 1.5 3.5]   ⍝ [1 2 3;1 2 3]

⍝ derivative-moves-pick — pick (`⊃`) moves a tangent with its primal, in forward mode and in reverse mode
[{1⊃⍵×1 2 3}∂ 1; {1⊃⍵}∂ 2.5 1.5 3.5]   ⍝ [2;0 1 0]

⍝ derivative-moves-partition — partition (`⊆`) moves a tangent with its primal, in forward mode and in reverse mode
[{∊1 1 2⊆⍵×1 2 3}∂ 1; {+/1 2 3×∊1 1 2⊆⍵}∂ 2.5 1.5 3.5]   ⍝ [1 2 3;1 2 3]

⍝ derivative-moves-take — take (`↑`) moves a tangent with its primal, in forward mode and in reverse mode
[{2↑⍵×1 2 3}∂ 1; {+/1 2 3 4×4↑⍵}∂ 2.5 1.5 3.5]   ⍝ [1 2;1 2 3]

⍝ derivative-moves-drop — drop (`↓`) moves a tangent with its primal, in forward mode and in reverse mode
[{1↓⍵×1 2 3}∂ 1; {+/1 2×1↓⍵}∂ 2.5 1.5 3.5]   ⍝ [2 3;0 1 2]

⍝ derivative-moves-rotate — rotate (`⌽`) moves a tangent with its primal, in forward mode and in reverse mode
[{1⌽⍵×1 2 3}∂ 1; {+/1 2 3×1⌽⍵}∂ 2.5 1.5 3.5]   ⍝ [2 3 1;3 1 2]

⍝ derivative-moves-rotate-first — rotate first (`⊖`) moves a tangent with its primal, in forward mode and in reverse mode
[{1⊖⍵×[1 2⋄3 4]}∂ 1; {+/,[1 2⋄3 4]×1⊖⍵}∂ [1.5 2.5⋄3.5 4.5]]   ⍝ [[3 4⋄1 2] [3 4⋄1 2]]

⍝ derivative-moves-reorder-axes — reorder axes (`⍉`) moves a tangent with its primal, in forward mode and in reverse mode
[{0 0⍉⍵×[1 2⋄3 4]}∂ 1; {+/1 2×0 0⍉⍵}∂ [1.5 2.5⋄3.5 4.5]]   ⍝ [[1 4] [1 0⋄0 2]]

⍝ derivative-moves-index — index (`⌷`) moves a tangent with its primal, in forward mode and in reverse mode
[{[[2 0]]⌷⍵×1 2 3}∂ 1; {+/1 2×[[2 0]]⌷⍵}∂ 2.5 1.5 3.5]   ⍝ [3 1;2 0 1]

⍝ derivative-moves-bracket — bracket (`.[]`) moves a tangent with its primal, in forward mode and in reverse mode
[{(⍵×1 2 3).[[2 0]]}∂ 1; {+/1 2×⍵.[[2 0]]}∂ 2.5 1.5 3.5]   ⍝ [3 1;2 0 1]

⍝ derivative-moves-replicate — replicate (`#`) moves a tangent with its primal, in forward mode and in reverse mode
[{2 0 1#⍵×1 2 3}∂ 1; {+/1 2 3×2 0 1#⍵}∂ 2.5 1.5 3.5]   ⍝ [1 1 3;3 0 3]

⍝ derivative-moves-windows — windows (`↕`) moves a tangent with its primal, in forward mode and in reverse mode
[{2↕⍵×1 2 3}∂ 1; {+/,[1 2⋄3 4]×2↕⍵}∂ 2.5 1.5 3.5]   ⍝ [[1 2⋄2 3] [1 5 4]]

⍝ derivative-moves-keyed — keyed (`:`) moves a tangent with its primal, in forward mode and in reverse mode
[{"a" "b":⍵×1 2}∂ 1; {+/1 2×:"a" "b":⍵}∂ 2.5 1.5]   ⍝ [["a":1 "b":2];1 2]

⍝ derivative-orders-sort-up — sort up (`<`) orders a tangent with its primal, in forward mode and in reverse mode
[{<3 1 2+⍵×10 20 30}∂ 0; {+/1 2 3×<⍵}∂ 3.5 1.5 2.5]   ⍝ [20 30 10;3 1 2]

⍝ derivative-orders-sort-down — sort down (`>`) orders a tangent with its primal, in forward mode and in reverse mode
[{>3 1 2+⍵×10 20 30}∂ 0; {+/1 2 3×>⍵}∂ 3.5 1.5 2.5]   ⍝ [10 30 20;1 3 2]

⍝ derivative-orders-unique — unique (`∪`) orders a tangent with its primal, in forward mode and in reverse mode
[{∪3 1 3 2+⍵×10 20 30 40}∂ 0; {+/1 2 3×∪⍵}∂ 3.5 1.5 3.5 2.5]   ⍝ [10 20 40;1 2 0 3]

⍝ derivative-orders-duplicates — duplicates (`∩`) orders a tangent with its primal, in forward mode and in reverse mode
[{∩3 1 3 2 3+⍵×10 20 30 40 50}∂ 0; {+/1×∩⍵}∂ 3.5 1.5 3.5 2.5 3.5]   ⍝ [[10];1 0 0 0 0]

⍝ derivative-orders-runs — runs (`#`) orders a tangent with its primal, in forward mode and in reverse mode
[{1⊃#3 3 1+⍵×10 20 30}∂ 0; {+/1 2×1⊃#⍵}∂ 3.5 3.5 1.5]   ⍝ [10 30;1 0 2]

⍝ derivative-orders-union — union (`∪`) orders a tangent with its primal, in forward mode and in reverse mode
[{(1 2+⍵×10 20)∪2 3+⍵×30 40}∂ 0; {+/1 2 3×⍵∪2.5 3.5}∂ 1.5 2.5]   ⍝ [10 20 40;1 2]

⍝ derivative-orders-union-right — union (`∪`) orders a tangent with its primal, in forward mode and in reverse mode
[{1 2∪2 3+⍵×30 40}∂ 0; {+/1 2 3×1.5 2.5∪⍵}∂ 2.5 3.5]   ⍝ [0 0 40;0 3]

⍝ derivative-orders-intersection — intersection (`∩`) orders a tangent with its primal, in forward mode and in reverse mode
[{(1 2 3 4+⍵×10 20 30 40)∩2 4}∂ 0; {+/1 2×⍵∩2.5 4.5}∂ 1.5 2.5 3.5 4.5]   ⍝ [20 40;0 1 0 2]

⍝ derivative-orders-without — without (`~`) orders a tangent with its primal, in forward mode and in reverse mode
[{(1 2 3 4+⍵×10 20 30 40)~2 4}∂ 0; {+/1 2×⍵~2.5 4.5}∂ 1.5 2.5 3.5 4.5]   ⍝ [10 30;1 0 2 0]

⍝ derivative-orders-duplicates-repeated — duplicates (`∩`) keeps the first of each repeated value, in first-occurrence order
[{∩3 1 1 3 3 1+⍵×10 20 30 40 50 60}∂ 0; {+/1 2×∩⍵}∂ 3.5 1.5 1.5 3.5 3.5 1.5]   ⍝ [10 20;1 2 0 0 0 0]

⍝ derivative-orders-union-duplicates — union (`∪`) keeps duplicates from both arguments, with their tangents
[{(1 1 2+⍵×10 20 30)∪2 3 3+⍵×40 50 60}∂ 0; {+/1 2 3 4 5×⍵∪⍵+1}∂ 1.5 2.5 2.5]   ⍝ [10 20 30 50 60;1 6 8]

⍝ derivative-orders-intersection-duplicates — intersection (`∩`) and without (`~`) keep the left argument's duplicates, with their tangents
[{(1 1 2 3+⍵×10 20 30 40)∩1 3}∂ 0; {(1 1 2 3+⍵×10 20 30 40)~,1}∂ 0]   ⍝ [10 20 40;30 40]

⍝ derivative-orders-keys — ordered items keep their keys, with their tangents
[{∪("a" "b" "c":3 1 3)+⍵×10 20 30}∂ 0; {<("a" "b" "c":3 1 2)+⍵×10 20 30}∂ 0]   ⍝ [["a":10 "b":20] ["b":20 "c":30 "a":10]]

⍝ derivative-orders-cells — unique (`∪`) and sort (`<`) order major cells, with their tangents
[{∪[1 2⋄3 4⋄1 2]+⍵×[10 20⋄30 40⋄50 60]}∂ 0; {<[3 4⋄1 2]+⍵×[10 20⋄30 40]}∂ 0]   ⍝ [[10 20⋄30 40] [30 40⋄10 20]]

⍝ derivative-orders-stable — sort (`<`) keeps equal values in order, with their tangents
[{<3 1 3+⍵×10 20 30}∂ 0; {+/1 2 3×>⍵}∂ 3.5 1.5 3.5]   ⍝ [20 10 30;1 3 2]

⍝ derivative-sort-unit — sorting (`<`, `>`) a unit leaves it unchanged under ∂, as it does without ∂
[{<⍵}∂ 3; {>2×⍵}∂ 3]   ⍝ 1 2

⍝ derivative-orders-unit — intersection (`∩`) still refuses a unit right argument under ∂
{⍵∩3}∂ 1 2
⍝ error: RANK ERROR

⍝⍝ Optimizing

⍝ — ∂ differentiates the cost, so the search runs L-BFGS from the start [atol=1e-6]
rosen←{[x y]←⍵ ⋄ ⊼1-x + 100×⊼y-⊼x}
rosen⇄ ¯1.2 1   ⍝ 1 1

⍝ — An attached JVP rule gives the gradient that L-BFGS follows
rosen←{[x y]←⍵ ⋄ ⊼1-x + 100×⊼y-⊼x}
rosen∂←{[x y]←⍵ ⋄ +/⍺×[(¯2×1-x) - 400×x×y-⊼x; 200×y-⊼x]}
(["report":$t] rosen⇄ ¯1.2 1).evaluations<100   ⍝ $t

⍝ — Bounds make the search run Powell's method within them [atol=1e-6]
rosen←{[x y]←⍵ ⋄ ⊼1-x + 100×⊼y-⊼x}
["lower":[¯∞ 0] "upper":[0.8 ∞]] rosen⇄ 0.5 0.5
⍝ =>
0.8 0.64

⍝ — A number as the start runs Brent's method [atol=1e-6]
{⊼⍵-2}⇄ 10   ⍝ 2

⍝ — A record as the start gives a record with its keys [atol=1e-6]
cost←{(⊼⍵.gap-6) + ⊼⍵.margin-2×⍵.gap}
cost⇄ ["gap":8 "margin":12]
⍝ =>
["gap":6 "margin":12]

⍝ — With batch, a call gets every candidate along a new last axis, so each field holds one value per candidate [atol=1e-6]
cost←{(⊼⍵.gap-6) + ⊼⍵.margin-2×⍵.gap}
lo←["gap":0 "margin":0] ⋄ hi←["gap":20 "margin":40]
["method":"differential-evolution" "lower":lo "upper":hi "batch":$t] cost⇄ ["gap":8 "margin":12]
⍝ =>
["gap":6 "margin":12]

⍝ — The report gives the evaluations and whether the search converged before reaching their limit
r←["method":"nelder-mead" "evaluations":10 "report":$t] {+/⊼⍵-3}⇄ 1 2
[r.evaluations r.converged]   ⍝ 10ₓ $f

⍝ — A method that uses no derivative never differentiates the cost, so a call with two arguments of a function with a rule doesn't stop the search [atol=1e-4]
r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵}
["method":"nelder-mead"] {+/(10 r ⍵-2)}⇄ 0 0   ⍝ 2 2

⍝ — A start of 32-bit floats searches and answers in 32-bit floats
(•storage {⊼⍵-0.5}⇄ 1ₛ).kind   ⍝ "float32"

⍝ — The function must give one real number
{⊼⍵}⇄ 1 2
⍝ error: DOMAIN ERROR

⍝ — L-BFGS needs a gradient, which ∂ can't take through a system function. The •ucs round trip is there only as two numeric system functions with no rule under ∂. It gives back its whole numbers unchanged
["method":"l-bfgs"] {+/⊼•ucs •ucs ⍵}⇄ 1 2
⍝ error: DOMAIN ERROR

⍝ — Differential evolution needs finite bounds
["method":"differential-evolution"] {+/⊼⍵}⇄ 1 2
⍝ error: DOMAIN ERROR

⍝ — Bounds give one number, or one for each variable in the start's shape
["lower":[0 0 0]] {+/⊼⍵}⇄ 1 2
⍝ error: LENGTH ERROR

⍝ — Each lower bound must be at most its upper bound
["lower":2 "upper":1] {⊼⍵}⇄ 0
⍝ error: DOMAIN ERROR

⍝ — Data goes to the function through ↣ [atol=1e-6]
data←1 2 3 4 ⋄ [data↣{+/⊼⍺-⍵}⇄ 0; ["method":"powell"] data↣{+/⊼⍺-⍵}⇄ 0]   ⍝ 2.5 2.5

⍝ — The left argument holds options, not data for the function
1 2 3 4 {+/⊼⍺-⍵}⇄ 0
⍝ error: DOMAIN ERROR

⍝ — With solve, one residual in one variable runs Brent's method, stepping outward from the start for a sign change [atol=1e-12]
["solve":$t] (2-⍨⊼)⇄ 1   ⍝ 1.4142135623730951

⍝ — A bracket gives two points where the signs differ [atol=1e-12]
["solve":$t "bracket":[1 2]] (2-⍨⊼)⇄ 1.5   ⍝ 1.4142135623730951

⍝ — A bracket whose ends have one sign doesn't converge
(["solve":$t "bracket":[2 3] "report":$t] {⊼⍵-1}⇄ 2.5).converged   ⍝ $f

⍝ — Several residuals run Levenberg–Marquardt, here where a circle and a line cross [atol=1e-10]
f←{[x y]←⍵ ⋄ [(⊼x)+(⊼y)-4 y-x-1]}
["solve":$t] f⇄ 1 0   ⍝ 1.8228756555322954 0.8228756555322954

⍝ — An attached derivative gives Levenberg–Marquardt its Jacobian through JVPs [atol=1e-10]
f←{[x y]←⍵ ⋄ [(⊼x)+(⊼y)-4 y-x-1]}
f∂←{[x y]←⍵ ⋄ [p q]←⍺ ⋄ [(2×x×p)+2×y×q; q-p]}
["solve":$t] f⇄ 1 0   ⍝ 1.8228756555322954 0.8228756555322954

⍝ — More residuals than variables give the point where the sum of their squares is least [atol=1e-6]
x←0 1 2 3 4 ⋄ y←2.1 5.4 15.2 40.3 109.1 ⋄ r←{⍵.((a×*b×x)-y)}
(["solve":$t] r⇄ ["a":1 "b":1]) - ["tolerance":1E¯12] {+/⊼ r ⍵}⇄ ["a":1 "b":1]
⍝ =>
["a":0 "b":0]

⍝ — With linear, a square system runs GMRES on products from the function [atol=1e-12]
m←[4 1⋄1 3] ⋄ b←1 2
(["solve":$t "linear":$t] {(m+.×⍵)-b}⇄ 0 0) - b⌹m   ⍝ 0 0

⍝ — A large right-hand side costs no accuracy, because each product is computed at a scaled point [atol=1e-12]
m←[4 1⋄1 3] ⋄ b←1E8×1 2
(["solve":$t "linear":$t] {(m+.×⍵)-b}⇄ 0 0) ÷ b⌹m   ⍝ 1 1

⍝ — CG on a symmetric positive definite system [atol=1e-12]
m←[4 1⋄1 3] ⋄ b←1 2
(["solve":$t "linear":$t "method":"cg"] {(m+.×⍵)-b}⇄ 0 0) - b⌹m   ⍝ 0 0

⍝ — An attached JVP rule, transposed, gives the products with the transpose for least squares, and the report gives the residual's norm [atol=1e-10]
t←[1 0⋄0 1⋄1 1] ⋄ c←1 2 4
g←{(t+.×⍵)-c} ⋄ g∂←{t+.×⍺}
r←["solve":$t "linear":$t "report":$t] g⇄ 0 0
[r.x-c⌹t; r.value]   ⍝ [0 0; 0.5773502691896258]

⍝ — Least squares needs products with the transpose, which ∂ can't give through a system function. The •ucs round trip is there only as two numeric system functions with no rule under ∂. It gives back its whole numbers unchanged
["solve":$t "linear":$t] {([1 0⋄0 1⋄1 1]+.×•ucs •ucs ⍵)-1 2 4}⇄ 0 0
⍝ error: DOMAIN ERROR

⍝ — Linear needs solve
["linear":$t] {⍵-1}⇄ 0
⍝ error: DOMAIN ERROR

⍝ — Bounds apply only to minimizing
["solve":$t "lower":0] {⍵-1}⇄ 0
⍝ error: DOMAIN ERROR

⍝ — The function gives the same number of residuals at every point
["solve":$t] {(1+⍵>2)⍴⍵-3}⇄ 1
⍝ error: LENGTH ERROR

⍝ — A diagonal preconditioner cuts the products a badly scaled system needs [atol=1e-10]
n←20 ⋄ d←10*5×(⍳n)÷n-1 ⋄ m←(d×=⊗⍨⍳n)+0.1×1=|-⊗⍨⍳n ⋄ b←n⍴1
f←{(m+.×⍵)-b} ⋄ o←["solve":$t "linear":$t "report":$t "tolerance":1E¯12]
p←o f⇄ n⍴0 ⋄ q←(o,["precondition":{⍵÷d}]) f⇄ n⍴0
[q.evaluations<p.evaluations; ⌈/|q.x-b⌹m]   ⍝ [$t; 0]

⍝ — A preconditioner needs a linear solve
["solve":$t "precondition":{⍵}] {⍵-1}⇄ 0
⍝ error: DOMAIN ERROR

⍝⍝ Prime and factor families

⍝ — Prime indexing counts from 0 and returns exact integers
⍭⍳8ₓ   ⍝ [2 3 5 7 11 13 17 19]ₓ

⍝ — Prime lookup preserves shape and accepts repeated, unordered indices
⍭[9999 0⋄9999 1]   ⍝ [104729 2⋄104729 3]ₓ

⍝ —
n←4 ⋄ ⍭n   ⍝ 11ₓ

⍝ — Previous prime is strictly below the argument
¯2⍭3 4 5 6   ⍝ [2 3 3 5]ₓ

⍝ — Count primes strictly below each argument
⍭⁻¹1 2 3 4 5 6   ⍝ [0 0 1 2 2 3]ₓ

⍝ — Test primality rather than indexing the prime sequence
1⍭¯1 0 1 2 3 4   ⍝ $f $f $f $t $t $f

⍝ — Next prime is strictly above the argument
2⍭1 2 3 4 5   ⍝ [2 3 5 5 7]ₓ

⍝ — Euler's totient counts positive integers up to n coprime to n
3⍭1 2 3 4 5 6 10   ⍝ [1 1 2 2 4 2 4]ₓ

⍝ —
⨸700   ⍝ [2 2 5 5 7]ₓ

⍝ — One has an empty exact factorization
⨸1   ⍝ ⍬ₓ

⍝ — Return exponents of only the first two primes
2⨸700   ⍝ [2 0]ₓ

⍝ — Requested primes beyond the largest factor receive zero exponents
10⨸700   ⍝ [2 0 2 1 0 0 0 0 0 0]ₓ

⍝ — Infinite count includes every prime through the largest factor
∞⨸700   ⍝ [2 0 2 1]ₓ

⍝ — Negative count selects the last distinct factors and their exponents
¯2⨸700   ⍝ [5 7⋄2 1]ₓ

⍝ — Negative infinity requests the complete factor/exponent table
¯∞⨸700   ⍝ [2 5 7⋄2 2 1]ₓ

⍝ —
0⨸700   ⍝ ⍬ₓ

⍝ — Factoring one retains the table's two-row shape
¯∞⨸1   ⍝ 2 0⍴0ₓ

⍝ — Inverse prime lookup recovers the positions, counting from 0
⍭⁻¹⍭⍳10ₓ   ⍝ ⍳10ₓ

⍝ — Inverse factorization multiplies the factors
⨸⁻¹⨸700   ⍝ 700ₓ

⍝ — Primality accepts exact integers at and beyond the unsigned 64-bit boundary
1⍭[18446744073709551557 18446744073709551615 170141183460469231731687303715884105727]ₓ
$t $f $t

⍝ — Strong pseudoprimes must not pass as primes
1⍭[341550071728321 3825123056546413051]ₓ   ⍝ $f $f

⍝ — Factor a semiprime whose two factors are both large
⨸1000000016000000063ₓ   ⍝ [1000000007 1000000009]ₓ

⍝ — Empty prime lookup preserves shape with an exact prototype
⍭0 2⍴0   ⍝ 0 2⍴0ₓ

⍝ — Factor lists of unequal lengths assemble with zero fill
⨸2 12   ⍝ 2 3⍴[2 0 0 2 2 3]ₓ

⍝ —
⍭¯1
⍝ error: DOMAIN ERROR

⍝ — A prime index accepts a float within comparison tolerance of an integer
⍭1.000000000000001   ⍝ 3ₓ

⍝ —
1⍭2.5
⍝ error: DOMAIN ERROR

⍝ —
⨸0
⍝ error: DOMAIN ERROR

⍝ —
⨸¯1
⍝ error: DOMAIN ERROR

⍝ —
⨸2j1
⍝ error: DOMAIN ERROR

⍝ —
6⍭7
⍝ error: DOMAIN ERROR

⍝ — There is no prime strictly below two
¯2⍭2
⍝ error: DOMAIN ERROR

⍝⍝ Windows

⍝ — Full vector windows overlap without padding
3↕⍳5ₓ   ⍝ [0 1 2⋄1 2 3⋄2 3 4]ₓ

⍝ — Monadic `↕` gives the adjacent pairs, as `2↕` does
↕1 4 9 16   ⍝ [1 4⋄4 9⋄9 16]

⍝ — Window-position axes precede the two-dimensional window axes
2 2↕2 3⍴⍳6ₓ   ⍝ (1 2 2 2⍴0 1 3 4 1 2 4 5)ₓ

⍝ — One window length slides along the leading axis, retaining whole rows
2↕3 2⍴⍳6ₓ   ⍝ (2 2 2⍴0 1 2 3 2 3 4 5)ₓ

⍝ — An oversized window produces no windows but retains window shape and character fill
4↕"ab"   ⍝ 0 4⍴' '

⍝ — Zero-length windows occur at every boundary, including both ends
0↕"ab"   ⍝ 3 0⍴' '

⍝ — No window axes leaves the argument unchanged
⍬↕2 3⍴⍳6ₓ   ⍝ 2 3⍴⍳6ₓ

⍝ — Zero-length and nonempty window axes combine without losing the frame
0 2↕2 3⍴⍳6ₓ   ⍝ (3 2 0 2⍴0)ₓ

⍝ — Negative sizes give stencil windows, including even sizes
(¯4↕⍳5ₓ)≡{⍵}⌺4⍳5   ⍝ $t

⍝ — Padded matrix windows with movements match stencil windows
([¯3 ¯2⋄2 1]↕4 5⍴⍳20ₓ)≡{⍵}⌺[3 2⋄2 1] 4 5⍴⍳20ₓ   ⍝ $t

⍝ — A fold along a vector's or matrix's windows slides over the source, directly for narrow windows and by van Herk-Gil-Werman for wide ones, and matches folding each window alone, NaN included
v←¿3000⍴0 ⋄ i←17 1000 ⋄ v.[i]←$n ⋄ w←¿45000⍴0 ⋄ m←¿[130 130]⍴0 ⋄ j←¿3000⍴100ₓ ⋄ b←0=¿3000⍴3ₓ ⋄ ∧/[(+/50↕v)≡{+/⍵}⍤1⊢50↕v;(⌈/50↕w)≡{⌈/⍵}⍤1⊢50↕w;(⌊/3↕w)≡{⌊/⍵}⍤1⊢3↕w;(+/+/3 3↕m)≡{+/,⍵}⍤2⊢3 3↕m;(+/50↕j)≡{+/⍵}⍤1⊢50↕j;(∨/50↕b)≡{∨/⍵}⍤1⊢50↕b;(⌈⌿3↕w)≡{⌈/⍵}⍤1⊢⍉3↕w;["sliding fold"]≡(•explain "+/50↕v").fast.combination]   ⍝ $t

⍝ — Under `∂`, a maximum or minimum of windows takes each window's first extreme from the source, as the fold of built windows does
w←¿45000⍴0 ⋄ t←⌊10×w ⋄ ∧/[({+/,⌈/50↕⍵}∂ w)≡{+/,⌈/1×50↕⍵}∂ w;({+/,⌊/50↕⍵}∂ t)≡{+/,⌊/1×50↕⍵}∂ t;({+/,⌈/3↕⍵}∂ t)≡{+/,⌈/1×3↕⍵}∂ t]   ⍝ $t

⍝ — With an edge mode in an options record, every windowed axis is centred, and its edges read by that mode
["size":3 "edge":"wrap"]↕1 2 3 4   ⍝ [4 1 2⋄1 2 3⋄2 3 4⋄3 4 1]

⍝ — Clamped, reflected with a movement, and symmetric windows
x←⍳5ₓ ⋄ [["size":3 "edge":"clamp"]↕x;["size":3 "edge":"reflect" "step":2]↕x;["size":4 "edge":"symmetric"]↕x]   ⍝ [[0 0 1⋄0 1 2⋄1 2 3⋄2 3 4⋄3 4 4]ₓ [1 0 1⋄1 2 3⋄3 4 3]ₓ [0 0 1 2⋄0 1 2 3⋄1 2 3 4⋄2 3 4 4]ₓ]

⍝ — A record without an edge mode reads as the plain specification does, and the fill mode pads as a negative size does
x←⍳5ₓ ⋄ ∧/[(["size":[3]]↕x)≡3↕x;(["size":¯3]↕x)≡¯3↕x;(["size":3 "edge":"fill"]↕x)≡¯3↕x;(["size":[3 2] "step":[1 2]]↕4 5⍴⍳20ₓ)≡[3 2⋄1 2]↕4 5⍴⍳20ₓ]   ⍝ $t

⍝ — Folds of wrapped and clamped windows slide over the padded source, and a product of them reads the windows in blocks
v←¿3000⍴0 ⋄ k←¿3 4⍴0 ⋄ wr←["size":50 "edge":"wrap"] ⋄ cl←["size":3 "edge":"clamp"] ⋄ ∧/[(+/wr↕v)≡{+/⍵}⍤1⊢wr↕v;(⌈/cl↕v)≡{⌈/⍵}⍤1⊢cl↕v;["sliding fold"]≡(•explain "+/wr↕v").fast.combination;(([3000 3]⍴cl↕v)+.×k)≡(1×[3000 3]⍴cl↕v)+.×k]   ⍝ $t

⍝ —
["size":3 "pad":1]↕⍳3ₓ
⍝ error: DOMAIN ERROR

⍝ —
["size":3 "edge":"nope"]↕⍳3ₓ
⍝ error: DOMAIN ERROR

⍝ —
["edge":"wrap"]↕⍳3ₓ
⍝ error: DOMAIN ERROR

⍝ —
["size":0 "edge":"wrap"]↕⍳3ₓ
⍝ error: DOMAIN ERROR

⍝ —
0.5↕⍳3ₓ
⍝ error: DOMAIN ERROR

⍝ —
2↕3
⍝ error: RANK ERROR

⍝ —
1 2↕⍳3ₓ
⍝ error: RANK ERROR

⍝ —
[[2]⋄]↕⍳3ₓ
⍝ error: LENGTH ERROR

⍝ —
(1 1 1⍴2)↕⍳3ₓ
⍝ error: RANK ERROR

⍝ —
[[2]⋄[0]]↕⍳3ₓ
⍝ error: DOMAIN ERROR

⍝ —
¯4↕⍳2ₓ
⍝ error: DOMAIN ERROR

⍝⍝ Assigned inverses, under and trajectories

⍝ — Negative power uses an assigned inverse
f←{⍵+1} ⋄ f⁻¹←{⍵-1} ⋄ f⍣¯2⊢5   ⍝ 3

⍝ — A dyadic assigned inverse recovers the right argument with the left fixed
f←{⍺+⍵} ⋄ f⁻¹←{⍵-⍺} ⋄ 3 f⁻¹ 8   ⍝ 5

⍝ — Binding the left argument retains the assigned inverse
f←{⍺+⍵} ⋄ f⁻¹←{⍵-⍺} ⋄ (3↣f)⁻¹8   ⍝ 5

⍝ — Inverting a function with an assigned inverse recovers the function
f←{⍵+1} ⋄ f⁻¹←{⍵-1} ⋄ (f⁻¹)⁻¹5   ⍝ 6

⍝ — Assigning an inverse inside a dfn updates the nearest binding, as selective assignment does
f←{⍵+1} ⋄ g←{⍵-1} ⋄ {f⁻¹←g ⋄ 0}0 ⋄ f⁻¹ 5   ⍝ 4

⍝ — Dyadic under transforms both arguments, then inverts the result
3 +@ 2× 4   ⍝ 7

⍝ — Monadic under changes coordinates before and after reversal
⌽@ 1+ 1 2 3   ⍝ 3 2 1

⍝ under-dfn-selector — A dfn selector picks what Under changes, as its body selects
r←["a":1 "b":2] ⋄ (1+)@{⍵.a} r   ⍝ ["a":2 "b":2]

⍝ —
v←10 20 30 ⋄ (1+)@{⍵.[⍸⍵>15]} v   ⍝ 10 21 31

⍝ —
v←10 20 30 ⋄ (1+)@{2↑⌽⍵} v   ⍝ 10 21 31

⍝ —
v←10 20 30 ⋄ (1+)@{m←⍵>15 ⋄ m#⍵} v   ⍝ 10 21 31

⍝ —
v←10 20 30 ⋄ (1+)@{[⍵₀ ⍵₂]} v   ⍝ 11 20 31

⍝ — Each inside a selector selects each item
v←10 20 30 ⋄ (1+)@{{⍵}¨⍵} v   ⍝ 11 21 31

⍝ under-selector-once — Under calls its selector once
n←0 ⋄ v←10 20 30 ⋄ r←(1+)@{n+←1 ⋄ 1↑⍵} v ⋄ n r   ⍝ [1 [11 20 30]]

⍝ under-dfn-selector-derivative — A dfn selector works under ∂
{+/(2×)@{1↑⍵} ⍵}∂ 1 2 3   ⍝ 2 1 1

⍝ under-walk-column — A selection that reads its target in a pattern writes back along the same pattern
(1ₓ+)@{[: 0]⌷⍵} [3 2]⍴⍳6ₓ   ⍝ [1 1⋄3 3⋄5 5]ₓ

⍝ under-replicate-repeated — A count of 2 selects an item twice, and the later write wins
(1+)@(1 0 2↣#) 10 20 30   ⍝ 11 20 31

⍝ — Power preserves the count array's shape, including negative and repeated counts
1+ ⍣[3 ¯2⋄0 3] 10   ⍝ [13 8⋄10 13]

⍝ — A singleton count vector adds a singleton result frame
+⍣[1] 2   ⍝ ,2

⍝ — Each state is one item, whatever its length
{⍵,1}⍣0 1 2 ,2   ⍝ (2⋄2 1⋄2 1 1)

⍝ — Empty counts never call the operand, and give an empty array of the counts' shape with the argument's fill as prototype
{1÷0}⍣(0 2⍴0) "ab"   ⍝ 0 2⍴⊂"  "

⍝ — Counts from 0 give the starting value and each step
1+⍣(⍳4ₓ) 5   ⍝ 5 6 7 8

⍝ — Negative counts follow the inverse
1+⍣(-⍳3ₓ) 5   ⍝ 5 4 3

⍝ — A one-item vector holding a predicate keeps every state, including the one that satisfies the test
1 +⍣[{⍺=3}] 0   ⍝ 0 1 2 3

⍝ — Fixed-point history includes both the initial and unchanged next value
⊢⍣[≡] 4   ⍝ 4 4

⍝ — An infinite count runs until the state stops changing, the same test as ⍣≡
{0.5×⍵+2÷⍵}⍣∞⊢1   ⍝ {0.5×⍵+2÷⍵}⍣≡(1)

⍝ — An infinite count can sit in a list of counts
{⌊⍵÷2}⍣[0 1 ∞] 100ₓ   ⍝ [100 50 0]ₓ

⍝ — A negative infinite count runs the inverse until it converges
d←{2×⍵} ⋄ d⁻¹←{⌊⍵÷2} ⋄ d⍣¯∞⊢100   ⍝ 0ₓ

⍝ — A zero count in a list keeps the initial value without calling the operand
{1÷0}⍣[0] "ab"   ⍝ ["ab"]

⍝ — A named predicate list uses ordinary function calls and global lookup
limit←4
stop←[{⍺≥limit}]
1+⍣stop 1   ⍝ 1 2 3 4

⍝ — Counts must be integral or infinite
+⍣0.5(1)
⍝ error: DOMAIN ERROR

⍝ — A list of counts is not inverted as if it were ordinary repeated application
((1+)⍣ ⍳3ₓ)⁻¹1 2 3
⍝ error: DOMAIN ERROR

⍝ — Parenthesizing power leaves the following backslash as ordinary scan
(+⍣1)\1 2 3   ⍝ 1 3 6

⍝ — An assigned inverse recovers the right argument only, so it gives no left-argument inverse
p←+ ⋄ p⁻¹←- ⋄ (p↢3)⁻¹8
⍝ error: DOMAIN ERROR

⍝ —
+⍣0 0.5(1)
⍝ error: DOMAIN ERROR

⍝ — Repeated power counts reuse iterates rather than calling the operand again
f←{⎕←⍵+1} ⋄ f⁻¹←{⎕←⍵-1} ⋄ f⍣3 ¯2 3 0⊢0
3 ¯2 3 0
⍝ ⎕: 1\n2\n3\n¯1\n¯2

⍝ — Under transforms right then left, calls the operand, then applies the inverse
g←{⎕←⍵ ⋄ 2×⍵} ⋄ g⁻¹←{⎕←⍵ ⋄ ⍵÷2} ⋄ 3(+@g)4
7
⍝ ⎕: 4\n3\n14

⍝⍝ Inverse power

⍝ — Inverting x×exp(x) gives the principal Lambert W function
W←×↢*⍨⁻¹ ⋄ ⌊1E12×W [0 1 *1 ¯0.1 1j1]
0 567143290409 1000000000000 ¯111832559159 656966069230j325450339413

⍝ — Lambert W reaches -1 at its real branch point -1/e
W←×↢*⍨⁻¹ ⋄ W ¯1÷*1   ⍝ ¯1

⍝ — Lambert W pervades nested exact input and produces approximate values
W←×↢*⍨⁻¹ ⋄ W [[0 0]ₓ 0⍴0ₓ]   ⍝ [0 0;⍬]

⍝ — Check W(x)exp(W(x))=x across tiny and large real inputs
W←×↢*⍨⁻¹
x←1E¯100 ¯1E¯100 1E¯12 ¯1E¯12 0.099 ¯0.099 1E300
∧/1E¯12>|1-(W x)×(*W x)÷x   ⍝ $t

⍝ — Check the Lambert W inverse identity on complex inputs
W←×↢*⍨⁻¹
x←¯1j0.1 1j¯1
∧/1E¯12>|1-(W x)×(*W x)÷x   ⍝ $t

⍝ — A real Lambert W argument below -1/e gives the complex principal value
×↢*⍨⁻¹ ¯1   ⍝ ¯0.31813150520476396j1.3372357014306895

⍝ —
×↢*⍨⁻¹ 'a'
⍝ error: DOMAIN ERROR

⍝ — Invert an outer subtraction with its right vector fixed
(-⊗↢4 5)⁻¹[¯3 ¯4⋄¯2 ¯3]   ⍝ 1 2

⍝ — Invert an outer subtraction with its left vector fixed
4 5↣(-⊗)⁻¹[3 2⋄4 3]   ⍝ 1 2

⍝ — Outer-product inversion preserves exact rational results
(×⊗↢[4 5]ₓ)⁻¹[2ₓ 5r2⋄1ₓ 5r4]   ⍝ 1r2 1r4

⍝ — A fixed matrix occupies the trailing axes of the outer-product result
(×⊗↢[1 2⋄3 4])⁻¹3 2 2⍴1 2 3 4 2 4 6 8 3 6 9 12   ⍝ 1 2 3

⍝ — Outer inversion with a fixed number recovers a vector
4↣(×⊗)⁻¹4 8   ⍝ 1 2

⍝ — Removing the fixed vector's axes can leave a scalar
(×⊗↢4 5)⁻¹4 5   ⍝ ⊂1

⍝ — Outer inversion recovers an empty argument from an empty result frame
(+⊗↢4 5)⁻¹0 2⍴0   ⍝ ⍬

⍝ — With the left vector fixed, the remaining axes describe the recovered matrix
4 5↣(-⊗)⁻¹2 2 2⍴3 2 1 0 4 3 2 1   ⍝ [1 2⋄3 4]

⍝ — Outer inversion preserves zero dimensions in the recovered frame
(+⊗↢4 5)⁻¹3 0 2⍴0   ⍝ 3 0⍴0

⍝ — Every outer-product cell must imply the same recovered value
(×⊗↢4 5)⁻¹[4 5⋄8 11]
⍝ error: DOMAIN ERROR

⍝ — The result axes must agree with the fixed outer-product argument
(×⊗↢4 5)⁻¹2 3⍴⍳6ₓ
⍝ error: DOMAIN ERROR

⍝ — An empty fixed argument contains no information to invert
(×⊗↢⍬)⁻¹2 0⍴0
⍝ error: DOMAIN ERROR

⍝ — Multiplication by an all-zero fixed argument is not invertible
(×⊗↢0 0)⁻¹2 2⍴0
⍝ error: DOMAIN ERROR

⍝ — Inverse iota returns the generating shape as an exact vector
⍳⁻¹0 1 2   ⍝ [3]ₓ

⍝ — Inverse iota also recognizes multidimensional index arrays
⍳⁻¹⍳[2 3]ₓ   ⍝ [2 3]ₓ

⍝ —
1+⍣¯3⊢10   ⍝ 7

⍝ — Invert a Celsius-to-Fahrenheit section in reverse function order
(32+1.8×)⁻¹32 212   ⍝ 0 100

⍝ — Inverse sum scan recovers successive differences
+\⁻¹1 3 6 10   ⍝ 1 2 3 4

⍝ — Inverse scan retains the exact numeric domain
+\⁻¹[1 3 6]ₓ   ⍝ [1 2 3]ₓ

⍝ — Inverse Boolean xor scan recovers changes between adjacent values
≠\⁻¹[1 1 0 0]ₓ   ⍝ [1 0 1 0]ₓ

⍝ — Inverse base-two decode chooses the required number of digits
2↣⊥⁻¹9   ⍝ 1 0 0 1

⍝ — Exact base and value produce exact inverse-decode digits
2ₓ↣⊥⁻¹9ₓ   ⍝ [1 0 0 1]ₓ

⍝ — Invert mixed-radix time decoding into hours, minutes and seconds
([0 60 60]ₓ↣⊥⁻¹)3661ₓ   ⍝ [1 1 1]ₓ

⍝ — Inverse encode decodes the supplied digits
2ₓ↣⊤⁻¹[1 0 1]ₓ   ⍝ 5ₓ

⍝ — Inverse decode permits a fractional least-significant digit
2↣⊥⁻¹2.5   ⍝ 1 0.5

⍝ — Inverse transpose applies the inverse axis permutation
(1 0↣⍉⁻¹)2 3⍴⍳6ₓ   ⍝ [0 3⋄1 4⋄2 5]ₓ

⍝ — Inverting self-addition halves the argument exactly
+⍨⁻¹3ₓ   ⍝ 3r2

⍝ — Inverse sine selects a branch which maps back to the input
0.5=1↣○(1↣○⁻¹)0.5   ⍝ $t

⍝ — Inverse sine within the real domain has no imaginary component
⊢/∨(1↣○⁻¹)0.5   ⍝ 0

⍝ — sqrt(1+x²) avoids overflowing its intermediate square
4○1E300   ⍝ 1E300

⍝ — Signed sqrt(x²-1) avoids intermediate overflow for large negative x
4○⁻¹¯1E300   ⍝ ¯1E300

⍝ — Inversion preserves rank-zero application
(2ₓ↣+⍤0)⁻¹[3 4]ₓ   ⍝ [1 2]ₓ

⍝ — Inverse where counts repeated indices
⍸⁻¹0 2 2   ⍝ [1 0 2]ₓ

⍝ — Inverse where infers multidimensional shape from coordinate vectors
⍸⁻¹[0 1;1 0]   ⍝ [0 1⋄1 0]ₓ

⍝ — Inverse where of no indices returns an empty exact count vector
⍸⁻¹⍬   ⍝ ⍬ₓ

⍝ — Zero needs no digits in minimal-width inverse decode
2↣⊥⁻¹0   ⍝ ⍬

⍝ —
3ₓ+⁻¹5ₓ   ⍝ 2ₓ

⍝ — Inversion distributes through each
(2ₓ↣+)¨⁻¹[3 4]ₓ   ⍝ [1 2]ₓ

⍝ — Inverting self-multiplication selects the positive square root
×⍨⁻¹4   ⍝ 2

⍝ —
⌽⁻¹⍳3ₓ   ⍝ [2 1 0]ₓ

⍝ —
⊂⁻¹⊂[1 2]ₓ   ⍝ [1 2]ₓ

⍝ — Multiplication by zero has no inverse
0↣×⁻¹0
⍝ error: DOMAIN ERROR

⍝ — Inverse where counts positions in any order
⍸⁻¹3 1   ⍝ [0 1 0 1]ₓ

⍝ —
⍸⁻¹¯1
⍝ error: DOMAIN ERROR

⍝ —
2↣⊥⁻¹¯1
⍝ error: DOMAIN ERROR

⍝ — Inverse decode keeps the low digits of a value beyond its radix range
(2 2↣⊥⁻¹)5   ⍝ 0 1

⍝⍝ Inverse combinations

⍝ — Right binding requires inversion with respect to the left argument of composition
(+↢-↢2)⁻¹3   ⍝ 5

⍝ — Commute swaps which argument must be recovered
2 (+↢-)⍨⁻¹ 3   ⍝ 5

⍝ — Invert atop with a fixed right argument
(÷∘-↢2)⁻¹0.5   ⍝ 4

⍝ — Invert over with a fixed right argument
(-⍥-↢2)⁻¹¯3   ⍝ 5

⍝ — Before transforms the fixed left argument before solving for the right
3 -↣+⁻¹ 7   ⍝ 10

⍝ — Right-bound before also inverts the left-side transformation
(-↣+↢3)⁻¹¯7   ⍝ 10

⍝ — Right-bound each recovers every left element
(-¨↢2)⁻¹3 4   ⍝ 5 6

⍝ — Right-bound rank recovers vector cells against the fixed number
(-⍤1 0↢2)⁻¹3 4   ⍝ 5 6

⍝ — Invert left-accumulating subtraction scan
-\⁻¹1 ¯1 ¯4   ⍝ 1 2 3

⍝ — Inverse seeded scan uses the seed to recover the first input
10-\⁻¹9 7 4   ⍝ 1 2 3

⍝ — Binding the seed retains scan inversion
10↣(+\)⁻¹11 13 16   ⍝ 1 2 3

⍝ — Scan inversion calls the composed operand's inverse
(-↢-)\⁻¹1 3 6   ⍝ 1 2 3

⍝ — Inverting seeded division scan retains exact fractions
2ₓ÷\⁻¹1ₓ 1r3 1r12   ⍝ [2 3 4]ₓ

⍝ — Axis-one scan inversion uses a separate seed for each column
⍉ 10 20 +\⁻¹⍤0 1 [11 22⋄14 26]ᵀ   ⍝ [1 2⋄3 4]

⍝ —
+\⁻¹⍬   ⍝ ⍬

⍝ — A seeded scan of a number still needs its first step undone
10+\⁻¹13   ⍝ 3

⍝ — Inverse axis-enclosure restores both shape and axis order
(⊂⍠1 0)⁻¹ ⊂⍠1 0 [2 3 4]⍴⍳24ₓ   ⍝ 2 3 4⍴⍳24ₓ

⍝ — Inverse split restores the original matrix axis
(↓⍠0)⁻¹ ↓⍠0 [2 3]⍴⍳6ₓ   ⍝ 2 3⍴⍳6ₓ

⍝ — Inverse mix along an axis recovers the original nested vectors
⊃⁻¹⍠0 [1 4⋄2 5⋄3 6]   ⍝ [1 2 3;4 5 6]

⍝ — Inverting a pervasive function with an axis aligns the fixed left vector with rows
10 20 +⁻¹⍠0 [11 12 13⋄24 25 26]   ⍝ [1 2 3⋄4 5 6]

⍝ — A zero product prefix loses information about subsequent factors
×\⁻¹1 0 0
⍝ error: DOMAIN ERROR

⍝ — A zero product seed loses information from the first step
0 ×\⁻¹ 0 0
⍝ error: DOMAIN ERROR

⍝ — Monadic (-↣+) is constant zero, so its input cannot be recovered
-↣+⁻¹ 0
⍝ error: DOMAIN ERROR

⍝ — Rank checks agreement between row seeds and cells
1 2 3 (+\⁻¹)⍤0 1 [2 3]⍴⍳6ₓ
⍝ error: LENGTH ERROR

⍝ — Inverse polar form rebuilds complex numbers
∧⁻¹ ∧ 3j4 1j1   ⍝ 3j4 1j1

⍝ — Inverse real and imaginary parts read the pairs on the last axis
∨⁻¹ [3 4⋄1 0]   ⍝ 3j4 1

⍝ — Inverse grade gives an array whose grade is the permutation
⍋⁻¹ 2 0 1   ⍝ [1 2 0]ₓ

⍝ — Inverse grade down
⍒ ⍒⁻¹ 2 0 1   ⍝ [2 0 1]ₓ

⍝ — Only a permutation is a grade
⍋⁻¹ 2 2 1
⍝ error: DOMAIN ERROR

⍝ — Inverse execute writes source that execute reads back
⍎⁻¹ [1 2.5;"ab"]   ⍝ "[[1 2.5]ₚ ""ab""]"

⍝ — Inverse ravel and table give back a vector
[,⁻¹ 1 2 3;⍪⁻¹ ⍪1 2 3]   ⍝ [1 2 3;1 2 3]

⍝ — Inverse pairs and windows rebuild the array
[↕⁻¹ ↕⍳4ₓ;3↕⁻¹ 3↕"abcde"]   ⍝ [0 1 2 3;"abcde"]ₓ

⍝ — Inverse reshape with the shape fixed ravels
2 3⍴⁻¹ 2 3⍴⍳6ₓ   ⍝ [0 1 2 3 4 5]ₓ

⍝ — Inverse index-of with the table fixed selects from it
"abc"⍳⁻¹ 2 0   ⍝ "ca"

⍝ — An index that wasn't found has no argument
"abc"⍳⁻¹ 3
⍝ error: DOMAIN ERROR

⍝ — Inverse keyed with the keys fixed removes them
"a" "b":⁻¹ ["a":1 "b":2]   ⍝ 1 2

⍝ — Equal on Booleans is its own inverse
1 0=⁻¹ 1 1   ⍝ $t $f

⍝ — Inverse partitioned enclose and partition join the parts
[1 2 0⊂⁻¹ 1 2 0⊂1 2 3;1 1 2⊆⁻¹ 1 1 2⊆1 2 3]   ⍝ [1 2 3;1 2 3]

⍝ — A partition that dropped items has no argument
0 1 1⊂⁻¹ (2⋄3)
⍝ error: DOMAIN ERROR

⍝ — Inverse •ucs converts back, with or without an encoding
[•ucs⁻¹ 97 98;"UTF-8" •ucs⁻¹ 195 169]   ⍝ ["ab";"é"]

⍝ — A distribution's cdf and quantile invert each other
d←•distribution "normal" ⋄ [d.cdf⁻¹ 0.5;d.quantile⁻¹ 0]   ⍝ 0 0.5

⍝ — Inverse •decompose multiplies SVD factors back into the matrix
"svd" •decompose⁻¹ "svd" •decompose 3 2⍴1 2 3 4 5 6
⍝ =>
[1 2⋄3 4⋄5 6]

⍝ — Inverse •decompose multiplies thin QR factors of a wide matrix back
"qr" •decompose⁻¹ "qr" •decompose 2 3⍴1 2 3 4 5 6
⍝ =>
[1 2 3⋄4 5 6]

⍝ — Inverse •decompose multiplies a complex Cholesky factor by its conjugate transpose
"cholesky" •decompose⁻¹ "cholesky" •decompose 2 2⍴4 1j1 1j¯1 3
⍝ =>
[4 1j1⋄1j¯1 3]

⍝ — Inverse •decompose rebuilds a matrix from complex eigenvalues and eigenvectors
"eigen" •decompose⁻¹ "eigen" •decompose 2 2⍴0 ¯1 1 0
⍝ =>
[0 ¯1⋄1 0]

⍝ — Eigenvectors that aren't independent can't give back a defective matrix
"eigen" •decompose⁻¹ "eigen" •decompose 2 2⍴1 1 0 1
⍝ error: DOMAIN ERROR

⍝ — •decompose⍤2 gives each matrix of a stack what it gives that matrix alone: a Hermitian matrix's real eigenvalues, beside a rotation's complex ones
s←2 2 2⍴2 1 1 2 0 ¯1 1 0
e←"eigen"•decompose⍤2 s
[(0⌷e)≡("eigen" •decompose 0⌷s);(1⌷e)≡("eigen" •decompose 1⌷s)]
⍝ =>
$t $t

⍝ — Inverse •decompose needs factors whose sizes agree
"qr" •decompose⁻¹ ["q":2 2⍴1 0 0 1;"r":3 3⍴1]
⍝ error: LENGTH ERROR

⍝ — Under writes back through the positions that ↕ selects
{⍵×10}@↕ 1 2 3   ⍝ 10 20 30

⍝⍝ Compact integers and promotion

⍝ — Exact tally keeps the mean of exact values rational
avg←+/÷≢ ⋄ avg [1 2 4]ₓ   ⍝ 7r3

⍝ —
+/⍳4ₓ   ⍝ 6ₓ

⍝ —
×/1ₓ+⍳4ₓ   ⍝ 24ₓ

⍝ —
[1 2]ₓ+.×[3 4]ₓ   ⍝ 11ₓ

⍝ —
⌊3r2   ⍝ 1ₓ

⍝ —
⌈3r2   ⍝ 2ₓ

⍝ —
|¯3ₓ   ⍝ 3ₓ

⍝ —
3ₓ⌊4ₓ   ⍝ 3ₓ

⍝ —
3ₓ⌈4ₓ   ⍝ 4ₓ

⍝ —
¯3ₓ|5ₓ   ⍝ ¯1ₓ

⍝ —
6ₓ∨4ₓ   ⍝ 2ₓ

⍝ —
6ₓ∧4ₓ   ⍝ 12ₓ

⍝ —
!20ₓ   ⍝ 2432902008176640000ₓ

⍝ —
5ₓ!10ₓ   ⍝ 252ₓ

⍝ —
2ₓ*10ₓ   ⍝ 1024ₓ

⍝ —
2ₓ*¯1ₓ   ⍝ 1r2

⍝ —
2ₓ⊥[1 0 1]ₓ   ⍝ 5ₓ

⍝ —
≢[[1 2]ₓ;1r3 'a']   ⍝ 2ₓ

⍝ —
≢0⍴1ₓ   ⍝ 0ₓ

⍝ — Subtraction below i64 minimum promotes without losing precision
¯9223372036854775808ₓ-1ₓ   ⍝ ¯9223372036854775809ₓ

⍝ — Multiplication above i64 maximum promotes without losing precision
9223372036854775807ₓ×2ₓ   ⍝ 18446744073709551614ₓ

⍝ — In an array, a product above i64 maximum promotes without losing precision, beside products that fit
[9223372036854775807 1 2]ₓ×2ₓ   ⍝ 18446744073709551614ₓ 2ₓ 4ₓ

⍝ — A keyed sum above i64 maximum promotes and keeps its keys
["a":9223372036854775807ₓ "b":1ₓ]+["a":1ₓ "b":2ₓ]   ⍝ ["a":9223372036854775808ₓ "b":3ₓ]

⍝ — A keyed difference below i64 minimum promotes, aligning keys that come in another order
["a":¯9223372036854775808ₓ "b":5ₓ]-["b":2ₓ "a":1ₓ]   ⍝ ["a":¯9223372036854775809ₓ "b":3ₓ]

⍝ — An exact sum remains exact at the i64 boundary
+/[9223372036854775807 1 ¯1]ₓ   ⍝ 9223372036854775807ₓ

⍝ — Ordinary division remains approximate, unlike explicit exact arithmetic
1÷3 + 1÷6   ⍝ 0.5

⍝ —
1ₓ+0.5   ⍝ 1.5

⍝ — Tally counts outer items regardless of nested numeric domains
≢[[1 2]ₓ;3 4]   ⍝ 2ₓ

⍝ —
≢"abc"   ⍝ 3ₓ

⍝ —
≢0⍴⊂1ₓ 2   ⍝ 0ₓ

⍝ —
=/⍬   ⍝ $t

⍝ —
≠/⍬   ⍝ $f

⍝ — An empty reduction by ∧ or ∨ on Booleans gives Booleans, as one by = or ≠ does, and so does an empty inner axis in ∨.∧
{(•storage ⍵).kind}¨[∧/[2 0]⍴$t ∨/[2 0]⍴$t ([2 0]⍴$t)∨.∧[0 3]⍴$t]   ⍝ "bool" "bool" "bool"

⍝ — Empty nested roll retains an exact vector prototype
↑¿0⍴⊂[1 2]ₓ   ⍝ [0 0]ₓ

⍝ — Exact arithmetic gives integer storage
v←[¯2 0 3]ₓ
x←[1ₓ 2r2 6r3;⍳3r1;1ₓ+[2 3]ₓ;10ₓ-[2 3]ₓ;[2 3]ₓ×4ₓ;[6 8]ₓ÷2ₓ;-[1 ¯2]ₓ;×[¯2 0 2]ₓ;(v>0)×v]
[x {(•storage ⍵).kind}¨x]
⍝ =>
[[[1 1 2]ₓ [0 1 2]ₓ [3 4]ₓ [8 7]ₓ [8 12]ₓ [3 4]ₓ [¯1 2]ₓ [¯1 0 1]ₓ [0 0 3]ₓ] 9⍴⊂"integer"]

⍝ — Truth values are Booleans, whatever the arguments
x←["ab"∊"b";~0 1;0 1⊼1 1;0 1⊽0 0] ⋄ [x {(•storage ⍵).kind}¨x]   ⍝ [[[$f $t] [$t $f] [$t $f] [$t $f]];4⍴⊂"bool"]

⍝ — Structural functions keep integer storage
x←[⌽⍳3ₓ;1⌽⍳3ₓ;5↑⍳3ₓ;1↓⍳3ₓ;[[2 0]]⌷⍳3ₓ;1 0 1#⍳3ₓ;1 0 1#⁻¹[1 2]ₓ;+\⍳3ₓ;-¨⍳3ₓ;[1 2]ₓ∪[2 3]ₓ;[1 2]ₓ∩[2]ₓ;
  2ₓ~[1]ₓ;,⊃[[1 2]ₓ 3ₓ];,⍉2 2⍴⍳4ₓ;,(1ₓ+⍳2ₓ)×⊗1ₓ+⍳2ₓ;0ₓ@1⍳3ₓ;{+/,⍵}⌺3⍳3ₓ;[2 2]ₓ⊤3ₓ;⍴2 3⍴1ₓ;⍳0ₓ;⍴1ₓ;↑0⍴⊂1r2 1r3]
[x {(•storage ⍵).kind}¨x]
⍝ =>
[[[2 1 0]ₓ;[1 2 0]ₓ;[0 1 2 0 0]ₓ;[1 2]ₓ;[2 0]ₓ;[0 2]ₓ;[1 0 2]ₓ;[0 1 3]ₓ;[0 ¯1 ¯2]ₓ;[1 2 3]ₓ;[2]ₓ;
  [2]ₓ;[1 2 3 0]ₓ;[0 2 1 3]ₓ;[1 2 2 4]ₓ;[0 0 2]ₓ;[1 3 3]ₓ;[1 1]ₓ;[2 3]ₓ;⍬ₓ;⍬ₓ;[0 0]ₓ];22⍴⊂"integer"]

⍝ — Overflow past the i64 range gives a big exact integer
[9223372036854775807ₓ+1ₓ ¯9223372036854775808ₓ÷¯1ₓ -¯9223372036854775808ₓ |¯9223372036854775808ₓ 2ₓ*63ₓ]
5⍴9223372036854775808ₓ

⍝ — A result that fits i64 again returns to integer storage
x←[(9223372036854775807ₓ+[1 0]ₓ)-1ₓ 9223372036854775808r1-[1 2]ₓ]
[x {(•storage ⍵).kind}¨x]
⍝ =>
[2⍴⊂[9223372036854775807 9223372036854775806]ₓ;"integer" "integer"]

⍝ — Roll gives each result the exactness of its bound. Iota gives exact results only for exact arguments
{(•storage ⍵).kind}¨[¿0ₓ;¿3ₓ 3;↑↑⍳[2 3]ₓ;↑↑⍳2 3]   ⍝ "float64" "mixed" "integer" "float64"

⍝ — An exact number beside an approximate one gives an approximate result
[0∨3ₓ 3ₓ∨0 1ₓ×3 3ₓ+0]   ⍝ 3 3 3 3

⍝⍝ Matrix division

⍝ —
⌹2ₓ   ⍝ 1r2

⍝ — Exact overdetermined solve recovers the line y=1+2ₓ
[3 5 7]ₓ⌹[1 1⋄1 2⋄1 3]ₓ   ⍝ [1 2]ₓ

⍝ — A vector pseudoinverse divides by the squared norm
⌹[1 2]ₓ   ⍝ 1r5 2r5

⍝ — Exact matrix inversion handles a zero leading pivot
⌹[0 2⋄1 0]ₓ   ⍝ [0ₓ 1ₓ⋄1r2 0ₓ]

⍝ — Triangular float inversion preserves integral path counts
⌹[1 ¯1 0⋄0 1 ¯1⋄0 0 1]   ⍝ [1 1 1⋄0 1 1⋄0 0 1]

⍝ — Square complex solve with row pivoting
2j2 4⌹[0 1j1⋄2 0]   ⍝ 2 2

⍝ — Inverting a zero-column matrix exchanges its dimensions
⌹3 0⍴0ₓ   ⍝ 0 3⍴0ₓ

⍝ — Solving for no unknowns preserves the number of right-hand sides
(3 2⍴1)⌹3 0⍴0   ⍝ 0 2⍴0

⍝ — The pseudoinverse of zero is zero
⌹0   ⍝ 0

⍝ — A singular matrix gives its pseudoinverse, the solution with the least norm
⌹2 2⍴1
⍝ =>
[0.25 0.25⋄0.25 0.25]

⍝ — The exact solver gives the pseudoinverse of a singular matrix in rationals
⌹2 2⍴1ₓ
⍝ =>
[1r4 1r4⋄1r4 1r4]

⍝ — A wide exact system gives its solution with the least norm
[1 2]ₓ⌹[1 0 1⋄0 1 0]ₓ
⍝ =>
[1r2 2 1r2]ₓ

⍝ — A rank-deficient tall exact matrix gives its least-squares solution with the least norm
[3 6 9]ₓ⌹[1 2⋄2 4⋄3 6]ₓ
⍝ =>
[3r5 6r5]

⍝ — A rank-deficient tall float matrix gives its pseudoinverse, through the QR factorization that the SVD of a tall matrix starts from [atol=1e-18]
u←1+⍳64 ⋄ v←1+⍳16 ⋄ a←u×⊗v
⌹a
⍝ =>
(⍉(1+⍳64)×⊗1+⍳16)÷(+/(1+⍳64)*2)×+/(1+⍳16)*2

⍝ — A full-rank tall float matrix's pseudoinverse is a left inverse [atol=1e-12]
a←1○(1+⍳64)×⊗1+⍳16
(⌹a)+.×a
⍝ =>
(=⊗⍨⍳16)×1.0

⍝ — Monadic -.× is the determinant, exact for exact numbers, and +.× the permanent
[-.×[1 2 3⋄4 5 6⋄7 8 10]ₓ +.×[1 2⋄3 4]ₓ -.×[1j1 2⋄3 4]]
⍝ =>
[¯3ₓ 10ₓ ¯2j4]

⍝ — Monadic f.g follows J's expansion on vectors, numbers, empty and tall matrices
[-.×[1 2 3];-.×5;-.×0 0⍴0;-.×[1 2⋄3 4⋄5 6]]
⍝ =>
[2 5 1 0]

⍝ — Monadic f.g needs a matrix
-.×2 2 2⍴⍳8ₓ
⍝ error: RANK ERROR

⍝ — Monadic f.g other than -.× expands at most 64 rows
+.×65 65⍴1
⍝ error: LIMIT ERROR

⍝ — Monadic f.g needs g to have an identity, as in J
{⍺-⍵}.{⍺×⍵}[1 2⋄3 4]
⍝ error: DOMAIN ERROR

⍝ — •decompose's "lu" factors rebuild the matrix, and its options leave out vectors or complete QR
f←"lu" •decompose [1 2⋄3 4]
["lu" •decompose⁻¹ f;f.p;⍴(["kind":"qr" "complete":$t]•decompose [1 2⋄3 4⋄5 6]).r;≢(["kind":"eigen" "vectors":$f]•decompose [2 1⋄1 2]).values]
⍝ =>
[[1 2⋄3 4] [1 0]ₓ [3 2]ₓ 2ₓ]

⍝ — Only svd and eigen can leave out their vectors
["kind":"qr" "vectors":$f]•decompose [1 2⋄3 4]
⍝ error: DOMAIN ERROR

⍝ — Only qr has complete factors
["kind":"svd" "complete":$t]•decompose [1 2⋄3 4]
⍝ error: DOMAIN ERROR

⍝ — lu's p must hold each position once
"lu" •decompose⁻¹ ["l":[1 0⋄0 1] "u":[1 0⋄0 1] "p":[0 0]]
⍝ error: DOMAIN ERROR

⍝ — $m is the machine epsilon of the session's float width, or of a width mark
[$m=2*¯52 $mₛ=2*¯23 $mₕ=2*¯10]
⍝ =>
$t $t $t

⍝ —
⌹"ab"
⍝ error: DOMAIN ERROR

⍝ — A wide matrix gives its pseudoinverse
⌹2 3⍴1
⍝ =>
3 2⍴÷6

⍝ —
1 2⌹3 2⍴1
⍝ error: LENGTH ERROR

⍝ — Inverse applies to each matrix of a higher-rank argument
⌹1 1 1⍴1   ⍝ 1 1 1⍴1

⍝⍝ Under with positions, and stencil

⍝ — At applies reverse to the selected subarray, not each item separately
⌽@1 3⍳5   ⍝ 0 3 2 1 4

⍝ — A keyed value aligns by key, as it does in assignment
x←["a":1 "b":2 "c":3] ⋄ ["b":20 "a":10]@0 1 x   ⍝ ["a":10 "b":20 "c":3]

⍝ —
10×@1 3⍳5   ⍝ 0 10 2 30 4

⍝ — `[p]` picks out the positions where `p` of the argument is true
0ₓ@[1=2|] ⍳5ₓ   ⍝ [0 0 2 0 4]ₓ

⍝ — With `[p]` or positions, a left argument lines up with the right argument by position
10 20 30 +@[0>] ¯1 2 ¯3   ⍝ 9 2 27

⍝ —
10 20 30 +@[1 2] 1 2 3   ⍝ 1 22 33

⍝ —
10 20 +@[1 2] 1 2 3
⍝ error: LENGTH ERROR

⍝ — A keyed left argument lines up with the right argument by key
k←["a":1 "b":0] ⋄ ["b":10 "a":20] +@[k⍨] ["a":1 "b":2]   ⍝ ["a":21 "b":2]

⍝ — A key that the left argument lacks reads the fill, and a key that only it has is ignored
k←["a":1 "b":1] ⋄ ["a":20] +@[k⍨] ["a":1 "b":2]   ⍝ ["a":21 "b":2]

⍝ —
["b":10 "a":20 "c":30] +@[0 1] ["a":1 "b":2]   ⍝ ["a":21 "b":12]

⍝ — `[p]` lines up with the argument by axis names
m←["i":2 "j":3]⍴⍳6ₓ ⋄ -@[{4<+⌿⍵}] m
(["i":2 "j":3]⍴0 ¯1 ¯2 3 ¯4 ¯5)ₓ

⍝ — A mask from `[p]` must have the argument's length
-@[{1 0}] 1 2 3
⍝ error: LENGTH ERROR

⍝ — A mask's values pick one of two cases, so a 2 is outside them
(10×⊢)@[⊢] 0 2 1
⍝ error: INDEX ERROR

⍝ — An empty selection from `[p]` doesn't run the function
{1 2 3+⍵}@[0<] ¯1 ¯2   ⍝ ¯1 ¯2

⍝ — A cell whose key the mask lacks counts as NaN, and keeps its value
k←["a":1] ⋄ -@[k⍨] ["b":4 "a":5]   ⍝ ["b":4 "a":¯5]

⍝ — A NaN in the mask picks no case, so its cell keeps its value
m←1 $n 0 ⋄ -@[m⍨] 4 5 6   ⍝ ¯4 5 6

⍝ — A key that only the mask has is ignored
k←["a":1 "c":0] ⋄ -@[k⍨] ["b":4 "a":5]   ⍝ ["b":4 "a":¯5]

⍝ — A unit mask of NaN leaves the argument as it is
-@[{$n}] 1 2 3   ⍝ 1 2 3

⍝ — `@` writes as assignment does, so an exact value written among floats stays exact
⍕0ₓ@[0>] ¯4 9   ⍝ "0ₓ 9"

⍝ — Repeated replacement indices use the last supplied value
10 20@1 1⍳3   ⍝ 0 20 2

⍝ — A matrix of indices gives the replacement function the same frame as ⌷
v←10 20 30 40
i←[0 3⋄1 2]
{⍵+[100 200⋄300 400]}@i v
⍝ =>
110 320 430 240

⍝ — Major-cell selection retains trailing axes after the index array's frame
{⍵+2 2 1⍴100 200 300 400}@[0 3⋄1 2] 1+4 2⍴⍳8ₓ
[101 102⋄303 304⋄405 406⋄207 208]

⍝ — An empty multidimensional index array leaves the argument unchanged
0@(0 2⍴0) ⍳4ₓ   ⍝ [0 1 2 3]ₓ

⍝ — Index arrays select along any axis, beside an atom or another index array, and a negative position counts back from the end. Evenly spaced positions, rising or falling, give the same items as any other
m←5 5⍴⍳25ₓ ⋄ [[2ₓ [0 1 4]ₓ]⌷m ; [[¯1 0]ₓ 1ₓ]⌷m ; [[1 3]ₓ [¯1 ¯2]ₓ]⌷m ; [⍳5ₓ]⌷⍳5ₓ ; [⌽⍳5ₓ]⌷10ₓ×⍳5ₓ ; m.[[4 0]ₓ [0 4]ₓ]]   ⍝ [[10 11 14]ₓ [21 1]ₓ [9 8⋄19 18]ₓ [0 1 2 3 4]ₓ [40 30 20 10 0]ₓ [20 4]ₓ]

⍝ — Writes through exact index arrays count a negative position back from the end, let a later write win, spread a single value, and combine in turn under modified assignment
x←⍳6 ⋄ y←⍳4 ⋄ z←⍳4 ⋄ x.[[1 ¯1 1 0]ₓ]←10 20 30 40 ⋄ y.[[3 ¯4]ₓ]←9 ⋄ z.[[2 2 ¯1]ₓ]+←1 ⋄ [x;y;z]   ⍝ [40 30 2 3 4 20;9 1 2 9;0 1 4 4]

⍝ — Functional update leaves the original array unchanged
a←⍳3ₓ
b←0@2 a
a   ⍝ [0 1 2]ₓ

⍝ — Stencil calls its operand with one argument, the window
(+⌺3 [1 2 3 4])≡[¯3]↕[1 2 3 4]   ⍝ $t

⍝ — A stencil takes an options record, and its windows read past the ends by the record's edge mode
{+/⍵}⌺["size":3 "edge":"wrap"] 1 2 3 4   ⍝ 7 6 9 8

⍝ — A two-row stencil specification gives window size then movement
{+/,⍵}⌺[[3]⋄[2]]⍳8   ⍝ 1 6 12 18

⍝ — A single stencil size varies the leading axis and retains whole rows
{⍴⍵}⌺3 [2 3]⍴⍳6ₓ   ⍝ 2 2⍴3ₓ

⍝ —
{⍵}⌺3⍳1
⍝ error: DOMAIN ERROR

⍝ — An argument that's empty along a windowed axis gives an empty result
⍴{+/,⍵}⌺1 3⊢0 5⍴0   ⍝ [0 5]ₓ

⍝ — An even size over an empty axis gives no windows
⍴{+/,⍵}⌺4⊢⍬   ⍝ [0]ₓ

⍝ — Each result's shape comes from one call on a window of fill
⍴{2⍴+/,⍵}⌺3⊢⍬   ⍝ [0 2]ₓ

⍝ — Padded windows over an empty axis give an empty frame
⍴¯3↕⍬   ⍝ [0 3]ₓ

⍝ —
{⍵}⌺0⍳3
⍝ error: DOMAIN ERROR

⍝ —
{⍵}⌺⍬⍳3ₓ
⍝ error: DOMAIN ERROR

⍝ —
{⍵}⌺1 1⍳3
⍝ error: RANK ERROR

⍝ — An array of integers gives positions
0@[2 0 1]⍳3   ⍝ 0 0 0

⍝ — Nested index paths select fields inside matrix items
G←2 3⍴[["ABC" 1] ["DEF" 2] ["GHI" 3] ["JKL" 4] ["MNO" 5] ["PQR" 6]]
(⊂[[[0 1] 0] [[1 2] 1]])⌷G   ⍝ "DEF" 6

⍝ — At replaces nested fields without replacing their containing items
G←2 3⍴[["ABC" 1] ["DEF" 2] ["GHI" 3] ["JKL" 4] ["MNO" 5] ["PQR" 6]]
H←("" '*'@[[[0 1] 0] [[1 2] 1]])G
[0⊃0 1⊃H;1⊃1 2⊃H]   ⍝ "" '*'

⍝ — At follows each path as Pick does: paths can differ in length, and one that goes on into an atom stops at it
{⍵×10}@[[0 1] [1 0]] [[1 2] 3]   ⍝ [[1 20] 30]

⍝ — At's paths all end at items, or all at cells of one shape
{⍵×10}@[[0 1] [1]] [1 2 3⋄4 5 6]
⍝ error: LENGTH ERROR

⍝ — An empty selection leaves the argument as it is, without calling the function
{⎕←99 ⋄ ⍵}@⍬⍳3ₓ   ⍝ [0 1 2]ₓ

⍝⍝ Encode decode

⍝ —
60⊥3 13   ⍝ 193

⍝ —
2ₓ⊥[1 0 1 0]ₓ   ⍝ 10ₓ

⍝ — A leading zero radix retains the remaining quotient
[0 10]ₓ⊤125ₓ   ⍝ [12 5]ₓ

⍝ — Empty digits decode to zero
2⊥⍬   ⍝ 0

⍝ — An empty radix vector leaves no terms to decode
⍬⊥1   ⍝ 0

⍝ — A zero radix consumes the remainder, leaving zero for earlier digits
0 0 2⊤3   ⍝ 0 1 1

⍝ — Encode puts the digit axis after the value frame, including a zero-length digit axis
⍴(0⍴0)⊤2 2⍴1   ⍝ [2 2 0]ₓ

⍝ — Encode gives each value's digits along the last axis, for negative bases and values too
[¯10 10]ₓ⊤[123 ¯45 7]ₓ   ⍝ [¯8 3⋄¯5 5⋄0 7]ₓ

⍝ — A quotient beyond the integer range still gives exact digits
[7 ¯1]ₓ⊤[¯9223372036854775808 5]ₓ   ⍝ [1 0⋄2 0]ₓ

⍝ — Approximate bases give approximate digits
x←10 10⊤[12 34]ₓ ⋄ [x (•storage x).kind]   ⍝ [[1 2⋄3 4];"float64"]

⍝ — Decode the same digits in binary and decimal
[[2]⋄[10]]⊥1 0 1   ⍝ 5 101

⍝ —
2 3⊥1 2 3
⍝ error: LENGTH ERROR

⍝⍝ Pick and partition

⍝ — Pick descends through a matrix coordinate, a nested field, then a character index
1⊃0⊃1 0⊃2 3⍴[["ABC" 1] ["DEF" 2] ["GHI" 3] ["JKL" 4] ["MNO" 5] ["PQR" 6]]
'K'

⍝ — Empty coordinates can repeatedly pick an atom without changing it
(⍬↣⊃)⁵ 10   ⍝ 10

⍝ — An empty pick path returns the whole argument
⍬⊃1 2   ⍝ 1 2

⍝ — Negative Pick positions count from the end
¯1⊃10 20 30   ⍝ 30

⍝ — Each Pick coordinate counts from the end when negative
¯1 0⊃[1 2⋄3 4]   ⍝ 3

⍝ — On an empty axis, Pick gives the fill for ¯1, as it does for 0
¯1⊃⍬   ⍝ 0

⍝ —
1⌷3 4⍴⍳12ₓ   ⍝ [4 5 6 7]ₓ

⍝ — Squad with an axis selects a column rather than a row
1⌷⍠1 (3 4⍴⍳12ₓ)   ⍝ [1 5 9]ₓ

⍝ — With an axis, a selector of coordinates lists every axis it covers, in any order, and its result takes the place of the first of them
a←2 3 4⍴⍳24ₓ ⋄ [(⊂[[0 1] [1 0]])⌷⍠2 1 ⊢a;(⊂[[0 1] [1 2]])⌷⍠0 2 ⊢a]   ⍝ [[4 1⋄16 13]ₓ;[1 5 9⋄14 18 22]ₓ]

⍝ — With an axis, a selector of coordinates that lists only its first axis has too few axes
(⊂[[0 1] [1 0]])⌷⍠1 ⊢2 3 4⍴⍳24ₓ
⍝ error: LENGTH ERROR

⍝ — Bracket indexing pairs arrays of positions at any rank
a←2 3 4⍴⍳24ₓ ⋄ a.[[0 1] [1 2]]   ⍝ [4 5 6 7⋄20 21 22 23]ₓ

⍝ — Paired positions count from the end when negative
[1 2⋄3 4].[[¯1 0] [¯1 ¯1]]   ⍝ 4 2

⍝ — Paired axes take the place of the first array, and : keeps its axis in place
a←2 3 4⍴⍳24ₓ ⋄ ⍴a.[[0 1] : [1 2]]   ⍝ [2 3]ₓ

⍝ — Paired arrays broadcast against each other
m←3 4⍴⍳12ₓ ⋄ m≡m.[[3 1]⍴0 1 2;[1 4]⍴⍳4ₓ]   ⍝ $t

⍝ — Paired arrays of different lengths don't agree
m←3 4⍴⍳12ₓ ⋄ m.[[0 1] [0 1 2]]
⍝ error: LENGTH ERROR

⍝ — A selector whose items are coordinates reads the item at each
(⊂[[0 1] [1 0]])⌷[1 2⋄3 4]   ⍝ 2 3

⍝ — Coordinates count from the end when negative
(⊂[[¯1 ¯1] [0 ¯1]])⌷[1 2⋄3 4]   ⍝ 4 2

⍝ — A selector of short coordinates covers their axes, and squad crosses it with the next selector
a←2 3 4⍴⍳24ₓ ⋄ [[[0 1] [1 2]] [3 0]]⌷a   ⍝ [7 4⋄23 20]ₓ

⍝ — Bracket indexing pairs a selector of coordinates with the next array
a←2 3 4⍴⍳24ₓ ⋄ a.[[[0 1] [1 2]] [3 0]]   ⍝ [7 20]ₓ

⍝ — Coordinates longer than the rank go on into the item they reach, as paths of Pick
(10 20⋄30 40 50).[[[1 2] [0 0]]]   ⍝ 50 10

⍝ — Selective assignment writes at each coordinate
m←3 4⍴⍳12ₓ ⋄ m.[⍸m>9]←0ₓ ⋄ m   ⍝ [0 1 2 3⋄4 5 6 7⋄8 9 0 0]ₓ

⍝ — Empty indices on both axes retain a rank-two empty result
⍴⍬ ⍬⌷3 4⍴0   ⍝ [0 0]ₓ

⍝ —
⍬⌷1   ⍝ 1

⍝ —
⌷1 2   ⍝ 1 2

⍝ — : takes a whole axis
[: 2]⌷3 4⍴⍳12ₓ   ⍝ [2 6 10]ₓ

⍝ — Reverse has its own primitive
⌽⍠0⊢[1 2⋄3 4]   ⍝ [3 4⋄1 2]

⍝ —
⌽[1 2⋄3 4]   ⍝ [2 1⋄4 3]

⍝ — Reverse keeps keys with their items
⌽"aa" "bb":1 2   ⍝ "bb" "aa":2 1

⍝ new-axis-shapes — ¯∞ inserts without consuming an axis, including on units
m←2 3⍴⍳6ₓ ⋄ ∧/[[1 2 3]≡⍴[¯∞ : :]⌷m;[2 1 3]≡⍴[: ¯∞ :]⌷m;[2 3 1]≡⍴m.[∞ ¯∞];[1]≡⍴¯∞⌷7;[1]≡⍴¯∞⌷⍬⍴7]   ⍝ $t

⍝ rest-axis-selection — Rest can span several axes or none between explicit selectors
a←2 3 4⍴⍳24ₓ ⋄ [a.[1 ∞ 2];⍴a.[¯∞ ∞ 0];a.[0 ∞ ¯∞ 1 2]]   ⍝ [[14 18 22]ₓ;[1 2 3]ₓ;[6]ₓ]

⍝ new-axis-selection — Paired indices, listed axes and Under use the same inserted axes
m←2 3⍴⍳6ₓ ⋄ [⍴m.[[0 1] ¯∞ [1 2]];⍴([¯∞ :]⌷⍠1) m;(-@([: ¯∞ 1]⌷))m]   ⍝ [[2 1]ₓ [2 1 3]ₓ [0 ¯1 2⋄3 ¯4 5]ₓ]

⍝ new-axis-paths — A path emits its index frame before suffixes traverse the ending cell
x←(10 20⋄30 40 50) ⋄ p←[[1 2] [0 0]] ⋄ c←1 1⍴⊂[10 20⋄30 40] ⋄ q←[[0 0 1]] ⋄ [⍴[¯∞ p]⌷x;⍴[p ¯∞]⌷x;[q ∞ ¯∞]⌷c]   ⍝ [[1 2]ₓ;[2 1]ₓ;1 2 1⍴30 40]

⍝ empty-path-cell-frame — Empty paths retain the nested ending cell from their prototype
x←[(2 3⍴⍳6ₓ)] ⋄ p←0↑[[0 0]] ⋄ ⍴[p ∞ ¯∞]⌷x   ⍝ [0 3 1]ₓ

⍝ named-coordinate-span — Even empty coordinates must fit after their named source axis
p←["b":0]⍴⊂0 0 0 ⋄ [p]⌷["a":2 "b":2 "c":2]⍴⍳8ₓ
⍝ error: INDEX ERROR

⍝ — Negative positions count from the end
¯1⌷10 20 30   ⍝ 30

⍝ —
¯2 ¯1⌷3 4⍴⍳12ₓ   ⍝ 7ₓ

⍝ — An array of positions can mix signs
[[¯1 0]]⌷10 20 30   ⍝ 30 10

⍝ — Selective assignment through Reverse writes in reverse order
x←[1 2⋄3 4]
⌽⍠0⊢x ←[5 6⋄7 8]
x   ⍝ [7 8⋄5 6]

⍝ new-axis-writeback — An inserted axis changes the selection frame, not its targets
m←2 3⍴⍳6ₓ ⋄ m.[¯∞ : 1]←1 2⍴[10 20]ₓ ⋄ T←["a":1] ⋄ [¯∞ "b"]⌷T ←2 ⋄ [m;T]   ⍝ [[0 10 2⋄3 20 5]ₓ;["a":1 "b":2]]

⍝ — Selective assignment through a negative position
x←10 20 30
x₋₁←9
x   ⍝ 10 20 9

⍝ —
¯4⌷10 20 30
⍝ error: INDEX ERROR

⍝ — ∞ is valid only as a whole item, not inside an array of positions
[[1 ∞]]⌷10 20 30
⍝ error: DOMAIN ERROR

⍝ —
⊆1 2   ⍝ ⊂1 2

⍝ —
⊆1   ⍝ ⊂1

⍝ — Nest leaves an already nested vector unchanged
⊆[1 2;3 4]   ⍝ [1 2;3 4]

⍝ — Partition starts on a positive rise, not every label change; zero omits an item
3 2 2 1 0 1⊆"abcdef"   ⍝ "abcd" "f"

⍝ — Axis-zero partition cuts blocks of rows and omits the zero-marked row
1 1 0 1⊆⍠0 (1+4 2⍴⍳8ₓ)   ⍝ [[1 2⋄3 4] [7 8⋄]]

⍝ — No partition starts: retain a prototype with an empty selected axis
0⊂2 3⍴0   ⍝ 0⍴⊂(0 3⍴0)

⍝ — Partition counts remain visible even when the other axes are empty
1 0 1⊆⍠1 (0 3⍴0)   ⍝ [(0 1⍴0) (0 1⍴0)]

⍝ —
2⊃1 2
⍝ error: INDEX ERROR

⍝ —
1 1⊃1 2
⍝ error: INDEX ERROR

⍝ —
1⊆3
⍝ error: RANK ERROR

⍝ —
¯1⊂1 2
⍝ error: DOMAIN ERROR

⍝ —
1 0⊆1 2 3
⍝ error: LENGTH ERROR

⍝ —
1 2⌷1 2
⍝ error: INDEX ERROR

⍝ —
2⌷1 2
⍝ error: INDEX ERROR

⍝ — A one-element partition marker extends over the whole vector
(,1)⊂"abcd"   ⍝ ,⊂"abcd"

⍝ — A unit partition label groups the whole vector
1⊆"abcd"   ⍝ ,⊂"abcd"

⍝ — A singleton label vector also extends over the whole argument
(,1)⊆"abcd"   ⍝ ,⊂"abcd"

⍝⍝ Grading

⍝ — Descending grade preserves the original order of ties
⍒2 1 2 1   ⍝ [0 2 1 3]ₓ

⍝ — Grade ignores comparison tolerance
⍋[1 (1+8E¯15) 1]   ⍝ [0 2 1]ₓ

⍝ — Fast combinations give what their parts give in turn: partial grades and sorts, ranks, and counts and folds under Key
v←¿1000⍴0 ⋄ k←¿1000⍴7 ⋄ b←0=¿1000⍴3
[((10↑)∘⍋ v)≡({10↑⍋⍵} v); (¯10↑< v)≡({¯10↑<⍵} v); (↑> v)≡({↑>⍵} v); ((3⌷)∘⍒ v)≡({3⌷⍒⍵} v); (⍋⍋ v)≡({⍋⍵}⍋v); (⍋⍒ v)≡({⍋⍵}⍒v); ((≢⊢)⌸ k)≡({≢⍵}⌸ k); (k (+/⊢)⌸ v)≡(k {+/⍵}⌸ v); (k (⌈⌿⊢)⌸ v)≡(k {⌈⌿⍵}⌸ v); (k (⌊/⊢)⌸ b)≡(k {⌊/⍵}⌸ b); (k (+/⊢)⌸ k)≡(k {+/⍵}⌸ k)]
⍝ =>
$t $t $t $t $t $t $t $t $t $t $t

⍝ — Numbers precede characters; complex numbers sort by real then imaginary part
⍋1j2 1 1j¯2 'a' 0   ⍝ [4 2 1 0 3]ₓ

⍝ — Mixed-domain grade does not round large exact integers through float
⍋9007199254740993ₓ 9007199254740992 9007199254740992ₓ   ⍝ [1 2 0]ₓ

⍝ — Equal-rank nested arrays compare ravelled contents before shape
⍋[[1 2⋄3 4] [1 2 0 0⋄]]   ⍝ [1 0]ₓ

⍝ — Monadic `<` sorts major cells up, keeping ties in their original order
<3 1 2 1   ⍝ 1 1 2 3

⍝ — Sorting a matrix reorders its rows
<[3 1⋄1 2⋄2 0]   ⍝ [1 2⋄2 0⋄3 1]

⍝ — Monadic `>` sorts down
>"b" "a" "c"   ⍝ "c" "b" "a"

⍝ — Sorting a record moves each key with its value
<["x":3 "y":1 "z":2]   ⍝ ["y":1 "z":2 "x":3]

⍝ — A unit has one item, so it is already sorted
<5   ⍝ 5

⍝ — Nested rank takes precedence over contents
⍋[[1 2⋄] [1 2]]   ⍝ [1 0]ₓ

⍝ — Empty nested arrays sort by rank, then shape
⍋[0 5 2⍴0;0 3 4⍴0;0 1⍴"";⍬]   ⍝ [3 2 1 0]ₓ

⍝ — Empty-array prototypes do not break sorting ties
⍋[0⍴⊂1 2;0⍴0;0⍴""]   ⍝ [0 1 2]ₓ

⍝ — Structural order puts numbers before characters before nested arrays
⍋['z' [0 0] 100 'a']   ⍝ [2 3 0 1]ₓ

⍝ — Character vectors sort lexicographically, not by length first
⍋"ba" "aaa" "ab"   ⍝ [1 2 0]ₓ

⍝ — Equal numbers retain their order across numeric representations
⍋1ₓ 1 1j0   ⍝ [0 1 2]ₓ

⍝ — Interval index shares grade's ordering across numbers, characters and nested arrays
[1 'a' [1 2]]⍸[0 'b' [2 3]]   ⍝ [0 2 3]ₓ

⍝ — Interval lookup preserves exact distinctions beyond float's integer range
[9007199254740992 9007199254740993]ₓ⍸9007199254740992 9007199254740994
[1 2]ₓ

⍝ — Complex interval lookup uses real-then-imaginary ordering
1j¯1 1 1j1⍸1j¯2 1j0 1j2   ⍝ [0 2 3]ₓ

⍝ — Zero-width rows are stable ties, not an empty collection of rows
⍋3 0⍴0   ⍝ [0 1 2]ₓ

⍝ —
⍋⍬   ⍝ ⍬ₓ

⍝ — Custom collation puts unlisted characters last, retaining their order
"cba"⍋"azb?c"   ⍝ [4 2 0 1 3]ₓ

⍝ — A multidimensional collation array supplies successive collation keys
["AB"⋄"BA"]⍋["BA"⋄"AB"⋄"BA"]   ⍝ [0 1 2]ₓ

⍝ —
⍋1
⍝ error: RANK ERROR

⍝ —
'a'⍋"ab"
⍝ error: RANK ERROR

⍝ —
"ab"⍒'a'
⍝ error: RANK ERROR

⍝ —
1 2⍋"ab"
⍝ error: DOMAIN ERROR

⍝ —
"ab"⍋1 2
⍝ error: DOMAIN ERROR

⍝ —
""⍋⍬
⍝ error: DOMAIN ERROR

⍝⍝ Float storage and kernels

⍝ —
1÷0 1   ⍝ ∞ 1

⍝ —
v←⍳1000ₓ ⋄ +/v+v   ⍝ 999000ₓ

⍝ —
×/1 2 3 4   ⍝ 24

⍝ — Generic reduction stays right-associated despite floating-point cancellation
{⍺+⍵}/1E100 ¯1E100 1   ⍝ 0

⍝ —
+/[1 2 3]ₓ   ⍝ 6ₓ

⍝ — Zero divided by zero is NaN, applied elementwise
0 1÷0 2   ⍝ [$n 0.5]

⍝ — Structure keeps float storage, and an empty array takes its prototype's storage
{(•storage ⍵).kind}¨[⌽⍳3;2 3⍴⍳6;0 3⍴0;0⍴1ₓ]   ⍝ "float64" "float64" "float64" "integer"

⍝ — Fused calls on a large array, across runs and parentheses, where one call joins two fused graphs, across assignments, through a shared intermediate, through a long iteration, and through both tines of a fork, give the result that each row gives, and their fold gives the fold of that result. 32-bit floats compare exactly, and their last bit can differ between routes, so `near` compares them within a tolerance
•prefs ["seed":7] ⋄ x←¿1000000⍴0
x.[[0 1 2 3]]←[∞ ¯∞ $n 0]
m←[1000 1000]⍴x
n←⌽m
s←32•float m
y←4↓x
f←{0⌈1⌊0.5+1.2×⍵-0.5}
g←{⍺×1-0.5×⍵}
h←{0ₛ⌈1ₛ⌊0.5ₛ+1.2ₛ×⍵-0.5ₛ}
k←{0⌈ 1⌊ 0.5+ 1.2× ⍵-0.5}
p←{((⍵-0.5)×1.2)⌈0}
q←{(⍵-0.5)×⍵}
j←{(⍵-0.5)×⍵+0.5}
u←{(⍵-0.5)×(⍵+0.5)×⍵×2}
r←{(⍵>0.5)+⍵×2}
w←{(⍺-0.5ₛ)×⍵+0.5}
c←{(⍵×2)<⍵+0.5}
d←{a←⍵-0.5 ⋄ a×a+1}
e←{t←⍵×0.5 ⋄ t×1-t}
v←{b←⍵ ⋄ a←b+1 ⋄ b←0 ⋄ a}
near←{((⍺≠⍺)≡⍵≠⍵)∧0=+/,(|⍺-⍵)>1e¯5×1⌈|⍺}
[(f m)≡ f⍤1 m;(m g n)≡ m g⍤1 n;(0.5<m)≡ {0.5<⍵}⍤1 m;(h s) near h⍤1 s;(k m)≡ k⍤1 m;(p m)≡ p⍤1 m;(q m)≡ q⍤1 m]
[(+/0.5<m)≡{+/⍵}0.5<m (+⌿0.5<m)≡{+⌿⍵}0.5<m (∨/m>0.999)≡{∨/⍵}m>0.999 (+/0⌈1⌊0.5+1.2×m-0.5)≡{+/⍵}0⌈1⌊0.5+1.2×m-0.5 (⌈⌿m-0.5)≡{⌈⌿⍵}m-0.5 (+/0.5ₛ<s)≡{+/⍵}0.5ₛ<s (+/y×2)≡{+/⍵}y×2 ((+⌿s×2) near {+⌿⍵}s×2)]
[(j m)≡ j⍤1 m;(u m)≡ u⍤1 m;(r m)≡ r⍤1 m;(s w m)≡ s w⍤1 m;(+/(m-0.5)×m+0.5)≡{+/⍵}(m-0.5)×m+0.5;(c m)≡ c⍤1 m;(+/(m×2)<m+0.5)≡{+/⍵}(m×2)<m+0.5]
[(d m)≡ d⍤1 m;(e m)≡ e⍤1 m;(v m)≡ v⍤1 m;({⍵+0.5}⍣100⊢m)≡{⍵+0.5}⍣100⍤1⊢m]
[((⊢÷+/) 0.5×y)≡{⍵÷+/⍵}0.5×y;((⊢-+/) 0.5×y)≡{⍵-+/⍵}0.5×y;((⌈/-+/) 0.5×y)≡{(⌈/⍵)-+/⍵}0.5×y;((+/-+/) 0.5×y)≡{(+/⍵)-+/⍵}0.5×y;((+/)≡(+/)) 0.5×y]
⍝ =>
$t $t $t $t $t $t $t
$t $t $t $t $t $t $t $t
$t $t $t $t $t $t $t
$t $t $t $t
$t $t $t $t $t

⍝ — Under dynamic kinds, ⌊ and ⌈ keep an exact number exact beside an infinity, and give their right argument beside NaN, as a float. On a large array they don't fuse beside an exact number or a comparison, because a fused graph reads them as floats
x←¿1000000⍴0 ⋄ x.[[0 1 2 3]]←[∞ ¯∞ $n 0] ⋄ [4↑x⌊0ₓ ⋄ 4↑x⌈0ₓ ⋄ 4↑x⌊x>0 ⋄ 4↑x⌈$t]
[0ₓ ¯∞ 0 0⋄∞ 0ₓ 0 0⋄1ₓ ¯∞ 0 0⋄∞ 1ₓ 1 1]

⍝ max-min-nan — ⌈ and ⌊ give their right argument beside NaN, and every fold, scan and product starts at ¯∞ or ∞ with each item on the left, so a NaN item changes no running result and a NaN seed stays: dyadic in either order, folds and scans short and long, columns, Key, seeded folds, windows, rows, products, exact and mixed numbers, and a long fold split into pieces
nans←{+/,~⍵=⍵} ⋄ r←⌈\(⍳10),$n,⍳53 ⋄ m←64 64⍴0.5+⍳4096 ⋄ m.[3 5]←$n ⋄ q←1000 4⍴0.5+⍳4000 ⋄ q.[7 2]←$n ⋄ p←100 20⍴0.5+⍳2000 ⋄ p.[3 4]←$n ⋄ w←0.5+⍳200 ⋄ w.[100]←$n ⋄ s←0.5+⍳1000000 ⋄ s.[900000]←$n ⋄ s.[900001]←1e9 ⋄ [$n⌈3;3⌈$n;$n⌊3;3⌊$n;⌈/1 $n 3;⌊/1 $n 3;⌈/$n,⍳15;⌈/(⍳15),$n;⌈/$n $n;⌈\$n 1 2;⌊\1 $n 3;nans ⌈\$n,⍳63;10↑r;nans r;⌈⌿[1 $n⋄3 4];nans ⌈⌿m;0 1 0 1 (⌈/⊢)⌸ 1 $n 3 4;0⌈/$n 1 2;0⌈/$n,⍳20;⌈/3↕1 $n 3 4 5;nans ⌈/50↕w;⌈/⍤1 [1 $n 3⋄4 5 6];nans ⌈/⍤1 q;nans ⌈/⍤1 p;nans ⌈/⍤1 m;[1 $n⋄3 4]⌈.+2 2⍴0;nans m⌈.+m;nans m⌊.+m;1ₓ⌈$n;$n⌊1ₓ;⌈/1ₓ $n 2.5;⌈/s;⌊/s;$n⌈/⍳40;nans $n⌈\⍳300]
[3 $n 3 $n 3 1 14 14 ¯∞ [¯∞ 1 2] [1 1 1] 0ₓ [0 1 2 3 4 5 6 7 8 9] 0ₓ [3 4] 0ₓ [3 4] 2 19 [3 4 5] 0ₓ [3 6] 0ₓ 0ₓ 0ₓ [1 1⋄4 4] 0ₓ 0ₓ $n 1 2.5 1000000000 0.5 $n 300ₓ]

⍝ — With •prefs ["fusion":$t], a call that computes a pending argument reports its glyph and position
x←1.5+¿1000000⍴0 ⋄ •prefs ["fusion":$t] ⋄ r←⍴⍋x ⋄ •prefs ["fusion":$f] ⋄ r
[1000000]ₓ
⍝ ⎕: fusion ends at ⍋ (<input>:1:46)

⍝ — A structural glyph on a large array gives a pending copy, which later calls and folds read through, with a fill that no call computes
v←1.0×¿1000000⍴1000 ⋄ mm←1000000 2⍴v ⋄ iv←¿1000000⍴100 ⋄ cv←1000000⍴"abc" ⋄ s←{s←⍵+1 ⋄ (2×⌽s)+s} ⋄ [(2×⌽v+1)≡⌽2×v+1; (2×1⌽v+1)≡1⌽2×v+1; (2×(10↓v)+1)≡10↓2×v+1; (2×⌽(v×2)+1)≡⌽2×(v×2)+1; (s v)≡(2×⌽v+1)+v+1; (2×1⌽⌽v+1)≡1⌽⌽2×v+1; ((⌽v)+mm)≡⌽⍠0 v+⌽⍠0 mm; (¯5↑2×1000005↑v+1)≡5⍴0; (+/⌽v+1)=+/v+1; (+/(⌽v)×v)=+/v×⌽v; (+/,3↕v)=+/(¯2↓v)+(1↓¯1↓v)+2↓v; ((⌽iv)₀=iv₋₁)∧(⌽cv)₀=cv₋₁; ({+/⌽v×⍵}∂1)=+/v]
$t $t $t $t $t $t $t $t $t $t $t $t $t

⍝ — Structural reads of a pending matrix copy compose with it on both axes, and read what reading a stored copy reads
m←0.5+600 1000⍴⍳600000ₓ ⋄ t←2↓m ⋄ [(⊖2↓m)≡⊖t; (400 500↑1 2↓2↓m)≡400 500↑1 2↓t; (3⌽2↓m)≡3⌽t; (¯600↑2↓m)≡¯600↑t; (⌽⊖2↓m)≡⌽⊖t]
$t $t $t $t $t

⍝ — A transpose reads through a large take from an empty array, whose items are all fill
≢+/⍉2 30000 3↑2 0 3⍴0   ⍝ 3ₓ

⍝ — A chain of pending calls that reads a fresh array with each call computes its earlier calls once it would keep alive more than 8 times its largest array, while a chain that reads one named array again and again keeps it once and stays pending
N←131072 ⋄ r←{⍵+¿N⍴0}⍣40⊢N⍴0.0 ⋄ x←¿N⍴0 ⋄ s←{⍵+x}⍣40⊢x ⋄ [((•memory r).bytes≤8×8×N) ((•memory s).bytes=8×N) (•storage s).pending]
$t $t $t

⍝ fusion:comparison-operand — A call reads a pending comparison inside its graph, as 1s and 0s, on either side of the call and in a dfn's recorded statement
v←(⍳262144ₓ)÷262144 ⋄ m←+/(v>0.5)#v ⋄ e←•explain "+/v×v>0.5" ⋄ f←{+/⍵×⍵>0.5} ⋄ [e.value=m; 0=≢e.forced.function; (+/(v>0.5)×v)=m; (f v)=m; (f v)=m]
$t $t $t $t $t

⍝⍝ Numeric storage

⍝ dynamic-integer-fold-storage — Integer reductions compact their array result
r←+/[1 2 3⋄4 5 6]ₓ
(•memory r).bytes=(•memory [6 15]ₓ).bytes   ⍝ $t

⍝ — •storage's kind names the storage that holds an array's items
{(•storage ⍵).kind}¨[⍳3ₓ;⍳3;1j2 3;"abc";1ₓ 0.5;[1 2;3]]
"integer" "float64" "complex128" "character" "mixed" "mixed"

⍝ — •storage gives a record: the storage's kind, whether the items wait in a pending call, whether the value reads another array's items, and how many ∂ levels' tags it carries
•storage 1.5 2.5   ⍝ ["kind":"float64" "pending":$f "view":$f "active":0ₓ]

⍝ — Reading a pending array's shape, tally or binding leaves it pending for later calls to fuse with
y←1.5+¿200000⍴0 ⋄ z←y ⋄ _←[≢y ≢⍴z] ⋄ (•storage y).pending   ⍝ $t

⍝ — Ordinary kernels and fused calls read pending copies at both float widths without forcing them; a dense reader fills their cache
check←{
  v←⍵•float⍳262144
  r←⌽v ⋄ z←⌊r
  reversed←(•storage r).pending∧z≡⌽⌊v
  ordinary←0=≢(•explain "⌊⌽v").fused.glyphs
  z←√r
  roots←(•storage r).pending∧z≡⌽√v
  z←2○r ⋄ deferred←(•storage z).pending
  fused←deferred∧(•storage r).pending∧z≡⌽2○v
  m←512 512⍴v ⋄ r←⊖m ⋄ z←⌊r
  rows←(•storage r).pending∧z≡⊖⌊m
  q←⌽r ⋄ z←⌊q
  composed←(•storage r).pending∧(•storage q).pending∧z≡⌽⊖⌊m
  z←<,r
  [ordinary reversed roots fused rows composed ~(•storage r).pending]
}
check¨32 64   ⍝ [7⍴$t;7⍴$t]

⍝ — Selections of a few items read a pending copy through its chain of copies and leave it pending, even where a rotation's map doesn't compose with the selection's. A write into the copy computes it, and •explain records the write as ←
sel←{
  r←⌽⍳262144ₓ ⋄ q←1⌽r
  parts←[r.[0] ↑r r₁ r.[[0 5 9]] 2↑r ¯2↑r 0⊃r 2↑q ↑q]
  [parts≡[262143;262143;262142;262143 262138 262134;262143 262142;1 0;262143;262142 262141;262142] (•storage r).pending∧(•storage q).pending (•explain "r.[0]←1").forced.function≡["←"] ~(•storage r).pending]
}
sel 0   ⍝ 4⍴$t

⍝ — A read of a few items from a pending graph gives the items that computing it whole gives
v←⍳262144ₓ ⋄ s←1+v ⋄ [5↑1+v;3⊃2×v+1;5↑s]   ⍝ [1 2 3 4 5;8;1 2 3 4 5]

⍝ — A read of a few items from a pending graph that nothing else holds computes only those items, as `•explain` counts them. A pending graph that a name holds is computed whole on its first read [unfused]
v←⍳262144 ⋄ s←1+v ⋄ items←{(•explain ⍵).forced.items} ⋄ [(items "5↑1+v")≡[5]ₓ;(items "3⊃2×v+1")≡[1]ₓ;(items "5↑s")≡[262144]ₓ]   ⍝ $t $t $t

⍝ — Rank leaves a pending argument pending where its operand isn't one that runs on whole stacks of matrices, so the call fuses as it would without Rank
x←[4 256 256]⍴0.5 ⋄ e←•explain "(-⍤2) x+1" ⋄ [0=≢e.forced.function;e.value≡-x+1]   ⍝ $t $t

⍝ — `+.×` on a pending copy of windows, and its gradients, give what they give on a stored copy, and the copy stays pending
conv←{
  img←¿64 64 16⍴0 ⋄ k←¿144 8⍴0
  w←[3844 144]⍴3 3↕img ⋄ wf←[3844 144]⍴(3 3↕img)+0
  same←((w+.×k)≡wf+.×k)∧({+/,w+.×⍵}∂k)≡{+/,wf+.×⍵}∂k
  image←({+/,([3844 144]⍴3 3↕⍵)+.×k}∂img)≡0.5×{+/,(2×[3844 144]⍴3 3↕⍵)+.×k}∂img
  [same image (•storage w).pending]
}
conv 0   ⍝ 3⍴$t

⍝ — `+.×` reads a pending copy of windows a block of rows at a time without forcing it, and its gradients force nothing and scatter nothing [unfused]
conv←{
  img←¿64 64 16⍴0 ⋄ k←¿144 8⍴0
  w←[3844 144]⍴3 3↕img
  e←•explain "{+/,w+.×⍵}∂k" ⋄ ei←•explain "{+/,([3844 144]⍴3 3↕⍵)+.×k}∂img"
  [0=≢e.forced.function 0=≢ei.forced.function 0=≢ei.accumulations.items]
}
conv 0   ⍝ 3⍴$t

⍝ fusion:reshaped-transpose-product — A product reads a reshaped pending transpose in its new matrix dimensions
x←[2 65536]⍴⍳131072ₓ
y←65536⍴1
([2 65536]⍴⍉x)+.×y
⍝ =>
3221192704 5368676352

⍝ — Unselected backing values do not change whole-array numeric policy
v←(262144⍴1E20),∞ ⋄ r←262144↑v
(•storage⌊r).kind   ⍝ "mixed"

⍝ — A reshape reads its argument's items as a view
(•storage 2 3⍴⍳6ₓ).view   ⍝ $t

⍝ — Under ∂, a value carries one tag for each ∂ that differentiates it, and keeps its primal's kind
a←0 ⋄ k←"" ⋄ _←{{a+←(•storage ⍵).active ⋄ k⊢←(•storage ⍵).kind ⋄ ⍵×⍵}∂⍵}∂ 2.5 ⋄ [a k]   ⍝ 2 "float64"

⍝ — An atom gives its own kind, including a rational, a big exact integer and a function
{(•storage ⍵).kind}¨[1ₓ 0.5 1j2 'a' 1r3 9223372036854775808ₓ +]
"integer" "float64" "complex128" "character" "rational" "rational" "function"

⍝ — An empty array has the storage its prototype gives
{(•storage ⍵).kind}¨[⍬;⍬ₓ;"";0⍴⊂1 2]   ⍝ "float64" "integer" "character" "mixed"

⍝ — Joining compact arrays widens exact integers to floats, while writing a float into them leaves the others exact
{(•storage ⍵).kind}¨[0.5,⍳3ₓ;⊃[[1 2]ₓ;0.5 1.5];0.5@1 ⍳3ₓ]   ⍝ "float64" "float64" "mixed"

⍝ — A complex number makes every number complex
{(•storage ⍵).kind}¨[1j2,0.5 1j2,⍳2ₓ]   ⍝ "complex128" "complex128"

⍝ — A rational has no compact form, so its array stays mixed
{(•storage ⍵).kind}¨[0.5,1r3 1j2,1r3]   ⍝ "mixed" "mixed"

⍝ — Writing a float into integer storage leaves the other items exact, in mixed storage
x←⍳3ₓ
x₁←0.5
(•storage x).kind   ⍝ "mixed"

⍝ — Writing an exact integer into float storage keeps it exact, in mixed storage
x←0.5 1.5
x₀←2ₓ
(•storage x).kind   ⍝ "mixed"

⍝ — A write keeps each number's exactness, whether the record came from a literal or from •json
r←["a":2ₓ "b":3ₓ] ⋄ j←•json "{""a"":2,""b"":3}" ⋄ r.b←2.5 ⋄ j.b←2.5 ⋄ [r j]   ⍝ [["a":2ₓ "b":2.5] ["a":2ₓ "b":2.5]]

⍝ — A new key's value keeps its exactness beside the others
r←["a":2ₓ] ⋄ r.c←2.5 ⋄ r   ⍝ ["a":2ₓ "c":2.5]

⍝ — A modified write through a selection follows the same rule in place: +← and ×← leave exact integers exact beside a float, keep one kind where the results share it, and leave a Boolean target mixed beside the number
x←[1 2 3]ₓ ⋄ y←[1 2 3]ₓ ⋄ z←[1 2 3]ₓ ⋄ b←1 0 1=1 ⋄ x.[[0 1]]+←0.5 ⋄ y.[[0]]×←0.5 ⋄ z.[[0]]+←1ₓ ⋄ b.[[0]]+←1 ⋄ [x y z b]   ⍝ [1.5 2.5 3ₓ;0.5 2ₓ 3ₓ;[2 2 3]ₓ;2 $f $t]

⍝ — A write keeps a Boolean apart from other numbers, as a literal does, so either written among the other leaves the array mixed
m←3⍴$f ⋄ m.[[1]]←1 ⋄ p←[1 2 0]ₓ ⋄ p.[[1]]←$t ⋄ [m p]   ⍝ [$f 1 $f;1ₓ $t 0ₓ]

⍝ — A write means the same whether the array is compact or mixed: a copy compacts mixed items that share a kind, a view keeps them mixed, and a later write gives the same on both
t←[2.5 2ₓ] ⋄ t₀←1ₓ ⋄ c←⌽t ⋄ v←,t ⋄ c₀←2.5 ⋄ v₁←2.5 ⋄ [c v]   ⍝ [2.5 1ₓ;1ₓ 2.5]

⍝⍝ Float widths

⍝ — •float converts to 32-bit floats, which keep their width through arithmetic with exact numbers and with each other
v←32 •float 0.1 0.2
{(•storage ⍵).kind}¨[v v+1ₓ v×v *v ⌊v v<0.2]   ⍝ "float32" "float32" "float32" "float32" "integer" "bool"

⍝ — Floats of two widths compute at the wider
v←32 •float 0.1 0.2
{(•storage ⍵).kind}¨[v+0.5ₚ;v,0.5ₚ;64 •float v]   ⍝ "float64" "float64" "float64"

⍝ — An exact operand rounds to the float width before the operation
(32 •float 1.5)×16777217ₓ   ⍝ 25165824

⍝ — Comparison at 32 bits is exact, after an exact operand rounds to the width. A weak literal rounds to the width too, while a strong 64-bit float compares at 64 bits
[(32 •float 0.1)=1r10 (32 •float 0.1)=0.1ₚ (32 •float 0.1)=0.1]   ⍝ $t $f $t

⍝ — A search at 32 bits reads an exact needle at that width
x←32 •float 16777216
(x,x)⍳16777217ₓ   ⍝ 0ₓ

⍝ — Infinite power stops when two 32-bit results lie within 4 units in the last place, and keeps the width
(•storage (2○⍣∞) 32 •float 1).kind   ⍝ "float32"

⍝ — Sines and cosines beyond the reach of the kernels' fast reduction take the exact reduction, even beside smaller arguments
(1○1e22 ¯1e22 0.5),2○1e22   ⍝ ¯0.8522008497671888 0.8522008497671888 0.479425538604203 0.523214785395139

⍝ — A 32-bit sine beyond that reach keeps its width
1○32•float 3e6   ⍝ ¯0.87849003ₛ

⍝ — The sine of ¯0 is ¯0, and the cosine of 0 is exactly 1
[÷1○¯0 0=1-2○0]   ⍝ ¯∞ $t

⍝ — The tangent of an infinity is NaN, a real number, as the sine's and the cosine's are
(3○∞ ¯∞),(1○∞),2○¯∞   ⍝ $n $n $n $n

⍝ — Writing a Boolean or an exact number into 32-bit storage leaves it mixed, and the other items stay 32-bit
x←32 •float 0.5 1.5
y←x
x₀←$t
y₀←2ₓ
{(•storage ⍵).kind}¨[x;y;y₁]   ⍝ "mixed" "mixed" "float32"

⍝ — Writing a 64-bit float into 32-bit storage widens it
x←32 •float 0.5 1.5
x₀←0.25ₚ
(•storage x).kind   ⍝ "float64"

⍝ — •float converts nested arrays too
{(•storage ⍵).kind}¨(32 •float [1ₓ 0.5;⊂1 2])   ⍝ "float32" "mixed"

⍝ — Monadic •float gives the width of its argument's floats, as a number that dyadic •float takes. Exact numbers give the default width, and a nested array its widest float
[•float 32•float 0.5;•float [1j2]ₛ;•float [1 2]ₓ;•float [32•float 0.5;⊂16•float 1 2]]   ⍝ [32 32 64 32]ₓ

⍝ — A constant at an array's width
x←16•float 1 2
(•storage x×(•float x)•float 0.5).kind   ⍝ "float16"

⍝ — Monadic •float takes numbers
•float "a"
⍝ error: DOMAIN ERROR

⍝ — Under ∂, •float converts a tangent with its primal. A gradient keeps its primal's width, whatever width the function computes at
g←{+/16•float ⍵×⍵}∂ 32•float 1 2 3
(g≡32•float 2 4 6)∧"float32"≡(•storage g).kind   ⍝ $t

⍝ — ₛ and ₚ mark a float's width, and after a list's closing bracket they mark every number in it
{(•storage ⍵).kind}¨[1.5ₛ 1.5ₚ [1 2]ₛ [1ₓ 2]ₚ ⍬ₛ []ₛ]   ⍝ "float32" "float64" "float32" "float64" "float32" "float32"

⍝ — •literal⁻¹ marks every float with its width, so the text reads back at either default width
•literal⁻¹ [1.5 2.5]ₛ   ⍝ "[1.5 2.5]ₛ"

⍝ — ₛ, ₚ and ₕ mark the width of a complex number's parts, after the number or after a list. ₕ gives 32-bit parts
{(•storage ⍵).kind}¨[[2j3]ₛ [2j3]ₚ [2j3]ₕ [1j2 3]ₛ]   ⍝ "complex64" "complex128" "complex64" "complex64"

⍝ — ₓ doesn't mark a complex number
2j3ₓ
⍝ error: SYNTAX ERROR

⍝ — Display marks complex numbers whose parts aren't of the default width once after the list
⎕←[1j2 3]ₛ
[1j2 3]ₛ
⍝ ⎕: [1ⱼ2 3]ₛ

⍝ — •literal⁻¹ marks the width of complex numbers' parts once after the list
•literal⁻¹¨[[1j2 3]ₛ;1j2 3]   ⍝ "[1ⱼ2 3]ₛ" "[1ⱼ2 3]ₚ"

⍝ — •float rounds complex numbers' parts to the width, and 16 bits gives 32-bit parts
{(•storage ⍵).kind}¨[64•float [1j2]ₛ;32•float [1j2];16•float [1j2]]   ⍝ "complex128" "complex64" "complex64"

⍝ — Complex numbers beside floats compute at the wider width, and exact numbers take the complex numbers' width
{(•storage ⍵).kind}¨[[2j3]ₛ×1.5ₚ [2j3]ₛ×1.5ₛ [2j3]ₛ×1.5ₕ [2j3]ₛ×2ₓ]   ⍝ "complex128" "complex64" "complex64" "complex64"

⍝ — Joining complex numbers to floats gives the wider width
{(•storage ⍵).kind}¨[[1j2]ₛ,0.5ₛ [1j2]ₛ,0.5ₚ [1j2]ₛ,⍳2ₓ [1j2]ₛ,[1j2]]   ⍝ "complex64" "complex128" "complex64" "complex128"

⍝ — A complex result from reals displays its parts' width
√¯4ₛ   ⍝ 0j2ₛ

⍝ — A complex result from reals has parts of the reals' width
{(•storage ⍵).kind}¨[√[¯4]ₛ √[¯4] √[¯4]ₕ]   ⍝ "complex64" "complex128" "complex64"

⍝ — Complex functions keep the width of the parts
{(•storage ⍵).kind}¨[1○[1j2]ₛ *[0j1]ₛ ⍟[0j1]ₛ [2j1]ₛ*[0.5j1]ₛ +\[1j2 3j4]ₛ]   ⍝ "complex64" "complex64" "complex64" "complex64" "complex64"

⍝ — A real result from complex numbers takes their parts' width
{(•storage ⍵).kind}¨[|[3j4]ₛ [1j2]ₛ×[1j¯2]ₛ]   ⍝ "float32" "float32"

⍝ — A fold keeps the width of complex numbers
+/[1j2 3j4]ₛ   ⍝ 4j6ₛ

⍝ — ₕ and 16 •float give 16-bit floats
{(•storage ⍵).kind}¨[1.5ₕ;[1 2]ₕ;⍬ₕ;16 •float 0.5]   ⍝ "float16" "float16" "float16" "float16"

⍝ — 16-bit floats keep their width beside exact numbers and widen beside 32-bit floats
v←16 •float 0.1 0.2
{(•storage ⍵).kind}¨[v+1ₓ v×v *v v+(32 •float 1)]   ⍝ "float16" "float16" "float16" "float32"

⍝ — Each 16-bit operation rounds its result to 16 bits
(16 •float 0.1)+(16 •float 0.2)   ⍝ 0.2998047ₕ

⍝ — +/, +\ and +.× on 16-bit floats keep 32-bit totals, where a 16-bit total would stop growing at 256
x←16 •float 4096⍴0.1
[+/x ¯1⌷+\x x+.×(16 •float 4096⍴1)]   ⍝ [409.5 409.5 409.5]ₕ

⍝ — A 16-bit matrix product stays 16-bit
,(16 •float [1 2⋄3 4])+.×(16 •float [5 6⋄7 8])   ⍝ [19 22 43 50]ₕ

⍝ — Writing into mixed storage keeps it mixed
x←'a' 1ₓ 2ₓ
x₀←0ₓ
(•storage x).kind   ⍝ "mixed"

⍝ — Rearranging or combining mixed storage keeps it mixed while its items need it
m←1ₓ 0.5 2ₓ
{(•storage ⍵).kind}¨[⌽m;2↑m;4↑m;1 0 1 1#⁻¹m;(⊂1 0)⌷m;2 1⍴m;⍉2 1⍴m;m,m;m⍪m;1⌽m;∪m;m~,2ₓ;↑⊂m;∊⊂m;↑↓2 2⍴m;↑1 1 0⊆m;0⊃⊂m]
⍝ =>
17⍴⊂"mixed"

⍝ — A copy of mixed storage whose items one dtype holds is compact, as a literal of those items is
{(•storage ⍵).kind}¨[1 0 1#1ₓ 0.5 2ₓ;1↓'a' 1ₓ 2ₓ;(1↓'a' 1ₓ 2ₓ),3ₓ]   ⍝ "integer" "integer" "integer"

⍝ — Arithmetic builds fresh storage from its results
(•storage 1ₓ×1↓'a' 1ₓ 2ₓ).kind   ⍝ "integer"

⍝ — Monadic ⌊ and × give exact integers
{(•storage ⍵).kind}¨[⌊0.5 1.5;×¯2.5 0 3.5]   ⍝ "integer" "integer"

⍝ — Monadic ⌊ on 32-bit floats gives exact integers, which widen past 32 bits when they need to
⌊32•float 1.5 3E9 ¯2.5   ⍝ [1 3000000000 ¯3]ₓ

⍝ — Exact integers joined with a 32-bit infinity give 32-bit floats
((⍳3ₓ),32•float∞)₃   ⍝ ∞ₛ

⍝ — Infinities of two widths join as floats of the wider width
(•storage (32•float∞),∞ₚ).kind   ⍝ "float64"

⍝ — Matrix division on 32-bit floats computes and gives 32-bit floats
(32•float 3 4)⌹32•float[1 1⋄0 2]   ⍝ [1 2]ₛ

⍝ — Beside exact integers, an infinity makes joined arrays floats and gathered items mixed
{(•storage ⍵).kind}¨[⌊1.5 ∞;(⍳3ₓ),∞;∞,⍳3ₓ;(⍳3ₓ),2⍴∞;{⍵=1?∞;⍵}¨⍳3ₓ]
"float64" "float64" "float64" "float64" "mixed"

⍝ — Written among exact integers, an infinity leaves them exact, in mixed storage
x←⍳3ₓ
x₁←∞
(•storage x).kind   ⍝ "mixed"

⍝ — Arithmetic with an infinity gives floats
{(•storage ⍵).kind}¨[0.5,∞ ¯∞;(⍳3ₓ)+∞]   ⍝ "float64" "float64"

⍝ — Under keyed agreement, a missing entry reads as its own array's fill
["a":1ₓ "b":∞]⌊["a":2ₓ "c":∞]   ⍝ ["a":1 "b":0 "c":0]ₓ

⍝ — An infinity is not a count
[2 ∞]ₓ↑2 3⍴⍳6ₓ
⍝ error: DOMAIN ERROR

⍝ — Written items keep each number's exactness, and only floats widen, to complex
{(•storage ⍵).kind}¨[1ₓ 2ₓ;1ₓ 0.5;1j2 0.5;1ₓ 1j2;1ₓ 1r2;[1ₓ 0.5];[1 0 1]ₓ;[1 ∞]ₓ;[0.5 1j2]]
"integer" "mixed" "complex128" "mixed" "mixed" "mixed" "integer" "mixed" "complex128"

⍝ —
{(•storage ⍵).kind}¨[[1ₓ 2ₓ⋄0.5 1.5] ["a":1ₓ "b":0.5]]   ⍝ "mixed" "mixed"

⍝ — Names written in brackets keep their values' exactness
a←2ₓ
b←0.5
(•storage [a b]).kind   ⍝ "mixed"

⍝ — Results computed item by item keep each number's exactness, as written items do. Kernels on compact storage give compact results
x←1ₓ 0.5
{(•storage ⍵).kind}¨[1×x;1ₓ×x;{⍵}¨x;¿6ₓ 7;{⍵=0?0.5;⍵}¨⍳3ₓ;1ₓ×⍳3ₓ]
⍝ =>
"float64" "mixed" "mixed" "mixed" "mixed" "integer"

⍝ — JSON arrays take the import rule, and JSON objects read as a literal of their fields does
{(•storage ⍵).kind}¨[•json "[1,1.5]";•json "[1.5,9007199254740993]";•json "{""a"":1,""b"":2}";•json "{""a"":1,""b"":2.5}"]
"float64" "mixed" "integer" "mixed"

⍝ — CSV columns take the same import rule
nl←•ucs 10
{(•storage ⍵).kind}¨•csv "i,f,m",nl,"1,2.5,9007199254740993",nl,"2,3,1.5"
⍝ =>
"i" "f" "m":"integer" "float64" "mixed"

⍝⍝ Static kinds

⍝ — An item of complex storage with no imaginary part reads as a float under dynamic kinds, and stays complex under static kinds
z←0j1 1j0×1 ⋄ d←(•storage z₁).kind ⋄ •prefs ["kinds":"static"] ⋄ s←(•storage z₁).kind ⋄ •prefs ["kinds":"dynamic"] ⋄ [d s]
"float64" "complex128"

⍝ — A weak seed takes the width of the items that it folds or scans, as a weak atom beside them does
v←32•float 1 2 3 ⋄ {(•storage ⍵).kind}¨[2.5+/v;2.5+\v]
"float32" "float32"

⍝ — A float or complex literal is weak, as NumPy treats a Python scalar: beside an array or a strong number it takes their float width. A literal with a width marker is strong, and so is an item taken from an array. Two weak numbers keep their own width
v←32•float 1.5 2.5 ⋄ w←1.5 2.5 ⋄ {(•storage ⍵).kind}¨[v×2.5;v×2.5ₚ;v×w₀;v₀×2.5;v×0j1;2.5×3.5]
"float32" "float64" "float64" "float32" "complex64" "float64"

⍝ — Each gives the same kinds whether it calls its function once on all cells or once for each cell, where weak and strong numbers meet
v←32•float 1.5 2.5 ⋄ w←1.5 2.5 ⋄ k←{(•storage ⍵).kind} ⋄ a←[k {⍵×2.5}¨v;k {+/⍵×v}¨w] ⋄ •prefs ["batch":$f] ⋄ b←[k {⍵×2.5}¨v;k {+/⍵×v}¨w] ⋄ •prefs ["batch":$t] ⋄ [(a≡b) a]
[$t ["float32" "float64"]]

⍝ — A float computed from exact numbers alone is weak, and so are the numbers that ⍎ reads. •json reads strong numbers
v←32•float 1.5 2.5 ⋄ {(•storage ⍵).kind}¨[v×√2ₓ;v×⍎"2.5";v×•json "2.5"]
"float32" "float32" "float64"

⍝ — A weak number compares at the width of the array beside it, and a modified assignment of one keeps the array's width
v←32•float 0.1 0.2 ⋄ e←v=0.1 ⋄ v.[0]+←2.5 ⋄ [e (•storage v).kind]
[[$t $f] "float32"]

⍝ — A weak number written into an array, catenated with one or added by union takes the array's width, as in arithmetic. A strong number widens the array, and a weak complex number beside 32-bit floats gives 64-bit complex numbers
v←32•float 1.5 2.5 ⋄ k←{(•storage ⍵).kind} ⋄ [k v,2.5;k 2.5⍪v;k v∪3.5;k v,2.5ₚ;k v,0j1;k {w←v ⋄ w.[[0]]←2.5 ⋄ w}0;k {w←v ⋄ w.[[0]]←2.5ₚ ⋄ w}0;k {2.5}@[0] v]
"float32" "float32" "float32" "float64" "complex64" "float32" "float64" "float32"

⍝ — An array built from items, such as a list in brackets or a map's results, gives a weak number the width of the strong floats among them, wherever it sits, and Each's two routes agree. A strong 64-bit float widens the array. Where the items stay mixed, a weak number becomes strong, so a copy keeps its width
v←32•float 1.5 0.5 2.5 ⋄ k←{(•storage ⍵).kind} ⋄ f←{[k {⍵>1?2.5;⍵}¨v;k {⍵>1?⍵;2.5}¨v]} ⋄ a←f 0 ⋄ •prefs ["batch":$f] ⋄ b←f 0 ⋄ •prefs ["batch":$t] ⋄ x←[0.1 v₀ "a"] ⋄ [k [v₀ 2.5];k [2.5 v₀];a≡b;a;k [2.5 v₀ (64•float 1)];k ¯1↓x;(¯1↓x)₀≡x₀]
["float32" "float32" $t ["float32" "float32"] "float64" "float64" $t]

⍝ — Under static kinds, an integer array computes at the wider of its storage width and the int width, so a 64-bit array under the int pref at 32 wraps at 64 bits
a←[4000000000 2]ₓ ⋄ •prefs ["kinds":"static" "int":32] ⋄ r←[a×a +/a×a (•storage a×a).kind] ⋄ •prefs ["kinds":"dynamic" "int":64] ⋄ r
[[¯2446744073709551616 4]ₓ ¯2446744073709551612ₓ "int64"]

⍝ — Under static kinds, √ fuses, because its kernel keeps NaN where dynamic kinds would rerun on complex numbers. ∂ gives a NaN tangent outside the domain
y←¿1000000⍴0 ⋄ •prefs ["kinds":"static"] ⋄ r←[(•explain "+/√y-0.5").fused.glyphs;{+/√⍵}∂4 ¯1 9] ⋄ •prefs ["kinds":"dynamic"] ⋄ r
("-√+/"⋄0.25 $n (÷6))

⍝⍝ Unicode conversion

⍝ — Case folding pervades nested text, keeps a final sigma and leaves numbers unchanged
•c 42 "Pete" "Πέτρος"   ⍝ 42 "pete" "πέτρος"

⍝ — A matrix converts each character on its own, and keeps one whose uppercase has several characters
1•c ["aBc"⋄"Σςß"]   ⍝ ["ABC"⋄"ΣΣß"]

⍝ — Strings convert with full mappings, which can change their length
1•c "straße"   ⍝ "STRASSE"

⍝ — Folding makes strings that differ only in case equal
•c "Straße" "STRASSE" "οδοσ" "ΟΔΟΣ"   ⍝ "strasse" "strasse" "οδος" "οδος"

⍝ — A singleton selector of any rank is accepted for case folding
[[¯3]⋄]•c "ίσως"   ⍝ "ίσως"

⍝ — Case conversion preserves an empty nested character prototype
•c 2 0⍴⊂"Ab"   ⍝ 2 0⍴⊂"  "

⍝ — System names are case-insensitive and may be bound to ordinary names
u←•UcS ⋄ u ["A⍳"⋄"λ😀"]   ⍝ [65 9075⋄955 128512]ₓ

⍝ —
•ucs [65 9075⋄955 128512]ₓ   ⍝ ["A⍳"⋄"λ😀"]

⍝ — Code-point conversion preserves an empty character array's shape
•ucs 2 0⍴""   ⍝ 2 0⍴0ₓ

⍝ — Empty numeric input converts to a character prototype
•ucs 0 3⍴0   ⍝ 0 3⍴""

⍝ —
"UTF-8"•ucs "Æ😀"   ⍝ [195 134 240 159 152 128]ₓ

⍝ —
["UTF-8" 0]•ucs 195 134 240 159 152 128   ⍝ "Æ😀"

⍝ — UTF-16 encodes an astral character as a surrogate pair
"UTF-16"•ucs "A😀"   ⍝ [65 55357 56832]ₓ

⍝ — UTF-16 decodes a surrogate pair into one character
"UTF-16"•ucs 65 55357 56832   ⍝ "A😀"

⍝ — An enclosed encoding name is accepted; UTF-32 retains one unit per character
(⊂"UTF-32")•ucs "A😀"   ⍝ [65 128512]ₓ

⍝ —
"UTF-32"•ucs 65 128512   ⍝ "A😀"

⍝ — Encoding a character returns a byte vector
"UTF-8"•ucs 'A'   ⍝ [65]ₓ

⍝ — Decoding one byte returns a character vector
"UTF-8"•ucs 65   ⍝ "A"

⍝ —
"UTF-16"•ucs ⍬   ⍝ ""

⍝ —
0•c 'a'
⍝ error: DOMAIN ERROR

⍝ —
1 2•c 'a'
⍝ error: DOMAIN ERROR

⍝ —
•ucs ¯1
⍝ error: DOMAIN ERROR

⍝ — Surrogates are not Unicode scalar values
•ucs 55296
⍝ error: DOMAIN ERROR

⍝ — Unicode code points stop at 10FFFF
•ucs 1114112
⍝ error: DOMAIN ERROR

⍝ —
•ucs 1.5
⍝ error: DOMAIN ERROR

⍝ — Mixed numeric and character input has no single conversion direction
•ucs 65 'B'
⍝ error: DOMAIN ERROR

⍝ — An empty array of strings gives an empty list of code-point vectors
•ucs 0⍴⊂"ab"   ⍝ 0⍴⊂⍬ₓ

⍝ — Encoding labels are case-sensitive
"utf-8"•ucs 'a'
⍝ error: DOMAIN ERROR

⍝ — Reject an overlong UTF-8 encoding of zero
"UTF-8"•ucs 192 128
⍝ error: DOMAIN ERROR

⍝ —
"UTF-8"•ucs 256
⍝ error: DOMAIN ERROR

⍝ — A lone high surrogate is invalid UTF-16
"UTF-16"•ucs 55357
⍝ error: DOMAIN ERROR

⍝ — UTF-16 code units must fit in 16 bits
"UTF-16"•ucs 65536
⍝ error: DOMAIN ERROR

⍝ —
"UTF-8"•ucs 2 2⍴'a'
⍝ error: RANK ERROR

⍝ — Signed-byte mode is explicitly outside basedpl's Unicode interface
["UTF-8" 83]•ucs "abc"
⍝ error: UNSUPPORTED

⍝⍝ Character and string literals

⍝ — A single-quoted character is an atom
⍴'a'   ⍝ ⍬ₓ

⍝ — Double quotes make a vector, even for one character
⍴"a"   ⍝ [1]ₓ

⍝ — The quote character needs no escaping inside single quotes
•ucs '''   ⍝ 39ₓ

⍝ — Apostrophes need no escaping inside double quotes
≢"it's"   ⍝ 4ₓ

⍝ — A doubled double quote is one quote character
≢"say ""hi"""   ⍝ 8ₓ

⍝ — The empty string
⍴""   ⍝ [0]ₓ

⍝ — A strand of strings holds each string as one item, so each is one key
("x" "y":1 2).y   ⍝ 2

⍝ — A strand of characters is one string
'x' 'y'   ⍝ "xy"

⍝ — A comment mark inside a string is an ordinary character
≢"it's ⍝ here"   ⍝ 11ₓ

⍝ — Single quotes around several characters
'ab'
⍝ error: SYNTAX ERROR

⍝ — Empty single quotes make the same empty character vector
''   ⍝ ""

⍝ — An unclosed string
"abc
⍝ error: SYNTAX ERROR

⍝ — Strings print in double quotes, and characters in single quotes
⎕←"ab" "cd"
⎕←'x' 1
⎕←''' "a""b"
⎕←"aa" "bb":"x" 'y'
⎕←"" "a"
0
⍝ =>
0
⍝ ⎕: "ab" "cd"\n'x' 1\n''' "a""b"\n["aa":"x" "bb":'y']\n"" "a"

⍝⍝ Character arithmetic

⍝ —
'a'+3   ⍝ 'd'

⍝ —
3ₓ+'a'   ⍝ 'd'

⍝ —
'd'-3r1   ⍝ 'a'

⍝ —
'd'-'a'   ⍝ 3ₓ

⍝ —
"012"-'0'   ⍝ [0 1 2]ₓ

⍝ —
"ab" "CD"+1   ⍝ "bc" "DE"

⍝ — Character offsets follow leading-axis agreement
["abc"⋄"def"]+1 2   ⍝ ["bcd"⋄"fgh"]

⍝ —
""+3   ⍝ ""

⍝ — Empty character subtraction has an exact numeric prototype
""-'a'   ⍝ ⍬ₓ

⍝ — Character offsets operate on code points, including astral characters
'😀'-1   ⍝ '🗿'

⍝ —
1-'a'
⍝ error: DOMAIN ERROR

⍝ —
'a'+'b'
⍝ error: DOMAIN ERROR

⍝ —
'a'×2
⍝ error: DOMAIN ERROR

⍝ — A character offset accepts a float within comparison tolerance of an integer
'a'+1.000000000000001   ⍝ 'b'

⍝ —
'a'+∞
⍝ error: DOMAIN ERROR

⍝ —
'a'+1j1
⍝ error: DOMAIN ERROR

⍝ —
'a'+1r2
⍝ error: DOMAIN ERROR

⍝ — Character arithmetic rejects rather than skips the surrogate gap
(•ucs 55295)+1
⍝ error: DOMAIN ERROR

⍝ — Character arithmetic cannot advance beyond the Unicode range
(•ucs 1114111)+1
⍝ error: DOMAIN ERROR

⍝ — Character arithmetic cannot move below code point zero
(•ucs 0)-1
⍝ error: DOMAIN ERROR

⍝ — A pending reversal or drop of characters computes through its source
[(⌽"abc")<"bbb";(1↓"abcd")+1]   ⍝ [$f $f $t;"cde"]

⍝ — A key that a character argument lacks reads a space
("xx" "yy" "zz":"abc")-("yy" "zz":"ab")   ⍝ "xx" "yy" "zz":[65 1 1]ₓ

⍝ — An inner product counts equal characters through their code points
"abc"+.="abd"   ⍝ 2ₓ

⍝⍝ Characters nesting and empty fill

⍝ — System functions use ordinary binding and composition
upper←1↣•c ⋄ •ucs∘upper "aZ"   ⍝ [65 90]ₓ

⍝ — System names resolve when executed
f←{•missing ⍵} ⋄ 1   ⍝ 1

⍝ — A system name includes its entire word
•c_unknown2 "abc"
⍝ error: VALUE ERROR

⍝ — System functions are read-only
•c←+
⍝ error: SYNTAX ERROR

⍝ — Brackets can hold system functions
fs←[•ucs •c] ⋄ (↑fs)'A'   ⍝ 65ₓ

⍝ —
$a   ⍝ "ABCDEFGHIJKLMNOPQRSTUVWXYZ"

⍝ —
$d   ⍝ "0123456789"

⍝ —
3↑$a   ⍝ "ABC"

⍝ — Alphabet constants are read-only
$a←"abc"
⍝ error: SYNTAX ERROR

⍝ — basedpl has fixed origin zero, not a mutable index-origin variable
•io←0
⍝ error: VALUE ERROR

⍝ —
'abc
⍝ error: SYNTAX ERROR

⍝ — A double-quoted string spans lines, with one newline for each line ending and its indentation kept
•ucs "a
  b"   ⍝ [97 10 32 32 98]ₓ

⍝ —
1.5↑1 2
⍝ error: DOMAIN ERROR

⍝ —
¯1⍴2
⍝ error: DOMAIN ERROR

⍝ —
1000000001⍴1
⍝ error: LIMIT ERROR

⍝ —
'a'   ⍝ 'a'

⍝ —
"界λ"   ⍝ "界λ"

⍝ —
"can't"   ⍝ "can't"

⍝ —
""   ⍝ ""

⍝ —
⍬   ⍝ ⍬

⍝ — Disclosing an empty numeric vector returns its zero prototype
↑⍬   ⍝ 0.0

⍝ —
↑⊂1 2   ⍝ 1.0 2.0

⍝ — An empty nested vector retains its first item's shape as prototype
↑0⍴[[1 2] [3 4 5]]   ⍝ 0.0 0.0

⍝ — Taking from empty text fills with spaces
3↑""   ⍝ "   "

⍝ — Negative overtake pads on the left
¯4↑1 2   ⍝ 0.0 0.0 1.0 2.0

⍝ —
2↓"abcd"   ⍝ "cd"

⍝ —
¯2↓"abcd"   ⍝ "ab"

⍝ —
5↓"ab"   ⍝ ""

⍝ —
0⍴"abc"   ⍝ ""

⍝ — Reshaping empty text uses its character prototype
3⍴""   ⍝ "   "

⍝ — An empty shape produces a scalar from the first element
⍬⍴1 2   ⍝ ⊂1.0

⍝ — Numeric negation of empty text yields an empty numeric result
-""   ⍝ ⍬

⍝ — Direct division on empty text makes no element calls
1÷""   ⍝ ⍬

⍝ —
"ab"=1 2   ⍝ $f $f

⍝ —
"abc"="axc"   ⍝ $t $f $t

⍝ — Characters order by code point
'a'<'b'   ⍝ $t

⍝ — A mixed array compares each pair of items, characters included
'a' 1<'b' 0   ⍝ $t $f

⍝ —
"ab"≥"ba"   ⍝ $f $t

⍝ — The maximum and minimum of characters are characters
"abc"⌈"bab"   ⍝ "bbc"

⍝ —
'a'⌊'b'   ⍝ 'a'

⍝ — Numbers order before characters, whatever their values
[3<'a';3<(•ucs 0);3⌈'a';3⌊'a']   ⍝ [$t;$t;'a';3]

⍝ — A compact array against characters takes the order of its kind
[1 2 3>"abc";"ab"⌊5 6]   ⍝ [$f $f $f;5 6]

⍝ — A mixed array orders each pair of items by kind first
'a' 2<1 'b'   ⍝ $f $t

⍝ —
'a' 2⌊1 'b'   ⍝ 1 2

⍝ — An exact fraction orders before a character, and the minimum keeps it exact
1r3⌊'a'   ⍝ 1r3

⍝ — An empty maximum of numbers and characters has a character prototype
⍬⌈""   ⍝ ""

⍝ —
+"abc"   ⍝ "abc"

⍝ —
+""   ⍝ ""

⍝ — An atomic index retrieves a character vector
1⌷"abc" "def" "ghi"   ⍝ "def"

⍝ —
gg←2 3 4 5 ⋄ 9,gg₁,3 4   ⍝ 9 3 3 4

⍝ — Split into rows; each further split encloses the resulting vector or scalar
↓↓↓2 2⍴⍳4ₓ   ⍝ (⊂⊂[[0 1] [2 3]])ₓ

⍝ — Reduction returns the accumulated vector
+/[[1 2] [3 4]]   ⍝ 4 6

⍝ —
[[1 2] [3 4]]+10   ⍝ [[11 12] [13 14]]

⍝ —
0 3⍴⍬   ⍝ 0 3⍴0

⍝ — An empty exact array keeps an exact fill
↑0↑1ₓ   ⍝ 0ₓ

⍝ — A string can hold parentheses, braces, a lamp and a diamond
≢"({⍝⋄})"
6ₓ

⍝⍝ Matrix axes scan and cell assembly

⍝ —
+/2 3⍴⍳6ₓ   ⍝ [3 12]ₓ

⍝ —
+⌿2 3⍴⍳6ₓ   ⍝ [3 5 7]ₓ

⍝ — Subtraction scan accumulates from left to right
-\1 2 3   ⍝ 1 ¯1 ¯4

⍝ — Subtraction scans of long lanes and along the leading axis match the dfn scan
v←1+⍳20 ⋄ m←[4 20]⍴v ⋄ z←v+0j1×v ⋄ [(-\v)≡{⍺-⍵}\v;(5-\v)≡5{⍺-⍵}\v;(-\m)≡{⍺-⍵}\m;(-⍀m)≡{⍺-⍵}⍀m;(-\z)≡{⍺-⍵}\z]   ⍝ $t $t $t $t $t

⍝ — Float scan retains left-to-right accumulation through cancellation
+\1E100 ¯1E100 1   ⍝ 1E100 0 1

⍝ — A dfn scan uses the same left-to-right order as primitive scan
{⍺+⍵}\1E100 ¯1E100 1   ⍝ 1E100 0 1

⍝ — Dyadic scan starts from a seed and omits the seed from its result
10-\1 2 3   ⍝ 9 7 4

⍝ —
100+\10 20   ⍝ 110 130

⍝ —
10ₓ-\[1 2 3]ₓ   ⍝ [9 7 4]ₓ

⍝ — A seeded scan of a number still applies the operand once
2+\5   ⍝ 7

⍝ — Trailing-axis scan takes one seed per row
10 20 (+\⍤0 1) 2 3⍴⍳6ₓ   ⍝ [10 11 13⋄23 27 32]

⍝ — Leading-axis scan takes one seed per column
⍉ 10 20 30 (+\⍤0 1) ⍉2 3⍴⍳6ₓ   ⍝ [10 21 32⋄13 25 37]

⍝ — Rank pairs row seeds for scan with an axis
10 20 ({⍺+⍵}\⍠0⍤0 1) 2 3⍴⍳6ₓ   ⍝ [10 11 13⋄23 27 32]

⍝ — Growing scan accumulators remain nested, without mix-style padding
{⍺,⍵}\1 2 3   ⍝ [1 [1 2] [1 2 3]]

⍝ — A seed participates in every growing accumulator
0{⍺,⍵}\1 2 3   ⍝ [[0 1] [0 1 2] [0 1 2 3]]

⍝ — An empty seeded scan makes no operand calls
10{1÷0}\⍬   ⍝ ⍬

⍝ — An unseeded singleton scan makes no operand calls
{1÷0}\,5   ⍝ ,5

⍝ — Empty scan rows retain their frame and shape
10 20+\2 0⍴0   ⍝ 2 0⍴0

⍝ — Three empty rows reduce to three identities
+/3 0⍴0   ⍝ 0 0 0

⍝ — No rows gives no reduction results
+/0 3⍴0   ⍝ ⍬

⍝ —
+\2 3⍴⍳6ₓ   ⍝ [0 1 3⋄3 7 12]ₓ

⍝ —
+⍀2 3⍴⍳6ₓ   ⍝ [0 1 2⋄3 5 7]ₓ

⍝ — A column scan keeps negative zeros
1÷+⍀ -[3 2]⍴0   ⍝ [¯∞ ¯∞⋄¯∞ ¯∞⋄¯∞ ¯∞]

⍝ native-scan-lanes — Bulk comparisons cross chunks and keep lane starts, seeds and inverse heads
m←[17 503]⍴3ₓ|⍳8551ₓ
[(<\m)≡{⍺<⍵}\m (<⍀m)≡{⍺<⍵}⍀m (0.5<⍀m)≡0.5{⍺<⍵}⍀m (+⍀⁻¹+⍀m)≡m]
⍝ =>
$t $t $t $t

⍝ — A sum of negative zeros stays negative at each float width, and an empty sum is 0
x←16•float -3⍴0 ⋄ 1÷[+/ -3⍴0; +/x; +/16•float⍬]   ⍝ ¯∞ ¯∞ ∞

⍝ —
+\0 3⍴0   ⍝ 0 3⍴0

⍝ — Mix pads shorter cells to the longest cell
⊃[[1 2] [3 4 5]]   ⍝ [1 2 0⋄3 4 5]

⍝ — Mix pads an atomic cell rather than extending its value
⊃[1 [2 3]]   ⍝ [1 0⋄2 3]

⍝ — A row without keys takes the keys of the rows that have them
["c":1 "d":3⋄8 ∞]   ⍝ ["c":1 "d":3⋄"c":8 "d":∞]

⍝ — Mix keeps the keys that its keyed cells agree on
⊃[10 20;["a":1 "b":2]]   ⍝ ["a":10 "b":20⋄"a":1 "b":2]

⍝ — Rows give no keys when a row needs padding
["a":1 "b":2⋄10 20 30]   ⍝ [1 2 0⋄10 20 30]

⍝ — Mix keeps the axis name that its named cells agree on, as catenate does
⍴⊃[(["i":2]⍴1 2);10 20]   ⍝ [2 "i":2]ₓ

⍝ — Empty mix uses the retained cell prototype to determine its trailing shape
⊃0⍴[[1 2] [3 4 5]]   ⍝ 0 2⍴0

⍝ —
⊃⊂1 2   ⍝ 1 2

⍝ — Empty nested reduction uses a conforming identity
+/0⍴[[1 2] [3 4]]   ⍝ 0 0

⍝ — Empty character sum uses the numeric addition identity
+/""   ⍝ 0

⍝ — Leading-axis replicate keeps the selected matrix row
1 0#2 3⍴⍳6ₓ   ⍝ [0 1 2⋄]ₓ

⍝ — Each accumulator retains the whole vector seed
(,10)+\1 2 3   ⍝ [,11 ,13 ,16]

⍝ — Each lane starts with the same whole seed
10 20 30+\2 3⍴⍳6ₓ
2 3⍴[[10 20 30] [11 21 31] [13 23 33] [13 23 33] [17 27 37] [22 32 42]]

⍝ — An empty scan makes no calls with its array seed
10 20 30+\2 0⍴0   ⍝ 2 0⍴0

⍝ — A named first-axis reduction reduces along the first axis
m←2 3⍴⍳6ₓ
r←⌿
+r m   ⍝ [3 5 7]ₓ

⍝ — The operand observes successive left accumulators in order
f←{⎕←[⍺ ⍵] ⋄ ⍺-⍵} ⋄ f\1 2 3
1 ¯1 ¯4
⍝ ⎕: 1 2\n¯1 3

⍝ — Seeded scan exposes the seed as the first left argument
r←10{⎕←[⍺ ⍵] ⋄ ⍺-⍵}\1 2 3
9 7 4
⍝ ⎕: 10 1\n9 2\n7 3

⍝⍝ Structural slices and brackets

⍝ — Brackets round one item make a one-item vector, even when nested
[[1 2 3]]   ⍝ ,⊂1 2 3

⍝ — A named operator takes a grouped operand through the ordinary binding rule
p←⍣ ⋄ (1↣+)p(⍳3ₓ) 0   ⍝ 0 1 2

⍝ — Functions inside brackets are values, including trains
fs←[+÷≢] ⋄ (↑fs)2 4 9   ⍝ 2 4 9÷3

⍝ — Items in brackets use the surrounding dfn's arguments
{[⍺+⍵]}⍨3   ⍝ ,6

⍝ — A one-item vector evaluates its contents once
[⎕←3]
,3
⍝ ⎕: 3

⍝ — Index selects from a disclosed enclosure
v←⊂1 2 3 ⋄ 1⌷↑v   ⍝ 2

⍝ — Empty brackets write an empty vector
[]   ⍝ ⍬

⍝ —
⍉[1 2 3⋄4 5 6]   ⍝ [1 4⋄2 5⋄3 6]

⍝ —
⌽[1 2 3⋄4 5 6]   ⍝ [3 2 1⋄6 5 4]

⍝ —
⊖[1 2 3⋄4 5 6]   ⍝ [4 5 6⋄1 2 3]

⍝ — Rotate each row by its own count
1 2⌽[1 2 3⋄4 5 6]   ⍝ [2 3 1⋄6 4 5]

⍝ — Catenating a vector to a matrix appends one element per row
[1 2 3⋄4 5 6],8 9   ⍝ [1 2 3 8⋄4 5 6 9]

⍝ —
1 2↑[1 2 3⋄4 5 6]   ⍝ [1 2⋄]

⍝ — Negative multidimensional overtake pads before the retained bottom-right cells
¯3 ¯2↑[1 2 3⋄4 5 6]   ⍝ [0 0⋄2 3⋄5 6]

⍝ —
1 ¯1↓[1 2 3⋄4 5 6]   ⍝ [4 5⋄]

⍝ — Taking a matrix from a number pads rather than repeating it
2 3↑7   ⍝ [7 0 0⋄0 0 0]

⍝ —
1 0⌷[1 2 3⋄4 5 6]   ⍝ 4

⍝ —
[: 1]⌷[1 2 3⋄4 5 6]   ⍝ 2 5

⍝ —
[1 :]⌷[1 2 3⋄4 5 6]   ⍝ 4 5 6

⍝ —
[[2 0]]⌷10 20 30   ⍝ 30 10

⍝ — The axis operand ends at the space before an array literal
+/⍠0 [1 2 3⋄4 5 6]   ⍝ 5 7 9

⍝ — A named reduction accepts the same axis operand
s←+/ ⋄ s⍠0 [1 2 3⋄4 5 6]   ⍝ 5 7 9

⍝ — Repeated transpose axes select the diagonal of a rectangular matrix
0 0⍉[1 2 3⋄4 5 6]   ⍝ 1 5

⍝ — A short transpose list leaves the remaining axes in order after the named ones, as in BQN
2⍉2 3 4⍴⍳24ₓ
(3 4 2⍴0 12 1 13 2 14 3 15 4 16 5 17 6 18 7 19 8 20 9 21 10 22 11 23)ₓ

⍝ — The inverse of a short transpose list restores the argument
2⍉⁻¹2⍉2 3 4⍴⍳24ₓ   ⍝ 2 3 4⍴⍳24ₓ

⍝ transpose:names — Forward transpose routes to positions, not existing axis names
["w" "b" "h"]⍉["b":2 "h":3 "w":4]⍴⍳24ₓ
⍝ error: DOMAIN ERROR

⍝ transpose:inverse-names — Inverse transpose lists source axes in output order
x←["b":2 "h":3 "w":4]⍴⍳24ₓ
["w" 0 "h"]⍉⁻¹x
⍝ =>
2 0 1⍉⁻¹["b":2 "h":3 "w":4]⍴⍳24ₓ

⍝ transpose:split — Split one input axis, then route its factors independently
[0 [2 2;1 3]]⍉2 6⍴⍳12ₓ
⍝ =>
0 2 1⍉2 2 3⍴⍳12ₓ

⍝ transpose:inverse — A bound split permutation has a genuine inverse
x←2 3 4⍴⍳24ₓ
f←[0 2 [1 2;3 ∞]]⍉
(f⁻¹f x)≡x   ⍝ $t

⍝ transpose:attention — Declared names label both sides of a bound split-and-route function
x←["b":2 "t":3 "d":8]⍴⍳48ₓ
f←["b":0 "t":2 "d":["h":1 2;"f":3 ∞]]⍉
[⍴f x;(f⁻¹f x)≡x;⍴+/⍠"h" f x]
⍝ =>
[["b":2 "h":2 "t":3 "f":4]ₓ $t ["b":2 "t":3 "f":4]ₓ]

⍝ transpose:patches — Named factors can be reordered and merged by an output pattern
images←["b":1 "h":4 "w":6 "c":2]⍴⍳48ₓ
f←["b":0 "h":["i":1 ∞;"y":3 2] "w":["j":2 ∞;"x":4 3] "c":5]⍉
y←f images
[⍴y (f⁻¹y)≡images ⍴["b" "patch":["i" "j"] "pixel":["y" "x" "c"]]⍉⁻¹y]
⍝ =>
[["b":1 "i":2 "j":2 "y":2 "x":3 "c":2]ₓ $t ["b":1 "patch":4 "pixel":12]ₓ]

⍝ transpose:merge — The inverse can omit factor sizes supplied by its input
[0 [2 1]]⍉⁻¹0 2 1⍉2 2 3⍴⍳12ₓ
⍝ =>
2 6⍴⍳12ₓ

⍝ transpose:merge-names — Merging named axes preserves only untouched axis names
x←["b":2 "h":3 "p":4]⍴⍳24ₓ
["b" ["h" "p"]]⍉⁻¹x
⍝ =>
["b":2 12]⍴⍳24ₓ

⍝ transpose:named-arrays — A named array in the left argument names its axis, as a string does, alone or in a merge
x←["i":2 "k":2 "j":3]⍴⍳12ₓ
i←⍳["i":2] ⋄ k←⍳["k":2] ⋄ j←⍳["j":3]
[j⍉⁻¹x [[i k]]⍉⁻¹x]
⍝ =>
x←["i":2 "k":2 "j":3]⍴⍳12ₓ
["j"⍉⁻¹x [["i" "k"]]⍉⁻¹x]

⍝ transpose:partial — A grouped partial map fills unused axes in order
[[1 2;0 ∞]]⍉6 4⍴⍳24ₓ
⍝ =>
1 0 2⍉2 3 4⍴⍳24ₓ

⍝ transpose:metadata — Untouched axes keep their position keys when another splits
x←"a" "b":2 6⍴⍳12ₓ
[0 [2 2;1 ∞]]⍉x
⍝ =>
"a" "b":0 2 1⍉2 2 3⍴⍳12ₓ

⍝ transpose:bad-product — Split dimensions must exactly factor the source axis
[[0 2;1 2]]⍉⍳6ₓ
⍝ error: LENGTH ERROR

⍝ transpose:bad-size — Inverse transpose checks supplied factor sizes
[[0 2;1 ∞]]⍉⁻¹3 2⍴⍳6ₓ
⍝ error: LENGTH ERROR

⍝ transpose:ambiguous — Zero times an inferred factor does not determine that factor
[[0 0;1 ∞]]⍉⍬
⍝ error: DOMAIN ERROR

⍝ transpose:under — Under writes back through inverse transpose, which moves and merges axes without dropping items
x←["b":2 "h":3 "w":4]⍴⍳24ₓ
[((10×)@(2 0 1⍉⁻¹) x)≡10×x ((10×)@(["b" ["h" "w"]]⍉⁻¹) x)≡10×x (⌽@(["w" "b" "h"]⍉⁻¹) x)≡⌽⍠"h"⊢x]   ⍝ $t $t $t

⍝ transpose:repeated-name — A key cannot give a name that another result axis has
["w":0]⍉["b":2 "h":3 "w":4]⍴⍳24ₓ
⍝ error: DOMAIN ERROR

⍝ — A negative replicate count replaces its item with fills
2 ¯1 1#10 20 30   ⍝ 10 10 0 30

⍝ — Inverse replicate gives a fill for a zero or negative count
2 0 ¯1 1#⁻¹10 10 0 30   ⍝ 10 0 0 30

⍝ — Expansion uses character fill for inserted positions
1 0 1#⁻¹"ab"   ⍝ "a b"

⍝ — Negative positions count from the end
¯1⌷1 2   ⍝ 2

⍝ —
¯3⌷1 2
⍝ error: INDEX ERROR

⍝ —
1.5⌷1 2
⍝ error: DOMAIN ERROR

⍝ — Omitted trailing axes select the whole cell
0⌷[1 2⋄3 4]   ⍝ 1 2

⍝ —
+/⍠0.5 [1 2⋄3 4]
⍝ error: DOMAIN ERROR

⍝ — A dfn's separators don't split the surrounding matrix
[: 0]⌷[{⍵=0?1;⍵+2}0 3⋄4 5]   ⍝ 1 4

⍝ — Reversing twice, flipping twice, transposing twice and splitting then mixing give back the argument
{[⌽⌽⍵ ⊖⊖⍵ ⍉⍉⍵ ⊃↓⍵]}¨[0 3;3 0;2 3;2 2 3]⍴¨⊂⍳12ₓ
{4⍴⊂⍵}¨[0 3;3 0;2 3;2 2 3]⍴¨⊂⍳12ₓ

⍝⍝ Pervasive math

⍝ — Complex floor uses Gaussian-integer cells, not independent component floors
⌊1.5j0.5   ⍝ 1j1

⍝ — Complex gcd uses Gaussian-integer arithmetic and tolerant comparison
0.2=3.8j7.6∨5.2j6.8   ⍝ $t

⍝ — Scale and ceil to expose the complex lcm despite floating-point roundoff
⌈1000×3.8j7.6∧5.2j6.8   ⍝ ¯159600j326800

⍝ —
|3 ¯3 3j4   ⍝ 3 3 5

⍝ — Residue follows the divisor's sign, including fractional divisors
2 10 ¯2.5|7 ¯13 8   ⍝ 1 7 ¯2

⍝ — Zero residue-base returns the argument unchanged
0 3|¯2 6   ⍝ ¯2 0

⍝ — Residue by one integer takes that integer's sign across the whole integer range
x←[¯7 7 0 ¯9223372036854775808 9223372036854775807]ₓ
[3ₓ|x ¯3ₓ|x 4ₓ|x ¯1ₓ|x]
⍝ =>
[[2 1 0 1 1]ₓ [¯1 ¯2 0 ¯2 ¯2]ₓ [1 3 0 0 3]ₓ [0 0 0 0 0]ₓ]

⍝ — Floor applies comparison tolerance near integers
⌊1.000000000000001 ¯1.000000000000001   ⍝ [1 ¯1]ₓ

⍝ — Ceiling of a float is an exact integer
⌈¯2.3 0.1 3   ⍝ [¯2 1 3]ₓ

⍝ — Floor of a large float is its exact integer
⌊2*70   ⍝ 1180591620717411303424ₓ

⍝ — Floor of floats that hold an infinity gives floats
⌊1.5 ∞ ¯∞   ⍝ 1 ∞ ¯∞

⍝ — Signum of a float is an exact integer
×¯2.5 0 ∞   ⍝ [¯1 0 1]ₓ

⍝ — An integer argument accepts a float within comparison tolerance of an integer
(0.1×3 × 10)⌷10 20 30 40   ⍝ 40

⍝ — A count accepts a float within comparison tolerance of an integer
⍳(0.1×3)×10   ⍝ 0 1 2

⍝ — A count still rejects a fraction outside tolerance
⍳2.5
⍝ error: DOMAIN ERROR

⍝ —
2 3⌊3 2   ⍝ 2 2

⍝ —
2 3⌈3 2   ⍝ 3 3

⍝ —
2 ¯2 0*3 3 0   ⍝ 8 ¯8 1

⍝ —
2ₓ*¯3ₓ   ⍝ 1r8

⍝ —
2r3|7r3   ⍝ 1r3

⍝ —
⌊¯4r3   ⍝ ¯2ₓ

⍝ —
⌈¯4r3   ⍝ ¯1ₓ

⍝ — The logarithm of zero and zero to a negative power are infinities
[⍟0 0*¯1]   ⍝ ¯∞ ∞

⍝ —
13○1
⍝ error: DOMAIN ERROR

⍝ —
0j1⌊2
⍝ error: DOMAIN ERROR

⍝ — Exponential, logarithm, pi times and circle functions
[*1 2⍟32 π1 1○⁻¹1]   ⍝ 2.718281828459045 5 3.141592653589793 1.5707963267948966

⍝ — A negative number to a fractional power is complex
¯4*0.5   ⍝ 0j2

⍝ — Complex floor and ceiling
[⌊3.3j2.5 ⌈3.3j2.5]   ⍝ 3j2 3j3

⍝ — Euler's identity holds within tolerance
*π0j1   ⍝ ¯1

⍝ — The gcd of two reals recovers their rational ratio, in either order [rtol=1e-14 atol=0]
x←[1 1ₓ ¯1 0 0 1 1E¯200 1E200 1E¯200]
y←[103993÷33102 π1 ¯103993÷33102 ¯0.75 0 1.000000000000005 3E¯200 3E200 1E200]
[x∨y y∨x]
⍝ =>
2⍴⊂[÷33102 ÷1725033 ÷33102 0.75 0 1 1E¯200 1E200 1E¯200]

⍝ — Reals outside comparison tolerance of a rational ratio have a tiny gcd
1E¯12>1∨1.00000000000005   ⍝ $t

⍝ — The gcd of NaN and any real is NaN
[$n∨3.5 3.5∨$n $n∨$n 0∨$n $n∧2.5]   ⍝ $n $n $n $n $n

⍝⍝ Search depth and random

⍝ — Index-of compares each candidate directly; tolerance is not transitive
x←1
y←1+8E¯15
z←1+16E¯15
[x y]⍳z   ⍝ 1ₓ

⍝ — Unique-mask puts a chain of tolerant matches in one class
≠[1 1+8E¯15 1+16E¯15]   ⍝ $t $f $f

⍝ — A row is a copy of the first earlier distinct row that it matches, so rows don't chain
≠[1 2⋄1+8E¯15 2⋄1+16E¯15 2]   ⍝ $t $f $t

⍝ — Monadic `∩` keeps each distinct major cell that occurs more than once, in order of first appearance
∩"mississippi"   ⍝ "isp"

⍝ — Duplicate rows of a matrix are its duplicate items
∩[1 2⋄3 4⋄1 2]   ⍝ 1 2⍴1 2

⍝ — Monadic `#` gives each run's length and the cell that repeats
#"aaabcc"   ⍝ [[3 1 2]ₓ "abc"]

⍝ — The runs of a matrix are runs of equal rows
#[1 2⋄1 2⋄3 4]   ⍝ [[2 1]ₓ [1 2⋄3 4]]

⍝ — `#/` rebuilds the argument from its runs
#/#"mississippi"   ⍝ "mississippi"

⍝ — `#⁻¹` decodes runs, so it inverts `#`
#⁻¹#"mississippi"   ⍝ "mississippi"

⍝ — A unit encodes with a rank-0 length, so `#⁻¹` gives the unit back
#⁻¹#5   ⍝ 5

⍝ — Any other one-number length replicates
#⁻¹ 3 5   ⍝ 5 5 5

⍝ — A keyed leading axis adds the key of every position, which `#⁻¹` attaches again
#⁻¹#["x":1 "y":1 "z":2]   ⍝ ["x":1 "y":1 "z":2]

⍝ — Under `#` changes the runs and decodes them again
{2×⍵}@(0⊃)@# "aab"   ⍝ "aaaabb"

⍝ — An empty argument has no runs
#⍬   ⍝ [⍬ₓ ⍬]

⍝ — Monadic `⍷` gives each distinct major cell's positions, in the order of `∪`
⍷"abca"   ⍝ (0 3⋄1⋄2)ₓ

⍝ — On a keyed axis, the positions are keys
⍷["x":1 "y":2 "z":1]   ⍝ ("x" "z"⋄"y")

⍝ — An empty argument has no groups
⍷⍬   ⍝ 0⍴⊂⍬ₓ

⍝ — Positions count along the whole axis, also in a group much smaller than the axis
1⊃⍷(300⍴0),1 1   ⍝ [300 301]ₓ

⍝ — A large index-of takes the first position within tolerance, not the nearest value
(300⍴5 , [1+16E¯15 1])⍳1+8E¯15   ⍝ 300ₓ

⍝ — A large index-of finds nothing outside tolerance
(300⍴5 , 1)⍳1+2E¯14   ⍝ 301ₓ

⍝ — A large search never matches a character with its code point
(97ₓ+⍳8ₓ)⍳36⍴"abcdefgh"   ⍝ 36⍴8ₓ

⍝ — Large membership compares exact integers with floats within tolerance
[1+8E¯15 1+2E¯14]∊⍳300ₓ   ⍝ $t $f

⍝ — A large index-of of exact values applies no tolerance
(⍳300ₓ)⍳1000000000000001r1000000000000000   ⍝ 300ₓ

⍝ — Without uses tolerance when approximate values participate
1 2~[1+8E¯15]   ⍝ [2]

⍝ — Without does not apply tolerance to all-exact values
[1 2]ₓ~[1000000000000001r1000000000000000]   ⍝ [1 2]ₓ

⍝ — Each empty row matches the first empty row
(3 0⍴0)⍳2 0⍴0   ⍝ [0 0]ₓ

⍝ — A matrix of empty rows still has one unique major cell
≠3 0⍴0   ⍝ $t $f $f

⍝ — Where repeats a position according to its count
⍸0 1 0 2   ⍝ [1 3 3]ₓ

⍝ — Interval index ignores comparison tolerance at interval boundaries
1 2⍸1-1E¯15   ⍝ 0ₓ

⍝ — Depth of an empty nested array comes from its prototype
≡0⍴[[1 2] 3]   ⍝ 2ₓ

⍝ — Not-match distinguishes an atom from a singleton vector
1≢,1   ⍝ $t

⍝ — Match compares numeric value across exact and approximate domains
1ₓ≡1   ⍝ $t

⍝ — Empty numeric prototypes match across exact and approximate domains
(0⍴1ₓ)≡⍬   ⍝ $t

⍝ —
⍳,3   ⍝ 0 1 2

⍝ — Union retains duplicates within either argument
1 1∪2 2   ⍝ 1 1 2 2

⍝ — Membership uses tolerance when approximate values participate
1 2∊[1+8E¯15]   ⍝ $t $f

⍝ — A vector pattern does not fit a unit search target
(,1)⍷1   ⍝ $f

⍝ native-search-shapes — Compact and mixed patterns share shape traversal; fractional needles keep tolerant matching
p←[1 [2 3]] ⋄ v←[1 [2 3] 0 1 [2 3]]
[p⍷v [1 2]⍷[1 2 1⋄1 2 3] [1 2 3]ₓ⍳[2 2.000000000000001]]
⍝ =>
[$t $f $f $t $f;[$t $f $f⋄$t $f $f];[1 1]ₓ]

⍝ native-find-widths — Compact float patterns use the wider operand's tolerance and match NaNs
[(32•float[1])⍷[1+8E¯15] (32•float[1])⍷32•float[1+8E¯7] [$n]⍷[$n 0]]
⍝ =>
[[$t] [$f] [$t $f]]

⍝ native-row-ranks — Narrow row keys grade stably and ordinal ranks invert that grade
m←[3 0⋄1 2⋄3 0⋄2 1]ₓ
[⍋m ⍋⍋m]   ⍝ [[1 3 0 2]ₓ [2 0 3 1]ₓ]

⍝ — An empty pattern still cannot exceed the target on another axis
(2 0⍴0)⍷1 3⍴1   ⍝ 1 3⍴$f

⍝ — Rank-zero iota contains one enclosed empty coordinate
⍳⍬   ⍝ ⊂⍬

⍝ — Iota applies to each row of a matrix, padding the results with fill to a common shape
⍳[2 3⋄1 1]ₓ   ⍝ ⊃[⍳[2 3]ₓ;[2 3]ₓ↑⍳[1 1]ₓ]

⍝ — Where on the number 0 retains an empty-coordinate prototype
⍸0   ⍝ 0⍴⊂⍬ₓ

⍝ —
3¿2
⍝ error: DOMAIN ERROR

⍝ —
¿¯1
⍝ error: DOMAIN ERROR

⍝ —
⍸¯1
⍝ error: DOMAIN ERROR

⍝ —
2⍳2
⍝ error: RANK ERROR

⍝ —
(2 3⍴0)⍳1 2
⍝ error: LENGTH ERROR

⍝ —
(2 2⍴1)~1
⍝ error: RANK ERROR

⍝ — Unsorted boundaries give a count, not an error
2 1⍸1   ⍝ 2ₓ

⍝ — Empty results of exact functions keep an exact fill
↑¨[⌊0⍴1ₓ ⌈0⍴1ₓ |0⍴1ₓ !0⍴1ₓ 2ₓ*0⍴1ₓ]   ⍝ [0 0 0 0 0]ₓ

⍝ — Roll gives whole numbers below its argument, exact only for an exact argument
{[⍴⍵ ∧/(⍵<9)∧⍵=⌊⍵ (•storage ⍵).kind]}¨[¿100⍴9 ¿100⍴9ₓ]
[[[100]ₓ $t "float64"] [[100]ₓ $t "integer"]]

⍝ — Deal gives distinct whole numbers below its right argument
{[⍴⍵ ≢∪⍵ ∧/(⍵<52)∧⍵=⌊⍵ (•storage ⍵).kind]}¨[13¿52 13ₓ¿52ₓ]
[[[13]ₓ 13ₓ $t "float64"] [[13]ₓ 13ₓ $t "integer"]]

⍝ — Roll of zero gives floats strictly between 0 and 1
x←¿100⍴0 ⋄ ∧/(0<x)∧x<1   ⍝ $t

⍝⍝ Each commute and reduction

⍝ — When no call gives a result, every item is `⍬`
{}¨1 2   ⍝ ⍬ ⍬

⍝ — Rank assembles the same `⍬` results
({}⍤0) 1 2   ⍝ 2 0⍴0

⍝ — Each on an empty argument gives a system function's result prototype without running it
≢•signal¨0⍴⊂"DOMAIN"   ⍝ 0ₓ

⍝ — Each on an empty argument draws nothing from a generator
g←•rand 1
a←g.roll 6
g←•rand 1
_←g.roll¨0⍴⊂6
a=g.roll 6   ⍝ $t

⍝ — Empty each calls Pick on prototypes to determine numeric fill
⍬⊃¨⊂1 2 3   ⍝ ⍬

⍝ — Prototype-mode Pick also works through a dfn
⍬{⍺⊃⍵}¨⊂1 2 3   ⍝ ⍬

⍝ — Prototype-mode Pick can supply character fill for an out-of-range positive index
f←{100⊃"abc"} ⋄ f¨⍬   ⍝ ""

⍝ — Prototype mode survives nested helper calls
pick←{⍺⊃⍵}
wrap←{⍺ pick ⍵}
⍬ wrap¨ ⊂"abc"   ⍝ ""

⍝ — Prototype mode also passes through a defined operator
op←{⍶ ⍵} ⋄ {100⊃"abc"}op ¨⍬   ⍝ ""

⍝ — A composed operand derives character fill in empty each
⍬(⊃↢⊢)¨⊂"abc"   ⍝ ""

⍝ — Commute preserves prototype-mode Pick
(⊂1 2 3)⊃⍨¨⍬   ⍝ ⍬

⍝ — Empty Pick-each retains the selected nested vector's shape
⍬⊃¨⊂[[1 2] [3 4 5]]   ⍝ 0⍴⊂0 0

⍝ — An empty-path prototype selects the whole right-hand vector
(0⍴⊂⍬)⊃¨⊂1 2 3   ⍝ 0⍴⊂0 0 0

⍝ — The requested nested item, not always the first, determines result fill
1⊃¨0⍴⊂[[1 2] [3 4 5]]   ⍝ 0⍴⊂0 0 0

⍝ — Named functions and operators retain their grammatical roles
e←¨
sum←+/
sum e [[1 2] [3 4 5]]   ⍝ 3 12

⍝ — Nested each applies singleton reduction to each numeric leaf
+/¨¨[[1 2] [3 4]]   ⍝ [[1 2] [3 4]]

⍝ — Selfie with an array operand creates a constant function
2⍨3   ⍝ 2

⍝ —
2-⍨5   ⍝ 3

⍝ — Oversized windows have an empty frame
+/3↕1 2   ⍝ ⍬

⍝ — Reduce adjacent-row windows along their window axis
+/⍠1 (2↕2 3⍴⍳6ₓ)   ⍝ [3 5 7⋄]ₓ

⍝ —
-/⍬   ⍝ 0

⍝ —
÷/⍬   ⍝ 1

⍝ — Empty minimum reduction has positive infinity as identity
⌊/⍬   ⍝ ∞

⍝ — Empty Each calls a dfn operand on the zero prototype. When that call fails, the result is empty with prototype 0, as in J
{⍵÷'a'}¨⍬   ⍝ ⍬

⍝ — Dyadic empty Each does the same
1{⍺÷'a'}¨⍬   ⍝ ⍬

⍝ — Each with a pervasive function gives the same result as the function, here `÷⍬`
÷¨⍬   ⍝ ⍬

⍝ — Dyadic Each with a pervasive function gives the same result as `1÷⍬`
1÷¨⍬   ⍝ ⍬

⍝ — An array operand acts as a constant function
2¨3   ⍝ 2

⍝ —
f←2¨ ⋄ f 1 2 3   ⍝ 2 2 2

⍝ —
f←2↣3 ⋄ f 1   ⍝ 3

⍝ —
f←+⍥3 ⋄ f 10   ⍝ 3

⍝ — Prototype mode accepts negative Pick positions
¯1⊃¨0⍴⊂1 2 3   ⍝ ⍬

⍝ — Negative Pick positions are valid through a dfn in prototype mode
¯1{⍺⊃⍵}¨0⍴⊂1 2 3   ⍝ ⍬

⍝ — Outer and Inner product call their operands in prototype mode for an empty result, as Each does, and Inner also for an empty contraction
[⍬{5⊃⍵}⊗0⍴⊂1 2;(0 2⍴⊂1 2)+.{5⊃⍵}(2 3⍴⊂1 2);(2 0⍴⊂1 2)+.{5⊃⍵}(0 3⍴⊂1 2)]   ⍝ [0 0⍴0;0 3⍴0;2 3⍴0]

⍝ —
+/0↕5
⍝ error: RANK ERROR

⍝ — Prototype-mode partial Pick retains trailing axes
⍬⊃¨⊂2 3⍴⍳6ₓ   ⍝ (0⍴⊂0 0 0)ₓ

⍝ — Oversized windows have an empty frame
+/4↕1 2   ⍝ ⍬

⍝ — Oversized windows have an empty frame
+/3↕⍬   ⍝ ⍬

⍝ —
2+\1 2   ⍝ 3 5

⍝ — Empty maximum reduction has negative infinity as identity
⌈/⍬   ⍝ ¯∞

⍝ — Empty each makes one observable call to determine its prototype
r←{⎕←7 ⋄ ⍵}¨⍬
⍬
⍝ ⎕: 7

⍝ — Dyadic empty each passes the extended left value and right prototype
1{⎕←[⍺ ⍵] ⋄ 0}¨⍬
⍬
⍝ ⎕: 1 0

⍝ — A call under Each that gives no result contributes `⍬` beside the other results
{⍵=2?;⍵}¨1 2 3   ⍝ 1 ⍬ 3

⍝ — Pick in an empty Each's prototype call selects by structure, so a missing position gives no error. The call's output shows.
f←{⎕←7 ⋄ 100⊃"abc"} ⋄ ≢f¨⍬
0ₓ
⍝ ⎕: 7

⍝ — A failing prototype call's output still shows
{⎕←8 ⋄ 1÷'a'}¨⍬
⍬
⍝ ⎕: 8

⍝⍝ Each, Rank, stencil and outer product operands

⍝ — Each keeps a keyed frame's keys
{⍵×2}¨["a":1 "b":2]   ⍝ ["a":2 "b":4]

⍝ — Rank keeps the keys of its frame
{+/⍵}⍤1⊢"x" "y":[1 2⋄3 4]   ⍝ ["x":3 "y":7]

⍝ — Each pairs keyed arguments by key, whatever order each holds its keys in
["a":1 "b":2] {⍵×2}¨ ["b":10 "a":20]   ⍝ ["a":40 "b":20]

⍝ — Rank pairs keyed frames by key, whatever order each holds its keys in
["a":1 "b":2] {⍵×2}⍤0 ⊢ ["b":10 "a":20]   ⍝ ["a":40 "b":20]

⍝ — A shorter frame extends over the other argument's trailing frame axis
1 2 3 {⍺+⍵}¨ [10 20⋄30 40⋄50 60]   ⍝ [11 21⋄32 42⋄53 63]

⍝ — A frame axis of length 1 extends to the other argument's length
[1 2 3⋄] {⍺+⍵}¨ [10 20 30⋄40 50 60]   ⍝ [11 22 33⋄41 52 63]

⍝ — Each inside an Each operand maps each cell's items
{{⍵+1}¨⍵}¨[1 2;3 4 5]   ⍝ [2 3;4 5 6]

⍝ — A guard on the cell picks each cell's body
{⍵<0?0;⍵}¨¯1 2 ¯3   ⍝ 0 2 0

⍝ — A guard on a batched value runs each side once, on the cells that take it, so √ sees no negative cell and the kind stays real
{⍵<0?0;√⍵}¨¯4 9 ¯1 16   ⍝ 0 3 0 4

⍝ — A batched guard makes no fallback [batched]
(•explain "{⍵<0?0;√⍵}¨¯4 9 ¯1 16").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — A later guard in the same call splits a side's cells again
{⍵<0?0;⍵>10?10;⍵}¨¯5 3 20 7 ¯1   ⍝ 0 3 10 7 0

⍝ — Locals assigned before a batched guard reach both sides, each at its own cells
{x←⍵×2 ⋄ x>3?x;x+100}¨⍳6ₓ   ⍝ 100 102 4 6 8 10

⍝ — A batched predicate may give the numbers 0 and 1
{2|⍵?⍵×3;⍵÷2}¨⍳8ₓ   ⍝ 0 3 1 9 2 15 3 21

⍝ — A guard in a dfn that the operand calls splits the batched run's cells
{g←{⍵>2?⍵×10;⍵} ⋄ g ⍵}¨⍳5   ⍝ 0 1 2 30 40

⍝ — A batched guard in Rank
{(+/⍵)>4?+/⍵;0}⍤1⊢[3 2]⍴⍳6   ⍝ 0 5 9

⍝ — A batched guard in an outer product
+/(⍳3ₓ){⍺>⍵?⍺;⍵×10}⊗⍳3ₓ   ⍝ 30 31 24

⍝ — A batched guard in a stencil
{(+/⍵)>6?1;0}⌺3⊢⍳6ₓ   ⍝ 0 0 0 1 1 1

⍝ — A stencil over a keyed axis makes a batched try, because its windows have no keys [batched]
V←"aa" "bb" "cc":1 2 3 ⋄ (•explain "{x←•time 0 ⋄ +/⍵}⌺3 V").fallbacks.reason   ⍝ ["an effect"]

⍝ — A batched guard in an Each over keys
{⍵>20?⍵;-⍵}¨["a":10 "b":30 "c":5]   ⍝ ["a":¯10 "b":30 "c":¯5]

⍝ — Sides whose cells differ in kind fall back, with each cell's own result
{⍵>2?⍵>3;⍵}¨⍳5ₓ   ⍝ (0 1 2 $f $t)ₓ

⍝ — Sides whose cells differ in shape fall back, with each cell's own result
{⍵>2?⍵ ⍵;0}¨⍳5ₓ   ⍝ [0ₚ 0ₚ 0ₚ [3 3]ₓ [4 4]ₓ]

⍝ — A constant scalar that isn't an atom stays a scalar in each cell
{⍬⍴5}¨⍳3ₓ   ⍝ [⊂5 ⊂5 ⊂5]

⍝ — A batched guard in a recursive call falls back [batched]
(•explain "{⍵<2?⍵;(∇⍵-1)+∇⍵-2}¨⍳8ₓ").fallbacks.reason   ⍝ ["a recursive call"]

⍝ — A side that writes to a binding of an enclosing frame gives each cell's own result
{n←0 ⋄ g←{⍵>2?n+←1;0} ⋄ r←g ⍵ ⋄ n}¨⍳5ₓ   ⍝ 0 0 0 1 1

⍝ — A side that writes to a binding of an enclosing frame falls back, because the frame outlives the side [batched]
(•explain "{n←0 ⋄ g←{⍵>2?n+←1;0} ⋄ r←g ⍵ ⋄ n}¨⍳5ₓ").fallbacks.reason   ⍝ ["a write"]

⍝ — A random draw in a side draws for the cells that take it [batched]
e←•explain "{⍵>0?¿⍵;0}¨0 3 0 5" ⋄ [0=≢e.fallbacks.reason;∧/e.value<1⌈0 3 0 5]   ⍝ $t $t

⍝ — A dfn whose batched run stopped on a count from its cell skips the batched try on later calls [batched]
f←{2×⍣⍵⊢1} ⋄ r←f¨⍳5ₓ
(•explain "f¨⍳5ₓ").fallbacks.reason   ⍝ ["an earlier stop: a cell's value"]

⍝ — A dfn whose guard caught an error in some cells of a batched run batches again on later calls, because the stop depended on the cells' data [batched]
f←{"DOMAIN"::¯1 ⋄ ⍵○0.5} ⋄ r←f¨1 2 13
(•explain "f¨1 2 3").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — A dfn whose batched run stopped on a call with no rule for batches skips the batched try on later calls whose arguments have the same kinds and cell shapes [batched]
f←{⍋⍵} ⋄ r←f⍤1⊢2 3⍴⍳6ₓ
(•explain "f⍤1⊢2 3⍴⍳6ₓ").fallbacks.reason   ⍝ ["an earlier stop: no batch rule"]

⍝ — The same dfn tries a batched run again on cells of another shape [batched]
f←{⍋⍵} ⋄ r←f⍤1⊢2 3⍴⍳6ₓ
(•explain "f⍤1⊢2 4⍴⍳8ₓ").fallbacks.reason   ⍝ ["no batch rule for ⍋"]

⍝ — A system function that acts stops a batched run for an effect, even on a constant [batched]
(•explain "{x←•time 0 ⋄ ⍵}¨⍳3ₓ").fallbacks.reason   ⍝ ["an effect"]

⍝ — A system function that doesn't act runs once for each cell inside a batched run [batched]
(•explain "{•ucs ⍵}¨""abc""").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — So does one on a constant inside the operand [batched]
(•explain "{x←•ucs ""a"" ⋄ ⍵}¨⍳3ₓ").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — So does a system function's inverse [batched]
(•explain "{•json⁻¹ ⍵}¨1 2").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — Cells whose system-function results don't stack into one array fall back [batched]
(•explain "{•json⁻¹ ⍵}¨1 22").fallbacks.reason   ⍝ ["no batch rule"]

⍝ — Each cell then gets its own result
{•json⁻¹ ⍵}¨1 22   ⍝ "1.0" "22.0"

⍝ — ⍕ of a cell stops a batched run for an effect [batched]
(•explain "{⍕⍵}¨⍳3ₓ").fallbacks.reason   ⍝ ["an effect"]

⍝ — ⍕ of a strand that holds a cell stops a batched run for an effect too [batched]
(•explain "{⍕[⍵ 1]}¨⍳3ₓ").fallbacks.reason   ⍝ ["an effect"]

⍝ — A write to a name outside the map stops a batched run [batched]
n←0
(•explain "{n+←1 ⋄ ⍵}¨1 2").fallbacks.reason   ⍝ ["a write"]

⍝ — A guard that would catch an error stops a batched run [batched]
(•explain "{""DOMAIN""::¯1 ⋄ ⍵○0.5}¨1 2 13").fallbacks.reason   ⍝ ["a caught error"]

⍝ — A system function's inverse writes each cell's literal
{•literal⁻¹ ⍵}¨1 2 3   ⍝ "1ₚ" "2ₚ" "3ₚ"

⍝ — Each cell's literal has its own length
{≢•literal⁻¹ ⍵}¨1 22 333   ⍝ [2 3 4]ₓ

⍝ — The inverse of ⍎ writes each cell's literal too
{⍎⁻¹ ⍵}¨1 2 3   ⍝ "1ₚ" "2ₚ" "3ₚ"

⍝ — ⍎ of each cell's text stops the batched run, because each cell holds its own code [batched]
(•explain "{⍎⍵}⍤1⊢2 3⍴""1+1""").fallbacks.reason   ⍝ ["a cell's value"]

⍝ — Each cell's floor keeps its own kind: an infinity stays a float beside exact integers
{⌊⍵}¨1.5 ∞ 2.5   ⍝ 1ₓ ∞ 2ₓ

⍝ — A batched run takes square roots of exact integers, because every cell gives floats [batched]
(•explain "{√⍵}¨4ₓ 9ₓ").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — A strand of the cell's values gives each cell a vector
{⍵ (2×⍵)}¨1 2 3   ⍝ [1 2;2 4;3 6]

⍝ — Rank mixes each cell's strand into a row
{[⍵ 2×⍵]}⍤0⊢1 2 3   ⍝ [1 2⋄2 4⋄3 6]

⍝ — A fold over a strand of the cell's values
{+/[⍵ (2×⍵) 3]}¨1 2   ⍝ 6 9

⍝ — A result that doesn't depend on the cell is every cell's result
{7}¨1 2 3   ⍝ 7 7 7

⍝ — Rank mixes a result that doesn't depend on the cell into each row
{1 2}⍤0⊢"abc"   ⍝ [1 2⋄1 2⋄1 2]

⍝ — Each cell gives the same empty vector
{⍬}¨1 2   ⍝ ⍬ ⍬

⍝ — A power count that comes from the cell
{2×⍣⍵ 1}¨0 1 2 3   ⍝ 1 2 4 8

⍝ — An axis that comes from the cell
{+/⍠⍵⊢[1 2⋄3 4]}¨0 1   ⍝ [4 6;3 7]

⍝ — Each cell joins itself
{⍵,⍵}⍤1⊢[1 2⋄3 4]   ⍝ [1 2 1 2⋄3 4 3 4]

⍝ — Atoms from the same cell join into a vector
{⍵,2×⍵}¨1 2 3   ⍝ [1 2;2 4;3 6]

⍝ — A vector that every cell shares joins each cell
{0 0,⍵}⍤1⊢[1 2⋄3 4]   ⍝ [0 0 1 2⋄0 0 3 4]

⍝ — An atom joins each matrix cell as a column
{⍵,0}⍤2⊢[[1 2⋄3 4]⋄[5 6⋄7 8]]   ⍝ [[1 2 0⋄3 4 0]⋄[5 6 0⋄7 8 0]]

⍝ — Each cell's sum joins the cell, which has one more axis
{(+/⍵),⍵}⍤1⊢[1 2⋄3 4]   ⍝ [3 1 2⋄7 3 4]

⍝ — `⍪` joins each cell along its first axis
{⍵⍪0}⍤2⊢[[1 2⋄3 4]⋄[5 6⋄7 8]]   ⍝ [[1 2⋄3 4⋄0 0]⋄[5 6⋄7 8⋄0 0]]

⍝ — `⊣` gives its left argument, which every cell shares
{0⊣⍵}¨1 2 3   ⍝ 0 0 0

⍝ — A rotation count that comes from the cell
{⍵⌽1 2 3}¨0 1 2   ⍝ [1 2 3;2 3 1;3 1 2]

⍝ — Each cell's transpose
{⍉⍵}⍤2⊢[[1 2 3⋄4 5 6]⋄[7 8 9⋄10 11 12]]   ⍝ [[1 4⋄2 5⋄3 6]⋄[7 10⋄8 11⋄9 12]]

⍝ — Replicate along each cell's first axis
{1 0 2#⍵}⍤1⊢[1 2 3⋄4 5 6]   ⍝ [1 3 3⋄4 6 6]

⍝ — Decode each cell
{2⊥⍵}⍤1⊢[1 0 1⋄1 1 1]   ⍝ 5 7

⍝ — Rank joins each pair of cells in one call where both arguments have the same frame
[1 2⋄3 4] ,⍤1 [5 6⋄7 8]   ⍝ [1 2 5 6⋄3 4 7 8]

⍝ — A batched run joins and transposes cells without falling back [batched]
(•explain "{⍵,⍉⍵}⍤2⊢2 2 2⍴0.5").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — Each passes a scalar's item to every cell
1 2{⍺+≢⍵}¨⊂10 20   ⍝ 3 4

⍝ — A grade inside a batched run orders each cell's own items
{⍋⍤1 ⍵}⍤2⊢2 3 2⍴3 2 1 4 2 1 5 6 4 5 6 4   ⍝ [[1 0⋄0 1⋄1 0]⋄[0 1⋄0 1⋄1 0]]ₓ

⍝ — A batched run multiplies each cell's matrices as one stack
{⍵+.×⍵}⍤2⊢2 2 2⍴1 2 3 4 5 6 7 8   ⍝ [[7 10⋄15 22]⋄[67 78⋄91 106]]

⍝ — Rank applies or-and to each pair of a stack's Boolean matrices
b←[2 2 2]⍴1 0 0 1 1 1 0 1=1 ⋄ b∨.∧⍤2 b   ⍝ [[$t $f⋄$f $t]⋄[$t $t⋄$f $t]]

⍝ — Rank counts equal items for each pair of a stack's exact matrices
k←[2 2 3]⍴3 1 4 1 5 9 2 6 5 3 5 8ₓ ⋄ k+.=⍤2⍉⍤2 k   ⍝ [[3 0⋄0 3]⋄[3 0⋄0 3]]ₓ

⍝ — A stack of exact products that overflow 64 bits computes each matrix exactly
j←[2 2 2]⍴(2*40)ₓ ⋄ j+.×⍤2 j   ⍝ [2 2 2]⍴(2*81)ₓ

⍝ — A batched run takes each cell's determinant as one stack
{-.×⍵}⍤2⊢2 2 2⍴1 2 3 4 5 6 7 8   ⍝ ¯2 ¯2

⍝ — A batched run divides each cell by a matrix that every cell shares
{⍵⌹2 2⍴2 0 0 2}⍤2⊢2 2 2⍴2 4 6 8 2 2 2 2   ⍝ [[1 2⋄3 4]⋄[1 1⋄1 1]]

⍝ — A batched run stops where one cell's share of a large constant alone would hold too many items [batched]
x←2000000⍴0.5
(•explain "{+/x×⍵}¨1 2 3 4 5 6 7 8 9 10").fallbacks.reason   ⍝ ["too many items"]

⍝ — A batched run whose constants, repeated for every cell, would hold too many items runs on chunks of cells instead [batched]
x←⍳16384ₓ ⋄ c←⍳2048ₓ
(•explain "{+/(x-⍵)×x-⍵}¨c").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — Chunks of cells give what the run over cells gives, cell by cell
x←⍳16384ₓ ⋄ c←⍳2048ₓ ⋄ f←{+/(x-⍵)×x-⍵} ⋄ a←f¨c
•prefs ["batch":$f] ⋄ a≡f¨c   ⍝ $t

⍝ — A chunk whose later call repeats a larger constant starts the chunks over, with smaller chunks
x←⍳1024ₓ ⋄ y←⍳4096ₓ ⋄ c←⍳4096ₓ ⋄ f←{(+/y×⍵)++/x-⍵} ⋄ a←f¨c
•prefs ["batch":$f] ⋄ a≡f¨c   ⍝ $t

⍝ — Chunks draw the numbers that the run over cells draws
•prefs ["seed":7] ⋄ x←⍳4096ₓ ⋄ c←⍳1024ₓ ⋄ f←{(+/x-⍵)+¿100} ⋄ a←f¨c
•prefs ["seed":7 "batch":$f] ⋄ b←f¨c ⋄ a≡b   ⍝ $t

⍝ — A short constant, repeated for every cell, still batches [batched]
(•explain "{⍵×1 2 3 4 5}¨0.5+⍳1000ₓ").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — A count that comes from the cell inside a strand stops the batched run [batched]
(•explain "{[⍵ 1]↑2 2⍴⍳4ₓ}¨1 2").fallbacks.reason   ⍝ ["a cell's value"]

⍝ — A kernel that reads a number from the cell, which no rule for batches covers, stops the batched run [batched]
(•explain "{⊤[⍵ 1]}¨1 2").fallbacks.reason   ⍝ ["no batch rule"]

⍝ — Cells whose results differ in shape
{⍳⍵}¨1 2 3   ⍝ [[0];0 1;0 1 2]

⍝ — Each cell writes to an outer name in turn
a←0
{a+←⍵}¨1 2 3
a   ⍝ 6

⍝ — Each cell writes selectively to an outer array in turn
c←0 0
r←{([[0]]⌷c)+←1 ⋄ ⍵×2}¨⍳5ₓ
c   ⍝ 5 0

⍝ — Each cell appends in place to an outer array in turn
a←⍬
r←{a,←1 ⋄ ⍵×2}¨⍳5ₓ
a   ⍝ 1 1 1 1 1

⍝ — Each cell displays in turn
{⎕←⍵}¨1 2
1 2
⍝ ⎕: 1\n2

⍝ — A system function runs on each cell
{•json⁻¹ ⍵}¨1 2   ⍝ "1.0" "2.0"

⍝ — An error in any cell is the map's error
{⍵⊃10 20}¨0 5
⍝ error: INDEX ERROR

⍝ — A map inside `∂` makes one batched call: the batch rules take each call first, and their calls on whole arrays reach the `∂` rules
m←[2 3]⍴⍳6 ⋄ {+/{+/⍵×⍵}⍤1⊢⍵}∂ m   ⍝ [0 2 4⋄6 8 10]

⍝ — A map inside `∂` records no fallback [batched]
(•explain "{+/{+/⍵×⍵}⍤1⊢⍵}∂ [2 3]⍴⍳6ₓ").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — A stencil inside `∂` whose kernel is the parameter: each offset's gradient sums its windows' items
{k←⍵ ⋄ +/, {+/,k×⍵}⌺[3 3] 4 4⍴⍳16ₓ}∂ 3 3⍴0   ⍝ [45 66 54⋄84 120 96⋄81 114 90]

⍝ — A product of stacks of matrices inside `∂` takes the `∂` rule for `+.×` matrix by matrix
s←[2 2 2]⍴⍳8ₓ ⋄ {+/,⍵+.×⍤2⊢⍵}∂ s   ⍝ ([2 2 2]⍴3 7 5 9 19 23 21 25)ₓ

⍝ — A shared matrix's gradient sums its products with every matrix of the stack
s←[2 2 2]⍴⍳8ₓ ⋄ {M←⍵ ⋄ +/,s+.×⍤2⊢M}∂ [2 2]⍴1 0 0 1   ⍝ [12 12⋄16 16]

⍝ — A stack with a shorter frame pairs with the product's frame from the front, and its gradient sums over the frame axes it lacks
c←[2 3 2 2]⍴1 ⋄ {M←⍵ ⋄ +/,c+.×⍤2⊢M}∂ [2 2 2]⍴0   ⍝ [2 2 2]⍴6

⍝ — The same pairing holds when the differentiated stack is the product's left factor
c←[2 3 2 2]⍴⍳24ₓ ⋄ {+/,⍵+.×⍤2⊢c}∂ [2 2 2]⍴0.5   ⍝ [[27 39⋄27 39]⋄[99 111⋄99 111]]

⍝ — Determinants of a stack of matrices inside `∂` take the `∂` rule for `-.×` matrix by matrix
s←[2 2 2]⍴⍳8ₓ ⋄ {+/-.×⍤2⊢⍵}∂ s+0.5   ⍝ [2 2 2]⍴3.5 ¯2.5 ¯1.5 0.5 7.5 ¯6.5 ¯5.5 4.5

⍝ — Inverses of a stack of matrices inside `∂` take the `∂` rule for `⌹` matrix by matrix
s←[2 2 2]⍴⍳8ₓ ⋄ {+/,⌹⍤2⊢⍵}∂ s+0.5   ⍝ [2 2 2]⍴¯0.5 0.5 0.5 ¯0.5 ¯0.5 0.5 0.5 ¯0.5

⍝ — A stack of matrices inside `∂` records no fallback [batched]
(•explain "s←[2 2 2]⍴⍳8 ⋄ {+/,⍵+.×⍤2⊢⍵}∂ s").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — `∂` inside the operand differentiates each cell on its own
{{⍵×⍵}∂ ⍵}¨1 2 3   ⍝ 2 4 6

⍝ — `∂` with respect to each row under Rank differentiates each row on its own
{{+/⍵×⍵}∂ ⍵}⍤1⊢2 3⍴⍳6ₓ   ⍝ [0 2 4⋄6 8 10]ₓ

⍝ — `∂` with respect to a batch of rows makes one batched call, with no fallback [batched]
(•explain "{{+/⍵×⍵}∂ ⍵}⍤1⊢2 3⍴⍳6ₓ").fallbacks.reason   ⍝ 0⍴⊂""

⍝ — A Jacobian under Rank, one for each row
{{⍵*2 3}∂⍵}⍤1⊢[2 2]⍴1 2 3 4   ⍝ [2 2 2]⍴2 0 0 12 6 0 0 48

⍝ — A Jacobian under Rank with more results than numbers, one for each row
{{(⍵*2),+/⍵}∂⍵}⍤1⊢[2 2]⍴1 2 3 4   ⍝ [2 3 2]⍴2 0 0 4 1 1 6 0 0 8 1 1

⍝ — A nested `∂` under Rank gives each row's second derivatives
{{{⍵*3}∂⍵}∂⍵}⍤1⊢[2 2]⍴1 2 3 4   ⍝ [2 2 2 2]⍴6 0 0 0 0 0 0 12 18 0 0 0 0 0 0 24

⍝ — Each row is also the left argument of `∂`'s function
{⍵ {⍺×+/⍵*2}∂ ⍵}⍤1⊢[2 2]⍴1 2 3 4   ⍝ [2 2 2]⍴2 4 4 8 18 24 24 32

⍝ — A row catenated with a constant, over enough rows that the batched call's records grow with the rows
+/,{{+/(⍵,1)*2}∂⍵}⍤1⊢[8 2]⍴⍳16ₓ   ⍝ 240

⍝ — Reshape, replicate and index under `∂` in a map move each row's derivative with its items
m←[2 3]⍴⍳6ₓ ⋄ [{{+/,(2 2⍴⍵)*2}∂⍵}⍤1⊢m;{{+/(1 0 2#⍵)*2}∂⍵}⍤1⊢m;{{+/([[2 0 2]]⌷⍵)*2}∂⍵}⍤1⊢m]   ⍝ [[0 2 4⋄12 8 10] [0 0 8⋄6 0 20] [0 0 8⋄6 0 20]]

⍝ — A fold along an axis under `∂` in a map
{{+/,+/⍠0⊢2 2⍴⍵×⍵}∂⍵}⍤1⊢[2 4]⍴⍳8ₓ   ⍝ [0 2 4 6⋄8 10 12 14]ₓ

⍝ — An item read twice under `∂` in a map accumulates both derivatives
{{+/([[0 0 1]]⌷⍵)*2}∂⍵}⍤1⊢[2 3]⍴⍳6ₓ   ⍝ [0 2 0⋄12 8 0]

⍝ — An attached rule inside Each under an outer `∂` uses the rule, which here differs from the function's body
w←{⍵×2} ⋄ w∂←{⍺×10} ⋄ {+/w¨⍵}∂ 1 2 3   ⍝ 10 10 10

⍝ — A Jacobian with more results than numbers, and a row catenated with a constant, make one batched call with no fallback [batched]
m←[2 2]⍴1 2 3 4 ⋄ [(•explain "{{(⍵*2),+/⍵}∂⍵}⍤1⊢m").fallbacks.reason;(•explain "{{+/(⍵,1)*2}∂⍵}⍤1⊢m").fallbacks.reason]   ⍝ [0⍴⊂"" 0⍴⊂""]

⍝ — `∂` inside the operand refuses complex cells, as it refuses a complex argument outside a map
{{⍵×⍵}∂ ⍵}¨1j1 2j1
⍝ error: UNSUPPORTED

⍝ — `∂` inside the operand refuses character cells, as it refuses a character argument outside a map
{⊢∂⍵}¨"ab"
⍝ error: DOMAIN ERROR

⍝ — Dot access reads each cell's record
{⍵.a}¨[["a":1 "b":2];["a":3 "b":4]]   ⍝ 1 3

⍝ — Selective assignment changes each cell's local copy
{x←⍵ ⋄ x.[0]←9 ⋄ x}¨[1 2;3 4]   ⍝ [9 2;9 4]

⍝ — A loss mapped over the rows of `X` has the gradient of the same loss written with `+.×`
X←[3 2]⍴0.1×⍳6ₓ
loss←{th←⍵ ⋄ +/{⊼1○+/th×⍵}⍤1⊢X}
lossw←{+/⊼1○X+.×⍵}
(loss∂ 0.5 ¯0.25)≡lossw∂ 0.5 ¯0.25   ⍝ $t

⍝ — Per-example gradients under Rank match the closed form
X←[3 2]⍴0.1×⍳6ₓ
f←{+/⊼1○⍵}
({f∂ ⍵}⍤1⊢X)≡2×(1○X)×2○X   ⍝ $t

⍝ — A Hessian–vector product through the mapped loss matches the one through `+.×`
X←[3 2]⍴0.1×⍳6ₓ
loss←{th←⍵ ⋄ +/{⊼1○+/th×⍵}⍤1⊢X}
lossw←{+/⊼1○X+.×⍵}
hv←{+/1 2×loss∂ ⍵}
hvw←{+/1 2×lossw∂ ⍵}
(hv∂ 0.5 ¯0.25)≡hvw∂ 0.5 ¯0.25   ⍝ $t

⍝ — A stencil steps a Game of Life glider
life←{s←+/,⍵ ⋄ (s=3)∨(1 1⌷⍵)∧s=4}
g←[0 1 0 0⋄0 0 1 0⋄1 1 1 0⋄0 0 0 0]=1
life⌺3 3⊢g
⍝ =>
[0 0 0 0⋄1 0 1 0⋄0 1 1 0⋄0 1 0 0]=1

⍝ — An outer product's result has the left argument's axes, then the right's
[1 2⋄3 4] {⍺×⍵}⊗ 10 100   ⍝ 2 2 2⍴10 100 20 200 30 300 40 400

⍝ stage-fold-rows — A dfn folds each row from the right, with each stage called on every row at once
m←[3 4]⍴⍳12ₓ
{⍺-⍵}/m   ⍝ [¯2 ¯2 ¯2]ₓ

⍝ stage-fold-columns — A dfn folds each column from the bottom
m←[3 4]⍴⍳12ₓ
{⍺-⍵}⌿m   ⍝ [4 5 6 7]ₓ

⍝ stage-fold-seed — A seed is each row's right argument at the first stage
m←[3 4]⍴⍳12ₓ
100{⍺-⍵}/m   ⍝ 98 98 98

⍝ stage-scan-rows — A dfn scans each row from the left
m←[3 4]⍴⍳12ₓ
{⍺-⍵}\m
⍝ =>
[0 ¯1 ¯3 ¯6⋄4 ¯1 ¯7 ¯14⋄8 ¯1 ¯11 ¯22]ₓ

⍝ stage-scan-seed — A seeded dfn scan down the columns
m←[3 4]⍴⍳12ₓ
100{⍺-⍵}⍀m
⍝ =>
[100 99 98 97⋄96 94 92 90⋄88 85 82 79]

⍝ stage-rank — A dfn fold or scan inside Rank runs on each cell's row, stage by stage
m←[3 4]⍴⍳12ₓ
({{⍺-⍵}/⍵}⍤1⊢m) ({{⍺-⍵}\⍵}⍤1⊢m)
⍝ =>
[¯2 ¯2 ¯2;[0 ¯1 ¯3 ¯6⋄4 ¯1 ¯7 ¯14⋄8 ¯1 ¯11 ¯22]]ₓ

⍝ stage-matrix-cells — A dfn fold of each matrix cell folds every row of the cell
{{⍺-⍵}/⍵}⍤2⊢[2 3 4]⍴⍳24ₓ   ⍝ ([2 3]⍴¯2)ₓ

⍝ stage-seed-each — A seed that differs from cell to cell
m←[3 4]⍴⍳12ₓ
{⍵{⍺-⍵}/m}¨10 20   ⍝ [8 8 8;18 18 18]

⍝ stage-inner — A dfn on either side of an inner product
a←[2 3]⍴⍳6ₓ
b←[3 2]⍴⍳6ₓ
(a {⍺-⍵}.× b) (a +.{⍺-⍵} b)   ⍝ [[6 7⋄12 16]ₓ;[¯3 ¯6⋄6 3]ₓ]

⍝ stage-inner-vectors — A dfn inner product of two vectors is an atom
1 2 3 {⍺-⍵}.{⍺×⍵} 4 5 6   ⍝ 12

⍝ stage-array-results — A dfn fold that returns arrays gives each row's result as an item
{⍺ ⍵}/[2 3]⍴⍳6ₓ   ⍝ [[0 [1 2]];[3 [4 5]]]ₓ

⍝ stage-effect-order — A dfn with output folds row by row, in the order of calls without batching
{⎕←⍺ ⋄ ⍺-⍵}/[2 3]⍴⍳6ₓ   ⍝ [1 4]ₓ
⍝ ⎕: 1ₓ\n0ₓ\n4ₓ\n3ₓ

⍝⍝ Composition rank and dyadic operators

⍝ — A named Atop operator derives a sum-of-iota function
c←∘
sum←+/c⍳
sum¨2 4 6   ⍝ 1 6 15

⍝ — Dyadic Before reshapes the right argument to the left argument's shape
"abc"⍴↣⍴'z'   ⍝ "zzz"

⍝ — Before combines matrix rows as imaginary and real components
0j1↣×↣+⌿[1 2 3⋄4 5 6]   ⍝ 4j1 5j2 6j3

⍝ — Naming Before and reduction preserves their binding
b←↣
r←⌿
0j1 b × b + r [1 2 3⋄4 5 6]
⍝ =>
4j1 5j2 6j3

⍝ — After chains bind before reduction derives its function
-↢+↢×/1 2 3   ⍝ 0

⍝ — Rank assembles differently sized iota cells with padding
⍳⍤0 [1 3 2]   ⍝ [0 0 0⋄0 1 2⋄0 1 0]

⍝ — Empty rank application uses an operand prototype call to determine cell shape
{⍳3ₓ}⍤1 [0 2]⍴0   ⍝ (0 3⍴0)ₓ

⍝ — A dyadic defined operator can take two array operands
op←{⍶+⍹×⍵} ⋄ (2 op 3)4   ⍝ 14

⍝ — A dyadic defined operator can compose two function operands
op←{⍶ ⍹ ⍵} ⋄ (+/op⍳)4   ⍝ 6

⍝ — Operator operands retain access to their active lexical frame
f←{k←3 ⋄ g←{k+⍵} ⋄ op←{⍶ ⍹ ⍵} ⋄ (+ op g)⍵} ⋄ f4   ⍝ 7

⍝ —
+⍤⍬(3)
⍝ error: LENGTH ERROR

⍝ —
+⍤0.5(3)
⍝ error: DOMAIN ERROR

⍝ — Rank ∞ gives the operand the whole argument
m←[1 2 3⋄4 5 6] ⋄ m (+⌿×)⍤1 ∞ [1 2 3]   ⍝ 14 32

⍝ — With ranks 1 ∞, the matrix-vector product also multiplies matrices
m←[1 2 3⋄4 5 6] ⋄ m (+⌿×)⍤1 ∞ [1 10⋄2 20⋄3 30]   ⍝ [14 140⋄32 320]

⍝ — Rank ∞ clamps to the argument's rank, and ¯∞ to rank 0
m←[1 2⋄3 4] ⋄ [(⊂⍤∞ m) ≡ ⊂m;(⊂⍤¯∞ m) ≡ ⊂⍤0 m]   ⍝ $t $t

⍝ — Empty rank application exposes its single prototype call
{⎕←⍵ ⋄ ⍳3ₓ}⍤0⍬
(0 3⍴0)ₓ
⍝ ⎕: 0

⍝ — Over transforms the right argument before the left
f←{⎕←⍵ ⋄ ⍵} ⋄ 2 +⍥f 3
5
⍝ ⎕: 3\n2

⍝⍝ Key and power

⍝ — Key groups right-hand values using the left-hand labels
1 1 2{+/⍵}⌸10 20 30   ⍝ 30 30

⍝ — Empty Key retains the operand's result-cell shape
{⍳3ₓ}⌸⍬   ⍝ (0 3⍴0)ₓ

⍝ — An empty Key whose operand fails on the prototypes gives an empty result, as Each's operand does
{÷0}⌸⍬   ⍝ ⍬

⍝ — Predicate power tests the next iterate after each step
1 +⍣{⍺>4} 0   ⍝ 5

⍝ — Power accepts a singleton-vector stopping result
(+↢1)⍣{,⍺=3}0   ⍝ 3

⍝ — A stopping result may have any rank but must contain one Boolean
(+↢1)⍣{1 1⍴⍺=3}0   ⍝ 3

⍝ —
2↣×⍣0⊢3   ⍝ 3

⍝ — Tolerant Key puts a chain of matches in one group
{≢⍵}⌸[1 1+8E¯15 1+16E¯15]   ⍝ [3]ₓ

⍝ —
{[⍺ ⍵]}⌸7
⍝ error: RANK ERROR

⍝ —
1 2{⍵}⌸3 4 5
⍝ error: LENGTH ERROR

⍝ —
(+↢1)⍣{1 1}0
⍝ error: LENGTH ERROR

⍝ —
(+↢1)⍣{⍬}0
⍝ error: LENGTH ERROR

⍝ — Empty Key calls the operand with an empty group retaining the value-cell shape
r←⍬ {⎕←⍴⍵ ⋄ ⍳3ₓ}⌸ 0 2⍴0
(0 3⍴0)ₓ
⍝ ⎕: [0 2]ₓ

⍝ — Zero iterations never call the operand
{⎕←7 ⋄ ⍵}⍣0⊢3
3
⍝ ⎕:

⍝ — Power propagates an operand's no-result return
{}⍣1⊢3   ⍝ {}0

⍝⍝ Products

⍝ — Inner product may reduce with catenate rather than a pervasive function
1 2 3,.-3 3⍴4 5 6   ⍝ [[¯3 ¯2 ¯1] [¯4 ¯3 ¯2] [¯5 ¯4 ¯3]]

⍝ — This fork enumerates leading-axis indices and selects each major cell
⍳∘≢⌷⊗⊂ 2 3 3⍴⍳18ₓ
[[0 1 2⋄3 4 5⋄6 7 8] [9 10 11⋄12 13 14⋄15 16 17]]ₓ

⍝ —
(2 3⍴⍳6ₓ)+.×3 2⍴⍳6ₓ   ⍝ [10 13⋄28 40]ₓ

⍝ — Native inner products keep right-fold order and comparisons can be either operand
a←[1 2 4⋄8 16 32] ⋄ b←[2 3⋄5 7⋄11 13]
[(a-.×b)≡a{⍺-⍵}.{⍺×⍵}b (a<.+b)≡a{⍺<⍵}.{⍺+⍵}b (a+.>b)≡a{⍺+⍵}.{⍺>⍵}b]
⍝ =>
$t $t $t

⍝ — Contractions carry the right tail across chunks, keep singleton kinds and widen exact overflow
a←⍳4103ₓ ⋄ b←4103⍴1
[(a-.×b)≡a{⍺-⍵}.{⍺×⍵}b ([9223372036854775807 9223372036854775807]ₓ)+.×[2 2]ₓ (•storage ((,2)+.>(,1))).kind]
⍝ =>
[$t 36893488147419103228ₓ "bool"]

⍝ — An empty contraction fills every result cell with the reduction identity
(2 0⍴0)+.×0 3⍴0   ⍝ 2 3⍴0

⍝ — Singleton extension also applies along the contracted axis
(,2)+.×1 2 3   ⍝ 12

⍝ — A small product and the same product inside a large one agree, a NaN term included, though rustymath computes the small one by a direct loop and the large one on its tiled kernels
v←⍳16 ⋄ v.[0]←$n ⋄ a←4 4⍴v ⋄ B←64 64↑b←4 4⍴⍳16 ⋄ pad←{(⍺,4 60⍴⍵)⍪60 64⍴⍵} ⋄ hi←[4 4]↑(a pad ¯∞)⌈.+B ⋄ lo←[4 4]↑(a pad ∞)⌊.+B ⋄ [(a⌈.+b)≡hi;(a⌊.+b)≡lo]   ⍝ $t $t

⍝ — Named outer-product operator
outer←⊗
times←×
1 2 times outer 3 4   ⍝ [3 4⋄6 8]

⍝ — Parenthesized outer-product operator
1 2 +(⊗) 10 20   ⍝ [11 21⋄12 22]

⍝ — Outer product accepts composed operands
1 2 (-∘+)⊗ 10 20   ⍝ [¯11 ¯21⋄¯12 ¯22]

⍝ — Commute follows outer-product derivation
-⊗⍨1 2   ⍝ [0 ¯1⋄1 0]

⍝ — Empty outer product preserves both argument frames
⍬ +⊗ 7 8   ⍝ 0 2⍴0

⍝ —
1 2+.×1 2 3
⍝ error: LENGTH ERROR

⍝ — Empty outer product makes one prototype call using the first right item
r←⍬ {⎕←[⍺ ⍵] ⋄ ⍺+⍵}⊗ 7 8
0 2⍴0
⍝ ⎕: 0 7

⍝ — Empty inner-product frames still derive fill through the real operand path
r←(0 2⍴0)+.{⎕←[⍺ ⍵] ⋄ ⍺×⍵}2 3⍴⍳6ₓ
0 3⍴0ₚ
⍝ ⎕: 0 0ₓ\n0 1ₓ\n0 2ₓ\n0 3ₓ\n0 4ₓ\n0 5ₓ

⍝⍝ Complex arithmetic and roundtrips

⍝ — Empty complex product has multiplicative identity one. https://docs.dyalog.com/20.0/programming-reference-guide/introduction/complex-numbers/
×/0#1j2   ⍝ 1

⍝ — Iota accepts a complex representation only when its imaginary part is zero
⍳3j0   ⍝ 0 1 2

⍝ — Complex literals, with exponents and the uppercase J alias
[1j2 1J2 ¯.5j2E¯1 1E2j¯4E¯1 1j¯0 ¯0j2]   ⍝ 1j2 1j2 ¯0.5j0.2 100j¯0.4 1 0j2

⍝ — Complex arithmetic
[1j2+3j4 1j2-3j4 1j2×3j4 1j2×1j¯2 1j2÷3j4 +1j2 -1j2 ×3j4 ÷1j2]
4j6 ¯2j¯2 ¯5j10 5 0.44j0.08 1j¯2 ¯1j¯2 0.6j0.8 0.2j¯0.4

⍝ — An exact number combines with a complex number as a float
[1r2+1j2 1j2-1r2 2ₓ÷0j1 1j2÷2ₓ]   ⍝ 1.5j2 0.5j2 0j¯2 0.5j1

⍝ — Reductions and dfns use the same complex arithmetic
sum←+/
f←{⍵×+⍵}
[sum 1j2 3j4;-/1j2 3j4 5j6;f 3j4]
⍝ =>
4j6 3j4 25

⍝ — A complex number with a zero imaginary part displays as a float
⍕¨[1j2×1j¯2 ¯0j2 1j2+3j4 +1j2]   ⍝ "5" "¯0ⱼ2" "4ⱼ6" "1ⱼ¯2"

⍝ — Written items keep their own kinds
{(•storage ⍵).kind}¨1ₓ 0.5 1j2   ⍝ "integer" "float64" "complex128"

⍝ — Empty complex results have a float fill
[↑0#1j2 ↑1j2+0#1ₓ +/0#1j2]   ⍝ 0 0 0

⍝ — A complex result with no imaginary part in any item has float storage
{(•storage ⍵).kind}¨[1j2 2×1j¯2 2;1j2 2×1j2 2]   ⍝ "float64" "complex128"

⍝ — A real part too small to change 1 still changes the smaller part of √(1+z²), √(1-z²) and √(z²-1). Complex-step differentiation reads 4○'s derivative at 0.4, 0.4÷√1.16, from its imaginary part
1e200×[(∨4○0.4j1e¯200)₁ (∨0○1e¯200j0.4)₁ (∨(4○⁻¹)1e¯200j0.4)₀]   ⍝ 0.3713906763541037 ¯0.3713906763541037 0.3713906763541037

⍝ — Reals whose results are complex for some items give what each item gives alone
[⍟¯1 2.5;2√¯4 9.5;1○⁻¹2 0.5]≡[(⍟¯1),⍟2.5 (2√¯4),2√9.5 (1○⁻¹2),1○⁻¹0.5]   ⍝ $t

⍝ — Complex storage holds infinities as reals, and an infinity equals only itself
[1j2 2 ∞ ∞=5 2 5 ∞;1j2 ∞=∞ 1j2]   ⍝ [[$f $t $f $t];[$f $f]]

⍝⍝ Complex comparison errors and recovery

⍝ — A complex literal requires an imaginary component
1ⱼ
⍝ error: SYNTAX ERROR

⍝ —
1j¯
⍝ error: SYNTAX ERROR

⍝ —
1j2E¯
⍝ error: SYNTAX ERROR

⍝ — Exact suffixes cannot be attached to a complex component
1j2ₓ
⍝ error: SYNTAX ERROR

⍝ — Complex components use approximate literals, not exact integers
1ₓj2
⍝ error: SYNTAX ERROR

⍝ — Complex components cannot use rational-literal syntax
1r2j3
⍝ error: SYNTAX ERROR

⍝ —
1j2j3
⍝ error: SYNTAX ERROR

⍝ — Complex parts can be infinite
[1j1E309 1E309j1 1E308j1E308+1E308j1E308]   ⍝ 1j∞ ∞j1 ∞j∞

⍝ —
÷0j0   ⍝ ∞

⍝ — Counts require exactly zero imaginary part, without tolerance
⍳1j1E¯15
⍝ error: DOMAIN ERROR

⍝ —
1j2#3
⍝ error: DOMAIN ERROR

⍝ — Complex numbers have no pervasive ordering, even against themselves
1j2<1j2
⍝ error: DOMAIN ERROR

⍝ —
1j2≤2
⍝ error: DOMAIN ERROR

⍝ —
2>1j2
⍝ error: DOMAIN ERROR

⍝ —
2≥1j2
⍝ error: DOMAIN ERROR

⍝ — Complex equality compares the magnitude of the difference within tolerance, in either order
x←1j1 1j1 1ₓ 1 0 1.7E308j1.7E308 1.7E308j1.7E308
y←1j1.000000000000012 1j1.00000000000002 1j5E¯15 1j5E¯14 0j1E¯100 1.7E308j1.7E308 ¯1.7E308j¯1.7E308
[x=y y=x x≠y y≠x]
⍝ =>
[[$t $f $t $f $f $t $f] [$t $f $t $f $f $t $f] [$f $t $f $t $t $f $t] [$f $t $f $t $t $f $t]]

⍝ — Output before a complex error still shows
⎕←1j2 ⋄ 1j2<0
⍝ error: DOMAIN ERROR
⍝ ⎕: 1ⱼ2

⍝⍝ Exact literals arithmetic and roundtrips

⍝ — Brackets marked ₓ read each integer's digits exactly
[9223372036854775809 1]ₓ   ⍝ 9223372036854775809ₓ 1ₓ

⍝ — A mark on inner brackets covers only them
[[1 2]ₓ 3]   ⍝ [1ₓ 2ₓ;3]

⍝ — Marked brackets need whole numbers
[1.5 2]ₓ
⍝ error: DOMAIN ERROR

⍝ — After brackets that hold anything but literals, ₓ is ⌊
x←1.5 ⋄ [x 2]ₓ   ⍝ [1 2]ₓ

⍝ — Empty marked brackets are the exact empty vector
1↑[]ₓ   ⍝ [0]ₓ

⍝ — A closing bracket inside a string doesn't end marked brackets
[1 "]" 2]ₓ   ⍝ [1ₓ;"]";2ₓ]

⍝ — ⍕ writes an exact vector in bracket form
⍕[1 ¯2]ₓ   ⍝ "[1 ¯2]ₓ"

⍝ — An empty format spec, ⍬ or '', gives ⍕'s text without the markers of numbers' kinds
[⍬⍕[1 ¯2]ₓ;''⍕2 2⍴⍳4ₓ;⍬⍕"ab" 3ₓ;⍬⍕["a":1ₓ];⍬⍕2 1⍴32•float 1.5 2.5]
⍝ =>
["1 ¯2" ["0 1"⋄"2 3"] " ab  3" "[""a"":1]" ["1.5"⋄"2.5"]]

⍝ — ₓ after rows marks every number in them exact
(0 1⋄2 3)ₓ   ⍝ [[0 1]ₓ [2 3]ₓ]

⍝ — One approximate operand makes division approximate, even when the other is exact
1ₓ÷2   ⍝ 0.5

⍝ —
1r4+0.5   ⍝ 0.75

⍝ —
1r0
⍝ error: DOMAIN ERROR

⍝ —
0r0
⍝ error: DOMAIN ERROR

⍝ — Exact division by zero gives an infinity, and zero divided by zero gives NaN
[1ₓ÷0ₓ ÷0ₓ ¯1ₓ÷0ₓ 0ₓ÷0ₓ]   ⍝ [∞ ∞ ¯∞ $n]

⍝ —
1ᵣ
⍝ error: SYNTAX ERROR

⍝ —
1r¯
⍝ error: SYNTAX ERROR

⍝ — Exact-number suffixes require integer syntax, not a decimal literal
1.5ₓ
⍝ error: SYNTAX ERROR

⍝ — Exact-number suffixes do not accept exponent notation
1E2ₓ
⍝ error: SYNTAX ERROR

⍝ —
1r2.5
⍝ error: SYNTAX ERROR

⍝ —
1r2ₓ
⍝ error: SYNTAX ERROR

⍝ —
1ₓ2
⍝ error: SYNTAX ERROR

⍝ — Exact literals and arithmetic normalise rationals
[42ₓ 42r1 2r4 1r¯2 ¯1r¯2 ¯0ₓ 1ₓ÷3ₓ 1r3+1r6 6ₓ÷3ₓ 1r3-1r6 2r3×3r4 +2r3 -2r3 ×¯2r3 ×0ₓ ÷2r3 ÷¯2r3 2r3÷¯1r3]
[42 42 1r2 ¯1r2 1r2 0 1r3 1r2 2 1r6 1r2 2r3 ¯2r3 ¯1 0 3r2 ¯3r2 ¯2]ₓ

⍝ — Reductions and dfns keep exact values exact, and exact integers subtract exactly beyond 2*53
sum←+/
add←{⍺+⍵}
[+/1r3 1r6;sum 1r3 1r6;add/1r3 1r6;-/[1 2 3]ₓ;9007199254740993ₓ-9007199254740992ₓ]
⍝ =>
[1r2 1r2 1r2 2 1]ₓ

⍝ — Display text reads back as the same exact number
⍎¨⍕¨[42r1 2r4 1r¯2 ¯0ₓ ÷¯2r3]   ⍝ [42 1r2 ¯1r2 0 ¯3r2]ₓ

⍝ — A whole rational displays as an exact integer
⍕42r1   ⍝ "42ₓ"

⍝ — An approximate number makes the result approximate
{(•storage ⍵).kind}¨[1r3+1÷6 1÷6+0 1ₓ÷2 1÷2ₓ]   ⍝ "float64" "float64" "float64" "float64"

⍝⍝ Exact arrays prototypes and counts

⍝ — Empty exact sum retains an exact additive identity
+/0#1r3   ⍝ 0ₓ

⍝ — Empty exact product retains an exact multiplicative identity
×/0#1r3   ⍝ 1ₓ

⍝ — Shape returns exact dimensions even for approximate data
⍴2 3⍴0.5   ⍝ [2 3]ₓ

⍝ — An atom's shape is an empty exact vector
⍴42   ⍝ ⍬ₓ

⍝ — Tally counts rows, so a zero-row matrix has tally zero
≢0 3⍴0.5   ⍝ 0ₓ

⍝ — Nested element shapes do not affect the outer shape
⍴[1r3 [1.5 2]]   ⍝ [2]ₓ

⍝ — Iota follows an exact argument's numeric domain
⍳3ₓ   ⍝ [0 1 2]ₓ

⍝ —
⍳3r2
⍝ error: DOMAIN ERROR

⍝ — A negative exact length counts down
⍳¯3ₓ   ⍝ [2 1 0]ₓ

⍝ —
⍳1000000001ₓ
⍝ error: LIMIT ERROR

⍝ —
⍳999999999999999999999ₓ
⍝ error: LIMIT ERROR

⍝ —
1r2#3
⍝ error: DOMAIN ERROR

⍝ — Written items keep their own kinds, including a big exact integer
{(•storage ⍵).kind}¨9007199254740993ₓ 0.5 1r3   ⍝ "integer" "float64" "rational"

⍝ — An empty result of exact numbers has an exact fill
[0ₓ#1r3 1ₓ+0#1r3]   ⍝ [⍬ₓ;⍬ₓ]

⍝ — An approximate number gives an approximate fill
↑1+0#1r3   ⍝ 0

⍝ — Replicate copies a rational
2ₓ#1r3   ⍝ 1r3 1r3

⍝⍝ Range

⍝ — A range counts up from its start to its end
2↦5   ⍝ 2 3 4 5

⍝ — A lower end counts down
5↦2   ⍝ 5 4 3 2

⍝ — Equal ends give one item
3↦3   ⍝ [3]

⍝ — Steps of 1 stop before passing an end that isn't whole
0.5↦3   ⍝ 0.5 1.5 2.5

⍝ — The number of steps is whole within tolerance
0↦0.3÷0.1   ⍝ 0 1 2 3

⍝ — Each item is the start plus an exact count, so an exact start gives exact items
(•storage 1ₓ↦5).kind   ⍝ "integer"

⍝ — An approximate start gives floats whatever the end
(•storage 1↦5ₓ).kind   ⍝ "float64"

⍝ — Characters count through code points
'e'↦'a'   ⍝ "edcba"

⍝ — Monadic range counts from 1 to its argument
↦4ₓ   ⍝ [1 2 3 4]ₓ

⍝ — A negative argument counts down from ¯1
↦¯3   ⍝ ¯1 ¯2 ¯3

⍝ — Zero gives an empty range
↦0   ⍝ ⍬

⍝ — Each item of a vector gives a range, and shorter ranges are padded
↦2 3   ⍝ [1 2 0⋄1 2 3]

⍝ — A range can't mix a character and a number
'a'↦5
⍝ error: DOMAIN ERROR

⍝ — A range needs a finite end
1↦∞
⍝ error: DOMAIN ERROR

⍝⍝ Exact and tolerant comparisons

⍝ — Approximate equality uses relative tolerance 1E¯14. https://docs.dyalog.com/20.0/language-reference-guide/system-functions/ct/
0.3=[0.3 0.1+0.2 0.4]   ⍝ $t $t $f

⍝ —
1 2≤2 1   ⍝ $t $f

⍝ — Tolerant equality is not transitive: x=y and y=z need not imply x=z
[1=1+8E¯15 (1+8E¯15)=1+16E¯15 1=1+16E¯15]   ⍝ $t $t $f

⍝ — Comparisons apply tolerance only when an approximate number takes part
x←[0.3 0.1+0.2 ¯0.3 1 1 1E8 1E8 0 1E¯100 1E308 1r3 1r3 9007199254740993ₓ 1ₓ 1 1r3]
y←[0.1+0.2 0.3 ¯0.1-0.2 1+5E¯15 1+5E¯14 1E8+5E¯7 1E8+5E¯5 1E¯100 0 ¯1E308 2r6 1r2 9007199254740992ₓ 1000000000000001r1000000000000000 1000000000000001r1000000000000000 1÷3]
[x=y⋄x≠y⋄x<y⋄x≤y⋄x>y⋄x≥y]
⍝ =>
[$t $t $t $t $f $t $f $f $f $f $t $f $f $f $t $t
 $f $f $f $f $t $f $t $t $t $t $f $t $t $t $f $f
 $f $f $f $f $t $f $t $t $f $f $f $t $f $t $f $f
 $t $t $t $t $t $t $t $t $f $f $t $t $f $t $t $t
 $f $f $f $f $f $f $f $f $t $t $f $f $t $f $f $f
 $t $t $t $t $f $t $f $f $t $t $t $f $t $f $t $t]

⍝ — An empty exact comparison has an exact fill
↑1ₓ=0#1ₓ   ⍝ $f

⍝⍝ Errors and evaluation order

⍝ — Output produced before an error is retained
⎕←7 ⋄ 1÷'a'
⍝ error: DOMAIN ERROR
⍝ ⎕: 7

⍝ — The right argument prints before the left argument
(⎕←1)+(⎕←2)
3
⍝ ⎕: 2\n1

⍝ —
⍳1.5
⍝ error: DOMAIN ERROR

⍝ — Iota respects the generated-element limit
⍳1000000001ₓ
⍝ error: LIMIT ERROR

⍝ —
missing
⍝ error: VALUE ERROR

⍝ — An arbitrary dfn has no declared empty-reduction identity
{⍺-⍵}/⍳0ₓ
⍝ error: DOMAIN ERROR

⍝ — Division by zero follows IEEE
[1÷0 ÷0 ¯1÷0]   ⍝ ∞ ∞ ¯∞

⍝ —
1ₑ
⍝ error: SYNTAX ERROR

⍝ — A negative exponent uses high minus, not the subtraction glyph
1ₑ-2
⍝ error: SYNTAX ERROR

⍝ —
¯
⍝ error: SYNTAX ERROR

⍝ — A left argument with no right argument makes a left section
(2+)3   ⍝ 5

⍝ —
)
⍝ error: SYNTAX ERROR

⍝ —
(2+3
⍝ error: SYNTAX ERROR

⍝ —
1 2+3 4 5
⍝ error: LENGTH ERROR

⍝ — A negative length counts down
⍳¯2ₓ   ⍝ [1 0]ₓ

⍝ —
2+2   ⍝ 4.0

⍝⍝ Operator categories and singleton replicate

⍝ — A right-operand reference classifies a definition as a dyadic operator
op←{⍹ ⍵} ⋄ (+op-)3   ⍝ ¯3

⍝ — Operand references in a nested definition do not classify the outer dfn
outer←{inner←{⍹ ⍵} ⋄ 1} ⋄ outer0   ⍝ 1

⍝ — Dyalog 20: operators acquire their function operand before binding the next operator.
+⍨¨/1 2 3   ⍝ 6

⍝ — Commute changes subtraction's scan recurrence to next minus accumulator
-⍨\1 2 3   ⍝ 1 1 2

⍝ — Named operators bind in the same order as their glyphs
each←¨
fold←/
+⍨ each fold 1 2 3   ⍝ 6

⍝ — Each and reduction bind to self-reshape, producing repeated-dimensional cells
⍴⍨¨/3#⊂⍳4ₓ   ⍝ [⍬⍴0;,1;2 2⍴2;3 3 3⍴3]ₓ

⍝⍝ Singleton agreement and empty counts

⍝ — Shared Dyalog cases; additional leading/unit-axis broadcasting is tested separately.
(,2)+3 4   ⍝ 5 6

⍝ —
3 4+,2   ⍝ 5 6

⍝ — A number plus a singleton preserves the vector shape
2+,3   ⍝ ,5

⍝ — A singleton count extends over an empty argument without creating items
(,2)#⍳0ₓ   ⍝ ⍬ₓ

⍝ — Empty counts replicate a singleton into an empty result
(⍳0ₓ)#,3   ⍝ ⍬

⍝ —
(,2)+⍳0ₓ   ⍝ ⍬

⍝ — A large valid count is harmless when the result contains no elements
1000001#⍳0ₓ   ⍝ ⍬ₓ

⍝ — Replicate validates count integrality even with an empty right argument
0.5#⍳0ₓ
⍝ error: DOMAIN ERROR

⍝ —
2 3#1 2 3
⍝ error: LENGTH ERROR

⍝ —
1000000001#1
⍝ error: LIMIT ERROR

⍝ —
99999999999999999999ₓ#1
⍝ error: LIMIT ERROR

⍝⍝ Real infinities

⍝ —
∞   ⍝ ∞

⍝ —
∞+3   ⍝ ∞

⍝ —
3÷∞   ⍝ 0

⍝ —
÷¯∞   ⍝ 0

⍝ — Signum of an infinity is an exact integer
×∞ ¯∞   ⍝ [1 ¯1]ₓ

⍝ — Approximate reduction overflow produces real infinity
+/1E308 1E308   ⍝ ∞

⍝ —
×/1E308 1E308   ⍝ ∞

⍝ —
1E308×2 3   ⍝ ∞ ∞

⍝ — An overflowing real literal is infinity rather than a parse error
1E999   ⍝ ∞

⍝ —
*1000   ⍝ ∞

⍝ —
⍟∞   ⍝ ∞

⍝ —
*¯∞   ⍝ 0

⍝ —
2*∞   ⍝ ∞

⍝ —
!171   ⍝ ∞

⍝ —
⌊∞ ¯∞   ⍝ ∞ ¯∞

⍝ —
|¯∞   ⍝ ∞

⍝ — Empty exact minimum still needs the non-rational identity infinity
⌊/0⍴0ₓ   ⍝ ∞

⍝ — Empty exact maximum uses negative infinity
⌈/0⍴0ₓ   ⍝ ¯∞

⍝ — Minimum retains a finite exact value even beyond float range
∞⌊10ₓ*1000ₓ   ⍝ 10ₓ*1000ₓ

⍝ — Minimum against infinity keeps an exact integer exact
3ₓ⌊∞   ⍝ 3ₓ

⍝ — Maximum against negative infinity preserves an exact fraction
¯∞⌈1r3   ⍝ 1r3

⍝ —
∞=∞ ¯∞ 1   ⍝ $t $f $f

⍝ — Infinity exceeds any finite exact integer without converting that integer to float
∞>10ₓ*1000ₓ   ⍝ $t

⍝ — Reversing comparison arguments retains the finite-versus-infinite distinction
(10ₓ*1000ₓ)<∞   ⍝ $t

⍝ —
∞=1j2   ⍝ $f

⍝ —
∞∊1 2 ∞   ⍝ $t

⍝ —
∞ ¯∞⍳¯∞ ∞ 0   ⍝ [1 0 2]ₓ

⍝ — Grade orders a huge finite integer between the two infinities
⍋[∞ 10ₓ*1000ₓ ¯∞]   ⍝ [2 1 0]ₓ

⍝ —
∪∞ ∞ ¯∞   ⍝ ∞ ¯∞

⍝ —
⍕∞ ¯∞   ⍝ "∞ ¯∞"

⍝ — Format and execute round-trip both real infinities
⍎⍕∞ ¯∞   ⍝ ∞ ¯∞

⍝ — Exponential formatting keeps the infinity symbol within the field
3 ¯2⍕∞   ⍝ "∞  "

⍝ —
3 2⍕¯∞   ⍝ " ¯∞"

⍝ —
(10ₓ*1000ₓ)+∞   ⍝ ∞

⍝ — A huge exact numerator divided by infinity is still zero
(10ₓ*1000ₓ)÷∞   ⍝ 0

⍝ — Subtracting a huge finite integer from infinity must not become infinity minus infinity
∞-(10ₓ*1000ₓ)   ⍝ ∞

⍝ — Division by a huge negative finite integer retains negative infinity
∞÷(¯10ₓ*1001ₓ)   ⍝ ¯∞

⍝ — Hyperbolic tangent has limit one at positive infinity
7○∞   ⍝ 1

⍝ — Magnitude, real part and imaginary part of negative real infinity
(|¯∞),∨¯∞   ⍝ ∞ ¯∞ 0

⍝ — Principal Lambert W has limit infinity
×↢*⍨⁻¹∞   ⍝ ∞

⍝ — Indeterminate real arithmetic gives NaN
[0÷0 ∞-∞ 0×∞ ∞×0ₓ ∞÷∞]   ⍝ 5⍴$n

⍝ — Complex numbers can have infinite parts. A number with no imaginary part is real
[∞+1j2 1j∞ ∞j0]   ⍝ ∞j2 1j∞ ∞

⍝ —
⍳∞
⍝ error: DOMAIN ERROR

⍝ —
∞⍴1
⍝ error: DOMAIN ERROR

⍝ —
∞#1
⍝ error: DOMAIN ERROR

⍝ —
¿∞
⍝ error: DOMAIN ERROR

⍝ —
1○∞   ⍝ $n

⍝ — An infinite matrix entry gives NaN, as LU and the SVD do
⌹1 1⍴∞
⍝ =>
1 1⍴$n

⍝ — Folds follow IEEE too
[+/∞ ¯∞;×/0 ∞]   ⍝ [$n $n]

⍝ —
∞∧2
⍝ error: DOMAIN ERROR

⍝ — The gamma function gives an infinity or NaN at its poles
[!¯1 !¯2ₓ]   ⍝ [∞ $n]

⍝⍝ Dfn defaults, results, predicates and error guards

⍝ — A default left argument applies only when none is given
f←{⍺←2 ⋄ ⍺+⍵} ⋄ f3   ⍝ 5

⍝ — A given left argument skips the default's expression
f←{⍺←1÷0 ⋄ ⍺+⍵} ⋄ 10 f 3   ⍝ 13

⍝ — A final assignment's value is the result, and statements before the last run without returning
f←{a←1} ⋄ [f 0;1+f 0;{f ⍵ ⋄ 2}0]   ⍝ 1 2 2

⍝ — An empty dfn gives no result
{}0   ⍝ {}0

⍝ — A dfn whose predicates all fail raises an error
{a←1 ⋄ 0?2}0
⍝ error: DOMAIN ERROR

⍝ — Output inside a dfn shows, and its value is the result
{⎕←7}0
7
⍝ ⎕: 7

⍝ — A guard catches only the kinds it names
[{"DOMAIN"::7 ⋄ 1÷⍵}'a' {"VALUE" "DOMAIN"::8 ⋄ missingname}0 {"DOMAIN"::7 ⋄ "VALUE"::8 ⋄ 1÷⍵}'a' {"DOMAIN"::a←7 ⋄ 1÷⍵}'a']   ⍝ 7 8 7 7

⍝ — A default left argument can be a function
{⍺←+ ⋄ ⍺ 4}0   ⍝ 4

⍝ — An inner dfn's default ignores the outer left argument
10{g←{⍺←2 ⋄ ⍺+⍵} ⋄ g ⍵}3   ⍝ 5

⍝ — A dfn that ends in an assignment works inside a train
{f←{a←1} ⋄ (+f+)3}0   ⍝ 1

⍝ — A predicate is a one-item Boolean of any rank, within tolerance
g←{⍵?7;9} ⋄ [g ,1;g 1 1⍴0;g 1 1 1⍴1;g 1.000000000000001]   ⍝ 7 9 7 7

⍝ — No result can't be assigned or used as an argument
x←{}0
⍝ error: VALUE ERROR

⍝ —
1+{}0
⍝ error: VALUE ERROR

⍝ — An error guard for other error kinds passes the error on
{"VALUE"::7 ⋄ 1÷⍵}'a'
⍝ error: DOMAIN ERROR

⍝ — A body can hold several predicates, and runs on only while each holds
[{1?1?2}0 {1?0?2;3}0]   ⍝ 2 3

⍝ — A body without a predicate must be last, because it always returns
{1;2}0
⍝ error: SYNTAX ERROR

⍝ — A predicate must be one Boolean
{⍵?7;9}1 1
⍝ error: LENGTH ERROR

⍝ —
{⍵?7;9}⍬
⍝ error: LENGTH ERROR

⍝ —
{⍵?7;9}⊂,1
⍝ error: DOMAIN ERROR

⍝ —
{⍵?7;9}'a'
⍝ error: DOMAIN ERROR

⍝ —
{⍵?7;9}2
⍝ error: DOMAIN ERROR

⍝ — Without a left argument, ⍺ drops out, along with any call on it, and out of lists and strands. An inner dfn has its own ⍺
[10{g←{⍺+⍵} ⋄ g ⍵}3 {(2×⍺)-⍵}3 {[⍺ ⍵]}3 {⍺ ⍵}3]   ⍝ [3 ¯3 [3] 3]

⍝ — An absent ⍺ can't be a value
{x←⍺}3
⍝ error: VALUE ERROR

⍝ — A guard catches an error that •signal raises
[{"SYNTAX"::7 ⋄ •signal "SYNTAX"}0;{"INDEX"::7 ⋄ •signal "INDEX"}0;{"RANK"::7 ⋄ •signal "RANK"}0;{"LENGTH"::7 ⋄ •signal "LENGTH"}0;
 {"VALUE"::7 ⋄ •signal "VALUE"}0;{"LIMIT"::7 ⋄ •signal "LIMIT"}0;{"DOMAIN"::7 ⋄ •signal "DOMAIN"}0]
⍝ =>
7⍴7

⍝ — A handler can call a function
f←{•signal "LENGTH"}
g←{⍵+1}
{∞::g ⍵ ⋄ f ⍵}3   ⍝ 4

⍝ — A signalled error that no guard names passes on
{"DOMAIN"::7 ⋄ •signal "LENGTH"}0
⍝ error: LENGTH ERROR

⍝ — An error in a handler passes on
{∞::•signal "LENGTH" ⋄ ÷'a'}0
⍝ error: LENGTH ERROR

⍝ —
{∞::1÷'a' ⋄ 1÷⍵}'a'
⍝ error: DOMAIN ERROR

⍝ — •signal takes a kind's name, not a number
•signal 11
⍝ error: DOMAIN ERROR

⍝ — Any other one-word name is a kind of its own, in upper case
•signal "unknown"
⍝ error: UNKNOWN ERROR

⍝ — A kind is one word, without ERROR
•signal "DOMAIN ERROR"
⍝ error: DOMAIN ERROR

⍝ — The left argument is the message, as text
0 •signal "DOMAIN"
⍝ error: DOMAIN ERROR

⍝ — $e is the caught error, with its kind and message
{∞::$e.kind $e.message ⋄ "too big" •signal "LENGTH"}0   ⍝ "LENGTH" "too big"

⍝ — $e gives the line and column where the error happened
{∞::[$e.line $e.column] ⋄ 2+1÷⍵}'a'   ⍝ [1 30]ₓ

⍝ — $e exists only in a handler
$e
⍝ error: VALUE ERROR

⍝ — A dfn in a handler sees its $e, and its own guard binds its own
[{∞::{$e.kind}0 ⋄ ÷'a'}0 {∞::{∞::$e.kind ⋄ •signal "RANK"}0 ⋄ ÷'a'}0]   ⍝ "DOMAIN" "RANK"

⍝ — A guard catches a kind of the program's own, named in any case
{"NOTFOUND"::$e.message ⋄ "no user" •signal "notfound"}0   ⍝ "no user"

⍝ — Signalling $e again keeps its kind and message, and a left argument replaces the message
[{∞::$e.message ⋄ {∞::•signal $e ⋄ "first" •signal "LENGTH"}0}0 {∞::$e.message ⋄ {∞::"second" •signal $e ⋄ ÷'a'}0}0]   ⍝ "first" "second"

⍝ — ∞ doesn't catch an unsupported feature
{∞::0 ⋄ •signal "UNSUPPORTED"}0
⍝ error: UNSUPPORTED

⍝ — A guard that names INTERRUPT catches it
{"INTERRUPT"::$e.kind ⋄ •signal "INTERRUPT"}0   ⍝ "INTERRUPT"

⍝ — A guard takes names or ∞, not numbers
{11::0 ⋄ ÷'a'}0
⍝ error: DOMAIN ERROR

⍝ — A handler sees names assigned after its guard
{∞::fresh ⋄ fresh←1 ⋄ 1÷⍵}'a'   ⍝ 1

⍝ — A predicate must be 0 or 1
{2?1;0}0
⍝ error: DOMAIN ERROR

⍝ — A dfn can't return a function that uses its local names
{x←2 ⋄ {x+⍵}}0
⍝ error: DOMAIN ERROR

⍝ — A predicate's body can give a function
f←{1?+;0}0 ⋄ 2 f 3   ⍝ 5

⍝ — A handler is an expression, including an output assignment, and runs only on an error
f←{∞::⎕←1} ⋄ f0   ⍝ {}0

⍝⍝ Lexical frames, recursion, predicates and error guards

⍝ — A false predicate moves on to the next body. Dyalog 20.0.53963.0, IO=1, CT=1E¯14, DIV=0.
{⍵=0?;3}1   ⍝ 3

⍝ — A predicate that ends its body gives no result, which addition rejects
1+{⍵=0?;3}0
⍝ error: VALUE ERROR

⍝ — Name lookup follows lexical nesting, not the caller's local bindings
outer←{x←10 ⋄ read←{x} ⋄ caller←{x←99 ⋄ read ⍵} ⋄ caller0}
outer0   ⍝ 10.0

⍝ — A derived function sees later changes to its active lexical binding
outer←{x←2 ⋄ f←{x+⍵} ⋄ apply←{⍶ ⍵} ⋄ g←f apply ⋄ x←3 ⋄ g4}
outer0   ⍝ 7.0

⍝ — A local defined operator accepts an array operand
outer←{offset←{⍶+⍵} ⋄ (2 offset)3} ⋄ outer0   ⍝ 5.0

⍝ — An array operand captures its value at derivation, not its name
offset←{⍶+⍵}
a←2
kept←a offset
a←9
kept3   ⍝ 5.0

⍝ — Recursion through del uses the current dfn
fact←{⍵=0?1;⍵×∇⍵-1} ⋄ fact6   ⍝ 720.0

⍝ — Mutually recursive local functions resolve definitions introduced later
outer←{even←{⍵=0?1;odd ⍵-1} ⋄ odd←{⍵=0?0;even ⍵-1} ⋄ even ⍵}
outer8   ⍝ 1.0

⍝ — A read before a dfn's own assignment of a name sees the outer binding
x←1 ⋄ f←{y←x ⋄ x←2 ⋄ y+x} ⋄ f 0   ⍝ 3

⍝ — Assigning a local from the outer name of the same name leaves the outer one unchanged
x←1 ⋄ f←{x←x+10 ⋄ x} ⋄ (f 0),x   ⍝ 11 1

⍝ — Execute binds a local that the dfn never assigns in its own text
f←{⍵>0?(⍎"q←5") ⋄ q;0} ⋄ f 1   ⍝ 5

⍝ — Execute rebinds a local that the dfn assigns in its own text
f←{x←1 ⋄ ⍎"x←2" ⋄ x} ⋄ f 0   ⍝ 2

⍝ — A name that execute binds in a dfn hides the global from a closure inside it
x←1 ⋄ f←{h←{x} ⋄ ⍎"x←2" ⋄ h 0} ⋄ f 0   ⍝ 2

⍝ — A name that a dfn assigns only in a body that didn't run is unbound for •nc and •nl
f←{⍵>0?slotted←1;[•nc "slotted";•nl 2]} ⋄ f 0   ⍝ [0ₓ 0⍴⊂""]

⍝ — ⍺ is never found in an outer frame
f←{⍺} ⋄ g←{f ⍵} ⋄ 1 g 2
⍝ error: VALUE ERROR

⍝ — A callee's error reaches the caller's guard, and the handler sees the caller's locals as they were at the error
bad←{1÷⍵}
guarded←{x←10 ⋄ ∞::x ⋄ x←20 ⋄ bad ⍵}
guarded 'a'   ⍝ 20.0

⍝ — A failing handler is inactive while it runs, allowing the earlier guard to catch it
guarded←{∞::7 ⋄ ∞::1÷'a' ⋄ 1÷⍵} ⋄ guarded 'a'   ⍝ 7.0

⍝ — A true predicate that ends its body returns no result
{⍵=0?;3}0   ⍝ {}0

⍝ — An empty error handler catches the error and returns no result
{∞:: ⋄ 1÷'a'}0   ⍝ {}0

⍝ — A final predicate that ends its body also returns no result
{⍵=0?}0   ⍝ {}0

⍝ — Output is not rolled back by a guard.
guarded←{∞::7 ⋄ ⎕←2 ⋄ 1÷⍵} ⋄ guarded 'a'
7
⍝ ⎕: 2

⍝⍝ Enclose

⍝ — Enclosing a vector produces rank zero
≢⍴⊂1 2 3   ⍝ 0ₓ

⍝ — Disclosing recovers the vector
↑⊂1 2 3   ⍝ 1 2 3

⍝⍝ Binder limits and single execution

⍝ — Groups execute once, right to left; the left read sees the right assignment
x←1 ⋄ (⎕←x)+⎕←x←2
4
⍝ ⎕: 2\n2

⍝ — Reduction calls its derived operand exactly once per step
apply←{⍶ ⍵}
f←{⎕←⍵ ⋄ ⍵}
(f apply)/1 2 3
⍝ =>
3
⍝ ⎕: 3\n3

⍝ — A whole strand binds as one array operand
offset←{+/⍶+⍵} ⋄ (1 2 offset)3   ⍝ 9

⍝⍝ Pervasive functions and reduction

⍝ — Reduction remains right-associated
-/1 2 3   ⍝ 2

⍝ — Empty sum uses the additive identity
+/⍳0ₓ   ⍝ 0ₓ

⍝ — Empty product uses the multiplicative identity
×/⍳0ₓ   ⍝ 1ₓ

⍝ — Reduction of an atom preserves it
+/7   ⍝ 7

⍝ —
1 2 3+10   ⍝ 11 12 13

⍝ —
10-1 2 3   ⍝ 9 8 7

⍝ —
1 2+3 4   ⍝ 4 6

⍝ — Tally of a singleton vector is exact
≢,7   ⍝ 1ₓ

⍝ — An atom has an empty shape vector
⍴7   ⍝ ⍬ₓ

⍝ — Unit extension over an empty vector remains empty
2ₓ+⍳0ₓ   ⍝ ⍬ₓ

⍝ —
2×3+4   ⍝ 14.0

⍝ —
(2×3)+4   ⍝ 10.0

⍝ —
10-3-2   ⍝ 9.0

⍝ —
¯2+5   ⍝ 3.0

⍝ —
2×-3+4   ⍝ ¯14.0

⍝ —
2+-3   ⍝ ¯1.0

⍝ —
--2   ⍝ 2.0

⍝ —
8÷4÷2   ⍝ 4.0

⍝ —
+¯2   ⍝ ¯2.0

⍝ —
×¯9   ⍝ ¯1ₓ

⍝ —
×0   ⍝ 0ₓ

⍝ —
×3   ⍝ 1ₓ

⍝ —
÷4   ⍝ 0.25

⍝ —
1.5+.5   ⍝ 2.0

⍝ —
¯.5×2   ⍝ ¯1.0

⍝ —
1E¯2+2e¯2   ⍝ 0.03

⍝ — First retrieves the function that a unit made with ᵘ holds
2(↑-ᵘ)3   ⍝ ¯1.0

⍝ — A parenthesis inside a comment does not affect grouping
(2×3) + 4 ⍝ comment )
10.0

⍝ —
0÷0   ⍝ $n

⍝ —
0÷¯0   ⍝ $n

⍝ —
¯0   ⍝ 0.0

⍝ — Comments alone produce no result
⍝ only a comment
{}0

⍝ — Empty input produces no result
   ⍝ {}0

⍝ — Display keeps negative zero
⎕←¯0
¯0
⍝ ⎕: ¯0

⍝ —
⎕←¯2
¯2
⍝ ⎕: ¯2

⍝⍝ Executing binding gate

⍝ — Reduction calls a named dfn through the ordinary function path
add←{⍺+⍵} ⋄ add/1 2 3   ⍝ 6

⍝ — A named fork computes sum divided by tally
avg←+/÷≢ ⋄ avg2 4 9   ⍝ 5

⍝ — A defined operator invokes its function operand
apply←{⍶ ⍵} ⋄ (-apply)3   ⍝ ¯3

⍝ — A defined operator may instead use an array operand
offset←{⍶+⍵} ⋄ (2 offset)3   ⍝ 5

⍝ — Names in a dfn resolve at call time, permitting a later definition
inc←{later ⍵}
later←{1+⍵}
inc4   ⍝ 5

⍝ — A caller's local name does not shadow the callee's global binding
x←10
read←{x}
caller←{x←99 ⋄ read ⍵}
caller0   ⍝ 10

⍝ — A named reduction retains its dfn operand
add←{⍺+⍵}
total←add/
total1 2 3   ⍝ 6

⍝ —
{2×⍵}3   ⍝ 6

⍝ —
1 0 1#2 4 6   ⍝ 2 6

⍝ — Fork evaluates the right arm before the left arm
f←{⎕←1 ⋄ ⍵}
h←{⎕←2 ⋄ ⍵}
t←f+h
t3
⍝ =>
6
⍝ ⎕: 2\n1

⍝ — A nested dfn can be defined and called within the active outer frame
outer←{inner←{⍵} ⋄ inner ⍵} ⋄ outer1   ⍝ 1

⍝⍝ Array literals and completeness

⍝ — A trailing semicolon leaves an empty item, which is an error
[42;]
⍝ error: SYNTAX ERROR

⍝ — A leading semicolon leaves an empty item, which is an error
[;42]
⍝ error: SYNTAX ERROR

⍝ — Repeated semicolons leave an empty item, which is an error
[1;;2]
⍝ error: SYNTAX ERROR

⍝ — A line break inside brackets counts as a space
[
1
2
]   ⍝ 1 2

⍝ — Matrix literals pad shorter rows with fill
[1 2⋄[3]]   ⍝ 2 2⍴1 2 3 0

⍝ — A one-cell matrix literal retains rank two
[[42]⋄]   ⍝ 1 1⍴42

⍝ — Unit rows are rank-0 cells, so they stack into a vector
[1⋄2]   ⍝ 1 2

⍝ — Dfn statements and comments do not split the enclosing literal's rows
[
({a←⍵+1 ⋄ a}2 3) ⋄ ⍝ first row
5
]
⍝ =>
2 2⍴3 4 5 0

⍝ — List items evaluate left to right and may refer to earlier assignments
[a←2 a+3]   ⍝ 2 5

⍝ — Semicolon-separated items keep their strands whole
[1 2;3 4]   ⍝ [[1 2] [3 4]]

⍝ — Explicit output exposes list items' left-to-right evaluation order
[⎕←1 ⎕←2]
1 2
⍝ ⎕: 1\n2

⍝⍝ Based values, mapping and selection

⍝ —
⍬⍉3   ⍝ 3

⍝ —
(⊂3)+4   ⍝ ⊂7

⍝ —
≢1 2 3   ⍝ 3ₓ

⍝ —
≢¨[1 2 3;4 5]   ⍝ [3 2]ₓ

⍝ —
+/1 2 3   ⍝ 6

⍝ —
{⍺+⍵}/1 2 3   ⍝ 6

⍝ — A one-item reduction returns the item without calling its operand
f←{1÷0} ⋄ [f/3;f/⊂3;f/,3;f⌿⊂⊂3;f/⊂2 3]   ⍝ [3 3 3 (⊂3) [2 3]]

⍝ —
1 2 3+.×4 5 6   ⍝ 32

⍝ —
{⍵}⍤0 [1 2 3]   ⍝ 1 2 3

⍝ —
↑2 3⍴⍳6ₓ   ⍝ [0 1 2]ₓ

⍝ —
1⊃2 3⍴⍳6ₓ   ⍝ [3 4 5]ₓ

⍝ —
1 2⊃2 3⍴⍳6ₓ   ⍝ 5ₓ

⍝ —
2⊃1⊃[10 20;30 40 50]   ⍝ 50

⍝ —
(3 4⍴⍳12ₓ).[0 2;1 3]   ⍝ [1 11]ₓ

⍝ —
↑⍬   ⍝ 0

⍝ —
↑0 3⍴0   ⍝ 0 0 0

⍝ —
a←2 3⍴⍳6ₓ
(1⊃a)←7ₓ 8ₓ 9ₓ
a   ⍝ [0 1 2⋄7 8 9]ₓ

⍝ —
a←2 3⍴⍳6ₓ
(1 2⊃a)←9ₓ
a   ⍝ [0 1 2⋄3 4 9]ₓ

⍝ —
a←[[1 2] [3 4]]
(1⊃0⊃a)←9
a   ⍝ [[1 9] [3 4]]

⍝ —
a←3
a.[]←4
a   ⍝ 4

⍝ —
v←3 4 ⋄ v₀²+v₁²   ⍝ 25

⍝ — A subscript after a space selects from the whole run before it
x←1 2 ⋄ [2 3 ₁;x+1 ₁]   ⍝ 3 3

⍝ subscript-floor-name — After a name or an expression in parentheses, ₓ is ⌊
x←3.7 ¯2.5 ⋄ [xₓ;(x+0.5)ₓ]   ⍝ [[3 ¯3]ₓ [4 ¯2]ₓ]

⍝ subscript-floor-positions — ⍳ of a count floored by ₓ gives exact positions
n←4.5 ⋄ ⍳nₓ   ⍝ [0 1 2 3]ₓ

⍝ subscript-floor-function — ₓ floors an array, not a function
+ₓ
⍝ error: DOMAIN ERROR

⍝ subscript-join-names — Between two items, ⱼ, ₑ and ᵣ join them as one term: xⱼy is x+y×0ⱼ1, xₑy is x×10*y and xᵣy is xₓ÷yₓ
x←3 ⋄ y←2 ⋄ [xⱼy xₑy xᵣy]   ⍝ [3ⱼ2 300 3ᵣ2]

⍝ subscript-join-literal-left — A literal before ⱼ, ₑ or ᵣ gives what the all-literal form gives
n←3 ⋄ [2ₑn 1ᵣn 1ⱼn]   ⍝ [2ₑ3 1ᵣ3 1ⱼ3]

⍝ subscript-join-items — Subscripts and dot paths bind before ⱼ, ₑ or ᵣ joins, the left item takes its superscripts, a superscript after the right item applies to the term, and ₑ joins before ⱼ
v←10 20 30 ⋄ T←["a":1 "b":2] ⋄ x←1 ⋄ y←2 ⋄ [v₁ⱼT.b xⱼy² x²ⱼy xⱼyₑ1]   ⍝ [20ⱼ2 ¯3ⱼ4 1ⱼ2 1ⱼ20]

⍝ subscript-join-function — ⱼ, ₑ and ᵣ join two arrays, not a function
f←- ⋄ fⱼ1
⍝ error: SYNTAX ERROR

⍝ subscript-join-path-function — A dot path to a function is a function there too
m←["f":-] ⋄ (m.f)ⱼ1
⍝ error: SYNTAX ERROR

⍝ —
[[1 3] [2 4]]⍳⊂1 3   ⍝ 0ₓ

⍝ —
v←10 20 30 ⋄ [1⌷v [⊂1]⌷v [,1]⌷v]   ⍝ [20 (⊂20) (,20)]

⍝ —
a←[[1 2] [3 4]]
a₁.[0]←9
a   ⍝ [[1 2] [9 4]]

⍝ —
a←[[1 2] [3 4]]
a₁,←5
a   ⍝ [[1 2] [3 4 5]]

⍝ —
a←[[1 2] [3 4]]
a₁←5 6 7
a   ⍝ [[1 2] [5 6 7]]

⍝ —
fs←[+ ×]
fs₁←-
f←fs₁
3 f 2   ⍝ 1

⍝ —
1 3⍳2   ⍝ 2ₓ

⍝ —
1 3⍳⊂2   ⍝ 2ₓ

⍝ —
1 3⍸2   ⍝ 1ₓ

⍝ —
1 3⍸⊂2   ⍝ 1ₓ

⍝ —
2∊1 2 3   ⍝ $t

⍝ —
(⊂2)∊1 2 3   ⍝ $t

⍝ —
1 3 (1⍨=⍸) 0 1 2 3 4   ⍝ $f $t $t $f $f

⍝ —
{⍵∊1 3}¨⍳4ₓ   ⍝ $f $t $f $t

⍝ —
(2 2⍴⍳4ₓ)⍳2 3   ⍝ 1ₓ

⍝ —
1 3⍳⍬   ⍝ ⍬ₓ

⍝ —
[⌽2 ⍉⊂2 ⍬↑2 ⍬↓⊂2]   ⍝ [2 (⊂2) 2 (⊂2)]

⍝ —
2⍷2   ⍝ $t

⍝⍝ Seeded reduction

⍝ — A reduction seed is the final right argument
10-/1 2 3   ⍝ ¯8

⍝ — Each matrix lane starts with the same reduction seed
10+/2 3⍴⍳6ₓ   ⍝ 13 22

⍝ — Rank pairs each row with its own seed
10 20 (+/⍤0 1) 2 3⍴⍳6ₓ   ⍝ 13 32

⍝ — Empty reduction returns the whole seed without calling the operand
[[1 2] [3 4]]{1÷0}/⍬   ⍝ [[1 2] [3 4]]

⍝ — Seeded reduction follows a nested path
tree←[[10 20] [30 [40 50]]] ⋄ tree⊃/⌽1 1 0   ⍝ 40

⍝ — A scalar seed remains distinct from an atom
(⊂10)+/1 2 3   ⍝ ⊂16

⍝ — Whole-seed scan follows a path and retains its intermediate arrays
[[10 20] [30 40]] ⊃⍨\ 0 1   ⍝ [[10 20] 20]

⍝⍝ Axis-key dictionaries and assignment

⍝ axis-dictionary — One string names a whole array; atomic selection returns that stored array
T←"price" "qty":[1 2 3;4 5 6]
[≢T;⍴T;0⊃T;"qty"⌷T;⍴"qty"⌷T;⍳⍠0 T]
⍝ =>
[2ₓ;[2]ₓ;1 2 3;4 5 6;[3]ₓ;"price" "qty"]

⍝ axis-function-value — Stored functions keep their predicates
T←["sign":{⍵<0?¯1;1}] ⋄ T.sign¨¯2 3   ⍝ ¯1 1

⍝ axis-empty — Empty labelled axes retain their key lists
T←⍬:⍬ ⋄ [≢T;⍴⍳⍠0 T;T≡⍬;T≡T;2⍴T]
[0ₓ;[0]ₓ;$t;$t;0 0]

⍝ axis-duplicate — Labels are unique within an axis
"aa" "aa":1 2
⍝ error: DOMAIN ERROR

⍝ axis-length — Key lists match axis lengths
"aa" "bb" "cc":2 3⍴⍳6ₓ
⍝ error: LENGTH ERROR

⍝ axis-key-type — Keys are strings, or a position's own number to leave that position unnamed
["aa" 1:3 4;"b":5]   ⍝ ["aa":3 4 "b":5]

⍝ —
"aa" 1:3 4   ⍝ ["aa":3 4]

⍝ —
0 1:3 4   ⍝ 3 4

⍝ —
1 0:3 4
⍝ error: DOMAIN ERROR

⍝ axis-pick-batch — Picking one key at a time may repeat extracted values
T←"price" "qty":[1 2 3;4 5 6]
[{⍵⊃T}¨"qty" "price";{⍵⊃T}¨"price" "price"]
⍝ =>
[[4 5 6;1 2 3] [1 2 3;1 2 3]]

⍝ axis-nested — Nested lookups use axis-local names
T←["name":"Ann" "addr":"city":"Paris" "items":[["name":"pen"] ["name":"ink"]]]
["city"⊃"addr"⊃T "name"⊃1⊃"items"⊃T T.addr.city]
⍝ =>
"Paris" "ink" "Paris"

⍝ axis-selection-repeat — Keyed selection cannot repeat a named position
[["aa" "aa"]]⌷"aa" "bb":1 2
⍝ error: DOMAIN ERROR

⍝ axis-missing — Reading a missing key errors
"missing"⌷"aa":1
⍝ error: INDEX ERROR

⍝ axis-assignment-alignment — Keyed RHS aligns to selection and ignores extra keys [mixed]
T←"price" "qty":[1 2 3;4 5 6]
T.[["price" "qty"]]←"qty" "extra" "price":7 9 8
T   ⍝ "price" "qty":8 7

⍝ axis-assignment-position — Unkeyed RHS assigns by position
T←"price" "qty":[1 2 3;4 5 6]
T.[["qty" "price"]]←[10 20;30]
T   ⍝ "price" "qty":[30;10 20]

⍝ axis-assignment-scalar — Unit selection replaces a value; vector selection aligns its retained axis
T←"aa" "bb":1 2
T.["aa"]←"bb" "aa":8 9
T.["bb"]+←10
T   ⍝ ["aa":["bb":8 "aa":9] "bb":12]

⍝ axis-assignment-singleton — A one-position vector selection aligns the RHS by key
T←"aa" "bb":1 2
T.[,⊂"aa"]←"bb" "aa":8 9
T   ⍝ "aa" "bb":9 2

⍝ axis-assignment-missing — Every selected key must occur in a keyed replacement
T←"aa" "bb":1 2 ⋄ T.[["aa" "bb"]]←"aa":5
⍝ error: INDEX ERROR

⍝ axis-assignment-repeat — Assignment cannot select a labelled position twice
T←"aa" "bb":1 2 ⋄ T.[[0 0]]←5 6
⍝ error: DOMAIN ERROR

⍝ axis-nested-write — Pick and dot replace stored values without changing other copies
T←["n":1 "addr":"city":"Paris"]
("n"⊃T)←5
T.n+←1
("city"⊃"addr"⊃T)←"Rome"
U←T
U.addr.city←"Oslo"
[T U.addr.city]
⍝ =>
[["n":6 "addr":"city":"Rome"] "Oslo"]

⍝ axis-nested-index-write — Dot indexing after a dotted value updates inside it
T←"v":1 2 3
T.v.[1]←9
T.v.[2]+←1
T   ⍝ "v":1 9 4

⍝ axis-dot-insert — Plain dot and Pick updates append to keyed vectors
T←⍬:⍬
T.a←1
("b"⊃T)←"c":2
T.b.d←3
T   ⍝ ["a":1 "b":["c":2 "d":3]]

⍝ axis-modified-missing — Modified assignment needs an existing key
T←"a":1 ⋄ T.b+←2
⍝ error: INDEX ERROR

⍝ axis-path-create — Plain assignment through a dot path or a Pick path creates missing records
T←"n":1
T.style.color←"red"
T.style.width←2
T.a.b.c←3
["a" "b" "d"]⊃T ←4
["p" "q"]⊃T ←5
T
⍝ =>
["n":1 "style":["color":"red" "width":2] "a":["b":["c":3 "d":4]] "p":["q":5]]

⍝ axis-path-nonrecord — A path cannot descend into a value that is not a record
T←"a":1 ⋄ T.a.y←2
⍝ error: RANK ERROR

⍝ axis-bracket-insert — New names append in selector order, including keyed RHS alignment
T←"aa":1
T.["bb"]←2
T.[["cc" "aa" "dd"]]←30 10 40
U←⍬:⍬
U.[["xx" "yy"]]←"yy" "xx":2 1
[T U]
⍝ =>
[("aa" "bb" "cc" "dd":10 2 30 40) ("xx" "yy":1 2)]

⍝ axis-nested-bracket-insert — Append within a dotted container
T←"addr":"city":"Paris"
T.addr.["zip"]←"75"
T   ⍝ ["addr":["city":"Paris" "zip":"75"]]

⍝ axis-modified-bracket-missing — Modified assignment to a missing key does not append
T←"aa":1 ⋄ T.["bb"]+←2
⍝ error: INDEX ERROR

⍝ axis-numeric-no-insert — Out-of-range numeric coordinates do not append
T←"aa":1 ⋄ T.[1]←5
⍝ error: INDEX ERROR

⍝ axis-new-repeat — A newly appended position cannot be selected twice
T←"aa":1 ⋄ T.[["zz" "zz"]]←1 2
⍝ error: DOMAIN ERROR

⍝ axis-rename — Reattach edited axis selectors to rename keys
T←"price" "qty":1 2
K←⍳⍠0 T
K₀←"cost"
K:T   ⍝ "cost" "qty":1 2

⍝ axis-nested-agreement — Nested keyed values align their own axes
("p":"xx" "yy":1 2)+("p":"yy":10)   ⍝ "p":"xx" "yy":1 12

⍝ axis-nested-broadcast — Enclosure broadcasts within each stored array
T←"price" "qty":[1 2 3;4 5 6]
[T+⊂10 20 30;10×T;+/T;+/¨T;≢¨T]
⍝ =>
[("price" "qty":[11 22 33;14 25 36]);("price" "qty":[10 20 30;40 50 60]);5 7 9;("price" "qty":6 15);("price" "qty":[3 3]ₓ)]

⍝ axis-filter — Filtering and ordering retain the selected labels
Q←"aa" "bb" "cc":10 20 5
[1 0 1#Q;1⌽Q;1↓Q;¯1↑Q;[⍋Q]⌷Q;[[2 0]]⌷Q]
⍝ =>
[("aa" "cc":10 5) ("bb" "cc" "aa":20 5 10) ("bb" "cc":20 5) ("cc":5) ("cc" "aa" "bb":5 10 20) ("cc" "aa":5 10)]

⍝ axis-overtake — Padding a keyed axis leaves the new positions unnamed
3↑"aa" "bb":1 2   ⍝ ["aa":1 "bb":2 0]

⍝ axis-expand — Expand leaves inserted positions unnamed
1 0 1#⁻¹("aa" "bb":1 2)   ⍝ ["aa":1 0 "bb":2]

⍝ axis-scan — Scan retains labels along its accumulated axis
+\("aa" "bb" "cc":1 2 3)   ⍝ "aa" "bb" "cc":1 3 6

⍝ axis-cat-duplicate — Catenate does not merge duplicate labels
("aa" "bb":1 2),("bb" "cc":20 30)
⍝ error: DOMAIN ERROR

⍝ axis-cat-plain — An unkeyed part of a joined axis leaves its positions unnamed
("aa" "bb":1 2),5   ⍝ ["aa":1 "bb":2 5]

⍝ axis-each — Each receives stored values and aligns outer axes
T←"price" "qty":[1 2 3;4 5 6]
[1⌷¨T ("aa" "bb":1 2){[⍺ ⍵]}¨("bb":3)]
⍝ =>
[("price" "qty":2 5) ("aa" "bb":[1 0;2 3])]

⍝ axis-match-position — Match ignores labelled-axis order; positional access observes it
A←"aa" "bb":1 2
B←"bb" "aa":2 1
[A≡B A₀≡B₀]   ⍝ $t $f

⍝ axis-sets — Set operations compare values, retaining selected labels
A←"aa" "bb":1 2
B←"aa" "bb" "cc":1 9 3
[A∊B;("xx":9)∊B;A∩B;A~B;A∪("cc":3);∪"aa" "bb" "cc":1 1 2]
⍝ =>
[("aa" "bb":[$t $f]) ("xx":$t) ("aa":1) ("bb":2) ("aa" "bb" "cc":1 2 3) ("aa" "cc":1 2)]

⍝ axis-search-cells — Frame names return positions. Keyed cells match by key in any order, and an unkeyed cell matches them by position
M←["alice" "bob";"price" "qty"]:[10 2⋄20 4]
[M⍳"bob"⌷M;M⍳⌽"bob"⌷M;M⍳20 4]
⍝ =>
"bob" "bob" "bob"

⍝ axis-set-rank — Set functions compare major cells at any rank, so a keyed matrix's union with itself is the matrix
M←"aa" "bb":2 2⍴⍳4ₓ ⋄ M∪M   ⍝ "aa" "bb":2 2⍴⍳4ₓ

⍝⍝ Libraries

⍝ — Signed 8-bit integers
[int:]←•load "lib/numeric.bpl"
8 int 0 127 128 255   ⍝ 0 127 ¯128 ¯1

⍝ — Normal random array shape
[NormRand:]←•load "lib/numeric.bpl"
⍴NormRand2 3   ⍝ [2 3]ₓ

⍝ — Phinary decoding
[phinary:]←•load "lib/numeric.bpl"
1e¯12>|42-phinary"10100010.00100001"   ⍝ $t

⍝ — Associative scan along the last axis
[ascana:]←•load "lib/array.bpl"
+ascana [1 2 3⋄4 5 6]   ⍝ [1 3 6⋄4 9 15]

⍝ — Associative scan of a vector of strings
[ascan:]←•load "lib/array.bpl"
{⍺,⍵}ascan "ab" "cd" "ef"   ⍝ "ab" "abcd" "abcdef"

⍝ — Unwrap line breaks
[unwrap:]←•load "lib/string.bpl"
unwrap "abc",(•ucs 10),"def"   ⍝ "abc def"

⍝ — Parent-first traversal
[trav:]←•load "lib/tree.bpl"
t←[1 [2 ,4 ,5] ,3]
⍬{⍺,↑⍵}trav{1↓⍵}t   ⍝ 1 2 4 5 3

⍝ — Children-first traversal
[ravt:]←•load "lib/tree.bpl"
t←[1 [2 ,4 ,5] ,3]
⍬{⍺,↑⍵}ravt{1↓⍵}t   ⍝ 4 5 2 3 1

⍝⍝ CSV

⍝ — Headers key compact, independently inferred columns
nl←•ucs 10 ⋄ •csv "price,qty",nl,"10.5,2",nl,"20.0,4"
["price":10.5 20;"qty":[2 4]ₓ]

⍝ — Inference is per column; numeric-looking text stays text in a mixed column
nl←•ucs 10 ⋄ •csv "id,note",nl,"001,001",nl,"002,no"
["id":1 2;"note":"001" "no"]ₓ

⍝ — Forced text preserves identifiers, including quoted numeric fields
src←"id,n", (•ucs 10), """00123"",5"
("text_columns":"id") •csv src
⍝ =>
["id":,⊂"00123" "n":,5ₓ]

⍝ — Empty numeric cells become NaN. Entirely empty columns are text
nl←•ucs 10 ⋄ •csv "a,b,c",nl,"1,1.5,",nl,",,"
["a":[1 $n] "b":[1.5 $n] "c":["" ""]]

⍝ — Custom markers and fill, with positional forced numeric selection
nl←•ucs 10
src←"a,b",nl,"NA,NA",nl,"3,yes"
["missing":"NA" "fill":¯1ₓ "numeric_columns":0] •csv src
⍝ =>
"a" "b":[¯1 3;"" "yes"]ₓ

⍝ — Headerless input and locale-specific numeric spelling
nl←•ucs 10
src←"1.234,5;2",nl,"2.000;3"
["header":0 "separator":';' "decimal":',' "thousands":'.'] •csv src
⍝ =>
[1234.5 2000;[2 3]ₓ]

⍝ — Quoting, doubled quotes, embedded newline and Unicode
nl←•ucs 10
•csv "note",nl,"""a,b""",nl,"""say """"hi""""""",nl,"""λ",nl,"😀"""
⍝ =>
"note":["a,b" "say ""hi""" 'λ',(•ucs 10),'😀']

⍝ — Whitespace is preserved unless trimming is requested
nl←•ucs 10
src←'n',nl," 2 "
[•csv src;("trim":1) •csv src]
⍝ =>
[("n":,⊂" 2 ") ("n":,2ₓ)]

⍝ — Empty input
•csv ""   ⍝ 0⍴⊂""

⍝ — Header-only input retains empty columns
•csv "a,b"   ⍝ "a" "b":[0⍴⊂"";0⍴⊂""]

⍝ — Integers beyond i64 remain exact
•csv 'n',(•ucs 10),"9223372036854775808"
"n":,9223372036854775808ₓ

⍝ — Float promotion must not round large exact integers
•csv 'n',(•ucs 10),"9007199254740993",(•ucs 10),"1.5"
"n":9007199254740993ₓ 1.5

⍝ — Export and import retain numeric domains, strings and headers
T←["price":10.5 20;"qty":[2 4]ₓ;"note":"a,b" "say ""hi"""]
•csv •csv⁻¹ T
⍝ =>
["price":10.5 20;"qty":[2 4]ₓ;"note":"a,b" "say ""hi"""]

⍝ — Export uses CSV minus signs and no exact suffix
•csv⁻¹ "n":[¯2 3]ₓ   ⍝ 'n',(•ucs 10),"-2",(•ucs 10),'3',•ucs 10

⍝ — Infinity remains a number unless fill is explicitly configured
T←"n":1ₓ ∞ ⋄ [•csv⁻¹ T;("fill":∞) •csv⁻¹ T]
['n',(•ucs 10),'1',(•ucs 10),"inf",•ucs 10;'n',(•ucs 10),'1',(•ucs 10),"""""",•ucs 10]

⍝ — Export dialect and CRLF
T←"n":,1234.5
["separator":';' "decimal":',' "thousands":'.' "lineending":(•ucs 13 10)] •csv⁻¹ T
⍝ =>
'n',(•ucs 13 10),"1.234,5",•ucs 13 10

⍝ — Duplicate headers
•csv "a,a"
⍝ error: DOMAIN ERROR

⍝ — Inconsistent record width
•csv "a,b",(•ucs 10),'1'
⍝ error: LENGTH ERROR

⍝ — Explicit numeric columns reject invalid nonmissing text
("numeric_columns":"a") •csv 'a',(•ucs 10),"no"
⍝ error: DOMAIN ERROR

⍝ — Unknown column selector
("text_columns":"c") •csv "a,b"
⍝ error: INDEX ERROR

⍝ — Conflicting column modes
["text_columns":0 "numeric_columns":0] •csv "a,b"
⍝ error: DOMAIN ERROR

⍝ — Unknown option
("seperator":';') •csv "a,b"
⍝ error: DOMAIN ERROR

⍝ — Export requires equal-length column vectors
•csv⁻¹ "a" "b":[1 2;,3]
⍝ error: LENGTH ERROR

⍝ — Export rejects nested cells
•csv⁻¹ "a":,⊂1 2
⍝ error: DOMAIN ERROR

⍝ — Quote-free export errors when a field needs quoting
("quotechar":"") •csv⁻¹ "a":,⊂"x,y"
⍝ error: DOMAIN ERROR

⍝ — Escape characters round-trip inside quoted text
T←"note":,⊂"a\b""c"
opts←["escapechar":'\' "doublequote":0]
csv←opts •csv⁻¹ T
opts •csv csv   ⍝ "note":,⊂"a\b""c"

⍝⍝ JSON

⍝ — JSON objects become keyed vectors; arrays retain nesting and integer exactness
•json "{""name"":""Ann"",""values"":[1,2.5,[3,4]]}"
["name":"Ann" "values":[1ₓ 2.5 [3 4]ₓ]]

⍝ — A vector of JSON texts gives a vector of values
•json ["1" "[2]"]   ⍝ [1ₓ [2]ₓ]

⍝ — Beside null, which defaults to NaN, Booleans read as the floats 1 and 0
•json "[true,false,null]"   ⍝ 1 0 $n

⍝ — Explicit null fill works recursively in both directions
fill←"fill":¯1ₓ
fill •json⁻¹ fill •json "{""x"":[null,2]}"
⍝ =>
"{""x"":[null,2]}"

⍝ — Import keeps an integer beyond i64 that no float equals, beside float syntax
•json "[9223372036854775809,1.0,1e2]"
9223372036854775809ₓ 1 100

⍝ — Large exact integers export without rounding
•json⁻¹ 9223372036854775808ₓ   ⍝ "9223372036854775808"

⍝ — JSON true and false read as Booleans and write back unchanged
•json⁻¹ •json "[true,false]"   ⍝ "[true,false]"

⍝ — Empty objects, arrays and strings retain their distinct meanings
•json⁻¹ •json "[{},[],""""]"   ⍝ "[{},[],""""]"

⍝ — JSON strings remain strings, including one-character strings
•json⁻¹ •json "[""a"","""",[""b"",""c""]]"
"[""a"","""",[""b"",""c""]]"

⍝ — Ordinary matrices export as nested JSON arrays
•json⁻¹ [1 2⋄3 4]ₓ   ⍝ "[[1,2],[3,4]]"

⍝ — Keyed axes export as object levels
•json⁻¹ "row":"col" "val":[1 2]ₓ
"{""row"":{""col"":1,""val"":2}}"

⍝ — Unkeyed scalar arrays export their contents
•json⁻¹ ⊂2ₓ   ⍝ "2"

⍝ — Duplicate object members follow the JSON library's last-value rule
•json "{""name"":1,""name"":2}"   ⍝ "name":2ₓ

⍝ — Malformed JSON gives a located error
•json "[1 2]"
⍝ error: DOMAIN ERROR

⍝ — Out-of-range float input reads as an infinity, not as a missing value
•json "1e999"   ⍝ ∞

⍝ — JSON5 adds comments, trailing commas, single quotes, unquoted keys, hexadecimal numbers and Infinity
•json "{a: [1, 0x1F,], b: 'x', c: -Infinity}  // done"   ⍝ ["a":[1 31]ₓ "b":"x" "c":¯∞]

⍝ — JSON5's NaN reads as NaN, not as a missing value
•json "[1, NaN]"   ⍝ [1 $n]

⍝ — Infinity requires explicit fill on export
•json⁻¹ ∞
⍝ error: DOMAIN ERROR

⍝ — Nonintegral rationals have no JSON number representation
•json⁻¹ 1r3
⍝ error: DOMAIN ERROR

⍝ — Complex values have no JSON number representation
•json⁻¹ 1j2
⍝ error: DOMAIN ERROR

⍝ — Functions have no JSON representation
•json⁻¹ [+ -]
⍝ error: DOMAIN ERROR

⍝ — LZW repeated-code expansion and capped dictionary
[packZ:]←•load "lib/dyalog.bpl"
[0 packZ packZ "aaaaaa";0 packZ 1 packZ "abababab"]
⍝ =>
"aaaaaa" "abababab"

⍝ — LZW dictionary
[packZ:]←•load "lib/dyalog.bpl" ⋄ ¯3 packZ "aba"
["a "⋄"b "⋄"ab"⋄"ba"]

⍝ — LZW alphabet exceeds code width
[packZ:]←•load "lib/dyalog.bpl" ⋄ 1 packZ "abc"
⍝ error: DOMAIN ERROR

⍝ — Absolute-year calendar layout
[cal:]←•load "lib/dyalog.bpl" ⋄ ⍴cal2025
[33 66]ₓ

⍝ — Unification substitutes repeated variables on either side
[unify:]←•load "lib/dyalog.bpl"
"xy" unify ['f' 3 'y';'f' 'x' 'x']
⍝ =>
'f' 3 3

⍝ — Unification rejects cyclic substitution
[unify:]←•load "lib/dyalog.bpl" ⋄ 'x' unify ['x';'f' 'x']
⍝ error: DOMAIN ERROR

⍝ — Unification rejects distinct constants
[unify:]←•load "lib/dyalog.bpl" ⋄ 'x' unify ['f' 1;'f' 2]
⍝ error: DOMAIN ERROR

⍝ — Unification rejects different term shapes
[unify:]←•load "lib/dyalog.bpl" ⋄ 'x' unify ['f' 1;'f' 1 2]
⍝ error: LENGTH ERROR

⍝ — Repeating rational carry and fractional normalization
[ratsum:]←•load "lib/dyalog.bpl"
rs←"0123456789"ratsum
["<0|0|3>"rs"<0|0|6>" "<0|0.5|0>"rs"<0|0.5|0>"]
⍝ =>
"<0|1|0>" "<0|1|0>"

⍝ — Repeating rational negation and binary carry
[ratsum:]←•load "lib/dyalog.bpl"
[("0123456789"ratsum)"<0|1|0>" "<0|1|0>"("01"ratsum)"<0|1|0>"]
⍝ =>
"<9|9|0>" "<0|10|0>"

⍝ — Repeating rational invalid input
[ratsum:]←•load "lib/dyalog.bpl"
"<0|1|0>"("0123456789"ratsum)"<0||0>"
⍝ =>
⍝ error: DOMAIN ERROR

⍝ — Append a field through a record selected by dot indexing
rows←[("aa":1) ("bb":2)]
rows.[1].cc←3
rows   ⍝ [["aa":1] ["bb":2 "cc":3]]

⍝⍝ Name inspection

⍝ — Name classes follow lexical bindings, including operators
x←7
f←+
op←¨
fold←/
•nc "x" "f" "op" "fold" "absent" "bad name"
⍝ =>
[2 3 4 4 0 ¯1]ₓ

⍝ — A glyph and a system name both name a function, and an unknown system name is undefined
•nc "+" "•json" "•bogus"   ⍝ [3 3 0]ₓ

⍝ — Each row of a character matrix is one name, as each string of an array of strings is
•nc ["•c"⋄"•r"]   ⍝ [3 3]ₓ

⍝ — Name lists are sorted, filtered by prefix and class
zeta←1
mean←{+/⍵}
member←+
"me" •nl 3   ⍝ "mean" "member"

⍝ — The nearest binding determines the visible class
x←1
f←{x←+ ⋄ local←2 ⋄ [•nc "x";•nl 2]}
f0   ⍝ [3ₓ;,⊂"local"]

⍝ — Expunging a local reveals its outer binding; handles retain definitions
x←1
f←{x←2 ⋄ erased←•ex "x" ⋄ x}
kept←f
•ex "f"
kept0   ⍝ 1

⍝ — Erasure is idempotent and protects implicit/system names
x←1 ⋄ •ex "x" "x" "bad name" "$a" "⍵"
[1 1 0 0 0]ₓ

⍝ — Source retains the definition, while derived functions use APL display
f←{⍵+1}
plus←+
•src "f" "plus"   ⍝ "{⍵+1}" "+"

⍝ — Batch inspection retains its frame and its empty result type
[⍴•nc ["aa" "bb"⋄"cc" "dd"];•nc 0⍴⊂"";•src 0⍴⊂""]
[[2 2]ₓ ⍬ₓ 0⍴⊂""]

⍝ — Inspection maps over names at any depth
•nc ["a" ["b" "c"]]   ⍝ [0ₓ [0 0]ₓ]

⍝ — An empty list of names gives an empty result
•nc ⍬   ⍝ ⍬ₓ

⍝ — Values have no function source
x←1 ⋄ •src "x"
⍝ error: DOMAIN ERROR

⍝ — Undefined source is a value error
•src "absent"
⍝ error: VALUE ERROR

⍝ — Listing rejects unsupported classes
•nl 9
⍝ error: DOMAIN ERROR

⍝⍝ Numeric text input

⍝ — Validity and values; minus is accepted without evaluating expressions
•vfi "12 nope -3 1.5 1+2"
[[1 0 1 1 0]ₓ;12 0 ¯3 1.5 0]

⍝ — Preserve numeric domains, signed exponents, fractions and complex components
•vfi "+2 3ₓ -4ₓ 1r3 -2r-3 1e-2 1j-2 ∞ -∞"
[9⍴1ₓ;2 3ₓ ¯4ₓ 1r3 2r3 0.01 1j¯2 ∞ ¯∞]

⍝ — An invalid field's zero never makes exact numbers approximate
•vfi "2ₓ nope"   ⍝ [[1 0]ₓ;2ₓ 0]

⍝ — Invalid fractions, malformed numbers and code remain invalid fields. The text NaN is a valid number, as Rust reads it
•vfi "1r0 1x2 . NaN ⎕←7"
[[0 0 0 1 0]ₓ [0 0 0 $n 0]]

⍝ — Explicit separators retain empty fields as valid zero
",;" •vfi ",3.9;2.4,,76,"
[6⍴1ₓ;0 3.9 2.4 0 76 0]

⍝ — Explicit separators trim whitespace but do not split on it
'⋄' •vfi "1 ⋄ 2 3 ⋄ 4 "
[[1 0 1]ₓ;1 0 4]

⍝ — Default whitespace splitting and empty result domains
[•vfi ' ';•vfi "";•vfi '2',(•ucs 9 10),'3']
[[⍬ₓ 0⍴0] [⍬ₓ 0⍴0] [[1 1]ₓ;2 3]]

⍝ — Empty separator list treats its input as one field
"" •vfi "1 2"
[[0]ₓ ,0]

⍝ — Numeric arrays are not text
•vfi 1 2
⍝ error: DOMAIN ERROR

⍝⍝ Timing

⍝ — •time t gives the seconds since t, for each item
t←•time 0 ⋄ [0≤•time t;⍴•time 2 3⍴t;(•storage •time t).kind]   ⍝ [$t;[2 3]ₓ;"float64"]

⍝ — F •time x gives each function's fastest time per call, in the shape and keys of F
T←["sum":+/] •time ⍳10ₓ
[0≤T.sum;(•storage T).kind;⍴ ([1 1]⍴[+/]) •time ⍳10ₓ;⍴ ⍬ •time 1]
⍝ =>
[$t "float64" [1 1]ₓ [0]ₓ]

⍝ — The left argument holds functions
1 2 •time 3
⍝ error: DOMAIN ERROR

⍝⍝ Dates

⍝ — •time 0 is the current moment, in seconds since the Unix epoch
1714990830<•time 0   ⍝ $t

⍝ — •date reads RFC 3339, a date and time with no offset, and a date alone
•date "2024-05-06T10:20:30+02:00" "1970-01-01T00:00:01.5" "1970-01-02"   ⍝ 1714983630 1.5 86400

⍝ — •date⁻¹ writes each moment's fields, which •date reads back
R←•date⁻¹ 0 1714990830
[R.year;R.weekday;R.iso_week;•date R]
⍝ =>
[[1970 2024]ₓ;[4 1]ₓ;[1 19]ₓ;0 1714990830]

⍝ — A pattern reads and writes text, and a locale names months and days
["pattern":"%A %e %B" "locale":"de_DE"] •date⁻¹ 1714990830   ⍝ "Montag  6 Mai"

⍝ — A pattern also reads text
"%d/%m/%Y" •date "06/05/2024"   ⍝ 1714953600

⍝ — The zone option places text with no offset
["zone":3600] •date "2024-05-06T10:20:30"   ⍝ 1714987230

⍝ — Under a bound pattern, a moment changes in its own text
(86400+)@("%Y-%m-%d"↣•date) "2024-12-31"   ⍝ "2025-01-01"

⍝ — Text that no pattern matches
•date "May 6"
⍝ error: DOMAIN ERROR

⍝ — A record needs a year
•date ["month":5]
⍝ error: DOMAIN ERROR

⍝⍝ Paths and the filesystem

⍝ — •path splits paths into parts, and •path⁻¹ joins them
P←•path "a/b.tsv" "c" ⋄ [P.parent;P.extension;•path⁻¹ P]   ⍝ [["a" ""];["tsv" ""];"a/b.tsv" "c"]

⍝ — The absolute option joins a relative path to the working directory
(["absolute":1] •path "a/b").parent≡•path⁻¹ ["parent":(•host "cwd") "stem":"a"]   ⍝ $t

⍝ — •readdir lists a directory, its subdirectories with recurse, and entries that match a glob
•mkdir testpath,"/s"
(testpath,"/a.txt") •nput "hi"
(testpath,"/s/b.txt") •copy testpath,"/a.txt"
[(•readdir testpath).name (["recurse":1] •readdir testpath).name (["recurse":1 "glob":"*.txt"] •readdir testpath).name]
⍝ =>
[["a.txt" "s"] ["a.txt" "s" "b.txt"] ["a.txt"]]

⍝ — •metadata describes each path, including a missing one
testpath •nput "hello"
M←•metadata testpath (testpath,"x")
[M.kind M.len M.readable M.target]
⍝ =>
[["file" "none"] [5 0]ₓ [$t $f] ["" ""]]

⍝ — •copy copies a directory with everything in it
•mkdir testpath,"/s"
(testpath,"/s/b") •nput "x"
(testpath,"/t") •copy testpath,"/s"
(•readdir testpath,"/t").name   ⍝ ["b"]

⍝ — •rename moves an entry, and •remove removes it
d←•mkdir testpath
(d,"/a") •nput "x"
(d,"/b") •rename d,"/a"
n←(•readdir d).name
•remove d,"/b"
[n ≢(•readdir d).path]   ⍝ [["b"];0ₓ]

⍝ — A unique directory stays until removed, and recurse removes a directory's contents
u←["unique":1] •mkdir testpath
k←(•metadata u).kind
["recurse":1] •remove testpath
[k (•metadata testpath).kind]
⍝ =>
[["dir"] ["none"]]

⍝ — A unique directory's name starts with the prefix option
u←["unique":1 "prefix":"run"] •mkdir testpath
"run"≡3↑(•path u).name   ⍝ $t

⍝ — The prefix option needs unique
["prefix":"run"] •mkdir testpath
⍝ error: DOMAIN ERROR

⍝ — Removing a directory that isn't empty needs recurse
•mkdir testpath,"/s" ⋄ •remove testpath
⍝ error: IO ERROR

⍝⍝ Standard input

⍝ — Reference sessions have no standard input, so reading ⎕ or •nget "-" is an error
⎕
⍝ error: IO ERROR

⍝ —
•nget "-"
⍝ error: IO ERROR
