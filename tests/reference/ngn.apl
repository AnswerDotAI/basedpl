⍝ ngn:1 —
⍬   ⍝ ⍬

⍝ ngn:2 —
⍴⍬   ⍝ 1⍴0

⍝ ngn:3 —
'abcd'~'bde'   ⍝ 'ac'

⍝ ngn:4 —
(⍳6)~0 2 4   ⍝ 1 3 5 6

⍝ ngn:5 —
'ab' 'cd' 'ad'~'a'   ⍝ ('ab') ('cd') ('ad')

⍝ ngn:6 —
'ab' 'cd' 'ad'~'cd'   ⍝ ('ab') ('cd') ('ad')

⍝ ngn:7 —
'ab' 'cd' 'ad'~⊂'cd'   ⍝ ('ab') ('ad')

⍝ ngn:8 —
'ab' 'cd' 'ad'~'a' 'cd'   ⍝ ('ab') ('ad')

⍝ ngn:9 —
(11+⍳6)~2 3⍴1 2 3 14 5 6   ⍝ 12 13 15 16 17

⍝ ngn:10 —
(2 2⍴⍳4)~2
⍝ error: RANK ERROR

⍝ ngn:11 — [rtol=1e-14]
(-⍟)2 3   ⍝ ¯0.6931471805599453 ¯1.09861228866811

⍝ ngn:12 —
2(-*)3   ⍝ ¯8

⍝ ngn:13 —
⊃3   ⍝ ⊂3

⍝ ngn:14 —
⊃(1 2)(3 4)   ⍝ 2 2⍴1 2 3 4

⍝ ngn:15 —
⊃(1 2)(3 4 5)   ⍝ 2 3⍴1 2 0 3 4 5

⍝ ngn:16 —
⊃1 2   ⍝ 1 2

⍝ ngn:17 —
⊃(1 2)3   ⍝ 2 2⍴1 2 3 0

⍝ ngn:18 —
⊃1(2 3)   ⍝ 2 2⍴1 0 2 3

⍝ ngn:19 —
⊃2 2⍴1(1 1 2⍴3 4)(5 6)(2 0⍴0)
2 2 1 2 2⍴1 0 0 0 3 4 0 0 5 6 0 0 0 0 0 0

⍝ ngn:20 —
⊃⍬   ⍝ ⍬

⍝ ngn:21 —
⊃2 3 0⍴0   ⍝ 2 3 0⍴0

⍝ ngn:22 —
⍬⊃3   ⍝ 3

⍝ ngn:23 —
3⊃'pick'   ⍝ 'c'

⍝ ngn:24 —
(2 1)⊃2 2⍴'abcd'   ⍝ 'c'

⍝ ngn:25 —
2⊃'foo' 'bar'   ⍝ 'bar'

⍝ ngn:26 —
3⊃2⊃'foo' 'bar'   ⍝ 'r'

⍝ ngn:27 —
(2 2⍴0)⊃1 2
⍝ error: RANK ERROR

⍝ ngn:28 —
(2 1⍴0)⊃2 2⍴0
⍝ error: RANK ERROR

⍝ ngn:29 —
(2 2⍴0)⊃1 2
⍝ error: RANK ERROR

⍝ ngn:30 —
(2 2)⊃1 2
⍝ error: RANK ERROR

⍝ ngn:31 —
(0 2)⊃2 2⍴'ABCD'
⍝ error: INDEX ERROR

⍝ ngn:32 —
a←' this is a test '⋄(a≠' ')⊂a
(1⍴'t') (1⍴'h') (1⍴'i') ('s ') (1⍴'i') ('s ') ('a ') (1⍴'t') (1⍴'e') (1⍴'s') ('t ')

⍝ ngn:33 —
↓1 2 3   ⍝ ⊂(1 2 3)

⍝ ngn:34 —
↓(1 2)(3 4)   ⍝ ⊂((1 2) (3 4))

⍝ ngn:35 —
↓2 2⍴⍳4   ⍝ (1 2) (3 4)

⍝ ngn:36 —
↓2 3 4⍴⍳24
2 3⍴(1 2 3 4) (5 6 7 8) (9 10 11 12) (13 14 15 16) (17 18 19 20) (21 22 23 24)

⍝ ngn:37 —
2↓'abc'   ⍝ 1⍴'c'

⍝ ngn:38 —
¯1↓'abc'   ⍝ 'ab'

⍝ ngn:39 —
5↓'abc'   ⍝ ''

⍝ ngn:40 — Uses already implemented ⎕A/⎕UCS with ordinary array operations; original independent expectation passes unchanged
0 ¯2↓3 3⍴•a   ⍝ 3 1⍴'ADG'

⍝ ngn:41 — Uses already implemented ⎕A/⎕UCS with ordinary array operations; original independent expectation passes unchanged
¯2 ¯1↓3 3⍴•a   ⍝ 1 2⍴'AB'

⍝ ngn:42 — Uses already implemented ⎕A/⎕UCS with ordinary array operations; original independent expectation passes unchanged
1↓3 3⍴•a   ⍝ 2 3⍴'DEFGHI'

⍝ ngn:43 —
⍬↓3 3⍴⍳9   ⍝ 3 3⍴1 2 3 4 5 6 7 8 9

⍝ ngn:44 — Uses already implemented ⎕A/⎕UCS with ordinary array operations; original independent expectation passes unchanged
1 1↓2 3 4⍴•a   ⍝ 1 2 4⍴'QRSTUVWX'

⍝ ngn:45 — Uses already implemented ⎕A/⎕UCS with ordinary array operations; original independent expectation passes unchanged
¯1 ¯1↓2 3 4⍴•a   ⍝ 1 2 4⍴'ABCDEFGH'

⍝ ngn:46 —
1↓0   ⍝ ⍬

⍝ ngn:47 —
0 1↓2   ⍝ 1 0⍴0

⍝ ngn:48 —
1 2↓3   ⍝ 0 0⍴0

⍝ ngn:49 —
⍬↓0   ⍝ 0

⍝ ngn:50 —
⍪2 3 4   ⍝ 3 1⍴2 3 4

⍝ ngn:51 —
⍪0   ⍝ 1 1⍴0

⍝ ngn:52 —
⍪2 2⍴2 3 4 5   ⍝ 2 2⍴2 3 4 5

⍝ ngn:53 —
⍴⍪2 3⍴⍳6   ⍝ 2 3

⍝ ngn:54 —
⍴⍪2 3 4⍴⍳24   ⍝ 2 12

⍝ ngn:55 —
(2 3⍴⍳6)⍪9   ⍝ 3 3⍴1 2 3 4 5 6 9 9 9

⍝ ngn:56 —
1⍪2   ⍝ 1 2

⍝ ngn:57 —
1⊢2   ⍝ 2

⍝ ngn:58 —
⊢3   ⍝ 3

⍝ ngn:59 —
1⊣2   ⍝ 1

⍝ ngn:60 —
⊣3   ⍝ 3

⍝ ngn:61 —
≢0   ⍝ 1

⍝ ngn:62 —
≢0 0   ⍝ 2

⍝ ngn:63 —
≢⍬   ⍝ 0

⍝ ngn:64 —
≢2 3⍴⍳6   ⍝ 2

⍝ ngn:65 —
2≢2   ⍝ 0

⍝ ngn:66 — Ravel alphabet matrix; use the supported uppercase spelling of the read-only alphabet constant; independent Dyalog expectation
,2 13⍴•A   ⍝ 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'

⍝ ngn:67 —
,1   ⍝ 1⍴1

⍝ ngn:68 —
⌹2   ⍝ 0.5

⍝ ngn:69 — [rtol=1e-14]
⌹2 2⍴4 3 3 2
2 2⍴¯1.999999999999999 2.999999999999999 2.999999999999999 ¯3.999999999999998

⍝ ngn:70 —
⌹2 2 2⍴⍳8
⍝ error: RANK ERROR

⍝ ngn:71 —
⌹2 3⍴⍳6
⍝ error: LENGTH ERROR

⍝ ngn:72 — [rtol=1e-14]
(4 4⍴12 1 4 10 ¯6 ¯5 4 7 ¯4 9 3 4 ¯2 ¯6 7 7)⌹93 81 93.5 120.5
0.0003898888816687244 ¯0.005029566573526545 0.04730651764247189 0.0705568912859835

⍝ ngn:73 —
17-⍨23   ⍝ 6

⍝ ngn:74 —
7⍴⍨2 3   ⍝ 2 3⍴7 7 7 7 7 7

⍝ ngn:75 —
+⍨2   ⍝ 4

⍝ ngn:76 —
-⍨123   ⍝ 0

⍝ ngn:77 —
¯3 ¯4*2   ⍝ 9 16

⍝ ngn:78 —
0j1*2   ⍝ ¯1

⍝ ngn:79 — [rtol=1e-14]
1j2*3   ⍝ ¯11j¯2

⍝ ngn:80 — [rtol=1e-14]
.5j1.5*5   ⍝ 9.875j¯0.375

⍝ ngn:81 — [rtol=1e-14]
9 4 0 ¯4 ¯9*.5   ⍝ 3 2 0 0j2 0j3

⍝ ngn:82 —
'hello'}
⍝ error: SYNTAX ERROR

⍝ ngn:83 —
(1 1⍴2)+1 1 1⍴3   ⍝ 1 1 1⍴5

⍝ ngn:84 —
+0((1j¯2 ¯3j4)¯5.6)   ⍝ 0 ((1j2 ¯3j¯4) ¯5.6)

⍝ ngn:85 —
1(2 3)+(4 5)6   ⍝ (5 6) (8 9)

⍝ ngn:86 —
(2 3⍴1 2 3 4 5 6)+¯2   ⍝ 2 3⍴¯1 0 1 2 3 4

⍝ ngn:87 —
1 2 3+4 5
⍝ error: LENGTH ERROR

⍝ ngn:88 —
(2 3⍴1 2 3 4 5 6)+2⍴¯2   ⍝ 2 3⍴¯1 0 1 2 3 4

⍝ ngn:89 —
(2 3⍴⍳6)+3 2⍴⍳6
⍝ error: LENGTH ERROR

⍝ ngn:90 —
(2 3⍴1 2 3 4 5 6)+2 3⍴¯2   ⍝ 2 3⍴¯1 0 1 2 3 4

⍝ ngn:91 —
1j¯2+¯2j3   ⍝ ¯1j1

⍝ ngn:92 —
+/⍬   ⍝ 0

⍝ ngn:93 — ngn bare-overbar infinity aliases ported to explicit ∞/¯∞; no complex infinities
∞+¯∞
⍝ error: DOMAIN ERROR

⍝ ngn:94 — ngn bare-overbar infinity aliases ported to explicit ∞/¯∞; no complex infinities
1j∞+2j¯∞
⍝ error: DOMAIN ERROR

⍝ ngn:95 —
-4(1 2 3)1j2   ⍝ ¯4 (¯1 ¯2 ¯3) ¯1j¯2

⍝ ngn:96 —
1-3   ⍝ ¯2

⍝ ngn:97 —
5j2-3j8   ⍝ 2j¯6

⍝ ngn:98 —
-/⍬   ⍝ 0

⍝ ngn:99 — ngn bare-overbar infinity aliases ported to explicit ∞/¯∞; no complex infinities
×¯2 ¯1 0 1 2 ∞ ¯∞ 3j¯4   ⍝ ¯1 ¯1 0 1 1 1 ¯1 0.6j¯0.8

⍝ ngn:100 —
7×8   ⍝ 56

⍝ ngn:101 —
1j¯2×¯2j3   ⍝ 4j7

⍝ ngn:102 —
2×1j¯2   ⍝ 2j¯4

⍝ ngn:103 —
×/⍬   ⍝ 1

⍝ ngn:104 —
÷2   ⍝ 0.5

⍝ ngn:105 — [rtol=1e-14]
÷2j3   ⍝ 0.1538461538461539j¯0.2307692307692308

⍝ ngn:106 —
0÷0   ⍝ 1

⍝ ngn:107 —
27÷9   ⍝ 3

⍝ ngn:108 —
4j7÷1j¯2   ⍝ ¯2j3

⍝ ngn:109 —
0j2÷0j1   ⍝ 2

⍝ ngn:110 —
5÷2j1   ⍝ 2j¯1

⍝ ngn:111 —
÷/⍬   ⍝ 1

⍝ ngn:112 —
*2   ⍝ 7.38905609893065

⍝ ngn:113 — [rtol=1e-14]
*2j3   ⍝ ¯7.315110094901103j1.042743656235904

⍝ ngn:114 —
2 3 ¯2 ¯3*3 2 3 2   ⍝ 8 9 ¯8 9

⍝ ngn:115 —
¯1*.5   ⍝ 0j1

⍝ ngn:116 —
*/⍬   ⍝ 1

⍝ ngn:117 — [rtol=1e-14]
1j2*3j4   ⍝ 0.129009594074467j0.03392409290517014

⍝ ngn:118 —
⍟123   ⍝ 4.812184355372417

⍝ ngn:120 —
⍟¯1   ⍝ 0j3.141592653589793

⍝ ngn:121 — [rtol=1e-14]
⍟123j456   ⍝ 6.157609243895447j1.307329785759979

⍝ ngn:122 —
12⍟34 ¯34
1.419111870829036 1.419111870829036j1.26426988871305

⍝ ngn:123 — [rtol=1e-14]
¯12⍟¯34   ⍝ 1.161297476399478j¯0.2039235425372641

⍝ ngn:124 — [rtol=1e-14]
1j2⍟3j4   ⍝ 1.239382825269869j¯0.5528462880299602

⍝ ngn:125 —
|¯8 0 8 ¯3.5   ⍝ 8 0 8 3.5

⍝ ngn:126 —
|5j12   ⍝ 13

⍝ ngn:127 —
3|5   ⍝ 2

⍝ ngn:128 —
1j2|3j4   ⍝ ¯1j1

⍝ ngn:129 —
7 ¯7 |⌝ 31 28 ¯30   ⍝ 2 3⍴3 0 5 ¯4 0 ¯2

⍝ ngn:130 — [rtol=1e-14]
¯0.2 0 0.2 |⌝ ¯0.3 0 0.3
3 3⍴¯0.09999999999999998 0 ¯0.1 ¯0.3 0 0.3 0.1 0 0.09999999999999998

⍝ ngn:131 —
|/⍬   ⍝ 0

⍝ ngn:132 —
0|¯4   ⍝ ¯4

⍝ ngn:133 —
0|¯4j5   ⍝ ¯4j5

⍝ ngn:134 —
10|4j3   ⍝ 4j3

⍝ ngn:135 —
4j6|7j10   ⍝ 3j4

⍝ ngn:136 —
¯10 7j10 0.3|17 5 10   ⍝ ¯3 ¯5j7 0.09999999999999964

⍝ ngn:137 —
+\20 10 ¯5 7   ⍝ 20 30 25 32

⍝ ngn:138 —
,\'AB' 'CD' 'EF'   ⍝ ('AB') ('ABCD') ('ABCDEF')

⍝ ngn:139 —
×\2 3⍴5 2 3 4 7 6   ⍝ 2 3⍴5 10 30 4 28 168

⍝ ngn:140 —
∧\1 1 1 0 1 1   ⍝ 1 1 1 0 0 0

⍝ ngn:141 — basedpl left-accumulating scan
-\1 2 3 4   ⍝ 1 ¯1 ¯4 ¯8

⍝ ngn:142 —
∨\0 0 1 0 0 1 0   ⍝ 0 0 1 1 1 1 1

⍝ ngn:143 —
+\1 2 3 4 5   ⍝ 1 3 6 10 15

⍝ ngn:144 —
+\(1 2 3)(4 5 6)(7 8 9)   ⍝ (1 2 3) (5 7 9) (12 15 18)

⍝ ngn:145 —
M←2 3⍴1 2 3 4 5 6⋄+\M   ⍝ 2 3⍴1 3 6 4 9 15

⍝ ngn:146 —
M←2 3⍴1 2 3 4 5 6⋄+⍀M   ⍝ 2 3⍴1 2 3 5 7 9

⍝ ngn:147 —
M←2 3⍴1 2 3 4 5 6⋄+\[1]M   ⍝ 2 3⍴1 2 3 5 7 9

⍝ ngn:148 —
,\'abc'   ⍝ 'a' ('ab') ('abc')

⍝ ngn:149 —
T←'ONE(TWO) BOOK(S)'⋄≠\T∊'()'   ⍝ 0 0 0 1 1 1 1 0 0 0 0 0 0 1 1 0

⍝ ngn:150 —
T←'ONE(TWO) BOOK(S)'⋄((T∊'()')⍱≠\T∊'()')/T
'ONE BOOK'

⍝ ngn:151 —
1 0 1\'ab'   ⍝ 'a b'

⍝ ngn:152 —
0 1 0 1 0\2 3   ⍝ 0 2 0 3 0

⍝ ngn:153 —
(2 2⍴0)\'food'
⍝ error: RANK ERROR

⍝ ngn:154 —
'abc'\'def'
⍝ error: DOMAIN ERROR

⍝ ngn:155 —
1 0 1 1\'ab'
⍝ error: LENGTH ERROR

⍝ ngn:156 —
1 0 1 1\'abcd'
⍝ error: LENGTH ERROR

⍝ ngn:157 —
1 0 1\2 2⍴'ABCD'   ⍝ 2 3⍴'A BC D'

⍝ ngn:158 —
1 0 1⍀2 2⍴'ABCD'   ⍝ 3 2⍴'AB  CD'

⍝ ngn:159 —
1 0 1\[1]2 2⍴'ABCD'   ⍝ 3 2⍴'AB  CD'

⍝ ngn:160 —
1 0 1\[2]2 2⍴'ABCD'   ⍝ 2 3⍴'A BC D'

⍝ ngn:161 —
π2   ⍝ 6.283185307179586

⍝ ngn:162 —
π2j2   ⍝ 6.283185307179586j6.283185307179586

⍝ ngn:163 —
π'ABC'
⍝ error: DOMAIN ERROR

⍝ ngn:164 — [rtol=1e-14]
¯12○2 2j3
¯0.4161468365471424j0.9092974268256817 ¯0.02071873100224288j0.04527125315609298

⍝ ngn:165 —
¯11○2 2j3   ⍝ 0j2 ¯3j2

⍝ ngn:166 —
¯10○2 2j3   ⍝ 2 2j¯3

⍝ ngn:167 —
¯9○2 2j3   ⍝ 2 2j3

⍝ ngn:168 — [rtol=1e-14]
¯8○2 2j3
0j¯2.23606797749979 ¯2.885230548905366j2.079556520111141

⍝ ngn:169 — [rtol=1e-14]
¯7○0.5 2 2j3
0.5493061443340548 0.5493061443340548j1.570796326794897 0.1469466662255297j1.338972522294493

⍝ ngn:170 — [rtol=1e-14]
¯6○0.5 2 2j3
¯1.110223024625157e¯16j1.047197551196598 1.316957896924817 1.983387029916535j1.000143542473797

⍝ ngn:171 — [rtol=1e-14]
¯5○2 2j3
1.44363547517881 1.968637925793096j0.9646585044076028

⍝ ngn:172 — [rtol=1e-14]
¯4○2 0 ¯2 2j3
1.732050807568877 0j1 ¯1.732050807568877 1.925669736091671j3.115799084103365

⍝ ngn:173 — [rtol=1e-14]
¯3○0.5 2 2j3
0.4636476090008061j1.110223024625157e¯16 1.10714871779409j1.110223024625157e¯16 1.409921049596576j0.2290726829685388

⍝ ngn:174 — [rtol=1e-14]
¯2○0.5 2 2j3
1.047197551196598 0j1.316957896924817 1.000143542473797j¯1.983387029916535

⍝ ngn:175 — [rtol=1e-14]
¯1○0.5 2 2j3
0.5235987755982988 1.570796326794897j¯1.316957896924817 0.5706527843210994j1.983387029916535

⍝ ngn:176 — [rtol=1e-14]
0○0.5 2 2j3
0.8660254037844386 0j1.732050807568877 3.115799084103365j¯1.925669736091672

⍝ ngn:177 — sin(pi/6) = .5
1e¯10>|.5-1○π÷6   ⍝ 1

⍝ ngn:178 —
1○1 2j3
0.8414709848078965 9.15449914691143j¯4.168906959966565

⍝ ngn:179 —
2○1 2j3
0.5403023058681398 ¯4.189625690968807j¯9.109227893755337

⍝ ngn:180 — [rtol=1e-14]
3○1 2j3
1.557407724654902 ¯0.003764025641504152j1.00323862735361

⍝ ngn:181 — [rtol=1e-14]
4○2 2j3
2.23606797749979 2.079556520111141j2.885230548905366

⍝ ngn:182 — [rtol=1e-14]
5○2 2j3
3.626860407847019 ¯3.590564589985779j0.5309210862485197

⍝ ngn:183 — [rtol=1e-14]
6○2 2j3
3.762195691083631 ¯3.724545504915322j0.5118225699873845

⍝ ngn:184 —
7○2 2j3
0.9640275800758169 0.9653858790221331j¯0.009884375038322494

⍝ ngn:185 — [rtol=1e-14]
8○2 2j3
0j2.23606797749979 2.885230548905366j¯2.079556520111141

⍝ ngn:186 —
9○2 2j3   ⍝ 2 2

⍝ ngn:187 —
10○¯2 ¯2j3   ⍝ 2 3.605551275463989

⍝ ngn:188 —
11○2  2j3   ⍝ 0 3

⍝ ngn:189 —
12○2  2j3   ⍝ 0 0.982793723247329

⍝ ngn:190 —
1○'a'
⍝ error: DOMAIN ERROR

⍝ ngn:191 —
99○1
⍝ error: DOMAIN ERROR

⍝ ngn:192 —
99○1j2
⍝ error: DOMAIN ERROR

⍝ ngn:193 —
10,66   ⍝ 10 66

⍝ ngn:194 —
⍬,⍬   ⍝ ⍬

⍝ ngn:195 —
⍬,1   ⍝ 1⍴1

⍝ ngn:196 —
1,⍬   ⍝ 1⍴1

⍝ ngn:197 —
'ab','c','def'   ⍝ 'abcdef'

⍝ ngn:198 —
(2 3⍴⍳6),2 2⍴⍳4   ⍝ 2 5⍴1 2 3 1 2 4 5 6 3 4

⍝ ngn:199 —
(2 3⍴⍳6),⍳2   ⍝ 2 4⍴1 2 3 1 4 5 6 2

⍝ ngn:200 —
(3 2⍴⍳6),2 2⍴⍳4
⍝ error: LENGTH ERROR

⍝ ngn:201 —
(⍳2),2 3⍴⍳6   ⍝ 2 4⍴1 1 2 3 2 4 5 6

⍝ ngn:202 —
(2 3⍴⍳6),9   ⍝ 2 4⍴1 2 3 9 4 5 6 9

⍝ ngn:203 — Uses already implemented ⎕A/⎕UCS with ordinary array operations; original independent expectation passes unchanged
(2 3 4⍴•a),'*'   ⍝ 2 3 5⍴'ABCD*EFGH*IJKL*MNOP*QRST*UVWX*'

⍝ ngn:204 —
12=12   ⍝ 1

⍝ ngn:205 —
2=12   ⍝ 0

⍝ ngn:206 —
'Q'='Q'   ⍝ 1

⍝ ngn:207 —
1='1'   ⍝ 0

⍝ ngn:208 —
'1'=1   ⍝ 0

⍝ ngn:209 —
11 7 2 9=11 3 2 6   ⍝ 1 0 1 0

⍝ ngn:210 —
4=2+2   ⍝ 1

⍝ ngn:211 —
2j3=2j3   ⍝ 1

⍝ ngn:212 —
2j3=3j2   ⍝ 0

⍝ ngn:213 —
0j0   ⍝ 0

⍝ ngn:214 —
123j0   ⍝ 123

⍝ ngn:215 —
2j¯3+¯2j3   ⍝ 0

⍝ ngn:216 —
=/⍬   ⍝ 1

⍝ ngn:217 —
'stoat'='toast'   ⍝ 0 0 0 0 1

⍝ ngn:218 —
(2 3⍴1 2 3 4 5 6)=2 3⍴3 3 3 5 5 5   ⍝ 2 3⍴0 0 1 0 1 0

⍝ ngn:219 —
3=2 3⍴1 2 3 4 5 6   ⍝ 2 3⍴0 0 1 0 0 0

⍝ ngn:220 —
3=(2 3⍴1 2 3 4 5 6)(2 3⍴3 3 3 5 5 5)   ⍝ (2 3⍴0 0 1 0 0 0) (2 3⍴1 1 1 0 0 0)

⍝ ngn:221 —
3≢5   ⍝ 1

⍝ ngn:222 —
8≠8   ⍝ 0

⍝ ngn:223 —
≠/⍬   ⍝ 0

⍝ ngn:224 —
</⍬   ⍝ 0

⍝ ngn:225 —
>/⍬   ⍝ 0

⍝ ngn:226 —
≤/⍬   ⍝ 1

⍝ ngn:227 —
≥/⍬   ⍝ 1

⍝ ngn:228 —
3≡3   ⍝ 1

⍝ ngn:229 —
3≡,3   ⍝ 0

⍝ ngn:230 —
4 7.1 8≡4 7.2 8   ⍝ 0

⍝ ngn:231 —
(3 4⍴⍳12)≡3 4⍴⍳12   ⍝ 1

⍝ ngn:232 —
(3 4⍴⍳12)≡⊂3 4⍴⍳12   ⍝ 0

⍝ ngn:233 —
('ab' 'c')≡'abc'   ⍝ 0

⍝ ngn:234 —
(2 0⍴0)≡(0 2⍴0)   ⍝ 0

⍝ ngn:235 —
≡4   ⍝ 0

⍝ ngn:236 —
≡⍳4   ⍝ 1

⍝ ngn:237 —
≡2 2⍴⍳4   ⍝ 1

⍝ ngn:238 —
≡'abc'1 2 3(23 55)   ⍝ ¯2

⍝ ngn:239 —
≡'abc'(2 4⍴'abc'2 3'k')   ⍝ ¯3

⍝ ngn:240 —
8(÷∘-)2   ⍝ ¯4

⍝ ngn:241 —
÷∘-2   ⍝ ¯0.5

⍝ ngn:242 —
8÷∘-2   ⍝ ¯4

⍝ ngn:243 —
⍴∘⍴2 3⍴⍳6   ⍝ 1⍴2

⍝ ngn:244 —
3⍴∘⍴2 3⍴⍳6   ⍝ 2 3 2

⍝ ngn:245 —
3∘-1   ⍝ 2

⍝ ngn:246 —
(-∘2)9   ⍝ 7

⍝ ngn:247 —
1 2∪2 3   ⍝ 1 2 3

⍝ ngn:248 —
'abc'∪'cad'   ⍝ 'abcd'

⍝ ngn:249 —
1∪1   ⍝ 1⍴1

⍝ ngn:250 —
1∪2   ⍝ 1 2

⍝ ngn:251 —
1∪2 1   ⍝ 1 2

⍝ ngn:252 —
1 2∪2 2 2 2   ⍝ 1 2

⍝ ngn:253 —
2 3 3∪4 5 3 4   ⍝ 2 3 3 4 5 4

⍝ ngn:254 —
⍬∪1   ⍝ 1⍴1

⍝ ngn:255 —
1 2∪⍬   ⍝ 1 2

⍝ ngn:256 —
⍬∪⍬   ⍝ ⍬

⍝ ngn:257 —
1 2∪2 2⍴3
⍝ error: RANK ERROR

⍝ ngn:258 —
(2 2⍴3)∪4 5
⍝ error: RANK ERROR

⍝ ngn:259 —
'ab' 'c'(0 1)∪'ab' 'de'   ⍝ ('ab') 'c' (0 1) ('de')

⍝ ngn:260 —
∪3 17   ⍝ 3 17

⍝ ngn:261 —
∪⍬   ⍝ ⍬

⍝ ngn:262 —
∪17   ⍝ 1⍴17

⍝ ngn:263 —
∪3 17 17 17 ¯3 17 0   ⍝ 3 17 ¯3 0

⍝ ngn:264 —
'abca'∩'dac'   ⍝ 'aca'

⍝ ngn:265 —
1'2'3∩⍳5   ⍝ 1 3

⍝ ngn:266 —
1∩2   ⍝ ⍬

⍝ ngn:267 —
1∩2 3⍴4
⍝ error: RANK ERROR

⍝ ngn:268 —
∩1
⍝ error: SYNTAX ERROR

⍝ ngn:269 —
10⊥3 2 6 9   ⍝ 3269

⍝ ngn:270 —
8⊥3 1   ⍝ 25

⍝ ngn:271 —
1760 3 12⊥1 2 8   ⍝ 68

⍝ ngn:272 —
2 2 2⊥1   ⍝ 7

⍝ ngn:273 —
0 20 12 4⊥2 15 6 3   ⍝ 2667

⍝ ngn:274 —
1760 3 12⊥3 3⍴1 1 1 2 0 3 0 1 8   ⍝ 60 37 80

⍝ ngn:275 —
60 60⊥3 13   ⍝ 193

⍝ ngn:276 —
0 60⊥3 13   ⍝ 193

⍝ ngn:277 —
60⊥3 13   ⍝ 193

⍝ ngn:278 —
2⊥1 0 1 0   ⍝ 10

⍝ ngn:279 —
2⊥1 2 3 4   ⍝ 26

⍝ ngn:280 —
3⊥1 2 3 4   ⍝ 58

⍝ ngn:281 —
2j3⊤4j5 6j7 8j9   ⍝ 2j2 2j1 ¯1j2

⍝ ngn:282 —
10⊥3 4.5j1   ⍝ 34.5j1

⍝ ngn:283 —
(4 3⍴1 1 1 2 2 2 3 3 3 4 4 4)⊥3 8⍴0 0 0 0 1 1 1 1 0 0 1 1 0 0 1 1 0 1 0 1 0 1 0 1
4 8⍴0 1 1 2 1 2 2 3 0 1 2 3 4 5 6 7 0 1 3 4 9 10 12 13 0 1 4 5 16 17 20 21

⍝ ngn:284 —
2⊥3 8⍴0 0 0 0 1 1 1 1 0 0 1 1 0 0 1 1 0 1 0 1 0 1 0 1
0 1 2 3 4 5 6 7

⍝ ngn:285 —
(2 1⍴2 10)⊥3 8 ⍴0 0 0 0 1 1 1 1 0 0 1 1 0 0 1 1 0 1 0 1 0 1 0 1
2 8⍴0 1 2 3 4 5 6 7 0 1 10 11 100 101 110 111

⍝ ngn:286 —
2 3 4 ×⌝ 1 2 3 4   ⍝ 3 4⍴2 4 6 8 3 6 9 12 4 8 12 16

⍝ ngn:287 —
0 1 2 3 4 !⌝ 0 1 2 3 4
5 5⍴1 1 1 1 1 0 1 2 3 4 0 0 1 3 6 0 0 0 1 4 0 0 0 0 1

⍝ ngn:288 —
1 2 ,⌝ 1+⍳3   ⍝ 2 3⍴(1 2) (1 3) (1 4) (2 2) (2 3) (2 4)

⍝ ngn:289 —
2 3 ↑⌝ 1 2   ⍝ 2 2⍴(1 0) (2 0) (1 0 0) (2 0 0)

⍝ ngn:290 —
⍴1 2 ,⌝ 1+⍳3   ⍝ 2 3

⍝ ngn:291 —
⍴2 3 ↑⌝ 1 2   ⍝ 2 2

⍝ ngn:292 —
⍴((4 3⍴0) +⌝ 5 2⍴0)   ⍝ 4 3 5 2

⍝ ngn:293 —
2 3 ×⌝ 4 5   ⍝ 2 2⍴8 10 12 15

⍝ ngn:294 —
2 3 × ⌝ 4 5   ⍝ 2 2⍴8 10 12 15

⍝ ngn:295 —
2 3 {⍺×⍵}⌝ 4 5   ⍝ 2 2⍴8 10 12 15

⍝ ngn:296 —
1 3 5 7+.=2 3 6 7   ⍝ 2

⍝ ngn:297 —
7+.=8 8 7 7 8 7 5   ⍝ 3

⍝ ngn:298 —
1 3 5 7∧.=2 3 6 7   ⍝ 0

⍝ ngn:299 —
8 8 7 7 8 7 5+.=7   ⍝ 3

⍝ ngn:300 —
1 3 5 7∧.=1 3 5 7   ⍝ 1

⍝ ngn:301 —
7+.=7   ⍝ 1

⍝ ngn:302 —
(3 2⍴5 ¯3 ¯2 4 ¯1 0)+.×2 2⍴6 ¯3 5 7   ⍝ 3 2⍴15 ¯36 8 34 ¯6 3

⍝ ngn:303 —
⍴¨(⍳4)(0 0 0)   ⍝ (1⍴4) (1⍴3)

⍝ ngn:304 —
⍴¨'ab' 'cde' 'f'   ⍝ (1⍴2) (1⍴3) (⍬)

⍝ ngn:305 —
⍴   (2 2⍴⍳4)(⍳10)97.3(3 4⍴'K')   ⍝ 1⍴4

⍝ ngn:306 —
⍴¨  (2 2⍴⍳4)(⍳10)97.3(3 4⍴'K')   ⍝ (2 2) (1⍴10) (⍬) (3 4)

⍝ ngn:307 —
⍴⍴¨ (2 2⍴⍳4)(⍳10)97.3(3 4⍴'K')   ⍝ 1⍴4

⍝ ngn:308 —
⍴¨⍴¨(2 2⍴⍳4)(⍳10)97.3(3 4⍴'K')   ⍝ (1⍴2) (1⍴1) (1⍴0) (1⍴2)

⍝ ngn:309 —
1 2 3,¨4 5 6   ⍝ (1 4) (2 5) (3 6)

⍝ ngn:310 —
2 3↑¨'monday' 'tuesday'   ⍝ ('mo') ('tue')

⍝ ngn:311 —
2↑¨'monday' 'tuesday'   ⍝ ('mo') ('tu')

⍝ ngn:312 —
2 3⍴¨1 2   ⍝ (1 1) (2 2 2)

⍝ ngn:313 —
4 5⍴¨'the' 'cat'   ⍝ ('thet') ('catca')

⍝ ngn:314 —
{1+⍵*2}¨2 3⍴⍳6   ⍝ 2 3⍴2 5 10 17 26 37

⍝ ngn:315 —
1760 3 12⊤75   ⍝ 2 0 3

⍝ ngn:316 —
3 12⊤75   ⍝ 0 3

⍝ ngn:317 —
100000 12⊤75   ⍝ 6 3

⍝ ngn:318 —
16 16 16 16⊤100   ⍝ 0 0 6 4

⍝ ngn:319 —
1760 3 12⊤75.3   ⍝ 2 0 3.299999999999997

⍝ ngn:320 — [rtol=1e-14]
0 1⊤75.3   ⍝ 75 0.2999999999999972

⍝ ngn:321 —
2 2 2 2 2⊤1 2 3 4 5
5 5⍴0 0 0 0 0 0 0 0 0 0 0 0 0 1 1 0 1 1 0 0 1 0 1 0 1

⍝ ngn:322 —
10⊤5 15 125   ⍝ 5 5 5

⍝ ngn:323 —
0 10⊤5 15 125   ⍝ 2 3⍴0 1 12 5 5 5

⍝ ngn:324 —
0j1 2j3 4j5⊤6j7   ⍝ 0 ¯2j2 2j2

⍝ ngn:325 —
(8 3⍴2 0 0 2 0 0 2 0 0 2 0 0 2 8 0 2 8 0 2 8 16 2 8 16)⊤75
8 3⍴0 0 0 1 0 0 0 0 0 0 0 0 1 0 0 0 1 0 1 1 4 1 3 11

⍝ ngn:326 —
2 3 4 5 6∊1 2 3 5 8 13 21   ⍝ 1 1 0 1 0

⍝ ngn:327 —
5∊1 2 3 5 8 13 21   ⍝ 1

⍝ ngn:328 —
∊17   ⍝ 1⍴17

⍝ ngn:329 —
⍴∊(1 2 3)'ab'(4 5 6)   ⍝ 1⍴8

⍝ ngn:330 —
∊2 2⍴(1+2 2⍴⍳4)'ab'(1+2 3⍴⍳6)(7 8)   ⍝ 2 3 4 5 'a' 'b' 2 3 4 5 6 7 7 8

⍝ ngn:331 —
!0 5 21   ⍝ 1 120 5.109094217170944e19

⍝ ngn:332 —
!1.5 ¯1.5 ¯2.5
1.329340388179137 ¯3.544907701811032 2.363271801207354

⍝ ngn:333 —
!¯200.5   ⍝ 0

⍝ ngn:334 —
!¯1
⍝ error: DOMAIN ERROR

⍝ ngn:335 —
!¯200
⍝ error: DOMAIN ERROR

⍝ ngn:336 —
2!4   ⍝ 6

⍝ ngn:337 —
3!20   ⍝ 1140

⍝ ngn:338 —
2!6 12 20   ⍝ 15 66 190

⍝ ngn:339 —
(2 3⍴1+⍳6)!2 3⍴3 6 9 12 15 18   ⍝ 2 3⍴3 20 126 792 5005 31824

⍝ ngn:340 — [rtol=1e-14]
0.5!1   ⍝ 1.273239544735163

⍝ ngn:341 — [rtol=1e-14]
1.2!3.4   ⍝ 3.795253463731265

⍝ ngn:342 —
!/⍬   ⍝ 1

⍝ ngn:343 —
(2!1000)=499500   ⍝ 1

⍝ ngn:344 —
(998!1000)=499500   ⍝ 1

⍝ ngn:345 —
0.5!¯1
⍝ error: DOMAIN ERROR

⍝ ngn:346 —
3!5   ⍝ 10

⍝ ngn:347 —
5!3   ⍝ 0

⍝ ngn:348 —
3!¯5   ⍝ ¯35

⍝ ngn:349 —
¯3!5   ⍝ 0

⍝ ngn:350 —
¯5!¯3   ⍝ 6

⍝ ngn:351 —
¯3!¯5   ⍝ 0

⍝ ngn:352 — Reviewed Execute example checked through the Rust reference worker
⍎'+/2 2⍴1 2 3 4'   ⍝ 3 7

⍝ ngn:353 — Reviewed Execute example checked through the Rust reference worker
⍴⍎'123 456'   ⍝ 1⍴2

⍝ ngn:354 — Reviewed Execute example checked through the Rust reference worker; Subtract one from iota to preserve the zero-origin upstream values inside Execute
⍎'{⍵*2}¯1+⍳5'   ⍝ 0 1 4 9 16

⍝ ngn:355 — Reviewed Execute example checked through the Rust reference worker
⍎'let'
⍝ error: VALUE ERROR

⍝ ngn:356 — Reviewed Execute example checked through the Rust reference worker
⍎'1 2 (3'
⍝ error: SYNTAX ERROR

⍝ ngn:357 —
'ab'⍷'bababc'   ⍝ 0 1 0 1 0 0

⍝ ngn:358 —
'ab' 'cde'⍷'ab' 'cde' 'fg'   ⍝ 1 0 0

⍝ ngn:359 —
'cd'⍷'abcd efghi'   ⍝ 0 0 1 0 0 0 0 0 0 0

⍝ ngn:360 —
'day'⍷7 9⍴'sunday   monday   tuesday  wednesdaythursday friday   saturday '
7 9⍴0 0 0 1 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 1 0 0 0

⍝ ngn:361 —
(2 2⍴'abcd')⍷'abcd'   ⍝ 0 0 0 0

⍝ ngn:362 —
(1 2)(3 4)⍷'start'(1 2 3)(1 2)(3 4)   ⍝ 0 0 1 0

⍝ ngn:363 —
(2 2⍴7 8 12 13)⍷1+4 5⍴⍳20
4 5⍴0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0

⍝ ngn:364 —
1⍷⍳5   ⍝ 1 0 0 0 0

⍝ ngn:365 —
1 2⍷⍳5   ⍝ 1 0 0 0 0

⍝ ngn:366 —
⍬⍷⍳5   ⍝ 1 1 1 1 1

⍝ ngn:367 —
⍬⍷⍬   ⍝ ⍬

⍝ ngn:368 —
1⍷⍬   ⍝ ⍬

⍝ ngn:369 —
1 2 3⍷⍬   ⍝ ⍬

⍝ ngn:370 —
(2 3 0⍴0)⍷3 4 5⍴0
3 4 5⍴1 1 1 1 1 1 1 1 1 1 0 0 0 0 0 0 0 0 0 0 1 1 1 1 1 1 1 1 1 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0

⍝ ngn:371 —
(2 3 4⍴0)⍷3 4 0⍴0   ⍝ 3 4 0⍴0

⍝ ngn:372 —
(2 3 0⍴0)⍷3 4 0⍴0   ⍝ 3 4 0⍴0

⍝ ngn:373 —
⌊123 12.3 ¯12.3 ¯123   ⍝ 123 12 ¯13 ¯123

⍝ ngn:374 —
⌊12j3 1.2j2.3 1.2j¯2.3 ¯1.2j2.3 ¯1.2j¯2.3
12j3 1j2 1j¯3 ¯1j2 ¯1j¯3

⍝ ngn:375 —
⌊0 5 ¯5 (π1) ¯1.5   ⍝ 0 5 ¯5 3 ¯2

⍝ ngn:376 —
⌊'a'
⍝ error: DOMAIN ERROR

⍝ ngn:377 —
3⌊5   ⍝ 3

⍝ ngn:378 — Agreed real infinity literal/empty-reduction identity; Real infinity replaces the earlier finite-identity adaptation
⌊/⍬   ⍝ ∞

⍝ ngn:379 —
⌈123 12.3 ¯12.3 ¯123   ⍝ 123 13 ¯12 ¯123

⍝ ngn:380 —
⌈12j3 1.2j2.3 1.2j¯2.3 ¯1.2j2.3 ¯1.2j¯2.3
12j3 1j3 1j¯2 ¯1j3 ¯1j¯2

⍝ ngn:381 —
⌈0 5 ¯5(π1)¯1.5   ⍝ 0 5 ¯5 4 ¯1

⍝ ngn:382 —
⌈'a'
⍝ error: DOMAIN ERROR

⍝ ngn:383 —
3⌈5   ⍝ 5

⍝ ngn:384 — Agreed real infinity literal/empty-reduction identity; Real infinity replaces the earlier finite-identity adaptation
⌈/⍬   ⍝ ¯∞

⍝ ngn:385 —
(+/÷⍴)4 5 10 7   ⍝ 1⍴6.5

⍝ ngn:386 —
(+,-,×,÷)2   ⍝ 2 ¯2 1 0.5

⍝ ngn:387 —
1(+,-,×,÷)2   ⍝ 3 ¯1 2 0.5

⍝ ngn:388 — ordinary identifier for quadratic-root helper
a←1⋄b←¯22⋄c←85⋄sqrt←{⍵*.5}⋄((-b)(+,-)sqrt(b*2)-4×a×c)÷2×a
17 5

⍝ ngn:389 —
⍕123   ⍝ '123'

⍝ ngn:390 —
⍕123 456   ⍝ '123 456'

⍝ ngn:391 —
⍕123 'a'   ⍝ '123 a'

⍝ ngn:392 —
⍕12 'ab'   ⍝ '12  ab '

⍝ ngn:393 —
⍕1 2⍴'a'   ⍝ 1 2⍴'aa'

⍝ ngn:394 —
⍕2 2⍴'a'   ⍝ 2 2⍴'aaaa'

⍝ ngn:395 —
⍕2 2⍴5   ⍝ 2 3⍴'5 55 5'

⍝ ngn:396 —
⍕2 2⍴0 0 0 'a'   ⍝ 2 3⍴'0 00 a'

⍝ ngn:397 —
⍕2 2⍴0 0 0 'ab'   ⍝ 2 6⍴'0   0 0  ab '

⍝ ngn:398 —
⍕2 2⍴0 0 0 123   ⍝ 2 5⍴'0   00 123'

⍝ ngn:399 —
⍕4 3⍴'---' '---' '---' 1 2 3 4 5 6 100 200 300
4 15⍴' ---  ---  ---    1    2    3    4    5    6  100  200  300 '

⍝ ngn:400 —
⍕1 ⍬ 2 '' 3   ⍝ '1    2    3'

⍝ ngn:401 — Infinity formatting; basedpl scalar format is a character vector, not the ngn matrix
⍕∞   ⍝ 1⍴'∞'

⍝ ngn:402 — Infinity formatting; basedpl scalar format is a character vector, not the ngn matrix
⍕¯∞   ⍝ '¯∞'

⍝ ngn:403 —
⍕¯1   ⍝ '¯1'

⍝ ngn:404 — Dyalog monadic-format result with basedpl lowercase j; original expectation retained
⍕¯1e¯100J¯2e¯99   ⍝ '¯1E¯100j¯2E¯99'

⍝ ngn:405 —
⍋13 8 122 4   ⍝ 4 2 1 3

⍝ ngn:406 —
a←13 8 122 4⋄a[⍋a]   ⍝ 4 8 13 122

⍝ ngn:407 —
⍋'ZAMBIA'   ⍝ 2 6 4 5 3 1

⍝ ngn:408 —
s←'ZAMBIA'⋄s[⍋s]   ⍝ 'AABIMZ'

⍝ ngn:409 —
t←3 3⍴'BOBALFZAK'⋄⍋t   ⍝ 2 1 3

⍝ ngn:410 —
t←3 3⍴4 5 6 1 1 3 1 1 2⋄⍋t   ⍝ 3 2 1

⍝ ngn:411 —
t←3 3⍴4 5 6 1 1 3 1 1 2⋄t[⍋t;]   ⍝ 3 3⍴1 1 2 1 1 3 4 5 6

⍝ ngn:412 —
a←3 2 3⍴2 3 4 0 1 0 1 1 3 4 5 6 1 1 2 10 11 12⋄a[⍋a;;]
3 2 3⍴1 1 2 10 11 12 1 1 3 4 5 6 2 3 4 0 1 0

⍝ ngn:413 —
a←3 2 5⍴'joe  doe  bob  jonesbob  zwart'⋄a[⍋a;;]
3 2 5⍴'bob  jonesbob  zwartjoe  doe  '

⍝ ngn:414 —
'ZYXWVUTSRQPONMLKJIHGFEDCBA'⍋'ZAMBIA'   ⍝ 1 3 5 4 2 6

⍝ ngn:415 — Fixed origin/constants; omit irrelevant PP assignment and constant rebinding; glyph-only formatting where needed
(⌽•A)⍋3 3⍴'BOBALFZAK'   ⍝ 3 1 2

⍝ ngn:416 —
a←6 4⍴'ABLEaBLEACREABELaBELACES'⋄a[(2 26⍴'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz')⍋a;]
6 4⍴'ABELaBELABLEaBLEACESACRE'

⍝ ngn:417 —
a←6 4⍴'ABLEaBLEACREABELaBELACES'⋄a[('AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUuVvWwXxYyZz')⍋a;]
6 4⍴'ABELABLEACESACREaBELaBLE'

⍝ ngn:418 —
⍋0 1 2 3 4 3 6 6 4 9 1 11 12 13 14 15   ⍝ 1 2 11 3 4 6 5 9 7 8 10 12 13 14 15 16

⍝ ngn:419 —
⍒3 1 8   ⍝ 3 1 2

⍝ ngn:420 —
f←{⍺+2×⍵}⋄f/⍬
⍝ error: DOMAIN ERROR

⍝ ngn:426 —
2 5 9 14 20⍳9   ⍝ 3

⍝ ngn:427 —
2 5 9 14 20⍳6   ⍝ 6

⍝ ngn:428 —
'abcde'⍳'d'   ⍝ 4

⍝ ngn:429 — Fixed-origin-1 index-of port; ⎕A is implemented
•a⍳'NGN/'   ⍝ 14 7 14 27

⍝ ngn:430 —
'ab' 'cd' 'efg'⍳'cd' 'efh'   ⍝ 2 4

⍝ ngn:431 —
1 3 2 0 3⍳⍳5   ⍝ 1 3 2 6 6

⍝ ngn:432 —
'cat' 'dog' 'mouse'⍳'dog' 'bird'   ⍝ 2 4

⍝ ngn:433 —
1⍳1
⍝ error: RANK ERROR

⍝ ngn:434 —
(1 2⍴3)⍳3
⍝ error: RANK ERROR

⍝ ngn:435 —
1 1⍳1   ⍝ 1

⍝ ngn:436 —
⍬⍳1 2   ⍝ 1 1

⍝ ngn:437 —
1 2⍳⍬   ⍝ ⍬

⍝ ngn:438 —
⍳5   ⍝ 1 2 3 4 5

⍝ ngn:439 —
⍴⍳5   ⍝ 1⍴5

⍝ ngn:440 —
⍳0   ⍝ ⍬

⍝ ngn:441 —
⍴⍳0   ⍝ 1⍴0

⍝ ngn:442 —
⍴⍳2 3 4   ⍝ 2 3 4

⍝ ngn:443 —
⍳¯1
⍝ error: DOMAIN ERROR

⍝ ngn:444 —
⍳2 3 4
2 3 4⍴(1 1 1) (1 1 2) (1 1 3) (1 1 4) (1 2 1) (1 2 2) (1 2 3) (1 2 4) (1 3 1) (1 3 2) (1 3 3) (1 3 4) (2 1 1) (2 1 2) (2 1 3) (2 1 4) (2 2 1) (2 2 2) (2 2 3) (2 2 4) (2 3 1) (2 3 2) (2 3 3) (2 3 4)

⍝ ngn:445 —
⍴⊂2 3⍴⍳6   ⍝ ⍬

⍝ ngn:446 —
⍴⍴⊂2 3⍴⍳6   ⍝ 1⍴0

⍝ ngn:447 —
⊂[1]2 3⍴⍳6   ⍝ (1 4) (2 5) (3 6)

⍝ ngn:448 —
⍴⊂[1]2 3⍴⍳6   ⍝ 1⍴3

⍝ ngn:449 —
⊂[2]2 3⍴⍳6   ⍝ (1 2 3) (4 5 6)

⍝ ngn:450 —
⍴⊂[2]2 3⍴⍳6   ⍝ 1⍴2

⍝ ngn:451 —
↑⊂[2 1]2 3⍴⍳6   ⍝ 3 2⍴1 4 2 5 3 6

⍝ ngn:452 —
⍴⊂[2 1]2 3⍴⍳6   ⍝ ⍬

⍝ ngn:453 —
⍴↑⊂⊂1 2 3   ⍝ ⍬

⍝ ngn:454 —
0 0 1 1∨0 1 0 1   ⍝ 0 1 1 1

⍝ ngn:455 —
12∨18   ⍝ 6

⍝ ngn:456 —
299∨323   ⍝ 1

⍝ ngn:457 —
12345∨12345   ⍝ 12345

⍝ ngn:458 —
0∨123   ⍝ 123

⍝ ngn:459 —
123∨0   ⍝ 123

⍝ ngn:460 —
∨/⍬   ⍝ 0

⍝ ngn:461 —
¯12∨18   ⍝ 6

⍝ ngn:462 —
12∨¯18   ⍝ 6

⍝ ngn:463 —
¯12∨¯18   ⍝ 6

⍝ ngn:464 — Allow floating Gaussian GCD/LCM rounding; retain independent upstream mathematical result [rtol=1e-14]
135j¯14∨155j34   ⍝ 5j12

⍝ ngn:465 —
2 3 4∨0j1 1j2 2j3   ⍝ 1 1 1

⍝ ngn:466 —
2j2 2j4∨5j5 4j4   ⍝ 1j1 2

⍝ ngn:467 —
1.5∨2.5   ⍝ 0.5

⍝ ngn:468 —
'a'∨1
⍝ error: DOMAIN ERROR

⍝ ngn:469 —
1∨'a'
⍝ error: DOMAIN ERROR

⍝ ngn:470 —
'a'∨'b'
⍝ error: DOMAIN ERROR

⍝ ngn:471 —
0 0 1 1∧0 1 0 1   ⍝ 0 0 0 1

⍝ ngn:472 —
1∧3 3⍴1 1 1 0 0 0 1 0 1   ⍝ 3 3⍴1 1 1 0 0 0 1 0 1

⍝ ngn:473 —
∧/3 3⍴1 1 1 0 0 0 1 0 1   ⍝ 1 0 0

⍝ ngn:474 —
12∧18   ⍝ 36

⍝ ngn:475 —
299∧323   ⍝ 96577

⍝ ngn:476 —
123∧123   ⍝ 123

⍝ ngn:477 —
0∧123   ⍝ 0

⍝ ngn:478 —
123∧0   ⍝ 0

⍝ ngn:479 —
∧/⍬   ⍝ 1

⍝ ngn:480 —
¯12∧18   ⍝ ¯36

⍝ ngn:481 —
12∧¯18   ⍝ ¯36

⍝ ngn:482 —
¯12∧¯18   ⍝ 36

⍝ ngn:483 —
1.5∧2.5   ⍝ 7.5

⍝ ngn:484 —
'a'∧1
⍝ error: DOMAIN ERROR

⍝ ngn:485 —
1∧'a'
⍝ error: DOMAIN ERROR

⍝ ngn:486 —
'a'∧'b'
⍝ error: DOMAIN ERROR

⍝ ngn:487 — Allow floating Gaussian GCD/LCM rounding; retain independent upstream mathematical result [rtol=1e-14]
135j¯14∧155j34   ⍝ 805j¯1448

⍝ ngn:488 —
2 3 4∧0j1 1j2 2j3   ⍝ 0j2 3j6 8j12

⍝ ngn:489 —
2j2 2j4∧5j5 4j4   ⍝ 10j10 ¯4j12

⍝ ngn:490 —
0 0 1 1⍱0 1 0 1   ⍝ 1 0 0 0

⍝ ngn:491 —
0⍱2
⍝ error: DOMAIN ERROR

⍝ ngn:492 —
0 0 1 1⍲0 1 0 1   ⍝ 1 1 1 0

⍝ ngn:493 —
0⍲2
⍝ error: DOMAIN ERROR

⍝ ngn:494 —
~0 1   ⍝ 1 0

⍝ ngn:495 —
~2
⍝ error: DOMAIN ERROR

⍝ ngn:496 —
({⍵+1}⍣5)3   ⍝ 8

⍝ ngn:497 —
({⍵+1}⍣0)3   ⍝ 3

⍝ ngn:498 —
(⍴⍣3)2 2⍴⍳4   ⍝ 1⍴1

⍝ ngn:499 —
'a'(,⍣3)'b'   ⍝ 'aaab'

⍝ ngn:500 —
1{⍺+÷⍵}⍣=1   ⍝ 1.618033988749897

⍝ ngn:507 —
•UCS'a'   ⍝ 97

⍝ ngn:508 —
•UCS'ab'   ⍝ 97 98

⍝ ngn:509 — Offset adjusted for fixed origin 1; retain upstream character expectation
•UCS 2 2⍴96+⍳4   ⍝ 2 2⍴'abcd'

⍝ ngn:510 —
n←6⋄r←?n⋄(1≤r)∧(r≤n)   ⍝ 1

⍝ ngn:511 — Fixed origin 1 single possible result
?1   ⍝ 1

⍝ ngn:512 —
r←?0⋄(0≤r)∧r<1   ⍝ 1

⍝ ngn:513 —
?1.5
⍝ error: DOMAIN ERROR

⍝ ngn:514 —
?'a'
⍝ error: DOMAIN ERROR

⍝ ngn:515 —
?1j2
⍝ error: DOMAIN ERROR

⍝ ngn:516 — Infinity is not an integer bound for roll; original DOMAIN ERROR retained
?∞
⍝ error: DOMAIN ERROR

⍝ ngn:517 — a permutation (an 'n?n' dealing) contains all 0...n
n←100⋄(+/n?n)=(+/⍳n)   ⍝ 1

⍝ ngn:518 —
n←100⋄A←(n÷2)?n⋄∧/(1≤A),A≤n   ⍝ 1

⍝ ngn:519 —
0?100   ⍝ ⍬

⍝ ngn:520 —
0?0   ⍝ ⍬

⍝ ngn:521 — Fixed origin 1 single possible result
1?1   ⍝ 1⍴1

⍝ ngn:522 —
1?1 1
⍝ error: LENGTH ERROR

⍝ ngn:523 —
5?3
⍝ error: DOMAIN ERROR

⍝ ngn:524 —
¯1?3
⍝ error: DOMAIN ERROR

⍝ ngn:526 —
2 5⍴¨⊂1 2 3   ⍝ (1 2) (1 2 3 1 2)

⍝ ngn:527 —
⍴1 2 3⍴0   ⍝ 1 2 3

⍝ ngn:528 —
⍴⍴1 2 3⍴0   ⍝ 1⍴3

⍝ ngn:529 —
2 3⍴⍳5   ⍝ 2 3⍴1 2 3 4 5 1

⍝ ngn:530 —
⍬⍴123   ⍝ ⊂123

⍝ ngn:531 —
⍬⍴⍬   ⍝ ⊂0

⍝ ngn:532 —
2 3⍴⍬   ⍝ 2 3⍴0 0 0 0 0 0

⍝ ngn:533 —
2 3⍴⍳7   ⍝ 2 3⍴1 2 3 4 5 6

⍝ ngn:534 —
⍴0 0   ⍝ 1⍴2

⍝ ngn:535 —
⍴⍴0   ⍝ 1⍴0

⍝ ngn:536 —
⍴⍴⍴0   ⍝ 1⍴1

⍝ ngn:537 —
⍴⍴⍴0 0   ⍝ 1⍴1

⍝ ngn:538 —
⍴'a'   ⍝ ⍬

⍝ ngn:539 —
⍴'ab'   ⍝ 1⍴2

⍝ ngn:540 —
⍴2 3 4⍴0   ⍝ 2 3 4

⍝ ngn:541 —
1⌽1 2 3 4 5 6   ⍝ 2 3 4 5 6 1

⍝ ngn:542 —
3⌽'abcdefgh'   ⍝ 'defghabc'

⍝ ngn:543 —
3⌽2 5⍴1 2 3 4 5 6 7 8 9 0   ⍝ 2 5⍴4 5 1 2 3 9 0 6 7 8

⍝ ngn:544 —
¯2⌽'abcdefgh'   ⍝ 'ghabcdef'

⍝ ngn:545 —
1⌽3 3⍴⍳9   ⍝ 3 3⍴2 3 1 5 6 4 8 9 7

⍝ ngn:546 —
0⌽1 2 3 4   ⍝ 1 2 3 4

⍝ ngn:547 —
0⌽1234   ⍝ 1234

⍝ ngn:548 —
5⌽⍬   ⍝ ⍬

⍝ ngn:549 —
⌽1 2 3 4 5 6   ⍝ 6 5 4 3 2 1

⍝ ngn:550 —
⌽(1 2)(3 4)(5 6)   ⍝ (5 6) (3 4) (1 2)

⍝ ngn:551 —
⌽'bob won pots'   ⍝ 'stop now bob'

⍝ ngn:552 —
⌽2 5⍴1 2 3 4 5 6 7 8 9 0   ⍝ 2 5⍴5 4 3 2 1 0 9 8 7 6

⍝ ngn:553 —
⌽[1]2 5⍴1 2 3 4 5 6 7 8 9 0   ⍝ 2 5⍴6 7 8 9 0 1 2 3 4 5

⍝ ngn:554 —
⊖1 2 3 4 5 6   ⍝ 6 5 4 3 2 1

⍝ ngn:555 —
⊖(1 2)(3 4)(5 6)   ⍝ (5 6) (3 4) (1 2)

⍝ ngn:556 —
⊖'bob won pots'   ⍝ 'stop now bob'

⍝ ngn:557 —
⊖2 5⍴1 2 3 4 5 6 7 8 9 0   ⍝ 2 5⍴6 7 8 9 0 1 2 3 4 5

⍝ ngn:558 —
⊖[2]2 5⍴1 2 3 4 5 6 7 8 9 0   ⍝ 2 5⍴5 4 3 2 1 0 9 8 7 6

⍝ ngn:559 —
1⊖3 3⍴⍳9   ⍝ 3 3⍴4 5 6 7 8 9 1 2 3

⍝ ngn:560 —
+/3   ⍝ 3

⍝ ngn:561 —
+/3 5 8   ⍝ 16

⍝ ngn:562 —
⌈/82 66 93 13   ⍝ 93

⍝ ngn:563 —
×/2 3⍴1 2 3 4 5 6   ⍝ 6 120

⍝ ngn:564 —
-/3 0⍴42   ⍝ 0 0 0

⍝ ngn:565 —
,/2↕'ab' 'cd' 'ef' 'hi'   ⍝ ('abcd') ('cdef') ('efhi')

⍝ ngn:566 —
,/3↕'ab' 'cd' 'ef' 'hi'   ⍝ ('abcdef') ('cdefhi')

⍝ ngn:567 —
+/2↕1+⍳5   ⍝ 5 7 9 11

⍝ ngn:568 —
+/5↕1+⍳8   ⍝ 20 25 30 35

⍝ ngn:569 —
+/10↕1+⍳10   ⍝ 1⍴65

⍝ ngn:570 —
+/11↕1+⍳10   ⍝ ⍬

⍝ ngn:571 — Oversized windows have an empty frame
+/12↕1+⍳10   ⍝ ⍬

⍝ ngn:572 —
-/2↕3 4 9 7   ⍝ ¯1 ¯5 2

⍝ ngn:573 —
-/⌽2↕3 4 9 7   ⍝ 1 5 ¯2

⍝ ngn:574 —
0 1 0 1/'abcd'   ⍝ 'bd'

⍝ ngn:575 —
m←45 60 33 50 66 19⋄(m≥50)/m   ⍝ 60 50 66

⍝ ngn:576 —
1/'ab'   ⍝ 'ab'

⍝ ngn:577 —
1 1 1 1 0/12 14 16 18 20   ⍝ 12 14 16 18

⍝ ngn:578 —
m←45 60 33 50 66 19⋄(m=50)/⍳≢m   ⍝ 1⍴4

⍝ ngn:579 —
0/'ab'   ⍝ ''

⍝ ngn:580 —
0 1 0/1+2 3⍴⍳6   ⍝ 2 1⍴3 6

⍝ ngn:581 —
1 0/[1]1+2 3⍴⍳6   ⍝ 1 3⍴2 3 4

⍝ ngn:582 —
1 0⌿1+2 3⍴⍳6   ⍝ 1 3⍴2 3 4

⍝ ngn:583 —
3/5   ⍝ 5 5 5

⍝ ngn:584 —
2 ¯2 2/1+2 3⍴⍳6   ⍝ 2 6⍴2 2 0 0 4 4 5 5 0 0 7 7

⍝ ngn:585 —
1 1 ¯2 1 1/1 2(2 2⍴⍳4)3 4   ⍝ 1 2 0 0 3 4

⍝ ngn:586 —
2 3 2/'abc'   ⍝ 'aabbbcc'

⍝ ngn:587 —
2/'def'   ⍝ 'ddeeff'

⍝ ngn:588 —
5 0 5/1 2 3   ⍝ 1 1 1 1 1 3 3 3 3 3

⍝ ngn:589 —
2/1+2 3⍴⍳6   ⍝ 2 6⍴2 2 3 3 4 4 5 5 6 6 7 7

⍝ ngn:590 —
2⌿1+2 3⍴⍳6   ⍝ 4 3⍴2 3 4 2 3 4 5 6 7 5 6 7

⍝ ngn:591 —
2 ¯1 2/[2]3 1⍴7 8 9   ⍝ 3 5⍴7 7 0 7 7 8 8 0 8 8 9 9 0 9 9

⍝ ngn:592 —
2 ¯1 2/[2]3 1⍴'abc'   ⍝ 3 5⍴'aa aabb bbcc cc'

⍝ ngn:593 —
2 ¯2 2/7   ⍝ 7 7 0 0 7 7

⍝ ngn:594 —
2 3/3 1⍴'abc'   ⍝ 3 5⍴'aaaaabbbbbccccc'

⍝ ngn:595 —
2⌷3 5 8   ⍝ 5

⍝ ngn:596 —
(3 5 8)[2]   ⍝ 5

⍝ ngn:597 —
(3 5 8)[⍬]   ⍝ ⍬

⍝ ngn:598 —
(3 3 1)(2 3)⌷3 3⍴⍳9   ⍝ 3 2⍴8 9 8 9 2 3

⍝ ngn:599 —
¯1⌷3 5 8
⍝ error: INDEX ERROR

⍝ ngn:600 —
(⊂2 3⍴3 1 4 1 2 3)⌷111 222 333 444   ⍝ 2 3⍴333 111 444 111 222 333

⍝ ngn:601 —
2 1   ⌷3 4⍴11 12 13 14 21 22 23 24 31 32 33 34
21

⍝ ngn:602 —
3⌷111 222 333 444   ⍝ 333

⍝ ngn:603 —
2     ⌷3 4⍴11 12 13 14 21 22 23 24 31 32 33 34
21 22 23 24

⍝ ngn:604 —
(⊂4 3)⌷111 222 333 444   ⍝ 444 333

⍝ ngn:605 —
3(2 1)⌷3 4⍴11 12 13 14 21 22 23 24 31 32 33 34
32 31

⍝ ngn:606 —
a←2 2⍴0⋄a[;1]←1⋄a   ⍝ 2 2⍴1 0 1 0

⍝ ngn:607 —
(2 3)1⌷3 4⍴11 12 13 14 21 22 23 24 31 32 33 34
21 31

⍝ ngn:608 —
a←2 3⍴0⋄a[2;1 3]←1⋄a   ⍝ 2 3⍴0 0 0 1 0 1

⍝ ngn:609 —
(23 54 38)[1]   ⍝ 23

⍝ ngn:610 —
(2 3⍴100 101 102 110 111 112)[2;3]   ⍝ 112

⍝ ngn:611 —
(23 54 38)[2]   ⍝ 54

⍝ ngn:612 —
(2 3⍴100 101 102 110 111 112)[1;¯1]
⍝ error: INDEX ERROR

⍝ ngn:613 —
(23 54 38)[3]   ⍝ 38

⍝ ngn:614 —
(2 3⍴100 101 102 110 111 112)[10;1]
⍝ error: INDEX ERROR

⍝ ngn:615 —
(23 54 38)[4]
⍝ error: INDEX ERROR

⍝ ngn:616 —
(2 3⍴100 101 102 110 111 112)[2;]   ⍝ 110 111 112

⍝ ngn:617 —
(23 54 38)[¯1]
⍝ error: INDEX ERROR

⍝ ngn:618 —
(2 3⍴100 101 102 110 111 112)[;2]   ⍝ 101 111

⍝ ngn:619 —
(23 54 38)[1 3]   ⍝ 23 38

⍝ ngn:620 —
' X'[1+(3 3⍴⍳9)∊1 3 6 7 8]   ⍝ 3 3⍴'X X  XXX '

⍝ ngn:621 —
'hello'[2]   ⍝ 'e'

⍝ ngn:622 —
'ipodlover'[2 3 6 9 4 8 7 1 5]   ⍝ 'poordevil'

⍝ ngn:623 —
('axlrose'[5 4 1 3 6 7 2])[⍳4]   ⍝ 'oral'

⍝ ngn:624 —
(1 2 3)[⍬]   ⍝ ⍬

⍝ ngn:625 —
⍴(1 2 3)[1 2 3 0 5⍴0]   ⍝ 1 2 3 0 5

⍝ ngn:626 —
(⍳3)[]   ⍝ 1 2 3

⍝ ngn:627 —
⍴(3 3⍴⍳9)[⍬;⍬]   ⍝ 0 0

⍝ ngn:628 —
a←⍳5⋄a[2 4]←7 8⋄a   ⍝ 1 7 3 8 5

⍝ ngn:629 —
a←1 2 3⋄a[2]←4⋄a   ⍝ 1 4 3

⍝ ngn:630 —
a←⍳5⋄a[2 4]←7⋄a   ⍝ 1 7 3 7 5

⍝ ngn:631 —
a←2 2⍴⍳4⋄a[1;1]←4⋄a   ⍝ 2 2⍴4 2 3 4

⍝ ngn:632 —
a←⍳5⋄a[2]←7 8⋄a   ⍝ 1 (7 8) 3 4 5

⍝ ngn:633 —
a←3 4⍴⍳12⋄a[;2 3]←99   ⍝ 99

⍝ ngn:634 —
a←5 5⍴0⋄a[2 4;3 5]←2 2⍴1+⍳4⋄a
5 5⍴0 0 0 0 0 0 0 2 0 3 0 0 0 0 0 0 0 4 0 5 0 0 0 0 0

⍝ ngn:635 —
a←'this is a test'⋄a[1 6]←'TI'   ⍝ 'TI'

⍝ ngn:636 —
a←0 4 8⋄10+(a[1 3]←7 9)   ⍝ 17 19

⍝ ngn:637 —
a←1 2 3⋄a[⍬]←4⋄a   ⍝ 1 2 3

⍝ ngn:638 —
a←3 3⍴⍳9⋄a[⍬;1 2]←789⋄a   ⍝ 3 3⍴1 2 3 4 5 6 7 8 9

⍝ ngn:639 —
a←1 2 3⋄a[]←4 5 6⋄a   ⍝ 4 5 6

⍝ ngn:640 — Uses already implemented ⎕A/⎕UCS with ordinary array operations; original independent expectation passes unchanged
2↑•a   ⍝ 'AB'

⍝ ngn:641 — Uses already implemented ⎕A/⎕UCS with ordinary array operations; original independent expectation passes unchanged
¯3↑•a   ⍝ 'XYZ'

⍝ ngn:642 —
5↑'abc'   ⍝ 'abc  '

⍝ ngn:643 —
¯5↑'abc'   ⍝ '  abc'

⍝ ngn:644 —
3↑⍳2   ⍝ 1 2 0

⍝ ngn:645 —
¯1↑⍳4   ⍝ 1⍴4

⍝ ngn:646 —
⍴1↑(2 2⍴⍳4)(⍳10)   ⍝ 1⍴1

⍝ ngn:647 —
2↑1   ⍝ 1 0

⍝ ngn:648 —
2 ¯2↑1 1⍴1   ⍝ 2 2⍴0 1 0 0

⍝ ngn:649 —
3 3↑1 1⍴'a'   ⍝ 3 3⍴'a        '

⍝ ngn:650 —
2 3↑1+4 3⍴⍳12   ⍝ 2 3⍴2 3 4 5 6 7

⍝ ngn:651 —
¯1 3↑1+4 3⍴⍳12   ⍝ 1 3⍴11 12 13

⍝ ngn:652 —
1 2↑1+4 3⍴⍳12   ⍝ 1 2⍴2 3

⍝ ngn:653 —
3↑⍬   ⍝ 0 0 0

⍝ ngn:654 —
¯2↑⍬   ⍝ 0 0

⍝ ngn:655 —
0↑⍬   ⍝ ⍬

⍝ ngn:656 —
3 3↑1   ⍝ 3 3⍴1 0 0 0 0 0 0 0 0

⍝ ngn:657 —
2↑3 3⍴⍳9   ⍝ 2 3⍴1 2 3 4 5 6

⍝ ngn:658 —
¯2↑3 3⍴⍳9   ⍝ 2 3⍴4 5 6 7 8 9

⍝ ngn:659 —
4↑3 3⍴⍳9   ⍝ 4 3⍴1 2 3 4 5 6 7 8 9 0 0 0

⍝ ngn:660 —
⍬↑3 3⍴⍳9   ⍝ 3 3⍴1 2 3 4 5 6 7 8 9

⍝ ngn:661 —
↑(1 2 3)(4 5 6)   ⍝ 1 2 3

⍝ ngn:662 —
↑(1 2)(3 4 5)   ⍝ 1 2

⍝ ngn:663 —
↑'ab'   ⍝ 'a'

⍝ ngn:664 —
↑123   ⍝ 123

⍝ ngn:665 —
↑⍬   ⍝ 0

⍝ ngn:666 —
1⍉1 2   ⍝ 1 2

⍝ ngn:667 —
(2 2⍴⍳4)⍉2 2 2 2⍴⍳3   ⍝ 2 2 2 2⍴1 2 3 1 2 3 1 2 3 1 2 3 1 2 3 1

⍝ ngn:668 —
1 0⍉2 2 2⍴⍳8
⍝ error: LENGTH ERROR

⍝ ngn:669 —
¯1⍉1 2
⍝ error: DOMAIN ERROR

⍝ ngn:670 —
'a'⍉1 2
⍝ error: DOMAIN ERROR

⍝ ngn:671 —
3⍉0 1
⍝ error: RANK ERROR

⍝ ngn:672 — Fixed origin/constants; omit irrelevant PP assignment and constant rebinding; glyph-only formatting where needed
3 1 2⍉2 3 4⍴•a   ⍝ 3 4 2⍴'AMBNCODPEQFRGSHTIUJVKWLX'

⍝ ngn:673 —
1 1 3⍉2 3 4⍴⍳24
⍝ error: RANK ERROR

⍝ ngn:674 —
1 1⍉3 3⍴⍳9   ⍝ 1 5 9

⍝ ngn:675 —
1 1⍉2 3⍴⍳9   ⍝ 1 5

⍝ ngn:676 —
1 1 1⍉3 3 3⍴⍳27   ⍝ 1 14 27

⍝ ngn:677 — Fixed origin/constants; omit irrelevant PP assignment and constant rebinding; glyph-only formatting where needed
1 2 1⍉3 3 3⍴•a   ⍝ 3 3⍴'ADGKNQUXA'

⍝ ngn:678 —
⍉⍬   ⍝ ⍬

⍝ ngn:679 —
⍉''   ⍝ ''

⍝ ngn:680 —
⍉⍳3   ⍝ 1 2 3

⍝ ngn:681 —
⍉2 3⍴⍳6   ⍝ 3 2⍴1 4 2 5 3 6

⍝ ngn:682 — Uses already implemented ⎕A/⎕UCS with ordinary array operations; original independent expectation passes unchanged
⍉2 3 4⍴•a   ⍝ 4 3 2⍴'AMEQIUBNFRJVCOGSKWDPHTLX'

⍝ ngn:685 —
x⋄x←0
⍝ error: VALUE ERROR

⍝ ngn:686 —
⌽¨⍣3⊢(1 2)3(4 5 6)   ⍝ (2 1) 3 (6 5 4)

⍝ ngn:687 —
{}0   ⍝ {}0

⍝ ngn:688 —
a←5   ⍝ 5

⍝ ngn:689 —
a×a←2 5   ⍝ 4 25

⍝ ngn:691 —
{⍺}0
⍝ error: VALUE ERROR

⍝ ngn:692 —
{x}0⋄x←0
⍝ error: VALUE ERROR

⍝ ngn:695 —
{1+1}1   ⍝ 2

⍝ ngn:696 —
{⍵=0:1⋄2×∇⍵-1}5   ⍝ 32

⍝ ngn:697 —
{⍵<2:1⋄(∇⍵-1)+∇⍵-2}8   ⍝ 34

⍝ ngn:698 —
⊂{⍶ ⍶ ⍵}'ab'   ⍝ ⊂(⊂('ab'))

⍝ ngn:699 —
⊂{⍶ ⍹ ⍵}⌽'ab'   ⍝ ⊂('ba')

⍝ ngn:700 — Dyalog operand names
⊂{⍶⍶⍵}'ab'   ⍝ ⊂(⊂('ab'))

⍝ ngn:701 — Dyalog operand names
⊂{⍶⍹⍵}⌽'ab'   ⍝ ⊂('ba')

⍝ ngn:702 — Dyalog operand names
+{⍵⍶⍵}1 2   ⍝ 2 4

⍝ ngn:703 — Dyalog operand names
f←{⍵⍶⍵}⋄+f 1 2   ⍝ 2 4

⍝ ngn:704 — Dyalog operand names [rtol=1e-14]
tw←{⍶⍶⍵}⋄*tw 2   ⍝ 1618.177991912654

⍝ ngn:711 —
⍴''   ⍝ 1⍴0

⍝ ngn:712 —
⍴'x'   ⍝ ⍬

⍝ ngn:713 —
⍴'xx'   ⍝ 1⍴2

⍝ ngn:714 —
⍴'a''b'   ⍝ 1⍴3

⍝ ngn:715 —
⍴'''a'   ⍝ 1⍴2

⍝ ngn:716 —
⍴'a'''   ⍝ 1⍴2

⍝ ngn:717 —
⍴''''   ⍝ ⍬

⍝ ngn:718 — Agreed real infinity literal/empty-reduction identity
∞   ⍝ ∞

⍝ ngn:719 — Agreed real infinity literal/empty-reduction identity
¯∞   ⍝ ¯∞

⍝ ngn:721 — Two infinity literals form a vector
∞∞   ⍝ ∞ ∞

⍝ ngn:722 — Two infinity literals form a vector
∞ ∞   ⍝ ∞ ∞

⍝ ngn:724 —
⍴x[⍋x←6?49]   ⍝ 1⍴6

⍝ ngn:725 —
(a b)←1 2⋄a   ⍝ 1

⍝ ngn:726 —
(a b)←1 2⋄b   ⍝ 2

⍝ ngn:727 — Upstream syntax error independently checked in Dyalog; Rust regression passes
(a b)←+ ⋄ (2 a 3)(4 b 5)   ⍝ 5 9

⍝ ngn:728 —
(a b c)←3 4 5⋄a b c   ⍝ 3 4 5

⍝ ngn:729 —
(a b c)←6⋄a b c   ⍝ 6 6 6

⍝ ngn:730 — Concrete upstream error checked in Dyalog
(a b c)←7 8⋄a b c
⍝ error: LENGTH ERROR

⍝ ngn:731 —
((a b)c)←3(4 5)⋄a b c   ⍝ 3 3 (4 5)

⍝ ngn:732 —
(÷-)2   ⍝ ¯0.5

⍝ ngn:733 —
(+/÷≢)3 4 8   ⍝ 5

⍝ ngn:734 —
(-+/÷≢)3 4 8   ⍝ ¯5

⍝ ngn/examples/0-rho-iota:1 — ⍳ n generates a list of numbers from 0 to n-1 n n ⍴ A rearranges the elements of A in an n×n matrix
5 5⍴¯1+⍳25
5 5⍴0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24

⍝ ngn/examples/1-mult:1 — Multiplication table a × b scalar multiplication, "a times b" ∘. is the "outer product" operator A ∘.× B every item in A times every item in B
(¯1+⍳10) ×⌝ ¯1+⍳10
10 10⍴0 0 0 0 0 0 0 0 0 0 0 1 2 3 4 5 6 7 8 9 0 2 4 6 8 10 12 14 16 18 0 3 6 9 12 15 18 21 24 27 0 4 8 12 16 20 24 28 32 36 0 5 10 15 20 25 30 35 40 45 0 6 12 18 24 30 36 42 48 54 0 7 14 21 28 35 42 49 56 63 0 8 16 24 32 40 48 56 64 72 0 9 18 27 36 45 54 63 72 81

⍝ ngn/examples/2-sierpinski:1 — Sierpinski's triangle; Pure glyph program with explicit-output wrapper removed; Dyalog and Rust agree; Remove shebang/comments and return the final value; Translate character subscripts and Life seed positions to origin one; Insert spaces before negative vector items
f←{(⍵,(⍴⍵)⍴0)⍪⍵,⍵} ⋄ S←{' #'[1+(f⍣⍵)1 1⍴1]} ⋄ S 5
32 32⍴'#                               ##                              # #                             ####                            #   #                           ##  ##                          # # # #                         ########                        #       #                       ##      ##                      # #     # #                     ####    ####                    #   #   #   #                   ##  ##  ##  ##                  # # # # # # # #                 ################                #               #               ##              ##              # #             # #             ####            ####            #   #           #   #           ##  ##          ##  ##          # # # #         # # # #         ########        ########        #       #       #       #       ##      ##      ##      ##      # #     # #     # #     # #     ####    ####    ####    ####    #   #   #   #   #   #   #   #   ##  ##  ##  ##  ##  ##  ##  ##  # # # # # # # # # # # # # # # # ################################'

⍝ ngn/examples/3-primes:1 —
(1=+⌿0=a |⌝ a)/a←1↓⍳99
2 3 5 7 11 13 17 19 23 29 31 37 41 43 47 53 59 61 67 71 73 79 83 89 97

⍝ ngn/examples/4-life:1 — Conway's game of life This example was inspired by the impressive demo at https://www.youtube.com/watch?v=a9xAKttWgP4 0 1 1 1 1 0 0 1 0; Pure glyph program with explicit-output wrapper removed; Dyalog and Rust agree; Remove shebang/comments and return the final value; Translate character subscripts and Life seed positions to origin one; Insert spaces before negative vector items
c←(3 3⍴⍳9)∊2 3 4 5 8 ⋄ c←(3 3⍴⍳9)∊2 4 7 8 9 ⋄ b←¯1⊖¯2⌽5 7↑c ⋄ life←{1⍵∨.∧3 4=⊂+/+⌿1 0 ¯1 ⊖⌝ 1 0 ¯1⌽¨⊂⍵} ⋄ gen←{' #'[1+(life⍣⍵)b]} ⋄ gen¨⍳3
(5 7⍴'                # #    ##      #   ') (5 7⍴'                #      # #    ##   ') (5 7⍴'                 #    ##      ##   ')

⍝ ngn/examples/5-rule30:1 — See https://en.wikipedia.org/wiki/Rule_30; Use eight generations
r←30 ⋄ n←8 ⋄ t←⌽r⊤⍨8⍴2 ⋄ ' #'[1+⊃⌽{⍵,⍨⊂t[1+2⊥¨(,/∘(3∘↕))0,0,⍨↑⍵]}⍣n⊂z,1,z←n⍴0]
9 17⍴'        #               ###             ##  #           ## ####         ##  #   #       ## #### ###     ##  #    #  #   ## ####  ###### ##  #   ###     #'

⍝ ngn/examples/6-queens:1 — Rotate and reflect the accumulator in basedpl's left scan
queens←{ search←{ (⊂⍬)∊⍵:0⍴⊂⍬ ⋄ 0=⍴⍵:rmdups ⍺ ⋄ (hd tl)←(↑⍵)(1↓⍵) ⋄ next←⍺∘,¨hd ⋄ rems←hd free¨⊂tl ⋄ ,/next ∇¨rems } ⋄ cvex←(⍳⍵)×⊂¯1 0 1 ⋄ free←{⍵~¨⍺+(⍴⍵)↑cvex} ⋄ rmdups←{ rots←{{⍒⍺}\4/⊂⍵} ⋄ refs←{{⍋⍺}\2/⊂⍵} ⋄ best←{(↑⍋⊃⍵)⊃⍵} ⋄ all8←,⊃refs¨rots ⍵ ⋄ (1+⍵≡best all8)⊃⍬(,⊂⍵) } ⋄ fmt←{ chars←'·⍟'[1+(⊃⍵) =⌝ ⍳⍺] ⋄ expd←1↓,⊃⍺⍴⊂0 1 ⋄ ⊃¨↓↓expd\chars } ⋄ squares←(⊂⍳⌈⍵÷2),1↓⍵⍴⊂⍳⍵ ⋄ ⍵ fmt ⍬ search squares } ⋄ queens 5
(5 9⍴'⍟ · · · ·· · ⍟ · ·· · · · ⍟· ⍟ · · ·· · · ⍟ ·') (5 9⍴'· ⍟ · · ·· · · · ⍟· · ⍟ · ·⍟ · · · ·· · · ⍟ ·')

⍝ ngn/examples/7-mandelbrot:1 — Use a 13 by 13 grid
' #'[1+9>|{⍺+⍵*2}/9⍴⊂¯3×.7J.5-⍉a +⌝ 0J1×a←(¯1+⍳n+1)÷n←12]
13 13⍴'                                  #            #          ####       #######   #########       #######        ####           #            #                              '

⍝ ngn:501 — ngn accepts count/function operands to power in either order (apl.js, voc[⍣]); port to function⍣count; Explicit modified assignment updates the outer counter under basedpl scope rules; Original expected 5 retained and checked in Dyalog 20.0.53963.0
c←0 ⋄ ({c+←1}⍣5)0 ⋄ c   ⍝ 5

