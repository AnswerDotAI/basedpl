⍝⍝ Vector-axis reduction

⍝ reduce-axes — Selected axes form one ravelled cell; remaining axes form the result frame
n←2 2 3⍴⍳6
(+/⍤[2 3]n ⋄ +⌿⍤[1 3]n ⋄ +/⍤[3 2]n)
⍝ =>
(21 21 ⋄ 12 30 ⋄ 21 21)

⍝ reduce-axes-order — Axis order determines the ravel, not a sequence of reductions
(-/⍤[1 2]2 2⍴⍳4 ⋄ -/⍤[2 1]2 2⍴⍳4)   ⍝ ((⊂¯2) ⋄ ⊂¯4)

⍝ reduce-axes-binding — Glyphs, named hybrids and already-derived reductions use the same axes
r←/ ⋄ sum←+/ ⋄ n←2 2 3⍴⍳6
(+r⍤[2 3]n ⋄ sum⍤[2 3]n ⋄ (+/)⍤[2 3]n)
⍝ =>
(21 21 ⋄ 21 21 ⋄ 21 21)

⍝ reduce-axes-empty — Empty cells use identities, empty frames retain prototypes, no axes means singleton cells
(+/⍤[2 3]2 0 3⍴0 ⋄ +/⍤[2 3]0 2 3⍴0 ⋄ +/⍤[⍬]2 2⍴⍳4)
(0 0 ⋄ ⍬ ⋄ 2 2⍴⍳4)

⍝ reduce-axes-seed — Each cell receives the whole seed
(10 20)+/⍤[2 3]2 2 3⍴⍳6   ⍝ (31 41 ⋄ 31 41)

⍝ reduce-axes-keys — Unselected axes retain their labels
+/⍤[2 3]"aa" "bb":2 2 3⍴⍳6   ⍝ "aa" "bb":21 21

⍝ reduce-axes-calls — Generic reductions retain right association and Each's cell order
f←{⎕←⍺ ⋄ ⍺-⍵} ⋄ f/⍤[2 3]2 2 2⍴⍳8
¯2 ¯2
⍝ ⎕: 3\n2\n1\n7\n6\n5

⍝ reduce-axes-prototype — An empty frame invokes the reduction on its prototype cell through Each
f←{⎕←⍺ ⋄ ⍺+⍵} ⋄ f/⍤[2 3]0 2 2⍴0
⍬
⍝ ⎕: 0\n0\n0

⍝ reduce-axes-repeat — Axes are distinct
+/⍤[2 2]2 2⍴⍳4
⍝ error: DOMAIN ERROR

⍝ reduce-axes-rank — An axis specification is a scalar or vector
+/⍤[[1 2 ⋄]] 2 2⍴⍳4
⍝ error: RANK ERROR

⍝⍝ Axis keys

⍝ axis-description-single — A one-position description labels the existing axis without enclosing its values
("row":"only"):,7   ⍝ ("row":1)⍴"only":⍤[1],7

⍝ axis-description-collision — Axis descriptions cannot duplicate a surviving axis name
("city":"Jan" "Feb" "Mar"):⍤[2]("city":2 ⋄ 3)⍴0
⍝ error: DOMAIN ERROR

⍝ axis-shape — The shape is keyed by axis names, and reshaping by a keyed shape names the axes
M←("city":2 ⋄ "month":3)⍴⍳6
(⍴M ⋄ "month"⌷⍴M ⋄ ⍳⍤[1]⍴M ⋄ ⍴(:⍴M)⍴M ⋄ ⍴("a" "b":⍴M)⍴M)
⍝ =>
(("city":2ₓ ⋄ "month":3ₓ) ⋄ 3ₓ ⋄ "city" "month" ⋄ 2ₓ 3ₓ ⋄ ("a":2ₓ ⋄ "b":3ₓ))

⍝ axis-shape-gaps — An unnamed axis is an unnamed entry of the shape
A←("city":2 ⋄ 3)⍴1 ⋄ B←("product":4 ⋄ 3)⍴2
(⍴A+B ⋄ ⍳⍤[1]⍴A+B)
⍝ =>
(("city":2ₓ ⋄ 3ₓ ⋄ "product":4ₓ) ⋄ ("city" ⋄ 2ₓ ⋄ "product"))

⍝ axis-gap-agreement — Named entries match by name, and unnamed entries match in order
(("a":1 ⋄ 2)+(10 ⋄ "a":20) ⋄ ("a":1 ⋄ 2)+("b":10 ⋄ 20))
⍝ =>
(("a":21 ⋄ 12) ⋄ ("a":1 ⋄ 22 ⋄ "b":10))

⍝ —
("a":1 ⋄ 2)+("a":1 ⋄ 2 ⋄ 3)
⍝ error: LENGTH ERROR

⍝ axis-gap-json — A JSON object needs a key for every entry
•tojson ("a":1 ⋄ 2)
⍝ error: DOMAIN ERROR

⍝ axis-colon — Colon binds as a function; literal separators and dfn guards retain their meanings
f←: ⋄ a←"n" f 5
g←{⍵<0:-⍵ ⋄ +/(:("n":⍵))}
((:a) ⋄ ≢("a":1 ⋄ "b":2) ⋄ g ¯3 ⋄ g 3)
⍝ =>
(,5 ⋄ 2ₓ ⋄ 3 ⋄ 3)

⍝ axis-pair-literal — A literal with key:value items builds one keyed vector, other items are unnamed entries, and display reads back
x←3
r←("one":1 2 ⋄ "two":x)
(r≡"one" "two":(1 2 ⋄ 3) ⋄ r≡⍎⍕r ⋄ ("a":1 ⋄ 2)≡("a":1),2 ⋄ (("a":1) ⋄ ("b":2))≡["a":1],["b":2])
⍝ =>
(1ₓ ⋄ 1ₓ ⋄ 1ₓ ⋄ 1ₓ)

⍝ axis-page-header — Each page of a keyed array of rank 3 or more is headed by the index that selects it
⎕←("pp":"x1" "x2" ⋄ "rr":"r1" "r2" ⋄ "cc":"c1" "c2"):2 2 2⍴⍳8 ⋄ 0
0
⍝ ⎕: "x1"⌷\n   c1 c2\nr1  1  2\nr2  3  4\n\n"x2"⌷\n   c1 c2\nr1  5  6\nr2  7  8

⍝ axis-colon-order — Construction uses ordinary right-to-left evaluation
:((⎕←'a'):(⎕←7))
,7
⍝ ⎕: 7\n'a'

⍝ axis-construct — Attach keys to axes without nesting the matrix; Iota with an axis inspects them
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
(⍴M ⋄ ⍳⍤[1]M ⋄ ⍳⍤[2]M ⋄ ⍳⍤[1 2](:M) ⋄ :M)
⍝ =>
(2ₓ 2ₓ ⋄ "alice" "bob" ⋄ "price" "qty" ⋄ (1ₓ 2ₓ ⋄ 1ₓ 2ₓ) ⋄ [10 2 ⋄ 20 4])

⍝ axis-selection — Atomic row selection retains column keys; one string is one selector
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
("bob"⌷M ⋄ ∞ "price"⌷M ⋄ "alice" "qty"⌷M ⋄ [,⊂"alice"]⌷M)
⍝ =>
(("price" "qty":20 4) ⋄ ("alice" "bob":10 20) ⋄ 2 ⋄ ("alice":⍤[1]("price" "qty":⍤[2][10 2 ⋄])))

⍝ axis-insertion — Appending both axes fills every new coordinate with the original prototype
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
M.("cara" "tax")←3
M
⍝ =>
("alice" "bob" "cara" ⋄ "price" "qty" "tax"):[10 2 0 ⋄ 20 4 0 ⋄ 0 0 3]

⍝ axis-agreement — Independently union axes, including coordinates absent from both inputs
A←("alice":⍤[1]("price":⍤[2][10 ⋄]))
B←("bob":⍤[1]("qty":⍤[2][2 ⋄]))
A+B
⍝ =>
("alice" "bob" ⋄ "price" "qty"):[10 0 ⋄ 0 2]

⍝ axis-singleton — Broadcasting expands positions; enclosure keeps a single named value
((("base":5)+10 20 30) ⋄ ("base":5)+⊂10 20 30)
(15 25 35 ⋄ ("base":15 25 35))

⍝ axis-iota — Iota without an axis uses dimension values, never the array's keys
⍳("rows" "cols":2 3)   ⍝ ⍳2 3

⍝ axis-structure — Transpose moves labels; reduction removes only the reduced axis
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
(+/M ⋄ +⌿M ⋄ ⍉M ⋄ ⌽M ⋄ 1↑M)
⍝ =>
(("alice" "bob":12 24) ⋄ ("price" "qty":30 6) ⋄ (("price" "qty" ⋄ "alice" "bob"):[10 20 ⋄ 2 4]) ⋄ (("alice" "bob" ⋄ "qty" "price"):[2 10 ⋄ 4 20]) ⋄ ("alice":⍤[1]("price" "qty":⍤[2][10 2 ⋄])))

⍝ axis-reshape — Shape-changing reshape drops keys and may cycle values
4⍴"aa" "bb":1 2   ⍝ 1 2 1 2

⍝ axis-replicate — An unkeyed axis may repeat while another axis retains its keys
2/("aa" "bb":[1 2 ⋄ 3 4])   ⍝ "aa" "bb":[1 1 2 2 ⋄ 3 3 4 4]

⍝ axis-repeat — A labelled axis cannot repeat a position
2⌿("aa" "bb":[1 2 ⋄ 3 4])
⍝ error: DOMAIN ERROR

⍝ axis-search — Outer labels do not restrict value search; found positions return labels
A←"left":7 ⋄ B←"right":7
(A∊B ⋄ A⍳B ⋄ A∩B ⋄ A∪B ⋄ A,B)
⍝ =>
(("left":1ₓ) ⋄ ("right":"left") ⋄ ("left":7) ⋄ ("left":7) ⋄ ("left" "right":7 7))

⍝ axis-indices — Grade, interval index and index-of return keys, retaining numeric sentinels
V←"low" "mid" "high":10 20 30
(⍒V ⋄ V⍳20 99 ⋄ V⍸5 15 25 40 ⋄ [⍒V]⌷V)
⍝ =>
("high" "mid" "low" ⋄ ("mid" ⋄ 4ₓ) ⋄ (0ₓ ⋄ "low" ⋄ "mid" ⋄ "high") ⋄ ("high" "mid" "low":30 20 10))

⍝ axis-match — Match aligns keys independently on both axes
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
M≡⌽⊖M
⍝ =>
1ₓ

⍝ axis-outer — Outer product keeps each argument's labelled axes
("aa" "bb":1 2)+⌝("xx" "yy":10 20)
("aa" "bb" ⋄ "xx" "yy"):[11 21 ⋄ 12 22]

⍝ axis-rank — Frame keys survive assembly; cell axes need the same labels in every result
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
((+/⍤1)M ⋄ (⊢⍤1)M ⋄ (⌽⍤1)M ⋄ ({10=↑⍵:⌽⍵ ⋄ ⍵}⍤1)M)
⍝ =>
(("alice" "bob":12 24) ⋄ (("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]) ⋄ (("alice" "bob" ⋄ "qty" "price"):[2 10 ⋄ 4 20]) ⋄ ("alice" "bob":[2 10 ⋄ 20 4]))

⍝ axis-rank-union — Rank aligns frame keys before applying the cell function
A←"alice" "bob":[1 2 ⋄ 3 4] ⋄ B←"bob" "cara":[10 20 ⋄ 30 40]
A(+⍤1)B
⍝ =>
"alice" "bob" "cara":[1 2 ⋄ 13 24 ⋄ 30 40]

⍝ axis-windows — Sliding frames are unkeyed; stencil centres keep their axis labels
V←"aa" "bb" "cc":1 2 3
(2↕V ⋄ 3↕V ⋄ ({+/⍵}⌺3)V ⋄ ({(⍳⍤[1]⍵)≡"aa" "bb" "cc"}⌺3)V)
⍝ =>
(([1 2 ⋄ 2 3]) ⋄ ("aa" "bb" "cc":⍤[2][1 2 3 ⋄]) ⋄ ("aa" "bb" "cc":3 6 5) ⋄ ("aa" "bb" "cc":0ₓ 1ₓ 0ₓ))

⍝ axis-coordinates — Pick, Squad and coordinate indexing resolve each axis independently
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
(("bob" "qty")⊃M ⋄ ("bob" "qty")⌷M ⋄ "alice"⊃M ⋄ [⊂"alice" "price"]⌷M ⋄ ↑⍸M=20)
⍝ =>
(4 ⋄ 4 ⋄ ("price" "qty":10 2) ⋄ (⊂10) ⋄ "bob" "price")

⍝ axis-explicit — Explicit scalar axes align the labels on those axes
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
M+⍤[2]("qty" "tax":10 2)
⍝ =>
("alice" "bob" ⋄ "price" "qty" "tax"):[10 12 2 ⋄ 20 14 2]

⍝ axis-assembly — Added axes are unkeyed; unchanged axes retain labels
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
((⊃⊂⍤[2]M) ⋄ ,⍤[1.5]M ⋄ ⍪"aa" "bb":1 2)
⍝ =>
((("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]) ⋄ (("alice" "bob" ⋄ "price" "qty"):⍤[1 3]2 1 2⍴10 2 20 4) ⋄ ("aa" "bb":[1 ⋄ 2]))

⍝ axis-permuted-agreement — Explicit axes carry keys with their mapped dimensions
X←("xx" "yy" "zz" ⋄ "aa" "bb"):3 2⍴⍳6
(2 3 1⍴0)+⍤[2 1]X
⍝ =>
("aa" "bb" ⋄ "xx" "yy" "zz"):⍤[1 2]2 3 1⍴1 3 5 2 4 6

⍝ axis-compact-union — Compact integer and float paths use the same missing-position fill
X←"aa" "bb":2ₓ 3ₓ ⋄ Y←"bb" "cc":5ₓ 7ₓ
(X+Y ⋄ X×Y ⋄ X<Y ⋄ (X+0)+Y ⋄ X+¨Y)
⍝ =>
(("aa" "bb" "cc":2ₓ 8ₓ 7ₓ) ⋄ ("aa" "bb" "cc":0ₓ 15ₓ 0ₓ) ⋄ ("aa" "bb" "cc":0ₓ 1ₓ 1ₓ) ⋄ ("aa" "bb" "cc":2 8 7ₓ) ⋄ ("aa" "bb" "cc":2ₓ 8ₓ 7ₓ))

⍝ axis-partition — Partition cells retain sliced labels; group axes are new and unkeyed
M←("alice" "bob" ⋄ "xx" "yy" "zz"):2 3⍴⍳6
(1 1 2⊆M ⋄ 1 0 1⊂M)
⍝ =>
(("alice" "bob":2 2⍴(("xx" "yy":1 2) ⋄ ("zz":3) ⋄ ("xx" "yy":4 5) ⋄ ("zz":6))) ⋄ ((("alice" "bob" ⋄ "xx" "yy"):[1 2 ⋄ 4 5]) ⋄ (("alice" "bob" ⋄ ,⊂"zz"):[3 ⋄ 6])))

⍝ axis-selective-write — Keyed RHS follows selected labels; Pick of a row updates its elements
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
("alice"⊃M)←"qty" "price":7 8
(⌽M)←("bob" "alice" ⋄ "price" "qty"):[30 3 ⋄ 40 4]
M
⍝ =>
("alice" "bob" ⋄ "price" "qty"):[40 4 ⋄ 30 3]

⍝ axis-catenate-alignment — Catenate aligns non-joined keys and concatenates joined labels
A←("aa" "bb" ⋄ "xx" "yy"):[1 2 ⋄ 3 4]
B←("bb" "aa" ⋄ ,⊂"zz"):[5 ⋄ 6]
A,B
⍝ =>
("aa" "bb" ⋄ "xx" "yy" "zz"):[1 2 6 ⋄ 3 4 5]

⍝ axis-empty-shapes — Empty axes preserve other dimensions and their labels
M←("xx" "yy":⍤[2]0 2⍴0)
(⍳⍤[2]M ⋄ ⍉M ⋄ +/M ⋄ (⊢⍤1)M ⋄ (0⍴0)⌽⍤[2]M)
⍝ =>
("xx" "yy" ⋄ ("xx" "yy":2 0⍴0) ⋄ (0⍴0) ⋄ ("xx" "yy":⍤[2]0 2⍴0) ⋄ (0 2⍴0))

⍝ axis-numeric-frames — Number-theory and radix results retain their argument frames
V←"aa" "bb":1 2
(ℙV ⋄ ⊤V ⋄ 2⊥⊤V ⋄ ("days" "hours":0 24)⊤("aa" "bb":25 50))
⍝ =>
(("aa" "bb":2ₓ 3ₓ) ⋄ ("aa" "bb":⍤[2][0 1 ⋄ 1 0]) ⋄ ("aa" "bb":1 2) ⋄ (("days" "hours" ⋄ "aa" "bb"):[1 2 ⋄ 1 2]))

⍝ axis-product-frame — Inner product retains uncontracted axes
A←"aa" "bb":[1 2 ⋄ 3 4] ⋄ B←"xx" "yy":⍤[2][5 6 ⋄ 7 8]
A+.×B
⍝ =>
("aa" "bb" ⋄ "xx" "yy"):[19 22 ⋄ 43 50]

⍝ axis-native-rank — Native numerical cell functions align labelled frames like Rank
P←"aa" "bb":[1 2 ⋄ 10 3] ⋄ X←"bb" "aa":4 5
(P⊛X ⋄ ("aa" "bb":5 5)ℙ("bb" "aa":10 9))
⍝ =>
(("aa" "bb":11 22) ⋄ ("aa" "bb":6ₓ 4ₓ))

⍝ axis-matrix-layout — Inversion swaps axes; a solution retains the coefficient column axis
M←("r1" "r2" ⋄ "xx" "yy"):[2ₓ 0ₓ ⋄ 0ₓ 4ₓ]
(⌹M ⋄ 4ₓ 12ₓ⌹M)
⍝ =>
((("xx" "yy" ⋄ "r1" "r2"):[1r2 0ₓ ⋄ 0ₓ 1r4]) ⋄ ("xx" "yy":2ₓ 3ₓ))

⍝ axis-power-frame — Array-valued iteration counts supply the result frame
N←"initial" "once" "twice":0 1 2
(2∘×)⍣N⊢3
⍝ =>
"initial" "once" "twice":3 6 12

⍝ axis-contract — Contracted axes pair names, retaining the left contraction order
A←"hi" "lo":1 2 ⋄ B←"lo" "hi":10 20
M←("r1" "r2" ⋄ "xx" "yy"):[2ₓ 0ₓ ⋄ 0ₓ 4ₓ]
(A+.×B ⋄ ("hi" "lo":10 10)⊥("lo" "hi":2 1) ⋄ ("r2" "r1":12ₓ 4ₓ)⌹M)
⍝ =>
(40 ⋄ 12 ⋄ ("xx" "yy":2ₓ 3ₓ))

⍝ axis-complex-parts — Complex decomposition adds an unkeyed axis after the original axes
∨"first" "second":3j4 5j12   ⍝ "first" "second":[3 4 ⋄ 5 12]

⍝ axis-gradient — VJP aligns output labels and preserves input coordinate labels
f←1ₓ 2ₓ 3ₓ∘⊛ ⋄ g←[[1ₓ 2ₓ 0ₓ ⋄ 1ₓ 0ₓ 2ₓ]]∘⊛
(("bb" "aa":20ₓ 10ₓ)(f∂)("aa" "bb":1ₓ 2ₓ) ⋄ g∂⊂"xx" "yy":3ₓ 4ₓ)
⍝ =>
(("aa" "bb":80ₓ 280ₓ) ⋄ ⊂"xx" "yy":6ₓ 8ₓ)

⍝ axis-contract-missing — Labelled contractions require the same key set, including singletons
("aa":1)+.×("bb":2)
⍝ error: LENGTH ERROR

⍝ axis-decode-missing — Decode cannot invent a radix position
("hi" "lo":10 10)⊥("hi" "other":1 2)
⍝ error: LENGTH ERROR

⍝ axis-inverse-layout — Inverse scan and outer-product inversion retain surviving axis labels
V←"aa" "bb":1 3 ⋄ B←"row1" "row2":10 20
f←(×∘*)⍨
((+\)⍣¯1⊢V ⋄ (B∘(+⌝))⍣¯1⊢⊖B+⌝V ⋄ ⍳⍤[1] f⍣¯1⊢f V)
⍝ =>
(("aa" "bb":1 2) ⋄ ("aa" "bb":1 3) ⋄ "aa" "bb")

⍝ axis-nested-matrix-write — Named insertion uses the same axis extension inside stored matrices
T←"data":("r1" "r2" ⋄ "xx" "yy"):[1 2 ⋄ 3 4]
T.data.("r3" "zz")←9 ⋄ ("r4"⊃T.data)←10 11 12 ⋄ T.data
⍝ =>
("r1" "r2" "r3" "r4" ⋄ "xx" "yy" "zz"):[1 2 0 ⋄ 3 4 0 ⋄ 0 0 9 ⋄ 10 11 12]

⍝ axis-selector-once — Preparing named insertion evaluates a computed selector once
T←"data":"aa":1 ⋄ calls←0 ⋄ T.data.({calls+←1 ⋄ "bb"}0)←2 ⋄ calls
1

⍝ axis-key-groups — Key returns named positions monadically and slices labelled value cells dyadically
V←"aa" "bb" "cc":1 2 1
({⊂⍵}⌸V ⋄ 1 2 1{⊂⍵}⌸V)
⍝ =>
((("aa" "cc") ⋄ ,⊂"bb") ⋄ (("aa" "cc":1 1) ⋄ ("bb":⍤[1],2)))

⍝ axis-solve-missing — Solve cannot invent an equation
("r1" "other":4 12)⌹("r1" "r2":[2 0 ⋄ 0 4])
⍝ error: LENGTH ERROR

⍝⍝ Operand glyphs

⍝ — Pipeline stages use ordinary APL binding, then apply left to right
1+2×3 → 2∘× → -∘1   ⍝ 13

⍝ — Assignment encloses the whole pipeline
r←s←⍳4 → +/ → √ ⋄ r s   ⍝ (√10⋄ √10)

⍝ — Parentheses select pipeline boundaries
1+(⍳4 → +/)   ⍝ 11

⍝ — Pipe bodies still classify defined operators
op←{⍵→⍶→⍹} ⋄ (-op|)3   ⍝ 3

⍝ — A pipeline may supply a guard condition or result
{⍵→0∘<:⍵→⍲ ⋄ 0}3   ⍝ 9

⍝ — Stages see earlier effects and run exactly once
v←0 ⋄ 3 → {v+←1 ⋄ ⍵+v} → {v+←10 ⋄ ⍵+v}   ⍝ 15

⍝ — A stage is a function, not another value
3→4
⍝ error: SYNTAX ERROR

⍝ — Empty stages are structurally invalid
3→→+
⍝ error: SYNTAX ERROR

⍝ — Stage assignment requires grouping
3→f←+
⍝ error: SYNTAX ERROR

⍝ — Parenthesized assignments may construct stages
3→(f←2∘×)→f   ⍝ 12

⍝ — Adjacent argument glyphs strand; operand glyphs classify operators
{⍵⍵}3   ⍝ 3 3

⍝ — Left and right operands bind without spaces
2{⍶+⍹×⍵}3⊢4   ⍝ 14

⍝⍝ Mathematical monads

⍝ — Real and imaginary parts form a trailing pair axis
∨3j4 5 0j¯2   ⍝ [3 4 ⋄ 5 0 ⋄ 0 ¯2]

⍝ — Exact real parts stay exact
∨1r3   ⍝ 1r3 0ₓ

⍝ — Magnitude and phase, in radians
∧3j4 0j1   ⍝ [5 0.9272952180016122 ⋄ 1 1.5707963267948966]

⍝ — Empty decomposition retains frame and pair axis
⍴∨2 0⍴0ₓ   ⍝ 2ₓ 0ₓ 2ₓ

⍝ — Self-classification keeps classes in first-occurrence order
="aba"   ⍝ [1ₓ 0ₓ 1ₓ ⋄ 0ₓ 1ₓ 0ₓ]

⍝ — Self-classify major cells
=[1 2 ⋄ 3 4 ⋄ 1 2]   ⍝ [1ₓ 0ₓ 1ₓ ⋄ 0ₓ 1ₓ 0ₓ]

⍝ — A scalar has one class and one item
=7   ⍝ [1ₓ ⋄]

⍝ — No items, no classes
=⍬   ⍝ 0 0⍴0ₓ

⍝ — Zero-width rows are equal
=3 0⍴0   ⍝ 1 3⍴1ₓ

⍝ — Binary encode chooses enough first-axis digits
⊤2ₓ 5ₓ   ⍝ [0ₓ 1ₓ ⋄ 1ₓ 0ₓ ⋄ 0ₓ 1ₓ]

⍝ — Binary decode uses the existing first digit axis
⊥[0ₓ 1ₓ ⋄ 1ₓ 0ₓ ⋄ 0ₓ 1ₓ]   ⍝ 2ₓ 5ₓ

⍝ — Binary zero needs no digits
⊤0ₓ   ⍝ 0⍴0ₓ

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
≥≤1ₓ (2ₓ 3ₓ)   ⍝ 1ₓ (2ₓ 3ₓ)

⍝ — Increment promotes on overflow
≥9223372036854775807ₓ   ⍝ 9223372036854775808ₓ

⍝ — Square promotes on overflow
⍲3037000500ₓ   ⍝ 9223372037000250000ₓ

⍝ — Doubling preserves rational values
⍱1r3   ⍝ 2r3

⍝ — Square and double preserve empty exact prototypes
(⍲0⍴0ₓ ⋄ ⍱0⍴0ₓ)   ⍝ (0⍴0ₓ⋄ 0⍴0ₓ)

⍝ — Square root extends into complex numbers
√0 9 ¯4 3j4   ⍝ 0 3 0j2 2j1

⍝ — Exact perfect rational roots
√1r9 18446744073709551616ₓ   ⍝ 1r3 4294967296ₓ

⍝ — Irrational roots become approximate
√2ₓ   ⍝ 1.4142135623730951

⍝ — Odd integral roots of negative reals use the real branch
3 ¯3√¯8   ⍝ ¯2 ¯0.5

⍝ — Negative exact degrees reciprocate
¯3ₓ√¯8ₓ   ⍝ ¯1r2

⍝ — Non-integral degree uses principal power
0.5√¯2   ⍝ 4

⍝ — Complex square root uses the principal branch
1E¯14>|(√¯3j¯4)-1j¯2   ⍝ 1ₓ

⍝ — Zero degree is undefined
0√2
⍝ error: DOMAIN ERROR

⍝ — Zero cannot have a negative-degree root
¯3√0
⍝ error: DOMAIN ERROR

⍝ — Pi fractions and multiples
(π2⋄ 1π2⋄ 2π3)
6.283185307179586 1.5707963267948966 2.0943951023931953

⍝ — Unit-circle points
○(0⋄ 1π2⋄ π1)   ⍝ 1 0j1 ¯1

⍝ — Complex angles use exp(i z)
○0j1   ⍝ 0.36787944117144233

⍝ — Principal inverses of root and circle
((√⍣¯1)3⋄ (○⍣¯1)○0.5)   ⍝ 9 0.5

⍝ — Bound pi fractions invert either argument
((1∘π)⍣¯1)1π4   ⍝ 4

⍝⍝ Function arrays

⍝ — Pick a function from a vector, then call it
fs←+˘×˘÷ ⋄ mul←2⊃fs ⋄ 2 mul 3   ⍝ 6

⍝ — Primitives, dfns and derived functions share one function vector
fs←(+/)˘{⍵×⍵}˘(3∘+) ⋄ square←2⊃fs ⋄ square 4   ⍝ 16

⍝ — Reverse and disclose a function vector
fs←+˘×˘÷ ⋄ div←↑⌽fs ⋄ 6 div 3   ⍝ 2

⍝ — A function vector remains one nested item in an explicit strand
fs←+˘× ⋄ gs←fs˘÷ ⋄ f←2⊃1⊃gs ⋄ 2 f 3   ⍝ 6

⍝ — Overtake preserves existing functions
fs←+˘× ⋄ f←1⊃3↑fs ⋄ 2 f 3   ⍝ 5

⍝ — Function-vector fill retains the first function
fs←+˘× ⋄ f←3⊃3↑fs ⋄ 2 f 3   ⍝ 5

⍝ — Empty function vectors retain their positions in a strand
fs←+˘× ⋄ f←3⊃(0↑fs)˘(0↑fs)˘÷ ⋄ 6 f 3   ⍝ 2

⍝ — Arrays and functions can share a mixed vector
fs←+˘× ⋄ result←[1 2 3],2⌷fs ⋄ back←2⊃result ⋄ 2 back 3   ⍝ 6

⍝ — A pick path descends into a nested function vector
x←(+˘×)(-˘÷) ⋄ f←2⊃1⊃x ⋄ 2 f 3   ⍝ 6

⍝ — A dfn can return a selected function
fs←+˘× ⋄ f←{2⊃⍵}fs ⋄ 2 f 3   ⍝ 6

⍝ — Inverse enclose discloses a callable function
fs←+˘× ⋄ f←⊂⍣¯1⊢[⊂2]⌷fs ⋄ 2 f 3   ⍝ 6

⍝ — Each assembles function results back into an array
fs←+˘× ⋄ f←↑↑¨fs ⋄ 2 f 3   ⍝ 5

⍝ — Rank assembles function results back into an array
fs←+˘× ⋄ f←↑↑⍤0⊢fs ⋄ 2 f 3   ⍝ 5

⍝ — An empty pick path preserves the scalar array, without disclosing its function
fs←+˘× ⋄ fs.[⊂2]≡⍬⊃fs.[⊂2]   ⍝ 1ₓ

⍝ — A function vector matches itself by function identity
fs←+˘× ⋄ fs≡fs   ⍝ 1ₓ

⍝ — A selected function can use its still-active lexical binding
{a←⍵ ⋄ fs←{a+⍵}˘× ⋄ f←↑fs ⋄ f 3}4   ⍝ 7

⍝ — Passing a function vector to another dfn retains lexical lookup
use←{f←↑⍵ ⋄ f 3} ⋄ {a←⍵ ⋄ use {a+⍵}˘+}4   ⍝ 7

⍝ — A helper may return a function while its defining frame remains active
id←{↑⍵} ⋄ {a←⍵ ⋄ f←id {a+⍵}˘+ ⋄ f 3}4   ⍝ 7

⍝ — Returning a top-level function needs no escaping local frame
fs←{⍵×⍵}˘+ ⋄ f←{↑⍵}fs ⋄ f 3   ⍝ 9

⍝ — Formatting a function vector produces character text
fs←+˘× ⋄ ⍴⍕fs   ⍝ ,6ₓ

⍝ — One strand may contain numbers, callable functions and text
x←1˘+˘"abc" ⋄ f←2⊃x ⋄ (1⊃x)f≢3⊃x   ⍝ 4

⍝ — Functions have no grade ordering
⍋+˘×
⍝ error: DOMAIN ERROR

⍝ — Interval index requires ordered data, not functions
(+˘×)⍸+˘×
⍝ error: DOMAIN ERROR

⍝ — A returned function vector cannot retain an expired local frame
{x←⍵ ⋄ {x+⍵}˘+}2
⍝ error: DOMAIN ERROR

⍝ — Assignment to a dot path cannot export a local function into an outer array
fs←+˘× ⋄ {fs.(1)←1⌷{⍵}˘+ ⋄ 0}2
⍝ error: DOMAIN ERROR

⍝ — Even an empty function vector retains its prototype function
{0↑{⍵}˘+}2
⍝ error: DOMAIN ERROR

⍝ — Disclosing before return does not permit an escaping closure
{x←⍵ ⋄ ↑{x+⍵}˘+}2
⍝ error: DOMAIN ERROR

⍝ — Directly returning an escaping closure is also rejected
{x←⍵ ⋄ {x+⍵}}2
⍝ error: DOMAIN ERROR

⍝ — Pick coordinates consume array axes
1 1⊃+˘×
⍝ error: RANK ERROR

⍝⍝ Explicit stranding

⍝ —
1˘2˘3   ⍝ 1 2 3

⍝ — Explicit stranding binds more tightly than adjacent implicit stranding
1 2˘3 4   ⍝ 1 (2 3) 4

⍝ — Parentheses make an inner strand one nested item
(1˘2)˘3   ⍝ (1 2) 3

⍝ — Parenthesized expressions become separate strand elements
(1+2)˘(3+4)   ⍝ 3 7

⍝ — Named vectors remain nested rather than being catenated
a←1 2 ⋄ b←3 4 ⋄ a˘b   ⍝ (1 2⋄ 3 4)

⍝ — A function strand can be one item within an implicit strand
x←1 +˘× 4 ⋄ f←2⊃2⊃x ⋄ (1⊃x)f 3⊃x   ⍝ 4

⍝ — The explicit strand forms before reduction applies
+/1˘2˘3   ⍝ 6

⍝ — Explicit strand elements evaluate right to left
a←0 ⋄ (a←1)˘a   ⍝ 1 0

⍝ — Explicit strands support destructuring and modified assignment
a˘b←1˘2 ⋄ a˘b+←3˘4 ⋄ a b   ⍝ 4 6

⍝ — A hybrid stored as an element supplies its function role on disclosure
fs←/˘+ ⋄ f←↑fs ⋄ 1 0 1 f 2 3 4   ⍝ 2 4

⍝ — A defined operator can put its function operand in a returned strand
op←{⍶˘+} ⋄ fs←(×op)0 ⋄ f←↑fs ⋄ 2 f 3   ⍝ 6

⍝⍝ Agenda

⍝ — Agenda chooses negate for a negative argument
cases←-˘⊢ ⋄ abs←{1+⍵≥0}◶cases ⋄ abs ¯3   ⍝ 3

⍝ — Agenda chooses identity for a nonnegative argument
cases←-˘⊢ ⋄ abs←{1+⍵≥0}◶cases ⋄ abs 3   ⍝ 3

⍝ — A constant selector chooses the second function
mul←2◶(+˘×) ⋄ 2 mul 3   ⍝ 6

⍝ — A negative selector counts from the end
mul←¯1◶(+˘×) ⋄ 2 mul 3   ⍝ 6

⍝ — The selector and selected function both receive the original arguments
choose←{1+⍺>⍵}◶(-˘÷) ⋄ 12 choose 3   ⍝ 4

⍝ — An agenda branch may itself return a function
choose←1◶({↑⍵}˘⊢) ⋄ f←choose (+˘×) ⋄ 2 f 3   ⍝ 5

⍝ — A singleton vector is not a scalar selector
(,1)◶(+˘×)
⍝ error: RANK ERROR

⍝ — The cases must be a vector, not a function-containing scalar
1◶[+]
⍝ error: RANK ERROR

⍝ — A selector function must return one scalar index
{1 2}◶(+˘×)⊢3
⍝ error: RANK ERROR

⍝ —
1.5◶(+˘×)
⍝ error: DOMAIN ERROR

⍝ —
1◶(0↑+˘×)
⍝ error: DOMAIN ERROR

⍝ —
1◶1 2
⍝ error: DOMAIN ERROR

⍝ —
{'a'}◶(+˘×)⊢3
⍝ error: DOMAIN ERROR

⍝ —
0◶(+˘×)
⍝ error: INDEX ERROR

⍝ —
3◶(+˘×)
⍝ error: INDEX ERROR

⍝ —
{3}◶(+˘×)⊢3
⍝ error: INDEX ERROR

⍝ — Agenda evaluates the selector, then only the selected branch
choose←{⎕←9 ⋄ 2}◶({⎕←1 ⋄ 1÷0}˘{⎕←2 ⋄ ⍺-⍵}) ⋄ 10 choose 3
7
⍝ ⎕: 9\n2

⍝⍝ Leading unit axis broadcasting

⍝ — Unit axes broadcast a column against a row; predicates return exact Booleans
[1 ⋄ 2]<[1 2 3 ⋄]   ⍝ [0ₓ 1ₓ 1ₓ ⋄ 0ₓ 0ₓ 1ₓ]

⍝ —
[1ₓ ⋄ 2ₓ]<[1ₓ 2ₓ 3ₓ ⋄]   ⍝ [0ₓ 1ₓ 1ₓ ⋄ 0ₓ 0ₓ 1ₓ]

⍝ — Rank pairs vector cells using broadcast frames
(2 1 2⍴1 2 3 4)(+⍤1)1 3 2⍴10 20 30 40 50 60
2 3 2⍴11 22 31 42 51 62 13 24 33 44 53 64

⍝ — Unit axes extend independently across three dimensions
(2 1 2⍴1 2 3 4)+1 3 1⍴10 20 30
2 3 2⍴11 12 21 22 31 32 13 14 23 24 33 34

⍝ — An explicit axis aligns the vector with columns rather than rows
(2 3⍴⍳6)+⍤[2]10 20 30   ⍝ [11 22 33 ⋄ 14 25 36]

⍝ — Axis permutation and unit-axis extension work together
[10 ⋄ 20]+⍤[2 1][1 ⋄ 2 ⋄ 3]   ⍝ [11 12 13 ⋄ 21 22 23]

⍝ — Broadcasting recurs inside nested arrays
[[1 ⋄ 2]]+⊂[10 20 30 ⋄]   ⍝ ⊂[11 21 31 ⋄ 12 22 32]

⍝ — Broadcast integer overflow promotes to exact big numbers
[9223372036854775807ₓ 1ₓ ⋄]+1ₓ 2ₓ
[9223372036854775808ₓ 2ₓ ⋄ 9223372036854775809ₓ 3ₓ]

⍝ — Rank-zero application extends a singleton frame
(⍳1)(+⍤0)⍳3   ⍝ 2 3 4

⍝ — A vector frame aligns with the leading matrix axis
1 2 (+⍤0)2 3⍴0   ⍝ [1 1 1 ⋄ 2 2 2]

⍝⍝ Selective assignment

⍝ — Selection through identity and disclose replaces the whole nested item
a←(1 2⋄ 3 4) ⋄ (⊢↑a)←7 8 9 ⋄ a   ⍝ (7 8 9⋄ 3 4)

⍝ — Identity selection may replace the whole array with a different shape
a←1 2 ⋄ (⊣a)←3 4 5 ⋄ a   ⍝ 3 4 5

⍝ — An empty pick path selects the whole scalar for replacement
a←1 ⋄ (⍬⊃a)←3 4 ⋄ a   ⍝ 3 4

⍝ — Whole-array replacement also works when the old array is empty
a←⍬ ⋄ (⍬⊃a)←3 4 ⋄ a   ⍝ 3 4

⍝ — Modified assignment through an empty pick path can resize the array
a←1 2 ⋄ (⍬⊃a),←3 4 ⋄ a   ⍝ 1 2 3 4

⍝ — Whole-item replacement composes with a nested pick
a←(1 2⋄ 3 4) ⋄ (⍬⊃1⊃a)←5 6 7 ⋄ a   ⍝ (5 6 7⋄ 3 4)

⍝ — Assignment through reverse maps replacements back to original positions
a←1 2 ⋄ (⍬⊃⌽a)←3 4 ⋄ a   ⍝ 4 3

⍝ — Ravel selection cannot resize its source through an empty pick path
a←1 2 ⋄ (⍬⊃,a)←3 4 5
⍝ error: LENGTH ERROR

⍝ — Squad selection still requires replacement-shape agreement
a←1 2 ⋄ (⍬⊃⍬⌷a)←2 2⍴3 4
⍝ error: LENGTH ERROR

⍝ — Replace vowels selected by a Boolean mask
a←"HELLO" ⋄ ((a∊"AEIOU")/a)←'*' ⋄ a   ⍝ "H*LL*"

⍝ — Assign through a ravel prefix without changing matrix shape
z←3 4⍴⍳12 ⋄ (5↑,z)←0 ⋄ ,z   ⍝ 0 0 0 0 0 6 7 8 9 10 11 12

⍝ — Repeated transpose axes select the diagonal
m←3 3⍴⍳9 ⋄ (1 1⍉m)←0 ⋄ ,m   ⍝ 0 2 3 4 0 6 7 8 0

⍝ — Selection through enlist updates characters inside nested vectors
a←"Andy" "Karen" "Liam" ⋄ (('a'=∊a)/∊a)←'*' ⋄ a
"Andy" "K*ren" "Li*m"

⍝ — Each selects a prefix in every nested vector
a←"HELLO" "WORLD" ⋄ (2↑¨a)←'*' ⋄ a   ⍝ "**LLO" "**RLD"

⍝ — Each uses a separate mask for each nested vector
a←"HELLO" "WORLD" ⋄ ((a='O')/¨a)←'*' ⋄ a   ⍝ "HELL*" "W*RLD"

⍝ — Replacing one nested item may change its length
a←(1 2⋄ 3 4) ⋄ (1↑a)←⊂8 9 10 ⋄ a   ⍝ (8 9 10⋄ 3 4)

⍝ — Repeated selection updates the same source element more than once
a←3⍴0ₓ ⋄ (5⍴a)+←1ₓ ⋄ a   ⍝ 2ₓ 2ₓ 1ₓ

⍝ — Reverse composes with an index selection
a←1 2 3 ⋄ (⌽[1 3]⌷a)←8 9 ⋄ a   ⍝ 9 2 8

⍝ — An empty selection leaves the source unchanged
a←1 2 3 ⋄ (0↑a)←9 ⋄ a   ⍝ 1 2 3

⍝ — Empty each-selection preserves the nested prototype shape
a←0⍴⊂2 3⍴⍳6 ⋄ (⌽⍤[2]¨a)←9 ⋄ ⍴↑a   ⍝ 2ₓ 3ₓ

⍝ — Overtake's fill positions do not create new source elements
a←1 2 ⋄ (3↑a)←4 ⋄ a   ⍝ 4 4

⍝ —
a←(1 2⋄ 3 4) ⋄ (↑a)←7 8 9 ⋄ 1⊃a   ⍝ 7 8 9

⍝ — First selection replaces an atom with a vector
a←1 ⋄ (↑a)←3 4 ⋄ a   ⍝ 3 4

⍝ — Disclose-each selects the first element of each nested vector
a←(1 2⋄ 3 4) ⋄ (↑¨a)←(5 6⋄ 7 8) ⋄ a   ⍝ ((5 6)2⋄ (7 8)4)

⍝ — A bound take function remains assignment-selective
a←1 2 3 ⋄ ((1∘↑)a)←9 ⋄ a   ⍝ 9 2 3

⍝ — A bound pick function can replace an item with a nested vector
a←1 2 ⋄ ((1∘⊃)a)←3 4 ⋄ a   ⍝ (3 4)2

⍝ — Binding an empty pick path retains whole-array replacement
a←1 2 ⋄ ((⍬∘⊃)a)←3 4 5 ⋄ a   ⍝ 3 4 5

⍝ — Bound pick under each updates the selected nested elements
a←(1 2⋄ 3 4) ⋄ ((1∘⊃)¨a)←(5 6⋄ 7 8) ⋄ a   ⍝ ((5 6)2⋄ (7 8)4)

⍝ — An empty array has no first item to replace
a←⍬ ⋄ (↑a)←3 4
⍝ error: INDEX ERROR

⍝ — The selection expression executes only once
a←1 2 ⋄ ((⎕←1)/a)←3 ⋄ a
3 3
⍝ ⎕: 1

⍝⍝ Modified, selective and strand assignment

⍝ — Modified assignment updates an outer lexical binding
a←10 ⋄ f←{a+←⍵ ⋄ a} ⋄ z←f 3 ⋄ z,a   ⍝ 13 13

⍝ — A local array can shadow an outer operator
o←¨ ⋄ {o←3 ⋄ o}0   ⍝ 3

⍝ — Strand assignment creates local names even when an outer name is an operator
o←¨ ⋄ {a o←3 4 ⋄ a o}0   ⍝ 3 4

⍝ — A local function can shadow an outer operator
o←¨ ⋄ {o←{⍵} ⋄ o 3}0   ⍝ 3

⍝ — Modified assignment finds the nearest lexical binding
a←10 ⋄ f←{a←2 ⋄ g←{a+←⍵ ⋄ a} ⋄ z←g ⍵ ⋄ z,a} ⋄ z←f 3 ⋄ z,a   ⍝ 5 5 10

⍝ — The caller's local name does not redirect a callee's lexical assignment
a←10 ⋄ g←{a+←⍵ ⋄ a} ⋄ f←{a←2 ⋄ z←g ⍵ ⋄ z,a} ⋄ z←f 3 ⋄ z,a   ⍝ 13 2 13

⍝ — Updating an outer array leaves a previously assigned copy unchanged
a←1 2 ⋄ b←a ⋄ f←{a.(1)←⍵ ⋄ a} ⋄ z←f 3 ⋄ z,a,b   ⍝ 3 2 3 2 1 2

⍝ — Selective modified assignment can update an outer array
a←1 2 ⋄ f←{(⌽a)+←⍵ ⋄ a} ⋄ z←f 3 ⋄ z,a   ⍝ 4 5 4 5

⍝ — A local error guard does not roll back writes to an outer binding
a←10 ⋄ f←{0::a ⋄ a+←⍵ ⋄ 1÷0} ⋄ z←f 3 ⋄ z,a   ⍝ 13 13

⍝ — A guard restores local bindings to their installation state
{a←2 ⋄ 0::a ⋄ a+←3 ⋄ 1÷0}0   ⍝ 2

⍝ —
1+a←3   ⍝ 4

⍝ — Applying a function to an assignment result makes it non-shy
{1+a←3 ⋄ 9}0   ⍝ 4

⍝ — Modified assignment yields its right argument, not the updated binding
{a←1 ⋄ +a+←3 ⋄ 9}0   ⍝ 3

⍝ —
a←1+b←2 ⋄ a b   ⍝ 3 2

⍝ — Modified assignment to a dot path yields the supplied increment
a←1 2 3 ⋄ 2×a.(2)+←10   ⍝ 20

⍝ — An enclosing expression sees the replacement in selection order
a←1 2 3 ⋄ 1+(⌽a)←4 5 6   ⍝ 5 6 7

⍝ —
(a b)c←(3 4)5 ⋄ a b c   ⍝ 3 4 5

⍝ — A named reverse function remains assignment-selective
a←1 2 3 ⋄ rev←⌽ ⋄ (rev a)←4 5 6 ⋄ a   ⍝ 6 5 4

⍝ — At top level a named function before the arrow performs modified assignment
a←1 ⋄ f←+ ⋄ 2×a f←3   ⍝ 6

⍝ — In a dfn the same spelling assigns the strand of local names
{a←1 ⋄ f←+ ⋄ a f←3 ⋄ a f}0   ⍝ 3 3

⍝ — Deriving the modifier removes the ambiguity with local strand assignment
{a←1 ⋄ f←+ ⋄ a f∘⊢←3 ⋄ a}0   ⍝ 4

⍝ — A named rank operand binds before modified assignment
a←1 2 ⋄ r←0 ⋄ 1+a +⍤r←3 4   ⍝ 4 5

⍝ —
a←1 ⋄ b←2 ⋄ a b+←3 4 ⋄ a b   ⍝ 4 6

⍝ —
a←1 ⋄ b←2 ⋄ (a b)+←3 ⋄ a b   ⍝ 4 5

⍝ — Repeated names receive successive modified assignments
a←1 ⋄ a a+←3 4 ⋄ a   ⍝ 8

⍝ —
a←1 2 3 ⋄ a.(2)←9 ⋄ a   ⍝ 1 9 3

⍝ — Repeated indices accumulate rather than overwriting from the original value
a←1 2 3 ⋄ r←a.[2 2]+←10 20 ⋄ a   ⍝ 1 32 3

⍝ — The result of modified assignment is the original right argument
a←1 2 3 ⋄ r←a.[2 2]+←10 20 ⋄ r   ⍝ 10 20

⍝ — Repeated row and column indices multiply the number of updates
a←3 5⍴0 ⋄ a.(1 1 3 ⋄ 1 3 3 5)+←1 ⋄ ,a   ⍝ 2 0 4 0 2 0 0 0 0 0 1 0 2 0 1

⍝ — Assignment to a dot path preserves value semantics for an earlier copy
a←1 2 3 ⋄ b←a ⋄ a.[1 3]←8 9 ⋄ b   ⍝ 1 2 3

⍝ —
a←1ₓ 2ₓ ⋄ a×←2ₓ ⋄ a   ⍝ 2ₓ 4ₓ

⍝ — Modified integer assignment promotes on overflow
a←1ₓ ⋄ a+←9223372036854775807ₓ ⋄ a   ⍝ 9223372036854775808ₓ

⍝ — Right-to-left evaluation reads the strand's a before the function updates it
a←1 ⋄ f←{a+←1 ⋄ a} ⋄ (f 0) a   ⍝ 2 1

⍝ —
(a (b c))←1 (2 3) ⋄ a b c   ⍝ 1 2 3

⍝ —
a b←1 ⋄ a b   ⍝ 1 1

⍝ — Successive dot selections map back to the original array
a←1 2 3 ⋄ a.[3 1].(2)←9 ⋄ a   ⍝ 9 2 3

⍝ — A reach index after the dot updates an element inside the second item
a←(1 2⋄ 3 4) ⋄ a.[⊂(,2⋄ ,1)]←9 ⋄ ↑2⊃a   ⍝ 9

⍝ — Strand modification calls the operand from left to right
a←1 ⋄ b←2 ⋄ a b{⎕←⍺ ⋄ ⍺+⍵}←3 4 ⋄ a b
4 6
⍝ ⎕: 1\n2

⍝ — The right-hand assignment runs before the parenthesized read
a←0 ⋄ (⎕←a)+a←⎕←3
6
⍝ ⎕: 3\n3

⍝ — A modifying dfn may read the array it is updating
a←1 2 ⋄ f←{⎕←a ⋄ ⍺+⍵} ⋄ a.[1 2]f←10 20 ⋄ a   ⍝ 11 22

⍝⍝ General axis forms

⍝ — Ravel merges adjacent selected axes. Dyalog 20.0.53963.0, IO=1, CT=1e-14, ML=1.
⍴,⍤[2 3]2 3 4⍴⍳24   ⍝ 2ₓ 12ₓ

⍝ — Fractional-axis ravel inserts a unit axis between existing axes
⍴,⍤[1.5]2 3⍴⍳6   ⍝ 2ₓ 1ₓ 3ₓ

⍝ — Empty-axis ravel appends a unit axis
⍴,⍤[⍬]2 3⍴⍳6   ⍝ 2ₓ 3ₓ 1ₓ

⍝ — Mix places item axes before the outer vector axis
⊃⍤[0.5](1 2⋄ 3 4)   ⍝ [1 3 ⋄ 2 4]

⍝ — Mix places the matrix-cell axes at positions one and three
⊃⍤[1 3](2 3⍴⍳6⋄ 2 3⍴6+⍳6)   ⍝ 2 2 3⍴1 2 3 7 8 9 4 5 6 10 11 12

⍝ — A single Mix axis outside the result rank is a domain error, as axis vectors are
⊃⍤[4](2 3⍴⍳6⋄ 2 3⍴6+⍳6)
⍝ error: DOMAIN ERROR

⍝ — Laminate inserts a leading axis
1 2,⍤[0.5]3 4   ⍝ [1 2 ⋄ 3 4]

⍝ — Laminate extends a scalar along the vector's existing axis
1,⍤[1.5]2 3   ⍝ [1 2 ⋄ 1 3]

⍝ — Scalar-function axis one aligns the vector with rows
1 2+⍤[1]2 3⍴⍳6   ⍝ [2 3 4 ⋄ 6 7 8]

⍝ — Take counts follow the specified axis order
2 1↑⍤[2 1]3 4⍴⍳12   ⍝ [1 2 ⋄]

⍝ — Drop counts follow the specified axis order
1 1↓⍤[2 1]3 4⍴⍳12   ⍝ [6 7 8 ⋄ 10 11 12]

⍝ — Squad's index vectors correspond to the listed axes
(2 1⋄ 1 2)⌷⍤[2 1]2 3⍴⍳6   ⍝ [2 1 ⋄ 5 4]

⍝ — Enclosing all axes in reverse order transposes the enclosed cell
↑⊂⍤[2 1]2 3⍴⍳6   ⍝ [1 4 ⋄ 2 5 ⋄ 3 6]

⍝ — Axis enclosure leaves the unselected frame, including its zero dimension
⍴⊂⍤[3]2 0 4⍴0ₓ   ⍝ 2ₓ 0ₓ

⍝ — Empty axis-enclosure retains the length-four cell prototype
⍴↑↑⊂⍤[3]2 0 4⍴0ₓ   ⍝ ,4ₓ

⍝⍝ Axes through rank

⍝ — An enclosed rank operand selects axes. Reduce keeps its own axis meaning
+/⍤[1]3 4⍴⍳12   ⍝ 15 18 21 24

⍝ — A plain number is still a cell rank
+/⍤1⊢3 4⍴⍳12   ⍝ 10 26 42

⍝ — Any enclosed value selects axes, including one held in a name
ax←⊂1 ⋄ +/⍤ax⊢3 4⍴⍳12   ⍝ 15 18 21 24

⍝ — A dfn applies to the cells made of the selected axes
{+/⍵}⍤[1]3 4⍴⍳12   ⍝ 15 18 21 24

⍝ — A primitive without its own axis meaning follows the general rule
≢⍤[1]3 4⍴⍳12   ⍝ 3ₓ 3ₓ 3ₓ 3ₓ

⍝ — Result cells of the selected rank return to the selected axes
{⌽⍵}⍤[1]3 4⍴⍳12   ⍝ ⊖3 4⍴⍳12

⍝ — Other result cells start at the first selected axis
{,⍵}⍤[1 2]2 3 4⍴⍳24   ⍝ ,⍤[1 2]2 3 4⍴⍳24

⍝ — An argument without the selected axes is one whole cell
1 0 1 0{⍺/⍵}⍤[2]3 4⍴⍳12   ⍝ 1 0 1 0/3 4⍴⍳12

⍝ —
1{⍺⌽⍵}⍤[1]3 4⍴⍳12   ⍝ 1⊖3 4⍴⍳12

⍝ — Both arguments supply cells when both have the selected axes
(2 3⍴⍳6){⍺,⍵}⍤[1]2 3⍴⍳6   ⍝ (2 3⍴⍳6)⍪2 3⍴⍳6

⍝ — Names select axes
m←("row":3 ⋄ "col":4)⍴⍳12 ⋄ ({+/⍵}⍤["row"]m)≡{+/⍵}⍤[1]m   ⍝ 1ₓ

⍝ — Exception: multi-axis reduce flattens each cell
+/⍤[2 3]2 3 4⍴⍳24   ⍝ 78 222

⍝ — Exception: a count array rotates each cell by its own count
1 2 3 4⌽⍤[1]3 4⍴⍳12   ⍝ [5 10 3 8 ⋄ 9 2 7 12 ⋄ 1 6 11 4]

⍝ — Exception: a fractional position laminates
1 2,⍤[0.5]3 4   ⍝ [1 2 ⋄ 3 4]

⍝⍝ Brackets enclose

⍝ — Brackets without ⋄ enclose
[1 2]   ⍝ ⊂1 2

⍝ —
[1]≡⊂1   ⍝ 1ₓ

⍝ — Brackets after a function are its enclosed argument
≢[1 2]   ⍝ 1ₓ

⍝ — Brackets no longer index
v←1 2 3 ⋄ v[2]
⍝ error: SYNTAX ERROR

⍝ — Bracketed values never strand
[1 2] [3 4]
⍝ error: SYNTAX ERROR

⍝ — The explicit strand joins bracketed values. Each is one enclosed item
[1 2]˘[3 4]   ⍝ (⊂1 2)(⊂3 4)

⍝ — Semicolons are not syntax
1 2 3[1;2]
⍝ error: SYNTAX ERROR

⍝ — Assignment through ⌷ adds a missing key
T←"aa" "bb":1 2 ⋄ ("cc"⌷T)←3 ⋄ T   ⍝ "aa" "bb" "cc":1 2 3

⍝ — Adding a key to an axis with no keys gives the new position a key and leaves the others without
v←10 20 30 ⋄ ("x"⌷v)←9 ⋄ v   ⍝ (10 ⋄ 20 ⋄ 30 ⋄ "x":9)

⍝ — Dot assignment adds a named position to an unkeyed vector
v←10 20 30 ⋄ v.x←9 ⋄ v   ⍝ (10 ⋄ 20 ⋄ 30 ⋄ "x":9)

⍝ — A matrix with no row keys gains a named row
M←[1 2 3 ⋄ 4 5 6] ⋄ ("r"⌷M)←7 8 9 ⋄ M   ⍝ 1 2 "r":[1 2 3 ⋄ 4 5 6 ⋄ 7 8 9]

⍝ — A key strand or a bracketed key array also adds missing keys
M←("aa" "bb" ⋄ "xx" "yy"):[1 2 ⋄ 3 4] ⋄ ("cc" "zz"⌷M)←9 ⋄ "cc" "zz"⌷M   ⍝ 9

⍝ —
T←"aa":1 ⋄ (["cc" "dd"]⌷T)←3 4 ⋄ T   ⍝ "aa" "cc" "dd":1 3 4

⍝ — A function cannot take a bracketed value alone when a value follows it
≢[1 2] 3 4
⍝ error: SYNTAX ERROR

⍝ — Dot access after a group that contains dot access
inner←"bb" "cc":1 2 ⋄ x←"aa" "zz":inner 0 ⋄ (x.aa).bb   ⍝ 1

⍝ —
inner←"bb" "cc":1 2 ⋄ x←"aa" "zz":inner 0 ⋄ (x.aa).bb←5 ⋄ x.aa   ⍝ "bb" "cc":5 2

⍝⍝ Dot indexing

⍝ — A group after the dot is the left argument of ⌷
m←3 4⍴⍳12 ⋄ m.(2 3)   ⍝ 7

⍝ — Brackets after the dot enclose the index
m←3 4⍴⍳12 ⋄ m.[3 1]   ⍝ [9 10 11 12 ⋄ 1 2 3 4]

⍝ — ⋄ separates index items that are expressions
m←3 4⍴⍳12 ⋄ k←⍳3 ⋄ m.(k~2 ⋄ 4)   ⍝ 4 12

⍝ — Dot indexing chains from left to right
A←2 3 4⍴⍳24 ⋄ A.(2 ⋄ ∞ ⋄ 1).(¯1)   ⍝ 21

⍝ — Dot indexing binds tightly inside a larger expression
v←10 20 30 ⋄ 9,v.[2],3   ⍝ 9 20 3

⍝ — Dot indexing follows a literal
"abc".[3 1]   ⍝ "ca"

⍝ — Dot indexing follows a system name
•a.[3 1]   ⍝ "CA"

⍝ — A decimal point needs a digit after it, so a dot after a number is not part of it
2.
⍝ error: SYNTAX ERROR

⍝ — The sort idiom
v←3 1 2 ⋄ v.[⍋v]   ⍝ 1 2 3

⍝ — Choose indexing through the dot
m←2 2⍴⍳4 ⋄ m.[(1 2)(2 1)]   ⍝ 2 3

⍝ — Assignment to a dot path goes through ⌷
v←10 20 30 ⋄ v.[3 1]←7 8 ⋄ v   ⍝ 8 20 7

⍝ —
m←2 2⍴⍳4 ⋄ m.(∞ 2)←0 ⋄ m   ⍝ [1 0 ⋄ 3 0]

⍝ — Dot assignment adds missing keys
T←"aa":1 ⋄ T.["bb" "cc"]←2 3 ⋄ T   ⍝ "aa" "bb" "cc":1 2 3

⍝ — Between functions the dot is inner product
1 2 3 +.(×) 4 5 6   ⍝ 32

⍝⍝ Format and execute

⍝ — Decimal formatting rounds halfway cases away from zero
2⍕3.125 ¯3.125 2.675 ¯2.675   ⍝ " 3.13 ¯3.13 2.68 ¯2.68"

⍝ — Integer formatting also rounds halves away from zero
0⍕2.5 ¯2.5 1.5 ¯1.5   ⍝ " 3 ¯3 2 ¯2"

⍝ — Rounded zero loses its minus sign but retains alignment space
2⍕¯0.001 0.001   ⍝ "  0.00 0.00"

⍝ — A negative precision selects significant digits in exponential notation
¯2⍕3.25 ¯3.25 325 0.0325   ⍝ " 3.3E0 ¯3.3E0 3.3E2 3.3E¯2"

⍝ — Matrix formatting aligns exponential fields by column
¯2⍕[3.125 0.002 ⋄ 1000 20]   ⍝ ⊃" 3.1E0 2.0E¯3" " 1.0E3 2.0E1 "

⍝ —
⍕¯1E¯100j¯2E¯99   ⍝ "¯1E¯100j¯2E¯99"

⍝ — Default formatting switches to exponents beyond these magnitude thresholds
⍕1E¯6 1E¯7 1E16 1E17   ⍝ "0.000001 1E¯7 10000000000000000 1E17"

⍝ — Default format and execute round-trip subnormal, large and complex values
⍎⍕5E¯324 ¯1.2345678901234567E200 1E¯100j2E100
5E¯324 ¯1.2345678901234567E200 1E¯100j2E100

⍝ — Formatting a scalar produces a character vector
⍴⍕1   ⍝ ,1ₓ

⍝ — A zero-row matrix retains column widths and separators
⍴⍕0 3⍴0   ⍝ 0ₓ 5ₓ

⍝ — A matrix with no columns formats to zero-width rows
⍴⍕3 0⍴0   ⍝ 3ₓ 0ₓ

⍝ — Fixed-width formatting expands only the trailing dimension
⍴5 2⍕2 3 4⍴⍳24   ⍝ 2ₓ 3ₓ 20ₓ

⍝ — Per-column formats replace overflowing fields with stars
,3 0 6 2⍕[10.1 15 ⋄ 1001 22.357 ⋄ 101 1110.1]
" 10 15.00*** 22.36101******"

⍝ — Default matrix formatting aligns decimal points within each column
,⍕[1 12.3 ⋄ 123 4]   ⍝ "  1 12.3123  4  "

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
⍕1ₓ 1r3   ⍝ "1ₓ 1r3"

⍝ —
⍕⍬   ⍝ ""

⍝ —
4 1⍕1.1 2 ¯4 2.547   ⍝ " 1.1 2.0¯4.0 2.5"

⍝ — Fixed-width fields retain a trailing space when the exponent is short
7 ¯3⍕5 15 155 1555   ⍝ "5.00E0 1.50E1 1.55E2 1.56E3 "

⍝ —
0 2⍕1 2   ⍝ " 1.00 2.00"

⍝ —
0 2⍕1r3 2r3   ⍝ " 0.33 0.67"

⍝ — Exact big integers format without conversion through float
0 0⍕9223372036854775808ₓ   ⍝ " 9223372036854775808"

⍝ — Requested digits beyond float precision are marked with underscores
0 20⍕÷3   ⍝ " 0.3333333333333333____"

⍝ — Execute sees the active local binding without changing the global
a←4 ⋄ f←{a←10 ⋄ ⍎"a+⍵"} ⋄ b←f 3 ⋄ b a   ⍝ 13 4

⍝ — Dyadic execute selects values without executing character data
T←("a":"1+2"),("b":4) ⋄ (T⍎"a" ⋄ T⍎"b" ⋄ T⍎⊂"b" "a")
("1+2" ⋄ 4 ⋄ (4 ⋄ "1+2"))

⍝ — A string selector requires keys on that axis
""⍎"1+2"
⍝ error: INDEX ERROR

⍝ — Execute can return a primitive function
g←⍎'+' ⋄ 2 g 3   ⍝ 5

⍝ — A final assignment in execute can also return a function
g←⍎"h←+" ⋄ 2 g 3   ⍝ 5

⍝ — A dfn can return the function produced by execute
g←{⍎'+'}0 ⋄ 2 g 3   ⍝ 5

⍝ — Execute preserves the hybrid category
r←⍎'/' ⋄ +r 1 2 3   ⍝ 6

⍝ — Execute can return a defined operator
op←⍎"{⍶ ⍵}" ⋄ -op 3   ⍝ ¯3

⍝ — Empty execute produces no value
⍎""   ⍝ {}0

⍝ — A missing execute result cannot be assigned
a←⍎""
⍝ error: VALUE ERROR

⍝⍝ Polynomial representations and derivatives

⍝ — Polynomial coefficients are constant-first
1ₓ 2ₓ 3ₓ⊛0ₓ 1ₓ 2ₓ   ⍝ 1ₓ 6ₓ 17ₓ

⍝ — Evaluating at an array of points preserves its shape
1ₓ 2ₓ 3ₓ⊛[0ₓ 1ₓ ⋄ 2ₓ 3ₓ]   ⍝ [1ₓ 6ₓ ⋄ 17ₓ 34ₓ]

⍝ — Evaluate the factored form 2(x-1)(x-3)
(2ₓ (1ₓ 3ₓ))⊛0ₓ 1ₓ 2ₓ 3ₓ   ⍝ 6ₓ 0ₓ ¯2ₓ 0ₓ

⍝ — Convert multiplier and roots to constant-first coefficients
⊛2ₓ (1ₓ 3ₓ)   ⍝ 6ₓ ¯8ₓ 2ₓ

⍝ — Enclosed roots imply a leading coefficient of one
⊛⊂1ₓ 3ₓ   ⍝ 3ₓ ¯4ₓ 1ₓ

⍝ — Convert an exponent table for x⁵-1, filling missing degrees with zero
⊛⊂[1ₓ 5ₓ ⋄ ¯1ₓ 0ₓ]   ⍝ ¯1ₓ 0ₓ 0ₓ 0ₓ 0ₓ 1ₓ

⍝ — Fractional exponents evaluate numerically even with exact input
[[2ₓ 1r2 ⋄ 3ₓ 1r4]]⊛16ₓ   ⍝ 14

⍝ — Evaluate a two-variable exponent table at an enclosed coordinate vector
[[¯1 2 1 ⋄ 1 1 1 ⋄ 2 1 2 ⋄ 3 0 2]]⊛⊂2.5 ¯1   ⍝ 11.75

⍝ — Empty exact evaluation points retain an exact prototype
1ₓ 2ₓ⊛0⍴0ₓ   ⍝ 0⍴0ₓ

⍝ — Polynomial rows pair with scalar evaluation points by frame
[1ₓ 2ₓ 3ₓ ⋄ 4ₓ 5ₓ 6ₓ]⊛1ₓ 2ₓ   ⍝ 6ₓ 38ₓ

⍝ —
5ₓ⊛2ₓ   ⍝ 5ₓ

⍝ —
0ₓ 0ₓ⊛2ₓ   ⍝ 0ₓ

⍝ — A multiplier with no roots is a constant polynomial
⊛2ₓ (0⍴0ₓ)   ⍝ ,2ₓ

⍝ — Differentiate the bound polynomial evaluator
f←1ₓ 2ₓ 3ₓ∘⊛ ⋄ f∂2ₓ   ⍝ 14ₓ

⍝ — Repeated differentiation gives the second derivative
f←1ₓ 2ₓ 3ₓ∘⊛ ⋄ f∂∂2ₓ   ⍝ 6ₓ

⍝ — Differentiating beyond the polynomial's degree gives exact zero
f←1ₓ 2ₓ 3ₓ∘⊛ ⋄ f∂∂∂2ₓ   ⍝ 0ₓ

⍝ — Dyadic derivative weights each output derivative by its cotangent
f←1ₓ 2ₓ 3ₓ∘⊛ ⋄ 10ₓ 20ₓ(f∂)1ₓ 2ₓ   ⍝ 80ₓ 280ₓ

⍝ — Differentiate directly from the factored representation
f←(2ₓ (1ₓ 3ₓ))∘⊛ ⋄ f∂2ₓ   ⍝ 0ₓ

⍝ — A multivariate gradient retains the coordinate enclosure
f←[[1ₓ 2ₓ 0ₓ ⋄ 1ₓ 0ₓ 2ₓ]]∘⊛ ⋄ f∂⊂3ₓ 4ₓ   ⍝ ⊂6ₓ 8ₓ

⍝ — A scalar cotangent scales the multivariate gradient
f←[[1ₓ 2ₓ 0ₓ ⋄ 1ₓ 0ₓ 2ₓ]]∘⊛ ⋄ 2ₓ(f∂)⊂3ₓ 4ₓ   ⍝ ⊂12ₓ 16ₓ

⍝ — A shared scalar coordinate sums the partial derivatives
f←[[1ₓ 2ₓ 0ₓ ⋄ 1ₓ 0ₓ 2ₓ]]∘⊛ ⋄ f∂3ₓ   ⍝ 12ₓ

⍝ — Multiple polynomial outputs contribute to one scalar-input VJP
f←[1ₓ 2ₓ ⋄ 3ₓ 4ₓ]∘⊛ ⋄ 10ₓ 20ₓ(f∂)3ₓ   ⍝ 100ₓ

⍝ — Negative exponents cannot be converted to a coefficient vector
⊛⊂[1 ¯1 ⋄]
⍝ error: DOMAIN ERROR

⍝ — The cotangent must match the scalar output's structure
[1 2](1 2∘⊛∂)3
⍝ error: DOMAIN ERROR

⍝ —
⊛1 ∞
⍝ error: DOMAIN ERROR

⍝ — Monadic derivative requires scalar output, not a vector of evaluations
1 2∘⊛∂1 2
⍝ error: RANK ERROR

⍝ — VJP requires one cotangent for each output
1(1 2∘⊛∂)1 2
⍝ error: LENGTH ERROR

⍝ — The coordinate count must match the exponent table's variables
[[1 2 3 ⋄]]⊛⊂1 2 3
⍝ error: LENGTH ERROR

⍝⍝ Prime and factor families

⍝ — Prime indexing is one-based and returns exact integers
ℙ⍳8   ⍝ 2ₓ 3ₓ 5ₓ 7ₓ 11ₓ 13ₓ 17ₓ 19ₓ

⍝ — Prime lookup preserves shape and accepts repeated, unordered indices
ℙ[10000 1 ⋄ 10000 2]   ⍝ [104729ₓ 2ₓ ⋄ 104729ₓ 3ₓ]

⍝ —
n←5 ⋄ ℙn   ⍝ 11ₓ

⍝ — Previous prime is strictly below the argument
¯4ℙ3 4 5 6   ⍝ 2ₓ 3ₓ 3ₓ 5ₓ

⍝ — Count primes strictly below each argument
¯1ℙ1 2 3 4 5 6   ⍝ 0ₓ 0ₓ 1ₓ 2ₓ 2ₓ 3ₓ

⍝ — Non-prime includes negative integers, zero and one
0ℙ¯1 0 1 2 3 4   ⍝ 1ₓ 1ₓ 1ₓ 0ₓ 0ₓ 1ₓ

⍝ — Test primality rather than indexing the prime sequence
1ℙ¯1 0 1 2 3 4   ⍝ 0ₓ 0ₓ 0ₓ 1ₓ 1ₓ 0ₓ

⍝ — Distinct factors occupy the first row, their exponents the second
2ℙ700   ⍝ [2ₓ 5ₓ 7ₓ ⋄ 2ₓ 2ₓ 1ₓ]

⍝ — Factor-list mode includes repeated prime factors
3ℙ700   ⍝ 2ₓ 2ₓ 5ₓ 5ₓ 7ₓ

⍝ — Next prime is strictly above the argument
4ℙ1 2 3 4 5   ⍝ 2ₓ 3ₓ 5ₓ 5ₓ 7ₓ

⍝ — Euler's totient counts positive integers up to n coprime to n
5ℙ1 2 3 4 5 6 10   ⍝ 1ₓ 1ₓ 2ₓ 2ₓ 4ₓ 2ₓ 4ₓ

⍝ —
⨸700   ⍝ 2ₓ 2ₓ 5ₓ 5ₓ 7ₓ

⍝ — One has an empty exact factorization
⨸1   ⍝ 0⍴0ₓ

⍝ — Return exponents of only the first two primes
2⨸700   ⍝ 2ₓ 0ₓ

⍝ — Requested primes beyond the largest factor receive zero exponents
10⨸700   ⍝ 2ₓ 0ₓ 2ₓ 1ₓ 0ₓ 0ₓ 0ₓ 0ₓ 0ₓ 0ₓ

⍝ — Infinite count includes every prime through the largest factor
∞⨸700   ⍝ 2ₓ 0ₓ 2ₓ 1ₓ

⍝ — Negative count selects the last distinct factors and their exponents
¯2⨸700   ⍝ [5ₓ 7ₓ ⋄ 2ₓ 1ₓ]

⍝ — Negative infinity requests the complete factor/exponent table
¯∞⨸700   ⍝ [2ₓ 5ₓ 7ₓ ⋄ 2ₓ 2ₓ 1ₓ]

⍝ —
0⨸700   ⍝ 0⍴0ₓ

⍝ — Factoring one retains the table's two-row shape
¯∞⨸1   ⍝ 2 0⍴0ₓ

⍝ — Inverse prime lookup recovers one-based indices
ℙ⍣¯1⊢ℙ⍳10   ⍝ ⍳10ₓ

⍝ — Inverse factorization multiplies the factors
⨸⍣¯1⊢⨸700   ⍝ 700ₓ

⍝ — Primality accepts exact integers at and beyond the unsigned 64-bit boundary
1ℙ18446744073709551557ₓ 18446744073709551615ₓ 170141183460469231731687303715884105727ₓ
1ₓ 0ₓ 1ₓ

⍝ — Strong pseudoprimes must not pass as primes
1ℙ341550071728321ₓ 3825123056546413051ₓ   ⍝ 0ₓ 0ₓ

⍝ — Factor a semiprime whose two factors are both large
⨸1000000016000000063ₓ   ⍝ 1000000007ₓ 1000000009ₓ

⍝ — Empty prime lookup preserves shape with an exact prototype
ℙ0 2⍴0   ⍝ 0 2⍴0ₓ

⍝ — Factor lists of unequal lengths assemble with zero fill
⨸2 12   ⍝ 2 3⍴2ₓ 0ₓ 0ₓ 2ₓ 2ₓ 3ₓ

⍝ —
ℙ0
⍝ error: DOMAIN ERROR

⍝ — Prime indices require exact integrality, without comparison tolerance
ℙ1.000000000000001
⍝ error: DOMAIN ERROR

⍝ —
1ℙ2.5
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
6ℙ7
⍝ error: DOMAIN ERROR

⍝ — There is no prime strictly below two
¯4ℙ2
⍝ error: DOMAIN ERROR

⍝⍝ Windows

⍝ — Full vector windows overlap without padding
3↕⍳5   ⍝ [1 2 3 ⋄ 2 3 4 ⋄ 3 4 5]

⍝ — Window-position axes precede the two-dimensional window axes
2 2↕2 3⍴⍳6   ⍝ 1 2 2 2⍴1 2 4 5 2 3 5 6

⍝ — One window length slides along the leading axis, retaining whole rows
2↕3 2⍴⍳6   ⍝ 2 2 2⍴1 2 3 4 3 4 5 6

⍝ — An oversized window produces no windows but retains window shape and character fill
4↕"ab"   ⍝ 0 4⍴' '

⍝ — Zero-length windows occur at every boundary, including both ends
0↕"ab"   ⍝ 3 0⍴' '

⍝ — No window axes leaves the argument unchanged
⍬↕2 3⍴⍳6   ⍝ 2 3⍴⍳6

⍝ — Zero-length and nonempty window axes combine without losing the frame
0 2↕2 3⍴⍳6   ⍝ 3 2 0 2⍴0

⍝ — Negative sizes give stencil windows, including even sizes
(¯4↕⍳5)≡{⍵}⌺4⍳5   ⍝ 1ₓ

⍝ — Padded matrix windows with movements match stencil windows
([¯3 ¯2 ⋄ 2 1]↕4 5⍴⍳20)≡{⍵}⌺([3 2 ⋄ 2 1])⊢4 5⍴⍳20   ⍝ 1ₓ

⍝ —
0.5↕⍳3
⍝ error: DOMAIN ERROR

⍝ —
2↕3
⍝ error: RANK ERROR

⍝ —
1 2↕⍳3
⍝ error: RANK ERROR

⍝ —
[2 ⋄]↕⍳3
⍝ error: LENGTH ERROR

⍝ —
(1 1 1⍴2)↕⍳3
⍝ error: RANK ERROR

⍝ —
[2 ⋄ 0]↕⍳3
⍝ error: DOMAIN ERROR

⍝ —
¯4↕⍳2
⍝ error: DOMAIN ERROR

⍝⍝ Paired inverse under and trajectories

⍝ — Negative power uses an explicitly declared inverse
f←{⍵+1}⇄{⍵-1} ⋄ f⍣¯2⊢5   ⍝ 3

⍝ — A dyadic declared inverse recovers the right argument with the left fixed
f←{⍺+⍵}⇄{⍵-⍺} ⋄ 3(f⍣¯1)8   ⍝ 5

⍝ — Binding the left argument retains the declared inverse
f←{⍺+⍵}⇄{⍵-⍺} ⋄ (3∘f)⍣¯1⊢8   ⍝ 5

⍝ — Inverting an explicitly paired inverse recovers the forward function
f←{⍵+1}⇄{⍵-1} ⋄ (f⍣¯1)⍣¯1⊢5   ⍝ 6

⍝ — Dyadic under transforms both arguments, then inverts the result
3 (+⌾(2∘×))4   ⍝ 7

⍝ — Monadic under changes coordinates before and after reversal
(⌽⌾(1∘+))1 2 3   ⍝ 3 2 1

⍝ — Power preserves the count array's shape, including negative and repeated counts
(1∘+)⍣([3 ¯2 ⋄ 0 3])⊢10   ⍝ [13 8 ⋄ 10 13]

⍝ — A singleton count vector adds a singleton result frame
(+⍣(,1))2   ⍝ ,2

⍝ — Different-sized iterates assemble into padded rows
{⍵,1}⍣0 1 2⊢,2   ⍝ 3 3⍴2 0 0 2 1 0 2 1 1

⍝ — Empty counts never invoke the operand and retain count and argument shapes
{1÷0}⍣(0 2⍴0)⊢"ab"   ⍝ 0 2 2⍴' '

⍝ — Iteration history includes the starting value
(1∘+)⍣[3]⊢5   ⍝ 5 6 7 8

⍝ — A negative history count follows the inverse
(1∘+)⍣[¯2]⊢5   ⍝ 5 4 3

⍝ — History pads growing iterates into rows
{⍵,1}⍣[2]⊢,2   ⍝ 3 3⍴2 0 0 2 1 0 2 1 1

⍝ — Predicate history includes the iterate that satisfies the stopping test
1(+⍣[{⍺=3}])0   ⍝ 0 1 2 3

⍝ — Fixed-point history includes both the initial and unchanged next value
⊢⍣[≡]⊢4   ⍝ 4 4

⍝ — Zero-step history retains the initial value without calling the operand
{1÷0}⍣[0]⊢"ab"   ⍝ ["ab" ⋄]

⍝ — An enclosed named predicate uses ordinary function calls and global lookup
limit←4 ⋄ stop←[{⍺≥limit}] ⋄ (1∘+)⍣stop⊢1   ⍝ 1 2 3 4

⍝ — History counts still require integral values
+⍣[0.5]⊢1
⍝ error: DOMAIN ERROR

⍝ — A history is not inverted as if it were ordinary repeated application
((1∘+)⍣[2])⍣¯1⊢1 2 3
⍝ error: DOMAIN ERROR

⍝ — Parenthesizing power leaves the following backslash as ordinary scan
(+⍣1)\1 2 3   ⍝ 1 3 6

⍝ — A declared right-argument inverse does not provide a left-argument inverse
((+⇄-)∘3)⍣¯1⊢8
⍝ error: DOMAIN ERROR

⍝ —
+⍣0 0.5⊢1
⍝ error: DOMAIN ERROR

⍝ —
+⍣[1 2]⊢3
⍝ error: RANK ERROR

⍝ — Repeated power counts reuse iterates rather than calling the operand again
f←{⎕←⍵+1}⇄{⎕←⍵-1} ⋄ f⍣3 ¯2 3 0⊢0
3 ¯2 3 0
⍝ ⎕: 1\n2\n3\n¯1\n¯2

⍝ — Under transforms right then left, calls the operand, then applies the inverse
g←{⎕←⍵ ⋄ 2×⍵}⇄{⎕←⍵ ⋄ ⍵÷2} ⋄ 3(+⌾g)4
7
⍝ ⎕: 4\n3\n14

⍝⍝ Inverse power

⍝ — Inverting x×exp(x) gives the principal Lambert W function
W←×∘*⍨⍣¯1 ⋄ ⌊1E12×W 0 1 (*1) ¯0.1 1j1
0 567143290409 1000000000000 ¯111832559159 656966069230j325450339413

⍝ — Lambert W reaches -1 at its real branch point -1/e
W←×∘*⍨⍣¯1 ⋄ W ¯1÷*1   ⍝ ¯1

⍝ — Lambert W pervades nested exact input and produces approximate values
W←×∘*⍨⍣¯1 ⋄ W (0ₓ 0ₓ⋄ 0⍴0ₓ)   ⍝ (0 0)⍬

⍝ — Check W(x)exp(W(x))=x across tiny and large real inputs
W←×∘*⍨⍣¯1 ⋄ x←1E¯100 ¯1E¯100 1E¯12 ¯1E¯12 0.099 ¯0.099 1E300 ⋄ ∧/1E¯12>|1-(W x)×(*W x)÷x
1ₓ

⍝ — Check the Lambert W inverse identity on complex inputs
W←×∘*⍨⍣¯1 ⋄ x←¯1j0.1 1j¯1 ⋄ ∧/1E¯12>|1-(W x)×(*W x)÷x   ⍝ 1ₓ

⍝ — A real Lambert W argument below -1/e is rejected
(×∘*⍨⍣¯1)¯1
⍝ error: DOMAIN ERROR

⍝ —
(×∘*⍨⍣¯1)'a'
⍝ error: DOMAIN ERROR

⍝ — Invert an outer subtraction with its right vector fixed
( -⌝ ∘4 5)⍣¯1⊢[¯3 ¯4 ⋄ ¯2 ¯3]   ⍝ 1 2

⍝ — Invert an outer subtraction with its left vector fixed
(4 5∘( -⌝ ))⍣¯1⊢[3 2 ⋄ 4 3]   ⍝ 1 2

⍝ — Outer-product inversion preserves exact rational results
( ×⌝ ∘4ₓ 5ₓ)⍣¯1⊢[2ₓ 5r2 ⋄ 1ₓ 5r4]   ⍝ 1r2 1r4

⍝ — A fixed matrix occupies the trailing axes of the outer-product result
( ×⌝ ∘([1 2 ⋄ 3 4]))⍣¯1⊢3 2 2⍴1 2 3 4 2 4 6 8 3 6 9 12   ⍝ 1 2 3

⍝ — Outer inversion with a fixed scalar recovers a vector
(4∘( ×⌝ ))⍣¯1⊢4 8   ⍝ 1 2

⍝ — Removing the fixed vector's axes can leave a scalar
( ×⌝ ∘4 5)⍣¯1⊢4 5   ⍝ ⊂1

⍝ — Outer inversion recovers an empty argument from an empty result frame
( +⌝ ∘4 5)⍣¯1⊢0 2⍴0   ⍝ ⍬

⍝ — With the left vector fixed, the remaining axes describe the recovered matrix
(4 5∘( -⌝ ))⍣¯1⊢2 2 2⍴3 2 1 0 4 3 2 1   ⍝ [1 2 ⋄ 3 4]

⍝ — Outer inversion preserves zero dimensions in the recovered frame
( +⌝ ∘4 5)⍣¯1⊢3 0 2⍴0   ⍝ 3 0⍴0

⍝ — Every outer-product cell must imply the same recovered value
( ×⌝ ∘4 5)⍣¯1⊢[4 5 ⋄ 8 11]
⍝ error: DOMAIN ERROR

⍝ — The result axes must agree with the fixed outer-product argument
( ×⌝ ∘4 5)⍣¯1⊢2 3⍴⍳6
⍝ error: DOMAIN ERROR

⍝ — An empty fixed argument contains no information to invert
( ×⌝ ∘⍬)⍣¯1⊢2 0⍴0
⍝ error: DOMAIN ERROR

⍝ — Multiplication by an all-zero fixed argument is not invertible
( ×⌝ ∘0 0)⍣¯1⊢2 2⍴0
⍝ error: DOMAIN ERROR

⍝ — Inverse iota returns the generating shape as an exact vector
⍳⍣¯1⊢1 2 3   ⍝ ,3ₓ

⍝ — Inverse iota also recognizes multidimensional index arrays
⍳⍣¯1⊢⍳2 3   ⍝ 2ₓ 3ₓ

⍝ — Each positive circle code's inverse agrees with its negative-code counterpart here
{(5○⍨-⍵)=⍵∘○⍣¯1⊢5}⍳12   ⍝ 12⍴1ₓ

⍝ —
(1∘+⍣¯3)10   ⍝ 7

⍝ — Invert a Celsius-to-Fahrenheit composition in reverse function order
((32∘+)∘(×∘1.8)⍣¯1)32 212   ⍝ 0 100

⍝ — Inverse sum scan recovers successive differences
(+\⍣¯1)1 3 6 10   ⍝ 1 2 3 4

⍝ — Inverse scan retains the exact numeric domain
(+\⍣¯1)1ₓ 3ₓ 6ₓ   ⍝ 1ₓ 2ₓ 3ₓ

⍝ — Inverse Boolean xor scan recovers changes between adjacent values
(≠\⍣¯1)1ₓ 1ₓ 0ₓ 0ₓ   ⍝ 1ₓ 0ₓ 1ₓ 0ₓ

⍝ — Inverse base-two decode chooses the required number of digits
(2∘⊥⍣¯1)9   ⍝ 1 0 0 1

⍝ — Exact base and value produce exact inverse-decode digits
(2ₓ∘⊥⍣¯1)9ₓ   ⍝ 1ₓ 0ₓ 0ₓ 1ₓ

⍝ — Invert mixed-radix time decoding into hours, minutes and seconds
(0ₓ 60ₓ 60ₓ∘⊥⍣¯1)3661ₓ   ⍝ 1ₓ 1ₓ 1ₓ

⍝ — Inverse encode decodes the supplied digits
(2ₓ∘⊤⍣¯1)1ₓ 0ₓ 1ₓ   ⍝ 5ₓ

⍝ — Inverse decode permits a fractional least-significant digit
(2∘⊥⍣¯1)2.5   ⍝ 1 0.5

⍝ — Inverse transpose applies the inverse axis permutation
(2 1∘⍉⍣¯1)2 3⍴⍳6   ⍝ [1 4 ⋄ 2 5 ⋄ 3 6]

⍝ — Inverting self-addition halves the argument exactly
(+⍨⍣¯1)3ₓ   ⍝ 3r2

⍝ — Inverse sine selects a branch which maps back to the input
0.5=1∘○(1∘○⍣¯1)0.5   ⍝ 1ₓ

⍝ — Inverse sine within the real domain has no imaginary component
11○(1∘○⍣¯1)0.5   ⍝ 0

⍝ — sqrt(1+x²) avoids overflowing its intermediate square
4○1E300   ⍝ 1E300

⍝ — Signed sqrt(x²-1) avoids intermediate overflow for large negative x
¯4○¯1E300   ⍝ ¯1E300

⍝ — Inversion preserves rank-zero application
((2ₓ∘+)⍤0⍣¯1)3ₓ 4ₓ   ⍝ 1ₓ 2ₓ

⍝ — Inverse where counts repeated indices
(⍸⍣¯1)1 3 3   ⍝ 1ₓ 0ₓ 2ₓ

⍝ — Inverse where infers multidimensional shape from coordinate vectors
(⍸⍣¯1)(1 2⋄ 2 1)   ⍝ [0ₓ 1ₓ ⋄ 1ₓ 0ₓ]

⍝ — Inverse where of no indices returns an empty exact count vector
(⍸⍣¯1)⍬   ⍝ 0⍴0ₓ

⍝ — Zero needs no digits in minimal-width inverse decode
(2∘⊥⍣¯1)0   ⍝ ⍬

⍝ —
3ₓ(+⍣¯1)5ₓ   ⍝ 2ₓ

⍝ — Inversion distributes through each
((2ₓ∘+)¨⍣¯1)3ₓ 4ₓ   ⍝ 1ₓ 2ₓ

⍝ — Inverting self-multiplication selects the positive square root
(×⍨⍣¯1)4   ⍝ 2

⍝ —
(⌽⍣¯1)⍳3ₓ   ⍝ 3ₓ 2ₓ 1ₓ

⍝ —
(⊂⍣¯1)⊂1ₓ 2ₓ   ⍝ 1ₓ 2ₓ

⍝ — Multiplication by zero has no inverse
(0∘×⍣¯1)0
⍝ error: DOMAIN ERROR

⍝ — Inverse where requires ordered positive indices
(⍸⍣¯1)3 1
⍝ error: DOMAIN ERROR

⍝ —
(⍸⍣¯1)0
⍝ error: DOMAIN ERROR

⍝ —
(2∘⊥⍣¯1)¯1
⍝ error: DOMAIN ERROR

⍝ — Inverse decode rejects a value that exceeds the fixed radix range
(2 2∘⊥⍣¯1)5
⍝ error: DOMAIN ERROR

⍝⍝ Inverse combinations

⍝ — Right binding requires inversion with respect to the left argument of composition
((+∘-)∘2)⍣¯1⊢3   ⍝ 5

⍝ — Commute swaps which argument must be recovered
2 ((+∘-)⍨⍣¯1)3   ⍝ 5

⍝ — Invert atop with a fixed right argument
((÷⍤-)∘2)⍣¯1⊢0.5   ⍝ 4

⍝ — Invert over with a fixed right argument
((-⍥-)∘2)⍣¯1⊢¯3   ⍝ 5

⍝ — Before transforms the fixed left argument before solving for the right
3 (-⍛+⍣¯1)7   ⍝ 10

⍝ — Right-bound before also inverts the left-side transformation
((-⍛+)∘3)⍣¯1⊢¯7   ⍝ 10

⍝ — Right-bound each recovers every left element
(-¨∘2)⍣¯1⊢3 4   ⍝ 5 6

⍝ — Right-bound rank recovers vector cells against the fixed scalar
((-⍤1 0)∘2)⍣¯1⊢3 4   ⍝ 5 6

⍝ — Invert left-accumulating subtraction scan
(-\⍣¯1)1 ¯1 ¯4   ⍝ 1 2 3

⍝ — Inverse seeded scan uses the seed to recover the first input
10 (-\⍣¯1)9 7 4   ⍝ 1 2 3

⍝ — Binding the seed retains scan inversion
(10∘(+\))⍣¯1⊢11 13 16   ⍝ 1 2 3

⍝ — Scan inversion calls the composed operand's inverse
((-∘-)\⍣¯1)1 3 6   ⍝ 1 2 3

⍝ — Inverting seeded division scan retains exact fractions
2ₓ (÷\⍣¯1)1ₓ 1r3 1r12   ⍝ 2ₓ 3ₓ 4ₓ

⍝ — Axis-one scan inversion uses a separate seed for each column
⍉10 20 ((+\⍣¯1)⍤0 1)⍉[11 22 ⋄ 14 26]   ⍝ [1 2 ⋄ 3 4]

⍝ —
(+\⍣¯1)⍬   ⍝ ⍬

⍝ — A seeded scalar scan still needs its first step undone
10 (+\⍣¯1)13   ⍝ 3

⍝ — Inverse axis-enclosure restores both shape and axis order
(⊂⍤[2 1]⍣¯1)⊂⍤[2 1]2 3 4⍴⍳24   ⍝ 2 3 4⍴⍳24

⍝ — Inverse split restores the original matrix axis
(↓⍤[1]⍣¯1)↓⍤[1]2 3⍴⍳6   ⍝ 2 3⍴⍳6

⍝ — Inverse fractional-axis mix recovers the original nested vectors
(⊃⍤[0.5]⍣¯1)[1 4 ⋄ 2 5 ⋄ 3 6]   ⍝ (1 2 3⋄ 4 5 6)

⍝ — Scalar inversion with an axis aligns the fixed left vector with rows
10 20 (+⍤[1]⍣¯1)[11 12 13 ⋄ 24 25 26]   ⍝ 2 3⍴⍳6

⍝ — A zero product prefix loses information about subsequent factors
(×\⍣¯1)1 0 0
⍝ error: DOMAIN ERROR

⍝ — A zero product seed loses information from the first step
0 (×\⍣¯1)0 0
⍝ error: DOMAIN ERROR

⍝ — Monadic (-⍛+) is constant zero, so its input cannot be recovered
(-⍛+⍣¯1)0
⍝ error: DOMAIN ERROR

⍝ — Rank checks agreement between row seeds and cells
1 2 3 ((+\⍣¯1)⍤0 1)2 3⍴⍳6
⍝ error: LENGTH ERROR

⍝⍝ Compact integers and promotion

⍝ — Exact tally keeps the mean of exact values rational
avg←+/÷≢ ⋄ avg 1ₓ 2ₓ 4ₓ   ⍝ 7r3

⍝ —
+/⍳3ₓ   ⍝ 6ₓ

⍝ —
×/⍳4ₓ   ⍝ 24ₓ

⍝ —
1ₓ 2ₓ+.×3ₓ 4ₓ   ⍝ 11ₓ

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
2ₓ⊥1ₓ 0ₓ 1ₓ   ⍝ 5ₓ

⍝ —
≢(1ₓ 2ₓ⋄ 1r3 'a')   ⍝ 2ₓ

⍝ —
≢0⍴1ₓ   ⍝ 0ₓ

⍝ — Subtraction below i64 minimum promotes without losing precision
¯9223372036854775808ₓ-1ₓ   ⍝ ¯9223372036854775809ₓ

⍝ — Multiplication above i64 maximum promotes without losing precision
9223372036854775807ₓ×2ₓ   ⍝ 18446744073709551614ₓ

⍝ — An exact sum remains exact at the i64 boundary
+/9223372036854775807ₓ 1ₓ ¯1ₓ   ⍝ 9223372036854775807ₓ

⍝ — Ordinary division remains approximate, unlike explicit exact arithmetic
(1÷3)+(1÷6)   ⍝ 0.5

⍝ —
1ₓ+0.5   ⍝ 1.5

⍝ — Tally counts outer items regardless of nested numeric domains
≢(1ₓ 2ₓ⋄ 3 4)   ⍝ 2ₓ

⍝ —
≢"abc"   ⍝ 3ₓ

⍝ —
≢0⍴⊂1ₓ 2   ⍝ 0ₓ

⍝ —
=/⍬   ⍝ 1ₓ

⍝ —
≠/⍬   ⍝ 0ₓ

⍝ — Empty nested roll retains an exact vector prototype
↑?0⍴⊂1ₓ 2ₓ   ⍝ 0ₓ 0ₓ

⍝⍝ Matrix division

⍝ —
⌹2ₓ   ⍝ 1r2

⍝ — Exact overdetermined solve recovers the line y=1+2ₓ
3ₓ 5ₓ 7ₓ⌹[1ₓ 1ₓ ⋄ 1ₓ 2ₓ ⋄ 1ₓ 3ₓ]   ⍝ 1ₓ 2ₓ

⍝ — A vector pseudoinverse divides by the squared norm
⌹1ₓ 2ₓ   ⍝ 1r5 2r5

⍝ — Exact matrix inversion handles a zero leading pivot
⌹[0ₓ 2ₓ ⋄ 1ₓ 0ₓ]   ⍝ [0ₓ 1ₓ ⋄ 1r2 0ₓ]

⍝ — Triangular float inversion preserves integral path counts
⌹[1 ¯1 0 ⋄ 0 1 ¯1 ⋄ 0 0 1]   ⍝ [1 1 1 ⋄ 0 1 1 ⋄ 0 0 1]

⍝ — Square complex solve with row pivoting
2j2 4⌹[0 1j1 ⋄ 2 0]   ⍝ 2 2

⍝ — Inverting a zero-column matrix exchanges its dimensions
⌹3 0⍴0ₓ   ⍝ 0 3⍴0ₓ

⍝ — Solving for no unknowns preserves the number of right-hand sides
(3 2⍴1)⌹3 0⍴0   ⍝ 0 2⍴0

⍝ —
⌹0
⍝ error: DOMAIN ERROR

⍝ — Singular matrices are rejected rather than assigned an arbitrary solution
⌹2 2⍴1
⍝ error: DOMAIN ERROR

⍝ — The exact solver also rejects singular matrices
⌹2 2⍴1ₓ
⍝ error: DOMAIN ERROR

⍝ —
⌹"ab"
⍝ error: DOMAIN ERROR

⍝ — This solver requires at least as many equations as unknowns
⌹2 3⍴1
⍝ error: LENGTH ERROR

⍝ —
1 2⌹3 2⍴1
⍝ error: LENGTH ERROR

⍝ —
⌹1 1 1⍴1
⍝ error: RANK ERROR

⍝⍝ At and stencil

⍝ — At applies reverse to the selected subarray, not each item separately
⌽@2 4⍳5   ⍝ 1 4 3 2 5

⍝ —
10×@2 4⍳5   ⍝ 1 20 3 40 5

⍝ — A selection function supplies a Boolean mask
0@(2∘|)⍳5   ⍝ 0 2 0 4 0

⍝ — Repeated replacement indices use the last supplied value
10 20@2 2⍳3   ⍝ 1 20 3

⍝ — A matrix of indices gives the replacement function the same frame as ⌷
v←10 20 30 40 ⋄ i←[1 4 ⋄ 2 3] ⋄ {⍵+[100 200 ⋄ 300 400]}@i⊢v
110 320 430 240

⍝ — Major-cell selection retains trailing axes after the index array's frame
{⍵+2 2 1⍴100 200 300 400}@([1 4 ⋄ 2 3])⊢4 2⍴⍳8
[101 102 ⋄ 303 304 ⋄ 405 406 ⋄ 207 208]

⍝ — An empty multidimensional index array leaves the argument unchanged
0@(0 2⍴0)⊢⍳4   ⍝ 1 2 3 4

⍝ — Functional update leaves the original array unchanged
a←⍳3 ⋄ b←0@2⊢a ⋄ a   ⍝ 1 2 3

⍝ — Stencil reports leading-axis edge padding even when rows are empty
{⍺}⌺3⊢2 0⍴0   ⍝ [1 ⋄ ¯1]

⍝ — A two-row stencil specification gives window size then movement
{+/,⍵}⌺([3 ⋄ 2])⍳8   ⍝ 3 9 15 21

⍝ — A single stencil size varies the leading axis and retains whole rows
{⍴⍵}⌺3⊢2 3⍴⍳6   ⍝ 2 2⍴3ₓ

⍝ —
{⍵}⌺3⍳1
⍝ error: DOMAIN ERROR

⍝ —
{⍵}⌺0⍳3
⍝ error: DOMAIN ERROR

⍝ —
{⍵}⌺⍬⍳3
⍝ error: DOMAIN ERROR

⍝ —
{⍵}⌺1 1⍳3
⍝ error: RANK ERROR

⍝ —
0@{2 0 1}⍳3
⍝ error: DOMAIN ERROR

⍝ — Nested index paths select fields inside matrix items
G←2 3⍴("ABC" 1⋄ "DEF" 2⋄ "GHI" 3⋄ "JKL" 4⋄ "MNO" 5⋄ "PQR" 6) ⋄ [((1 2)1⋄ (2 3)2)]⌷G
"DEF" 6

⍝ — At replaces nested fields without replacing their containing items
G←2 3⍴("ABC" 1⋄ "DEF" 2⋄ "GHI" 3⋄ "JKL" 4⋄ "MNO" 5⋄ "PQR" 6) ⋄ H←("" '*' @((1 2)1⋄ (2 3)2))G ⋄ (1⊃1 2⊃H⋄ 2⊃2 3⊃H)
"" '*'

⍝ — At calls its operand once even for an empty selection
{⎕←99 ⋄ ⍵}@⍬⍳3
⍳3
⍝ ⎕: 99

⍝⍝ Encode decode

⍝ —
60⊥3 13   ⍝ 193

⍝ —
2ₓ⊥1ₓ 0ₓ 1ₓ 0ₓ   ⍝ 10ₓ

⍝ — A leading zero radix retains the remaining quotient
0ₓ 10ₓ⊤125ₓ   ⍝ 12ₓ 5ₓ

⍝ — Empty digits decode to zero
2⊥⍬   ⍝ 0

⍝ — An empty radix vector leaves no terms to decode
⍬⊥1   ⍝ 0

⍝ — A zero radix consumes the remainder, leaving zero for earlier digits
0 0 2⊤3   ⍝ 0 1 1

⍝ — Encode appends the value frame to the radix shape, including zero dimensions
⍴(0 3⍴0)⊤2 2⍴1   ⍝ 0ₓ 3ₓ 2ₓ 2ₓ

⍝ — Decode the same digits in binary and decimal
[2 ⋄ 10]⊥1 0 1   ⍝ 5 101

⍝ —
2 3⊥1 2 3
⍝ error: LENGTH ERROR

⍝⍝ Pick and partition

⍝ — Pick descends through a matrix coordinate, a nested field, then a character index
2⊃1⊃2 1⊃2 3⍴("ABC" 1⋄ "DEF" 2⋄ "GHI" 3⋄ "JKL" 4⋄ "MNO" 5⋄ "PQR" 6)
'K'

⍝ — Empty coordinates can repeatedly pick an atom without changing it
(⍬∘⊃)⍣5⊢10   ⍝ 10

⍝ — An empty pick path returns the whole argument
⍬⊃1 2   ⍝ 1 2

⍝ — Negative Pick positions count from the end
¯1⊃10 20 30   ⍝ 30

⍝ — Each Pick coordinate counts from the end when negative
¯1 1⊃[1 2 ⋄ 3 4]   ⍝ 3

⍝ — On an empty axis, Pick gives the fill for ¯1, as it does for 1
¯1⊃⍬   ⍝ 0

⍝ —
2⌷3 4⍴⍳12   ⍝ 5 6 7 8

⍝ — Squad with an axis selects a column rather than a row
2⌷⍤[2]3 4⍴⍳12   ⍝ 2 6 10

⍝ — Choose indexing through ⌷ works at any rank
[(1 2)(2 1)]⌷[1 2 ⋄ 3 4]   ⍝ 2 3

⍝ — Choose coordinates count from the end when negative
[(¯1 ¯1)(1 ¯1)]⌷[1 2 ⋄ 3 4]   ⍝ 4 2

⍝ — Empty indices on both axes retain a rank-two empty result
⍴⍬ ⍬⌷3 4⍴0   ⍝ 0ₓ 0ₓ

⍝ —
⍬⌷1   ⍝ 1

⍝ —
⌷1 2   ⍝ 1 2

⍝ — ∞ takes a whole axis
∞ 2⌷3 4⍴⍳12   ⍝ 2 6 10

⍝ — ¯∞ takes a whole axis in reverse order
¯∞ ∞⌷[1 2 ⋄ 3 4]   ⍝ [3 4 ⋄ 1 2]

⍝ —
∞ ¯∞⌷[1 2 ⋄ 3 4]   ⍝ [2 1 ⋄ 4 3]

⍝ — ¯∞ reverses a keyed axis with its keys
¯∞⌷"aa" "bb":1 2   ⍝ "bb" "aa":2 1

⍝ — Negative positions count from the end
¯1⌷10 20 30   ⍝ 30

⍝ —
¯2 ¯1⌷3 4⍴⍳12   ⍝ 8

⍝ — An array of positions can mix signs
[¯1 1]⌷10 20 30   ⍝ 30 10

⍝ — Selective assignment through ¯∞ writes in reverse order
x←[1 2 ⋄ 3 4] ⋄ (¯∞ ∞⌷x)←[5 6 ⋄ 7 8] ⋄ x   ⍝ [7 8 ⋄ 5 6]

⍝ — Selective assignment through a negative position
x←10 20 30 ⋄ (¯1⌷x)←9 ⋄ x   ⍝ 10 20 9

⍝ —
¯4⌷10 20 30
⍝ error: INDEX ERROR

⍝ — ∞ is valid only as a whole item, not inside an array of positions
[1 ∞]⌷10 20 30
⍝ error: DOMAIN ERROR

⍝ —
⊆1 2   ⍝ ⊂1 2

⍝ —
⊆1   ⍝ ⊂1

⍝ — Nest leaves an already nested vector unchanged
⊆(1 2⋄ 3 4)   ⍝ (1 2⋄ 3 4)

⍝ — Partition starts on a positive rise, not every label change; zero omits an item
3 2 2 1 0 1⊆"abcdef"   ⍝ "abcd" "f"

⍝ — Axis-one partition groups column segments and omits the zero-marked row
1 1 0 1⊆⍤[1]4 2⍴⍳8   ⍝ 2 2⍴(1 3⋄ 2 4⋄ ,7⋄ ,8)

⍝ — No partition starts: retain a prototype with an empty selected axis
0⊂2 3⍴0   ⍝ 0⍴⊂2 0⍴0

⍝ — Partition counts remain visible even when the outer frame is empty
1 0 1⊆0 3⍴0   ⍝ 0 2⍴⊂⍬

⍝ —
0⊃1 2
⍝ error: INDEX ERROR

⍝ —
1 1⊃1 2
⍝ error: RANK ERROR

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
⍝ error: LENGTH ERROR

⍝ —
0⌷1 2
⍝ error: INDEX ERROR

⍝ — A one-element partition marker extends over the whole vector
(,1)⊂"abcd"   ⍝ ,⊂"abcd"

⍝ — A scalar partition label groups the whole vector
1⊆"abcd"   ⍝ ,⊂"abcd"

⍝ — A singleton label vector also extends over the whole argument
(,1)⊆"abcd"   ⍝ ,⊂"abcd"

⍝⍝ Grading

⍝ — Descending grade preserves the original order of ties
⍒2 1 2 1   ⍝ 1ₓ 3ₓ 2ₓ 4ₓ

⍝ — Grade ignores comparison tolerance
⍋1 (1+8E¯15) 1   ⍝ 1ₓ 3ₓ 2ₓ

⍝ — Numbers precede characters; complex numbers sort by real then imaginary part
⍋1j2 1 1j¯2 'a' 0   ⍝ 5ₓ 3ₓ 2ₓ 1ₓ 4ₓ

⍝ — Mixed-domain grade does not round large exact integers through float
⍋9007199254740993ₓ 9007199254740992 9007199254740992ₓ   ⍝ 2ₓ 3ₓ 1ₓ

⍝ — Equal-rank nested arrays compare ravelled contents before shape
⍋([1 2 ⋄ 3 4]⋄ [1 2 0 0 ⋄])   ⍝ 2ₓ 1ₓ

⍝ — Nested rank takes precedence over contents
⍋([1 2 ⋄]⋄ 1 2)   ⍝ 2ₓ 1ₓ

⍝ — Empty nested arrays sort by rank, then shape
⍋(0 5 2⍴0)(0 3 4⍴0)(0 1⍴"")⍬   ⍝ 4ₓ 3ₓ 2ₓ 1ₓ

⍝ — Empty-array prototypes do not break sorting ties
⍋(0⍴⊂1 2⋄ 0⍴0⋄ 0⍴"")   ⍝ 1ₓ 2ₓ 3ₓ

⍝ — Structural order puts numbers before characters before nested arrays
⍋'z' (0 0) 100 'a'   ⍝ 3ₓ 4ₓ 1ₓ 2ₓ

⍝ — Character vectors sort lexicographically, not by length first
⍋"ba" "aaa" "ab"   ⍝ 2ₓ 3ₓ 1ₓ

⍝ — Equal numbers retain their order across numeric representations
⍋1ₓ 1 1j0   ⍝ 1ₓ 2ₓ 3ₓ

⍝ — Interval index shares grade's ordering across numbers, characters and nested arrays
1 'a' (1 2)⍸0 'b' (2 3)   ⍝ 0ₓ 2ₓ 3ₓ

⍝ — Interval lookup preserves exact distinctions beyond float's integer range
9007199254740992ₓ 9007199254740993ₓ⍸9007199254740992 9007199254740994
1ₓ 2ₓ

⍝ — Complex interval lookup uses real-then-imaginary ordering
1j¯1 1 1j1⍸1j¯2 1j0 1j2   ⍝ 0ₓ 2ₓ 3ₓ

⍝ — Zero-width rows are stable ties, not an empty collection of rows
⍋3 0⍴0   ⍝ 1ₓ 2ₓ 3ₓ

⍝ —
⍋⍬   ⍝ 0⍴0ₓ

⍝ — Custom collation puts unlisted characters last, retaining their order
"cba"⍋"azb?c"   ⍝ 5ₓ 3ₓ 1ₓ 2ₓ 4ₓ

⍝ — A multidimensional collation array supplies successive collation keys
["AB" ⋄ "BA"]⍋["BA" ⋄ "AB" ⋄ "BA"]   ⍝ 1ₓ 2ₓ 3ₓ

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
1÷0 1
⍝ error: DOMAIN ERROR

⍝ —
v←⍳1000 ⋄ +/v+v   ⍝ 1001000

⍝ —
×/1 2 3 4   ⍝ 24

⍝ — Generic reduction stays right-associated despite floating-point cancellation
{⍺+⍵}/1E100 ¯1E100 1   ⍝ 0

⍝ —
+/1ₓ 2ₓ 3ₓ   ⍝ 6ₓ

⍝ — Zero divided by zero is one, applied elementwise
0 1÷0 2   ⍝ 1 0.5

⍝⍝ Unicode conversion

⍝ — Case folding pervades nested text and leaves numbers unchanged
•c 42 "Pete" "Πέτρος"   ⍝ 42 "pete" "πέτροσ"

⍝ — Simple uppercase preserves shape and never expands one character to several
1•c ["aBc" ⋄ "Σςß"]   ⍝ ["ABC" ⋄ "ΣΣß"]

⍝ — Simple lowercase maps Unicode characters without expanding them
¯1•c "İẞᾈΣ"   ⍝ "ißᾀσ"

⍝ — Simple case folding differs from full folding for dotted I and ligatures
•c "ẞİﬀᾀ"   ⍝ "ßİﬀᾀ"

⍝ — A singleton selector of any rank is accepted for case folding
[¯3 ⋄]•c "ίσως"   ⍝ "ίσωσ"

⍝ — Case conversion preserves an empty nested character prototype
•c 2 0⍴⊂"Ab"   ⍝ 2 0⍴⊂"  "

⍝ — System names are case-insensitive and may be bound to ordinary names
u←•UcS ⋄ u ["A⍳" ⋄ "λ😀"]   ⍝ [65ₓ 9075ₓ ⋄ 955ₓ 128512ₓ]

⍝ —
•ucs [65ₓ 9075ₓ ⋄ 955ₓ 128512ₓ]   ⍝ ["A⍳" ⋄ "λ😀"]

⍝ — Code-point conversion preserves an empty character array's shape
•ucs 2 0⍴""   ⍝ 2 0⍴0ₓ

⍝ — Empty numeric input converts to a character prototype
•ucs 0 3⍴0   ⍝ 0 3⍴""

⍝ —
"UTF-8"•ucs "Æ😀"   ⍝ 195ₓ 134ₓ 240ₓ 159ₓ 152ₓ 128ₓ

⍝ —
("UTF-8" 0)•ucs 195 134 240 159 152 128   ⍝ "Æ😀"

⍝ — UTF-16 encodes an astral character as a surrogate pair
"UTF-16"•ucs "A😀"   ⍝ 65ₓ 55357ₓ 56832ₓ

⍝ — UTF-16 decodes a surrogate pair into one character
"UTF-16"•ucs 65 55357 56832   ⍝ "A😀"

⍝ — An enclosed encoding name is accepted; UTF-32 retains one unit per character
["UTF-32"]•ucs "A😀"   ⍝ 65ₓ 128512ₓ

⍝ —
"UTF-32"•ucs 65 128512   ⍝ "A😀"

⍝ — Encoding a scalar character returns a byte vector
"UTF-8"•ucs 'A'   ⍝ ,65ₓ

⍝ — Decoding a scalar byte returns a character vector
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

⍝ — A nested prototype does not make empty input a simple character array
•ucs 0⍴⊂"ab"
⍝ error: DOMAIN ERROR

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
("UTF-8" 83)•ucs "abc"
⍝ error: UNSUPPORTED

⍝⍝ Character and string literals

⍝ — Single quotes hold exactly one character, a scalar
⍴'a'   ⍝ 0⍴0ₓ

⍝ — Double quotes make a vector, even for one character
⍴"a"   ⍝ ,1ₓ

⍝ — The quote character needs no escaping inside single quotes
•ucs '''   ⍝ 39ₓ

⍝ — Apostrophes need no escaping inside double quotes
≢"it's"   ⍝ 4ₓ

⍝ — A doubled double quote is one quote character
≢"say ""hi"""   ⍝ 8ₓ

⍝ — The empty string
⍴""   ⍝ ,0ₓ

⍝ — Adjacent strings strand into separate items, so each is one key
("x" "y":1 2).y   ⍝ 2

⍝ — Adjacent characters strand into one string
'x' 'y'   ⍝ "xy"

⍝ — A comment mark inside a string is an ordinary character
≢"it's ⍝ here"   ⍝ 11ₓ

⍝ — Single quotes around several characters
'ab'
⍝ error: SYNTAX ERROR

⍝ — Empty single quotes
''
⍝ error: SYNTAX ERROR

⍝ — An unclosed string
"abc
⍝ error: SYNTAX ERROR

⍝ — Strings print in double quotes, and characters in single quotes
⎕←"ab" "cd" ⋄ ⎕←'x' 1 ⋄ ⎕←''' "a""b" ⋄ ⎕←"aa" "bb":"x" 'y' ⋄ ⎕←"" "a" ⋄ 0
0
⍝ ⎕: "ab" "cd"\n'x' 1\n''' "a""b"\n("aa":"x" ⋄ "bb":'y')\n"" "a"

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
"012"-'0'   ⍝ 0ₓ 1ₓ 2ₓ

⍝ —
("ab" "CD")+1   ⍝ "bc" "DE"

⍝ — Character offsets follow leading-axis agreement
["abc" ⋄ "def"]+1 2   ⍝ ["bcd" ⋄ "fgh"]

⍝ —
""+3   ⍝ ""

⍝ — Empty character subtraction has an exact numeric prototype
""-'a'   ⍝ 0⍴0ₓ

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

⍝ — Character offsets must be integral without comparison tolerance
'a'+1.000000000000001
⍝ error: DOMAIN ERROR

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

⍝⍝ Characters nesting and empty fill

⍝ — System functions use ordinary binding and composition
upper←1∘•c ⋄ (•ucs∘upper)"aZ"   ⍝ 65ₓ 90ₓ

⍝ — System names resolve when executed
f←{•missing ⍵} ⋄ 1   ⍝ 1

⍝ — A system name includes its entire word
•c_unknown2 "abc"
⍝ error: UNSUPPORTED

⍝ — System functions are read-only
•c←+
⍝ error: SYNTAX ERROR

⍝ — Explicit strands can hold system functions
fs←•ucs˘•c ⋄ (↑fs)'A'   ⍝ 65ₓ

⍝ —
•a   ⍝ "ABCDEFGHIJKLMNOPQRSTUVWXYZ"

⍝ —
•d   ⍝ "0123456789"

⍝ —
3↑•a   ⍝ "ABC"

⍝ — Alphabet constants are read-only
•a←"abc"
⍝ error: SYNTAX ERROR

⍝ — basedpl has fixed origin one, not a mutable index-origin variable
•io←0
⍝ error: UNSUPPORTED

⍝ —
'abc
⍝ error: SYNTAX ERROR

⍝ — String literals cannot span source lines
"a
b"
⍝ =>
⍝ error: SYNTAX ERROR

⍝ — Scalar ordering is numeric; character sorting uses grade
'a'<'b'
⍝ error: DOMAIN ERROR

⍝ —
1.5↑1 2
⍝ error: DOMAIN ERROR

⍝ —
¯1⍴2
⍝ error: DOMAIN ERROR

⍝ —
1000001⍴1
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
↑0⍴(1 2⋄ 3 4 5)   ⍝ 0.0 0.0

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

⍝ — Direct scalar division on empty text makes no element calls
1÷""   ⍝ ⍬

⍝ —
"ab"=1 2   ⍝ 0ₓ 0ₓ

⍝ —
"abc"="axc"   ⍝ 1ₓ 0ₓ 1ₓ

⍝ —
+"abc"   ⍝ "abc"

⍝ —
+""   ⍝ ""

⍝ — An atomic index retrieves a character vector
2⌷"abc" "def" "ghi"   ⍝ "def"

⍝ —
gg←2 3 4 5 ⋄ 9,gg.(2),3 4   ⍝ 9 3 3 4

⍝ — Split into rows; each further split encloses the resulting vector or scalar
↓↓↓2 2⍴⍳4   ⍝ ⊂⊂(1 2⋄ 3 4)

⍝ — Reduction returns the accumulated vector
+/(1 2⋄ 3 4)   ⍝ 4 6

⍝ —
(1 2⋄ 3 4)+10   ⍝ (11 12⋄ 13 14)

⍝ —
0 3⍴⍬   ⍝ 0 3⍴0

⍝⍝ Matrix axes scan and cell assembly

⍝ —
+/2 3⍴⍳6   ⍝ 6 15

⍝ —
+⌿2 3⍴⍳6   ⍝ 5 7 9

⍝ — Subtraction scan accumulates from left to right
-\1 2 3   ⍝ 1 ¯1 ¯4

⍝ — Float scan retains left-to-right accumulation through cancellation
+\1E100 ¯1E100 1   ⍝ 1E100 0 1

⍝ — A dfn scan uses the same left-to-right order as primitive scan
{⍺+⍵}\1E100 ¯1E100 1   ⍝ 1E100 0 1

⍝ — Dyadic scan starts from a seed and omits the seed from its result
10 -\1 2 3   ⍝ 9 7 4

⍝ —
100 +\10 20   ⍝ 110 130

⍝ —
10ₓ -\1ₓ 2ₓ 3ₓ   ⍝ 9ₓ 7ₓ 4ₓ

⍝ — Seeded scalar scan still applies the operand once
2 +\5   ⍝ 7

⍝ — Trailing-axis scan takes one seed per row
10 20 (+\⍤0 1)2 3⍴⍳6   ⍝ [11 13 16 ⋄ 24 29 35]

⍝ — Leading-axis scan takes one seed per column
⍉10 20 30 (+\⍤0 1)⍉2 3⍴⍳6   ⍝ [11 22 33 ⋄ 15 27 39]

⍝ — Rank pairs row seeds for scan with an axis
10 20 ({⍺+⍵}\⍤[1]⍤0 1)2 3⍴⍳6   ⍝ [11 13 16 ⋄ 24 29 35]

⍝ — Growing scan accumulators remain nested, without mix-style padding
{⍺,⍵}\1 2 3   ⍝ 1 (1 2) (1 2 3)

⍝ — A seed participates in every growing accumulator
0 {⍺,⍵}\1 2 3   ⍝ (0 1⋄ 0 1 2⋄ 0 1 2 3)

⍝ — An empty seeded scan makes no operand calls
10 {1÷0}\⍬   ⍝ ⍬

⍝ — An unseeded singleton scan makes no operand calls
{1÷0}\,5   ⍝ ,5

⍝ — Empty scan rows retain their frame and shape
10 20 +\2 0⍴0   ⍝ 2 0⍴0

⍝ — Three empty rows reduce to three identities
+/3 0⍴0   ⍝ 0 0 0

⍝ — No rows gives no reduction results
+/0 3⍴0   ⍝ ⍬

⍝ —
+\2 3⍴⍳6   ⍝ [1 3 6 ⋄ 4 9 15]

⍝ —
+⍀2 3⍴⍳6   ⍝ [1 2 3 ⋄ 5 7 9]

⍝ —
+\0 3⍴0   ⍝ 0 3⍴0

⍝ — Mix pads shorter cells to the longest cell
⊃(1 2⋄ 3 4 5)   ⍝ 2 3⍴1 2 0 3 4 5

⍝ — Mix pads an atomic cell rather than extending its value
⊃1 (2 3)   ⍝ 2 2⍴1 0 2 3

⍝ — Empty mix uses the retained cell prototype to determine its trailing shape
⊃0⍴(1 2⋄ 3 4 5)   ⍝ 0 2⍴0

⍝ —
⊃⊂1 2   ⍝ 1 2

⍝ — Empty nested reduction uses a conforming identity
+/0⍴(1 2⋄ 3 4)   ⍝ 0 0

⍝ — Empty character sum uses the numeric addition identity
+/""   ⍝ 0

⍝ — Leading-axis replicate keeps the selected matrix row
1 0⌿2 3⍴⍳6   ⍝ [1 2 3 ⋄]

⍝ — Each accumulator retains the whole vector seed
(,10)+\1 2 3   ⍝ (,11⋄ ,13⋄ ,16)

⍝ — Each lane starts with the same whole seed
10 20 30+\2 3⍴⍳6
2 3⍴(11 21 31⋄ 13 23 33⋄ 16 26 36⋄ 14 24 34⋄ 19 29 39⋄ 25 35 45)

⍝ — An empty scan makes no calls with its array seed
10 20 30+\2 0⍴0   ⍝ 2 0⍴0

⍝ — A named leading-axis hybrid still acts as reduction
m←2 3⍴⍳6 ⋄ r←⌿ ⋄ +r m   ⍝ 5 7 9

⍝ — The operand observes successive left accumulators in order
f←{⎕←⍺ ⍵ ⋄ ⍺-⍵} ⋄ f\1 2 3
1 ¯1 ¯4
⍝ ⎕: 1 2\n¯1 3

⍝ — Seeded scan exposes the seed as the first left argument
r←10 {⎕←⍺ ⍵ ⋄ ⍺-⍵}\1 2 3
9 7 4
⍝ ⎕: 10 1\n9 2\n7 3

⍝⍝ Structural slices and brackets

⍝ — Standalone brackets enclose arrays; nested brackets retain both scalar layers
[[1 2 3]]   ⍝ ⊂⊂1 2 3

⍝ — A named operator accepts an enclosed operand through the ordinary binding rule
p←⍣ ⋄ (1∘+)p[2]⊢0   ⍝ 0 1 2

⍝ — Functions inside brackets are values, including trains
fs←[+÷≢] ⋄ (↑fs)2 4 9   ⍝ (2 4 9)÷3

⍝ — Bracket expressions use the enclosing function's arguments
{[⍺+⍵]}⍨3   ⍝ ⊂6

⍝ — Enclosure evaluates its contents once
[⎕←3]
⊂3
⍝ ⎕: 3

⍝ — Index selects from a disclosed enclosure
v←[1 2 3] ⋄ 2⌷↑v   ⍝ 2

⍝ — Standalone empty brackets have no expression to enclose
[]
⍝ error: SYNTAX ERROR

⍝ —
⍉[1 2 3 ⋄ 4 5 6]   ⍝ [1 4 ⋄ 2 5 ⋄ 3 6]

⍝ —
⌽[1 2 3 ⋄ 4 5 6]   ⍝ [3 2 1 ⋄ 6 5 4]

⍝ —
⊖[1 2 3 ⋄ 4 5 6]   ⍝ [4 5 6 ⋄ 1 2 3]

⍝ — Rotate each row by its own count
1 2⌽[1 2 3 ⋄ 4 5 6]   ⍝ [2 3 1 ⋄ 6 4 5]

⍝ — Catenating a vector to a matrix appends one element per row
[1 2 3 ⋄ 4 5 6],8 9   ⍝ [1 2 3 8 ⋄ 4 5 6 9]

⍝ —
1 2↑[1 2 3 ⋄ 4 5 6]   ⍝ [1 2 ⋄]

⍝ — Negative multidimensional overtake pads before the retained bottom-right cells
¯3 ¯2↑[1 2 3 ⋄ 4 5 6]   ⍝ [0 0 ⋄ 2 3 ⋄ 5 6]

⍝ —
1 ¯1↓[1 2 3 ⋄ 4 5 6]   ⍝ [4 5 ⋄]

⍝ — Taking a matrix from a scalar pads rather than repeating it
2 3↑7   ⍝ [7 0 0 ⋄ 0 0 0]

⍝ —
2 1⌷[1 2 3 ⋄ 4 5 6]   ⍝ 4

⍝ —
∞ 2⌷[1 2 3 ⋄ 4 5 6]   ⍝ 2 5

⍝ —
2 ∞⌷[1 2 3 ⋄ 4 5 6]   ⍝ 4 5 6

⍝ —
[3 1]⌷10 20 30   ⍝ 30 10

⍝ — The first brackets enclose the axis operand; the second form an array literal
+/⍤[1][1 2 3 ⋄ 4 5 6]   ⍝ 5 7 9

⍝ — A named reduction accepts the same axis operand
s←+/ ⋄ s⍤[1][1 2 3 ⋄ 4 5 6]   ⍝ 5 7 9

⍝ — Repeated transpose axes select the diagonal of a rectangular matrix
1 1⍉[1 2 3 ⋄ 4 5 6]   ⍝ 1 5

⍝ — Negative replicate counts replace that item with fill
2 ¯1 1/10 20 30   ⍝ 10 10 0 30

⍝ — Negative expand counts insert fill without consuming an input item
2 ¯1 1\10 20   ⍝ 10 10 0 20

⍝ — Expansion uses character fill for inserted positions
1 0 1\"ab"   ⍝ "a b"

⍝ —
0⌷1 2
⍝ error: INDEX ERROR

⍝ — Negative positions count from the end
¯1⌷1 2   ⍝ 2

⍝ —
3⌷1 2
⍝ error: INDEX ERROR

⍝ —
1.5⌷1 2
⍝ error: DOMAIN ERROR

⍝ — Omitted trailing axes select the whole cell
1⌷[1 2 ⋄ 3 4]   ⍝ 1 2

⍝ —
+/⍤[0][1 2 ⋄ 3 4]
⍝ error: DOMAIN ERROR

⍝ — Nested literals and dfn guards do not split the surrounding matrix incorrectly
∞ 1⌷[({⍵=0:1 ⋄ ⍵+2}0 ⋄ 3) ⋄ (4 ⋄ 5)]   ⍝ 1 4

⍝⍝ Scalar math

⍝ — Complex floor uses Gaussian-integer cells, not independent component floors
⌊1.5j0.5   ⍝ 1j1

⍝ — Complex gcd uses Gaussian-integer arithmetic and tolerant comparison
0.2=3.8j7.6∨5.2j6.8   ⍝ 1ₓ

⍝ — Scale and ceil to expose the complex lcm despite floating-point roundoff
⌈1000×3.8j7.6∧5.2j6.8   ⍝ ¯159600j326800

⍝ —
|3 ¯3 3j4   ⍝ 3 3 5

⍝ — Residue follows the divisor's sign, including fractional divisors
2 10 ¯2.5|7 ¯13 8   ⍝ 1 7 ¯2

⍝ — Zero residue-base returns the argument unchanged
0 3|¯2 6   ⍝ ¯2 0

⍝ — Floor applies comparison tolerance near integers
⌊1.000000000000001 ¯1.000000000000001   ⍝ 1 ¯1

⍝ —
⌈¯2.3 0.1 3   ⍝ ¯2 1 3

⍝ —
2 3⌊3 2   ⍝ 2 2

⍝ —
2 3⌈3 2   ⍝ 3 3

⍝ —
2 ¯2 0*3 3 0   ⍝ 8 ¯8 1

⍝ — Circle selectors nine and eleven extract real and imaginary parts
9 11○3j4   ⍝ 3 4

⍝ —
2ₓ*¯3ₓ   ⍝ 1r8

⍝ —
2r3|7r3   ⍝ 1r3

⍝ —
⌊¯4r3   ⍝ ¯2ₓ

⍝ —
⌈¯4r3   ⍝ ¯1ₓ

⍝ —
⍟0
⍝ error: DOMAIN ERROR

⍝ —
0*¯1
⍝ error: DOMAIN ERROR

⍝ —
13○1
⍝ error: DOMAIN ERROR

⍝ —
0j1⌊2
⍝ error: DOMAIN ERROR

⍝⍝ Search depth and random

⍝ — Index-of compares each candidate directly; tolerance is not transitive
x←1 ⋄ y←1+8E¯15 ⋄ z←1+16E¯15 ⋄ x y⍳z   ⍝ 2ₓ

⍝ — Unique-mask compares against retained representatives, not every earlier item
≠(1⋄ 1+8E¯15⋄ 1+16E¯15)   ⍝ 1ₓ 0ₓ 1ₓ

⍝ — Without uses tolerance when approximate values participate
1 2~1+8E¯15   ⍝ ,2

⍝ — Without does not apply tolerance to all-exact values
1ₓ 2ₓ~1000000000000001r1000000000000000   ⍝ 1ₓ 2ₓ

⍝ — Each empty row matches the first empty row
(3 0⍴0)⍳2 0⍴0   ⍝ 1ₓ 1ₓ

⍝ — A matrix of empty rows still has one unique major cell
≠3 0⍴0   ⍝ 1ₓ 0ₓ 0ₓ

⍝ — Where repeats a position according to its count
⍸0 1 0 2   ⍝ 2ₓ 4ₓ 4ₓ

⍝ — Interval index ignores comparison tolerance at interval boundaries
1 2⍸1-1E¯15   ⍝ 0ₓ

⍝ — Depth of an empty nested array comes from its prototype
≡0⍴(1 2)3   ⍝ 2ₓ

⍝ — Not-match distinguishes an atom from a singleton vector
1≢,1   ⍝ 1ₓ

⍝ — Match compares numeric value across exact and approximate domains
1ₓ≡1   ⍝ 1ₓ

⍝ — Empty numeric prototypes match across exact and approximate domains
(0⍴1ₓ)≡⍬   ⍝ 1ₓ

⍝ —
⍳,3   ⍝ 1 2 3

⍝ — Union retains duplicates within either argument
1 1∪2 2   ⍝ 1 1 2 2

⍝ —
1 2∊1+8E¯15   ⍝ 1ₓ 0ₓ

⍝ — A vector pattern does not fit a scalar search target
(,1)⍷1   ⍝ 0ₓ

⍝ — An empty pattern still cannot exceed the target on another axis
(2 0⍴0)⍷1 3⍴1   ⍝ 1 3⍴0ₓ

⍝ — Rank-zero iota contains one enclosed empty coordinate
⍳⍬   ⍝ ⊂⍬

⍝ — Where on scalar zero retains an empty-coordinate prototype
⍸0   ⍝ 0⍴⊂0⍴0ₓ

⍝ —
3?2
⍝ error: DOMAIN ERROR

⍝ —
?¯1
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

⍝ —
2 1⍸1
⍝ error: DOMAIN ERROR

⍝⍝ Each commute and reduction

⍝ — Empty each calls Pick on prototypes to determine numeric fill
⍬⊃¨⊂1 2 3   ⍝ ⍬

⍝ — Prototype-mode Pick also works through a dfn
⍬{⍺⊃⍵}¨⊂1 2 3   ⍝ ⍬

⍝ — Prototype-mode Pick can supply character fill for an out-of-range positive index
f←{100⊃"abc"} ⋄ f¨⍬   ⍝ ""

⍝ — Prototype mode survives nested helper calls
pick←{⍺⊃⍵} ⋄ wrap←{⍺ pick ⍵} ⋄ ⍬ wrap¨⊂"abc"   ⍝ ""

⍝ — Prototype mode also passes through a defined operator
op←{⍶ ⍵} ⋄ ({100⊃"abc"}op)¨⍬   ⍝ ""

⍝ — A composed operand derives character fill in empty each
⍬(⊃∘⊢)¨⊂"abc"   ⍝ ""

⍝ — Commute preserves prototype-mode Pick
[1 2 3]⊃⍨¨⍬   ⍝ ⍬

⍝ — Empty Pick-each retains the selected nested vector's shape
⍬⊃¨⊂(1 2⋄ 3 4 5)   ⍝ 0⍴⊂0 0

⍝ — An empty-path prototype selects the whole right-hand vector
(0⍴⊂⍬)⊃¨⊂1 2 3   ⍝ 0⍴⊂0 0 0

⍝ — The requested nested item, not always the first, determines result fill
2⊃¨0⍴⊂(1 2⋄ 3 4 5)   ⍝ 0⍴⊂0 0 0

⍝ — Named functions and operators retain their grammatical roles
e←¨ ⋄ sum←+/ ⋄ sum e (1 2)(3 4 5)   ⍝ 3 12

⍝ — Nested each applies singleton reduction to each numeric leaf
+/¨¨(1 2⋄ 3 4)   ⍝ (1 2⋄ 3 4)

⍝ — Selfie with an array operand creates a constant function
2⍨3   ⍝ 2

⍝ —
2-⍨5   ⍝ 3

⍝ — Oversized windows have an empty frame
+/3↕1 2   ⍝ ⍬

⍝ — Reduce adjacent-row windows along their window axis
+/⍤[2]2↕2 3⍴⍳6   ⍝ [5 7 9 ⋄]

⍝ —
-/⍬   ⍝ 0

⍝ —
÷/⍬   ⍝ 1

⍝ — Empty minimum reduction has positive infinity as identity
⌊/⍬   ⍝ ∞

⍝ — Empty each invokes reciprocal on the zero prototype, which errors
÷¨⍬
⍝ error: DOMAIN ERROR

⍝ — Dyadic empty each also evaluates its prototype call
1÷¨⍬
⍝ error: DOMAIN ERROR

⍝ —
2¨3
⍝ error: DOMAIN ERROR

⍝ —
f←2¨
⍝ error: DOMAIN ERROR

⍝ —
f←2∘3
⍝ error: DOMAIN ERROR

⍝ —
f←+⍥3
⍝ error: DOMAIN ERROR

⍝ —
f←+⌺×
⍝ error: DOMAIN ERROR

⍝ — Prototype mode accepts negative Pick positions
¯1⊃¨0⍴⊂1 2 3   ⍝ ⍬

⍝ — Negative Pick positions are valid through a dfn in prototype mode
¯1{⍺⊃⍵}¨0⍴⊂1 2 3   ⍝ ⍬

⍝ —
+/0↕5
⍝ error: RANK ERROR

⍝ — Prototype-mode partial Pick retains trailing axes
⍬⊃¨⊂2 3⍴⍳6   ⍝ 0⍴⊂0 0 0

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
1{⎕←⍺ ⍵ ⋄ 0}¨⍬
⍬
⍝ ⎕: 1 0

⍝ — A no-result operand call makes the whole each return no result
{⍵=2:{}⍵ ⋄ ⍵}¨1 2 3   ⍝ {}0

⍝⍝ Composition rank and dyadic operators

⍝ — A named compose operator derives a sum-of-iota function
c←∘ ⋄ sum←+/c⍳ ⋄ sum¨2 4 6   ⍝ 3 10 21

⍝ — Dyadic Behind reshapes the right argument to the left argument's shape
"abc"⍴⍛⍴'z'   ⍝ "zzz"

⍝ — Behind combines matrix rows as imaginary and real components
¯11∘○⍛+⌿[1 2 3 ⋄ 4 5 6]   ⍝ 4j1 5j2 6j3

⍝ — Naming compose, Behind and reduction preserves their binding
c←∘ ⋄ b←⍛ ⋄ r←⌿ ⋄ ¯11 c ○ b + r [1 2 3 ⋄ 4 5 6]   ⍝ 4j1 5j2 6j3

⍝ — Compose chains bind before reduction derives its function
-∘+∘×/1 2 3   ⍝ 0

⍝ — Rank assembles differently sized iota cells with padding
⍳⍤0⊢1 3 2   ⍝ 3 3⍴1 0 0 1 2 3 1 2 0

⍝ — Empty rank application uses an operand prototype call to determine cell shape
({⍳3}⍤1)0 2⍴0   ⍝ 0 3⍴0

⍝ — A dyadic defined operator can take two array operands
op←{⍶+⍹×⍵} ⋄ (2 op 3)4   ⍝ 14

⍝ — A dyadic defined operator can compose two function operands
op←{⍶ ⍹ ⍵} ⋄ (+/op⍳)4   ⍝ 10

⍝ — Operator operands retain access to their active lexical frame
f←{k←3 ⋄ g←{k+⍵} ⋄ op←{⍶ ⍹ ⍵} ⋄ (+op g)⍵} ⋄ f 4   ⍝ 7

⍝ —
+⍤⍬⊢3
⍝ error: LENGTH ERROR

⍝ —
+⍤0.5⊢3
⍝ error: DOMAIN ERROR

⍝ — Rank ∞ gives the operand the whole argument
m←[1 2 3 ⋄ 4 5 6] ⋄ m +⌿⍤×⍤1 ∞⊢1 2 3   ⍝ 14 32

⍝ — With ranks 1 ∞, the matrix-vector product also multiplies matrices
m←[1 2 3 ⋄ 4 5 6] ⋄ m +⌿⍤×⍤1 ∞⊢[1 10 ⋄ 2 20 ⋄ 3 30]   ⍝ [14 140 ⋄ 32 320]

⍝ — Rank ∞ clamps to the argument's rank, and ¯∞ to rank 0
m←[1 2 ⋄ 3 4] ⋄ ((⊂⍤∞⊢m)≡⊂m) ((⊂⍤¯∞⊢m)≡⊂⍤0⊢m)   ⍝ 1ₓ 1ₓ

⍝ — Empty rank application exposes its single prototype call
{⎕←⍵ ⋄ ⍳3}⍤0⊢⍬
0 3⍴0
⍝ ⎕: 0

⍝ — Over transforms the right argument before the left
f←{⎕←⍵ ⋄ ⍵} ⋄ 2 +⍥f 3
5
⍝ ⎕: 3\n2

⍝⍝ Key and power

⍝ — Key groups right-hand values using the left-hand labels
1 1 2{+/⍵}⌸10 20 30   ⍝ 30 30

⍝ — Empty Key retains the operand's result-cell shape
{⍳3}⌸⍬   ⍝ 0 3⍴0

⍝ — Predicate power tests the next iterate after each step
1(+⍣{⍺>4})0   ⍝ 5

⍝ — Power accepts a singleton-vector stopping result
(+∘1)⍣{,⍺=3}0   ⍝ 3

⍝ — A stopping result may have any rank but must contain one Boolean
(+∘1)⍣{1 1⍴⍺=3}0   ⍝ 3

⍝ —
(2∘×⍣0)3   ⍝ 3

⍝ — Tolerant Key groups by representatives, not transitive closure
{≢⍵}⌸1 (1+8E¯15)(1+16E¯15)   ⍝ 2ₓ 1ₓ

⍝ —
{⍺ ⍵}⌸7
⍝ error: RANK ERROR

⍝ —
1 2{⍵}⌸3 4 5
⍝ error: LENGTH ERROR

⍝ —
(+∘1)⍣{1 1}0
⍝ error: LENGTH ERROR

⍝ —
(+∘1)⍣{⍬}0
⍝ error: LENGTH ERROR

⍝ —
(+⍣0.5)1
⍝ error: DOMAIN ERROR

⍝ — Empty Key calls the operand with an empty group retaining the value-cell shape
r←⍬ {⎕←⍴⍵ ⋄ ⍳3}⌸0 2⍴0
0 3⍴0
⍝ ⎕: 0ₓ 2ₓ

⍝ — Zero iterations never call the operand
({⎕←7 ⋄ ⍵}⍣0)3
3
⍝ ⎕:

⍝ — Power propagates an operand's no-result return
({}⍣1)3   ⍝ {}0

⍝⍝ Products

⍝ — Inner product may use catenate rather than a scalar reduction
1 2 3,.-3 3⍴4 5 6   ⍝ (¯3 ¯2 ¯1⋄ ¯4 ¯3 ¯2⋄ ¯5 ¯4 ¯3)

⍝ — This fork enumerates leading-axis indices and selects each major cell
(⍳∘≢( ⌷⌝ )⊂)2 3 3⍴⍳18
([1 2 3 ⋄ 4 5 6 ⋄ 7 8 9]⋄ [10 11 12 ⋄ 13 14 15 ⋄ 16 17 18])

⍝ —
(2 3⍴⍳6)+.×3 2⍴⍳6   ⍝ [22 28 ⋄ 49 64]

⍝ — An empty contraction fills every result cell with the reduction identity
(2 0⍴0)+.×0 3⍴0   ⍝ 2 3⍴0

⍝ — Singleton extension also applies along the contracted axis
(,2)+.×1 2 3   ⍝ 12

⍝ — Named outer-product operator
outer←⌝ ⋄ times←× ⋄ 1 2 times outer 3 4   ⍝ [3 4 ⋄ 6 8]

⍝ — Parenthesized outer-product operator
1 2 +(⌝) 10 20   ⍝ [11 21 ⋄ 12 22]

⍝ — Outer product accepts composed operands
1 2 (-⍤+)⌝ 10 20   ⍝ [¯11 ¯21 ⋄ ¯12 ¯22]

⍝ — Commute follows outer-product derivation
-⌝⍨1 2   ⍝ [0 ¯1 ⋄ 1 0]

⍝ — Empty outer product preserves both argument frames
⍬ +⌝ 7 8   ⍝ 0 2⍴0

⍝ —
1 2+.×1 2 3
⍝ error: LENGTH ERROR

⍝ — Empty outer product makes one prototype call using the first right item
r←⍬ {⎕←⍺ ⍵ ⋄ ⍺+⍵}⌝ 7 8
0 2⍴0
⍝ ⎕: 0 7

⍝ — Empty inner-product frames still derive fill through the real operand path
r←(0 2⍴0)+.{⎕←⍺ ⍵ ⋄ ⍺×⍵}2 3⍴⍳6
0 3⍴0
⍝ ⎕: 0 1\n0 2\n0 3\n0 4\n0 5\n0 6

⍝⍝ Complex arithmetic and roundtrips

⍝ — Empty complex product has multiplicative identity one. https://docs.dyalog.com/20.0/programming-reference-guide/introduction/complex-numbers/
×/0/1j2   ⍝ 1

⍝ — Iota accepts a complex representation only when its imaginary part is zero
⍳3j0   ⍝ 1 2 3

⍝⍝ Complex comparison errors and recovery

⍝ — A complex literal requires an imaginary component
1j
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
1xj2
⍝ error: SYNTAX ERROR

⍝ — Complex components cannot use rational-literal syntax
1r2j3
⍝ error: SYNTAX ERROR

⍝ —
1j2j3
⍝ error: SYNTAX ERROR

⍝ — Complex components must remain finite
1j1E309
⍝ error: DOMAIN ERROR

⍝ —
1E309j1
⍝ error: DOMAIN ERROR

⍝ — Overflow to complex infinity is rejected
1E308j1E308+1E308j1E308
⍝ error: DOMAIN ERROR

⍝ —
1j2÷0
⍝ error: DOMAIN ERROR

⍝ —
÷0j0
⍝ error: DOMAIN ERROR

⍝ — Counts require exactly zero imaginary part, without tolerance
⍳1j1E¯15
⍝ error: DOMAIN ERROR

⍝ —
1j2/3
⍝ error: DOMAIN ERROR

⍝ — Complex numbers have no scalar ordering, even against themselves
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

⍝⍝ Exact literals arithmetic and roundtrips

⍝ — One approximate operand makes division approximate, even when the other is exact
1ₓ÷2   ⍝ 0.5

⍝ —
(1÷3)+(1÷6)   ⍝ 0.5

⍝ —
1r4+0.5   ⍝ 0.75

⍝ —
1r0
⍝ error: DOMAIN ERROR

⍝ —
0r0
⍝ error: DOMAIN ERROR

⍝ —
1ₓ÷0ₓ
⍝ error: DOMAIN ERROR

⍝ —
÷0ₓ
⍝ error: DOMAIN ERROR

⍝ —
1r
⍝ error: SYNTAX ERROR

⍝ —
1r¯
⍝ error: SYNTAX ERROR

⍝ — Exact-number suffixes require integer syntax, not a decimal literal
1.5x
⍝ error: SYNTAX ERROR

⍝ — Exact-number suffixes do not accept exponent notation
1E2x
⍝ error: SYNTAX ERROR

⍝ —
1r2.5
⍝ error: SYNTAX ERROR

⍝ —
1r2ₓ
⍝ error: SYNTAX ERROR

⍝ —
1x2
⍝ error: SYNTAX ERROR

⍝⍝ Exact arrays prototypes and counts

⍝ — Empty exact sum retains an exact additive identity
+/0/1r3   ⍝ 0ₓ

⍝ — Empty exact product retains an exact multiplicative identity
×/0/1r3   ⍝ 1ₓ

⍝ — Shape returns exact dimensions even for approximate data
⍴2 3⍴0.5   ⍝ 2ₓ 3ₓ

⍝ — Scalar shape is an empty exact vector
⍴42   ⍝ 0⍴0ₓ

⍝ —
⍴""   ⍝ ,0ₓ

⍝ —
≢"abc"   ⍝ 3ₓ

⍝ — Tally counts rows, so a zero-row matrix has tally zero
≢0 3⍴0.5   ⍝ 0ₓ

⍝ — Nested element shapes do not affect the outer shape
⍴(1r3⋄ 1.5 2)   ⍝ ,2ₓ

⍝ — Iota follows an exact argument's numeric domain
⍳3ₓ   ⍝ 1ₓ 2ₓ 3ₓ

⍝ —
⍳3r2
⍝ error: DOMAIN ERROR

⍝ —
⍳¯1ₓ
⍝ error: DOMAIN ERROR

⍝ —
⍳1000001ₓ
⍝ error: LIMIT ERROR

⍝ —
⍳999999999999999999999ₓ
⍝ error: LIMIT ERROR

⍝ —
1r2/3
⍝ error: DOMAIN ERROR

⍝⍝ Exact and tolerant comparisons

⍝ — Approximate equality uses relative tolerance 1E¯14. https://docs.dyalog.com/20.0/language-reference-guide/system-functions/ct/
0.3=0.3 (0.1+0.2) 0.4   ⍝ 1ₓ 1ₓ 0ₓ

⍝ —
1 2≤2 1   ⍝ 1ₓ 0ₓ

⍝ — Tolerant equality is not transitive: x=y and y=z need not imply x=z
(1=1+8E¯15⋄ (1+8E¯15)=1+16E¯15⋄ 1=1+16E¯15)   ⍝ 1ₓ 1ₓ 0ₓ

⍝⍝ Errors and evaluation order

⍝ — Output produced before an error is retained
⎕←7 ⋄ 1÷0
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
⍳1000001
⍝ error: LIMIT ERROR

⍝ —
missing
⍝ error: VALUE ERROR

⍝ — An arbitrary dfn has no declared empty-reduction identity
{⍺-⍵}/⍳0
⍝ error: DOMAIN ERROR

⍝ —
1÷0
⍝ error: DOMAIN ERROR

⍝ —
÷0
⍝ error: DOMAIN ERROR

⍝ —
1e
⍝ error: SYNTAX ERROR

⍝ — A negative exponent uses high minus, not the subtraction glyph
1e-2
⍝ error: SYNTAX ERROR

⍝ —
¯
⍝ error: SYNTAX ERROR

⍝ —
.
⍝ error: SYNTAX ERROR

⍝ —
2+
⍝ error: SYNTAX ERROR

⍝ —
)
⍝ error: SYNTAX ERROR

⍝ —
(2+3
⍝ error: SYNTAX ERROR

⍝ —
1 2+3 4 5
⍝ error: LENGTH ERROR

⍝ —
⍳¯1
⍝ error: DOMAIN ERROR

⍝ —
2+2   ⍝ 4.0

⍝⍝ Hybrid categories and singleton replicate

⍝ — A right-operand reference classifies a definition as a dyadic operator
op←{⍹ ⍵} ⋄ (+op-)3   ⍝ ¯3

⍝ — Operand references in a nested definition do not classify the outer dfn
outer←{inner←{⍹ ⍵} ⋄ 1} ⋄ outer 0   ⍝ 1

⍝ — Dyalog 20: operators acquire their function operand before binding the next operator.
+⍨¨/1 2 3   ⍝ 6

⍝ — Commute changes subtraction's scan recurrence to next minus accumulator
-⍨\1 2 3   ⍝ 1 1 2

⍝ — Named operators bind in the same order as their glyphs
each←¨ ⋄ fold←/ ⋄ +⍨ each fold 1 2 3   ⍝ 6

⍝ — Each and reduction bind to self-reshape, producing repeated-dimensional cells
⍴⍨¨/3/⊂⍳4   ⍝ (,1⋄ 2 2⍴2⋄ 3 3 3⍴3⋄ 4 4 4 4⍴4)

⍝⍝ Singleton agreement and empty counts

⍝ — Shared Dyalog cases; additional leading/unit-axis broadcasting is tested separately.
(,2)+3 4   ⍝ 5 6

⍝ —
3 4+,2   ⍝ 5 6

⍝ — Scalar plus singleton preserves the vector shape
2+,3   ⍝ ,5

⍝ — A singleton count extends over an empty argument without creating items
(,2)/⍳0   ⍝ ⍬

⍝ — Empty counts replicate a singleton into an empty result
(⍳0)/,3   ⍝ ⍬

⍝ —
(,2)+⍳0   ⍝ ⍬

⍝ — A large valid count is harmless when the result contains no elements
1000001/⍳0   ⍝ ⍬

⍝ — Replicate validates count integrality even with an empty right argument
0.5/⍳0
⍝ error: DOMAIN ERROR

⍝ —
2 3/1 2 3
⍝ error: LENGTH ERROR

⍝ —
1000001/1
⍝ error: LIMIT ERROR

⍝ —
99999999999999999999ₓ/1
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

⍝ —
×∞ ¯∞   ⍝ 1 ¯1

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

⍝ — Maximum against negative infinity preserves an exact fraction
¯∞⌈1r3   ⍝ 1r3

⍝ —
∞=∞ ¯∞ 1   ⍝ 1ₓ 0ₓ 0ₓ

⍝ — Infinity exceeds any finite exact integer without converting that integer to float
∞>10ₓ*1000ₓ   ⍝ 1ₓ

⍝ — Reversing comparison arguments retains the finite-versus-infinite distinction
(10ₓ*1000ₓ)<∞   ⍝ 1ₓ

⍝ —
∞=1j2   ⍝ 0ₓ

⍝ —
∞∊1 2 ∞   ⍝ 1ₓ

⍝ —
∞ ¯∞⍳¯∞ ∞ 0   ⍝ 2ₓ 1ₓ 3ₓ

⍝ — Grade orders a huge finite integer between the two infinities
⍋∞ (10ₓ*1000ₓ) ¯∞   ⍝ 3ₓ 2ₓ 1ₓ

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

⍝ — Extract real part, magnitude and imaginary part of negative real infinity
9 10 11○¯∞   ⍝ ¯∞ ∞ 0

⍝ — Principal Lambert W has limit infinity
×∘*⍨⍣¯1⊢∞   ⍝ ∞

⍝ —
0÷0   ⍝ 1

⍝ — Indeterminate real arithmetic errors instead of producing NaN
∞-∞
⍝ error: DOMAIN ERROR

⍝ —
0×∞
⍝ error: DOMAIN ERROR

⍝ —
∞×0ₓ
⍝ error: DOMAIN ERROR

⍝ —
∞÷∞
⍝ error: DOMAIN ERROR

⍝ —
1÷0
⍝ error: DOMAIN ERROR

⍝ —
÷0
⍝ error: DOMAIN ERROR

⍝ — Complex infinities are outside the numeric domain
∞+1j2
⍝ error: DOMAIN ERROR

⍝ —
1j∞
⍝ error: DOMAIN ERROR

⍝ —
∞j0
⍝ error: DOMAIN ERROR

⍝ —
⍳∞
⍝ error: DOMAIN ERROR

⍝ —
∞⍴1
⍝ error: DOMAIN ERROR

⍝ —
∞/1
⍝ error: DOMAIN ERROR

⍝ —
?∞
⍝ error: DOMAIN ERROR

⍝ —
1○∞
⍝ error: DOMAIN ERROR

⍝ — Matrix factorization requires finite entries
⌹1 1⍴∞
⍝ error: DOMAIN ERROR

⍝ —
+/∞ ¯∞
⍝ error: DOMAIN ERROR

⍝ —
×/0 ∞
⍝ error: DOMAIN ERROR

⍝ —
∞∧2
⍝ error: DOMAIN ERROR

⍝ —
!¯1
⍝ error: DOMAIN ERROR

⍝ —
!¯2ₓ
⍝ error: DOMAIN ERROR

⍝⍝ Lexical frames recursion and guard rollback

⍝ — A false empty guard falls through. Dyalog 20.0.53963.0, IO=1, CT=1E¯14, DIV=0.
{⍵=0: ⋄ 3}1   ⍝ 3

⍝ — A no-result guard cannot supply an argument to addition
1+{⍵=0: ⋄ 3}0
⍝ error: VALUE ERROR

⍝ — Name lookup follows lexical nesting, not the caller's local bindings
outer←{x←10 ⋄ read←{x} ⋄ caller←{x←99 ⋄ read ⍵} ⋄ caller 0} ⋄ outer 0
10.0

⍝ — A derived function sees later changes to its active lexical binding
outer←{x←2 ⋄ f←{x+⍵} ⋄ apply←{⍶ ⍵} ⋄ g←f apply ⋄ x←3 ⋄ g 4} ⋄ outer 0
7.0

⍝ — A local defined operator accepts an array operand
outer←{offset←{⍶+⍵} ⋄ (2 offset)3} ⋄ outer 0   ⍝ 5.0

⍝ — An array operand captures its value at derivation, not its name
offset←{⍶+⍵} ⋄ a←2 ⋄ kept←a offset ⋄ a←9 ⋄ kept 3   ⍝ 5.0

⍝ — Recursion through del uses the current dfn
fact←{⍵=0:1 ⋄ ⍵×∇⍵-1} ⋄ fact 6   ⍝ 720.0

⍝ — Mutually recursive local functions resolve definitions introduced later
outer←{even←{⍵=0:1 ⋄ odd ⍵-1} ⋄ odd←{⍵=0:0 ⋄ even ⍵-1} ⋄ even ⍵} ⋄ outer 8
1.0

⍝ — A callee's error reaches the caller's guard, which restores the caller's local binding
bad←{1÷⍵} ⋄ guarded←{x←10 ⋄ 0::x ⋄ x←20 ⋄ bad ⍵} ⋄ guarded 0   ⍝ 10.0

⍝ — Guard rollback removes a newly introduced local, revealing the outer name
temp←9 ⋄ guarded←{0::temp ⋄ temp←20 ⋄ 1÷⍵} ⋄ guarded 0   ⍝ 9.0

⍝ — The checkpoint is taken after evaluating the guard's error-code expression
guarded←{x←10 ⋄ (0×(x←20))::x ⋄ x←30 ⋄ 1÷⍵} ⋄ guarded 0   ⍝ 20.0

⍝ — A failing handler is inactive while it runs, allowing the earlier guard to catch it
guarded←{0::7 ⋄ 0::1÷0 ⋄ 1÷⍵} ⋄ guarded 0   ⍝ 7.0

⍝ — A true guard with an empty body returns no result
{⍵=0: ⋄ 3}0   ⍝ {}0

⍝ — An empty error handler catches the error and returns no result
{0:: ⋄ 1÷0}0   ⍝ {}0

⍝ — A final empty guard also returns no result
{⍵=0:}0   ⍝ {}0

⍝ — Output is not rolled back by a guard.
guarded←{0::7 ⋄ ⎕←2 ⋄ 1÷⍵} ⋄ guarded 0
7
⍝ ⎕: 2

⍝⍝ Enclose

⍝ — Enclosing a vector produces rank zero
≢⍴⊂1 2 3   ⍝ 0ₓ

⍝ — Disclosing recovers the vector
↑⊂1 2 3   ⍝ 1 2 3

⍝⍝ Binder limits and single execution

⍝ — Groups execute once, right to left; the left read sees the right assignment
x←1 ⋄ (⎕←x)+(⎕←(x←2))
4
⍝ ⎕: 2\n2

⍝ — Reduction calls its derived operand exactly once per step
apply←{⍶ ⍵} ⋄ f←{⎕←⍵ ⋄ ⍵} ⋄ (f apply)/1 2 3
3
⍝ ⎕: 3\n3

⍝ — A whole numeric strand binds as one array operand
offset←{+/⍶+⍵} ⋄ (1 2 offset)3   ⍝ 9

⍝⍝ Scalar apl

⍝ — Reduction remains right-associated
-/1 2 3   ⍝ 2

⍝ — Empty sum uses the additive identity
+/⍳0   ⍝ 0

⍝ — Empty product uses the multiplicative identity
×/⍳0   ⍝ 1

⍝ — Reduction of a scalar preserves it
+/7   ⍝ 7

⍝ —
1 2 3+10   ⍝ 11 12 13

⍝ —
10-1 2 3   ⍝ 9 8 7

⍝ —
1 2+3 4   ⍝ 4 6

⍝ — Tally of a singleton vector is exact
≢,7   ⍝ 1ₓ

⍝ — A scalar has an empty shape vector
⍴7   ⍝ 0⍴0ₓ

⍝ — Scalar extension over an empty vector remains empty
2+⍳0   ⍝ ⍬

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
×¯9   ⍝ ¯1.0

⍝ —
×0   ⍝ 0.0

⍝ —
×3   ⍝ 1.0

⍝ —
÷4   ⍝ 0.25

⍝ —
1.5+.5   ⍝ 2.0

⍝ —
¯.5×2   ⍝ ¯1.0

⍝ —
1E¯2+2e¯2   ⍝ 0.03

⍝ —
(+)3   ⍝ 3.0

⍝ —
2(-)3   ⍝ ¯1.0

⍝ — A parenthesis inside a comment does not affect grouping
(2×3) + 4 ⍝ comment )
10.0

⍝ —
0÷0   ⍝ 1.0

⍝ —
0÷¯0   ⍝ 1.0

⍝ —
¯0   ⍝ 0.0

⍝ — Comments alone produce no result
⍝ only a comment
{}0

⍝ — Empty input produces no result
   ⍝ {}0

⍝ — Display normalizes negative zero
⎕←¯0
0
⍝ ⎕: 0

⍝ —
⎕←¯2
¯2
⍝ ⎕: ¯2

⍝⍝ Executing binding gate

⍝ — Reduction calls a named dfn through the ordinary function path
add←{⍺+⍵} ⋄ add/1 2 3   ⍝ 6

⍝ — A named fork computes sum divided by tally
avg←+/÷≢ ⋄ avg 2 4 9   ⍝ 5

⍝ — A defined operator invokes its function operand
apply←{⍶ ⍵} ⋄ (-apply)3   ⍝ ¯3

⍝ — A defined operator may instead use an array operand
offset←{⍶+⍵} ⋄ (2 offset)3   ⍝ 5

⍝ — Names in a dfn resolve at call time, permitting a later definition
inc←{later ⍵} ⋄ later←{1+⍵} ⋄ inc 4   ⍝ 5

⍝ — A caller's local name does not shadow the callee's global binding
x←10 ⋄ read←{x} ⋄ caller←{x←99 ⋄ read ⍵} ⋄ caller 0   ⍝ 10

⍝ — A named reduction retains its dfn operand
add←{⍺+⍵} ⋄ total←add/ ⋄ total 1 2 3   ⍝ 6

⍝ —
{2×⍵}3   ⍝ 6

⍝ —
1 0 1/2 4 6   ⍝ 2 6

⍝ — A named hybrid can act as replicate
rep←/ ⋄ 1 0 1 rep 2 4 6   ⍝ 2 6

⍝ — Fork evaluates the right arm before the left arm
f←{⎕←1 ⋄ ⍵} ⋄ h←{⎕←2 ⋄ ⍵} ⋄ t←f+h ⋄ t 3
6
⍝ ⎕: 2\n1

⍝ — A nested dfn can be defined and called within the active outer frame
outer←{inner←{⍵} ⋄ inner ⍵} ⋄ outer 1   ⍝ 1

⍝⍝ Dyalog array literals and completeness

⍝ — A trailing separator makes a one-element vector literal
(42 ⋄)   ⍝ ,42

⍝ — A leading separator also makes a one-element vector literal
(⋄42)   ⍝ ,42

⍝ — Repeated separators do not insert empty items
(1 ⋄ ⋄ 2)   ⍝ 1 2

⍝ — Newlines inside parentheses act as array-literal separators
(
42
)
⍝ =>
,42

⍝ — Matrix literals pad shorter rows with fill
[1 2 ⋄ 3]   ⍝ 2 2⍴1 2 3 0

⍝ — A one-cell matrix literal retains rank two
[42 ⋄]   ⍝ 1 1⍴42

⍝ — Scalar rows assemble as a one-column matrix
[1 ⋄ 2]   ⍝ 2 1⍴1 2

⍝ — Dfn statements and comments do not split the enclosing literal's rows
[
{a←⍵+1 ⋄ a}2 3 ⍝ first row
5
]
⍝ =>
2 2⍴3 4 5 0

⍝ — Literal elements evaluate left to right and may refer to earlier assignments
(a←2 ⋄ a+3)   ⍝ 2 5

⍝ — Parenthesized literals retain vector elements as nested items
(1 2 ⋄ 3 4)   ⍝ (1 2)(3 4)

⍝ — Explicit output exposes literal elements' left-to-right evaluation order
(⎕←1 ⋄ ⎕←2)
1 2
⍝ ⎕: 1\n2

⍝⍝ Based values, mapping and selection

⍝ —
⍬⍉3   ⍝ 3

⍝ —
[3]+4   ⍝ ⊂7

⍝ —
≢1 2 3   ⍝ 3ₓ

⍝ —
≢¨(1 2 3⋄ 4 5)   ⍝ 3ₓ 2ₓ

⍝ —
+/1 2 3   ⍝ 6

⍝ —
{⍺+⍵}/1 2 3   ⍝ 6

⍝ — A one-item reduction returns the item without calling its operand
f←{1÷0} ⋄ (f/3⋄ f/⊂3⋄ f/,3⋄ f⌿⊂⊂3⋄ f/⊂2 3)   ⍝ 3 3 3 (⊂3) (2 3)

⍝ —
1 2 3+.×4 5 6   ⍝ 32

⍝ —
{⍵}⍤0⊢1 2 3   ⍝ 1 2 3

⍝ —
+⌿2 3⍴⍳6   ⍝ 5 7 9

⍝ —
+/0⍴(1 2⋄ 3 4)   ⍝ 0 0

⍝ —
↑2 3⍴⍳6   ⍝ 1 2 3

⍝ —
2⊃2 3⍴⍳6   ⍝ 4 5 6

⍝ —
2 3⊃2 3⍴⍳6   ⍝ 6

⍝ —
3⊃2⊃(10 20⋄ 30 40 50)   ⍝ 50

⍝ —
(1 3)˘(2 4)⊃3 4⍴⍳12   ⍝ 2 12

⍝ —
⊃(1 2⋄ 3 4 5)   ⍝ 2 3⍴1 2 0 3 4 5

⍝ —
↑⍬   ⍝ 0

⍝ —
↑0 3⍴0   ⍝ 0 0 0

⍝ —
a←1 2 ⋄ (↑a)←3 4 ⋄ a   ⍝ (3 4)2

⍝ —
a←2 3⍴⍳6 ⋄ (2⊃a)←7 8 9 ⋄ a   ⍝ [1 2 3 ⋄ 7 8 9]

⍝ —
a←2 3⍴⍳6 ⋄ (2 3⊃a)←9 ⋄ a   ⍝ [1 2 3 ⋄ 4 5 9]

⍝ —
a←(1 2⋄ 3 4) ⋄ (2⊃1⊃a)←9 ⋄ a   ⍝ (1 9⋄ 3 4)

⍝ —
a←3 ⋄ a.(⍬)←4 ⋄ a   ⍝ 4

⍝ —
v←3 4 ⋄ (v.(1)*2)+v.(2)*2   ⍝ 25

⍝ —
(1 3⋄ 2 4)⍳⊂1 3   ⍝ 1ₓ

⍝ —
v←10 20 30 ⋄ (2⌷v⋄ [⊂2]⌷v⋄ [,2]⌷v)   ⍝ (20⋄ ⊂20⋄ ,20)

⍝ —
a←(1 2⋄ 3 4) ⋄ a.(2).(1)←9 ⋄ a   ⍝ (1 2⋄ 9 4)

⍝ —
a←(1 2⋄ 3 4) ⋄ a.(2),←5 ⋄ a   ⍝ (1 2⋄ 3 4 5)

⍝ —
a←(1 2⋄ 3 4) ⋄ a.(2)←5 6 7 ⋄ a   ⍝ (1 2⋄ 5 6 7)

⍝ —
fs←+˘× ⋄ fs.(2)←- ⋄ f←2⌷fs ⋄ 3 f 2   ⍝ 1

⍝ —
1 3⍳2   ⍝ 3ₓ

⍝ —
1 3⍳⊂2   ⍝ 3ₓ

⍝ —
1 3⍸2   ⍝ 1ₓ

⍝ —
1 3⍸⊂2   ⍝ 1ₓ

⍝ —
2∊1 2 3   ⍝ 1ₓ

⍝ —
[2]∊1 2 3   ⍝ 1ₓ

⍝ —
(1 3 (1=⍸) 0 ⋄ 1 3 (1=⍸) 1 ⋄ 1 3 (1=⍸) 2 ⋄ 1 3 (1=⍸) 3 ⋄ 1 3 (1=⍸) 4)
0ₓ 1ₓ 1ₓ 0ₓ 0ₓ

⍝ —
{⍵∊1 3}¨⍳4   ⍝ 1ₓ 0ₓ 1ₓ 0ₓ

⍝ —
(2 2⍴⍳4)⍳3 4   ⍝ 2ₓ

⍝ —
1 3⍳⍬   ⍝ 0⍴0ₓ

⍝ —
(⌽2⋄ ⍉⊂2⋄ ⍬↑2⋄ ⍬↓⊂2)   ⍝ 2 (⊂2) 2 (⊂2)

⍝ —
2⍷2   ⍝ 1ₓ

⍝⍝ Seeded reduction

⍝ — A reduction seed is the final right argument
10 -/1 2 3   ⍝ ¯8

⍝ — Each matrix lane starts with the same reduction seed
10 +/2 3⍴⍳6   ⍝ 16 25

⍝ — Rank pairs each row with its own seed
10 20 (+/⍤0 1)2 3⍴⍳6   ⍝ 16 35

⍝ — Empty reduction returns the whole seed without calling the operand
(1 2⋄ 3 4) {1÷0}/⍬   ⍝ (1 2⋄ 3 4)

⍝ — Seeded reduction follows a nested path
tree←(10 20⋄ 30 (40 50)) ⋄ tree⊃/⌽2 2 1   ⍝ 40

⍝ — A scalar seed remains distinct from an atom
[10]+/1 2 3   ⍝ ⊂16

⍝ — Whole-seed scan follows a path and retains its intermediate arrays
(10 20⋄ 30 40) ⊃⍨\1 2   ⍝ (10 20) 20

⍝⍝ Axis-key dictionaries and assignment

⍝ axis-dictionary — One string names a whole array; atomic selection returns that stored array
T←"price" "qty":(1 2 3 ⋄ 4 5 6)
(≢T ⋄ ⍴T ⋄ 1⊃T ⋄ "qty"⌷T ⋄ ⍴"qty"⌷T ⋄ ⍳⍤[1]T)
⍝ =>
(2ₓ ⋄ ,2ₓ ⋄ 1 2 3 ⋄ 4 5 6 ⋄ ,3ₓ ⋄ "price" "qty")

⍝ axis-function-value — Stored functions retain ordinary dfn guards
("sign"⊃"sign":{⍵<0:¯1 ⋄ 1})¨¯2 3   ⍝ ¯1 1

⍝ axis-empty — Empty labelled axes retain their key lists
T←⍬:⍬ ⋄ (≢T ⋄ ⍴⍳⍤[1]T ⋄ T≡⍬ ⋄ T≡T ⋄ 2⍴T)
(0ₓ ⋄ ,0ₓ ⋄ 0ₓ ⋄ 1ₓ ⋄ 0 0)

⍝ axis-duplicate — Labels are unique within an axis
"aa" "aa":1 2
⍝ error: DOMAIN ERROR

⍝ axis-length — Key lists match axis lengths
"aa" "bb" "cc":2 3⍴⍳6
⍝ error: LENGTH ERROR

⍝ axis-key-type — Keys are strings, or a position's own number to leave that position unnamed
("aa" 2:3 4 ⋄ "b":5)   ⍝ ("aa":3 ⋄ 4 ⋄ "b":5)

⍝ —
"aa" 2:3 4   ⍝ ("aa":3 ⋄ 4)

⍝ —
1 2:3 4   ⍝ 3 4

⍝ —
2 1:3 4
⍝ error: DOMAIN ERROR

⍝ axis-pick-batch — An array-valued coordinate field may repeat extracted values
T←"price" "qty":(1 2 3 ⋄ 4 5 6)
(["qty" "price"]⊃T ⋄ ["price" "price"]⊃T)
⍝ =>
((4 5 6 ⋄ 1 2 3) ⋄ (1 2 3 ⋄ 1 2 3))

⍝ axis-nested — Nested lookups use axis-local names
T←("name":"Ann"),("addr":"city":"Paris"),("items":(("name":"pen") ⋄ ("name":"ink")))
("city"⊃"addr"⊃T ⋄ "name"⊃2⊃"items"⊃T ⋄ T.addr.city)
⍝ =>
("Paris" ⋄ "ink" ⋄ "Paris")

⍝ axis-selection-repeat — Keyed selection cannot repeat a named position
["aa" "aa"]⌷"aa" "bb":1 2
⍝ error: DOMAIN ERROR

⍝ axis-missing — Reading a missing key errors
"missing"⌷"aa":1
⍝ error: INDEX ERROR

⍝ axis-assignment-alignment — Keyed RHS aligns to selection and ignores extra keys
T←"price" "qty":(1 2 3 ⋄ 4 5 6)
T.["price" "qty"]←"qty" "extra" "price":7 9 8
T
⍝ =>
"price" "qty":8 7

⍝ axis-assignment-position — Unkeyed RHS assigns by position
T←"price" "qty":(1 2 3 ⋄ 4 5 6) ⋄ T.["qty" "price"]←(10 20 ⋄ 30) ⋄ T
"price" "qty":(30 ⋄ 10 20)

⍝ axis-assignment-scalar — Scalar selection replaces a value; vector selection aligns its retained axis
T←"aa" "bb":1 2 ⋄ T.("aa")←"bb" "aa":8 9 ⋄ T.("bb")+←10 ⋄ T
"aa" "bb":(("bb" "aa":8 9) ⋄ 12)

⍝ axis-assignment-singleton — A one-position vector selection aligns the RHS by key
T←"aa" "bb":1 2 ⋄ T.[,⊂"aa"]←"bb" "aa":8 9 ⋄ T   ⍝ "aa" "bb":9 2

⍝ axis-assignment-missing — Every selected key must occur in a keyed replacement
T←"aa" "bb":1 2 ⋄ T.["aa" "bb"]←"aa":5
⍝ error: INDEX ERROR

⍝ axis-assignment-repeat — Assignment cannot select a labelled position twice
T←"aa" "bb":1 2 ⋄ T.[1 1]←5 6
⍝ error: DOMAIN ERROR

⍝ axis-nested-write — Pick and dot replace stored values without changing other copies
T←("n":1),("addr":"city":"Paris")
("n"⊃T)←5 ⋄ T.n+←1 ⋄ ("city"⊃"addr"⊃T)←"Rome"
U←T ⋄ U.addr.city←"Oslo" ⋄ (T ⋄ U.addr.city)
⍝ =>
((("n":6),("addr":"city":"Rome")) ⋄ "Oslo")

⍝ axis-nested-index-write — Dot indexing after a dotted value updates inside it
T←"v":1 2 3 ⋄ T.v.(2)←9 ⋄ T.v.(3)+←1 ⋄ T   ⍝ "v":1 9 4

⍝ axis-dot-insert — Plain dot and Pick updates append to keyed vectors
T←⍬:⍬ ⋄ T.a←1 ⋄ ("b"⊃T)←"c":2 ⋄ T.b.d←3 ⋄ T
("a":1),("b":("c":2),("d":3))

⍝ axis-modified-missing — Modified assignment needs an existing key
T←"a":1 ⋄ T.b+←2
⍝ error: INDEX ERROR

⍝ axis-path-create — Plain assignment through a dot path creates missing records
T←("n":1)
T.style.color←"red" ⋄ T.style.width←2 ⋄ T.a.b.c←3 ⋄ T
⍝ =>
("n":1 ⋄ "style":("color":"red" ⋄ "width":2) ⋄ "a":("b":("c":3)))

⍝ axis-path-nonrecord — A path cannot descend into a value that is not a record
T←"a":1 ⋄ T.a.y←2
⍝ error: RANK ERROR

⍝ axis-bracket-insert — New names append in selector order, including keyed RHS alignment
T←"aa":1 ⋄ T.("bb")←2 ⋄ T.["cc" "aa" "dd"]←30 10 40
U←⍬:⍬ ⋄ U.["xx" "yy"]←"yy" "xx":2 1 ⋄ (T ⋄ U)
⍝ =>
(("aa" "bb" "cc" "dd":10 2 30 40) ⋄ ("xx" "yy":1 2))

⍝ axis-nested-bracket-insert — Append within a dotted container
T←"addr":"city":"Paris" ⋄ T.addr.("zip")←"75" ⋄ T
"addr":"city" "zip":("Paris" ⋄ "75")

⍝ axis-modified-bracket-missing — Modified assignment to a missing key does not append
T←"aa":1 ⋄ T.("bb")+←2
⍝ error: INDEX ERROR

⍝ axis-numeric-no-insert — Out-of-range numeric coordinates do not append
T←"aa":1 ⋄ T.(2)←5
⍝ error: INDEX ERROR

⍝ axis-new-repeat — A newly appended position cannot be selected twice
T←"aa":1 ⋄ T.["zz" "zz"]←1 2
⍝ error: DOMAIN ERROR

⍝ axis-rename — Reattach edited axis selectors to rename keys
T←"price" "qty":1 2 ⋄ K←⍳⍤[1]T ⋄ K.(1)←"cost" ⋄ K:T
"cost" "qty":1 2

⍝ axis-nested-agreement — Nested keyed values align their own axes
("p":"xx" "yy":1 2)+("p":"yy":10)   ⍝ "p":"xx" "yy":1 12

⍝ axis-nested-broadcast — Enclosure broadcasts within each stored array
T←"price" "qty":(1 2 3 ⋄ 4 5 6)
(T+⊂10 20 30 ⋄ 10×T ⋄ +/T ⋄ +/¨T ⋄ ≢¨T)
⍝ =>
(("price" "qty":(11 22 33 ⋄ 14 25 36)) ⋄ ("price" "qty":(10 20 30 ⋄ 40 50 60)) ⋄ 5 7 9 ⋄ ("price" "qty":6 15) ⋄ ("price" "qty":3ₓ 3ₓ))

⍝ axis-filter — Filtering and ordering retain the selected labels
Q←"aa" "bb" "cc":10 20 5
(1 0 1/Q ⋄ 1⌽Q ⋄ 1↓Q ⋄ ¯1↑Q ⋄ [⍋Q]⌷Q ⋄ [3 1]⌷Q)
⍝ =>
(("aa" "cc":10 5) ⋄ ("bb" "cc" "aa":20 5 10) ⋄ ("bb" "cc":20 5) ⋄ ("cc":5) ⋄ ("cc" "aa" "bb":5 10 20) ⋄ ("cc" "aa":5 10))

⍝ axis-overtake — Padding a keyed axis leaves the new positions unnamed
3↑"aa" "bb":1 2   ⍝ ("aa":1 ⋄ "bb":2 ⋄ 0)

⍝ axis-expand — Expand leaves inserted positions unnamed
1 0 1\("aa" "bb":1 2)   ⍝ ("aa":1 ⋄ 0 ⋄ "bb":2)

⍝ axis-scan — Scan retains labels along its accumulated axis
+\("aa" "bb" "cc":1 2 3)   ⍝ "aa" "bb" "cc":1 3 6

⍝ axis-cat-duplicate — Catenate does not merge duplicate labels
("aa" "bb":1 2),("bb" "cc":20 30)
⍝ error: DOMAIN ERROR

⍝ axis-cat-plain — An unkeyed part of a joined axis leaves its positions unnamed
("aa" "bb":1 2),5   ⍝ ("aa":1 ⋄ "bb":2 ⋄ 5)

⍝ axis-each — Each receives stored values and aligns outer axes
T←"price" "qty":(1 2 3 ⋄ 4 5 6)
(2⌷¨T ⋄ ("aa" "bb":1 2){⍺ ⍵}¨("bb":3))
⍝ =>
(("price" "qty":2 5) ⋄ ("aa" "bb":(1 0 ⋄ 2 3)))

⍝ axis-match-position — Match ignores labelled-axis order; positional access observes it
A←"aa" "bb":1 2 ⋄ B←"bb" "aa":2 1 ⋄ (A≡B ⋄ (1⊃A)≡1⊃B)   ⍝ (1ₓ ⋄ 0ₓ)

⍝ axis-sets — Set operations compare values, retaining selected labels
A←"aa" "bb":1 2 ⋄ B←"aa" "bb" "cc":1 9 3
(A∊B ⋄ ("xx":9)∊B ⋄ A∩B ⋄ A~B ⋄ A∪("cc":3) ⋄ ∪"aa" "bb" "cc":1 1 2)
⍝ =>
(("aa" "bb":1ₓ 0ₓ) ⋄ ("xx":1ₓ) ⋄ ("aa":1) ⋄ ("bb":2) ⋄ ("aa" "bb" "cc":1 2 3) ⋄ ("aa" "cc":1 2))

⍝ axis-search-cells — Frame names return positions; cell labels participate in Match
M←("alice" "bob" ⋄ "price" "qty"):[10 2 ⋄ 20 4]
(M⍳"bob"⌷M ⋄ M⍳⌽"bob"⌷M ⋄ M⍳20 4)
⍝ =>
("bob" ⋄ "bob" ⋄ 3ₓ)

⍝ axis-set-rank — Set functions retain their ordinary rank limits
M←"aa" "bb":2 2⍴⍳4 ⋄ M∪M
⍝ error: RANK ERROR

⍝⍝ Libraries

⍝ — Signed 8-bit integers
•load "lib/numeric.apl"
8 int 0 127 128 255
⍝ =>
0 127 ¯128 ¯1

⍝ — Normal random array shape
•load "lib/numeric.apl"
⍴NormRand 2 3
⍝ =>
2ₓ 3ₓ

⍝ — Phinary decoding
•load "lib/numeric.apl"
1e¯12>|42-phinary "10100010.00100001"
⍝ =>
1ₓ

⍝ — Associative scan along the last axis
•load "lib/array.apl"
+ascana [1 2 3 ⋄ 4 5 6]
⍝ =>
[1 3 6 ⋄ 4 9 15]

⍝ — Unwrap line breaks
•load "lib/string.apl"
unwrap "abc",(•ucs 10),"def"
⍝ =>
"abc def"

⍝ — Parent-first traversal
•load "lib/tree.apl"
t←1(2(,4)(,5))(,3)
⍬{⍺,↑⍵}trav{1↓⍵}t
⍝ =>
1 2 4 5 3

⍝ — Children-first traversal
•load "lib/tree.apl"
t←1(2(,4)(,5))(,3)
⍬{⍺,↑⍵}ravt{1↓⍵}t
⍝ =>
4 5 2 3 1

⍝⍝ CSV

⍝ — Headers key compact, independently inferred columns
nl←•ucs 10 ⋄ •csv "price,qty",nl,"10.5,2",nl,"20.0,4"
("price":10.5 20 ⋄ "qty":2ₓ 4ₓ)

⍝ — Inference is per column; numeric-looking text stays text in a mixed column
nl←•ucs 10 ⋄ •csv "id,note",nl,"001,001",nl,"002,no"
("id":1ₓ 2ₓ ⋄ "note":"001" "no")

⍝ — Forced text preserves identifiers, including quoted numeric fields
src←"id,n", (•ucs 10), """00123"",5"
("text_columns":"id") •csv src
⍝ =>
("id":(,⊂"00123") ⋄ "n":,5ₓ)

⍝ — Empty numeric cells use infinity; entirely empty columns are text
nl←•ucs 10 ⋄ •csv "a,b,c",nl,"1,1.5,",nl,",,"
"a" "b" "c":((1ₓ ∞) ⋄ 1.5 ∞ ⋄ ("" ""))

⍝ — Custom markers and fill, with positional forced numeric selection
nl←•ucs 10 ⋄ src←"a,b",nl,"NA,NA",nl,"3,yes"
("missing":"NA" ⋄ "fill":¯1ₓ ⋄ "numeric_columns":1) •csv src
⍝ =>
"a" "b":(¯1ₓ 3ₓ ⋄ "" "yes")

⍝ — Headerless input and locale-specific numeric spelling
nl←•ucs 10 ⋄ src←"1.234,5;2",nl,"2.000;3"
("header":0 ⋄ "separator":';' ⋄ "decimal":',' ⋄ "thousands":'.') •csv src
⍝ =>
(1234.5 2000 ⋄ 2ₓ 3ₓ)

⍝ — Quoting, doubled quotes, embedded newline and Unicode
nl←•ucs 10 ⋄ •csv "note",nl,"""a,b""",nl,"""say """"hi""""""",nl,"""λ",nl,"😀"""
"note":("a,b" ⋄ "say ""hi""" ⋄ 'λ',(•ucs 10),'😀')

⍝ — Whitespace is preserved unless trimming is requested
nl←•ucs 10 ⋄ src←'n',nl," 2 "
(•csv src) (("trim":1) •csv src)
⍝ =>
("n":,⊂" 2 ") ("n":,2ₓ)

⍝ — Empty input
•csv ""   ⍝ 0⍴⊂""

⍝ — Header-only input retains empty columns
•csv "a,b"   ⍝ "a" "b":((0⍴⊂"") ⋄ 0⍴⊂"")

⍝ — Integers beyond i64 remain exact
•csv 'n',(•ucs 10),"9223372036854775808"
"n":,9223372036854775808ₓ

⍝ — Float promotion must not round large exact integers
•csv 'n',(•ucs 10),"9007199254740993",(•ucs 10),"1.5"
"n":9007199254740993ₓ 1.5

⍝ — Export and import retain numeric domains, strings and headers
T←("price":10.5 20 ⋄ "qty":2ₓ 4ₓ ⋄ "note":"a,b" "say ""hi""")
•csv •tocsv T
⍝ =>
("price":10.5 20 ⋄ "qty":2ₓ 4ₓ ⋄ "note":"a,b" "say ""hi""")

⍝ — Export uses CSV minus signs and no exact suffix
•tocsv ("n":¯2ₓ 3ₓ)   ⍝ 'n',(•ucs 10),"-2",(•ucs 10),'3',•ucs 10

⍝ — Infinity remains a number unless fill is explicitly configured
T←"n":1ₓ ∞ ⋄ (•tocsv T) (("fill":∞) •tocsv T)
('n',(•ucs 10),'1',(•ucs 10),"inf",•ucs 10) ('n',(•ucs 10),'1',(•ucs 10),"""""",•ucs 10)

⍝ — Export dialect and CRLF
T←"n":,1234.5
("separator":';' ⋄ "decimal":',' ⋄ "thousands":'.' ⋄ "lineending":•ucs 13 10) •tocsv T
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
("text_columns":1 ⋄ "numeric_columns":1) •csv "a,b"
⍝ error: DOMAIN ERROR

⍝ — Unknown option
("seperator":';') •csv "a,b"
⍝ error: DOMAIN ERROR

⍝ — Export requires equal-length column vectors
•tocsv "a" "b":(1 2 ⋄ ,3)
⍝ error: LENGTH ERROR

⍝ — Export rejects nested cells
•tocsv ("a":,⊂1 2)
⍝ error: DOMAIN ERROR

⍝ — Quote-free export errors when a field needs quoting
("quotechar":"") •tocsv ("a":,⊂"x,y")
⍝ error: DOMAIN ERROR

⍝ — Escape characters round-trip inside quoted text
T←"note":,⊂"a\b""c"
opts←("escapechar":'\' ⋄ "doublequote":0) ⋄ csv←opts •tocsv T
opts •csv csv
⍝ =>
"note":,⊂"a\b""c"

⍝⍝ JSON

⍝ — JSON objects become keyed vectors; arrays retain nesting and integer exactness
•json "{""name"":""Ann"",""values"":[1,2.5,[3,4]]}"
("name":"Ann" ⋄ "values":(1ₓ ⋄ 2.5 ⋄ 3ₓ 4ₓ))

⍝ — Booleans are exact numbers and null defaults to infinity
•json "[true,false,null]"   ⍝ 1ₓ 0ₓ ∞

⍝ — Explicit null fill works recursively in both directions
fill←("fill":¯1ₓ)
fill •tojson fill •json "{""x"":[null,2]}"
⍝ =>
"{""x"":[null,2]}"

⍝ — Import keeps integers beyond i64 and float syntax distinct
•json "[9223372036854775808,1.0,1e2]"
9223372036854775808ₓ 1 100

⍝ — Large exact integers export without rounding
•tojson 9223372036854775808ₓ   ⍝ "9223372036854775808"

⍝ — Booleans export as numbers
•tojson •json "[true,false]"   ⍝ "[1,0]"

⍝ — Empty objects, arrays and strings retain their distinct meanings
•tojson •json "[{},[],""""]"   ⍝ "[{},[],""""]"

⍝ — Scalar strings remain strings, including one-character strings
•tojson •json "[""a"","""",[""b"",""c""]]"
"[""a"","""",[""b"",""c""]]"

⍝ — Ordinary matrices export as nested JSON arrays
•tojson [1ₓ 2ₓ ⋄ 3ₓ 4ₓ]   ⍝ "[[1,2],[3,4]]"

⍝ — Keyed axes export as object levels
•tojson ("row":"col"  "val":(1ₓ 2ₓ))
"{""row"":{""col"":1,""val"":2}}"

⍝ — Unkeyed scalar arrays export their contents
•tojson ⊂2ₓ   ⍝ "2"

⍝ — Duplicate object members follow the JSON library's last-value rule
•json "{""name"":1,""name"":2}"   ⍝ "name":2ₓ

⍝ — Malformed JSON gives a located error
•json "[1,]"
⍝ error: DOMAIN ERROR

⍝ — Out-of-range float input cannot silently become a missing sentinel
•json "1e999"
⍝ error: DOMAIN ERROR

⍝ — Infinity requires explicit fill on export
•tojson ∞
⍝ error: DOMAIN ERROR

⍝ — Nonintegral rationals have no JSON number representation
•tojson 1r3
⍝ error: DOMAIN ERROR

⍝ — Complex values have no JSON number representation
•tojson 1j2
⍝ error: DOMAIN ERROR

⍝ — Functions have no JSON representation
•tojson +˘-
⍝ error: DOMAIN ERROR

⍝ — LZW repeated-code expansion and capped dictionary
•load "lib/dyalog.apl" ⋄ (0 packZ packZ "aaaaaa" ⋄ 0 packZ 1 packZ "abababab")
("aaaaaa" ⋄ "abababab")

⍝ — LZW dictionary
•load "lib/dyalog.apl" ⋄ ¯3 packZ "aba"
["a " ⋄ "b " ⋄ "ab" ⋄ "ba"]

⍝ — LZW alphabet exceeds code width
•load "lib/dyalog.apl" ⋄ 1 packZ "abc"
⍝ error: DOMAIN ERROR

⍝ — Absolute-year calendar layout
•load "lib/dyalog.apl" ⋄ ⍴cal 2025
33ₓ 66ₓ

⍝ — Unification substitutes repeated variables on either side
•load "lib/dyalog.apl" ⋄ "xy"unify('f' 3 'y')('f' 'x' 'x')
('f' ⋄ 3 ⋄ 3)

⍝ — Unification rejects cyclic substitution
•load "lib/dyalog.apl" ⋄ 'x'unify('x' ⋄ 'f' 'x')
⍝ error: DOMAIN ERROR

⍝ — Unification rejects distinct constants
•load "lib/dyalog.apl" ⋄ 'x'unify('f' 1)('f' 2)
⍝ error: DOMAIN ERROR

⍝ — Unification rejects different term shapes
•load "lib/dyalog.apl" ⋄ 'x'unify('f' 1)('f' 1 2)
⍝ error: LENGTH ERROR

⍝ — Repeating rational carry and fractional normalization
•load "lib/dyalog.apl" ⋄ rs←"0123456789"ratsum ⋄ ("<0|0|3>"rs"<0|0|6>" ⋄ "<0|0.5|0>"rs"<0|0.5|0>")
("<0|1|0>" ⋄ "<0|1|0>")

⍝ — Repeating rational negation and binary carry
•load "lib/dyalog.apl" ⋄ (("0123456789"ratsum)"<0|1|0>" ⋄ "<0|1|0>"("01"ratsum)"<0|1|0>")
("<9|9|0>" ⋄ "<0|10|0>")

⍝ — Repeating rational invalid input
•load "lib/dyalog.apl" ⋄ "<0|1|0>"("0123456789"ratsum)"<0||0>"
⍝ error: DOMAIN ERROR

⍝ — Append a field through a record selected by dot indexing
rows←("aa":1)("bb":2) ⋄ rows.(2).cc←3 ⋄ rows
("aa":1)(("bb":2),("cc":3))

⍝⍝ Name inspection

⍝ — Name classes follow lexical bindings, including hybrids and operators
x←7 ⋄ f←+ ⋄ op←¨ ⋄ hybrid←/ ⋄ •nc "x" "f" "op" "hybrid" "absent" "bad name"
2ₓ 3ₓ 4ₓ 3ₓ 0ₓ ¯1ₓ

⍝ — Name lists are sorted, filtered by prefix and class
zeta←1 ⋄ mean←{+/⍵} ⋄ member←+ ⋄ "me" •nl 3
"mean" "member"

⍝ — The nearest binding determines the visible class
x←1 ⋄ f←{x←+ ⋄ local←2 ⋄ (•nc "x" ⋄ •nl 2)} ⋄ f 0
(3ₓ ⋄ ,⊂"local")

⍝ — Expunging a local reveals its outer binding; handles retain definitions
x←1 ⋄ f←{x←2 ⋄ erased←•ex "x" ⋄ x} ⋄ kept←f ⋄ •ex "f" ⋄ kept 0
1

⍝ — Erasure is idempotent and protects implicit/system names
x←1 ⋄ •ex "x" "x" "bad name" "•a" "⍵"
1ₓ 1ₓ 0ₓ 0ₓ 0ₓ

⍝ — Source retains the definition, while derived functions use APL display
f←{⍵+1} ⋄ plus←+ ⋄ •src "f" "plus"
("{⍵+1}" ⋄ "+")

⍝ — Batch inspection retains its frame and empty result domain
(⍴•nc [ "aa" "bb" ⋄ "cc" "dd"] ⋄ •nc 0⍴⊂"" ⋄ •src 0⍴⊂"")
(2ₓ 2ₓ ⋄ (0⍴0ₓ) ⋄ 0⍴⊂"")

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
(1ₓ 0ₓ 1ₓ 1ₓ 0ₓ ⋄ 12 0 ¯3 1.5 0)

⍝ — Preserve numeric domains, signed exponents, fractions and complex components
•vfi "+2 3x -4x 1r3 -2r-3 1e-2 1j-2 ∞ -∞"
((9⍴1ₓ) ⋄ 2 3ₓ ¯4ₓ 1r3 2r3 0.01 1j¯2 ∞ ¯∞)

⍝ — Invalid fractions, malformed numbers and code remain invalid fields
•vfi "1r0 1x2 . NaN ⎕←7"
((5⍴0ₓ) ⋄ 5⍴0)

⍝ — Explicit separators retain empty fields as valid zero
",;" •vfi ",3.9;2.4,,76,"
((6⍴1ₓ) ⋄ 0 3.9 2.4 0 76 0)

⍝ — Explicit separators trim whitespace but do not split on it
'⋄' •vfi "1 ⋄ 2 3 ⋄ 4 "
(1ₓ 0ₓ 1ₓ ⋄ 1 0 4)

⍝ — Default whitespace splitting and empty result domains
(•vfi ' ' ⋄ •vfi "" ⋄ •vfi '2',(•ucs 9 10),'3')
(((0⍴0ₓ) ⋄ 0⍴0) ⋄ ((0⍴0ₓ) ⋄ 0⍴0) ⋄ (1ₓ 1ₓ ⋄ 2 3))

⍝ — Empty separator list treats its input as one field
"" •vfi "1 2"
((,0ₓ) ⋄ ,0)

⍝ — Numeric arrays are not text
•vfi 1 2
⍝ error: DOMAIN ERROR
