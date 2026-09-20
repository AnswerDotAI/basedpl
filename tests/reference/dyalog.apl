⍝ dyalog:magnitude:1 —
|2 ¯3.4 0 ¯2.7   ⍝ 2 3.4 0 2.7

⍝ dyalog:power:inverse-scan —
(+\⍣¯1)1 3 6 10   ⍝ 1 2 3 4

⍝ dyalog:power:inverse-decode —
(2∘⊥⍣¯1)9   ⍝ 1 0 0 1

⍝ dyalog:power:inverse-mixed-base —
(0 60 60∘⊥⍣¯1)3661   ⍝ 1 1 1

⍝ dyalog:axes:laminate —
1 2,[0.5]3 4   ⍝ 2 2⍴1 2 3 4

⍝ dyalog:axes:mix —
⊃[0.5](1 2⋄ 3 4)   ⍝ 2 2⍴1 3 2 4

⍝ dyalog:assignment:indexed-modified —
B←3 5⍴0 ⋄ B[1 1 3;1 3 3 5]+←1 ⋄ B   ⍝ 3 5⍴2 0 4 0 2 0 0 0 0 0 1 0 2 0 1

⍝ dyalog:assignment:selective —
A←'HELLO' ⋄ ((A∊'AEIOU')/A)←'*' ⋄ A   ⍝ 'H*LL*'

⍝ dyalog:assignment:selective-modified —
a←3⍴0 ⋄ (5⍴a)+←1 ⋄ a   ⍝ 2 2 1

⍝ dyalog:magnitude:2 —
|3J4   ⍝ 5

⍝ dyalog:residue:1 —
3 3 ¯3 ¯3|¯5 5 ¯4 4   ⍝ 1 2 ¯1 ¯2

⍝ dyalog:residue:2 —
0.5|3.12 ¯1 ¯0.6   ⍝ 0.1200000000000001 0 0.4

⍝ dyalog:residue:3 —
1J2|2J3 3J4 5J6   ⍝ 1j1 ¯1j1 0j1

⍝ dyalog:floor:1 —
⌊¯2.3 0.1 100 3.3   ⍝ ¯3 0 100 3

⍝ dyalog:floor:2 —
⌊1J3.2 3.3J2.5 ¯3.3J¯2.5   ⍝ 1j3 3j2 ¯3j¯3

⍝ dyalog:ceiling:1 —
⌈¯2.3 0.1 100 3.3   ⍝ ¯2 1 100 4

⍝ dyalog:ceiling:2 —
⌈1.2J2.5 1.2J¯2.5   ⍝ 1j3 1j¯2

⍝ dyalog:minimum:1 —
¯2.1 0.1 15.3⌊¯3.2 1 22   ⍝ ¯3.2 0.1 15.3

⍝ dyalog:maximum:1 —
¯2.01 0.1 15.3⌈¯3.2 ¯1.1 22.7   ⍝ ¯2.01 0.1 22.7

⍝ dyalog:exponential:1 —
*1 0   ⍝ 2.718281828459045 1

⍝ dyalog:exponential:2 —
*0J1 1J2
0.5403023058681398j0.8414709848078965 ¯1.131204383756814j2.471726672004819

⍝ dyalog:power:1 —
2*2 ¯2   ⍝ 4 0.25

⍝ dyalog:power:2 —
9 64*0.5   ⍝ 3 8

⍝ dyalog:natural-logarithm:1 —
⍟1 2   ⍝ 0 0.6931471805599453

⍝ dyalog:natural-logarithm:2 —
⍟2 2⍴0J1 1J2 2J3 3J4
2 2⍴0j1.570796326794897 0.8047189562170503j1.10714871779409 1.282474678730768j0.982793723247329 1.6094379124341j0.9272952180016122

⍝ dyalog:logarithm:1 —
10⍟100 2   ⍝ 2 0.3010299956639811

⍝ dyalog:logarithm:2 —
2 10⍟0J1 1J2
0j2.266180070913597 0.3494850021680094j0.480828578784234

⍝ dyalog:logarithm:3 —
1⍟1   ⍝ 1

⍝ dyalog:pi-times:1 —
π0.5 1 2
1.570796326794897 3.141592653589793 6.283185307179586

⍝ dyalog:circular-functions:1 —
0 ¯1○1   ⍝ 0 1.570796326794897

⍝ dyalog:circular-functions:2 —
9 11○3.5J¯1.2   ⍝ 3.5 ¯1.2

⍝ dyalog:circular-functions:3 —
¯4○¯1   ⍝ 0

⍝ dyalog:factorial:1 —
!1 2 3 4 5   ⍝ 1 2 6 24 120

⍝ dyalog:factorial:2 —
!¯1.5 0 1.5 3.3
¯3.544907701811032 1 1.329340388179137 8.855343360454034

⍝ dyalog:factorial:3 —
!0J1 1J2
0.498015668118356j¯0.1549498283018108 0.1122942423463263j0.3236128855019272

⍝ dyalog:binomial:1 —
1 1.2 1.4 1.6 1.8 2!5
5 6.105689247785079 7.21942468559846 8.281104786421748 9.227916704038812 10

⍝ dyalog:binomial:2 —
2!3J2   ⍝ 1j5

⍝ dyalog:greatest-common-divisor-or:1 —
15 1 2 7∨35 1 4 0   ⍝ 5 1 2 7

⍝ dyalog:lowest-common-multiple-and:1 —
15 1 2 7∧35 1 4 0   ⍝ 105 1 4 0

⍝ dyalog:lowest-common-multiple-and:2 —
2 3 4∧0J1 1J2 2J3   ⍝ 0j2 3j6 8j12

⍝ dyalog:lowest-common-multiple-and:3 —
2J2 2J4∧5J5 4J4   ⍝ 10j10 ¯4j12

⍝ dyalog:not:1 —
~0 1   ⍝ 1 0

⍝ dyalog:nand:1 —
(0 1⋄ 1 0)⍲(0 0⋄ 1 1)   ⍝ (1 1 ⋄ 0 1)

⍝ dyalog:nor:1 —
0 0 1 1⍱0 1 0 1   ⍝ 1 0 0 0

⍝ dyalog:without:1 —
'HELLO'~'GOODBYE'   ⍝ 'HLL'

⍝ dyalog:without:2 —
'MONDAY' 'TUESDAY' 'WEDNESDAY'~'TUESDAY' 'FRIDAY'
('MONDAY' ⋄ 'WEDNESDAY')

⍝ dyalog:unique-mask:1 —
≠22 10 22 22 21 10 5 10   ⍝ 1 1 0 0 1 0 1 0

⍝ dyalog:unique-mask:2 —
≠⊃'CAT' 'DOG' 'CAT' 'DUCK' 'DOG' 'DUCK'   ⍝ 1 1 0 1 0 0

⍝ dyalog:match:1 —
⍬≡⍳0   ⍝ 1

⍝ dyalog:match:2 —
''≡⍳0   ⍝ 0

⍝ dyalog:depth:1 —
≡1 'A'   ⍝ 1

⍝ dyalog:depth:2 —
≡(1 2⋄ 3 (4 5))   ⍝ ¯3

⍝ dyalog:index-of:1 —
2 4 3 1 4⍳1 2 3 4 5   ⍝ 4 1 3 2 6

⍝ dyalog:index-of:2 —
'CAT' 'DOG' 'MOUSE'⍳'DOG' 'BIRD'   ⍝ 2 4

⍝ dyalog:index-of:3 —
(3 4⍴⍳12)⍳2 4⍴1 2 3 4 9 10 11 12   ⍝ 1 3

⍝ dyalog:index-generator:1 —
⍳2 3   ⍝ 2 3⍴(1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3)

⍝ dyalog:index-generator:2 — Combined setup and indexed expression
A←2 4⍴'MAINEXIT' ⋄ A[⍳⍴A]   ⍝ 2 4⍴'MAINEXIT'

⍝ dyalog:exponential:3 —
1+*π0j1   ⍝ 0

⍝ dyalog:power:3 —
¯27*3 2 1.2 .5
¯19683 729 ¯42.22738244439835j¯30.67998919220237 0j5.196152422706632

⍝ dyalog:unique:1 —
∪22 10 22 22 21 10 5 10   ⍝ 22 10 21 5

⍝ dyalog:unique:2 —
∪⊃'CAT' 'DOG' 'CAT' 'DUCK' 'DOG' 'DUCK'   ⍝ 3 4⍴'CAT DOG DUCK'

⍝ dyalog:union:1 —
'WASH'∪'SHOUT'   ⍝ 'WASHOUT'

⍝ dyalog:intersection:1 —
'ABRA'∩'CAR'   ⍝ 'ARA'

⍝ dyalog:intersection:2 —
1 'PLUS' 2∩⍳5   ⍝ 1 2

⍝ dyalog:enlist:1 —
∊2 2⍴'MISS' 'IS' 'SIP' 'PI'   ⍝ 'MISSISSIPPI'

⍝ dyalog:enlist:2 —
∊1 (2 2⍴2 3 4 5) (6(7 8))   ⍝ 1 2 3 4 5 6 7 8

⍝ dyalog:membership:1 —
'THIS NOUN'∊'THAT WORD'   ⍝ 1 1 0 0 1 0 1 0 0

⍝ dyalog:membership:2 —
'CAT' 'DOG' 'MOUSE'∊'CAT' 'FOX' 'DOG' 'LLAMA'
1 1 0

⍝ dyalog:find:1 —
'AN'⍷'BANANA'   ⍝ 0 1 0 1 0 0

⍝ dyalog:find:2 —
'ANA'⍷'BANANA'   ⍝ 0 1 0 1 0 0

⍝ dyalog:find:3 —
'BIRDS' 'NEST'⍷'BIRDS' 'NEST' 'SOUP'   ⍝ 1 0 0

⍝ dyalog:commute:1 — Included displayed N as setup
N←3 2 5 4 6 1 3 ⋄ N/⍨2|N   ⍝ 3 5 1 3

⍝ dyalog:commute:2 —
⍴⍨3   ⍝ 3 3 3

⍝ dyalog:each-monadic:1 — Inlined G
⍴¨('TOM' (⍳3)⋄ 'DICK' (⍳4)⋄ 'HARRY' (⍳5))
(1⍴2 ⋄ 1⍴2 ⋄ 1⍴2)

⍝ dyalog:each-dyadic:1 —
'ABC',¨'XYZ'   ⍝ ('AX' ⋄ 'BY' ⋄ 'CZ')

⍝ dyalog:each-dyadic:2 — Inlined G
1 2 3 4↑¨(1 (2 3))(4 (5 6))(8 9)10   ⍝ (1⍴1 ⋄ 4 (5 6) ⋄ 8 9 0 ⋄ 10 0 0 0)

⍝ dyalog:reduce-n-wise:1 —
+/3↕⍳4   ⍝ 6 9

⍝ dyalog:reduce-n-wise:2 —
+/2↕⍳4   ⍝ 3 5 7

⍝ dyalog:reduce-n-wise:3 —
+/1↕⍳4   ⍝ 1 2 3 4

⍝ dyalog:reduce-n-wise:4 —
+/0↕⍳4   ⍝ 0 0 0 0 0

⍝ dyalog:reduce-n-wise:5 —
×/0↕⍳4   ⍝ 1 1 1 1 1

⍝ dyalog:reduce-n-wise:6 —
,/2↕⍳4   ⍝ (1 2 ⋄ 2 3 ⋄ 3 4)

⍝ dyalog:reduce-n-wise:7 —
,/⌽2↕⍳4   ⍝ (2 1 ⋄ 3 2 ⋄ 4 3)

⍝ dyalog:reduce:identity-catenate —
''≡,/0⍴'Hello' 'World'   ⍝ 1

⍝ dyalog:reduce:identity-table —
(0 3 4⍴0)≡⍪/0⍴⊂2 3 4⍴0   ⍝ 1

⍝ dyalog:where:1 —
⍸1 0 1 0 0 0 0 1 0   ⍝ 1 3 8

⍝ dyalog:where:2 —
⍸2 2⍴0 1 2 3   ⍝ (1 2 ⋄ 2 1 ⋄ 2 1 ⋄ 2 2 ⋄ 2 2 ⋄ 2 2)

⍝ dyalog:interval-index:1 —
10 20 30⍸11 1 31 21   ⍝ 1 0 3 2

⍝ dyalog:interval-index:2 —
'AEIOU'⍸'DYALOG'   ⍝ 1 5 1 3 4 2

⍝ dyalog:interval-index:3 —
(⊃'Fi' 'Jay' 'John' 'Morten' 'Roger')⍸⊃'JD' 'Jd' 'Geoff' 'Alpha' 'Omega' 'Zeus  '
1 2 1 0 4 5

⍝ dyalog:bind:1 —
(*∘0.5)4 16 25   ⍝ 2 4 5

⍝ dyalog:atop:1 —
12 -⍤÷4   ⍝ ¯3

⍝ dyalog:over:1 —
2 3 ,⍥⊂ 'text'   ⍝ (2 3 ⋄ 'text')

⍝ dyalog:behind:1 —
3∘<⍛/2 7 1 8 2 8   ⍝ 7 8 8

⍝ dyalog:behind:2 —
⌊⍛=⍛/1 3.2 ¯5 0 ¯3.2 8.1   ⍝ 1 ¯5 0

⍝ dyalog:rank:1 — Origin 1 instead of displayed origin 0
10 20 30 (+⍤0 1)3 4⍴⍳12   ⍝ 3 4⍴11 12 13 14 25 26 27 28 39 40 41 42

⍝ dyalog:rank:2 — Smaller iota array replaces the displayed Y
⊂⍤1⊢2 2 3⍴⍳12   ⍝ 2 2⍴(1 2 3 ⋄ 4 5 6 ⋄ 7 8 9 ⋄ 10 11 12)

⍝ dyalog:right:1 —
(⊢÷+/)4 3 0 1   ⍝ 0.5 0.375 0 0.125

⍝ dyalog:right:2 —
⊢/1 2 3   ⍝ 3

⍝ dyalog:left:1 —
⊣/1 2 3   ⍝ 1

⍝ dyalog:trains:1 —
6(+,-,×,÷)2   ⍝ 8 4 12 3

⍝ dyalog:trains:2 —
6(⌽+,-,×,÷)2   ⍝ 3 12 4 8

⍝ dyalog:trains:3 —
(2/⍳)3   ⍝ 1 1 2 2 3 3

⍝ dyalog:trains:4 —
(⍳(/∘⊢)⍳)3   ⍝ 1 2 2 3 3 3

⍝ dyalog:inner-product:1 —
1 2 3+.×10 12 14   ⍝ 76

⍝ dyalog:outer-product:1 —
1 2 3 ×⌝ 10 20 30 40
3 4⍴10 20 30 40 20 40 60 80 30 60 90 120

⍝ dyalog:outer-product:2 —
1 2 ,⌝ 1 2 3   ⍝ 2 3⍴(1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3)

⍝ dyalog:outer-product:3 —
(⍳3) =⌝ ⍳3   ⍝ 3 3⍴1 0 0 0 1 0 0 0 1

⍝ dyalog:key:1 — Inline letters definition
{⍺(≢⍵)}⌸'zabayza'   ⍝ 4 2⍴'z' 2 'a' 3 'b' 1 'y' 1

⍝ dyalog:power:successor — Inline successor definition
(1∘+⍣4)10   ⍝ 14

⍝ dyalog:grade-up:vector —
⍋22.5 1 15 3 ¯4   ⍝ 5 2 4 3 1

⍝ dyalog:grade-down:vector —
⍒22.5 1 15 3 ¯4   ⍝ 1 3 4 2 5

⍝ dyalog:grade-up:matrix — Inline displayed M
⍋3 2 3⍴2 3 5 1 4 7 2 3 4 5 2 4 2 3 5 1 2 6
2 3 1

⍝ dyalog:grade-down:collation — Inline explanation's left and right matrices
(2 3⍴'abcABA')⍒4 2⍴'abacAaAc'   ⍝ 4 2 1 3

⍝ dyalog:partition:runs —
1 1 1 2 2 3 3 3⊆'NOWISTHE'   ⍝ ('NOW' ⋄ 'IS' ⋄ 'THE')

⍝ dyalog:partitioned-enclose:dividers —
2 0 1 3 0 2 0 1⊂'abcdefg'
('' ⋄ 'ab' ⋄ 1⍴'c' ⋄ '' ⋄ '' ⋄ 'de' ⋄ '' ⋄ 'fg' ⋄ '')

⍝ dyalog:partitioned-enclose:axis —
1 0 1⊂[1]3 4⍴⍳12   ⍝ (2 4⍴1 2 3 4 5 6 7 8 ⋄ 1 4⍴9 10 11 12)

⍝ dyalog:pick:path — Inline G definition
2⊃1⊃2 1⊃2 3⍴('ABC' 1⋄ 'DEF' 2⋄ 'GHI' 3⋄ 'JKL' 4⋄ 'MNO' 5⋄ 'PQR' 6)
'K'

⍝ dyalog:index:shape — Inline VEC definition
[2 3⍴3 1 4 1 2 3]⌷111 222 333 444   ⍝ 2 3⍴333 111 444 111 222 333

⍝ dyalog:encode:mixed-base —
0 10⊤5 15 125   ⍝ 2 3⍴0 1 12 5 5 5

⍝ dyalog:encode:fraction —
0 1⊤1.25 10.5   ⍝ 2 2⍴1 10 0.25 0.5

⍝ dyalog:decode:complex —
1j1⊥1 2 3 4   ⍝ 5j9

⍝ dyalog:at:replace —
10 20@2 4⍳5   ⍝ 1 10 3 20 5

⍝ dyalog:stencil:sum —
{+/,⍵}⌺3 3⊢3 3⍴⍳12   ⍝ 3 3⍴12 21 16 27 45 33 24 39 28

⍝ dyalog:stencil:even — Shorter input
{⊂⍵}⌺2⍳4   ⍝ (1 2 ⋄ 2 3 ⋄ 3 4)

⍝ dyalog:matrix-inverse:1 —
⌹2 2⍴2 ¯3 4 10   ⍝ 2 2⍴0.3125 0.09375 ¯0.125 0.0625

⍝ dyalog:matrix-inverse:2 —
⌹1j1 2
0.1666666666666667j¯0.1666666666666667 0.3333333333333333

⍝ dyalog:matrix-divide:1 —
3 5 7⌹3 2⍴1 1 1 2 1 3   ⍝ 1 2

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 packN packN 2 3⍴0 0 3 0 0 4
⍝ =>
2 3⍴0 0 3 0 0 4

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 packR packR 2 3⍴'abbccc'
⍝ =>
2 3⍴'abbccc'

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 packU packU 2 3⍴'abbccc'
⍝ =>
2 3⍴'abbccc'

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 packB packB 2 3⍴0 1 1 0 1 0
⍝ =>
2 3⍴0 1 1 0 1 0

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 packX packX 2 3⍴'abbccc'
⍝ =>
2 3⍴'abbccc'

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 packQ packQ 2 3⍴'abbccc'
⍝ =>
2 3⍴'abbccc'

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 packS packS 2 3⍴'abbccc'
⍝ =>
2 3⍴'abbccc'

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 pack4 pack4 2 3⍴'abbccc'
⍝ =>
2 3⍴'abbccc'

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 packH packH 2 3⍴'abbccc'
⍝ =>
2 3⍴'abbccc'

⍝ — Dyalog dfns: queens up to symmetry
•LOAD 'lib/dyalog.apl'
queens 4
⍝ =>
1⍴⊂(4 7⍴'· ⍟ · ·· · · ⍟⍟ · · ·· · ⍟ ·')

⍝ — Dyalog dfns: nested macro definitions
•LOAD 'lib/dyalog.apl'
mac 'A=++ B=(2A) B'
⍝ =>
'++'

⍝ — Dyalog dfns: internal box borders
•LOAD 'lib/dyalog.apl'
(1 2⋄ 1 2)box 2 3⍴'abcdef'
⍝ =>
5 7⍴'┌─┬─┬─┐│a│b│c│├─┼─┼─┤│d│e│f│└─┴─┴─┘'

⍝ — Dyalog dfns: lisp
•LOAD 'lib/dyalog.apl'
lisp '((lambda (x) (+ x 1)) 41)'
⍝ =>
42

⍝ — Dyalog dfns: lisp
•LOAD 'lib/dyalog.apl'
lisp '(cond ((= 2 3) 10) (else 20))'
⍝ =>
20

⍝ — Dyalog dfns: lisp
•LOAD 'lib/dyalog.apl'
lisp '(quote (a (b c)))'
⍝ =>
(1⍴'a' ⋄ (1⍴'b' ⋄ 1⍴'c'))

⍝ — Dyalog dfns: lisp
•LOAD 'lib/dyalog.apl'
0 lisp '(+ 2 3)'
⍝ =>
(1⍴'+') 2 3

⍝ — Dyalog dfns: lisp
•LOAD 'lib/dyalog.apl'
lisp '(+ 2'
⍝ =>
'unexpected eof'

⍝ — Dyalog dfns: Cut
•LOAD 'lib/dyalog.apl'
1 0 1 0(+/Cut ¯1)1 2 3 4
⍝ =>
2 4

⍝ — Dyalog dfns: Cut
•LOAD 'lib/dyalog.apl'
0 1 0 1(+/Cut 2)1 2 3 4
⍝ =>
3 7

⍝ — Dyalog dfns: Cut
•LOAD 'lib/dyalog.apl'
0 1 0 1(+/Cut ¯2)1 2 3 4
⍝ =>
1 3

⍝ — Dyalog dfns: Cut
•LOAD 'lib/dyalog.apl'
(2 2⍴2 2 2 2)(⊢Cut 0)3 4⍴⍳12
⍝ =>
2 2⍴6 7 10 11

⍝ — Dyalog dfns: Cut
•LOAD 'lib/dyalog.apl'
2(+/Cut 3)⍳5
⍝ =>
3 5 7 9 5

⍝ — Dyalog dfns: Cut
•LOAD 'lib/dyalog.apl'
2(+/Cut ¯3)⍳5
⍝ =>
3 5 7 9

⍝ — Dyalog dfns: Cut
•LOAD 'lib/dyalog.apl'
(1 0 1)(1 0 1 0)((+/,)Cut 1)3 4⍴⍳12
⍝ =>
2 2⍴14 22 19 23

⍝ — Dyalog dfns: Cut
•LOAD 'lib/dyalog.apl'
(+/Cut 1)1 2 1 4
⍝ =>
3 5

⍝ — Dyalog dfns: eis
•LOAD 'lib/dyalog.apl'
(eis 1⋄ 1 2 eis 3 4⋄ eis(1 2⋄ 3 4))
⍝ =>
(⊂(1⍴1) ⋄ (1 2 ⋄ 3 4) ⋄ (1 2 ⋄ 3 4))

⍝ — Dyalog dfns: iotag
•LOAD 'lib/dyalog.apl'
iotag 2 3
⍝ =>
2 3⍴(1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3)

⍝ — Dyalog dfns: iotag
•LOAD 'lib/dyalog.apl'
iotag ¯3
⍝ =>
¯1 ¯2 ¯3

⍝ — Dyalog dfns: iotag
•LOAD 'lib/dyalog.apl'
iotag 'c'
⍝ =>
'abc'

⍝ — Dyalog dfns: descending character interval
•LOAD 'lib/dyalog.apl'
'c'iotag'a'
⍝ =>
'cba'

⍝ — Dyalog dfns: row search
•LOAD 'lib/dyalog.apl'
(2 3⍴1 2 3 4 5 6)iotag 2 3⍴4 5 6 1 2 3
⍝ =>
2 1

⍝ — Dyalog dfns: ssword
•LOAD 'lib/dyalog.apl'
ssword 'alpha alphabet alpha12 alpha' 'alpha' 'X'
⍝ =>
'X alphabet alpha12 X'

⍝ — Dyalog dfns: tokens
•LOAD 'lib/dyalog.apl'
1 tokens 'x←1 2 3 ⋄ ''abc'' ⍝ comment'
⍝ =>
(1⍴'x' ⋄ 1⍴'←' ⋄ '1 2 3' ⋄ 1⍴' ' ⋄ 1⍴'⋄' ⋄ 1⍴' ' ⋄ '''abc''' ⋄ 1⍴' ' ⋄ '⍝ comment')

⍝ — Dyalog dfns: word boundaries
•LOAD 'lib/dyalog.apl'
words¨'' '123' 'abc12 + 34def' 'abc_def'
⍝ =>
(0⍴⊂('') ⋄ 1⍴⊂('123') ⋄ ('abc12' ⋄ ' + 34' ⋄ 'def') ⋄ 1⍴⊂('abc_def'))

⍝ — Dyalog dfns: word boundaries
•LOAD 'lib/dyalog.apl'
('abc' '123')words 'a12+b3 cZ1a'
⍝ =>
('a12' ⋄ 1⍴'+' ⋄ 'b3' ⋄ 1⍴' ' ⋄ 1⍴'c' ⋄ 'Z1' ⋄ 1⍴'a')

⍝ — Dyalog dfns: compression round trip
•LOAD 'lib/dyalog.apl'
0 packD packD 2 3⍴'abbccc'
⍝ =>
2 3⍴'abbccc'

⍝ — Dyalog dfns: run boundaries and escaped characters
•LOAD 'lib/dyalog.apl'
0 packT packT (257⍴'a'),'bbb',(3⍴•UCS 0),(260⍴'c')
⍝ =>
(257⍴'a'),'bbb',(3⍴•UCS 0),(260⍴'c')

⍝ — Dyalog dfns: Baby skips a store with a negative accumulator
•LOAD 'lib/dyalog.apl'
m←⌽⍉(32⍴2)⊤32↑0 16389 49152 24582 57344 13 0 ⋄ 2⊥⌽(baby m)[7;]
⍝ =>
0

⍝ — Dyalog dfns: parse
•LOAD 'lib/dyalog.apl'
g←'A 1 2 3' 'F + ×' 'B' '' 'F:A→B' 'A:B→A' ⋄ g parse '1+(2×3)'
⍝ =>
7 11⍴' A         ┌┴─┐       1 ┌┴─┐       + ┌┴─┐       ( ┌┴─┐       2 ┌┴┐        × 3'

⍝ — Dyalog dfns: parse
•LOAD 'lib/dyalog.apl'
g←'A 1 2 3' 'F + ×' 'B' '' 'F:A→B' 'A:B→A' ⋄ c←(0 g)parse'' ⋄ c parse '1+2×3'
⍝ =>
5 9⍴'    A     ┌──┴──┐ ┌┴─┐  ┌┴┐1 ┌┴┐ × 3  + 2    '

⍝ — Dyalog dfns: parse
•LOAD 'lib/dyalog.apl'
g←'A 1 2 3' 'F + ×' 'B' '' 'F:A→B' 'A:B→A' ⋄ g parse''
⍝ =>
7 11⍴'  ┌───┬───┐  │A  │B  │┌─┼───┼───┤│A│   │1 A│├─┼───┼───┤│F│1 B│   │└─┴───┴───┘'

⍝ — Dyalog dfns: parse
•LOAD 'lib/dyalog.apl'
g←'A 1 2 3' 'F + ×' 'B' '' 'F:A→B' 'A:B→A' ⋄ g parse '1+z'
⍝ =>
4 7⍴'┌─┬─┬─┐│A│F│z││1│+│ │└─┴─┴─┘'

⍝ — Dyalog dfns: parse
•LOAD 'lib/dyalog.apl'
g←'N=A' 'N 1 2 3' 'F + ×' 'B' '' 'F:N→B' 'N:B→N' ⋄ g parse '1+2×3'
⍝ =>
5 9⍴'    A     ┌──┴──┐ ┌┴─┐  ┌┴┐1 ┌┴┐ × 3  + 2    '


