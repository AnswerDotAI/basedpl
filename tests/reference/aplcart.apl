⍝ aplcart/table.tsv:2 — Empty Numeric Vector
⍬ ≡ 0⍴0   ⍝ 1

⍝ aplcart/table.tsv:3 — Same (I-combinator): Y
⊢  1 2 3   ⍝ 1 2 3

⍝ aplcart/table.tsv:4 — Separate right operand of dyadic operator (DOP) from its right argument (same as (X DOP Y)Z )
⌽⍣3 ⊢ 'Yyy'   ⍝ 'yyY'

⍝ aplcart/table.tsv:5 — Right (K-combinator): Y
'L' ⊢ 'R'   ⍝ 'R'

⍝ aplcart/table.tsv:6 — Church Boolean false (X if false, else Y)
'true' ⊢ 'false'   ⍝ 'false'

⍝ aplcart/table.tsv:7 — Same (I-combinator): Y
⊣  1 2 3   ⍝ 1 2 3

⍝ aplcart/table.tsv:8 — Left (KI-combinator): X
'L' ⊣ 'R'   ⍝ 'L'

⍝ aplcart/table.tsv:9 — Church Boolean true (X if true, else Y)
'true' ⊣ 'false'   ⍝ 'true'

⍝ aplcart/table.tsv:10 — Conjugate ('Identity' if Y not complex)
+ 1.2 0j4 ¯5j¯6 'abc'   ⍝ 1.2 0j¯4 ¯5j6 ('abc')

⍝ aplcart/table.tsv:11 — Mirror complex N across x-axis
+1J2 ¯1J¯2   ⍝ 1j¯2 ¯1j2

⍝ aplcart/table.tsv:12 — Adding N to M
(1 2 3 4 + 10 ⋄ 1 2 3 + 2 ¯4 1)   ⍝ (11 12 13 14 ⋄ 3 ¯2 4)

⍝ aplcart/table.tsv:13 — Negate: 0-N
- 3.2 ¯7 0   ⍝ ¯3.2 7 0

⍝ aplcart/table.tsv:14 — Subtracting N from M
(3 7 9 - 5 ⋄ 5 1 4 - 2 3 4)   ⍝ (¯2 2 4 ⋄ 3 ¯2 0)

⍝ aplcart/table.tsv:15 — Direction ('Signum' if N is real); Dyalog floating arithmetic differs at the final bit; explicit relative tolerance
× 3.1 ¯2 0 3j4   ⍝ 1 ¯1 0 0.6000000000000001j0.8

⍝ aplcart/table.tsv:16 — Multiplying M and N
(2 ¯3 4.5 × ¯3 ¯4 2 ⋄ 3 1 4 × 10)   ⍝ (¯6 12 9 ⋄ 30 10 40)

⍝ aplcart/table.tsv:17 — Probabilistic AND
1 0.5 0.25 0 × 0.5   ⍝ 0.5 0.25 0.125 0

⍝ aplcart/table.tsv:18 — Hadamard product of Mm and Nm
A ← 2 3⍴10 20,30 40,50 60 ⋄ B ← 2 3⍴1 2,3 4,5 6 ⋄ A × B
2 3⍴10 40 90 160 250 360

⍝ aplcart/table.tsv:19 — Selecting elements satisfying condition A, others to 0
1 1 0 1 0 × 3 1 4 1 5   ⍝ 3 1 0 1 0

⍝ aplcart/table.tsv:20 — Greatest Common Divisor of M and N
15 1 2 7 ∨ 35 1 4 0   ⍝ 5 1 2 7

⍝ aplcart/table.tsv:21 — Logical OR
0 1 0 1 ∨ 0 0 1 1   ⍝ 0 1 1 1

⍝ aplcart/table.tsv:22 — Lowest Common Multiple of M and N
15 1 2 7 ∧ 35 1 4 0   ⍝ 105 1 4 0

⍝ aplcart/table.tsv:23 — Logical AND
0 1 0 1 ∧ 0 0 1 1   ⍝ 0 0 0 1

⍝ aplcart/table.tsv:24 — Rounding up to integer
⌈ 3.4 ¯3.4 3 0   ⍝ 4 ¯3 3 0

⍝ aplcart/table.tsv:25 — Maximum of M and N
1.1 ¯2 ⌈ 8.1 ¯3.4   ⍝ 8.1 ¯2

⍝ aplcart/table.tsv:26 — Rounding down to integer
⌊ 3.4 ¯3.4 3 0   ⍝ 3 ¯4 3 0

⍝ aplcart/table.tsv:27 — Minimum of M and N
1.1 ¯2 ⌊ 8.1 ¯3.4   ⍝ 1.1 ¯3.4

⍝ aplcart/table.tsv:28 — Reverse last axis of Y
⌽ 'trams'   ⍝ 'smart'

⍝ aplcart/table.tsv:29 — Reflect vertically
mat ← 3 4⍴⍳12 ⋄ ⌽ mat   ⍝ 3 4⍴4 3 2 1 8 7 6 5 12 11 10 9

⍝ aplcart/table.tsv:30 — Rotate vectors along last axis of Y
(3 ⌽ 'HatStand' ⋄ ¯2 ⌽ 1 2 3 4 5 6)   ⍝ ('StandHat' ⋄ 5 6 1 2 3 4)

⍝ aplcart/table.tsv:31 — Reverse leading axis of Y
mat ← 2 3 4⍴⍳24 ⋄ ⊖mat
2 3 4⍴13 14 15 16 17 18 19 20 21 22 23 24 1 2 3 4 5 6 7 8 9 10 11 12

⍝ aplcart/table.tsv:32 — Reflect horizontally
mat ← 3 4⍴⍳12 ⋄ ⊖mat   ⍝ 3 4⍴9 10 11 12 5 6 7 8 1 2 3 4

⍝ aplcart/table.tsv:33 — Rotate vectors along leading axis of Y
mat ← 3 4⍴⍳12 ⋄ 0 1 2 ¯1 ⊖ mat   ⍝ 3 4⍴1 6 11 12 5 10 3 4 9 2 7 8

⍝ aplcart/table.tsv:34 — Reciprocal: 1÷N
÷ 1 2 3   ⍝ 1 0.5 0.3333333333333333

⍝ aplcart/table.tsv:35 — Dividing M by N; Dyalog floating arithmetic differs at the final bit; explicit relative tolerance
(1 2 3 ÷ 4 5 7 ⋄ 10 ÷ ¯2 0.5)   ⍝ (0.25 0.4 0.4285714285714285 ⋄ ¯5 20)

⍝ aplcart/table.tsv:36 — Magnitude (absolute value)
| 2.3 ¯4 0 3j4   ⍝ 2.3 4 0 5

⍝ aplcart/table.tsv:37 — Residue after dividing N by M
2 10 ¯2.5 | 7 ¯13 8   ⍝ 1 7 ¯2

⍝ aplcart/table.tsv:38 — Factorial (Gamma function of N+1)
! 3 9 ¯0.11   ⍝ 6 362880 1.076830682829913

⍝ aplcart/table.tsv:39 — Number of selections of size M from N (using Beta function): C(N,M)
2 1 3 ! 3 10 ¯0.11   ⍝ 3 10 ¯0.0429385

⍝ aplcart/table.tsv:40 — Selecting elements satisfying condition A, others to 1
1 1 0 1 0 ! 3 1 4 1 4   ⍝ 3 1 1 1 1

⍝ aplcart/table.tsv:41 — Random number selected from ⍳J; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
J←2 3⍴2 3 4 5 6 7 ⋄ r←?J ⋄ ((⍴J)≡⍴r)∧∧/∊(1≤r)∧r≤J
1

⍝ aplcart/table.tsv:42 — Random real number between (0,1) if B=0 or ⎕IO if B=1; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
B←0 1 0 1 ⋄ r←?B ⋄ ((⍴B)≡⍴r)∧∧/((B=1)∧r=1)∨(B=0)∧(0≤r)∧r<1
1

⍝ aplcart/table.tsv:43 — Deal: Is random numbers between 1 and Js (without replacement); Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Is←5 ⋄ Js←12 ⋄ r←Is?Js ⋄ (Is=≢r)∧(r≡∪r)∧∧/(1≤r)∧r≤Js
1

⍝ aplcart/table.tsv:44 — Less Than
(1 2 3 < 4 2 ¯1 ⋄ 1 2 3 < 2)   ⍝ (1 0 0 ⋄ 1 0 0)

⍝ aplcart/table.tsv:45 — Logical converse nonimplication
0 1 0 1 < 0 0 1 1   ⍝ 0 0 1 0

⍝ aplcart/table.tsv:46 — Less Than Or Equal To
(1 2 3 ≤ 4 2 ¯1 ⋄ 1 2 3 ≤ 2)   ⍝ (1 1 0 ⋄ 1 1 0)

⍝ aplcart/table.tsv:47 — Logical implication
0 1 0 1 ≤ 0 0 1 1   ⍝ 1 0 1 1

⍝ aplcart/table.tsv:48 — Equal To
(1 2 3 = 4 2 ¯1 ⋄ 'Banana' = 'a' ⋄ 7 = '7')
(0 1 0) (0 1 0 1 0 1) 0

⍝ aplcart/table.tsv:49 — Logical XNOR
0 1 0 1 = 0 0 1 1   ⍝ 1 0 0 1

⍝ aplcart/table.tsv:50 — Greater Than Or Equal To
(1 2 3 ≥ 4 2 ¯1 ⋄ 1 2 3 ≥ 2)   ⍝ (0 1 1 ⋄ 0 1 1)

⍝ aplcart/table.tsv:51 — Logical converse implication
0 1 0 1 ≥ 0 0 1 1   ⍝ 1 1 0 1

⍝ aplcart/table.tsv:52 — Greater Than
(1 2 3 > 4 2 ¯1 ⋄ 1 2 3 > 2)   ⍝ (0 0 1 ⋄ 0 0 1)

⍝ aplcart/table.tsv:53 — Logical nonimplication
0 1 0 1 > 0 0 1 1   ⍝ 0 1 0 0

⍝ aplcart/table.tsv:54 — Nub sieve: mask for major cells to leave the distinct (∪Y)
Y←1 2 1 3 2 ⋄ ≠Y   ⍝ 1 1 0 1 0

⍝ aplcart/table.tsv:55 — Not Equal To
(1 2 3 ≠ 4 2 ¯1 ⋄ 'Banana' ≠ 'a' ⋄ 7 ≠ '7')
(1 0 1) (1 0 1 0 1 0) 1

⍝ aplcart/table.tsv:56 — Logical XOR
0 1 0 1 ≠ 0 0 1 1   ⍝ 0 1 1 0

⍝ aplcart/table.tsv:57 — Depth: Maximum level of nesting in Y (negative if uneven)
(≡ 7 ⋄ ≡ 'abc' ⋄ ≡ (1 2⋄ 3 4) ⋄ ≡ (1 2)(3 4)5)
0 1 2 ¯2

⍝ aplcart/table.tsv:58 — Match: 1 if X is identical to Y, else 0
('b' 'e' 'x' ≡ 'bex' ⋄ 1 ≡ 1 1)   ⍝ 1 0

⍝ aplcart/table.tsv:59 — Number of rows in matrix Xm
mat ← 2 3⍴⍳6 ⋄ (≢ mat ⋄ ⍴ mat)   ⍝ 2 (2 3)

⍝ aplcart/table.tsv:60 — Tally: Number of items in leading axis
(≢ 'a' ⋄ ≢ 7 4 2 ⋄ ≢ 5 4 3⍴0 ⋄ ≢ (1 2⋄ 3 4))
1 3 5 2

⍝ aplcart/table.tsv:61 — Not Match: ~X≡Y
('bex' ≢ 'b','e','x' ⋄ 1 ≢ 1 1)   ⍝ 0 1

⍝ aplcart/table.tsv:62 — Split: Nest sub-arrays (from last axis)
mat ← 2 3 4⍴⍳24 ⋄ ↓ mat
2 3⍴(1 2 3 4 ⋄ 5 6 7 8 ⋄ 9 10 11 12 ⋄ 13 14 15 16 ⋄ 17 18 19 20 ⋄ 21 22 23 24)

⍝ aplcart/table.tsv:63 — Matrix to vector of row vectors
mat ← 3 4⍴⍳12 ⋄ ↓ mat   ⍝ (1 2 3 4 ⋄ 5 6 7 8 ⋄ 9 10 11 12)

⍝ aplcart/table.tsv:64 — Drop Is rows from matrix Ym
mat ← 3 4⍴⍳24 ⋄ 1 ↓ mat   ⍝ 2 4⍴5 6 7 8 9 10 11 12

⍝ aplcart/table.tsv:65 — Drop Iv items along leading axes of Y
mat ← 3 4⍴⍳24 ⋄ 1 ¯2 ↓ mat   ⍝ 2 2⍴5 6 9 10

⍝ aplcart/table.tsv:66 — Mix: Remove nesting (adding trailing axes)
⊃ (6 4) 5 3   ⍝ 3 2⍴6 4 5 0 3 0

⍝ aplcart/table.tsv:67 — Vector of row vectors to matrix
⊃ 'Hip' 'Hop'   ⍝ 2 3⍴'HipHop'

⍝ aplcart/table.tsv:68 — Padding Yv on the right to width Is
(5 ↑ 3 1 4 ⋄ ¯5 ↑ 3 1 4)   ⍝ (3 1 4 0 0 ⋄ 0 0 3 1 4)

⍝ aplcart/table.tsv:69 — Take Is rows from matrix Ym
mat ← 3 4⍴⍳12 ⋄ ¯2 ↑ mat   ⍝ 2 4⍴5 6 7 8 9 10 11 12

⍝ aplcart/table.tsv:70 — Take Iv items along leading axes of Y
mat ← 3 4⍴⍳12 ⋄ 2 ¯3 ↑ mat   ⍝ 2 3⍴2 3 4 6 7 8

⍝ aplcart/table.tsv:71 — Padding Ys to shape Iv
¯2 3 ↑ 7   ⍝ 2 3⍴0 0 0 7 0 0

⍝ aplcart/table.tsv:72 — Shape: Length of each axis of Y
mat ← 3 4⍴⍳12 ⋄ (⍴ mat ⋄ ⍴⍴ mat ⋄ ⍴ 'your boat' ⋄ ⍴ 7 ⋄ ⍴⍴ 7)
(3 4 ⋄ 1⍴2 ⋄ 1⍴9 ⋄ ⍬ ⋄ 1⍴0)

⍝ aplcart/table.tsv:73 — Reshape Y to have shape Iv
2 3 4 ⍴ 1 2 3 4 5 6 7
2 3 4⍴1 2 3 4 5 6 7 1 2 3 4 5 6 7 1 2 3 4 5 6 7 1 2 3

⍝ aplcart/table.tsv:74 — e raised to the power N
* 0 1 2   ⍝ 1 2.718281828459045 7.38905609893065

⍝ aplcart/table.tsv:75 — M raised to the power N
49 5 ¯4 * 0.5 2 0.5   ⍝ 6.999999999999999 25 0j2

⍝ aplcart/table.tsv:76 — Natural logarithm of N
⍟ 1 2 3 2.7182818285
0 0.6931471805599453 1.09861228866811 1.000000000015066

⍝ aplcart/table.tsv:77 — Base-M logarithm of N
2 10 ⍟ 32 1000   ⍝ 5 3

⍝ aplcart/table.tsv:78 — pi times N
π 0 1 2   ⍝ 0 3.141592653589793 6.283185307179586

⍝ aplcart/table.tsv:79 — Circular functions
(1 ○ 0 1.5707963 3.1415927 ⋄ 2 ○ 0 1.5707963 3.1415927 ⋄ 3 ○ 0 1.5707963 3.1415927)
(0 0.9999999999999997 ¯4.641020666628482e¯08 ⋄ 1 2.679489658502863e¯08 ¯0.9999999999999989 ⋄ 0 37320539.63435482 4.641020666628487e¯08)

⍝ aplcart/table.tsv:80 — Logical inverse (NOT): 0=B
~ 0 1 0 1   ⍝ 1 0 1 0

⍝ aplcart/table.tsv:81 — Without: (~Xv∊Y)/Xv
(3 1 4 1 5 ~ 5 1 ⋄ 'aa' 'bb' 'cc' 'bb'  ~ 'bb' 'xx')
(3 4 ⋄ ('aa' ⋄ 'cc'))

⍝ aplcart/table.tsv:82 — Commute (C-combinator): same as Y f X
(2 ⍴ 3 ⋄ 2 ⍴⍨ 3)   ⍝ (3 3 ⋄ 2 2 2)

⍝ aplcart/table.tsv:83 — Church Boolean Logical Inverse
('true' ⊣ 'false' ⋄ 'true' ⊣⍨ 'false')   ⍝ ('true' ⋄ 'false')

⍝ aplcart/table.tsv:84 — Constant (K-combinator): ignore argument and return Y
Y←3 1 3 2 ⋄ Z←42 ⋄ Y⍨Z   ⍝ 3 1 3 2

⍝ aplcart/table.tsv:85 — Commute (W-combinator): same as Y f Y
⍴⍨ 3   ⍝ 3 3 3

⍝ aplcart/table.tsv:86 — Beside (D-combinator): X∘f on the result of g on Y, that is, X f g Y
¯1 ⌽∘⍳¨ 3 4 5   ⍝ (3 1 2 ⋄ 4 1 2 3 ⋄ 5 1 2 3 4)

⍝ aplcart/table.tsv:87 — Curry: g between X and Y, that is, XgY
next ← 1∘+ ⋄ next 23   ⍝ 24

⍝ aplcart/table.tsv:88 — Beside: f on the result of g on Y, that is, f g Y
⌽∘⍳¨ 3 4 5   ⍝ (3 2 1 ⋄ 4 3 2 1 ⋄ 5 4 3 2 1)

⍝ aplcart/table.tsv:89 — Each: f between items of X and Y
3 ↑¨ 1 2 (3 4) 'V'   ⍝ (1 0 0 ⋄ 2 0 0 ⋄ 3 4 0 ⋄ 'V  ')

⍝ aplcart/table.tsv:90 — Each: f on items of Y
↑¨ 1 2 3 'ABC' (9 8 7)   ⍝ 1 2 3 'A' 9

⍝ aplcart/table.tsv:91 — Over (Ψ-combinator): preprocess (g) arguments before applying main function (f)
X←2 ⋄ f←+ ⋄ g←×⍨ ⋄ Y←3 ⋄ X f⍥g Y   ⍝ 13

⍝ aplcart/table.tsv:92 — Over: f on the result of g on Y, that is, f g Y
f←- ⋄ g←× ⋄ Y←3 ⋄ f⍥g Y   ⍝ ¯1

⍝ aplcart/table.tsv:93 — Behind: g on f X and Y, that is, (f X) g Y
X←2 ⋄ f←- ⋄ g←× ⋄ Y←3 ⋄ X f⍛g Y   ⍝ ¯6

⍝ aplcart/table.tsv:94 — Behind: apply g between (f Y) and Y, that is (f Y) g Y
f←- ⋄ g←× ⋄ Y←3 ⋄ f⍛g Y   ⍝ ¯9

⍝ aplcart/table.tsv:95 — N-wise Reduce: f between all items of Y in groups of Is on last axis
+/2↕ 1 2 3 4 5   ⍝ 3 5 7 9

⍝ aplcart/table.tsv:96 — Replicate along last axis of Y
3 1 ¯2 2 / 6 7 8 9   ⍝ 6 6 6 7 0 0 9 9

⍝ aplcart/table.tsv:97 — Filtering columns of Y according to mask Av
1 0 1 0 1 / 'Heart'   ⍝ 'Hat'

⍝ aplcart/table.tsv:98 — Reduce: f between all items of Y on last axis
+/ 1 2 3 4 5   ⍝ 15

⍝ aplcart/table.tsv:99 — N-wise Reduce First: f between all items of Y in groups of Is on first axis
mat ← 3 4⍴⍳12 ⋄ +⌿ mat   ⍝ 15 18 21 24

⍝ aplcart/table.tsv:100 — Replicate along leading axis of Y
mat ← 3 4⍴⍳12 ⋄ 1 0 2 ⌿ mat   ⍝ 3 4⍴1 2 3 4 9 10 11 12 9 10 11 12

⍝ aplcart/table.tsv:101 — Filtering major cells of Y according to mask Av
mat ← 3 4⍴⍳12 ⋄ 1 0 1 ⌿ mat   ⍝ 2 4⍴1 2 3 4 9 10 11 12

⍝ aplcart/table.tsv:102 — Reduce First: f between all items of Y on first axis
mat ← 3 4⍴⍳12 ⋄ +⌿ mat   ⍝ 15 18 21 24

⍝ aplcart/table.tsv:103 — Atop (B₁-combinator): f on the result of X g Y, that is, f X g Y
X←2 ⋄ f←- ⋄ g←× ⋄ Y←3 ⋄ X f⍤g Y   ⍝ ¯6

⍝ aplcart/table.tsv:104 — Atop (B-combinator): f on the result of g on Y, that is, f g Y
f←- ⋄ g←× ⋄ Y←3 ⋄ f⍤g Y   ⍝ ¯1

⍝ aplcart/table.tsv:105 — Logical NOR
0 1 0 1 ⍱ 0 0 1 1   ⍝ 1 0 0 0

⍝ aplcart/table.tsv:106 — Logical NAND
0 1 0 1 ⍲ 0 0 1 1   ⍝ 1 1 1 0

⍝ aplcart/table.tsv:107 — Ravel: Reshape into a vector
cube ← 2 2 2⍴⍳8 ⋄ , cube   ⍝ 1 2 3 4 5 6 7 8

⍝ aplcart/table.tsv:108 — Catenate: Join along last axis
1 2 3 , 4 5 6   ⍝ 1 2 3 4 5 6

⍝ aplcart/table.tsv:109 — Append scalar to each row of matrix
cube ← 2 2⍴⍳4 ⋄ cube , 99   ⍝ 2 3⍴1 2 99 3 4 99

⍝ aplcart/table.tsv:110 — Append elements of vector to respective rows of matrix
cube ← 2 2⍴⍳4 ⋄ cube , 0 99   ⍝ 2 3⍴1 2 0 3 4 99

⍝ aplcart/table.tsv:111 — Table: Reshape into 2-dimensional array
hypercube ← 2 2 2 2⍴⍳16 ⋄ ⍪ hypercube
2 8⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16

⍝ aplcart/table.tsv:112 — Ravel planes of rank 3 array Y to form rows of a matrix
cube ← 2 2 2⍴⍳8 ⋄ ⍪ cube   ⍝ 2 4⍴1 2 3 4 5 6 7 8

⍝ aplcart/table.tsv:113 — Reshaping vector Yv into a one-column matrix
⍪ 2 3 4   ⍝ 3 1⍴2 3 4

⍝ aplcart/table.tsv:114 — Catenate First: Join along leading axis
mat ← 2 3⍴⍳6 ⋄ mat ⍪ mat   ⍝ 4 3⍴1 2 3 4 5 6 1 2 3 4 5 6

⍝ aplcart/table.tsv:115 — Append scalar to each column of matrix
mat ← 2 3⍴⍳6 ⋄ mat ⍪ 0   ⍝ 3 3⍴1 2 3 4 5 6 0 0 0

⍝ aplcart/table.tsv:116 — Append elements of vector to respective columns of matrix
mat ← 2 3⍴⍳6 ⋄ mat ⍪ 7 8 9   ⍝ 3 3⍴1 2 3 4 5 6 7 8 9

⍝ aplcart/table.tsv:117 — First item of Y
↑ 'Word'   ⍝ 'W'

⍝ aplcart/table.tsv:118 — Reach into Y along path given by Iv
(3 ⊃ 'Word' ⋄ 2 ⊃ (1 2⋄ 3 4 5) ⋄ 1⊃2 ⊃ (1 2⋄ 3 4 5))
'r' (3 4 5) 3

⍝ aplcart/table.tsv:119 — Enclose: Scalar containing Y
(1(2 3) ⋄ ⊂ 1(2 3) ⋄ ⊂⊂ 1(2 3))   ⍝ (1 (2 3) ⋄ ⊂(1 (2 3)) ⋄ ⊂(⊂(1 (2 3))))

⍝ aplcart/table.tsv:120 — Partitioned enclose of Y according to Av (along last axis) beginning enclosures on 1s
0 1 0 1 ⊂ 1 2 3 4   ⍝ (2 3 ⋄ 1⍴4)

⍝ aplcart/table.tsv:121 — Nest: Y if already nested, else scalar containing Y
('this' ⋄ ⊆ 'this' ⋄ 'this' 'that' ⋄ ⊆ 'this' 'that')
('this' ⋄ ⊂('this') ⋄ ('this' ⋄ 'that') ⋄ ('this' ⋄ 'that'))

⍝ aplcart/table.tsv:122 — Partition Y according to Mv (along last axis) beginning partitions on increases
(1 0 0 1 1 ⊆ 1 2 3 4 5 ⋄ 1 1 2 2 2 ⊆ ⍳5)
((1⍴1 ⋄ 4 5) ⋄ (1 2 ⋄ 3 4 5))

⍝ aplcart/table.tsv:123 — Materialise items of Y in workspace
Y←2 3⍴⍳6 ⋄ ⌷Y   ⍝ 2 3⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:124 — Index Y using indices Iv
mat ← 3 4⍴⍳12 ⋄ (2 3 ⌷ mat ⋄ 2 ⌷ mat)   ⍝ 7 (5 6 7 8)

⍝ aplcart/table.tsv:125 — Integers from 1 to Js
⍳ 10   ⍝ 1 2 3 4 5 6 7 8 9 10

⍝ aplcart/table.tsv:126 — Indices of all items of array of shape Jv
⍳ 2 3   ⍝ 2 3⍴(1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3)

⍝ aplcart/table.tsv:127 — Index of: First indices in X of major cells Y
mat ← 3 2⍴⍳6 ⋄ ('ABCDABCDEF' ⍳ 'ACF' ⋄ mat ⍳ 5 6)
(1 3 10) 3

⍝ aplcart/table.tsv:128 — Index of keys Y in key vector Xv
'Abe' 'Bob' 'Carl' ⍳ 'Bob' 'Abe'   ⍝ 2 1

⍝ aplcart/table.tsv:129 — Enlist: Simple vector from elements of Y
mat← 2 3⍴⍳6 ⋄ (∊ 0 mat (7 8) 9 ⋄ ∊ 2 3⍴1 'abc')
(0 1 2 3 4 5 6 7 8 9 ⋄ 1 'a' 'b' 'c' 1 'a' 'b' 'c' 1 'a' 'b' 'c')

⍝ aplcart/table.tsv:130 — For each item of X, 1 if found in Y, else 0
mat← 2 3⍴⍳6 ⋄ ('abc' 4 ∊ 4 'ab' 'abcd' ⋄ mat ∊ 6 2 7 4)
(0 1 ⋄ 2 3⍴0 1 0 1 0 1)

⍝ aplcart/table.tsv:131 — Indices of all 1s in B
bmat← 2 3⍴0 1 0 1 0 1 ⋄ (⍸ 1 0 0 1 1 ⋄ ⍸ bmat)
(1 4 5 ⋄ (1 2 ⋄ 2 1 ⋄ 2 3))

⍝ aplcart/table.tsv:132 — List of arcs from adjacency matrix Bm
amat ← 3 3⍴0 1 0 0 0 1 1 1 0 ⋄ ⍸ amat   ⍝ (1 2 ⋄ 2 3 ⋄ 3 1 ⋄ 3 2)

⍝ aplcart/table.tsv:133 — Indices of major cells of Y in left-inclusive intervals with cut-offs X
mat←3 2⍴⍳6 ⋄ ('AEIOU' ⍸ 'DYALOG' ⋄ 2 4 6 ⍸ 1 2 3 4 5 6 7 ⋄ mat ⍸ 3 3 ⋄ mat ⍸ 3 4)
(1 5 1 3 4 2) (0 1 1 2 2 3 3) 1 2

⍝ aplcart/table.tsv:134 — Unique: Distinct major cells of Y
mat←2 3⍴'flyshyfly' ⋄ (∪ 'ab' 'ba' 'ab' 1 1 2 ⋄ ∪ mat)
('ab' 'ba' 1 2 ⋄ 2 3⍴'flyshy')

⍝ aplcart/table.tsv:135 — Union: Xv,Yv~Xv
'ab' 'cde' 'fg' ∪ 'a' 'ab'   ⍝ ('ab') ('cde') ('fg') 'a'

⍝ aplcart/table.tsv:136 — Intersection: (Xv∊Yv)/Xv
22 'ab' 'fg' ∩ 'a' 'ab' 22   ⍝ 22 ('ab')

⍝ aplcart/table.tsv:137 — Power: iterating X∘f on Y until condition (f Y) g Y is true
1 +∘÷⍣= 1   ⍝ 1.618033988749897

⍝ aplcart/table.tsv:138 — Power: iterating f on Y until condition (f Y) g Y is true
1∘+∘÷⍣= 1   ⍝ 1.618033988749897

⍝ aplcart/table.tsv:139 — Ascending grade: Indices to reorder Y into ascending order
Y←3 1 2 1 ⋄ ⍋Y   ⍝ 2 4 3 1

⍝ aplcart/table.tsv:140 — Invert permutation
(⍋ 2 5 1 3 4 ⋄ ⍋⍋ 2 5 1 3 4)   ⍝ (3 1 4 5 2 ⋄ 2 5 1 3 4)

⍝ aplcart/table.tsv:141 — Ascending grade using collation sequence C
(⍋ 'Banana' ⋄ 'Banana'[⍋ 'Banana'] ⋄ 'an' ⍋ 'Banana' ⋄ 'Banana'['an' ⍋ 'Banana'])
(1 2 4 6 3 5 ⋄ 'Baaann' ⋄ 2 4 6 3 5 1 ⋄ 'aaannB')

⍝ aplcart/table.tsv:142 — Descending grade: Indices to reorder Y into descending order
Y←3 1 2 1 ⋄ ⍒Y   ⍝ 1 3 2 4

⍝ aplcart/table.tsv:143 — Descending grade using collation sequence C
(⍒ 'Banana' ⋄ 'Banana'[⍒ 'Banana'] ⋄ 'an' ⍒ 'Banana' ⋄ 'Banana'['an' ⍒ 'Banana'])
(3 5 2 4 6 1 ⋄ 'nnaaaB' ⋄ 1 3 5 2 4 6 ⋄ 'Bnnaaa')

⍝ aplcart/table.tsv:144 — Expand last axis of Y
(3 ¯2 4 \ 7 8 ⋄ 1 0 1 0 1 \ 'Hat')   ⍝ (7 7 7 0 0 8 8 8 8 ⋄ 'H a t')

⍝ aplcart/table.tsv:145 — Scan: f between items of Y in progressively longer vectors along last axis
mat← 3 4⍴1 3 6 10 5 11 18 26 9 19 30 42 ⋄ (+\ 1 2 3 4 5 ⋄ +\ mat)
(1 3 6 10 15 ⋄ 3 4⍴1 4 10 20 5 16 34 60 9 28 58 100)

⍝ aplcart/table.tsv:146 — Expand leading axis of Y
mat←3 4⍴⍳12 ⋄ 1 0 2 1 ⍀ mat
5 4⍴1 2 3 4 0 0 0 0 5 6 7 8 5 6 7 8 9 10 11 12

⍝ aplcart/table.tsv:147 — Scan First: f between items of Y in progressively longer vectors along first axis
mat←3 4⍴⍳12 ⋄ +⍀ mat   ⍝ 3 4⍴1 2 3 4 6 8 10 12 15 18 21 24

⍝ aplcart/table.tsv:148 — Transpose: Reverse order of axes of Y
mat←2 3 4⍴⍳24 ⋄ ⍉ mat
4 3 2⍴1 13 5 17 9 21 2 14 6 18 10 22 3 15 7 19 11 23 4 16 8 20 12 24

⍝ aplcart/table.tsv:149 — Reflect diagonally
mat←2 3⍴⍳6 ⋄ ⍉ mat   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/table.tsv:150 — Reorder the axes of Y
mat ← 2 3 4⍴⍳24 ⋄ 1 3 2 ⍉ mat
2 4 3⍴1 5 9 2 6 10 3 7 11 4 8 12 13 17 21 14 18 22 15 19 23 16 20 24

⍝ aplcart/table.tsv:151 — Key: f between unique X values and their corresponding items of Y; decorative --- separators omitted
('Banana' {⍺ ⍵}⌸ 3 1 4 1 5 9 ⋄ 'Banana' {⍺,+/⍵}⌸ 3 1 4 1 5 9 ⋄ 'Banana' {⍺ ⍵}⌸ 1 2 3 4 5 6)
(3 2⍴'B' (1⍴3) 'a' (1 1 9) 'n' (4 5) ⋄ 3 2⍴'B' 3 'a' 11 'n' 9 ⋄ 3 2⍴'B' (1⍴1) 'a' (2 4 6) 'n' (3 5))

⍝ aplcart/table.tsv:152 — Key: f between unique Y values and their first-axis indices
{⍺ ⍵}⌸ 'Banana'   ⍝ 3 2⍴'B' (1⍴1) 'a' (2 4 6) 'n' (3 5)

⍝ aplcart/table.tsv:153 — Boolean indication of top left corner of occurrences of entire array X within Y
'ana' ⍷ 'Banana'   ⍝ 0 1 0 1 0 0

⍝ aplcart/table.tsv:154 — Inner Product: f / g between trailing vectors of X and leading vectors of Y
mat ← 2 2⍴⍳4 ⋄ (1 2 3 +.× 4 5 6 ⋄ 3 ∧.= 3 3 3 3 ⋄ mat +.× mat)
32 1 (2 2⍴7 10 15 22)

⍝ aplcart/table.tsv:155 — Decode: Evaluate N in number system M
(2 ⊥ 1 1 0 1 ⋄ 24 60 60 ⊥ 2 46 40)   ⍝ 13 10000

⍝ aplcart/table.tsv:156 — Value of polynomial with descending coefficients N at point Ns
3 ⊥ 1 0 6   ⍝ 15

⍝ aplcart/table.tsv:157 — Encoding value N in number system M
(2 2 2 2 ⊤ 5 7 12 ⋄ 24 60 60 ⊤ 10000)   ⍝ (4 3⍴0 0 1 1 1 1 0 1 0 1 1 0 ⋄ 2 46 40)

⍝ aplcart/table.tsv:158 — Matrix inverse of Nm (square Nm)
mat ← 2 2⍴⍳4 ⋄ ⌹ mat
2 2⍴¯2 0.9999999999999998 1.5 ¯0.4999999999999999

⍝ aplcart/table.tsv:159 — Matrix pseudo-inverse of Nm (over-determined Nm)
mat ← 3 2⍴1 2 0 1 2 4 ⋄ ⌹ mat   ⍝ 2 3⍴0.2 ¯2 0.4 0 1 0

⍝ aplcart/table.tsv:160 — Solve equation system with variable coefficients Nm and values Mv
mat ← 2 2⍴⍳4 ⋄ 5 6 ⌹ mat   ⍝ ¯3.999999999999998 4.499999999999999

⍝ aplcart/table.tsv:161 — Multiplying Mm with inversed Nm
mat ← 2 2⍴⍳4 ⋄ mat ⌹ mat   ⍝ 2 2⍴1 0 0 1

⍝ aplcart/table.tsv:162 — Execute: Result of expression Dv; Reviewed Execute example checked through the Rust reference worker
Dv←'2+3×4' ⋄ ⍎Dv   ⍝ 14

⍝ aplcart/table.tsv:164 — Format: Character representation of Y
(⌽ 12 34 ⋄ ⌽ ⍕ 12 34)   ⍝ (34 12 ⋄ '43 21')

⍝ aplcart/table.tsv:165 — Format Y using ({width}, decimals) pairs Iv (negative decimals for scaled notation)
(2 ⍕ 3.125 0.002 ⋄ 6 2 ⍕ 3.125 0.002 ⋄ 6 2 ⍕ 1234 ⋄ ¯2 ⍕ 3.125 0.002)
(' 3.13 0.00' ⋄ '  3.13  0.00' ⋄ '******' ⋄ ' 3.1E0 2.0E¯3')

⍝ aplcart/table.tsv:169 — Modified Assignment (tradfns/tradops only)
var←40 ⋄ var+←2 ⋄ var   ⍝ 42

⍝ aplcart/table.tsv:170 — Assignment
var←42 ⋄ var   ⍝ 42

⍝ aplcart/table.tsv:171 — Dfn Self
Fact←{⍵≤1: 1 ⋄ ⍵×∇ ⍵-1} ⋄ Fact 5   ⍝ 120

⍝ aplcart/table.tsv:175 — Dfn/dop Right Argument
2 {⍵+1} 5   ⍝ 6

⍝ aplcart/table.tsv:176 — Dfn/dop Left Argument
2 {⍺+1} 5   ⍝ 3

⍝ aplcart/table.tsv:177 — Avoiding parentheses by swapping arguments in (X,Y) f Z
3 -⍨ 10×2   ⍝ 17

⍝ aplcart/table.tsv:178 — Outer Product: g between each item of X and every item of Y
1 2 3  ×⌝  4 5 6 7   ⍝ 3 4⍴4 5 6 7 8 10 12 14 12 15 18 21

⍝ aplcart/table.tsv:179 — Combining two lines into one
var×⍳4 ⊣ var←10   ⍝ 10 20 30 40

⍝ aplcart/table.tsv:180 — Dop Self
_Pow←{⍹=0:⍵ ⋄ ⍶ ⍢(⍹-1)⍶ ⍵} ⋄ ({1+⍵} _Pow 3) 5
8

⍝ aplcart/table.tsv:181 — N-row matrix from N vectors
⊃ 'Tic' 'Tac' 'Toe'   ⍝ 3 3⍴'TicTacToe'

⍝ aplcart/table.tsv:182 — Dop Right Operand
3 +{⍺ ⍹ ⍵}× 4   ⍝ 12

⍝ aplcart/table.tsv:183 — Dop Left Operand
3 +{⍺ ⍶ ⍵} 4   ⍝ 7

⍝ aplcart/table.tsv:184 — Reverse axis ax of Y
mat ← 3 4⍴⍳12 ⋄ ⌽[1] mat   ⍝ 3 4⍴9 10 11 12 5 6 7 8 1 2 3 4

⍝ aplcart/table.tsv:185 — Rotate vectors along axis ax of Y
mat ← 3 4⍴⍳12 ⋄ 0 1 2 ¯1 ⌽[1] mat   ⍝ 3 4⍴1 6 11 12 5 10 3 4 9 2 7 8

⍝ aplcart/table.tsv:186 — Split: Nest sub-arrays (from axis ax)
mat ← 3 4⍴⍳12 ⋄ ↓[1] mat   ⍝ (1 5 9 ⋄ 2 6 10 ⋄ 3 7 11 ⋄ 4 8 12)

⍝ aplcart/table.tsv:187 — Drop Iv items along axes ax of Y
mat ← 3 4⍴⍳12 ⋄ 1 ↓[2] mat   ⍝ 3 3⍴2 3 4 6 7 8 10 11 12

⍝ aplcart/table.tsv:188 — Mix: Remove nesting (adding axes between ⌊ax and ⌈ax)
⊃[0.5] 'Hip' 'Hop'   ⍝ 3 2⍴'HHiopp'

⍝ aplcart/table.tsv:189 — Take Iv items along axes ax of Y
mat ← 3 4⍴⍳12 ⋄ 1 ↑[2] mat   ⍝ 3 1⍴1 5 9

⍝ aplcart/table.tsv:190 — N-wise Reduce: f between all items of Y in groups of Is on axis ax
mat ← 3 4⍴⍳12 ⋄ +⌿ mat   ⍝ 15 18 21 24

⍝ aplcart/table.tsv:191 — Replicate along axis ax
mat ← 3 4⍴⍳12 ⋄ 2 0 1 /[1] mat   ⍝ 3 4⍴1 2 3 4 1 2 3 4 9 10 11 12

⍝ aplcart/table.tsv:192 — Reduce: f between all items of Y on axis ax
mat ← 3 4⍴⍳12 ⋄ +/[1] mat   ⍝ 15 18 21 24

⍝ aplcart/table.tsv:193 — Ravel with Axis: insert new axis between ⌊ax and ⌈ax
,[1.5] 'Foo'   ⍝ 3 1⍴'Foo'

⍝ aplcart/table.tsv:194 — Ravel with Axes: combine axes
cube ← 2 2 2⍴⍳8 ⋄ ,[2 3] cube   ⍝ 2 4⍴1 2 3 4 5 6 7 8

⍝ aplcart/table.tsv:195 — Laminate: Join along new axis
1 2 3 ,[0.5] 4 5 6   ⍝ 2 3⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:196 — Enclose: Contain axes ax inside scalars
cube ← 2 2 2⍴⍳8 ⋄ ⊂[2 3] cube   ⍝ (2 2⍴1 2 3 4 ⋄ 2 2⍴5 6 7 8)

⍝ aplcart/table.tsv:197 — Partitioned enclose of Y according to Av (along axis ax)
mat ← 3 2⍴⍳6 ⋄ 1 0 1 ⊂[1] mat   ⍝ (2 2⍴1 2 3 4 ⋄ 1 2⍴5 6)

⍝ aplcart/table.tsv:198 — Partition Y according to Mv (along axis ax)
mat ← 3 2⍴⍳6 ⋄ 1 1 2 ⊆[1] mat   ⍝ 2 2⍴(1 3 ⋄ 2 4 ⋄ 1⍴5 ⋄ 1⍴6)

⍝ aplcart/table.tsv:199 — Index Y using indices Iv along axes ax
mat ← 3 2⍴⍳6 ⋄ 2 ⌷[2] mat   ⍝ 2 4 6

⍝ aplcart/table.tsv:200 — Expand axis ax of Y
mat ← 3 2⍴⍳6 ⋄ 1 0 1 1 \[1] mat   ⍝ 4 2⍴1 2 0 0 3 4 5 6

⍝ aplcart/table.tsv:201 — Scan: f between items of Y in progressively longer vectors along axis ax
mat ← 3 2⍴⍳6 ⋄ +\[1] mat   ⍝ 3 2⍴1 2 4 6 9 12

⍝ aplcart/table.tsv:202 — Indexed Assignment
var←42 ⋄ var   ⍝ 42

⍝ aplcart/table.tsv:203 — Modified Indexed Assignment (tradfns/tradops only)
var←20 30 40 ⋄ var[1 1 3]+←2 ⋄ var   ⍝ 24 30 42

⍝ aplcart/table.tsv:204 — Constant (K-combinator): ignore arguments and return Y
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ Z←42 ⋄ X(Y⍨)Z   ⍝ 3 1 3 2

⍝ aplcart/table.tsv:205 — Curry: f between Z and Y, that is, Z f Y
prev ← -∘1 ⋄ sqrt ← *∘0.5 ⋄ (prev 23 ⋄ sqrt sqrt 16 81)
22 (2 3)

⍝ aplcart/table.tsv:206 — Rank: f between every trailing rank-Is subarray of X and every trailing rank-Js subarray of Y
nmat ← 3 4⍴⍳12 ⋄ 10 20 30 (+⍤0 1) nmat   ⍝ 3 4⍴11 12 13 14 25 26 27 28 39 40 41 42

⍝ aplcart/table.tsv:207 — Rank: f on every trailing rank-Js subarray of Y
cube ← 2 2 3⍴⍳12 ⋄ cmat ← 2 3⍴'abczxy' ⋄ ((,⍤2) cube ⋄ (⍋⍤1) cmat)
(2 6⍴1 2 3 4 5 6 7 8 9 10 11 12 ⋄ 2 3⍴1 2 3 2 3 1)

⍝ aplcart/table.tsv:208 — Unpack a vector X into an array based on mask B
'XYZ'@{0 1 1 0 0 1} 'abcdef'   ⍝ 'aXYdeZ'

⍝ aplcart/table.tsv:209 — At: apply X∘f to modify positions N in Y
10 (×@2 4) 1 2 3 4 5   ⍝ 1 20 3 40 5

⍝ aplcart/table.tsv:210 — At: apply X∘f to modify positions identified by Boolean mask (g Y) in Y
10 (×@(≤∘3)) 3 1 4 1 5   ⍝ 30 10 4 10 5

⍝ aplcart/table.tsv:211 — At: use values in X to replace positions N in Y
(0@2 4) 1 2 3 4 5   ⍝ 1 0 3 0 5

⍝ aplcart/table.tsv:212 — At: use values in X to replace positions identified by Boolean mask (g Y) in Y
'*'@(2∘|) 1 2 3 4 5   ⍝ '*' 2 '*' 4 '*'

⍝ aplcart/table.tsv:213 — Amend row Js of matrix Ym
mat ← 3 2⍴⍳6 ⋄ (8 9@2) mat   ⍝ 3 2⍴1 2 8 9 5 6

⍝ aplcart/table.tsv:214 — At: apply f to modify positions N in Y
(÷@2 4) 1 2 3 4 5   ⍝ 1 0.5 3 0.25 5

⍝ aplcart/table.tsv:215 — At: apply f to modify positions identified by Boolean mask (g Y) in Y
⌽@(2∘|) 1 2 3 4 5   ⍝ 5 2 3 4 1

⍝ aplcart/table.tsv:216 — Power: apply X∘f on Y Js times
1 (+⍣3) 5   ⍝ 8

⍝ aplcart/table.tsv:217 — Power: apply f on Y Js times
(1∘+⍣3) 5   ⍝ 8

⍝ aplcart/table.tsv:218 — Stencil: f on padding-size and (possibly overlapping) rectangles of Y of size and (optionally step) Jm; dfns display import/wrappers omitted to test underlying arrays
mat ← 4 4⍴⍳16 ⋄ (({⊂⍵}⌺3 3) mat ⋄ ({+/,⍵}⌺3 3) mat)
(4 4⍴(3 3⍴0 0 0 0 1 2 0 5 6 ⋄ 3 3⍴0 0 0 1 2 3 5 6 7 ⋄ 3 3⍴0 0 0 2 3 4 6 7 8 ⋄ 3 3⍴0 0 0 3 4 0 7 8 0 ⋄ 3 3⍴0 1 2 0 5 6 0 9 10 ⋄ 3 3⍴1 2 3 5 6 7 9 10 11 ⋄ 3 3⍴2 3 4 6 7 8 10 11 12 ⋄ 3 3⍴3 4 0 7 8 0 11 12 0 ⋄ 3 3⍴0 5 6 0 9 10 0 13 14 ⋄ 3 3⍴5 6 7 9 10 11 13 14 15 ⋄ 3 3⍴6 7 8 10 11 12 14 15 16 ⋄ 3 3⍴7 8 0 11 12 0 15 16 0 ⋄ 3 3⍴0 9 10 0 13 14 0 0 0 ⋄ 3 3⍴9 10 11 13 14 15 0 0 0 ⋄ 3 3⍴10 11 12 14 15 16 0 0 0 ⋄ 3 3⍴11 12 0 15 16 0 0 0 0) ⋄ 4 4⍴14 24 30 22 33 54 63 45 57 90 99 69 46 72 78 54)

⍝ aplcart/table.tsv:222 — Selective Assignment: exp is an expression that selects elements of "name"
var←42 ⋄ var   ⍝ 42

⍝ aplcart/table.tsv:223 — Modified Selective Assignment: exp is an expression that selects elements of "name" (tradfns/tradops only)
var←20 30 40 ⋄ (2 0 1/var)+←2 ⋄ var   ⍝ 24 30 42

⍝ aplcart/table.tsv:224 — Modified Indexed Assignment (also dfns/dops)
var←20 30 40 ⋄ plus←+ ⋄ {var[1 1 3]plus∘⊢←2}⍬ ⋄ var
24 30 42

⍝ aplcart/table.tsv:225 — Modified Selective Assignment: exp is an expression that selects elements of "name" (also dfns/dops)
var←20 30 40 ⋄ plus←+ ⋄ {(2 0 1/var)plus∘⊢←2}⍬ ⋄ var
24 30 42

⍝ aplcart/table.tsv:227 — The letters from A to Z; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
•A   ⍝ 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'

⍝ aplcart/table.tsv:228 — Casefold
Y←42 'Pete' 'Πέτρος'  ⋄ •C Y   ⍝ 42 ('pete') ('πέτροσ')

⍝ aplcart/table.tsv:230 — The digits from 0 to 9; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
•D   ⍝ '0123456789'

⍝ aplcart/table.tsv:238 — Uppercase
Y←42 'Pete' 'Πέτρος'  ⋄ 1∘•C Y   ⍝ 42 ('PETE') ('ΠΈΤΡΟΣ')

⍝ aplcart/table.tsv:287 — Lowercase
Y←42 'Pete' 'Πέτρος'  ⋄ ¯1∘•C Y   ⍝ 42 ('pete') ('πέτρος')

⍝ aplcart/table.tsv:395 — Map characters to/from Unicode code points
(•UCS 'ABC'⋄ •UCS 100 200 300⋄ 'UTF-8'•UCS '⍺*⎕')
(65 66 67 ⋄ 'dÈĬ' ⋄ 226 141 186 42 226 142 149)

⍝ aplcart/table.tsv:527 — Double: 2×N
(+⍨ 5 ⋄ +⍨ ⍳10 ⋄ +⍨ ¯1 ¯2.5 0 2.5 4.3j1.1 ⋄ +⍨ (3 3⍴⍳9))
10 (2 4 6 8 10 12 14 16 18 20) (¯2 ¯5 0 5 8.6j2.2) (3 3⍴2 4 6 8 10 12 14 16 18)

⍝ aplcart/table.tsv:528 — Square: N*2
(×⍨ 5 ⋄ ×⍨ ⍳10 ⋄ ×⍨ ¯1 ¯2.5 0 2.5 4.3j1.1 ⋄ ×⍨ (3 3⍴⍳9))
25 (1 4 9 16 25 36 49 64 81 100) (1 6.25 0 6.25 17.28j9.46) (3 3⍴1 4 9 16 25 36 49 64 81)

⍝ aplcart/table.tsv:529 — Random Permutation of length Js; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Js←7 ⋄ r←?⍨Js ⋄ (⍳Js)≡r[⍋r]   ⍝ 1

⍝ aplcart/table.tsv:530 — Ones, same shape and structure
(3 3⍴⍳9 ⋄ '' ⋄ =⍨ 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 3 3⍴1 1 1 1 1 1 1 1 1)

⍝ aplcart/table.tsv:531 — Zeros, same shape and structure
(3 3⍴⍳9 ⋄ '' ⋄ ≠⍨ 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 3 3⍴0 0 0 0 0 0 0 0 0)

⍝ aplcart/table.tsv:532 — Increment: N+1
(1∘+ 10 20 30 ⋄ 1∘+ ¯10 ¯20 ¯30 ⋄ (3 3⍴4) ⋄ 1∘+ (3 3⍴4))
(11 21 31 ⋄ ¯9 ¯19 ¯29 ⋄ 3 3⍴4 4 4 4 4 4 4 4 4 ⋄ 3 3⍴5 5 5 5 5 5 5 5 5)

⍝ aplcart/table.tsv:533 — 2's-complement bit-wise NOT
J←¯3 0 1 7 ⋄ ¯1∘-J   ⍝ 2 ¯1 ¯2 ¯8

⍝ aplcart/table.tsv:534 — Zero array of shape, size, and structure of N; dfns display import/wrappers omitted to test underlying arrays
(0∘× 1 2 3 4 5 ⋄ 0∘× (8 8 ⋄ 100 200 ⋄ 5.3 ¯6) ⋄ 0∘× 3 3⍴⍳9)
(0 0 0 0 0 ⋄ (0 0 ⋄ 0 0 ⋄ 0 0) ⋄ 3 3⍴0 0 0 0 0 0 0 0 0)

⍝ aplcart/table.tsv:535 — Percentage corresponding to rate N
N←0 0.5 1 ⋄ 100∘×N   ⍝ 0 50 100

⍝ aplcart/table.tsv:536 — Triple: 3×N; dfns display import/wrappers omitted to test underlying arrays
(3∘× 1 2 3 4 5 ⋄ 3∘× (8 8 ⋄ 100 200 ⋄ 5.3 ¯6) ⋄ 3∘× 3 3⍴⍳9)
(3 6 9 12 15 ⋄ (24 24 ⋄ 300 600 ⋄ 15.9 ¯18) ⋄ 3 3⍴3 6 9 12 15 18 21 24 27)

⍝ aplcart/table.tsv:537 — Ensure that N is non-negative (negatives become zero); dfns display import/wrappers omitted to test underlying arrays
(0∘⌈ ¯3 ¯2 ¯1 0 1 2 3 ⋄ 0∘⌈ (30 ¯30 ⋄ 100 200 ⋄ ¯5.3 ¯6) ⋄ 0∘⌈ ¯5+3 3⍴⍳9)
(0 0 0 0 1 2 3 ⋄ (30 0 ⋄ 100 200 ⋄ 0 0) ⋄ 3 3⍴0 0 0 0 0 1 2 3 4)

⍝ aplcart/table.tsv:538 — Ensure that N is non-positive (positives become zero); dfns display import/wrappers omitted to test underlying arrays
(0∘⌊ ¯3 ¯2 ¯1 0 1 2 3 ⋄ 0∘⌊ (30 ¯30 ⋄ 100 200 ⋄ ¯5.3 ¯6) ⋄ 0∘⌊ ¯5+3 3⍴⍳9)
(¯3 ¯2 ¯1 0 0 0 0 ⋄ (0 ¯30 ⋄ 0 0 ⋄ ¯5.3 ¯6) ⋄ 3 3⍴¯4 ¯3 ¯2 ¯1 0 0 0 0 0)

⍝ aplcart/table.tsv:539 — Rightmost neighbouring elements (cyclically)
Y←3 1 3 2 ⋄ 1∘⌽Y   ⍝ 1 3 2 3

⍝ aplcart/table.tsv:540 — Leftmost neighbouring elements (cyclically)
Y←3 1 3 2 ⋄ ¯1∘⌽Y   ⍝ 2 3 1 3

⍝ aplcart/table.tsv:541 — Fractional part of number; dfns display import/wrappers omitted to test underlying arrays
(1∘| 0.55 1.23 8.76 0 ⋄ 1∘| (3.3 ¯3.3 ⋄ 100.2 200.1 ⋄ ¯5.3 ¯6) ⋄ 1∘| 1.1×3 3⍴⍳9)
(0.55 0.23 0.7599999999999998 0 ⋄ (0.2999999999999998 0.7000000000000002 ⋄ 0.2000000000000028 0.09999999999999432 ⋄ 0.7000000000000002 0) ⋄ 3 3⍴0.1000000000000001 0.2000000000000002 0.3000000000000003 0.4000000000000004 0.5 0.6000000000000005 0.7000000000000011 0.8000000000000007 0.9000000000000004)

⍝ aplcart/table.tsv:542 — Last part (last three digits) of packed numeric code with digits ABBB
J←1234 5099 ⋄ 1000∘|J   ⍝ 234 99

⍝ aplcart/table.tsv:543 — Parity of J (is J odd?); dfns display import/wrappers omitted to test underlying arrays
(2∘| 1 2 3 4 5 6 ⋄ 2∘| (51 ¯25 ⋄ 103 3 ⋄ 4 5) ⋄ 2∘| 3 3⍴⍳9)
(1 0 1 0 1 0 ⋄ (1 1 ⋄ 1 1 ⋄ 0 1) ⋄ 3 3⍴1 0 1 0 1 0 1 0 1)

⍝ aplcart/table.tsv:544 — Convert from signed short integers to unsigned short integers
J←¯128 ¯1 0 127 ⋄ 256∘|J   ⍝ 128 255 0 127

⍝ aplcart/table.tsv:545 — Strictly positive?
(0∘< 1 2 3 4 ⋄ 0∘< ¯2 ¯1 0 1 2)   ⍝ (1 1 1 1 ⋄ 0 0 0 1 1)

⍝ aplcart/table.tsv:546 — Non-negative?
(0∘≤ 1 2 3 4 ⋄ 0∘≤ ¯2 ¯1 0 1 2)   ⍝ (1 1 1 1 ⋄ 0 0 1 1 1)

⍝ aplcart/table.tsv:547 — Zero?
(0∘= 1 2 3 4 ⋄ 0∘= ¯2 ¯1 0 1 2)   ⍝ (0 0 0 0 ⋄ 0 0 1 0 0)

⍝ aplcart/table.tsv:548 — Non-positive?
(0∘≥ 1 2 3 4 ⋄ 0∘≥ ¯2 ¯1 0 1 2)   ⍝ (0 0 0 0 ⋄ 1 1 1 0 0)

⍝ aplcart/table.tsv:549 — Strictly negative?
(0∘> 1 2 3 4 ⋄ 0∘> ¯2 ¯1 0 1 2)   ⍝ (0 0 0 0 ⋄ 1 1 0 0 0)

⍝ aplcart/table.tsv:550 — Non-zero?
(0∘≠ 1 2 3 4 ⋄ 0∘≠ ¯2 ¯1 0 1 2)   ⍝ (1 1 1 1 ⋄ 1 1 0 1 1)

⍝ aplcart/table.tsv:551 — Behead: Remove first major cell; dfns display import/wrappers omitted to test underlying arrays
(1∘↓ 1 2 3 4 ⋄ 1∘↓ (50 80 ⋄ 10 20 ⋄ 33 66) ⋄ 1∘↓ 3 3⍴⍳9)
(2 3 4 ⋄ (10 20 ⋄ 33 66) ⋄ 2 3⍴4 5 6 7 8 9)

⍝ aplcart/table.tsv:552 — Curtail: Remove last major cell; dfns display import/wrappers omitted to test underlying arrays
(¯1∘↓ 1 2 3 4 ⋄ ¯1∘↓ (50 80 ⋄ 10 20 ⋄ 33 66) ⋄ ¯1∘↓ 3 3⍴⍳9)
(1 2 3 ⋄ (50 80 ⋄ 10 20) ⋄ 2 3⍴1 2 3 4 5 6)

⍝ aplcart/table.tsv:553 — First element of Y as a scalar; dfns display import/wrappers omitted to test underlying arrays
(⍬∘⍴ 1 2 3 4 ⋄ ⍬∘⍴ (50 80 ⋄ 10 20 ⋄ 33 66) ⋄ ⍬∘⍴ 3 3⍴⍳9)
⊂¨1 (50 80) 1

⍝ aplcart/table.tsv:554 — Common anti-logarithm
(10∘* 1 2 3 4 ⋄ 10∘* ¯1 ¯2 ¯3 ¯4 ⋄ 10∘* 1.32 1.54 1.75 1.99)
(10 100 1000 10000 ⋄ 0.1 0.01 0.001 0.0001 ⋄ 20.8929613085404 34.67368504525317 56.23413251903491 97.72372209558107)

⍝ aplcart/table.tsv:555 — Common logarithm
(10∘⍟ 1 10 100 1000 10000 1e5 1e6 ⋄ 10∘⍟ 132 15454 17501 199999)
(0 1 2 3 4 5 5.999999999999999 ⋄ 2.12057393120585 4.18904090790901 4.243062864804807 5.301027824186143)

⍝ aplcart/table.tsv:556 — cos ↔ sin: (1-N*2)*.5 (more precise cos arcsin N or sin arccos N)
N←0.25 0.5 0.75 ⋄ 0∘○N
0.9682458365518543 0.8660254037844386 0.6614378277661477

⍝ aplcart/table.tsv:557 — Triangle side as function of side (hypotenuse≤1)
N←0.25 0.5 0.75 ⋄ 0∘○N
0.9682458365518543 0.8660254037844386 0.6614378277661477

⍝ aplcart/table.tsv:558 — Sine N
N←0.25 0.5 0.75 ⋄ 1∘○N
0.2474039592545229 0.479425538604203 0.6816387600233341

⍝ aplcart/table.tsv:559 — Magnitude of N
N←3j4 0j2 ⋄ 10∘○N   ⍝ 5 2

⍝ aplcart/table.tsv:560 — Imaginary part of N
N←3j4 0j2 ⋄ 11∘○N   ⍝ 4 2

⍝ aplcart/table.tsv:561 — Phase of N
N←3j4 0j2 ⋄ 12∘○N   ⍝ 0.9272952180016122 1.570796326794897

⍝ aplcart/table.tsv:562 — Cosine N
N←0.25 0.5 0.75 ⋄ 2∘○N
0.9689124217106447 0.8775825618903728 0.7316888688738209

⍝ aplcart/table.tsv:563 — Tangent N
N←0.25 0.5 0.75 ⋄ 3∘○N
0.2553419212210363 0.5463024898437905 0.9315964599440724

⍝ aplcart/table.tsv:564 — sinh → cosh: (1+N*2)*.5 (more precise cosh arsinh N or sec arctan N)
N←0.25 0.5 0.75 ⋄ 4∘○N
1.030776406404415 1.118033988749895 1.25

⍝ aplcart/table.tsv:565 — Triangle hypotenuse as function of side ratio
N←0.25 0.5 0.75 ⋄ 4∘○N
1.030776406404415 1.118033988749895 1.25

⍝ aplcart/table.tsv:566 — Hyperbolic sine N
N←0.25 0.5 0.75 ⋄ 5∘○N
0.2526123168081683 0.5210953054937474 0.82231673193583

⍝ aplcart/table.tsv:567 — Hyperbolic cosine N
N←0.25 0.5 0.75 ⋄ 6∘○N
1.031413099879573 1.127625965206381 1.294683284676845

⍝ aplcart/table.tsv:568 — Hyperbolic tangent N
N←0.25 0.5 0.75 ⋄ 7∘○N
0.2449186624037091 0.4621171572600097 0.6351489523872873

⍝ aplcart/table.tsv:569 — icos ↔ isin: (-1+N*2)*.5
N←0.25 0.5 0.75 ⋄ 8∘○N
0j1.030776406404415 0j1.118033988749895 0j1.25

⍝ aplcart/table.tsv:570 — Real part of N
N←3j4 0j2 ⋄ 9∘○N   ⍝ 3 0

⍝ aplcart/table.tsv:571 — Arcsine N
N←0.25 0.5 0.75 ⋄ ¯1∘○N
0.2526802551420786 0.5235987755982988 0.848062078981481

⍝ aplcart/table.tsv:572 — +N (complex conjugate)
N←3j4 0j2 ⋄ ¯10∘○N   ⍝ 3j¯4 0j¯2

⍝ aplcart/table.tsv:573 — N×0J1
N←3j4 0j2 ⋄ ¯11∘○N   ⍝ ¯4j3 ¯2

⍝ aplcart/table.tsv:574 — *N×0J1
N←0 0.5 1 ⋄ ¯12∘○N
1 0.8775825618903728j0.479425538604203 0.5403023058681398j0.8414709848078965

⍝ aplcart/table.tsv:575 — Arccosine N
N←0.25 0.5 0.75 ⋄ ¯2∘○N
1.318116071652818 1.047197551196598 0.7227342478134157

⍝ aplcart/table.tsv:576 — Arctangent N
N←0.25 0.5 0.75 ⋄ ¯3∘○N
0.2449786631268641 0.4636476090008061 0.6435011087932844

⍝ aplcart/table.tsv:577 — cosh → sinh: (N+1)×((N-1)÷N+1)*.5 (more precise sinh arcosh N or tan arcsec N)
N←1 2 3 ⋄ ¯4∘○N   ⍝ 0 1.732050807568877 2.82842712474619

⍝ aplcart/table.tsv:578 — Triangle side (≥1) as function of hypotenuse
N←1 2 3 ⋄ ¯4∘○N   ⍝ 0 1.732050807568877 2.82842712474619

⍝ aplcart/table.tsv:579 — Hyperbolic arsin N
N←0.25 0.5 0.75 ⋄ ¯5∘○N
0.2474664615472635 0.4812118250596035 0.6931471805599453

⍝ aplcart/table.tsv:580 — Hyperbolic arcos N
N←1 2 3 ⋄ ¯6∘○N   ⍝ 0 1.316957896924817 1.762747174039086

⍝ aplcart/table.tsv:581 — Hyperbolic artan N
N←0.25 0.5 0.75 ⋄ ¯7∘○N
0.2554128118829953 0.5493061443340549 0.9729550745276566

⍝ aplcart/table.tsv:582 — icos ↔ -isin: -(-1+N*2)*.5
N←0.25 0.5 0.75 ⋄ ¯8∘○N
0j¯1.030776406404415 0j¯1.118033988749895 0j¯1.25

⍝ aplcart/table.tsv:583 — N (identity)
N←0.25 0.5 0.75 ⋄ ¯9∘○N   ⍝ 0.25 0.5 0.75

⍝ aplcart/table.tsv:584 — Hook (S-combinator): apply f between Y and (g Y), that is Y f g Y
f←- ⋄ g←⌽ ⋄ Y←1 2 3 ⋄ f∘g⍨Y   ⍝ ¯2 0 2

⍝ aplcart/table.tsv:585 — Split-Compose (D₂-combinator): apply g between (f X) and (h Y), that is (f X) g (h Y)
f←+/ ⋄ g←- ⋄ h←×/ ⋄ X←1 2 3 ⋄ Y←2 3 4 ⋄ X f⍛g∘h Y
¯18

⍝ aplcart/table.tsv:586 — Fast: The last sub-array along the last axis of Y
(⊢/ 1 2 3 4 5 ⋄ '' ⋄ 3 3⍴⍳9 ⋄ '' ⋄ ⊢/ 3 3⍴⍳9)
5 ('') (3 3⍴1 2 3 4 5 6 7 8 9) ('') (3 6 9)

⍝ aplcart/table.tsv:587 — Fast: The first sub-array along the last axis of Y
(⊣/ 1 2 3 4 5 ⋄ '' ⋄ 3 3⍴⍳9 ⋄ '' ⋄ ⊣/ 3 3⍴⍳9)
1 ('') (3 3⍴1 2 3 4 5 6 7 8 9) ('') (1 4 7)

⍝ aplcart/table.tsv:588 — Sum of N (row-wise)
(+/ 1 2 3 4 5 ⋄ '' ⋄ 3 3⍴⍳9 ⋄ '' ⋄ +/ 3 3⍴⍳9)
15 ('') (3 3⍴1 2 3 4 5 6 7 8 9) ('') (6 15 24)

⍝ aplcart/table.tsv:589 — Running sum of Is consecutive elements of N
((+/∘(2∘↕)) 2 2 3 2 2 ⋄ '' ⋄ 5 5⍴⍳9 ⋄ '' ⋄ (+/∘(3∘↕)⍤1) 5 5⍴⍳9)
(4 5 5 4 ⋄ '' ⋄ 5 5⍴1 2 3 4 5 6 7 8 9 1 2 3 4 5 6 7 8 9 1 2 3 4 5 6 7 ⋄ '' ⋄ 5 3⍴6 9 12 21 24 18 9 12 15 24 18 12 12 15 18)

⍝ aplcart/table.tsv:590 — Row-wise alternating sum: ((N[1]-N[2])+N[3])-N[4]+…
-/ 5 10 15 20 25 30   ⍝ ¯15

⍝ aplcart/table.tsv:591 — Product of N (row-wise)
(×/ 1 2 3 4 5 ⋄ '' ⋄ 5 5⍴⍳25 ⋄ '' ⋄ ×/ 5 5⍴⍳25)
120 ('') (5 5⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25) ('') (120 30240 360360 1860480 6375600)

⍝ aplcart/table.tsv:592 — Area of rectangle with sides Nv
×/ 5 10   ⍝ 50

⍝ aplcart/table.tsv:593 — Volume of box with sides Nv
×/ 10 10 2   ⍝ 200

⍝ aplcart/table.tsv:594 — Are any true? (row-wise)
(∨/ 0 0 0 0 0 ⋄ ∨/ 0 0 0 1 0 0 0 ⋄ ∨/ 1 1 1)
0 1 1

⍝ aplcart/table.tsv:595 — Are all true?
(∧/ 0 0 0 0 0 ⋄ ∧/ 0 0 0 1 0 0 0 ⋄ ∧/ 1 1 1)
0 0 1

⍝ aplcart/table.tsv:596 — Maximum of N
(⌈/ 58 15 22 80 26 11 ⋄ ⌈/ 0 0 0 1 0 0 0 ⋄ 3 3⍴ 2 1 3 6 5 4 7 9 8 ⋄ '' ⋄ ⌈/ 3 3⍴ 2 1 3 6 5 4 7 9 8)
80 1 (3 3⍴2 1 3 6 5 4 7 9 8) ('') (3 6 9)

⍝ aplcart/table.tsv:597 — Minimum of N
(⌊/ 58 15 22 80 26 11 ⋄ ⌊/ 0 0 0 1 0 0 0 ⋄ 3 3⍴ 2 1 3 6 5 4 7 9 8 ⋄ '' ⋄ ⌊/ 3 3⍴ 2 1 3 6 5 4 7 9 8)
11 0 (3 3⍴2 1 3 6 5 4 7 9 8) ('') (1 4 7)

⍝ aplcart/table.tsv:598 — Alternating product of N
(÷/ 1 2 ⋄ ÷/ 1 2 3 ⋄ ÷/ 1 2 3 4 ⋄ ÷/ 1 2 3 4 5)
0.5 1.5 0.375 1.875

⍝ aplcart/table.tsv:599 — Boolean Parity (even number of 1s in vectors)
(≠/1 ⋄ ≠/1 1 ⋄ ≠/1 1 1 ⋄ ≠/1 0 0 1 0 1)   ⍝ 1 0 1 1

⍝ aplcart/table.tsv:600 — Ensure minimum rank 1 (reshaping scalar into one-element vector)
(⍴     27 ⋄ ⍴ 1∘/ 27 ⋄ ⍴     27 30 35 ⋄ ⍴ 1∘/ 27 30 35)
(⍬ ⋄ 1⍴1 ⋄ 1⍴3 ⋄ 1⍴3)

⍝ aplcart/table.tsv:601 — Fast: The last sub-array along the first axis of Y
(3 3⍴⍳9 ⋄ '' ⋄ ⊢⌿ 3 3⍴⍳9)   ⍝ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 7 8 9)

⍝ aplcart/table.tsv:602 — Fast: The first sub-array along the first axis of Y
(3 3⍴⍳9 ⋄ '' ⋄ ⊣⌿ 3 3⍴⍳9)   ⍝ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 1 2 3)

⍝ aplcart/table.tsv:603 — Sum of N (column-wise)
(3 3⍴1 2 3 ⋄ '' ⋄ +⌿ 3 3⍴1 2 3)   ⍝ (3 3⍴1 2 3 1 2 3 1 2 3 ⋄ '' ⋄ 3 6 9)

⍝ aplcart/table.tsv:604 — Column-wise alternating sum: ((N[1]-N[2])+N[3])-N[4]+…
(5 3⍴3×⍳5 ⋄ '' ⋄ -⌿ 5 3⍴3×⍳5)
(5 3⍴3 6 9 12 15 3 6 9 12 15 3 6 9 12 15 ⋄ '' ⋄ ¯9 9 27)

⍝ aplcart/table.tsv:605 — Product of N (column-wise)
(4 3⍴⍳12 ⋄ '' ⋄ ×⌿ 4 3⍴⍳12)
(4 3⍴1 2 3 4 5 6 7 8 9 10 11 12 ⋄ '' ⋄ 280 880 1944)

⍝ aplcart/table.tsv:606 — Empty array along first axis
(⍴     4 3⍴⍳12 ⋄ '' ⋄ ⍴ 0∘⌿ 4 3⍴⍳12)   ⍝ (4 3 ⋄ '' ⋄ 0 3)

⍝ aplcart/table.tsv:607 — Ignore left argument (call f monadically on Y)
f←+/ ⋄ X←1 2 ⋄ Y←3 4 ⋄ X f⍤⊢Y   ⍝ 7

⍝ aplcart/table.tsv:608 — Ignore right argument (call f monadically on X)
f←+/ ⋄ X←1 2 ⋄ Y←3 4 ⋄ X f⍤⊣Y   ⍝ 3

⍝ aplcart/table.tsv:609 — Preface a column of 1s
(1∘, 5 6 7 8 ⋄ '' ⋄ 1∘, 3 3⍴⍳9)
(1 5 6 7 8 ⋄ '' ⋄ 3 4⍴1 1 2 3 1 4 5 6 1 7 8 9)

⍝ aplcart/table.tsv:610 — Ensure that all elements are vectors; dfns display import/wrappers omitted to test underlying arrays
(⍳5 ⋄ ,¨ ⍳5)
(1 2 3 4 5 ⋄ (1⍴1 ⋄ 1⍴2 ⋄ 1⍴3 ⋄ 1⍴4 ⋄ 1⍴5))

⍝ aplcart/table.tsv:611 — Join corresponding items; dfns display import/wrappers omitted to test underlying arrays
(5 6 7 ,¨ 10 11 12 ⋄ 5 6 7 ,¨ 15)
((5 10 ⋄ 6 11 ⋄ 7 12) ⋄ (5 15 ⋄ 6 15 ⋄ 7 15))

⍝ aplcart/table.tsv:612 — Catenate items of Yv along their last axes; dfns display import/wrappers omitted to test underlying arrays
((9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ ,/ (9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ (3 3⍴⍳9 ⋄ 3 3⍴⍳9) ⋄ ,/ (3 3⍴⍳9 ⋄ 3 3⍴⍳9))
((9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ (9 10 20 40 60 4 5) ⋄ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴1 2 3 4 5 6 7 8 9) ⋄ (3 6⍴1 2 3 1 2 3 4 5 6 4 5 6 7 8 9 7 8 9))

⍝ aplcart/table.tsv:613 — All possible subvectors of length Is (Yv must be simple); dfns display import/wrappers omitted to test underlying arrays
(2 4 6 8 10 12 ⋄ ,/2↕  2 4 6 8 10 12 ⋄ ,/4↕  2 4 6 8 10 12)
(2 4 6 8 10 12 ⋄ (2 4 ⋄ 4 6 ⋄ 6 8 ⋄ 8 10 ⋄ 10 12) ⋄ (2 4 6 8 ⋄ 4 6 8 10 ⋄ 6 8 10 12))

⍝ aplcart/table.tsv:614 — Preface a row of 1s
1∘⍪ 2 3⍴8   ⍝ 3 3⍴1 1 1 8 8 8 8 8 8

⍝ aplcart/table.tsv:615 — Catenate items of Yv along their first axes; dfns display import/wrappers omitted to test underlying arrays
((9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ ⍪/ (9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ (3 3⍴⍳9 ⋄ 3 3⍴⍳9) ⋄ ⍪/ (3 3⍴⍳9 ⋄ 3 3⍴⍳9))
((9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ (9 10 20 40 60 4 5) ⋄ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴1 2 3 4 5 6 7 8 9) ⋄ (6 3⍴1 2 3 4 5 6 7 8 9 1 2 3 4 5 6 7 8 9))

⍝ aplcart/table.tsv:616 — First element (as vector) and remaining elements
Y←1 2 3 4 5 ⋄ 1 1∘⊂Y   ⍝ (1⍴1 ⋄ 2 3 4 5)

⍝ aplcart/table.tsv:617 — Head: First major cell of Y
Y←2 3⍴⍳6 ⋄ 1∘⌷Y   ⍝ 1 2 3

⍝ aplcart/table.tsv:618 — Assign ranking based on non-descending scores Nv (ties all get highest ranking of used slots)
⍳⍨ 3 3 3 5 8 8 21   ⍝ 1 1 1 4 5 5 7

⍝ aplcart/table.tsv:619 — Not all true?
(0∘∊ 1 1 1 1 ⋄ 0∘∊ 1 1 0 1)   ⍝ 0 1

⍝ aplcart/table.tsv:620 — Any true?
(1∘∊ 0 0 0 0 ⋄ 1∘∊ 0 1 1 0)   ⍝ 0 1

⍝ aplcart/table.tsv:621 — Assign ranking based on non-descending scores Nv (ties all get lowest ranking of used slots)
⍸⍨ 3 3 3 5 8 8 21   ⍝ 3 3 3 4 6 6 7

⍝ aplcart/table.tsv:622 — Limit: apply X∘f until stable
X←2 ⋄ f←⌈ ⋄ Y←¯1 0 3 ⋄ X f⍣≡Y   ⍝ 2 2 3

⍝ aplcart/table.tsv:623 — Limit: apply f until stable
f←⌊ ⋄ Y←¯1.2 0 3.9 ⋄ f⍣≡Y   ⍝ ¯2 0 3

⍝ aplcart/table.tsv:624 — Cumulative sum
(+\ 1 1 1 1 1 ⋄ +\ 2 4 6 8 10)   ⍝ (1 2 3 4 5 ⋄ 2 6 12 20 30)

⍝ aplcart/table.tsv:625 — Turn on all 0s after first 1 (indicate all elements except leading 0s)
∨\ 0 0 1 0 0 1 1 1   ⍝ 0 0 1 1 1 1 1 1

⍝ aplcart/table.tsv:626 — Turn off all 1s after first 0 (indicate all elements until the first 0)
∧\ 1 1 0 0 1 1 1   ⍝ 1 1 0 0 0 0 0

⍝ aplcart/table.tsv:627 — Progressive maxima (row-wise)
⌈\ 20 11 47 2 5 300 99   ⍝ 20 20 47 47 47 300 300

⍝ aplcart/table.tsv:628 — Progressive minima (row-wise)
⌊\ 20 11 47 2 5 300 99   ⍝ 20 11 11 2 2 2 2

⍝ aplcart/table.tsv:629 — Turn off all 1s after first 1 (indicate only the first 1); First-true masks use cumulative counts under basedpl left scan
{⍵∧1=+\⍵} 0 1 0 1 0 1   ⍝ 0 1 0 0 0 0

⍝ aplcart/table.tsv:630 — Turn on all 0s after first 0 (indicate all elements except the first 0); First-zero mask uses cumulative counts under basedpl left scan
{⍵∨1≠+\~⍵} 1 0 1 0 1 0   ⍝ 1 0 1 1 1 1

⍝ aplcart/table.tsv:631 — Convert reflected Gray code to binary
B←1 0 1 1 0 0 1 ⋄ ≠\B   ⍝ 1 1 0 1 1 1 0

⍝ aplcart/table.tsv:632 — Parity: Connect odd and even ones
B←1 0 1 1 0 0 1 ⋄ ≠\B   ⍝ 1 1 0 1 1 1 0

⍝ aplcart/table.tsv:633 — Progressive maxima (column-wise)
col←⍪20 11 47 2 5 300 99 ⋄ col, ⌈⍀ col
7 2⍴20 20 11 20 47 47 2 47 5 47 300 300 99 300

⍝ aplcart/table.tsv:634 — Progressive minima (column-wise)
col←⍪20 11 47 2 5 300 99 ⋄ col, ⌊⍀ col
7 2⍴20 20 11 11 47 11 2 2 5 2 300 2 99 2

⍝ aplcart/table.tsv:635 — Main diagonal of matrix
(3 3⍴⍳9 ⋄ '' ⋄ 1 1∘⍉ 3 3⍴⍳9)   ⍝ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 1 5 9)

⍝ aplcart/table.tsv:636 — Count of trailing ones
⊥⍨ 1 0 1 0 1 1 1   ⍝ 3

⍝ aplcart/table.tsv:637 — Last major cell of numeric array
(0∘⊥ ⍳9 ⋄ 0∘⊥ 3 3⍴⍳9)   ⍝ 9 (7 8 9)

⍝ aplcart/table.tsv:638 — Joining date YYYY M D to packed YYYYMMDD integer
100∘⊥ 1969 7 21   ⍝ 19690721

⍝ aplcart/table.tsv:639 — Integer representation of logical vector Bv
(2∘⊥ 0 0 1 ⋄ 2∘⊥ 0 1 0 ⋄ 2∘⊥ 0 1 1 ⋄ 2∘⊥ 1 0 0)
1 2 3 4

⍝ aplcart/table.tsv:640 — Integral and fractional part of positive number
0 1∘⊤ 8.75   ⍝ 8 0.75

⍝ aplcart/table.tsv:641 — Separating packed YYYYMMDD date integer date
0 100 100∘⊤ 19690721   ⍝ 1969 7 21

⍝ aplcart/table.tsv:657 — Caseless operation
C←'ABC'  ⋄ f←≡ ⋄ D←'Abc 19 Σς!'  ⋄ C f⍥•C D
0

⍝ aplcart/table.tsv:658 — Fast: 0 irrespective of Y
{0} (4 5 6) 'ABC'   ⍝ 0

⍝ aplcart/table.tsv:659 — Negate real part (“real conjugate”)
+∘- ¯7J3   ⍝ 7j3

⍝ aplcart/table.tsv:660 — Mirror complex N across y-axis
-∘+ ¯7J3   ⍝ 7j3

⍝ aplcart/table.tsv:661 — Rotate 180°
(3 3⍴⍳9 ⋄ '' ⋄ ⌽∘⊖  3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 3 3⍴9 8 7 6 5 4 3 2 1)

⍝ aplcart/table.tsv:662 — Division: force DOMAIN ERROR for division by 0
M←2 3 ⋄ N←4 6 ⋄ M×∘÷N   ⍝ 0.5 0.5

⍝ aplcart/table.tsv:663 — Conditional drop of last element of Y
As←1 ⋄ Y←2 3⍴⍳6 ⋄ As-⍛↓Y   ⍝ 1 3⍴1 2 3

⍝ aplcart/table.tsv:664 — Padding Yv on the left to width Is
Is←7 ⋄ Yv←1 2 3 ⋄ Is-⍛↑Yv   ⍝ 0 0 0 0 1 2 3

⍝ aplcart/table.tsv:665 — Vertically lengthening matrix Ym to be compatible (for ,) with Xm
Xm←3 2⍴0 ⋄ Ym←2 2⍴⍳4 ⋄ Xm≢⍛↑Ym   ⍝ 3 2⍴1 2 3 4 0 0

⍝ aplcart/table.tsv:666 — Array with shape of X and content of Y
X←2 3⍴0 ⋄ Y←1 2 ⋄ X⍴⍛⍴Y   ⍝ 2 3⍴1 2 1 2 1 2

⍝ aplcart/table.tsv:667 — Replicate along last axis of Y (forces / to be a function even with a function on its left)
3 1 ¯2 2 / 6 7 8 9   ⍝ 6 6 6 7 0 0 9 9

⍝ aplcart/table.tsv:668 — Filtering columns of Y according to mask Av (forces / to be a function even with a function on its left)
1 0 1 0 1 / 'Heart'   ⍝ 'Hat'

⍝ aplcart/table.tsv:669 — Replicate along leading axis of Y (forces ⌿ to be a function even with a function on its left)
mat ← 3 4⍴⍳12 ⋄ 1 0 2 ⌿ mat   ⍝ 3 4⍴1 2 3 4 9 10 11 12 9 10 11 12

⍝ aplcart/table.tsv:670 — Filtering major cells of Y according to mask Av (forces ⌿ to be a function even with a function on its left)
mat ← 3 4⍴⍳12 ⋄ 1 0 1 ⌿ mat   ⍝ 2 4⍴1 2 3 4 9 10 11 12

⍝ aplcart/table.tsv:671 — Create vector of elements in array Y selected by integer array I of the same shape
I←2 3⍴1 0 2 1 0 1 ⋄ Y←2 3⍴⍳6 ⋄ I/⍥,Y   ⍝ 1 3 3 4 6

⍝ aplcart/table.tsv:672 — Matrix with Is columns, each consisting of Yv
4 /∘⍪ 1 2 3   ⍝ 3 4⍴1 1 1 1 2 2 2 2 3 3 3 3

⍝ aplcart/table.tsv:673 — Array of shape Iv filled with copies of Y
2 3 ⍴∘⊂ 'abc'
2 3⍴('abc' ⋄ 'abc' ⋄ 'abc' ⋄ 'abc' ⋄ 'abc' ⋄ 'abc')

⍝ aplcart/table.tsv:674 — Corner element of a (non-empty) array Y[1;1;1…]
(3 3⍴ (,/∘(2∘↕)) ⍳9 ⋄ '' ⋄ ↑∘,  3 3⍴ (,/∘(2∘↕)) ⍳9)
(3 3⍴(1 2 ⋄ 2 3 ⋄ 3 4 ⋄ 4 5 ⋄ 5 6 ⋄ 6 7 ⋄ 7 8 ⋄ 8 9 ⋄ 1 2) ⋄ '' ⋄ 1 2)

⍝ aplcart/table.tsv:675 — Select major cells Iv from Y
Iv←3 1 2 ⋄ Y←3 2⍴⍳6 ⋄ Iv⊂⍛⌷Y   ⍝ 3 2⍴5 6 1 2 3 4

⍝ aplcart/table.tsv:676 — Permute: Reorder major cells of Y according to permutation vector Iv
Iv←3 1 2 ⋄ Y←3 2⍴⍳6 ⋄ Iv⊂⍛⌷Y   ⍝ 3 2⍴5 6 1 2 3 4

⍝ aplcart/table.tsv:677 — Generate consolidated left argument for successive transposes Jv⍉Iv⍉Y
Iv←2 3 1 ⋄ Jv←3 1 2 ⋄ Iv⊂⍛⌷Jv   ⍝ 1 2 3

⍝ aplcart/table.tsv:678 — Arithmetic progression vector: Js steps of Ms
(⊂3 3⍴⍳9) ×∘⍳ 3
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴2 4 6 8 10 12 14 16 18 ⋄ 3 3⍴3 6 9 12 15 18 21 24 27)

⍝ aplcart/table.tsv:679 — All row indices of matrix Ym
(4 3⍴0.25 ⋄ ⍳∘≢ 4 3⍴0.25)
(4 3⍴0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 ⋄ 1 2 3 4)

⍝ aplcart/table.tsv:680 — Index of last occurrence of major cells Y in X, counted from the rear
X←4 2⍴1 2 3 4 1 2 5 6 ⋄ Y←3 2⍴1 2 5 6 7 8 ⋄ X⊖⍛⍳Y
2 1 5

⍝ aplcart/table.tsv:681 — All tuples of corresponding elements of ⍳¨Jv (for small Jv of max length 15)
,∘⍳ 3 6
(1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 1 4 ⋄ 1 5 ⋄ 1 6 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3 ⋄ 2 4 ⋄ 2 5 ⋄ 2 6 ⋄ 3 1 ⋄ 3 2 ⋄ 3 3 ⋄ 3 4 ⋄ 3 5 ⋄ 3 6)

⍝ aplcart/table.tsv:682 — Position of first occurrence of string Dv in list of strings C
'lorem' 'ipsum' 'dolor' 'sit' 'amet' ⍳∘⊂  'sit'
4

⍝ aplcart/table.tsv:683 — Is string Cv a member of list of strings D
Cv←'cat' ⋄ D←'dog' 'cat' 'eel'  ⋄ Cv⊂⍛∊D
1

⍝ aplcart/table.tsv:684 — Is Ms in range 1…Js?
27 ∊∘⍳ 50   ⍝ 1

⍝ aplcart/table.tsv:685 — Boolean array of shape Iv with ones in locations Jv (inverse of Jv←⍸Bv)
2 4 7 ∊⍨∘⍳ 10   ⍝ 0 1 0 1 0 0 1 0 0 0

⍝ aplcart/table.tsv:686 — Replace 1s in Boolean array B with their enumeration
⍸@⊢ 1 0 1 1 0 0 1 1   ⍝ 1 0 2 3 0 0 4 5

⍝ aplcart/table.tsv:687 — Cut Yv into non-empty partitions of length Iv (+/Iv ↔ ⍴Y)
Iv←2 0 3 ⋄ Y←⍳5 ⋄ Iv⍸⍛⊆Y   ⍝ (1 2 ⋄ 3 4 5)

⍝ aplcart/table.tsv:688 — Limit: apply inverse of X∘f until stable; output expression returned directly
f←+∘÷ ⋄ 1 f⍣¯1⍣≡0   ⍝ ¯0.618033988749894

⍝ aplcart/table.tsv:689 — Limit: apply inverse of f until stable; output expression returned directly
f←1∘+∘÷ ⋄ f⍣¯1⍣≡0   ⍝ ¯0.618033988749894

⍝ aplcart/table.tsv:690 — Permutation vector that sorts like Y
Y←3 1 2 1 ⋄ ⍋∘⍋Y   ⍝ 4 1 3 2

⍝ aplcart/table.tsv:691 — Expand last axis of Y (forces \ to be a function even with a function on its left)
(3 ¯2 4 \ 7 8 ⋄ 1 0 1 0 1 \ 'Hat')   ⍝ (7 7 7 0 0 8 8 8 8 ⋄ 'H a t')

⍝ aplcart/table.tsv:692 — Expand leading axis of Y (forces ⍀ to be a function even with a function on its left)
mat←3 4⍴⍳12 ⋄ 1 0 2 1 ⍀ mat
5 4⍴1 2 3 4 0 0 0 0 5 6 7 8 5 6 7 8 9 10 11 12

⍝ aplcart/table.tsv:693 — Enclose columns of a matrix; dfns display import/wrappers omitted to test underlying arrays
(3 3⍴⍳9 ⋄ ⊂[1] 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ (1 4 7 ⋄ 2 5 8 ⋄ 3 6 9))

⍝ aplcart/table.tsv:694 — Convert table to inverted table (character data as vectors of vectors); dfns display import/wrappers omitted to test underlying arrays
(⊃¨⊂[1]) 2 3⍴'Ab' 1 7 'Cdef' 2 3   ⍝ (2 4⍴'Ab  Cdef' ⋄ 1 2 ⋄ 7 3)

⍝ aplcart/table.tsv:695 — Conjugate Transpose
Nm←2 2⍴1j2 3j4 5j6 7j8 ⋄ ⍉∘+Nm   ⍝ 2 2⍴1j¯2 5j¯6 3j¯4 7j¯8

⍝ aplcart/table.tsv:696 — Rotate 90° clockwise
(3 3⍴⍳9 ⋄ ⌽∘⍉ 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴7 4 1 8 5 2 9 6 3)

⍝ aplcart/table.tsv:697 — Rotate 90° counter-clockwise
(3 3⍴⍳9 ⋄ ⍉∘⌽ 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴3 6 9 2 5 8 1 4 7)

⍝ aplcart/table.tsv:698 — Matrix with columns from vectors Yv
Yv←(1 2⋄ 3 4 5) ⋄ ⍉∘⊃Yv   ⍝ 3 2⍴1 3 2 4 0 5

⍝ aplcart/table.tsv:699 — Is Ym symmetric?
Ym←3 3⍴1 2 3 2 4 5 3 5 6 ⋄ ⍉⍛≡Ym   ⍝ 1

⍝ aplcart/table.tsv:700 — Forming first row of a matrix for later expansion
(⍴     1 2 3 ⋄ ⍴ ⍉∘⍪ 1 2 3)   ⍝ (1⍴3 ⋄ 1 3)

⍝ aplcart/table.tsv:701 — Reshaping vector Yv into a one-row matrix
(⍴     1 2 3 ⋄ ⍴ ⍉∘⍪ 1 2 3)   ⍝ (1⍴3 ⋄ 1 3)

⍝ aplcart/table.tsv:702 — Dot/Vector/Cross/Matrix Product of M and N (¯1↑⍴M ↔ 1↑⍴N)
M←2 3⍴⍳6 ⋄ N←3 2⍴⍳6 ⋄ M+.×N   ⍝ 2 2⍴22 28 49 64

⍝ aplcart/table.tsv:703 — Summation over subsets of Nv specified by rows of A
(4 3⍴1 0 1,0 1 1,1 1 1,1 0 0) +.× 3 1 4   ⍝ 7 5 8 3

⍝ aplcart/table.tsv:704 — Alternating Matrix Product of M and N (¯1↑⍴M ↔ 1↑⍴N)
M←2 3⍴⍳6 ⋄ N←3 2⍴⍳6 ⋄ M-.×N   ⍝ 2 2⍴10 12 19 24

⍝ aplcart/table.tsv:705 — Extending a transitive binary relation
Am←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ Bm←Am ⋄ Am∨.∧Bm
3 3⍴0 0 1 1 0 0 0 1 0

⍝ aplcart/table.tsv:706 — Maximum of Nv with weights Mv
Mv←1 2 3 ⋄ Nv←3 2 1 ⋄ Mv⌈.×Nv   ⍝ 4

⍝ aplcart/table.tsv:707 — Extending a distance table to next leg
Mm←3 3⍴0 2 5 2 0 1 5 1 0 ⋄ Nm←Mm ⋄ Mm⌊.+Nm
3 3⍴0 2 3 2 0 1 3 1 0

⍝ aplcart/table.tsv:708 — Minimum of Nv with weights Mv
Mv←1 2 3 ⋄ Nv←3 2 1 ⋄ Mv⌊.×Nv   ⍝ 3

⍝ aplcart/table.tsv:709 — Sum of reciprocal series Mv÷Nv
Mv←1 2 3 ⋄ Nv←1 2 3 ⋄ Mv+.÷Nv   ⍝ 3

⍝ aplcart/table.tsv:710 — Sum of alternating reciprocal series Mv÷Nv
Mv←1 2 3 ⋄ Nv←1 2 3 ⋄ Mv-.÷Nv   ⍝ 1

⍝ aplcart/table.tsv:711 — Counting pairwise matches (equal elements) in two vectors
Xv←1 2 3 4 ⋄ Yv←1 3 3 2 ⋄ Xv+.=Yv   ⍝ 2

⍝ aplcart/table.tsv:712 — Comparing vector Yv with rows of array X
m ← ⊃'Finn' 'Gary' 'Gary' 'Anna' 'Carl' 'Dana' 'Carl' 'Gary' 'Gary' 'Beau' ⋄ (m ⋄ m ∧.= 'Gary')
(10 4⍴'FinnGaryGaryAnnaCarlDanaCarlGaryGaryBeau' ⋄ 0 1 1 0 0 0 0 1 1 0)

⍝ aplcart/table.tsv:713 — Boolean rows of Xm all equal to scalar Ys
Xm←3 3⍴1 1 1 1 0 1 0 0 0 ⋄ Ys←1 ⋄ Xm∧.=Ys
1 0 0

⍝ aplcart/table.tsv:714 — Products over subsets of Nv specified by B
Nv←2 3 5 ⋄ B←3 4⍴1 0 1 0 0 1 1 0 0 0 1 0 ⋄ Nv×.*B
2 3 30 1

⍝ aplcart/table.tsv:715 — Addition Table for Iv down and Jv across
Iv←¯1 0 1 ⋄ Jv←2 3 ⋄ Iv +⌝ Jv   ⍝ 3 2⍴1 2 2 3 3 4

⍝ aplcart/table.tsv:716 — Multiplication Table for Iv down and Jv across
Iv←¯1 0 1 ⋄ Jv←2 3 ⋄ Iv ×⌝ Jv   ⍝ 3 2⍴¯2 ¯3 0 0 2 3

⍝ aplcart/table.tsv:717 — Mid product of M and N
M←2 3⍴⍳6 ⋄ N←3 2⍴⍳6 ⋄ M,.×N
2 2⍴(1 6 15 ⋄ 2 8 18 ⋄ 4 15 30 ⋄ 8 20 36)

⍝ aplcart/table.tsv:718 — Cartesian product: all pairs of X and Y
X←1 2 ⋄ Y←'ab'  ⋄ X ,⌝ Y   ⍝ 2 2⍴(1 'a' ⋄ 1 'b' ⋄ 2 'a' ⋄ 2 'b')

⍝ aplcart/table.tsv:719 — Ascendingly ordered Nv-coefficient polynomial at point Ms
Ms←2 ⋄ Nv←1 3 4 ⋄ Ms⊥∘⌽Nv   ⍝ 23

⍝ aplcart/table.tsv:720 — Evaluate polynomial with descending coefficients Nv for point(s) Mv
Mv←0 1 2 ⋄ Nv←1 3 4 ⋄ Mv⍪⍛⊥Nv   ⍝ 4 8 14

⍝ aplcart/table.tsv:721 — Is Y a simple character array?
Y←2 3⍴'abcdef'  ⋄ ⍕⍛≡Y   ⍝ 1

⍝ aplcart/table.tsv:722 — Convert character or numeric data into numeric (unsafe); Reviewed Execute example checked through the Rust reference worker
Yv←'1 2 3' ⋄ ⍎∘⍕Yv   ⍝ 1 2 3

⍝ aplcart/table.tsv:724 — Create a “without” function with a hashed principal argument (fast X~Y for subsequent values of X)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ name←~∘Y ⋄ name X
⍬

⍝ aplcart/table.tsv:725 — Create an “index-in” function with a hashed principal argument (fast X⍳Y for subsequent values of Y)
Y←3 1 3 2 ⋄ X←3 1 2 1 ⋄ name←X∘⍳ ⋄ name Y
1 2 1 3

⍝ aplcart/table.tsv:726 — Create a “membership-in” function with a hashed principal argument (fast X∊Y for subsequent values of X)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ name←∊∘Y ⋄ name X
1 1 1 1

⍝ aplcart/table.tsv:727 — Create a “union-with” function with a hashed principal argument (fast X∪Y for subsequent values of Y)
Y←3 1 3 2 ⋄ X←3 1 2 1 ⋄ name←X∘∪ ⋄ name Y
3 1 2 1

⍝ aplcart/table.tsv:728 — Create an “intersection-with” function with a hashed principal argument (fast X∩Y for subsequent values of X)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ name←∩∘Y ⋄ name X
3 1 2 1

⍝ aplcart/table.tsv:729 — Create a “grade-ascending-according-to” function with a hashed principal argument (fast X⍋Y for subsequent values of Y)
Y← 'abcac'  ⋄ X← 'cba'  ⋄ name←X∘⍋ ⋄ name Y
3 5 2 1 4

⍝ aplcart/table.tsv:730 — Create a “grade decending according to” function with a hashed principal argument (fast X⍒Y for subsequent values of Y)
Y← 'abcac'  ⋄ X← 'cba'  ⋄ name←X∘⍒ ⋄ name Y
1 4 2 3 5

⍝ aplcart/table.tsv:731 — Fast: 0 corresponding to each item of Y
Y←(1 2⋄ 3 4 5) ⋄ {0}¨Y   ⍝ 0 0

⍝ aplcart/table.tsv:732 — Run f on scalars
X←1 2 ⋄ f←, ⋄ Y←3 4 ⋄ X(f⍤0)Y   ⍝ 2 2⍴1 3 2 4

⍝ aplcart/table.tsv:733 — Apply scalar function f between vector Mv and each column of Nm
Mv←1 2 ⋄ f←+ ⋄ Nm←2 3⍴⍳6 ⋄ Mv(f⍤0 1)Nm   ⍝ 2 3⍴2 3 4 6 7 8

⍝ aplcart/table.tsv:734 — Apply f between vector Mv and each row of Nm
Mv←1 2 3 ⋄ f←+ ⋄ Nm←2 3⍴⍳6 ⋄ Mv(f⍤1)Nm   ⍝ 2 3⍴2 4 6 5 7 9

⍝ aplcart/table.tsv:735 — Replacing first major cell of Y with Xs
Xs←99 ⋄ Y←2 3⍴⍳6 ⋄ Xs(@1)Y   ⍝ 2 3⍴99 99 99 4 5 6

⍝ aplcart/table.tsv:736 — Inverse: find Z such that Y ≡ X f Z
X←3 ⋄ f←+ ⋄ Y←1 2 3 ⋄ X(f⍣¯1)Y   ⍝ ¯2 ¯1 0

⍝ aplcart/table.tsv:737 — Inverse: find Z such that Y ≡ f Z
f←- ⋄ Y←1 2 3 ⋄ (f⍣¯1)Y   ⍝ ¯1 ¯2 ¯3

⍝ aplcart/table.tsv:738 — Rotate figure Nv in direction of point Ms
Ms←1j1 ⋄ Nv←1 1j1 0j1 ⋄ Ms×∘×⍨Nv
0.7071067811865475j0.7071067811865475 0j1.414213562373095 ¯0.7071067811865475j0.7071067811865475

⍝ aplcart/table.tsv:739 — Square without changing sign
N←¯3 ¯2 0 2 3 ⋄ ×∘|⍨N   ⍝ ¯9 ¯4 0 4 9

⍝ aplcart/table.tsv:740 — Random Permutation vector for Y; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Y←3 2⍴⍳6 ⋄ r←?⍨∘≢Y ⋄ (⍳≢Y)≡r[⍋r]   ⍝ 1

⍝ aplcart/table.tsv:741 — M'th Root of N
M←2 3 ⋄ N←4 27 ⋄ M*∘÷⍨N   ⍝ 2 3

⍝ aplcart/table.tsv:742 — Conditional change of elements of N to one according to A
A←1 0 1 ⋄ N←2 3 4 ⋄ A*∘~⍨N   ⍝ 1 3 1

⍝ aplcart/table.tsv:743 — Continued fraction with terms N
N←2 3 4 ⋄ +∘÷/N   ⍝ 2.307692307692307

⍝ aplcart/table.tsv:744 — Locate all instances of maximum of N
N←3 1 3 2 3 ⋄ ⌈⌿⍛=N   ⍝ 1 0 1 0 1

⍝ aplcart/table.tsv:745 — Duplicate Y cells where indicated by Av
Av←1 0 1 ⋄ Y←3 2⍴⍳6 ⋄ Av+∘1⍛⌿Y   ⍝ 5 2⍴1 2 1 2 3 4 5 6 5 6

⍝ aplcart/table.tsv:746 — Using Boolean array A for expanding Yv (Yv's elements at 1s in A)
A←1 0 1 0 1 ⋄ Yv←2 3 4 ⋄ A⊣@⊢⍨Yv   ⍝ 2 0 3 0 4

⍝ aplcart/table.tsv:747 — Initialise a matrix with Js columns and no rows
Js←4 ⋄ 0∘,⍛⍴Js   ⍝ 0 4⍴0

⍝ aplcart/table.tsv:748 — Are any true?
B←2 3⍴1 0 0 0 0 0 ⋄ ∨/∘,B   ⍝ 1

⍝ aplcart/table.tsv:749 — Are all true?
B←2 3⍴1 1 1 1 0 1 ⋄ ∧/∘,B   ⍝ 0

⍝ aplcart/table.tsv:750 — Fast: The length of the first axis of each item in X
Y←(1 2⋄ 3 4 5⋄ 2 2⍴⍳4) ⋄ ↑∘⍴¨Y   ⍝ 2 3 2

⍝ aplcart/table.tsv:751 — Pad elements of vector of arrays Yv to equal shape
Yv←(1 2⋄ 3 4 5) ⋄ ⊂⍤¯1∘⊃Yv   ⍝ (1 2 0 ⋄ 3 4 5)

⍝ aplcart/table.tsv:752 — Selective picking from array
X←(1 2⋄ 2 1) ⋄ Y←(1 2⋄ 3 4) ⋄ X⊃¨⊂⊃Y   ⍝ 2 3

⍝ aplcart/table.tsv:753 — Continued fraction convergents with terms N; Reduce each prefix to preserve right association
N←2 3 4 ⋄ (+∘÷/)¨,\N   ⍝ 2 2.333333333333333 2.307692307692307

⍝ aplcart/table.tsv:754 — Turn off all 1s before first 0 (remove leading 1s)
B←1 1 0 1 0 1 0 ⋄ ∧\⍛<B   ⍝ 0 0 0 1 0 1 0

⍝ aplcart/table.tsv:755 — Turn on all 0s before first 1 (add leading 0s)
B←1 1 0 1 0 1 0 ⋄ ∨\⍛≤B   ⍝ 1 1 0 1 0 1 0

⍝ aplcart/table.tsv:756 — Parity with connectors: Joining pairs of odd and even ones (fill gaps with ones)
B←1 1 0 1 0 1 0 ⋄ ≠⍀⍛∨B   ⍝ 1 1 0 1 1 1 0

⍝ aplcart/table.tsv:757 — Places between pairs of ones
B←1 1 0 1 0 1 0 ⋄ ≠⍀⍛>B   ⍝ 0 0 0 0 1 0 0

⍝ aplcart/table.tsv:758 — Merge the leading two axes of Y
(a ← 2 3 4⍴⍳24 ⋄ ⍴ a ⋄ ,[⍳2] a ⋄ ⍴ ,[⍳2] a)
(2 3 4⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 ⋄ 2 3 4 ⋄ 6 4⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 ⋄ 6 4)

⍝ aplcart/table.tsv:759 — Transpose matrix Ym on condition Bs
Bs←1 ⋄ Ym←2 3⍴⍳6 ⋄ Bs⌽∘1 2⍛⍉Ym   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/table.tsv:760 — Count of occurrences of each unique major cell
Y←3 1 3 2 ⋄ ⊢∘≢⌸Y   ⍝ 2 1 1

⍝ aplcart/table.tsv:761 — Sum of squares of Nv
Nv←1 2 3 ⋄ +.×⍨Nv   ⍝ 14

⍝ aplcart/table.tsv:762 — Do rows of Y contain elements differing from Xs?
Xs←9 ⋄ Y←2 3⍴9 9 9 1 9 1 ⋄ Xs∨.≠⍨Y   ⍝ 0 1

⍝ aplcart/table.tsv:763 — Square matrix with Yv as rows
 ⊢⌝ ⍨ 1 2 3 4   ⍝ 4 4⍴1 2 3 4 1 2 3 4 1 2 3 4 1 2 3 4

⍝ aplcart/table.tsv:764 — Square matrix with Yv as columns
 ⊣⌝ ⍨ 1 2 3 4   ⍝ 4 4⍴1 1 1 1 2 2 2 2 3 3 3 3 4 4 4 4

⍝ aplcart/table.tsv:765 — Two-row matrix from two vectors (repeat scalars)
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv,[0.5]Yv   ⍝ 2 3⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:766 — Are characters of D lowercase?
D←'Abc 19 Σς!'  ⋄ 1∘•C⍛≠D   ⍝ 0 1 1 0 0 0 0 0 1 0

⍝ aplcart/table.tsv:767 — Are characters of D uppercase?
D←'Abc 19 Σς!'  ⋄ ¯1∘•C⍛≠D   ⍝ 1 0 0 0 0 0 0 1 0 0

⍝ aplcart/table.tsv:768 — Apply Z as constant function if array, but like normal dyadic function if function
X←1 ⋄ Z←42 ⋄ Y←2 ⋄ X(Z⊣⊢)Y   ⍝ 42

⍝ aplcart/table.tsv:769 — Apply f as normal dyadic function if function, but like constant function if array
X←1 ⋄ f←+ ⋄ Y←2 ⋄ X(f⊣⊢)Y   ⍝ 3

⍝ aplcart/table.tsv:770 — Apply Z as constant function if array, but like normal monadic function if function
Z←42 ⋄ Y←3 1 3 2 ⋄ (Z⊣⊢)Y   ⍝ 42

⍝ aplcart/table.tsv:771 — Apply f as normal monadic function if function, but like constant function if array
f←+ ⋄ Y←3 1 3 2 ⋄ (f⊣⊢)Y   ⍝ 3 1 3 2

⍝ aplcart/table.tsv:772 — Inclusive integer difference
I←2 3 4 ⋄ J←4 9 16 ⋄ I(1+-)J   ⍝ ¯1 ¯5 ¯11

⍝ aplcart/table.tsv:773 — Sign of difference (¯1:M is smaller, 0:M=N, 1:M is bigger)
M←2 3 4 ⋄ N←4 9 16 ⋄ M(×-)N   ⍝ ¯1 ¯1 ¯1

⍝ aplcart/table.tsv:774 — Probabilistic NAND
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(1-×)N   ⍝ 1 0.875 1

⍝ aplcart/table.tsv:775 — Force numbers N to range 0≤N≤M
M←2 3 4 ⋄ N←4 9 16 ⋄ M(0⌈⌊)N   ⍝ 2 3 4

⍝ aplcart/table.tsv:776 — Floored division
¯10 10 ¯10 10 (⌊÷) ¯3 ¯3 3 3   ⍝ 3 ¯4 ¯4 3

⍝ aplcart/table.tsv:777 — Incrementing cyclic counter J with upper limit I
I←2 3 4 ⋄ J←4 9 16 ⋄ I(1+|)J   ⍝ 1 1 1

⍝ aplcart/table.tsv:778 — Absolute distance between X and Y
M←2 3 4 ⋄ N←4 9 16 ⋄ M(|-)N   ⍝ 2 6 12

⍝ aplcart/table.tsv:779 — Magnitude of fractional part of N
N←4 9 16 ⋄ (1||)N   ⍝ 0 0 0

⍝ aplcart/table.tsv:780 — Are I and J co-prime?
I←2 3 4 ⋄ J←4 9 16 ⋄ I(1=∨)J   ⍝ 0 0 0

⍝ aplcart/table.tsv:781 — Does I divide J?
I←2 3 4 ⋄ J←4 9 16 ⋄ I(0=|)J   ⍝ 1 1 1

⍝ aplcart/table.tsv:782 — Does I not divide J?
I←2 3 4 ⋄ J←4 9 16 ⋄ I(0≠|)J   ⍝ 0 0 0

⍝ aplcart/table.tsv:783 — Does Y have Uniform Depth?
Y←(1 2⋄ 3(4 5)) ⋄ (0≤≡)Y   ⍝ 0

⍝ aplcart/table.tsv:784 — Is Y a Simple Scalar?
Y←42 ⋄ (0=≡)Y   ⍝ 1

⍝ aplcart/table.tsv:785 — Does Y have Non-Uniform Depth?
Y←(1 2⋄ 3(4 5)) ⋄ (0>≡)Y   ⍝ 1

⍝ aplcart/table.tsv:786 — Choosing Is random numbers in the range 1 to Js with replacement; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Is←8 ⋄ Js←3 ⋄ r←Is(?⍴)Js ⋄ (Is=≢r)∧∧/(1≤r)∧r≤Js
1

⍝ aplcart/table.tsv:787 — Is Y a Scalar?
Y←3 1 3 2 ⋄ (⍬≡⍴)Y   ⍝ 0

⍝ aplcart/table.tsv:788 — Rank (number of dimensions) of Y
Y←3 1 3 2 ⋄ (≢⍴)Y   ⍝ 1

⍝ aplcart/table.tsv:789 — Dimensions of major cells
Y←3 1 3 2 ⋄ (1↓⍴)Y   ⍝ ⍬

⍝ aplcart/table.tsv:790 — Lengths of leading axes
Y←3 1 3 2 ⋄ (¯1↓⍴)Y   ⍝ ⍬

⍝ aplcart/table.tsv:791 — Swap real and imaginary
N←1j2 ¯3j4 ⋄ (¯11○+)N   ⍝ 2j1 4j¯3

⍝ aplcart/table.tsv:792 — Area of cone with height M and radius N (excluding base)
M←2 3 4 ⋄ N←4 9 16 ⋄ M(π×)N
25.13274122871834 84.82300164692441 201.0619298297468

⍝ aplcart/table.tsv:793 — Circumference of circle with radius N
N←4 9 16 ⋄ (2×π)N
25.13274122871834 56.54866776461628 100.5309649148734

⍝ aplcart/table.tsv:794 — Angle of right triangle with height M and width N
M←1 ⋄ N←1 2 ⋄ M(¯3○÷)N   ⍝ 0.7853981633974483 0.4636476090008061

⍝ aplcart/table.tsv:795 — Arccosecant
N←4 9 16 ⋄ (¯1○÷)N
0.2526802551420786 0.1113410143409639 0.06254076179649139

⍝ aplcart/table.tsv:796 — Arcsecant
N←4 9 16 ⋄ (¯2○÷)N
1.318116071652818 1.459455312453933 1.508255564998405

⍝ aplcart/table.tsv:797 — Arccotangent
N←4 9 16 ⋄ (¯3○÷)N
0.2449786631268641 0.1106572211738956 0.06241880999595735

⍝ aplcart/table.tsv:798 — Hyperbolic area cosecant
N←4 9 16 ⋄ (¯5○÷)N
0.2474664615472635 0.1108837483012855 0.06245938125554031

⍝ aplcart/table.tsv:799 — Hyperbolic area secant
N←0.25 0.5 1 ⋄ (¯6○÷)N   ⍝ 2.063437068895561 1.316957896924817 0

⍝ aplcart/table.tsv:800 — Hyperbolic area cotangent
N←4 9 16 ⋄ (¯7○÷)N
0.2554128118829953 0.1115717756571049 0.06258157147700301

⍝ aplcart/table.tsv:801 — Map 0/1 to ¯1/1
(¯1*~) 1 0 1 1 0   ⍝ 1 ¯1 1 1 ¯1

⍝ aplcart/table.tsv:802 — Decrement
N←4 9 16 ⋄ (-∘1)N   ⍝ 3 8 15

⍝ aplcart/table.tsv:803 — Probabilistic inverse (NOT)
N←4 9 16 ⋄ (1∘-)N   ⍝ ¯3 ¯8 ¯15

⍝ aplcart/table.tsv:804 — Rate corresponding to percentage N
N←4 9 16 ⋄ (÷∘100)N   ⍝ 0.04 0.09 0.16

⍝ aplcart/table.tsv:805 — Halve: N÷2
N←4 9 16 ⋄ (÷∘2)N   ⍝ 2 4.5 8

⍝ aplcart/table.tsv:806 — First row as a row matrix (row vector)
Ym←2 3⍴⍳6 ⋄ (1∘↑)Ym   ⍝ 1 3⍴1 2 3

⍝ aplcart/table.tsv:807 — Last row as a row matrix (row vector)
Ym←2 3⍴⍳6 ⋄ (¯1∘↑)Ym   ⍝ 1 3⍴4 5 6

⍝ aplcart/table.tsv:808 — Cube
N←4 9 16 ⋄ (*∘3)N   ⍝ 64 729 4096

⍝ aplcart/table.tsv:809 — Drop Is columns from matrix Ym
Is←2 ⋄ Ym←2 3⍴⍳6 ⋄ Is(↓⍤1)Ym   ⍝ 2 1⍴3 6

⍝ aplcart/table.tsv:810 — Append a column of 1s
Y←3 1 3 2 ⋄ (,∘1)Y   ⍝ 3 1 3 2 1

⍝ aplcart/table.tsv:811 — Two-column matrix from two vectors (repeat scalars)
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,⍤0)Yv   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/table.tsv:812 — Prefix vector to each row of matrix
Xv←1 2 3 ⋄ Ym←2 3⍴⍳6 ⋄ Xv(,⍤1)Ym   ⍝ 2 6⍴1 2 3 1 2 3 1 2 3 4 5 6

⍝ aplcart/table.tsv:813 — Postfix vector to each row of matrix
Xm←2 3⍴⍳6 ⋄ Yv←4 5 6 ⋄ Xm(,⍤1)Yv   ⍝ 2 6⍴1 2 3 4 5 6 4 5 6 4 5 6

⍝ aplcart/table.tsv:814 — Increment rank by inserting a new dimension after the trailing one
(⍴ (,⍤0) 'abc' ⋄ ⍴ (,⍤0) 3 4⍴•A)   ⍝ (3 1 ⋄ 3 4 1)

⍝ aplcart/table.tsv:815 — Append a row of 1s
Y←3 1 3 2 ⋄ (⍪∘1)Y   ⍝ 3 1 3 2 1

⍝ aplcart/table.tsv:816 — Pick item of vector Yv at cyclic offset Is (like ⎕IO←0, default Is:¯1)
7 (↑⌽) 'abcdef'   ⍝ 'b'

⍝ aplcart/table.tsv:817 — Pick last item of vector Yv
Yv←(1 2⋄ 3 4 5) ⋄ (↑⌽)Yv   ⍝ 3 4 5

⍝ aplcart/table.tsv:818 — ±N by juxtaposition
N←4 9 16 ⋄ (1 ¯1×⊂)N   ⍝ (4 9 16 ⋄ ¯4 ¯9 ¯16)

⍝ aplcart/table.tsv:819 — ∓N by juxtaposition
N←4 9 16 ⋄ (¯1 1×⊂)N   ⍝ (¯4 ¯9 ¯16 ⋄ 4 9 16)

⍝ aplcart/table.tsv:820 — Split complex array into two-element vector of magnitude array and angle array
N←1j2 ¯3j4 ⋄ (10 12○⊂)N
(2.23606797749979 5 ⋄ 1.10714871779409 2.214297435588181)

⍝ aplcart/table.tsv:821 — Split complex array into two-element vector of real array and imaginary array
N←1j2 ¯3j4 ⋄ (9 11○⊂)N   ⍝ (1 ¯3 ⋄ 2 4)

⍝ aplcart/table.tsv:822 — Enclose each major cell for any rank Y
Y←3 1 3 2 ⋄ (⊂⍤¯1)Y   ⍝ 3 1 3 2

⍝ aplcart/table.tsv:823 — Select major cell of Y at cyclic offset Is (like ⎕IO←0, default Is:¯1)
Is←2 ⋄ Y←3 1 3 2 ⋄ Is(1⌷⊖)Y   ⍝ 3

⍝ aplcart/table.tsv:824 — Tail: Last major cell
Y←3 1 3 2 ⋄ (1⌷⊖)Y   ⍝ 2

⍝ aplcart/table.tsv:825 — Select: each major cell of Im selects a cell from Y
Im←2 2⍴1 2 2 3 ⋄ Y←3 4⍴⍳12 ⋄ Im(⌷⍤¯1 99)Y
2 7

⍝ aplcart/table.tsv:826 — Integers from 0 to Js-1
Js←4 ⋄ (¯1+⍳)Js   ⍝ 0 1 2 3

⍝ aplcart/table.tsv:827 — Indices of Major Cells of Y
Y←3 1 3 2 ⋄ (⍳≢)Y   ⍝ 1 2 3 4

⍝ aplcart/table.tsv:828 — All indices of array Y
Y←3 1 3 2 ⋄ (⍳⍴)Y   ⍝ 1 2 3 4

⍝ aplcart/table.tsv:829 — Index of first 1 in N
N←0 0 1 0 ⋄ (⍳∘1)N   ⍝ 3

⍝ aplcart/table.tsv:830 — Indices of elements of X in corresponding rows of X (X[i;]⍳Y[i;])
X←2 3⍴1 2 3 4 5 6 ⋄ Y←2 2⍴2 7 5 4 ⋄ X(⍳⍤1)Y
2 2⍴2 4 2 1

⍝ aplcart/table.tsv:831 — Catalogue of all pairs from ⍳Is and ⍳Js
Is←2 ⋄ Js←4 ⋄ Is(⍳,)Js
2 4⍴(1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 1 4 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3 ⋄ 2 4)

⍝ aplcart/table.tsv:832 — Are any major cells identical?
Y←3 1 3 2 ⋄ (0∊≠)Y   ⍝ 1

⍝ aplcart/table.tsv:833 — Is Y an Empty Array?
Y←2 0 3⍴0 ⋄ (0∊⍴)Y   ⍝ 1

⍝ aplcart/table.tsv:834 — Boolean items in X that are not in Y
X←3 1 2 ⋄ Y←3 1 3 2 ⋄ X(~∊)Y   ⍝ 0 0 0

⍝ aplcart/table.tsv:835 — Zeros, simple with same shape
Y←3 1 3 2 ⋄ (∊∘⍬)Y   ⍝ 0 0 0 0

⍝ aplcart/table.tsv:836 — Which elements of X belong to corresponding row of Y (≢X ↔ ≢Y)
X←2 3⍴1 2 3 4 5 6 ⋄ Y←2 2⍴2 7 5 4 ⋄ X(∊⍤1)Y
2 3⍴0 1 0 1 1 0

⍝ aplcart/table.tsv:837 — Is Y within the range [ 1⌷X , 2⌷X ) ?
(1 3 (1=⍸) 0 ⋄ 1 3 (1=⍸) 1 ⋄ 1 3 (1=⍸) 2 ⋄ 1 3 (1=⍸) 3 ⋄ 1 3 (1=⍸) 4)
0 1 1 0 0

⍝ aplcart/table.tsv:838 — Is Y outside the range [ 1⌷X , 2⌷X ) ?
(1 3 (1≠⍸) 0 ⋄ 1 3 (1≠⍸) 1 ⋄ 1 3 (1≠⍸) 2 ⋄ 1 3 (1≠⍸) 3 ⋄ 1 3 (1≠⍸) 4)
1 0 0 1 1

⍝ aplcart/table.tsv:839 — Index of first satisfied condition in B
B←1 1 0 1 0 1 0 ⋄ (↑⍸)B   ⍝ 1

⍝ aplcart/table.tsv:840 — Indices of all occurrences of elements of X in Y
X←1 2 3 4 2 ⋄ Y←2 4 ⋄ X(⍸∊)Y   ⍝ 2 4 5

⍝ aplcart/table.tsv:841 — Index of Smallest
Y←3 1 3 2 ⋄ (↑⍋)Y   ⍝ 2

⍝ aplcart/table.tsv:842 — Grade up of Y according to key X
X←4 3 2 1 ⋄ Y←1 3 2 ⋄ X(⍋⍳)Y   ⍝ 2 3 1

⍝ aplcart/table.tsv:843 — Ascending cardinal numbers (ranking, all different)
Y←3 1 3 2 ⋄ (⍋⍋)Y   ⍝ 3 1 4 2

⍝ aplcart/table.tsv:844 — Index of Largest
Y←3 1 3 2 ⋄ (↑⍒)Y   ⍝ 1

⍝ aplcart/table.tsv:845 — Grade down of Y according to key X
X←4 3 2 1 ⋄ Y←1 3 2 ⋄ X(⍒⍳)Y   ⍝ 1 3 2

⍝ aplcart/table.tsv:846 — Descending cardinal numbers (ranking, all different)
Y←3 1 3 2 ⋄ (⍋⍒)Y   ⍝ 1 4 2 3

⍝ aplcart/table.tsv:847 — Matrix to vector of column vectors
(↓⍉) 3 4⍴⍳12   ⍝ (1 5 9 ⋄ 2 6 10 ⋄ 3 7 11 ⋄ 4 8 12)

⍝ aplcart/table.tsv:848 — Convert inverted table to table (character data as vectors of vectors); dfns display import/wrappers omitted to test underlying arrays
(⍉⊃) ('Ab' 'Cdef' ⋄ 1 2 ⋄ 7 3)   ⍝ 2 3⍴('Ab') 1 7 ('Cdef') 2 3

⍝ aplcart/table.tsv:849 — Transpose every submatrix of Y
Y←2 3 4⍴⍳24 ⋄ (⍉⍤2)Y
2 4 3⍴1 5 9 2 6 10 3 7 11 4 8 12 13 17 21 14 18 22 15 19 23 16 20 24

⍝ aplcart/table.tsv:850 — If Y begins with X
X←1 2 ⋄ Y←1 2 3 1 2 ⋄ X(↑⍷)Y   ⍝ 1

⍝ aplcart/table.tsv:851 — Positions of starts of subarrays X in Y
X←1 2 ⋄ Y←1 2 3 1 2 ⋄ X(⍸⍷)Y   ⍝ 1 4

⍝ aplcart/table.tsv:852 — Parallel projection of 3D object in Nm
Nm←2 3⍴⍳6 ⋄ (0J1⊥⊖)Nm   ⍝ 1j4 2j5 3j6

⍝ aplcart/table.tsv:853 — Number of columns in Y
Y←3 1 3 2 ⋄ (0⊥⍴)Y   ⍝ 4

⍝ aplcart/table.tsv:854 — N in Base M
M←2 ⋄ N←5 8 15 ⋄ M(⊥⍣¯1)N   ⍝ 4 3⍴0 1 1 1 0 1 0 0 1 1 0 1

⍝ aplcart/table.tsv:855 — Number of digit positions in Js (depends on ⎕PP)
Js←4 ⋄ (≢⍕)Js   ⍝ 1

⍝ aplcart/table.tsv:856 — Rounding N to Is decimal places; Reviewed Execute example checked through the Rust reference worker
Is←2 ⋄ N←¯1.234 1.236 20.125 ⋄ Is(⍎⍕)N   ⍝ ¯1.23 1.24 20.13

⍝ aplcart/table.tsv:857 — Reshaping only one-element numeric vector Nv into a scalar (leave longer vectors as-is); Reviewed Execute example checked through the Rust reference worker
Nv←,7 ⋄ (⍎⍕)Nv   ⍝ 7

⍝ aplcart/table.tsv:858 — Rounding to ⎕PP precision; pure Execute recipe checked against Dyalog with fixed settings
(⍎⍕)0.1 0.2 1.23456789012345   ⍝ 0.1 0.2 1.23456789012345

⍝ aplcart/table.tsv:863 — Join real and imaginary major cells by removing leading axis
N←2 3⍴1 2 3 4 5 6 ⋄ ¯11∘○⍛+⌿N   ⍝ 4j1 5j2 6j3

⍝ aplcart/table.tsv:864 — Join magnitude and radians major cells by removing leading axis
N←2 3⍴2 3 4 0 0.5 1 ⋄ ¯12∘○⍛×⌿N
0 ¯0.4949962483002227j0.0705600040299336 ¯0.6536436208636119j¯0.7568024953079283

⍝ aplcart/table.tsv:865 — Pick random item from vector; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Yv←10 20 30 ⋄ r←?∘≢⍛⊃Yv ⋄ (0=≢⍴r)∧r∊Yv   ⍝ 1

⍝ aplcart/table.tsv:866 — Normalise scalar/vector/vector of scalars/vectors to vector of vectors
Y←(1 2)3(4 5) ⋄ ,∘⊆∘,Y   ⍝ (1 2) 3 (4 5)

⍝ aplcart/table.tsv:867 — Index random item from array; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Y←2 3⍴10×⍳6 ⋄ r←?∘⍴⍛⌷Y ⋄ (0=≢⍴r)∧r∊Y   ⍝ 1

⍝ aplcart/table.tsv:868 — All axes of array Y
Y←3 1 3 2 ⋄ ⍳∘≢∘⍴Y   ⍝ 1⍴1

⍝ aplcart/table.tsv:869 — Attach row numbers to a matrix
Ym←2 3⍴⍳6 ⋄ ⍳∘≢⍛,Ym   ⍝ 2 4⍴1 1 2 3 2 4 5 6

⍝ aplcart/table.tsv:870 — Starting points for Y in indices pointed by Iv
Iv←2 1 ⋄ Y←3 1 3 2 ⋄ Iv∊∘⍳∘≢Y   ⍝ 1 1

⍝ aplcart/table.tsv:871 — Sort Ascending
Y←3 1 3 2 ⋄ ⊂∘⍋⍛⌷Y   ⍝ 1 2 3 3

⍝ aplcart/table.tsv:872 — Fast: Y sorted into ascending order
Y←3 1 3 2 ⋄ ⊂⍤⍋⍛⌷Y   ⍝ 1 2 3 3

⍝ aplcart/table.tsv:873 — Fast: Y sorted into ascending order
Y←3 1 3 2 ⋄ ⊂⍤⍋⍛⌷Y   ⍝ 1 2 3 3

⍝ aplcart/table.tsv:874 — Sorting Y according to X
X← 'cba'  ⋄ Y← 'abca'  ⋄ X⊂⍤⍋⍛⌷Y   ⍝ 'cba'

⍝ aplcart/table.tsv:875 — Is Nv a permutation vector?
Nv←3 1 2 ⋄ ⍋⍤⍋⍛≡Nv   ⍝ 1

⍝ aplcart/table.tsv:876 — Sort Descending
Y←3 1 3 2 ⋄ ⊂∘⍒⍛⌷Y   ⍝ 3 3 2 1

⍝ aplcart/table.tsv:877 — Fast: Y sorted into descending order
Y←3 1 3 2 ⋄ ⊂⍤⍒⍛⌷Y   ⍝ 3 3 2 1

⍝ aplcart/table.tsv:878 — Moving cells of Y indicated by Av to the start of Y
Av←1 0 1 0 ⋄ Y←3 1 3 2 ⋄ Av⊂⍤⍒⍛⌷Y   ⍝ 3 3 1 2

⍝ aplcart/table.tsv:879 — First Js triangular pyramidal numbers
Js←4 ⋄ +\⍣2∘⍳Js   ⍝ 1 4 10 20

⍝ aplcart/table.tsv:880 — Reflect counter-diagonally
Ym←2 3⍴⍳6 ⋄ ⌽∘⍉∘⌽Ym   ⍝ 3 2⍴6 3 5 2 4 1

⍝ aplcart/table.tsv:881 — Matrix with Is rows, each consisting of Yv
Is←2 ⋄ Yv←4 5 6 ⋄ Is⌿∘⍉∘⍪Yv   ⍝ 2 3⍴4 5 6 4 5 6

⍝ aplcart/table.tsv:882 — Number of occurrences of scalar Xs in array Y
Xs←9 ⋄ Y←3 1 3 2 ⋄ Xs+.=∘,Y   ⍝ 0

⍝ aplcart/table.tsv:883 — Probabilistic converse nonimplication
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(⊢-×)N   ⍝ 1 0.125 0

⍝ aplcart/table.tsv:884 — Probabilistic nonimplication
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(⊣-×)N   ⍝ 0 0.375 1

⍝ aplcart/table.tsv:885 — Probabilistic OR
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(+-×)N   ⍝ 1 0.625 1

⍝ aplcart/table.tsv:886 — Largest whole multiple of M less than or equal to N (N rounded down to closest smaller multiple of M)
10 (⊢-|) 9 10 11 19 20 21   ⍝ 0 10 10 10 20 20

⍝ aplcart/table.tsv:887 — Fractional part with sign
N←¯2.5 0 3.75 ⋄ (×|⊢)N   ⍝ ¯0.5 0 0.75

⍝ aplcart/table.tsv:888 — Is N real?
N←1 2j3 0j1 ⋄ (⊢=+)N   ⍝ 1 0 0

⍝ aplcart/table.tsv:889 — Is N integer?
N←1 1.5 2j3 ⋄ (⌊=⊢)N   ⍝ 1 0 1

⍝ aplcart/table.tsv:890 — Is N complex?
N←1 2j3 0j1 ⋄ (⊢≠+)N   ⍝ 0 1 1

⍝ aplcart/table.tsv:891 — Is Dv a palindrome?
((⌽≡⊢) 'racecar' ⋄ (⌽≡⊢) 'carrace')   ⍝ 1 0

⍝ aplcart/table.tsv:892 — Length to represent J in base I
I←2 3 4 ⋄ J←4 9 16 ⋄ I(⌊1+⍟)J   ⍝ 3 3 3

⍝ aplcart/table.tsv:893 — Complementary Angle
N←0 0.25 0.5 ⋄ (¯2○1○⊢)N
1.570796326794897 1.320796326794897 1.070796326794897

⍝ aplcart/table.tsv:894 — Vector having as many ones as Ym has rows
Ym←2 3⍴⍳6 ⋄ (1⍴⍨≢)Ym   ⍝ 1 1

⍝ aplcart/table.tsv:895 — Limit of nominal rate N when continuously compounded
N←0.01 0.05 0.1 ⋄ (*×⍨)N
1.000100005000167 1.002503127605795 1.010050167084168

⍝ aplcart/table.tsv:896 — Gamma function of N
N←1 2 3 4 ⋄ (!-∘1)N   ⍝ 1 1 2 6

⍝ aplcart/table.tsv:897 — Cosecant
N←4 9 16 ⋄ (÷1∘○)N
¯1.321348708810902 2.426486643551989 ¯3.473388259584929

⍝ aplcart/table.tsv:898 — Secant
N←4 9 16 ⋄ (÷2∘○)N
¯1.529885656466397 ¯1.097537906304962 ¯1.044212499898521

⍝ aplcart/table.tsv:899 — Cotangent
N←4 9 16 ⋄ (÷3∘○)N
0.8636911544506167 ¯2.210845410999195 3.326323195635449

⍝ aplcart/table.tsv:900 — Hyperbolic cosecant; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (÷5∘○)N
0.03664357032586561 0.0002468196119324168 2.250703494385211e¯07

⍝ aplcart/table.tsv:901 — Hyperbolic secant; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (÷6∘○)N
0.03661899347368653 0.0002468196044143015 2.250703494385154e¯07

⍝ aplcart/table.tsv:902 — Hyperbolic cotangent; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (÷7∘○)N
1.000671150401683 1.00000003045996 1.000000000000025

⍝ aplcart/table.tsv:903 — Area of circle with radius N; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (π*∘2)N
50.26548245743669 254.4690049407732 804.247719318987

⍝ aplcart/table.tsv:904 — J is Even; optional {X} instantiated as dyadic use
J←4 9 16 ⋄ (~2∘|)J   ⍝ 1 0 1

⍝ aplcart/table.tsv:905 — J Hook: Y f g Y when monadic and X f g Y when dyadic; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ f←+ ⋄ g←⌽ ⋄ Y←3 1 3 2 ⋄ X f∘g⍨⍨Y
5 4 3 4

⍝ aplcart/table.tsv:906 — Remove blanks in each string; optional {X} instantiated as dyadic use
D← ' a b' 'c  d ' ''  ⋄ ~∘' '¨D   ⍝ ('ab' ⋄ 'cd' ⋄ '')

⍝ aplcart/table.tsv:907 — Deltas: (N[2]-N[1])(N[3]-N[2])(N[4]-N[3])…; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ -/⌽2↕N   ⍝ 5 7

⍝ aplcart/table.tsv:908 — Ratio of each number in a list to its predecessor: (N[2]÷N[1])(N[3]÷N[2])(N[4]÷N[3])…; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ ÷/⌽2↕N   ⍝ 2.25 1.777777777777778

⍝ aplcart/table.tsv:909 — Largest row-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌈/|)N   ⍝ 3 6

⍝ aplcart/table.tsv:910 — Smallest row-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌊/|)N   ⍝ 1 4

⍝ aplcart/table.tsv:911 — Ones, same shape plus one; optional {X} instantiated as dyadic use
Yv←4 5 6 ⋄ =/0↕Yv   ⍝ 1 1 1 1

⍝ aplcart/table.tsv:912 — Zeros, same shape plus one; optional {X} instantiated as dyadic use
Yv←4 5 6 ⋄ ≠/0↕Yv   ⍝ 0 0 0 0

⍝ aplcart/table.tsv:913 — Hamming distance; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(+/≠)Yv   ⍝ 3

⍝ aplcart/table.tsv:914 — Count the number of elements in an array; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (×/⍴)Y   ⍝ 4

⍝ aplcart/table.tsv:915 — Is Ym a square matrix?
((=/⍴) ⊃'abc' 'def' ⋄ (=/⍴) ⊃'ab' 'cd')   ⍝ 0 1

⍝ aplcart/table.tsv:916 — Tetration: ᴺˢIs
(3 (*/⍴) 2 ⋄ 2 (*/⍴) 4)   ⍝ 16 256

⍝ aplcart/table.tsv:917 — Sum of magnitude of N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (+⌿|)N   ⍝ 5 7 9

⍝ aplcart/table.tsv:918 — Largest column-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌈⌿|)N   ⍝ 4 5 6

⍝ aplcart/table.tsv:919 — Infinity-norm
(⌈⌿|) 3 4   ⍝ 4

⍝ aplcart/table.tsv:920 — Smallest column-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌊⌿|)N   ⍝ 1 2 3

⍝ aplcart/table.tsv:921 — Is N outside the range ( 1⌷M , 2⌷M ] or ( 2⌷M , 1⌷M ] ?
(1 3 (=⌿<) 0 ⋄ 1 3 (=⌿<) 1 ⋄ 1 3 (=⌿<) 2 ⋄ 1 3 (=⌿<) 3 ⋄ 1 3 (=⌿<) 4 ⋄ 3 1 (=⌿<) 0 ⋄ 3 1 (=⌿<) 1 ⋄ 3 1 (=⌿<) 2 ⋄ 3 1 (=⌿<) 3 ⋄ 3 1 (=⌿<) 4)
1 1 0 0 1 1 1 0 0 1

⍝ aplcart/table.tsv:922 — Is N outside the range [ 1⌷M , 2⌷M ) or [ 2⌷M , 1⌷M ) ?
(1 3 (=⌿≤) 0 ⋄ 1 3 (=⌿≤) 1 ⋄ 1 3 (=⌿≤) 2 ⋄ 1 3 (=⌿≤) 3 ⋄ 1 3 (=⌿≤) 4 ⋄ 3 1 (=⌿≤) 0 ⋄ 3 1 (=⌿≤) 1 ⋄ 3 1 (=⌿≤) 2 ⋄ 3 1 (=⌿≤) 3 ⋄ 3 1 (=⌿≤) 4)
1 0 0 1 1 1 0 0 1 1

⍝ aplcart/table.tsv:923 — Is N within the range ( 1⌷M , 2⌷M ] or ( 2⌷M , 1⌷M ] ?
(1 3 (≠⌿<) 0 ⋄ 1 3 (≠⌿<) 1 ⋄ 1 3 (≠⌿<) 2 ⋄ 1 3 (≠⌿<) 3 ⋄ 1 3 (≠⌿<) 4 ⋄ 3 1 (≠⌿<) 0 ⋄ 3 1 (≠⌿<) 1 ⋄ 3 1 (≠⌿<) 2 ⋄ 3 1 (≠⌿<) 3 ⋄ 3 1 (≠⌿<) 4)
0 0 1 1 0 0 0 1 1 0

⍝ aplcart/table.tsv:924 — Is N within the range [ 1⌷M , 2⌷M ) or [ 2⌷M , 1⌷M ) ?
(1 3 (≠⌿≤) 0 ⋄ 1 3 (≠⌿≤) 1 ⋄ 1 3 (≠⌿≤) 2 ⋄ 1 3 (≠⌿≤) 3 ⋄ 1 3 (≠⌿≤) 4 ⋄ 3 1 (≠⌿≤) 0 ⋄ 3 1 (≠⌿≤) 1 ⋄ 3 1 (≠⌿≤) 2 ⋄ 3 1 (≠⌿≤) 3 ⋄ 3 1 (≠⌿≤) 4)
0 1 1 0 0 0 1 1 0 0

⍝ aplcart/table.tsv:925 — First column as a column matrix (column vector); optional {X} instantiated as dyadic use
Ym←2 3⍴⍳6 ⋄ (1∘↑⍤1)Ym   ⍝ 2 1⍴1 4

⍝ aplcart/table.tsv:926 — Last column as a column matrix (column vector); optional {X} instantiated as dyadic use
Ym←2 3⍴⍳6 ⋄ (¯1∘↑⍤1)Ym   ⍝ 2 1⍴3 6

⍝ aplcart/table.tsv:927 — Replacing all values Ys in Y with Xs; optional {X} instantiated as dyadic use
Xs←9 ⋄ Ys←1 ⋄ Y←3 1 3 2 ⋄ Xs@(Ys∘=)Y   ⍝ 3 9 3 2

⍝ aplcart/table.tsv:928 — Handling array Y temporarily as a vector (optionally with left argument X); optional {X} instantiated as dyadic use
X←10 ⋄ f←⌽ ⋄ Y←2 3⍴⍳6 ⋄ X f@(1⍨¨)Y   ⍝ 2 3⍴5 6 1 2 3 4

⍝ aplcart/table.tsv:929 — Fill array Y with Xs; optional {X} instantiated as dyadic use
Xs←9 ⋄ Y←2 3⍴⍳6 ⋄ Xs@(1⍨¨)Y   ⍝ 2 3⍴9 9 9 9 9 9

⍝ aplcart/table.tsv:930 — Ms±Ns; optional {X} instantiated as dyadic use
Ms←2 ⋄ Ns←3 ⋄ Ms(+,-)Ns   ⍝ 5 ¯1

⍝ aplcart/table.tsv:931 — Ms∓Ns; optional {X} instantiated as dyadic use
Ms←2 ⋄ Ns←3 ⋄ Ms(-,+)Ns   ⍝ ¯1 5

⍝ aplcart/table.tsv:932 — Sum all elements in an array; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (+/,)N   ⍝ 29

⍝ aplcart/table.tsv:933 — Number of trues; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (+/,)B   ⍝ 4

⍝ aplcart/table.tsv:934 — Inserting Xs before each element of Yv; optional {X} instantiated as dyadic use
Xs←9 ⋄ Yv←4 5 6 ⋄ Xs(,,⍤0)Yv   ⍝ 9 4 9 5 9 6

⍝ aplcart/table.tsv:935 — Merging equal-length vectors Xv and Yv alternately; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,,⍤0)Yv   ⍝ 1 4 2 5 3 6

⍝ aplcart/table.tsv:936 — Inserting Xv before every element of Yv; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,,⍤1 0)Yv   ⍝ 1 2 3 4 1 2 3 5 1 2 3 6

⍝ aplcart/table.tsv:937 — Fill element (converts characters to spaces, numbers to zeros); optional {X} instantiated as dyadic use
Y←('ab'⋄ 1 2)  ⋄ (↑0∘⍴)Y   ⍝ '  '

⍝ aplcart/table.tsv:938 — Join array of arrays horizontally; optional {X} instantiated as dyadic use
Yv←(2 2⍴⍳4⋄ 2 3⍴⍳6) ⋄ (,/)Yv   ⍝ 2 5⍴1 2 1 2 3 3 4 4 5 6

⍝ aplcart/table.tsv:939 — Join array of arrays vertically; optional {X} instantiated as dyadic use
Y←(2 3⍴⍳6⋄ 1 3⍴7 8 9) ⋄ (⍪⌿)Y   ⍝ 3 3⍴1 2 3 4 5 6 7 8 9

⍝ aplcart/table.tsv:940 — Vector of major cells for any rank Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (,⊂⍤¯1)Y   ⍝ 3 1 3 2

⍝ aplcart/table.tsv:941 — Type: 'a' 1 ⎕NULL → ' ' 0 ⎕NULL (∊ with ⎕ML←0); optional {X} instantiated as dyadic use
Y← 'a' 1  ⋄ (↑0⍴⊂)Y   ⍝ ' ' 0

⍝ aplcart/table.tsv:942 — Split Yv at occurrences of Xs (removes separators and empty segments)
'/' (≠⊆⊢) 'hello/there/world'   ⍝ ('hello' ⋄ 'there' ⋄ 'world')

⍝ aplcart/table.tsv:943 — Is Y a Simple Array?; optional {X} instantiated as dyadic use
Y←2 3⍴⍳6 ⋄ (⊂≡⊆)Y   ⍝ 1

⍝ aplcart/table.tsv:944 — Is Y a Nested Array?; optional {X} instantiated as dyadic use
Y←(1 2⋄ 3 4) ⋄ (⊂≢⊆)Y   ⍝ 1

⍝ aplcart/table.tsv:945 — Odd integers from 1 to 2×Js
((¯1+2×⍳) 5 ⋄ (¯1+2×⍳) 6)   ⍝ (1 3 5 7 9 ⋄ 1 3 5 7 9 11)

⍝ aplcart/table.tsv:946 — Indices of trailing axis of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⍳¯1↑⍴)Y   ⍝ 1 2 3 4

⍝ aplcart/table.tsv:947 — Index of last occurrence in X of any major cell of Y; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←1 2 ⋄ X(⌈/⍳)Y   ⍝ 3

⍝ aplcart/table.tsv:948 — Index of first occurrence in X of any major cell of Y; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←1 2 ⋄ X(⌊/⍳)Y   ⍝ 2

⍝ aplcart/table.tsv:949 — Convert permutation matrices in B to permutation vectors
(⍳∘1⍤1) ⊃(1 0 0 0 0⋄ 0 0 0 1 0⋄ 0 1 0 0 0⋄ 0 0 0 0 1⋄ 0 0 1 0 0)
1 4 2 5 3

⍝ aplcart/table.tsv:950 — Even integers from 0 to 2×Js
((0,2×⍳) 5 ⋄ (0,2×⍳) 6)   ⍝ (0 2 4 6 8 10 ⋄ 0 2 4 6 8 10 12)

⍝ aplcart/table.tsv:951 — All; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (~0∘∊)B   ⍝ 0

⍝ aplcart/table.tsv:952 — Not Any; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (~1∘∊)B   ⍝ 0

⍝ aplcart/table.tsv:953 — Sum of all atoms in N; optional {X} instantiated as dyadic use
N←(1 2⋄ 3(4 5)) ⋄ (+/∊)N   ⍝ 15

⍝ aplcart/table.tsv:954 — Do Xv and Yv have any elements in common?; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←3 4 ⋄ Xv(∨/∊)Yv   ⍝ 1

⍝ aplcart/table.tsv:955 — Is Xv a Subset of Yv?; optional {X} instantiated as dyadic use
Xv←1 2 ⋄ Yv←3 2 1 ⋄ Xv(∧/∊)Yv   ⍝ 1

⍝ aplcart/table.tsv:956 — Replace all occurrences of elements from Y in array Z with X
'x'@(∊∘'AEIOU') 'HELLO AND GOODBYE'   ⍝ 'HxLLx xND GxxDBYx'

⍝ aplcart/table.tsv:957 — Index of first occurrence in X of any item of Y; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X(1⍳⍨∊)Y   ⍝ 1

⍝ aplcart/table.tsv:958 — Are all major cells distinct?
((⊢≡∪) 'abcd' ⋄ (⊢≡∪) 'abca')   ⍝ 1 0

⍝ aplcart/table.tsv:959 — Consecutive ids (indices with equal major cells mapping to same index)
(∪⍳⊢) 2 7 1 8 2 8 1 8   ⍝ 1 2 3 4 1 4 3 4

⍝ aplcart/table.tsv:960 — Elements of Jv divisible by Is; optional {X} instantiated as dyadic use
Is←3 ⋄ Jv←1 2 3 4 5 6 ⋄ Is(⊢∩∧)Jv   ⍝ 3 6

⍝ aplcart/table.tsv:961 — Move elements Yv (which are members of Xv) to the rear of Xv; optional {X} instantiated as dyadic use
Xv←1 2 3 4 5 ⋄ Yv←2 4 ⋄ Xv(~,∩)Yv   ⍝ 1 3 5 2 4

⍝ aplcart/table.tsv:962 — Move elements Yv (which are members of Xv) to the front of Xv; optional {X} instantiated as dyadic use
Xv←1 2 3 4 5 ⋄ Yv←2 4 ⋄ Xv(∩,~)Yv   ⍝ 2 4 1 3 5

⍝ aplcart/table.tsv:963 — Symmetric Set Difference; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←2 3 4 ⋄ Xv(∪~∩)Yv   ⍝ 1 4

⍝ aplcart/table.tsv:964 — Index of last maximum element of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⊢/⍋)Y   ⍝ 3

⍝ aplcart/table.tsv:965 — Index of last minimum element of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⊢/⍒)Y   ⍝ 2

⍝ aplcart/table.tsv:966 — Indicate leading elements that are equal; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X(∧\=)Y   ⍝ 1 1 0 0

⍝ aplcart/table.tsv:967 — Indicate leading elements that are unequal; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X(∧\≠)Y   ⍝ 0 0 0 0

⍝ aplcart/table.tsv:968 — First Js triangular numbers; optional {X} instantiated as dyadic use
Js←4 ⋄ (+\⍳)Js   ⍝ 1 3 6 10

⍝ aplcart/table.tsv:969 — Alternating series (1,-1,2,-2, …) of length Js; optional {X} instantiated as dyadic use; Reduce each prefix to preserve right association
Js←4 ⋄ {-/¨,\⍳⍵}Js   ⍝ 1 ¯1 2 ¯2

⍝ aplcart/table.tsv:970 — Difference of adjacent pairs with seed value; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (+\⍣¯1)N   ⍝ 4 5 7

⍝ aplcart/table.tsv:971 — Convert binary to reflected Gray code; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (≠\⍣¯1)B   ⍝ 1 0 1 1 1 1 1

⍝ aplcart/table.tsv:972 — Is Nm a Hermitian matrix?; optional {X} instantiated as dyadic use
Nm←2 2⍴1 2j3 2j¯3 4 ⋄ (⍉≡+)Nm   ⍝ 1

⍝ aplcart/table.tsv:973 — Is Ym anti-symmetric?; optional {X} instantiated as dyadic use
Ym←2 2⍴0 ¯2 2 0 ⋄ (-≡⍉)Ym   ⍝ 1

⍝ aplcart/table.tsv:974 — Using sample row Yv to form an initially empty matrix for later expansion; optional {X} instantiated as dyadic use
Yv←4 5 6 ⋄ (⍉0/⍪)Yv   ⍝ 0 3⍴0

⍝ aplcart/table.tsv:975 — Boolean rows of Ym starting with X; optional {X} instantiated as dyadic use
Xv←1 2 ⋄ Ym←2 3⍴1 2 3 2 1 3 ⋄ Xv(⊣/⍷)Ym   ⍝ 1 0

⍝ aplcart/table.tsv:976 — First occurrence of string Cv in string Dv; optional {X} instantiated as dyadic use
Cv← 'ab'  ⋄ Dv← 'xxabyyab'  ⋄ Cv(1⍳⍨⍷)Dv
3

⍝ aplcart/table.tsv:977 — Square Root; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (*∘0.5)N   ⍝ 2 3 4

⍝ aplcart/table.tsv:978 — Ohm's Law: resistance of parallel resistors/capacitance of serial capacitors; optional {X} instantiated as dyadic use
Nv←1 2 3 ⋄ (÷1⊥÷)Nv   ⍝ 0.5454545454545455

⍝ aplcart/table.tsv:979 — Count of leading ones; optional {X} instantiated as dyadic use
Bv←1 1 1 0 0 ⋄ (⊥⍨⌽)Bv   ⍝ 3

⍝ aplcart/table.tsv:980 — Count of trailing zeros; optional {X} instantiated as dyadic use
Bv←1 1 1 0 0 ⋄ (⊥⍨~)Bv   ⍝ 2

⍝ aplcart/table.tsv:981 — All column indices of array Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⍳0⊥⍴)Y   ⍝ 1 2 3 4

⍝ aplcart/table.tsv:982 — Digits of N; optional {X} instantiated as dyadic use
N←123 450 7 ⋄ (10∘⊥⍣¯1)N   ⍝ 3 3⍴1 4 0 2 5 0 3 0 7

⍝ aplcart/table.tsv:983 — Binary representation of J; optional {X} instantiated as dyadic use
J←3 8 15 ⋄ (2∘⊥⍣¯1)J   ⍝ 4 3⍴0 1 1 0 0 1 1 0 1 1 0 1

⍝ aplcart/table.tsv:984 — Is Nm an Orthogonal matrix?; optional {X} instantiated as dyadic use
Nm←2 2⍴0 1 ¯1 0 ⋄ (⍉≡⌹)Nm   ⍝ 1

⍝ aplcart/table.tsv:985 — Show all digits of integer Js (unknown digits as “_”)
(3.41252E10 ⋄ (1↓0∘⍕) 3.41252E10)   ⍝ 34125200000 ('34125200000')

⍝ aplcart/table.tsv:986 — Vertically stack digits of ⍳Is (helps locating column positions); optional {X} instantiated as dyadic use
Js←4 ⋄ (1 0⍕10 10⊤⍳)Js   ⍝ 2 4⍴'00001234'

⍝ aplcart/table.tsv:998 — Frequencies of major cells; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ {≢⍵}⌸Y   ⍝ 2 1 1

⍝ aplcart/table.tsv:999 — Group values Y by keys X; optional {X} instantiated as dyadic use
X←1 2 1 2 ⋄ Y←10 20 30 40 ⋄ X{⊂⍵}⌸Y   ⍝ (10 30 ⋄ 20 40)

⍝ aplcart/table.tsv:1000 — Group indices of Y by keys Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ {⊂⍵}⌸Y   ⍝ (1 3 ⋄ 1⍴2 ⋄ 1⍴4)

⍝ aplcart/table.tsv:1001 — Remove every second cell of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ ≢⍛⍴∘1 0⍛⌿Y   ⍝ 3 3

⍝ aplcart/table.tsv:1002 — Continued fraction: 1+÷2+÷3+÷4+÷5+÷6+÷…Js; optional {X} instantiated as dyadic use
Js←4 ⋄ +∘÷/∘⍳Js   ⍝ 1.433333333333333

⍝ aplcart/table.tsv:1003 — Moving width-Is window of indices for array Y; optional {X} instantiated as dyadic use
Is←2 ⋄ Y←3 1 3 2 ⋄ ,/Is↕⍳≢Y   ⍝ (1 2 ⋄ 2 3 ⋄ 3 4)

⍝ aplcart/table.tsv:1004 — Addition Table for Numbers up to Js; optional {X} instantiated as dyadic use
Js←4 ⋄  +⌝ ⍨∘⍳Js   ⍝ 4 4⍴2 3 4 5 3 4 5 6 4 5 6 7 5 6 7 8

⍝ aplcart/table.tsv:1005 — Multiplication Table for Numbers up to Js; optional {X} instantiated as dyadic use
Js←4 ⋄  ×⌝ ⍨∘⍳Js   ⍝ 4 4⍴1 2 3 4 2 4 6 8 3 6 9 12 4 8 12 16

⍝ aplcart/table.tsv:1006 — Maximum table of 1…Js; optional {X} instantiated as dyadic use
Js←4 ⋄  ⌈⌝ ⍨∘⍳Js   ⍝ 4 4⍴1 2 3 4 2 2 3 4 3 3 3 4 4 4 4 4

⍝ aplcart/table.tsv:1007 — Upper triangular matrix without diagonal: Js by Js
 <⌝ ⍨∘⍳ 5
5 5⍴0 1 1 1 1 0 0 1 1 1 0 0 0 1 1 0 0 0 0 1 0 0 0 0 0

⍝ aplcart/table.tsv:1008 — Upper triangular matrix with diagonal: Js by Js
 ≤⌝ ⍨∘⍳ 5
5 5⍴1 1 1 1 1 0 1 1 1 1 0 0 1 1 1 0 0 0 1 1 0 0 0 0 1

⍝ aplcart/table.tsv:1009 — Lower triangular matrix with diagonal: Js by Js
 ≥⌝ ⍨∘⍳ 5
5 5⍴1 0 0 0 0 1 1 0 0 0 1 1 1 0 0 1 1 1 1 0 1 1 1 1 1

⍝ aplcart/table.tsv:1010 — Lower triangular matrix without diagonal: Js by Js
 >⌝ ⍨∘⍳ 5
5 5⍴0 0 0 0 0 1 0 0 0 0 1 1 0 0 0 1 1 1 0 0 1 1 1 1 0

⍝ aplcart/table.tsv:1011 — Probabilistic converse implication; optional {X} instantiated as dyadic use
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(1+×-⊢)N   ⍝ 0 0.875 1

⍝ aplcart/table.tsv:1012 — Probabilistic implication; optional {X} instantiated as dyadic use
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(1+×-⊣)N   ⍝ 1 0.625 0

⍝ aplcart/table.tsv:1013 — Conditional elementwise change of sign; optional {X} instantiated as dyadic use
A←1 0 1 ⋄ N←4 9 16 ⋄ A(⊢×¯1*⊣)N   ⍝ ¯4 9 ¯16

⍝ aplcart/table.tsv:1014 — Join real part M and imaginary part N to form complex; optional {X} instantiated as dyadic use
M←1 2 3 ⋄ N←4 5 6 ⋄ M(⊣+¯11○⊢)N   ⍝ 1j4 2j5 3j6

⍝ aplcart/table.tsv:1015 — Join magnitude M and radians N to form complex; optional {X} instantiated as dyadic use
M←1 2 3 ⋄ N←0 0.5 1 ⋄ M(⊣×¯12○⊢)N
1 1.755165123780746j0.958851077208406 1.620906917604419j2.524412954423689

⍝ aplcart/table.tsv:1016 — Convert from unsigned short integers to signed short integers; optional {X} instantiated as dyadic use
J←0 127 128 255 ⋄ (¯128+256|128∘+)J   ⍝ 0 127 ¯128 ¯1

⍝ aplcart/table.tsv:1017 — Shifting Y left/up one position (padding on right/bottom); optional {X} instantiated as dyadic use
Y←2 3⍴⍳6 ⋄ (≢↑1∘↓)Y   ⍝ 2 3⍴4 5 6 0 0 0

⍝ aplcart/table.tsv:1018 — Is Y a vector?
((1=≢∘⍴) 'Hello' ⋄ (1=≢∘⍴) 'H' ⋄ (1=≢∘⍴) '')
1 0 1

⍝ aplcart/table.tsv:1019 — Number of digits in strictly positive integers in J
(⌊1+10∘⍟) 1618 1 271828   ⍝ 4 1 6

⍝ aplcart/table.tsv:1020 — N Degrees in Radians
N←0 30 90 180 ⋄ (π÷180)∘×N
0 0.5235987755982988 1.570796326794897 3.141592653589793

⍝ aplcart/table.tsv:1021 — Area of sphere with radius N
N←4 9 16 ⋄ (π4×*∘2)N
201.0619298297468 1017.876019763093 3216.990877275948

⍝ aplcart/table.tsv:1022 — Cube Root
N←4 9 16 ⋄ (*∘÷∘3)N
1.587401051968199 2.080083823051904 2.519842099789746

⍝ aplcart/table.tsv:1023 — Probabilistic NOR
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M×⍥(1∘-)N   ⍝ 0 0.375 0

⍝ aplcart/table.tsv:1024 — Mirror complex N across x-axis if As
As←1 ⋄ N←1j2 3j¯4 ⋄ As(¯10+~)⍛○N   ⍝ 1j¯2 3j4

⍝ aplcart/table.tsv:1025 — Number of elements in major cells
Y←2 3 4⍴⍳24 ⋄ (×/1↓⍴)Y   ⍝ 12

⍝ aplcart/table.tsv:1026 — Number of rows in array Y (also of vector)
Y←2 3 4⍴⍳24 ⋄ (×/¯1↓⍴)Y   ⍝ 6

⍝ aplcart/table.tsv:1027 — Volume of pyramid with height, width, length Nv
Nv←1 2 3 ⋄ (3÷⍨×/)Nv   ⍝ 2

⍝ aplcart/table.tsv:1028 — Normalise N so that minimum item is 0
N←¯2 4 1 ⋄ (⊢-⌊⌿)N   ⍝ 0 6 3

⍝ aplcart/table.tsv:1029 — Normalise N so that sum is 1
(⊢÷+⌿) 3 1 4 1
0.3333333333333333 0.1111111111111111 0.4444444444444444 0.1111111111111111

⍝ aplcart/table.tsv:1030 — Is-wise rolling average
Is←2 ⋄ N←2 4 6 8 10 ⋄ (+/Is↕N)÷Is   ⍝ 3 5 7 9

⍝ aplcart/table.tsv:1031 — Arithmetic mean of N
N←2 3⍴⍳6 ⋄ (+⌿÷≢)N   ⍝ 2.5 3.5 4.5

⍝ aplcart/table.tsv:1032 — Convert permutation vectors in I to permutation matrices
(1↑⍨⍤0-) 1 4 2 5 3
5 5⍴1 0 0 0 0 0 0 0 1 0 0 1 0 0 0 0 0 0 0 1 0 0 1 0 0

⍝ aplcart/table.tsv:1033 — Segment lengths from beginning indices
Iv←2 5 9 ⋄ ((-/∘⌽∘(2∘↕))0∘,)Iv   ⍝ 2 3 4

⍝ aplcart/table.tsv:1034 — Positive maximum, at least zero (also for empty N)
N←⍬ ⋄ (⌈/,∘0)N   ⍝ 0

⍝ aplcart/table.tsv:1035 — Matrix to vector using Xs as separator (excludes initial separator)
Xs←9 ⋄ Ym←2 3⍴⍳6 ⋄ Xs(1↓∘,,)Ym   ⍝ 1 2 3 9 4 5 6

⍝ aplcart/table.tsv:1036 — Inserting Xv after every element of Yv
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,,⍤0 1⍨)Yv   ⍝ 4 1 2 3 5 1 2 3 6 1 2 3

⍝ aplcart/table.tsv:1037 — Indicate increases in N
N←1 3 3 2 5 ⋄ (0⍪(>/[2]∘(2∘↕)))N   ⍝ 0 0 0 1 0

⍝ aplcart/table.tsv:1038 — Indicate starting points of groups of equal elements (non-empty Yv)
Y←1 1 2 2 1 ⋄ (1⍪(≢/[2]∘(2∘↕)))Y   ⍝ 1 0 1 0 1

⍝ aplcart/table.tsv:1039 — Indicate which elements differ from previous ones (non-empty Yv)
Y←1 1 2 2 1 ⋄ (1⍪(≢/[2]∘(2∘↕)))Y   ⍝ 1 0 1 0 1

⍝ aplcart/table.tsv:1040 — First ones in each group of ones
B←1 1 0 1 0 1 0 ⋄ ((</[2]∘(2∘↕))0∘⍪)B   ⍝ 1 0 0 1 0 1 0

⍝ aplcart/table.tsv:1041 — Last ones in each group of ones
B←1 1 0 1 0 1 0 ⋄ ((>/[2]∘(2∘↕))⍪∘0)B   ⍝ 0 1 0 1 0 1 0

⍝ aplcart/table.tsv:1042 — Number of items in trailing axis
Y←3 1 3 2 ⋄ (↑⌽∘⍴)Y   ⍝ 4

⍝ aplcart/table.tsv:1043 — The last item of Y
Y←3 1 3 2 ⋄ (↑⌽∘,)Y   ⍝ 2

⍝ aplcart/table.tsv:1044 — Iv copies of Y
3 (⊃⍴∘⊂) 'abc'   ⍝ 3 3⍴'abcabcabc'

⍝ aplcart/table.tsv:1045 — Increment rank by inserting a new dimension before the leading one
(⍴ (⊃,∘⊂) 'abc' ⋄ ⍴ (⊃,∘⊂) 3 4⍴•A)   ⍝ (1 3 ⋄ 1 3 4)

⍝ aplcart/table.tsv:1046 — Select: each element of Iv selects a cell from Y
Iv←(1 2⋄ 2 3) ⋄ Y←2 3⍴⍳6 ⋄ Iv{⍺⊃⍵}¨∘⊂Y   ⍝ 2 6

⍝ aplcart/table.tsv:1047 — Indices of dimensions of Y
Y←3 1 3 2 ⋄ (⍳∘≢⍴)Y   ⍝ 1⍴1

⍝ aplcart/table.tsv:1048 — All indices of Y
Y←3 1 3 2 ⋄ (,∘⍳⍴)Y   ⍝ 1 2 3 4

⍝ aplcart/table.tsv:1049 — Is Xv a Superset of Yv?
Xv←1 2 3 4 ⋄ Yv←2 3 ⋄ Xv(∧/∊⍨)Yv   ⍝ 1

⍝ aplcart/table.tsv:1050 — Changing index of an unfound element to zero (slow)
Xv←1 2 3 ⋄ Yv←2 4 1 ⋄ Xv(∊⍨×⍳)Yv   ⍝ 2 0 1

⍝ aplcart/table.tsv:1051 — Boolean array of shape Iv with zeros in locations J
Iv←2 3 ⋄ J←(1 2⋄ 2 3) ⋄ Iv(~⍳⍛∊)J   ⍝ 2 3⍴1 0 1 1 1 0

⍝ aplcart/table.tsv:1052 — Index of first differing element in X and Y
X←1 2 3 ⋄ Y←1 4 3 ⋄ X(↑∘⍸≠)Y   ⍝ 2

⍝ aplcart/table.tsv:1053 — Indices of items of Yv in right-inclusive intervals with cut-offs Xv
('AEIOU' (⍸-∊⍨) 'DYALOG' ⋄ 2 4 6 (⍸-∊⍨) 1 2 3 4 5 6 7)
(1 5 0 3 3 2 ⋄ 0 0 1 1 2 2 3)

⍝ aplcart/table.tsv:1054 — Index in X of the first element which is a member of Y
X←1 2 3 ⋄ Y←3 4 ⋄ X(↑∘⍸∊)Y   ⍝ 3

⍝ aplcart/table.tsv:1055 — Are any major cells distinct?
Y←3 1 3 2 ⋄ (1<≢∘∪)Y   ⍝ 1

⍝ aplcart/table.tsv:1056 — Are all major cells of Y identical?
((1=≢∘∪) 'abc' 'abc' 'abc' ⋄ (1=≢∘∪) 'abc' 'abc' 'abd')
1 0

⍝ aplcart/table.tsv:1057 — Are all major cells identical?
Y←3 1 3 2 ⋄ (1≥≢∘∪)Y   ⍝ 0

⍝ aplcart/table.tsv:1058 — All divisors of Js
Js←4 ⋄ (∪⍳⍛∨)Js   ⍝ 1 2 4

⍝ aplcart/table.tsv:1059 — Force numbers N to range (-M)≤N≤M
3 (⌊∘-⍣2) 0 1 ¯2 4 ¯5   ⍝ 0 1 ¯2 3 ¯3

⍝ aplcart/table.tsv:1060 — Prefixes of a vector
Yv←4 5 6 ⋄ (,¨,\)Yv   ⍝ (1⍴4 ⋄ 4 5 ⋄ 4 5 6)

⍝ aplcart/table.tsv:1061 — Join (⍪) planes of rank 3 array Y to form a single matrix
Y←2 3 4⍴⍳24 ⋄ (⍉⍪∘⍉)Y
6 4⍴1 2 3 4 13 14 15 16 5 6 7 8 17 18 19 20 9 10 11 12 21 22 23 24

⍝ aplcart/table.tsv:1062 — Is X a Subarray of Y?
X←1 2 ⋄ Y←3 1 2 4 ⋄ X(≡∨1∊⍷)Y   ⍝ 1

⍝ aplcart/table.tsv:1063 — Position of first subarray X in Y
X←1 2 ⋄ Y←3 1 2 4 ⋄ X(↑∘⍸⍷)Y   ⍝ 2

⍝ aplcart/table.tsv:1064 — Positions of item X in Y
X←1 2 ⋄ Y←(1 2⋄ 3 4⋄ 1 2) ⋄ X(⍸⊂⍛⍷)Y   ⍝ 1 3

⍝ aplcart/table.tsv:1065 — Rounding to nearest integer (favouring up)
(⌊0.5+⊢) 31.4 1.5 92.6   ⍝ 31 2 93

⍝ aplcart/table.tsv:1066 — ±N increasing rank
N←4 9 16 ⋄ (1 ¯1 ×⌝ ⊢)N   ⍝ 2 3⍴4 9 16 ¯4 ¯9 ¯16

⍝ aplcart/table.tsv:1067 — ∓N increasing rank
N←4 9 16 ⋄ (¯1 1 ×⌝ ⊢)N   ⍝ 2 3⍴¯4 ¯9 ¯16 4 9 16

⍝ aplcart/table.tsv:1068 — Celsius to Fahrenheit
N←¯40 0 100 ⋄ (32+1.8∘×)N   ⍝ ¯40 32 212

⍝ aplcart/table.tsv:1069 — From complex to magnitude and radians (increase rank with leading length-two axis)
N←1j2 ¯3j4 ⋄ (10 12 ○⌝ ⊢)N
2 2⍴2.23606797749979 5 1.10714871779409 2.214297435588181

⍝ aplcart/table.tsv:1070 — From complex to real and imaginary (increase rank with leading length-two axis)
N←1j2 ¯3j4 ⋄ (9 11 ○⌝ ⊢)N   ⍝ 2 2⍴1 ¯3 2 4

⍝ aplcart/table.tsv:1071 — Sum of common parts of matrices (matrix sum)
Mm←3 2⍴⍳6 ⋄ Nm←2 3⍴⍳6 ⋄ Mm(1 2 1 2⍉ +⌝ )Nm
2 2⍴2 4 7 9

⍝ aplcart/table.tsv:1072 — Product of common parts of matrices (matrix sum)
Mm←3 2⍴⍳6 ⋄ Nm←2 3⍴⍳6 ⋄ Mm(1 2 1 2⍉ ×⌝ )Nm
2 2⍴1 4 12 20

⍝ aplcart/table.tsv:1073 — Direct matrix product
Mm←3 2⍴⍳6 ⋄ Nm←2 3⍴⍳6 ⋄ Mm(1 3 2 4⍉ ×⌝ )Nm
3 2 2 3⍴1 2 3 2 4 6 4 5 6 8 10 12 3 6 9 4 8 12 12 15 18 16 20 24 5 10 15 6 12 18 20 25 30 24 30 36

⍝ aplcart/table.tsv:1074 — Harmonic mean
N←4 9 16 ⋄ (≢÷1⊥÷)N   ⍝ 7.081967213114754

⍝ aplcart/table.tsv:1075 — Count of leading zeros
Bv←0 0 1 1 0 ⋄ (⊥⍨0=⌽)Bv   ⍝ 2

⍝ aplcart/table.tsv:1076 — Manhattan distance between two points in N-space
0 0 ¯1 1 (1⊥∘|-) 0 3 0 0   ⍝ 5

⍝ aplcart/table.tsv:1077 — Convert hours,minutes,seconds to decimal degrees/hours
(3600÷⍨60∘⊥) 1 15 0   ⍝ 1.25

⍝ aplcart/table.tsv:1078 — Future value of cash flows N at interest Ms
Ms←0.1 ⋄ N←100 200 300 ⋄ Ms(1∘+⍛⊥)N   ⍝ 641

⍝ aplcart/table.tsv:1079 — Base-Is digit sum
Is←10 ⋄ J←123 450 7 ⋄ Is(+⌿⊥⍣¯1)J   ⍝ 6 9 7

⍝ aplcart/table.tsv:1080 — An array that begins with 1↑N and has pair-wise sums 1↓N
((¯1⊥¨,\) 3,4 5 5 6 ⋄ (+/∘(2∘↕)) (¯1⊥¨,\) 3,4 5 5 6)
(3 1 4 1 5 ⋄ 4 5 5 6)

⍝ aplcart/table.tsv:1081 — Decoding numeric codes J packed with field widths Iv (ZYYYZZZ:1 3 2)
Iv←1 3 2 ⋄ J←123456 987654 ⋄ Iv(10∘*⍛⊤)J
3 2⍴1 9 234 876 56 54

⍝ aplcart/table.tsv:1086 — Convert letters to their positions in the alphabet
D←'Abc 19 Σς!'  ⋄ (•A⍳1∘•C)D   ⍝ 1 2 3 27 27 27 27 27 27 27

⍝ aplcart/table.tsv:1087 — Sum Nv by buckets Xv (⍴Nv ↔ ⍴Xv)
Xv←1 2 1 2 ⋄ Nv←10 20 30 40 ⋄ Xv{+/⍵}⌸Nv
40 60

⍝ aplcart/table.tsv:1089 — Remove blanks in string
Dv← ' a b  c '  ⋄ (~∘' ')Dv   ⍝ 'abc'

⍝ aplcart/table.tsv:1090 — Two-column matrix from two vectors (pad shorter vector)
Xv←1 2 ⋄ Yv←3 4 5 ⋄ Xv{⍉⊃⍺⍵}Yv   ⍝ 3 2⍴1 3 2 4 0 5

⍝ aplcart/table.tsv:1091 — Catalogue of all pairs from Xv and Yv
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv {⍺⍵}⌝ Yv
3 3⍴(1 4 ⋄ 1 5 ⋄ 1 6 ⋄ 2 4 ⋄ 2 5 ⋄ 2 6 ⋄ 3 4 ⋄ 3 5 ⋄ 3 6)

⍝ aplcart/table.tsv:1092 — First number with smallest magnitude
Nv←¯5 2 ¯2 4 ⋄ ↑∘⍋∘|⍛⊃Nv   ⍝ 2

⍝ aplcart/table.tsv:1093 — Ascending cardinals numbers (ranking, ties equal)
Y←3 1 3 2 ⋄ ⊂⍤⍋⍛⌷⍛⍳Y   ⍝ 3 1 3 2

⍝ aplcart/table.tsv:1094 — Reorder Y according to the order of X
X←3 1 2 1 ⋄ Y←10 20 30 40 ⋄ X⊂⍤⍋⍤⍋⍛⌷Y   ⍝ 40 10 30 20

⍝ aplcart/table.tsv:1095 — First number with largest magnitude
Nv←¯5 2 ¯2 4 ⋄ ↑∘⍒∘|⍛⊃Nv   ⍝ ¯5

⍝ aplcart/table.tsv:1096 — Descending cardinals numbers (ranking, tie equal)
Y←3 1 3 2 ⋄ ⊂⍤⍒⍛⌷⍛⍳Y   ⍝ 1 4 1 3

⍝ aplcart/table.tsv:1098 — 2-argument arctangent (M:x, N:y)
M←1 ¯1 ¯1 1 ⋄ N←1 1 ¯1 ¯1 ⋄ M(12○⊣+0J1×⊢)N
0.7853981633974483 2.356194490192345 ¯2.356194490192345 ¯0.7853981633974483

⍝ aplcart/table.tsv:1099 — Prefix Vector: length Is with Js ones on the left, the rest zeroes
Is←6 ⋄ Js←3 ⋄ Is(⊣↑1⍴⍨⊢)Js   ⍝ 1 1 1 0 0 0

⍝ aplcart/table.tsv:1100 — Starting points for Is fields of width Js
Is←2 ⋄ Js←4 ⋄ Is(×⍴1↑⍨⊢)Js   ⍝ 1 0 0 0 1 0 0 0

⍝ aplcart/table.tsv:1101 — Rounding towards zero
N←¯2.5 0 3.75 ⋄ (××∘⌊|)N   ⍝ ¯2 0 3

⍝ aplcart/table.tsv:1102 — Number of ordered of combinations of I out of J
I←2 3 4 ⋄ J←4 9 16 ⋄ I(!×∘!⊣)J   ⍝ 12 504 43680

⍝ aplcart/table.tsv:1103 — Null near-zero (within absolute distance Ms) values in N
Ms←0.1 ⋄ N←0.05 ¯0.2 1 ⋄ Ms(⊢×<∘|)N   ⍝ 0 ¯0.2 1

⍝ aplcart/table.tsv:1104 — Rounding to zero values of N within M of zero
M←0.1 ⋄ N←0.05 ¯0.2 1 ⋄ M(≤∘|×⊢)N   ⍝ 0 ¯0.2 1

⍝ aplcart/table.tsv:1105 — Shifting Y left/up Is positions (padding on right/bottom)
Is←2 ⋄ Y←3 1 3 2 ⋄ Is(⊢∘≢↑↓)Y   ⍝ 3 2 0 0

⍝ aplcart/table.tsv:1106 — Duplicating vector Yv Is times
Is←2 ⋄ Yv←4 5 6 ⋄ Is(×∘⍴⍴⊢)Yv   ⍝ 4 5 6 4 5 6

⍝ aplcart/table.tsv:1107 — N Radians in Degrees
N←0 1 2 ⋄ (÷π÷180)∘×N   ⍝ 0 57.29577951308232 114.5915590261646

⍝ aplcart/table.tsv:1108 — Volume of sphere with radius N
N←4 9 16 ⋄ (π4÷3÷*∘3)N
268.082573106329 3053.628059289279 17157.28467880506

⍝ aplcart/table.tsv:1109 — Random Boolean array of shape Jv; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Jv←2 3 ⋄ r←(1=∘?⍴∘2)Jv ⋄ (Jv≡⍴r)∧∧/,r∊0 1
1

⍝ aplcart/table.tsv:1110 — Jacobsthal-Lucas number
Js←4 ⋄ (2∘*+¯1∘*)Js   ⍝ 17

⍝ aplcart/table.tsv:1111 — Hartley kernel
N←4 9 16 ⋄ (1∘○+2∘○)N
¯1.41044611617154 ¯0.4990117766429203 ¯1.24556279698845

⍝ aplcart/table.tsv:1112 — Residue after dividing N by M but replacing 0 with M
M←3 ⋄ N←0 3 4 6 ⋄ M(⊣+-⍛|)N   ⍝ 3 3 1 3

⍝ aplcart/table.tsv:1113 — Is Y a Singleton?
Y←1 1 1⍴42 ⋄ (1=×/∘⍴)Y   ⍝ 1

⍝ aplcart/table.tsv:1114 — Comparison of successive rows
Ym←3 2⍴1 2 1 2 3 4 ⋄ (∧/(=/[2]∘(2∘↕)))Ym
1 0

⍝ aplcart/table.tsv:1115 — Drop first and last Iv items along leading axes of Y
Iv←1 2 ⋄ Y←4 6⍴⍳24 ⋄ Iv(-⍤⊣↓↓)Y   ⍝ 2 2⍴9 10 15 16

⍝ aplcart/table.tsv:1116 — Replacing last major cell of Y with Xs
Xs←9 ⋄ Y←3 2⍴⍳6 ⋄ Xs(⊖⊣@1∘⊖)Y   ⍝ 3 2⍴1 2 3 4 9 9

⍝ aplcart/table.tsv:1117 — Forming an Is-row matrix with all rows being Yv
Is←2 ⋄ Yv←4 5 6 ⋄ Is(,∘≢⍴⊢)Yv   ⍝ 2 3⍴4 5 6 4 5 6

⍝ aplcart/table.tsv:1118 — Non-diagonal matrix of shape of matrix Nm
Nm←3 3⍴0 ⋄ (⍴⍴0,1⍨¨)Nm   ⍝ 3 3⍴0 1 1 1 0 1 1 1 0

⍝ aplcart/table.tsv:1119 — Adjust Xm to width Js (positive Js to pad/chop on right, negative Js to pad/chop on left)
Xm←2 3⍴⍳6 ⋄ Js←4 ⋄ Xm(≢⍛,↑⊣)Js   ⍝ 2 4⍴1 2 3 0 4 5 6 0

⍝ aplcart/table.tsv:1120 — Ending points of groups of equal elements (non-empty Yv)
Yv←1 1 2 2 1 ⋄ (1,⍨(≠/∘(2∘↕)))Yv   ⍝ 0 1 0 1 1

⍝ aplcart/table.tsv:1121 — Which elements differ from next ones (non-empty Yv)
Yv←1 1 2 2 1 ⋄ (1,⍨(≠/∘(2∘↕)))Yv   ⍝ 0 1 0 1 1

⍝ aplcart/table.tsv:1122 — Are none true?
B←1 1 0 1 0 1 0 ⋄ (~∨/∘,)B   ⍝ 0

⍝ aplcart/table.tsv:1123 — Join scalar elements of vector Yv with separator Xs
Xs←9 ⋄ Yv←4 5 6 ⋄ Xs(1↓∘,,⍤0)Yv   ⍝ 4 9 5 9 6

⍝ aplcart/table.tsv:1124 — Shift before: Inserting X at front/left/top of Y, pushing corresponding cells off the back/right/bottom edge
X←1 2 ⋄ Y←3 4 5 6 ⋄ X(⊢∘≢↑⍪)Y   ⍝ 1 2 3 4

⍝ aplcart/table.tsv:1125 — Take first and last Iv items along leading axes of Y
Iv←1 2 ⋄ Y←4 6⍴⍳24 ⋄ Iv(↑⍪-⍛↑)Y   ⍝ 2 2⍴1 2 23 24

⍝ aplcart/table.tsv:1126 — Picking one of two values according to Bs
Bs←1 ⋄ Yv←(1 2⋄ 3 4 5) ⋄ Bs(⊢⊃⍨1+⊣)Yv   ⍝ 3 4 5

⍝ aplcart/table.tsv:1127 — Cutting Y at offset Is
Is←2 ⋄ Y←3 1 3 2 ⋄ Is(↑,⍥⊂↓)Y   ⍝ (3 1 ⋄ 3 2)

⍝ aplcart/table.tsv:1128 — Increment rank by inserting a new dimension before the trailing one
⍴ (⊃,∘⊂⍤1) 2 3 4⍴•A   ⍝ 2 3 1 4

⍝ aplcart/table.tsv:1129 — Prototype (converts characters to spaces, numbers to zeros)
Y← ('ab'⋄ 1 2)  ⋄ ↑0⍴,Y   ⍝ '  '

⍝ aplcart/table.tsv:1130 — Index of first one after index Is in Bv
I←2 ⋄ B←0 1 0 0 1 0 ⋄ I(⊣+↓⍳1⍨)B   ⍝ 5

⍝ aplcart/table.tsv:1131 — Coefficients of the binomial (exact, fastest below 10)
Js←6 ⋄ (⊢!⍨0,⍳)Js   ⍝ 1 6 15 20 15 6 1

⍝ aplcart/table.tsv:1132 — Is Y within the range ( 1⌷X , 2⌷X )
(1 3 (6=⍳+3×⍸) 0 ⋄ 1 3 (6=⍳+3×⍸) 1 ⋄ 1 3 (6=⍳+3×⍸) 2 ⋄ 1 3 (6=⍳+3×⍸) 3 ⋄ 1 3 (6=⍳+3×⍸) 4)
0 0 1 0 0

⍝ aplcart/table.tsv:1133 — Is Y outside the range ( 1⌷X , 2⌷X )
(1 3 (6≠⍳+3×⍸) 0 ⋄ 1 3 (6≠⍳+3×⍸) 1 ⋄ 1 3 (6≠⍳+3×⍸) 2 ⋄ 1 3 (6≠⍳+3×⍸) 3 ⋄ 1 3 (6≠⍳+3×⍸) 4)
1 1 0 1 1

⍝ aplcart/table.tsv:1134 — Totatives of Js
Js←12 ⋄ (⍸1=⍳⍛∨)Js   ⍝ 1 5 7 11

⍝ aplcart/table.tsv:1135 — Proper divisors of Js
Js←12 ⋄ (∪⊢∨¯1↓⍳)Js   ⍝ 1 2 3 4 6

⍝ aplcart/table.tsv:1136 — Index of first instance of each major cell
Y←3 1 3 2 ⋄ (⍳⍨∪⍛⍳)Y   ⍝ 1 2 1 4

⍝ aplcart/table.tsv:1137 — Main branch of the Lambert W function (N≥-*¯1)
(×∘*⍨⍣¯1) 0 1 (*1) (*1+*1) (2÷⍨*÷2)
0 0.5671432904097838 1 2.718281828459045 0.5

⍝ aplcart/table.tsv:1138 — Is-smallest (default: the smallest) major cell of Y
names ← 'Bob' 'Dan' 'Cal' 'Abe' ⋄ (2 {⍺←1 ⋄ (⍺⊃⍋⍵)⊃⍵} names ⋄ {⍺←1 ⋄ (⍺⊃⍋⍵)⊃⍵} names)
('Bob' ⋄ 'Abe')

⍝ aplcart/table.tsv:1139 — Is N Non-decreasing?
((⍳∘≢≡⍋) 31 41 59 26 ⋄ (⍳∘≢≡⍋) 31 41 59 265 ⋄ (⍳∘≢≡⍋) 27 1828 1828 4590)
0 1 1

⍝ aplcart/table.tsv:1140 — Is-largest (default: the largest) major cell of Y
names ← 'Bob' 'Dan' 'Cal' 'Abe' ⋄ (2 {⍺←1 ⋄ (⍺⊃⍒⍵)⊃⍵} names ⋄ {⍺←1 ⋄ (⍺⊃⍒⍵)⊃⍵} names)
('Cal' ⋄ 'Dan')

⍝ aplcart/table.tsv:1141 — Is N Non-increasing?
((⍳∘≢≡⍒) 314 159 265 ⋄ (⍳∘≢≡⍒) 314 159 26 5 ⋄ (⍳∘≢≡⍒) 2700 1828 1828 459)
0 1 1

⍝ aplcart/table.tsv:1142 — Indicate trailing elements that are equal
X←1 2 3 4 ⋄ Y←0 2 3 4 ⋄ X(∧\∘⌽=)Y   ⍝ 1 1 1 0

⍝ aplcart/table.tsv:1143 — Indicate trailing elements that are unequal
X←1 2 3 4 ⋄ Y←0 2 0 0 ⋄ X(∧\∘⌽≠)Y   ⍝ 1 1 0 0

⍝ aplcart/table.tsv:1144 — Starting positions of subvectors having lengths Jv
Jv←2 0 3 ⋄ (+\¯1↓1∘,)Jv   ⍝ 1 3 3

⍝ aplcart/table.tsv:1145 — Convert table to inverted table (character data as matrices); dfns display import/wrappers omitted to test underlying arrays
⊂[1] 2 3⍴'Ab' 1 7 'Cdef' 2 3   ⍝ (('Ab' ⋄ 'Cdef') ⋄ 1 2 ⋄ 7 3)

⍝ aplcart/table.tsv:1146 — Main diagonal of any rank array
Y←2 3 4⍴⍳24 ⋄ (⊢⍉⍨1*⍴)Y   ⍝ 1 18

⍝ aplcart/table.tsv:1147 — Anti-diagonal of any rank array
Y←2 3 4⍴⍳24 ⋄ (⌽⍉⍨1*⍴)Y   ⍝ 4 19

⍝ aplcart/table.tsv:1148 — Bayes' formula
Mv←0.2 0.3 0.5 ⋄ Nv←0.5 0.2 0.1 ⋄ Mv(×÷+.×)Nv
0.4761904761904762 0.2857142857142857 0.2380952380952381

⍝ aplcart/table.tsv:1149 — Fahrenheit to Celsius
N←¯40 32 212 ⋄ (1.8÷⍨¯32∘+)N   ⍝ ¯40 0 100

⍝ aplcart/table.tsv:1150 — Number of segments in delimited string Dv where the first character is the delimiter ≢ ⍴
Dv← '/ab/cd/ef'  ⋄ (↑+.=⊢)Dv   ⍝ 3

⍝ aplcart/table.tsv:1151 — Conversion of indices Jm of array to indices of ravelled array
Jm←2 3⍴1 1 2 1 2 3 ⋄ (1+⍴⊥¯1∘+)Jm   ⍝ 1 2 6

⍝ aplcart/table.tsv:1152 — Hamming weight
J←0 3 15 ⋄ 2∘(+⌿⊥⍣¯1)J   ⍝ 0 2 4

⍝ aplcart/table.tsv:1153 — ISBN check digit generator from ten first digits Jv
Jv←0 3 0 6 4 0 6 1 5 ⋄ (|¯11|1⊥+\)Jv   ⍝ 5

⍝ aplcart/table.tsv:1154 — Survivor number in the Josephus problem of order Js
Js←10 ⋄ (2⊥1⊖2⊥⍣¯1⊢)Js   ⍝ 5

⍝ aplcart/table.tsv:1155 — Convert decimal degrees/hours to hours,minutes,seconds
(××0 60 60⊤3600×|) 1.25   ⍝ 1 15 0

⍝ aplcart/table.tsv:1156 — Is Nm a Unitary matrix?
Nm←2 2⍴0 0j1 0j1 0 ⋄ (⌹≡⍉∘+)Nm   ⍝ 1

⍝ aplcart/table.tsv:1158 — Safe conversion of string into integer; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
(10⊥¯1+•D∘⍳)'2718'   ⍝ 2718

⍝ aplcart/table.tsv:1160 — Do characters in D have no case?
D←'Abc 19 Σς!'  ⋄ (¯1∘•C=1∘•C)D   ⍝ 0 0 0 1 1 1 1 0 0 1

⍝ aplcart/table.tsv:1162 — Indices of first blanks in rows of array D
D← 3 4⍴'abc a  b xyz'  ⋄ (⍳∘' '⍤1)D   ⍝ 4 2 1

⍝ aplcart/table.tsv:1164 — Shuffle major cells; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Y←4 2⍴⍳8 ⋄ r←⊂⍤?⍨∘≢⍛⌷Y ⋄ Y≡r[⍋r;]   ⍝ 1

⍝ aplcart/table.tsv:1165 — Transitive closure
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ ∨.∧⍨⍛∨⍣≡Bm   ⍝ 3 3⍴1 1 1 1 1 1 1 1 1

⍝ aplcart/table.tsv:1166 — Rounding to nearest even integer (favouring up)
N←¯3 ¯2.5 0 1 2.5 3 ⋄ (⌊⊢+1≤2|⊢)N   ⍝ ¯2 ¯2 0 2 2 4

⍝ aplcart/table.tsv:1167 — Replacing zeroes in N with corresponding elements of M
M←10 20 30 ⋄ N←0 1 0 ⋄ M(⊢+⊣×0=⊢)N   ⍝ 10 1 30

⍝ aplcart/table.tsv:1168 — Change all 0s in J into Is
10 (⊢+⊣×0=⊢) 3 1 0 2 7 0 0 1 6   ⍝ 3 1 10 2 7 10 10 1 6

⍝ aplcart/table.tsv:1169 — Reshaping non-empty lower-rank array Yv into a matrix
Yv←4 5 6 ⋄ (⊢⍴⍨1⌈¯2↑⍴)Yv   ⍝ 1 3⍴4 5 6

⍝ aplcart/table.tsv:1170 — Volume of cone with height M and radius N
M←2 3 4 ⋄ N←4 9 16 ⋄ M(π××3÷⍨⊢)N
33.51032163829112 254.4690049407732 1072.330292425316

⍝ aplcart/table.tsv:1171 — Effective rate of interest with nominal rate N for I periods
I←12 ⋄ N←0.05 ⋄ I(⊣*⍨1+÷⍨)N   ⍝ 1.051161897881733

⍝ aplcart/table.tsv:1172 — Shifting Y right/down one position (padding on left/top)
Y←2 3⍴⍳6 ⋄ (-∘≢↑¯1∘↓)Y   ⍝ 2 3⍴0 0 0 1 2 3

⍝ aplcart/table.tsv:1173 — Resign: Transfer of sign from M to N
M←¯1 0 1 ⋄ N←2 ¯3 4 ⋄ M(×∘×⍨∘|)N   ⍝ ¯2 0 4

⍝ aplcart/table.tsv:1174 — Normalisation by Infinity-norm
(⊢÷⌈⌿∘|) 3 4   ⍝ 0.75 1

⍝ aplcart/table.tsv:1175 — Geometric mean
N←1 4 16 ⋄ (×⌿*∘÷≢)N   ⍝ 4

⍝ aplcart/table.tsv:1176 — Row-wise percentage per row
N←2 3⍴1 2 3 4 5 6 ⋄ (100×⊢÷⍤1 0+/)N
2 3⍴16.66666666666666 33.33333333333333 50 26.66666666666667 33.33333333333333 40

⍝ aplcart/table.tsv:1177 — Column-wise percentage per column
N←2 3⍴1 2 3 4 5 6 ⋄ (100×⊢÷⍤1 +⌿)N
2 3⍴20 28.57142857142857 33.33333333333333 80 71.42857142857143 66.66666666666666

⍝ aplcart/table.tsv:1178 — Apply function f (optionally with left argument X) to last cell of Y
X←10 ⋄ f←+ ⋄ Y←1 2 3 ⋄ X f@(1↑⍨∘-≢)Y   ⍝ 1 2 13

⍝ aplcart/table.tsv:1179 — Rotation matrix for angle Ns (in radians) counter-clockwise
Ns←0.5 ⋄ (2 2⍴2 1 1 2-@2⍤○⊢)Ns
2 2⍴0.8775825618903728 ¯0.479425538604203 0.479425538604203 0.8775825618903728

⍝ aplcart/table.tsv:1180 — Increase rank scalar/vector/matrix Ym to 2 (matrix: 1-row if vector, 1-column if scalar); nested displayed values saved once in evaluation order and returned together
(r1←(⊢⍴⍨¯2↑1 1,⍴) 'a' ⋄ ⍴ r1 ⋄ r3←(⊢⍴⍨¯2↑1 1,⍴) 'abc' ⋄ ⍴ r3 ⋄ r5←(⊢⍴⍨¯2↑1 1,⍴) 2 3⍴'abcdef' ⋄ ⍴ r5)
(1 1⍴'a' ⋄ 1 1 ⋄ 1 3⍴'abc' ⋄ 1 3 ⋄ 2 3⍴'abcdef' ⋄ 2 3)

⍝ aplcart/table.tsv:1181 — Diagonal matrix of size Jv (n or m,n)
(2∘⍴⍴1,⍴∘0) 3   ⍝ 3 3⍴1 0 0 0 1 0 0 0 1

⍝ aplcart/table.tsv:1182 — Non-diagonal matrix of order Js
Js←4 ⋄ (,⍨⍴0,⍴∘1)Js   ⍝ 4 4⍴0 1 1 1 1 0 1 1 1 1 0 1 1 1 1 0

⍝ aplcart/table.tsv:1183 — Choose the number closer to zero (the left one if tied)
Is←¯2 ⋄ Js←3 ⋄ Is(↑>⍥|⌽,)Js   ⍝ ¯2

⍝ aplcart/table.tsv:1184 — Increment rank by inserting a new dimension after the leading one
⍴ (⊃∘,∘⊂⍤¯1) 2 3 4⍴•A   ⍝ 2 1 3 4

⍝ aplcart/table.tsv:1185 — Membership (∊) on major cells for any rank
X←3 2⍴1 2 3 4 1 2 ⋄ Y←2 2⍴3 4 5 6 ⋄ X(⊢∘≢≥⍳⍨)Y
0 1 0

⍝ aplcart/table.tsv:1186 — Euler's totient function (fastest up to about 1000)
(+/1=⊢∨⍳) 1000   ⍝ 400

⍝ aplcart/table.tsv:1187 — Prefixes
((⍳∘≢↑¨⊂) 'ABCD' ⋄ (⍳∘≢↑¨⊂) ⍪'ABCD')
((1⍴'A' ⋄ 'AB' ⋄ 'ABC' ⋄ 'ABCD') ⋄ (1 1⍴'A' ⋄ 2 1⍴'AB' ⋄ 3 1⍴'ABC' ⋄ 4 1⍴'ABCD'))

⍝ aplcart/table.tsv:1188 — Indicator of first occurrence of each unique major cell of Y
Y←3 1 3 2 ⋄ (⍳⍨=⍳∘≢)Y   ⍝ 1 1 0 1

⍝ aplcart/table.tsv:1189 — Zeroing elements of N that are found in M
M←2 4 ⋄ N←1 2 3 4 ⋄ M(⊢×∘~∊⍨)N   ⍝ 1 0 3 0

⍝ aplcart/table.tsv:1190 — Split Yv at occurrences of sequences of elements in Xv (removes separators and empty segments)
Xv←0 9 ⋄ Yv←0 1 2 9 0 3 0 ⋄ Xv(~⍤∊⍨⊆⊢)Yv
(1 2 ⋄ 1⍴3)

⍝ aplcart/table.tsv:1191 — Identity of two sets
Xv←1 2 3 ⋄ Yv←3 2 1 1 ⋄ Xv(∧/∊⍨,∊)Yv   ⍝ 1

⍝ aplcart/table.tsv:1192 — Changing starting indicators Bv of subvectors to lengths
Bv←1 0 1 0 0 ⋄ ((-/∘⌽∘(2∘↕))∘⍸,∘1)Bv   ⍝ 2 3

⍝ aplcart/table.tsv:1193 — Segment lengths from ending indices
Iv←0 1 0 0 1 ⋄ ((-/∘⌽∘(2∘↕))∘⍸1∘,)Iv   ⍝ 2 3

⍝ aplcart/table.tsv:1194 — Assign ranking based on non-descending scores Nv (ties all get average ranking of used slots)
(2÷⍨⍳+⍸)⍨ 3 3 3 5 8 8 21   ⍝ 2 2 2 4 5.5 5.5 7

⍝ aplcart/table.tsv:1195 — Position of first item in X not in Y
X←1 2 3 ⋄ Y←1 3 ⋄ X(↑∘⍸∘~∊)Y   ⍝ 2

⍝ aplcart/table.tsv:1196 — Is Y within the range [ 1⌷X , 2⌷X ]
(1 3 (1=4 9⍸⍳+3×⍸) 0 ⋄ 1 3 (1=4 9⍸⍳+3×⍸) 1 ⋄ 1 3 (1=4 9⍸⍳+3×⍸) 2 ⋄ 1 3 (1=4 9⍸⍳+3×⍸) 3 ⋄ 1 3 (1=4 9⍸⍳+3×⍸) 4)
0 1 1 1 0

⍝ aplcart/table.tsv:1197 — Is Y within the range ( 1⌷X , 2⌷X ]
(1 3 (1=6 9⍸⍳+3×⍸) 0 ⋄ 1 3 (1=6 9⍸⍳+3×⍸) 1 ⋄ 1 3 (1=6 9⍸⍳+3×⍸) 2 ⋄ 1 3 (1=6 9⍸⍳+3×⍸) 3 ⋄ 1 3 (1=6 9⍸⍳+3×⍸) 4)
0 0 1 1 0

⍝ aplcart/table.tsv:1198 — Is Y outside the range [ 1⌷X , 2⌷X ]
(1 3 (1≠4 9⍸⍳+3×⍸) 0 ⋄ 1 3 (1≠4 9⍸⍳+3×⍸) 1 ⋄ 1 3 (1≠4 9⍸⍳+3×⍸) 2 ⋄ 1 3 (1≠4 9⍸⍳+3×⍸) 3 ⋄ 1 3 (1≠4 9⍸⍳+3×⍸) 4)
1 0 0 0 1

⍝ aplcart/table.tsv:1199 — Is Y outside the range ( 1⌷X , 2⌷X ]
(1 3 (1≠6 9⍸⍳+3×⍸) 0 ⋄ 1 3 (1≠6 9⍸⍳+3×⍸) 1 ⋄ 1 3 (1≠6 9⍸⍳+3×⍸) 2 ⋄ 1 3 (1≠6 9⍸⍳+3×⍸) 3 ⋄ 1 3 (1≠6 9⍸⍳+3×⍸) 4)
1 1 0 0 1

⍝ aplcart/table.tsv:1200 — Are all elements of simple Y equal?
((1=≢∘∪∘,) 2 3⍴'a' ⋄ (1=≢∘∪∘,) 2 3⍴'ab')
1 0

⍝ aplcart/table.tsv:1201 — Number-of-divisors of Js
Js←4 ⋄ (≢∘∪⍳⍛∨)Js   ⍝ 3

⍝ aplcart/table.tsv:1202 — Sort each row in ascending order
Y←2 3⍴3 1 2 2 3 1 ⋄ (⊂⍤⍋⍛⌷⍤1)Y   ⍝ 2 3⍴1 2 3 1 2 3

⍝ aplcart/table.tsv:1203 — Sort major cells ascending
Y←2 3⍴3 1 2 2 3 1 ⋄ (⊂⍤⍋⍛⌷⍤¯1)Y   ⍝ 2 3⍴1 2 3 1 2 3

⍝ aplcart/table.tsv:1204 — Choosing grading direction (¯1,0,1) dynamically during execution
Is←¯1 ⋄ Y←3 1 3 2 ⋄ Is(⍋×∘⍋∘⍋)Y   ⍝ 3 1 4 2

⍝ aplcart/table.tsv:1205 — Sort major cells descending
Y←2 3⍴3 1 2 2 3 1 ⋄ (⊂⍤⍒⍛⌷⍤¯1)Y   ⍝ 2 3⍴3 2 1 3 2 1

⍝ aplcart/table.tsv:1206 — Mask to get subvectors with indices Iv as indicated by Bv
Iv←1 3 ⋄ Bv←1 0 1 0 1 0 ⋄ Iv(+\⍤⊢∊⊣)Bv   ⍝ 1 1 0 0 1 1

⍝ aplcart/table.tsv:1207 — Are columns of N in ascending order?
N←3 2⍴1 3 2 2 3 1 ⋄ (∧⌿⌈⍀⍛=)N   ⍝ 1 0

⍝ aplcart/table.tsv:1208 — Are columns of N in descending order?
N←3 2⍴1 3 2 2 3 1 ⋄ (∧⌿⌊⍀⍛=)N   ⍝ 0 1

⍝ aplcart/table.tsv:1209 — Increment on change: Array of same shape as Y beginning with 1 and increasing for each change in adjacent values
(+⍀1⍪(≢/[2]∘(2∘↕))) 'Mississippi'   ⍝ 1 2 3 3 4 5 5 6 7 7 8

⍝ aplcart/table.tsv:1210 — Matrix with shape of Xm and Yv as its columns
Xm←2 3⍴⍳6 ⋄ Yv←4 5 6 ⋄ Xm(⍉⌽⍤⍴⍛⍴)Yv   ⍝ 2 3⍴4 6 5 5 4 6

⍝ aplcart/table.tsv:1211 — Convert inverted table to table (character data as matrices; keep trailing spaces); dfns display import/wrappers omitted to test underlying arrays
(⍉∘⊃⊂⍤¯1¨) (2 4⍴'Ab  Cdef' ⋄ 1 2 ⋄ 7 3)   ⍝ 2 3⍴('Ab  ') 1 7 ('Cdef') 2 3

⍝ aplcart/table.tsv:1212 — Surround matrix Ym with scalar Xs
Xs←9 ⋄ Ym←2 3⍴⍳6 ⋄ Xs(,∘⌽∘⍉⍣4)Ym
4 5⍴9 9 9 9 9 9 1 2 3 9 9 4 5 6 9 9 9 9 9 9

⍝ aplcart/table.tsv:1213 — Index of first consecutive occurrence of major cells of X in Y
X←2 2⍴3 4 5 6 ⋄ Y←4 2⍴1 2 3 4 5 6 7 8 ⋄ X(↑⍤¯1⍤⍷⍳1⍨)Y
2

⍝ aplcart/table.tsv:1214 — Conway's Game of Life: next generation given Bv of 140 surviving 3-by-3 subarrays; the Game of Life recipe constructs its table of surviving neighbourhoods
states←(⊂3 3)⍴¨↓⍉(9⍴2)⊤¯1+⍳512 ⋄ Bv←states/⍨{(3=+/∊⍵)∨(1=2 2⊃⍵)∧4=+/∊⍵}¨states ⋄ Bm←5 5⍴0 0 0 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0 ⋄ (Bv∊⍨⊢∘⊂⌺3 3)Bm
5 5⍴0 0 0 0 0 0 0 0 0 0 0 1 1 1 0 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/table.tsv:1215 — Remove blank rows
Dm←3 3⍴'abc   de '  ⋄ ∨.≠∘' '⍛⌿Dm   ⍝ 2 3⍴'abcde '

⍝ aplcart/table.tsv:1216 — Binomial coefficients until Js
Js←4 ⋄ ( !⌝ ⍨0,⍳)Js
5 5⍴1 1 1 1 1 0 1 2 3 4 0 0 1 3 6 0 0 0 1 4 0 0 0 0 1

⍝ aplcart/table.tsv:1217 — Set Identity (are the sets identical?)
Xv←1 2 3 ⋄ Yv←3 2 1 ⋄ Xv(∊∧.∧∊⍨)Yv   ⍝ 1

⍝ aplcart/table.tsv:1218 — Row averages (0 if none)
N←2 0⍴0 ⋄ (+/÷1⌈0⊥⍴)N   ⍝ 0 0

⍝ aplcart/table.tsv:1219 — Base-Is digital root
Is←10 ⋄ J←123 999 ⋄ Is(+⌿⊥⍣¯1)⍣≡J   ⍝ 6 9

⍝ aplcart/table.tsv:1220 — Weighted average of columns of Nm with weights Mv
Mv←1 2 ⋄ Nm←2 3⍴1 2 3 4 5 6 ⋄ Mv(+.×÷1⊥⊣)Nm
3 4 5

⍝ aplcart/table.tsv:1221 — Coefficients of least squares linear fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←1 3 5 7 ⋄ Mv(⊢⌹1,∘⍪⊣)Nv   ⍝ 1 2

⍝ aplcart/table.tsv:1223 — Is D entirely ASCII-only
D←'Abc 19 Σς!'  ⋄ (∧/127≥•UCS∘∊)D   ⍝ 0

⍝ aplcart/table.tsv:1230 — SWIFT check digit from Is bank number
Is←123456 ⋄ (¯97(|-⊣)⊢)Is   ⍝ 72

⍝ aplcart/table.tsv:1231 — Vector (Jv[1]⍴1),(Jv[2]⍴0),(Jv[3]⍴1),…
Jv←1 2 3 ⋄ {⍵/1 0⍴⍨≢⍵}Jv   ⍝ 1 0 0 1 1 1

⍝ aplcart/table.tsv:1232 — Convert fraction to (numerator,denominator)
Ns←0.75 ⋄ ((,÷∨)∘1)Ns   ⍝ 3 4

⍝ aplcart/table.tsv:1233 — Locate leading blanks
D← 2 4⍴'  ab c d'  ⋄ (∨\' '∘≠)D   ⍝ 2 4⍴0 0 1 1 0 1 1 1

⍝ aplcart/table.tsv:1234 — Differences of successive elements of N along direction Is
Is←2 ⋄ N←2 3⍴1 3 6 2 5 9 ⋄ (-/∘⌽∘(2∘↕)⍤1)N
2 2⍴2 3 3 4

⍝ aplcart/table.tsv:1235 — Merge the leading Is axes of Y
(a ← 2 3 4⍴⍳24 ⋄ ⍴ a ⋄ ⍴ 1 {,[⍳⍺]⍵} a ⋄ ⍴ 2 {,[⍳⍺]⍵} a ⋄ ⍴ 3 {,[⍳⍺]⍵} a)
(2 3 4⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 ⋄ 2 3 4 ⋄ 2 3 4 ⋄ 6 4 ⋄ 1⍴24)

⍝ aplcart/table.tsv:1236 — Does D have any duplicated spaces? (per row)
D←2 4⍴'a  bc d '  ⋄ (∨/'  '∘⍷)D   ⍝ 1 0

⍝ aplcart/table.tsv:1237 — Mask for blank rows
D←3 3⍴'abc   de '  ⋄ (∧.=∘' ')D   ⍝ 0 1 0

⍝ aplcart/table.tsv:1238 — Word lengths of words in list D
D←3 3⍴'abc   de '  ⋄ (+.≠∘' ')D   ⍝ 3 0 2

⍝ aplcart/table.tsv:1239 — Number of leading blanks
Dv← '  abc '  ⋄ (⊥⍨' '=⌽)Dv   ⍝ 2

⍝ aplcart/table.tsv:1240 — Number of trailing blanks
Dv← '  abc '  ⋄ (⊥⍨' '∘=)Dv   ⍝ 1

⍝ aplcart/table.tsv:1243 — Indices (⍳) in X of items of Y; Upstream compose calls dyadic-only find monadically (Dyalog SYNTAX ERROR); use atop ⍤⍷ to preserve both arguments and compute the described indices
X←1 2 3 4 2 ⋄ Y←2 4 ⋄ X(↑∘⍸⍤⍷)¨∘⊂⍨Y   ⍝ 2 4

⍝ aplcart/table.tsv:1244 — Beta function
Ms←2 ⋄ Ns←3 ⋄ Ms(+÷××⊣!+)Ns   ⍝ 0.08333333333333333

⍝ aplcart/table.tsv:1245 — Skew N in y-axis by fraction Ms
Ms←0.5 ⋄ N←1j2 3j4 ⋄ Ms(⊢+¯11○⊣×9○⊢)N   ⍝ 1j2.5 3j5.5

⍝ aplcart/table.tsv:1246 — Number of digits in integers in J
(⌊1+10⍟|+0∘=) 1618 0 ¯271828   ⍝ 4 1 6

⍝ aplcart/table.tsv:1247 — Ending points for Is fields of width Js
Is←2 ⋄ Js←4 ⋄ Is(×⍴1↑⍨∘-⊢)Js   ⍝ 0 0 0 1 0 0 0 1

⍝ aplcart/table.tsv:1248 — Increasing absolute value without change of sign
M←1 ⋄ N←¯3 0 2 ⋄ M(⊢∘××+∘|)N   ⍝ ¯4 0 3

⍝ aplcart/table.tsv:1249 — Jacobsthal number
Js←4 ⋄ (3÷⍨2∘*-¯1∘*)Js   ⍝ 5

⍝ aplcart/table.tsv:1250 — Length of path given as complex points
Nv←0 3 3j4 ⋄ (+/∘|(-/∘(2∘↕)))Nv   ⍝ 7

⍝ aplcart/table.tsv:1251 — Suffix Vector: length Is with Js ones on the right, the rest zeroes
Is←6 ⋄ Js←3 ⋄ Is(-⍤⊣↑1⍴⍨⊢)Js   ⍝ 0 0 0 1 1 1

⍝ aplcart/table.tsv:1252 — Identity matrix of order Js
Js←4 ⋄ (,⍨⍴1∘+↑1⍨)Js   ⍝ 4 4⍴1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1

⍝ aplcart/table.tsv:1253 — Range (difference between largest and smallest element) in N
N←4 9 16 ⋄ (⌈/-⌊/)∘,N   ⍝ 12

⍝ aplcart/table.tsv:1254 — Picking one of three values according to sign of Ms
Ms←¯1 ⋄ Yv←10 20 30 ⋄ Ms(⊢⊃⍨2+∘×⊣)Yv   ⍝ 10

⍝ aplcart/table.tsv:1255 — Split Yv (which has to be simple) at occurrences of Xs (removes separators and keeps empty segments)
Xs←0 ⋄ Yv←0 1 2 0 0 3 0 ⋄ Xs(1↓¨,⊂⍨1,=)Yv
(⍬ ⋄ 1 2 ⋄ ⍬ ⋄ 1⍴3 ⋄ ⍬)

⍝ aplcart/table.tsv:1256 — Align the diagonals of a matrix into columns (with wrap-around)
Ym←2 3⍴⍳6 ⋄ (⊢⌽⍨¯1+⍳∘≢)Ym   ⍝ 2 3⍴1 2 3 5 6 4

⍝ aplcart/table.tsv:1257 — First indices in X of major cells Y, 0 if not found
X←1 2 3 ⋄ Y←2 4 ⋄ X(⍳|⍨1+∘≢⊣)Y   ⍝ 2 0

⍝ aplcart/table.tsv:1258 — Integers from -Js to Js
Js←4 ⋄ (⌽⌽,0,-)∘⍳Js   ⍝ ¯4 ¯3 ¯2 ¯1 0 1 2 3 4

⍝ aplcart/table.tsv:1259 — All possible subvectors grouped by length (Yv must be simple)
Yv←1 2 3 4 ⋄ (⍳≢Yv){,/⍺↕⍵}¨⊂Yv
(1 2 3 4 ⋄ (1 2 ⋄ 2 3 ⋄ 3 4) ⋄ (1 2 3 ⋄ 2 3 4) ⋄ 1⍴⊂(1 2 3 4))

⍝ aplcart/table.tsv:1260 — Indices of major cells of Y in right-inclusive intervals with cut-offs X
mat←3 2⍴⍳6 ⋄ (mat (⍸-⍳≤∘≢⊣) 3 3 ⋄ mat (⍸-⍳≤∘≢⊣) 3 4)
1 1

⍝ aplcart/table.tsv:1261 — Sum of positive divisors of Js
Js←12 ⋄ (+/∘∪⍳⍛∨)Js   ⍝ 28

⍝ aplcart/table.tsv:1262 — Are X and Y permutations of each other?
X←1 2 3 1 ⋄ Y←3 1 1 2 ⋄ X≡⍥(⊂∘⍋⌷⊢)Y   ⍝ 1

⍝ aplcart/table.tsv:1263 — Is N Strictly Decreasing?
((⍳∘≢≡⌽∘⍋) 314 159 265 ⋄ (⍳∘≢≡⌽∘⍋) 314 159 26 5 ⋄ (⍳∘≢≡⌽∘⍋) 2700 1828 1828 459)
0 1 0

⍝ aplcart/table.tsv:1264 — Move items Yv to end of Xv
Xv←1 2 3 4 5 ⋄ Yv←2 4 ⋄ Xv(∊⊂⍤⍋⍛⌷⊣)Yv   ⍝ 1 3 5 2 4

⍝ aplcart/table.tsv:1265 — Is N Strictly Increasing?
((⍳∘≢≡⌽∘⍒) 31 41 59 26 ⋄ (⍳∘≢≡⌽∘⍒) 31 41 59 265 ⋄ (⍳∘≢≡⌽∘⍒) 27 1828 1828 4590)
0 1 0

⍝ aplcart/table.tsv:1266 — Remove leading blanks
Dv← '  a b '  ⋄ ∨\⍤≠∘' '⍛/Dv   ⍝ 'a b '

⍝ aplcart/table.tsv:1267 — Remove leading zeroes; the Game of Life recipe constructs its table of surviving neighbourhoods
(∨\⍤≠∘'0'⍛/)'0001020' 
'1020'

⍝ aplcart/table.tsv:1268 — Position of first item Y in X
X←(1 2⋄ 3 4⋄ 1 2) ⋄ Y←1 2 ⋄ X(↑∘⍸⍷⍨∘⊂)Y   ⍝ 1

⍝ aplcart/table.tsv:1269 — Area of box with sides Nv
Nv←1 2 3 ⋄ (2×⊢+.×1∘⌽)Nv   ⍝ 22

⍝ aplcart/table.tsv:1270 — Rounding to nearest whole number (favouring towards 0)
N←¯2.5 ¯1.5 1.5 2.5 ⋄ (××∘⌈¯0.5+|)N   ⍝ ¯2 ¯1 1 2

⍝ aplcart/table.tsv:1271 — Rounding to nearest whole number (favouring away from 0)
N←¯2.5 ¯1.5 1.5 2.5 ⋄ (××∘⌊0.5+|)N   ⍝ ¯3 ¯2 2 3

⍝ aplcart/table.tsv:1272 — Indicate which numbers in N are perfect squares
(=∘⌊⍨*∘0.5) 1 2 3 4 5 6 7 8 9 10   ⍝ 1 0 0 1 0 0 0 0 1 0

⍝ aplcart/table.tsv:1273 — Remove blank columns
Dm←3 3⍴'abc   de '  ⋄ ∨.≠⍨∘' '⍛/Dm   ⍝ 3 3⍴'abc   de '

⍝ aplcart/table.tsv:1274 — Boolean matrix indicating saddle points
Nm←3 3⍴3 1 2 4 2 3 5 3 4 ⋄ (⌊/ =⌝ ⌈⌿)Nm   ⍝ 3 3⍴0 0 0 0 0 0 0 1 0

⍝ aplcart/table.tsv:1275 — Is Ns a prime?
Ns←7 ⋄ (2=0+.=⍳⍛|)Ns   ⍝ 1

⍝ aplcart/table.tsv:1276 — Hilbert matrix of order Js
Js←4 ⋄ (÷⍳ +⌝ ¯1+⍳)Js
4 4⍴1 0.5 0.3333333333333333 0.25 0.5 0.3333333333333333 0.25 0.2 0.3333333333333333 0.25 0.2 0.1666666666666667 0.25 0.2 0.1666666666666667 0.1428571428571428

⍝ aplcart/table.tsv:1277 — Move set of points Nm into first quadrant
Nm←3 3⍴¯2 1 4 0 ¯3 2 1 5 ¯1 ⋄ (1 2 1⍉⊢ -⌝ ⌊/)Nm
3 3⍴0 3 6 3 0 5 2 6 0

⍝ aplcart/table.tsv:1278 — Is J (YYYY) a leap year?
J←1900 2000 2024 2025 ⋄ (0≠.=400 100 4 |⌝ ⊢)J
0 1 1 0

⍝ aplcart/table.tsv:1279 — Manhattan distance table for points in N-space (one point per row); nested displayed values saved once in evaluation order and returned together
(r1←4 2⍴0 0 0 1 1 0 1 1 ⋄ (1⊥∘|-)⍤1⍤1 99⍨ r1)
(4 2⍴0 0 0 1 1 0 1 1 ⋄ 4 4⍴0 1 1 2 1 0 2 1 1 2 0 1 2 1 1 0)

⍝ aplcart/table.tsv:1280 — Weighted average of rows of Nm with weights Mv
Mv←1 2 3 ⋄ Nm←2 3⍴1 2 3 4 5 6 ⋄ Mv(+.×⍨÷1⊥⊣)Nm
2.333333333333333 5.333333333333333

⍝ aplcart/table.tsv:1281 — Digital sum in base Is
Is←10 ⋄ Js←12345 ⋄ Is(⊣⊥⊣|1⊥⊥⍣¯1)Js   ⍝ 5

⍝ aplcart/table.tsv:1282 — Indices of Iv'th elements in ravel order of an array of dimensions Jv
Iv←1 4 6 ⋄ Jv←2 3 ⋄ Iv(,⌿1+⊢⊤¯1+⊣)Jv   ⍝ (1 1 ⋄ 2 1 ⋄ 2 3)

⍝ aplcart/table.tsv:1283 — Formatting N with Jv decimals in fields of width Iv; Swap outer arguments so the width/precision specification, not the data matrix, goes through (∊,⍤0/); original recipe errors in Dyalog
Iv←7 8 ⋄ Jv←1 2 ⋄ N←2 2⍴1.25 2.375 3.25 4.625 ⋄ N⍕⍨∘(∊,⍤0/)Iv Jv
2 15⍴'    1.3    2.38    3.3    4.63'

⍝ aplcart/table.tsv:1285 — Ascending shortlex grade
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⍋(≢,⊂)¨)Yv   ⍝ 3 1 2 4

⍝ aplcart/table.tsv:1286 — Descending shortlex grade
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⍒(≢,⊂)¨)Yv   ⍝ 4 2 1 3

⍝ aplcart/table.tsv:1287 — Mask for blank columns
D←3 3⍴'  a  b  c'  ⋄ (∧.=⍨∘' ')D   ⍝ 1 1 0

⍝ aplcart/table.tsv:1288 — Underlines a string (1=⎕IO)
Dv← 'xxabyyab'  ⋄ (,[0.5]∘'¯')Dv   ⍝ 2 8⍴'xxabyyab¯¯¯¯¯¯¯¯'

⍝ aplcart/table.tsv:1289 — Ravel order indices of elements at indices Jv in an array of dimensions Jv
Iv←2 3 ⋄ Jv←(1 1⋄ 2 1⋄ 2 3) ⋄ Iv{1+⍺∘⊥¨⍵-1}Jv
1 4 6

⍝ aplcart/table.tsv:1290 — Bar chart
(⊃⍴¨∘'⎕') 3 1 4 1 5   ⍝ 5 5⍴'⎕⎕⎕  ⎕    ⎕⎕⎕⎕ ⎕    ⎕⎕⎕⎕⎕'

⍝ aplcart/table.tsv:1291 — Join lines with line feed (LF)
Cv←'first'  ⋄ Dv←'Hello, world! 123'  ⋄ Cv(⊣,(•UCS 10),⊢)Dv
•UCS 102 105 114 115 116 10 72 101 108 108 111 44 32 119 111 114 108 100 33 32 49 50 51

⍝ aplcart/table.tsv:1294 — Extract the upper triangular part of the matrix Mm without main diagonal
Mm←3 3⍴⍳9 ⋄  <⌝ ⍨∘⍳∘≢⍛×Mm   ⍝ 3 3⍴0 2 3 0 0 6 0 0 0

⍝ aplcart/table.tsv:1295 — Extract the upper triangular part of the matrix Mm with main diagonal
Mm←3 3⍴⍳9 ⋄  ≤⌝ ⍨∘⍳∘≢⍛×Mm   ⍝ 3 3⍴1 2 3 0 5 6 0 0 9

⍝ aplcart/table.tsv:1296 — Extract the lower triangular part of the matrix Mm with main diagonal
Mm←3 3⍴⍳9 ⋄  ≥⌝ ⍨∘⍳∘≢⍛×Mm   ⍝ 3 3⍴1 0 0 4 5 0 7 8 9

⍝ aplcart/table.tsv:1297 — Extract the lower triangular part of the matrix Mm without main diagonal
Mm←3 3⍴⍳9 ⋄  >⌝ ⍨∘⍳∘≢⍛×Mm   ⍝ 3 3⍴0 0 0 4 0 0 7 8 0

⍝ aplcart/table.tsv:1298 — Is Bm a full upper triangular matrix without diagonal?
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄  <⌝ ⍨∘⍳∘≢⍛≡Bm
0

⍝ aplcart/table.tsv:1299 — Is Bm a full upper triangular matrix with diagonal?
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄  ≤⌝ ⍨∘⍳∘≢⍛≡Bm
0

⍝ aplcart/table.tsv:1300 — Is Bm a full lower triangular matrix with diagonal?
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄  ≥⌝ ⍨∘⍳∘≢⍛≡Bm
0

⍝ aplcart/table.tsv:1301 — Is Bm a full lower triangular matrix without diagonal?
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄  >⌝ ⍨∘⍳∘≢⍛≡Bm
0

⍝ aplcart/table.tsv:1302 — Annual rate to modal rate
M←12 ⋄ N←0.05 ⋄ M(¯1+⊣*∘÷⍨1+⊢)N   ⍝ 0.004074123783648353

⍝ aplcart/table.tsv:1303 — Shifting Y Is positions forward/left/up (if Is is positive, padding on right/bottom) or backward/right/down (if Is is negative, padding on left/top)
Is←¯2 ⋄ Y←1 2 3 4 5 ⋄ Is(×∘×⍨∘≢↑↓)Y   ⍝ 0 0 1 2 3

⍝ aplcart/table.tsv:1304 — Circumference of polygon given as complex points
Nv←0 3 3j4 ⋄ (+/∘|⊢-1∘⌽)Nv   ⍝ 12

⍝ aplcart/table.tsv:1305 — Identity matrix of shape of matrix Nm
Nm←2 3⍴⍳6 ⋄ (⌽⍤⍴⍴1↑⍨1+≢)Nm   ⍝ 3 2⍴1 0 0 1 0 0

⍝ aplcart/table.tsv:1306 — Reshaping vector Yv into a two-column matrix
Yv←1 2 3 4 5 6 ⋄ (⊢⍴⍨2,⍨2÷⍨≢)Yv   ⍝ 3 2⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:1307 — Shift after: Appending X at back/right/bottom of Y, pushing corresponding cells off the front/left/top edge
X←8 9 ⋄ Y←1 2 3 4 ⋄ X(⊢∘-∘≢↑⍪⍨)Y   ⍝ 3 4 8 9

⍝ aplcart/table.tsv:1308 — Boolean gaps of lengths Nv after each one
Nv←2 0 3 ⋄ (1/⍨∘,1,∘⍪-)Nv   ⍝ 1 0 0 1 1 0 0 0

⍝ aplcart/table.tsv:1309 — Is N outside the range [ 1⌷Mv , 2⌷Mv ]
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(↑⍛<∨⊢/⍛<)N   ⍝ 0 0 1 1 1

⍝ aplcart/table.tsv:1310 — Is N outside the range ( 1⌷Mv , 2⌷Mv ]
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(↑⍛≥∨⊢/⍛<)N   ⍝ 1 1 0 0 1

⍝ aplcart/table.tsv:1311 — Is N within the range ( 1⌷Mv , 2⌷Mv ]
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(↑⍛<∧⊢/⍛≥)N   ⍝ 0 0 1 1 0

⍝ aplcart/table.tsv:1312 — Is Y outside the range ( 1⌷Mv , 2⌷Mv )
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(↑⍛≥∨⊢/⍛≤)N   ⍝ 1 1 0 1 1

⍝ aplcart/table.tsv:1313 — Is N within the range [ 1⌷Mv , 2⌷Mv ]
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(↑⍛≤∧⊢/⍛≥)N   ⍝ 0 1 1 1 0

⍝ aplcart/table.tsv:1314 — Is N within the range ( 1⌷Mv , 2⌷Mv )
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(↑⍛<∨⊢/⍛>)N   ⍝ 1 1 1 1 1

⍝ aplcart/table.tsv:1315 — Is N outside the range [ 1⌷Mv , 2⌷Mv )
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(↑⍛>∨⊢/⍛≤)N   ⍝ 1 0 0 1 1

⍝ aplcart/table.tsv:1316 — Is N within the range [ 1⌷Mv , 2⌷Mv )
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(↑⍛≤∧⊢/⍛>)N   ⍝ 0 1 1 0 0

⍝ aplcart/table.tsv:1317 — Widening matrix Ym to be compatible with Xm
Xm←2 4⍴0 ⋄ Ym←2 2⍴⍳4 ⋄ Xm↑∘⌽∘⍴⍛(↑⍤1)Ym   ⍝ 2 4⍴1 2 0 0 3 4 0 0

⍝ aplcart/table.tsv:1318 — last index of ((⍳): Last indices in X of major cells Y
X←1 2 3 2 ⋄ Y←2 4 ⋄ X(1-⊖⍛⍳-∘≢⊣)Y   ⍝ 4 0

⍝ aplcart/table.tsv:1319 — Consecutive integers from Is to Js (Is≤Js)
Is←¯2 ⋄ Js←3 ⋄ Is(⊣,⊣+∘⍳-⍨)Js   ⍝ ¯2 ¯1 0 1 2 3

⍝ aplcart/table.tsv:1320 — Aliquot sum (sum of proper divisors)
Js←12 ⋄ (+/∘∪⊢∨¯1↓⍳)Js   ⍝ 16

⍝ aplcart/table.tsv:1321 — Sort Y ascending according to column Is
Is←2 ⋄ Y←3 2⍴10 3 20 1 30 2 ⋄ Is(⌷⍤1⊂⍤⍋⍛⌷⊢)Y
3 2⍴20 1 30 2 10 3

⍝ aplcart/table.tsv:1322 — More accurately sum a vector of floating point numbers
Nv←1 2 3 ⋄ (+/⍒∘|⊃¨⊂)Nv   ⍝ 6

⍝ aplcart/table.tsv:1323 — Sort Y descending according to column Is
Is←2 ⋄ Y←3 2⍴10 3 20 1 30 2 ⋄ Is(⌷⍤1⊂⍤⍒⍛⌷⊢)Y
3 2⍴10 3 30 2 20 1

⍝ aplcart/table.tsv:1324 — Suffixes of a vector
Yv←1 2 3 4 ⋄ (⌽∘,¨,\∘⌽)Yv   ⍝ (1⍴4 ⋄ 3 4 ⋄ 2 3 4 ⋄ 1 2 3 4)

⍝ aplcart/table.tsv:1325 — Changing lengths Jv of subvectors to ending indicators
Jv←2 3 1 ⋄ (+\∊⍨∘⍳+/)Jv   ⍝ 0 1 0 0 1 1

⍝ aplcart/table.tsv:1326 — First group of ones
B←1 1 0 1 0 1 0 ⋄ (∧⍀∨⍀⍛=)⍛∧B   ⍝ 1 1 0 0 0 0 0

⍝ aplcart/table.tsv:1327 — Progressive index of (⍳) cyclic uniques (∪) without replacement
(i ← (0~⍨∘,∘⍉⊢⌸) 'abracadabra' ⋄ 'abracadabra'[i])
(1 2 3 5 7 4 9 10 6 8 11 ⋄ 'abrcdabraaa')

⍝ aplcart/table.tsv:1328 — Length of vector Nv
Nv←3 4 ⋄ (2*∘÷⍨+.×⍨)Nv   ⍝ 5

⍝ aplcart/table.tsv:1329 — Row averages of non-zero elements (0 if none)
N←2 3⍴0 2 4 0 0 0 ⋄ (+/÷1⌈+.≠∘0)N   ⍝ 3 0

⍝ aplcart/table.tsv:1330 — Test relations (¯2…2) of elements of N to ranges M (2=¯1↑⍴M)
M←2 2⍴1 3 2 4 ⋄ N←0 1 2 3 4 5 ⋄ M(+/∘× -⌝ ⍨)N
6 2⍴¯2 ¯2 ¯1 ¯2 0 ¯1 1 0 2 1 2 2

⍝ aplcart/table.tsv:1331 — Boolean matrix with Iv[i] leading zeroes on row i
Iv←0 2 4 ⋄ (⊢ <⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴1 1 1 1 0 0 1 1 0 0 0 0

⍝ aplcart/table.tsv:1332 — Boolean matrix with Iv[i] leading ones on row i
Iv←0 2 4 ⋄ (⊢ ≥⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴0 0 0 0 1 1 0 0 1 1 1 1

⍝ aplcart/table.tsv:1333 — Test relations (¯2…2) of major cells Y to range 1⌷X , 2⌷X
X←1 3 ⋄ Y←0 1 2 3 4 ⋄ X(⌊¯3+0.6×⍳+3×⍸)Y   ⍝ ¯2 ¯1 0 1 2

⍝ aplcart/table.tsv:1334 — Number of days in February of year J (YYYY)
J←1900 2000 2024 2025 ⋄ (28+0≠.=400 100 4 |⌝ ⊢)J
28 29 29 28

⍝ aplcart/table.tsv:1335 — Stereo pair (Eye separation Ms)
Ms←0.5 ⋄ N←1j2 3j4 ⋄ Ms(⊢∘⊂+¯0.5 0.5×⊣)N
(0.75j2 2.75j4 ⋄ 1.25j2 3.25j4)

⍝ aplcart/table.tsv:1336 — Euclidean distance between two points in N-space
0 0 0 0 (2*∘÷⍨1⊥2*⍨-) 0 3 4 0   ⍝ 5

⍝ aplcart/table.tsv:1337 — Encode a vector of positive integers as equal-width fields of digits in an integer
(10⊥∘,∘⍉10∘⊥⍣¯1) 31 1 27 0   ⍝ 31012700

⍝ aplcart/table.tsv:1340 — Truncated division
¯10 10 ¯10 10 ((××∘⌊|)÷) ¯3 ¯3 3 3   ⍝ 3 ¯3 ¯3 3

⍝ aplcart/table.tsv:1341 — First word in Dv
Dv← 'hello world'  ⋄ (⊢↑⍨¯1+⍳∘' ')Dv   ⍝ 'hello'

⍝ aplcart/table.tsv:1342 — Students grades given score; the Game of Life recipe constructs its table of surviving neighbourhoods
J←55 65 75 85 95 ⋄ (0 60 70 80 90∘⍸⊂⍛⌷'FDCBA'⍨)J
'FDCBA'

⍝ aplcart/table.tsv:1343 — Remove multiple blanks
Dv← '  a   b  '  ⋄ ('  '∘⍷~⍛/⊢)Dv   ⍝ ' a b '

⍝ aplcart/table.tsv:1344 — Boolean one at first occurrence of X in Y; First-true masks use cumulative counts under basedpl left scan
'fab' ({⍵∧1=+\⍵}@(=⍨)⍷) 3 5⍴'abcdef'   ⍝ 3 5⍴0 0 0 0 0 1 0 0 0 0 0 0 0 0 0

⍝ aplcart/table.tsv:1345 — Locate leading blank columns
Dm←2 4⍴'  ab  cd'  ⋄ (∧\' '∧.=⊢)Dm   ⍝ 1 1 0 0

⍝ aplcart/table.tsv:1346 — Locate leading blank rows
Dm←3 3⍴'   abc   '  ⋄ (∧\∧.=∘' ')Dm   ⍝ 1 0 0

⍝ aplcart/table.tsv:1348 — Histogram
N←0 2 4 ⋄ (⊃'⎕'⍴¨⍨⌊)N   ⍝ 3 4⍴'    ⎕⎕  ⎕⎕⎕⎕'

⍝ aplcart/table.tsv:1349 — Translate characters to digits (bases 2 through 36); Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
(¯1+(•D,•A)⍳⊢)'09AZ'   ⍝ 0 9 10 35

⍝ aplcart/table.tsv:1351 — Matrix to segmented string using Cs or linefeed as delimiter (includes initial delimiter)
Cs←'|'  ⋄ Dm←2 4⍴'ABCDabcd'  ⋄ Cs{⍺←•UCS 10 ⋄ ,⍺,⍵}Dm
'|ABCD|abcd'

⍝ aplcart/table.tsv:1352 — Replace all blanks with dashes
D←2 4⍴'  ab c d'  ⋄ '-'@(=∘' ')D   ⍝ 2 4⍴'--ab-c-d'

⍝ aplcart/table.tsv:1353 — Moving all blanks to end of text; dfns display import/wrappers omitted to test underlying arrays
((~,∩)∘' ') 'Here be spaces'   ⍝ 'Herebespaces  '

⍝ aplcart/table.tsv:1354 — General comparison according to Total Array Order (¯1:X precedes Y, 0:X≡Y, 1:X succeeds Y)
X←1 2 ⋄ Y←1 3 ⋄ X{↑(⍋-⍒)⍺⍵}Y   ⍝ ¯1

⍝ aplcart/table.tsv:1355 — Formatting with zero values replaced with blanks
N←2 3⍴0 1 2 3 0 4 ⋄ (⍕' '@(0∘=))N   ⍝ 2 5⍴'  1 23   4'

⍝ aplcart/table.tsv:1356 — Rounding to nearest even number (favouring away from 0)
N←¯3 ¯2.5 2.5 3 ⋄ (××∘⌊|+1≤2||)N   ⍝ ¯4 ¯2 2 4

⍝ aplcart/table.tsv:1357 — Rounding to nearest odd number (favouring away from 0)
(××∘⌊|+1>2||) 1.6 2.7 3.1   ⍝ 1 3 3

⍝ aplcart/table.tsv:1358 — Rounding to nearest odd number (favouring towards 0)
N←¯4 ¯2 0 2 4 ⋄ (××¯1+2×∘⌈2÷⍨|)N   ⍝ ¯3 ¯1 0 1 3

⍝ aplcart/table.tsv:1359 — Stochastic rounding to integer; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
N←¯2 ¯1.25 0 0.75 3 ⋄ r←(⌊+1∘|>∘?0∘×)N ⋄ ((⍴N)≡⍴r)∧∧/(r=⌊N)∨r=⌈N
1

⍝ aplcart/table.tsv:1360 — Aspect ratio of a triangle given its side lengths
Nv←3 4 5 ⋄ (×/⊢÷+/-2×⊢)Nv   ⍝ 1.25

⍝ aplcart/table.tsv:1361 — Tiling a matrix Ym over a matrix of shape Iv
Iv←4 6 ⋄ Ym←2 3⍴⍳6 ⋄ Iv(⊣⍴⊢⌿⍤⊣⍴⍤1⊢)Ym
4 6⍴1 2 3 1 2 3 4 5 6 4 5 6 1 2 3 1 2 3 4 5 6 4 5 6

⍝ aplcart/table.tsv:1362 — Reshape as in J (outer shape Iv with inner shape of major cells of Y)
Iv←2 2 ⋄ Y←2 3⍴⍳6 ⋄ Iv(⊢⍴⍨⊣,1↓⊢∘⍴)Y   ⍝ 2 2 3⍴1 2 3 4 5 6 1 2 3 4 5 6

⍝ aplcart/table.tsv:1363 — Reshape Yv to Is-row matrix (filled row-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⊢⍴⍨⊣,÷⍨∘≢)Yv   ⍝ 2 3⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:1364 — Reshape Yv to Is-column matrix (filled row-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⊢⍴⍨÷⍨∘≢,⊣)Yv   ⍝ 3 2⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:1365 — Remove consecutive duplicate Xs's from vector Yv
Xs←0 ⋄ Yv←0 0 1 1 0 0 2 0 ⋄ Xs{⍵/⍨∨/2↕1,⍺≠⍵}Yv
0 1 1 0 2 0

⍝ aplcart/table.tsv:1366 — Arithmetic mean value of all elements
N←2 3⍴⍳6 ⋄ (+/∘,÷×/⍤⍴)N   ⍝ 3.5

⍝ aplcart/table.tsv:1367 — Appending X as additional major cell of Y, adjusting dimensions of both as necessary
X←1 2 3 4 ⋄ Y←2 3⍴⍳6 ⋄ X(⊃⊂⍤¯1⍤⊢,⊂⍤⊣)Y   ⍝ 3 4⍴1 2 3 0 4 5 6 0 1 2 3 4

⍝ aplcart/table.tsv:1368 — Js spokes of unit wheel
Js←4 ⋄ (*∘π0J2×⊢÷⍨1+⍳)Js
¯1 0j¯1 1 0j1

⍝ aplcart/table.tsv:1369 — Last indices in X of major cells Y, 0 if not found
X←1 2 3 2 ⋄ Y←2 4 ⋄ X(⊖⍛⍳-⍨1+∘≢⊣)Y   ⍝ 4 0

⍝ aplcart/table.tsv:1370 — Regular unit polygon of Js edges
Js←4 ⋄ (*∘π0J2×⊢÷⍨0,⍳)Js
1 0j1 ¯1 0j¯1 1

⍝ aplcart/table.tsv:1371 — Attach column numbers to a matrix; Use shape rather than tally to obtain the column count; upstream ≢ fails on rectangular matrices; Concrete 2×3 matrix, independently checked in Dyalog
Ym←2 3⍴⍳6 ⋄ (⍳∘↑∘⌽∘⍴⍪⊢)Ym   ⍝ 3 3⍴1 2 3 1 2 3 4 5 6

⍝ aplcart/table.tsv:1372 — Conversion of set of indices Jv to a mask
Jv←2 4 5 ⋄ (⊢∊⍨∘⍳∘⊃⌈/)Jv   ⍝ 0 1 0 1 1

⍝ aplcart/table.tsv:1373 — Sorting indices Iv according to data Y
Iv←3 1 4 ⋄ Y←20 10 30 15 ⋄ Iv(⊂⍛⌷⊂⍤⍋⍛⌷⊣)Y
4 1 3

⍝ aplcart/table.tsv:1374 — Changing lengths Jv of subvectors to starting indicators
Jv←2 3 1 ⋄ (¯1⌽+\∊⍨∘⍳+/)Jv   ⍝ 1 0 1 0 0 1

⍝ aplcart/table.tsv:1375 — Mask for selecting between first and last 1 on each row
B←2 5⍴0 1 0 1 0 0 0 1 0 0 ⋄ (∨\∧∘⌽∨\∘⌽)B
2 5⍴0 1 1 1 0 0 0 1 0 0

⍝ aplcart/table.tsv:1376 — Rounding, to nearest even integer for 0.5 = 1||N
N←¯2.5 ¯1.5 0.5 1.5 2.5 ⋄ (⌊⊢+2÷⍨0.5≠2|⊢)N
¯2 ¯2 0 2 2

⍝ aplcart/table.tsv:1377 — Matrix with Iv[i] trailing zeroes on row i
Iv←0 2 4 ⋄ (⌽⊢ <⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴1 1 1 1 1 1 0 0 0 0 0 0

⍝ aplcart/table.tsv:1378 — Matrix with Iv[i] trailing ones on row i
Iv←0 2 4 ⋄ (⌽⊢ ≥⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴0 0 0 0 0 0 1 1 1 1 1 1

⍝ aplcart/table.tsv:1379 — Quadratic mean
N←4 9 16 ⋄ (2*∘÷⍨1⊥×⍨÷≢)N   ⍝ 10.84742673018199

⍝ aplcart/table.tsv:1380 — Present value of cash flows Nv at interval Ms
Ms←0.1 ⋄ Nv←100 200 300 ⋄ Ms(⊢∘⌽⊥⍨∘÷1+⊣)Nv
529.7520661157025

⍝ aplcart/table.tsv:1381 — Join digits of strictly positive integers into a single integer
(10⊥∘∊10∘⊥⍣¯1¨) 31 1 27   ⍝ 31127

⍝ aplcart/table.tsv:1382 — Is Js an abundant number?
Js←12 ⋄ (+⍨<1⊥∘∪⍳⍛∨)Js   ⍝ 1

⍝ aplcart/table.tsv:1383 — Is Js a perfect number?
Js←6 ⋄ (+⍨=1⊥∘∪⍳⍛∨)Js   ⍝ 1

⍝ aplcart/table.tsv:1384 — Is Js a deficient number?
Js←8 ⋄ (+⍨>1⊥∘∪⍳⍛∨)Js   ⍝ 1

⍝ aplcart/table.tsv:1385 — Test relations (¯2…2) of elements of N to range 1⌷Mv , 2⌷Mv
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(¯2+1⊥ <⌝ ⍪ ≤⌝ )N
¯2 ¯1 0 1 2

⍝ aplcart/table.tsv:1386 — All binary representations with Js bits (truth table with Js variables, matrix for choosing all subsets)
2∘(⍉⍴⍨⊤¯1+∘⍳*) 3
8 3⍴0 0 0 0 0 1 0 1 0 0 1 1 1 0 0 1 0 1 1 1 0 1 1 1

⍝ aplcart/table.tsv:1387 — All tuples of corresponding elements of ⍳¨Jv (for large Jv even above length 15)
Jv←2 3 2 ⋄ (⍉1+⊢⊤¯1+∘⍳×/)Jv
12 3⍴1 1 1 1 1 2 1 2 1 1 2 2 1 3 1 1 3 2 2 1 1 2 1 2 2 2 1 2 2 2 2 3 1 2 3 2

⍝ aplcart/table.tsv:1388 — Vertical column headings for character matrix of width Js; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
(•D⌷⍨∘⊂1+10⊥⍣¯1⍳)12   ⍝ 2 12⍴'000000000111123456789012'

⍝ aplcart/table.tsv:1389 — Detect case of characters (1:uppercase, ¯1:lowercase, 0:neither)
D←'Abc 19 Σς!'  ⋄ (¯1∘•C⍛≠-1∘•C⍛≠)D   ⍝ 1 ¯1 ¯1 0 0 0 0 1 ¯1 0

⍝ aplcart/table.tsv:1390 — Are characters of D titlecase?
D←'Abc 19 Σς!'  ⋄ (¯1∘•C⍛≠∧1∘•C⍛≠)D   ⍝ 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/table.tsv:1391 — Mean squared error
1 2 3 ((+⌿÷≢)2*⍨-) 0.9 2.1 3.1   ⍝ 0.01000000000000001

⍝ aplcart/table.tsv:1392 — Expansion mask (left argument for ⍀) for fields of length Jv to uniform field of length |Is
Is←4 ⋄ Jv←2 3 1 ⋄ Is(,(⊣↑1⍴⍨⊢)⍤0)Jv   ⍝ 1 1 0 0 1 1 1 0 1 0 0 0

⍝ aplcart/table.tsv:1393 — Rightmost neighbouring elements (padding at edge)
Y←2 3⍴⍳6 ⋄ ((1↓⊢,1↑0∘⍴)⍤1)Y   ⍝ 2 3⍴2 3 0 5 6 0

⍝ aplcart/table.tsv:1395 — Self-classify: table of unique vs all major cells of Y
Y←1 2 1 3 ⋄ ((∪ =⌝ ⊢)⍳⍨)Y   ⍝ 3 4⍴1 0 1 0 0 1 0 0 0 0 0 1

⍝ aplcart/table.tsv:1396 — Bit-wise OR for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(∨/2⊥⍣¯1,⍤0))J   ⍝ 6 11 20

⍝ aplcart/table.tsv:1397 — Bit-wise AND for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(∧/2⊥⍣¯1,⍤0))J   ⍝ 0 1 0

⍝ aplcart/table.tsv:1398 — Bit-wise converse nonimplication for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(</2⊥⍣¯1,⍤0))J   ⍝ 4 8 16

⍝ aplcart/table.tsv:1399 — Bit-wise implication for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(≤/2⊥⍣¯1,⍤0))J   ⍝ 29 29 27

⍝ aplcart/table.tsv:1400 — Bit-wise XNOR for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(=/2⊥⍣¯1,⍤0))J   ⍝ 25 21 11

⍝ aplcart/table.tsv:1401 — Bit-wise converse implication for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(≥/2⊥⍣¯1,⍤0))J   ⍝ 27 23 15

⍝ aplcart/table.tsv:1402 — Bit-wise nonimplication for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(>/2⊥⍣¯1,⍤0))J   ⍝ 2 2 4

⍝ aplcart/table.tsv:1403 — Bit-wise XOR for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(≠/2⊥⍣¯1,⍤0))J   ⍝ 6 10 20

⍝ aplcart/table.tsv:1404 — Bit-wise NOR for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(⍱/2⊥⍣¯1,⍤0))J   ⍝ 25 20 11

⍝ aplcart/table.tsv:1405 — Bit-wise NAND for positive integers
I←2 3 4 ⋄ J←4 9 16 ⋄ I(2⊥(⍲/2⊥⍣¯1,⍤0))J   ⍝ 31 30 31

⍝ aplcart/table.tsv:1406 — Date (⎕TS format) to YYYY-MM-DD
Jv←2026 9 18 ⋄ ('-'@5 8∘⍕1000⊥3∘↑)Jv   ⍝ '2026-09-18'

⍝ aplcart/table.tsv:1408 — Translate digits to characters (bases 2 through 36); Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
((•D,•A)⌷⍨1+⊂)0 9 10 35   ⍝ '09AZ'

⍝ aplcart/table.tsv:1409 — Conditional in text
Bs←0 ⋄ ('correct',⍨'in'/⍨~)Bs   ⍝ 'incorrect'

⍝ aplcart/table.tsv:1410 — Replace backslashes with slashes
'/'@('\'∘=) 'path\to\file'   ⍝ 'path/to/file'

⍝ aplcart/table.tsv:1411 — Is Nm an upper triangular matrix without diagonal?
Nm←3 3⍴0 1 2 0 0 3 0 0 0 ⋄  <⌝ ⍨∘⍳∘≢⍛×⍛≡Nm
1

⍝ aplcart/table.tsv:1412 — Is Nm an upper triangular matrix with diagonal?
Nm←3 3⍴1 2 3 0 4 5 0 0 6 ⋄  ≤⌝ ⍨∘⍳∘≢⍛×⍛≡Nm
1

⍝ aplcart/table.tsv:1413 — Is Nm a lower triangular matrix with diagonal?
Nm←3 3⍴1 0 0 2 3 0 4 5 6 ⋄  ≥⌝ ⍨∘⍳∘≢⍛×⍛≡Nm
1

⍝ aplcart/table.tsv:1414 — Is Nm a lower triangular matrix without diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (⊢≡⊢× >⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 0 0 1 1 0 1 1 1 ⋄ (⊢≡⊢×.>⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 0 0 1 0 0 1 1 0 ⋄ (⊢≡⊢× >⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 0 0 0 0 1 0 0 ⋄ (⊢≡⊢× >⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 0 0 1 1 0 1 1 1) 0 (3 3⍴0 0 0 1 0 0 1 1 0) 1 (3 3⍴0 0 0 0 0 0 1 0 0) 1

⍝ aplcart/table.tsv:1415 — Rounding to nearest even number (favouring towards 0)
N←¯3 ¯2.5 2.5 3 ⋄ (××2×∘⌈2÷⍨1-⍨|)N   ⍝ ¯2 ¯2 2 2

⍝ aplcart/table.tsv:1416 — Take of at most Iv elements from Y
Iv←5 1 ⋄ Y←2 3⍴⍳6 ⋄ Iv(⊢↑⍨≢⍤⊣↑⌊∘⍴)Y   ⍝ 2 1⍴1 4

⍝ aplcart/table.tsv:1417 — Ending points of groups of equal elements
Yv←1 1 2 3 3 ⋄ (≠/2↕Yv),1   ⍝ 0 1 1 0 1

⍝ aplcart/table.tsv:1418 — Derivative of polynomial with descending coefficients Nv
Nv←2 3 4 5 ⋄ ¯1∘(↓×∘⌽∘⍳+∘≢)Nv   ⍝ 6 6 4

⍝ aplcart/table.tsv:1419 — Major cells of Y except those enumerated in I
I←1 3 ⋄ Y←4 2⍴⍳8 ⋄ I(⊂⍤~⍨∘⍳∘≢⌷⊢)Y   ⍝ 2 2⍴3 4 7 8

⍝ aplcart/table.tsv:1420 — Lengths of subvectors of Yv having equal elements
Yv←1 1 2 3 3 ⋄ ((-/∘⌽∘(2∘↕))∘⍸1,1,⍨(≠/∘(2∘↕)))Yv
2 1 2

⍝ aplcart/table.tsv:1421 — Segment lengths (excluding delimiters) in delimited string Dv where the first character is the delimiter
Dv← '/ab//c/'  ⋄ ¯1+-/⌽2↕⍸(Dv=↑Dv),1   ⍝ 2 0 1 0

⍝ aplcart/table.tsv:1422 — Reshape Yv to Is-column matrix (filled column-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⍉⊢⍴⍨⊣,÷⍨∘≢)Yv   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/table.tsv:1423 — Reshape Yv to Is-row matrix (filled column-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⍉⊢⍴⍨≢⍛÷⍨,⊣)Yv   ⍝ 2 3⍴1 3 5 2 4 6

⍝ aplcart/table.tsv:1424 — Distribute major cells of Y into Is (default Is:≢Y) groups as evenly as possible
UnZip ← |∘⍳∘≢⊢∘⊂⌸⊢ ⋄ var ← 'abcdef' ⋄ (1 UnZip var ⋄ 2 UnZip var ⋄ 3 UnZip var ⋄ 4 UnZip var ⋄ 5 UnZip var ⋄ 6 UnZip var ⋄ 7 UnZip var ⋄ 8 UnZip var ⋄ UnZip var)
(1⍴⊂('abcdef') ⋄ ('ace' ⋄ 'bdf') ⋄ ('ad' ⋄ 'be' ⋄ 'cf') ⋄ ('ae' ⋄ 'bf' ⋄ 1⍴'c' ⋄ 1⍴'d') ⋄ ('af' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e') ⋄ (1⍴'a' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e' ⋄ 1⍴'f') ⋄ (1⍴'a' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e' ⋄ 1⍴'f') ⋄ (1⍴'a' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e' ⋄ 1⍴'f') ⋄ (1⍴'a' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e' ⋄ 1⍴'f'))

⍝ aplcart/table.tsv:1425 — Divisibility table
Jv←2 3 6 ⋄ (0=⊢ |⌝ ⍨∘⍳⌈/)Jv   ⍝ 6 3⍴1 1 1 1 0 1 0 1 1 0 0 0 0 0 0 0 0 1

⍝ aplcart/table.tsv:1426 — Rounding to nearest hundredth (favouring up)
N←¯1.234 0.125 2.678 ⋄ 0.01∘(⊣×∘⌊0.5+÷⍨)N
¯1.23 0.13 2.68

⍝ aplcart/table.tsv:1427 — Rounding currencies to nearest 5 subunits
N←1.23 2.47 3.51 ⋄ 0.05∘(⊣×∘⌊0.5+÷⍨)N   ⍝ 1.25 2.45 3.5

⍝ aplcart/table.tsv:1428 — Is-norm
2 (⊣*∘÷⍨1⊥*⍨∘|) 3 4   ⍝ 5

⍝ aplcart/table.tsv:1429 — Is Js an almost perfect number?
Js←8 ⋄ (+⍨=1+1⊥∘∪⍳⍛∨)Js   ⍝ 1

⍝ aplcart/table.tsv:1430 — Is Js a quasiperfect number? (none are known)
Js←6 ⋄ (+⍨=¯1+1⊥∘∪⍳⍛∨)Js   ⍝ 0

⍝ aplcart/table.tsv:1431 — Coefficients of least squares exponential fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←2 6 18 54 ⋄ Mv(*@1⊢∘⍟⌹1,∘⍪⊣)Nv
2 1.09861228866811

⍝ aplcart/table.tsv:1433 — Enlist (∊Y) but keep leaf simple arrays intact
{,/,¨⊆¨⍵}⍣≡ ('aaa' 'bbb'⋄ ('ccc' 'ccc' 'ccc') 'ddd'⋄ ⊂'eee')
('aaa' ⋄ 'bbb' ⋄ 'ccc' ⋄ 'ccc' ⋄ 'ccc' ⋄ 'ddd' ⋄ 'eee')

⍝ aplcart/table.tsv:1434 — Leftmost neighbouring elements (padding at edge)
Y←2 3⍴⍳6 ⋄ ((¯1↓⊢,⍨1↑0∘⍴)⍤1)Y   ⍝ 2 3⍴0 1 2 0 4 5

⍝ aplcart/table.tsv:1435 — Choose the number closer to zero (the positive one if tied)
Is←¯2 ⋄ Js←2 ⋄ Is(⌈(↑>⍥|⌽,)⌊)Js   ⍝ 2

⍝ aplcart/table.tsv:1436 — Indices of last non-blanks in rows
D← 3 4⍴'ab    c d   '  ⋄ (↑⍤⌽⍤⍸⍤1≠∘' ')D
2 3 1

⍝ aplcart/table.tsv:1437 — Juxtapositioning planes of rank 3 array Y
Y←2 3 4⍴⍳24 ⋄ ((×⌿2 2⍴1,⍴)⍴2 1 3∘⍉)Y
3 8⍴1 2 3 4 13 14 15 16 5 6 7 8 17 18 19 20 9 10 11 12 21 22 23 24

⍝ aplcart/table.tsv:1438 — Primes until Js
Js←20 ⋄ ((⊢~ ×⌝ ⍨)1↓⍳)Js   ⍝ 2 3 5 7 11 13 17 19

⍝ aplcart/table.tsv:1439 — Right justify matrix Dm
Dm← 2 4⍴'ab  c   '  ⋄ (⊢⌽⍨1-1⊥⍨=∘' ')Dm   ⍝ 2 4⍴'  ab   c'

⍝ aplcart/table.tsv:1443 — Remove non-alphanumeric ASCII characters
Dv←'Hello, world! 123'  ⋄ (∩∘(•D,•A,•C•A))Dv
'Helloworld123'

⍝ aplcart/table.tsv:1444 — Remove punctuation
Dv← 'Hello, world! Why?'  ⋄ (~∘'.,:;?!')Dv
'Hello world Why'

⍝ aplcart/table.tsv:1446 — Sort A according to Ms (1: ascending, 0: unordered, ¯1: descending)
Ms←¯1 ⋄ A←3 1 2 ⋄ Ms{⍵⌷⍨⊂⍋⍋⍺×⍋⍵}A   ⍝ 1 3 2

⍝ aplcart/table.tsv:1447 — Generate Roman numeral (purely additive, no refinements)
Js←1984 ⋄ ('MDCLXVI'/⍨(0,6⍴2 5)⊤⊢)Js   ⍝ 'MDCCCCLXXXIIII'

⍝ aplcart/table.tsv:1448 — Replace leading zeros with blanks
D← ' 00120'  ⋄ ' '@{2=⌈\' 0'⍳⍵}D   ⍝ '   120'

⍝ aplcart/table.tsv:1449 — Poisson distribution of states N with average M
M←2 ⋄ N←0 1 2 3 ⋄ M(*⍤-⍤⊣×*÷!⍤⊢)N
0.1353352832366127 0.2706705664732254 0.2706705664732254 0.1804470443154836

⍝ aplcart/table.tsv:1450 — Merging Y[1] Y[2] Y[3], … under control of Iv (1:cell from Y[1], 2:cell from Y[2], …)
Iv←1 2 1 2 ⋄ Y←(10 11 12 13⋄ 20 21 22 23) ⋄ Iv(⍳⍤≢⍤⊣⊃¨⌷¨∘⊂)Y
10 21 12 23

⍝ aplcart/table.tsv:1451 — Lengths of groups of ones in Bv
Bv←0 1 1 0 1 0 1 1 1 0 ⋄ 0~⍨¯1+-/⌽2↕⍸1,(~Bv),1
2 1 3

⍝ aplcart/table.tsv:1452 — Merge X and Y alternately
X←1 2 3 ⋄ Y←4 5 ⋄ X(,⍥⍳⍥≢⊂⍤⍋⍛⌷⍪)Y   ⍝ 1 4 2 5 3

⍝ aplcart/table.tsv:1453 — Assign ascending ranking based on scores Nv (ties all get average ranking of used slots)
(2÷⍨⍋∘⍋+⍒∘⍋∘⌽) 3 3 3 5 8 8 21   ⍝ 2 2 2 4 5.5 5.5 7

⍝ aplcart/table.tsv:1454 — Assign descending ranking based on scores Nv (ties all get average ranking of used slots)
(2÷⍨⍋∘⍒+⍒∘⍒∘⌽) 3 3 3 5 8 8 21   ⍝ 6 6 6 4 2.5 2.5 1

⍝ aplcart/table.tsv:1455 — Split Yv at occurrences of Xv (removes separators and keeps empty segments)
Xv←0 9 ⋄ Yv←0 9 1 2 0 9 0 9 3 0 9 ⋄ Xv(≢⍤⊣↓¨,⊂⍨⊣⍷,)Yv
(⍬ ⋄ 1 2 ⋄ ⍬ ⋄ 1⍴3 ⋄ ⍬)

⍝ aplcart/table.tsv:1456 — Annuity coefficient: I periods at interest N
I←12 ⋄ N←0.05 ⋄ I(⊢÷∘⍉1+⊣ ×⌝ 1+⊢)N   ⍝ 0.003676470588235294

⍝ aplcart/table.tsv:1457 — Changing connection matrix Jm (¯1 to 1) to a node matrix
Jm←3 3⍴¯1 1 0 0 ¯1 1 1 0 ¯1 ⋄ (⍳∘≢+.×⍨1 ¯1 =⌝ ⍉)Jm
2 3⍴3 1 2 1 2 3

⍝ aplcart/table.tsv:1458 — Euclidean distance table for points in N-space (one point per row); nested displayed values saved once in evaluation order and returned together
(r1←4 2⍴0 0 0 1 1 0 1 1 ⋄ (2*∘÷⍨1⊥2*⍨-)⍤1⍤1 99⍨ r1)
(4 2⍴0 0 0 1 1 0 1 1 ⋄ 4 4⍴0 1 1 1.414213562373095 1 0 1.414213562373095 1 1 1.414213562373095 0 1 1.414213562373095 1 1 0)

⍝ aplcart/table.tsv:1459 — Matrix of all indices of Y (↑,⍳⍴Y for large Y)
Y←2 3⍴0 ⋄ (⍉1+⍴⊤¯1+∘⍳×/∘⍴)Y   ⍝ 6 2⍴1 1 1 2 1 3 2 1 2 2 2 3

⍝ aplcart/table.tsv:1460 — Probabilistic XNOR
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M((1+×-⊣)×1+×-⊢)N
0 0.546875 0

⍝ aplcart/table.tsv:1461 — Extract text (including quotes) in expression
Dv← 'a+''hello''+b'  ⋄ ≠\⍛∨⍤=∘''''⍛/Dv   ⍝ '''hello'''

⍝ aplcart/table.tsv:1462 — Merge vectors in Yv alternately (↓⍉↑ that removes trailing elements from longer vectors)
(↓(⌊/≢¨)↑⍉∘⊃) 'Now' 'is' 'the' 'time'   ⍝ ('Nitt' ⋄ 'oshi')

⍝ aplcart/table.tsv:1463 — Rectangular scale of complex N by complex factor Ms
Ms←2j3 ⋄ N←1j2 ¯3j4 ⋄ Ms((9 11○⊣)+.×9 11 ○⌝ ⊢)N
8 6

⍝ aplcart/table.tsv:1465 — Number of digit positions in integers in J
J←0 1 9 10 ¯99 100 ⋄ 0∘(1+<+∘⌊10⍟∘|⊢+=)J
1 2 2 3 2 4

⍝ aplcart/table.tsv:1466 — Increasing the leading dimension of Y to multiple of Is
Is←4 ⋄ Y←1 2 3 4 5 ⋄ Is(⊢↑⍨⊢∘≢+|∘-∘≢)Y   ⍝ 1 2 3 4 5 0 0 0

⍝ aplcart/table.tsv:1467 — Increasing the dimensions of Y to multiples of Iv
Iv←3 4 ⋄ Y←2 3⍴⍳6 ⋄ Iv(⊢↑⍨⊢∘⍴+|∘-∘⍴)Y   ⍝ 3 4⍴1 2 3 0 4 5 6 0 0 0 0 0

⍝ aplcart/table.tsv:1468 — Centering text line Dv into a field of width Is
Is←9 ⋄ Dv← 'abc'  ⋄ Is(⊢↑⍨∘-∘⌊+∘≢÷2⍨)Dv   ⍝ '   abc'

⍝ aplcart/table.tsv:1469 — Expansion vector (left argument for \ or ⍀) to insert a new element after each one in Bv
Bv←1 0 1 0 ⋄ (⍪⍛,∘1⊢⍤/⍥,⍪,~)Bv   ⍝ 1 0 1 1 0 1

⍝ aplcart/table.tsv:1470 — Expansion vector for Y with zeros after indices Iv
Iv←1 3 ⋄ Y←1 2 3 4 ⋄ Iv(,⍨∘⍳∘≢⍋⍛≤∘≢⊢)Y   ⍝ 1 0 1 1 0 1

⍝ aplcart/table.tsv:1471 — Normalisation by Is-norm
2 (⊢÷⊣*∘÷⍨1⊥*⍨∘|) 3 4   ⍝ 0.6 0.8

⍝ aplcart/table.tsv:1472 — Generalised mean
Ms←2 ⋄ N←1 2 3 ⋄ Ms(⊣*∘÷⍨1⊥*⍨÷⊢∘≢)N   ⍝ 2.160246899469287

⍝ aplcart/table.tsv:1473 — Length of subvectors indicated by Bv (Fast ≢¨⊂⍨Bv)
(≢(⊢-⍨1↓⊢,1+⊣)⍸) 1 0 0 0 1 1 0 0   ⍝ 4 1 3

⍝ aplcart/table.tsv:1474 — Ascending shortlex sort
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⊂⊃¨⍨∘⍋(≢,⊂)¨)Yv
'a' 'b' ('aa') ('ab')

⍝ aplcart/table.tsv:1475 — Descending shortlex sort
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⊂⊃¨⍨∘⍒(≢,⊂)¨)Yv
('ab') ('aa') 'b' 'a'

⍝ aplcart/table.tsv:1476 — Expansion vector (left argument for \ or ⍀) to insert Jv[i] elements before i'th element
Jv←2 0 1 ⋄ ((+\∊⍨∘⍳+/)1∘+)Jv   ⍝ 0 0 1 1 0 1

⍝ aplcart/table.tsv:1477 — Sort each column in ascending order
Y←2 3 2⍴3 2 1 4 2 1 5 6 4 5 6 4 ⋄ ((⍉⊂⍤⍋⍛⌷⍤1∘⍉)⍤2)Y
2 3 2⍴1 1 2 2 3 4 4 4 5 5 6 6

⍝ aplcart/table.tsv:1478 — Kronecker product
A ← 2 3⍴1 ¯4 7,¯2 3 3 ⋄ B ← 4 4⍴8 ¯9 ¯6 5,1 ¯3 ¯4 7,2 8 ¯8 ¯3,1 2 ¯5 ¯1 ⋄ A {,⍤2,[⍳2]1 3 2 4⍉⍺ ×⌝ ⍵} B
8 12⍴8 ¯9 ¯6 5 ¯32 36 24 ¯20 56 ¯63 ¯42 35 1 ¯3 ¯4 7 ¯4 12 16 ¯28 7 ¯21 ¯28 49 2 8 ¯8 ¯3 ¯8 ¯32 32 12 14 56 ¯56 ¯21 1 2 ¯5 ¯1 ¯4 ¯8 20 4 7 14 ¯35 ¯7 ¯16 18 12 ¯10 24 ¯27 ¯18 15 24 ¯27 ¯18 15 ¯2 6 8 ¯14 3 ¯9 ¯12 21 3 ¯9 ¯12 21 ¯4 ¯16 16 6 6 24 ¯24 ¯9 6 24 ¯24 ¯9 ¯2 ¯4 10 2 3 6 ¯15 ¯3 3 6 ¯15 ¯3

⍝ aplcart/table.tsv:1479 — Predicted values of least squares linear fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←1 3 5 7 ⋄ Mv(⊢(⊢+.×⌹)1,∘⍪⊣)Nv
1 3 5 7

⍝ aplcart/table.tsv:1480 — Conway's Game of Life: next generation
Bm←5 5⍴0 0 0 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0 ⋄ ({≢⍸⍵}⌺3 3∊¨3+0,¨⊢)Bm
5 5⍴0 0 0 0 0 0 0 0 0 0 0 1 1 1 0 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/table.tsv:1482 — Shift each dimension of Y by corresponding amount in Iv
Iv←1 ¯1 ⋄ Y←2 3⍴⍳6 ⋄ Iv(↓↑⍨⍴⍤⊢××⍤⊣+0=⊣)Y
2 3⍴0 4 5 0 0 0

⍝ aplcart/table.tsv:1483 — Compound interest for principals N[1] at rates N[2] in times N[3]
N←(100 200⋄ 0.05 0.1⋄ 1 2 3) ⋄ (↑ ×⌝ 3∘⊃ *⌝ ⍨1+2∘⊃)N
2 2 3⍴105 110.25 115.7625 110 121 133.1000000000001 210 220.5 231.525 220 242 266.2000000000001

⍝ aplcart/table.tsv:1484 — Area of triangle with side lengths N
N←3 4 5 ⋄ 0.5∘(⊣*⍨+.××.-0,⊢)N   ⍝ 6

⍝ aplcart/table.tsv:1485 — Valid credit card?
Jv←4 5 3 9 1 4 8 8 0 3 4 3 6 4 6 7 ⋄ (0=10|1⊥∘,0 10⊤⊢×∘⌽1 2⍴⍨≢)Jv
1

⍝ aplcart/table.tsv:1486 — Remove leading, multiple and trailing Xs's
Xs←0 ⋄ Yv←0 0 1 0 0 2 0 0 ⋄ Xs(1↓,⊢⍤/⍨1(⊢∨⌽)0,≠)Yv
1 0 2

⍝ aplcart/table.tsv:1487 — Consecutive integers from Is to Js (descending if Is>Js)
Is←5 ⋄ Js←2 ⋄ Is(⊣,⊣-∘(⍳∘|××)-)Js   ⍝ 5 4 3 2

⍝ aplcart/table.tsv:1488 — Move items X to end of Y
X←2 4 ⋄ Y←1 2 3 4 2 ⋄ X(⊂⍤⍋⍤∊⍨⍥(⊂⍤¯1)⌷⊢)Y
1 3 2 4 2

⍝ aplcart/table.tsv:1489 — Expansion vector (left argument for \ or ⍀) to insert Jv[i] elements after i'th element
Jv←2 0 1 ⋄ ((¯1⌽+\∊⍨∘⍳+/)1∘+)Jv   ⍝ 1 0 0 1 1 0

⍝ aplcart/table.tsv:1490 — Changing node matrix Im (starts,ends) to a connection matrix
Im←3 2⍴1 2 2 3 3 1 ⋄ (-/(⍳⌈/∘,) =⌝ ⍉)Im   ⍝ 3 2⍴1 1 ¯1 1 1 ¯1

⍝ aplcart/table.tsv:1491 — Shannon entropy of array ⍵
Y← 'aaabbc'  ⋄ (-1⊥2(⍟×⊢)⊢∘≢⌸÷≢)Y   ⍝ 1.459147917027245

⍝ aplcart/table.tsv:1492 — Null near-zero (within absolute distance Ms) real and imaginary parts in N
Ms←0.01 ⋄ N←0.001j2 3j0.002 0.001j0.002 ⋄ Ms(0j1⊥⊣(⊢×<∘|)11 9 ○⌝ ⊢)N
0j2 3 0

⍝ aplcart/table.tsv:1493 — Number of decimals of elements of Nv; Reviewed Execute example checked through the Rust reference worker
Nv←1.2 1.23 1.234 ⋄ (⌊10⍟⊢÷⍨∘⍎'.'~⍨⍕)Nv   ⍝ 1 2 3

⍝ aplcart/table.tsv:1494 — Is'th number in the Aliqout sequence for Js
Is←2 ⋄ Js←6 ⋄ Is{(+/∘∪⍳⍛∨)⍣⍺⊢⍵}Js   ⍝ 28

⍝ aplcart/table.tsv:1495 — Extract text (without quotes) in expression
Dv← 'a+''hello''+''world''+b'  ⋄ (~∧≠\)⍤=∘''''⍛⊆Dv
('hello' ⋄ 'world')

⍝ aplcart/table.tsv:1496 — Underlines non-blanks in a string
Dv← 'ab cd'  ⋄ (⊢,[0.5]'¯'\⍨≠∘' ')Dv   ⍝ 2 5⍴'ab cd¯¯ ¯¯'

⍝ aplcart/table.tsv:1497 — Diagonal matrix with elements of Yv (filled appropriately)
Yv←1 2 3 ⋄ (⌽∘⍳∘≢⌽⊢,0∘⍴⍴⍨0 ¯1+≢)Yv   ⍝ 3 3⍴1 0 0 0 2 0 0 0 3

⍝ aplcart/table.tsv:1498 — Rounding N to nearest M (favouring towards 0)
M←0.5 ⋄ N←¯1.25 ¯0.75 0.75 1.25 ⋄ M(⊢∘××⊣×∘⌈¯0.5+∘|÷⍨)N
¯1 ¯0.5 0.5 1

⍝ aplcart/table.tsv:1499 — Rounding N to nearest M (favouring away from 0)
M←0.5 ⋄ N←¯1.25 ¯0.75 0.75 1.25 ⋄ M(⊢∘××⊣×∘⌊0.5+∘|÷⍨)N
¯1.5 ¯1 1 1.5

⍝ aplcart/table.tsv:1500 — Is Bm an upper triangular matrix without diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (~0∊×∘|≤ <⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 1 1 0 1 1 0 0 1 ⋄ (~0∊×∘|≤ <⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 1 1 0 0 1 0 0 0 ⋄ (~0∊×∘|≤ <⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 1 0 0 0 0 0 0 ⋄ (~0∊×∘|≤ <⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 1 1 0 1 1 0 0 1) 0 (3 3⍴0 1 1 0 0 1 0 0 0) 1 (3 3⍴0 0 1 0 0 0 0 0 0) 1

⍝ aplcart/table.tsv:1501 — Is Bm an upper triangular matrix with diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (~0∊×∘|≤ ≤⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 1 1 0 1 1 0 0 1 ⋄ (~0∊×∘|≤ ≤⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 1 1 0 0 1 0 0 0 ⋄ (~0∊×∘|≤ ≤⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 1 0 0 0 0 0 0 ⋄ (~0∊×∘|≤ ≤⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 1 1 0 1 1 0 0 1) 1 (3 3⍴0 1 1 0 0 1 0 0 0) 1 (3 3⍴0 0 1 0 0 0 0 0 0) 1

⍝ aplcart/table.tsv:1502 — Is Bm a lower triangular matrix with diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (~0∊×∘|≤ ≥⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 0 0 1 1 0 1 1 1 ⋄ (~0∊×∘|≤ ≥⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 0 0 1 0 0 1 1 0 ⋄ (~0∊×∘|≤ ≥⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 0 0 0 0 1 0 0 ⋄ (~0∊×∘|≤ ≥⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 0 0 1 1 0 1 1 1) 1 (3 3⍴0 0 0 1 0 0 1 1 0) 1 (3 3⍴0 0 0 0 0 0 1 0 0) 1

⍝ aplcart/table.tsv:1503 — Is Bm a lower triangular matrix without diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (~0∊×∘|≤ >⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 0 0 1 1 0 1 1 1 ⋄ (~0∊×∘|≤ >⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 0 0 1 0 0 1 1 0 ⋄ (~0∊×∘|≤ >⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 0 0 0 0 1 0 0 ⋄ (~0∊×∘|≤ >⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 0 0 1 1 0 1 1 1) 0 (3 3⍴0 0 0 1 0 0 1 1 0) 1 (3 3⍴0 0 0 0 0 0 1 0 0) 1

⍝ aplcart/table.tsv:1504 — Sum of M'th powers of positive divisors of Js
M←1 2 ⋄ Js←6 ⋄ M(+⌿⊣ *⌝ ⍨∘∪⊢∨⊢∘⍳)Js   ⍝ 12 50

⍝ aplcart/table.tsv:1505 — Multivariate Beta Function
N←2 3 4 ⋄ ((×⌿∘!-∘1)÷∘!¯1++⌿)N   ⍝ 0.0002976190476190476

⍝ aplcart/table.tsv:1506 — Area of a polygon given Mv,Nv endpoints
Mv←0 2 2 0 ⋄ Nv←0 0 3 3 ⋄ Mv(|+.×∘(¯1∘⌽-1∘⌽)÷2⍨)Nv
6

⍝ aplcart/table.tsv:1507 — Perspective projection of Nm from distance Ms; Correct perspective projection to multiply x+iy by distance/(distance-z) per column; Upstream matrix division gives LENGTH ERROR; independently checked two points with different depths
Ms←10 ⋄ Nm←3 2⍴1 2 3 4 5 6 ⋄ Ms((0J1⊥1↓⊢∘⊖)×⊣÷⊣-⊢⌿⍤⊢)Nm
2j6 5j10

⍝ aplcart/table.tsv:1508 — Rot-13; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1. Corrected ROT13 to 1+26|12+index so M does not select index zero; sample covers M/N and nonletters
{•A[1+26|12+•A⍳⍵]}@(∊∘•A)'AMNZ 123 abc!'
'NZAM 123 abc!'

⍝ aplcart/table.tsv:1509 — Cyclic compression of successive blanks
Dv← '  ab  c  '  ⋄ (⊢⊢⍤/⍨1(⊢∨⌽)' '∘≠)Dv   ⍝ ' ab c'

⍝ aplcart/table.tsv:1510 — Sorted frequency table
Y←1 2 1 3 1 2 ⋄ ({⍵⌷⍨⊂⍒⊢/⍵},∘≢⌸)Y   ⍝ 3 2⍴1 3 2 2 3 1

⍝ aplcart/table.tsv:1511 — First one (<\) in each subvector of Bv indicated by Av (fast ∊<\¨Av⊂Bv)
Av←1 0 0 1 0 0 ⋄ Bv←0 1 1 1 0 1 ⋄ Av(∧∨⊢{⍵\(</∘(2∘↕))0,⍵/⍺}∨)Bv
0 1 0 1 0 0

⍝ aplcart/table.tsv:1512 — State of switch given Bv on and Av off spikes
Av←0 0 1 0 0 1 ⋄ Bv←1 0 0 0 1 0 ⋄ Av(≠\∨{⍺\(≠/∘(2∘↕))0,⍺/⍵}⊢)Bv
1 1 0 0 1 0

⍝ aplcart/table.tsv:1516 — Position of /*comments*/
D← 'a/*bc*/d/*e*/f'  ⋄ '/*'∘(≠\⍷∨¯1⌽∘⌽⍷∘⌽)D
0 1 1 1 1 1 1 0 1 1 1 1 1 0

⍝ aplcart/table.tsv:1517 — Predicted values of least squares exponential fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←2 6 18 54 ⋄ Mv(*⊢∘⍟(⊢+.×⌹)1,∘⍪⊣)Nv
2 6.000000000000001 18 54.00000000000001

⍝ aplcart/table.tsv:1518 — Distribution of Y into intervals with cut-offs X
X←2 4 ⋄ Y←0 1 2 3 4 5 ⋄ X({¯1+≢⍵}⌸⍸,⍨0,∘⍳∘≢⊣)Y
2 2 2

⍝ aplcart/table.tsv:1521 — Leading ones (∧⍀) in each subvector of Bv indicated by Av
Av←1 0 0 1 0 0 ⋄ Bv←1 1 0 1 0 1 ⋄ Av(≥{~≠\⍺\(≠/∘(2∘↕))1,⍺⌿⍵}⊢)Bv
1 1 0 1 0 0

⍝ aplcart/table.tsv:1522 — Probabilistic XOR
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M((⊣-×)(+-×)(⊢-×))N
1 0.453125 1

⍝ aplcart/table.tsv:1523 — Depth of parentheses
Dv← 'a(b(c)d)e'  ⋄ (+\'('∘=-¯1↓0,')'∘=)Dv
0 1 1 2 2 2 1 1 0

⍝ aplcart/table.tsv:1524 — Are Is and Js amicable numbers?
Is←220 ⋄ Js←284 ⋄ Is(∧/+=∘(+/∘∪⍳⍛∨)¨,)Js
1

⍝ aplcart/table.tsv:1525 — Bubble sort; First-true masks use cumulative counts under basedpl left scan
⌽@(({⍵∧1=+\⍵}∨1⌽{⍵∧1=+\⍵})0,(>/∘(2∘↕)))⍣≡ 3 1 4 1 5
1 1 3 4 5

⍝ aplcart/table.tsv:1526 — Length of edges of pyramid with height and width Mv and length Ns
Mv←3 4 ⋄ Ns←6 ⋄ Mv(⊣,⍥(2*∘÷⍨+.×⍨)⊢÷2⍨)Ns
5 3

⍝ aplcart/table.tsv:1528 — Remove leading, trailing and duplicate blanks
Dv← '  ab  c  '  ⋄ ' '∘(1↓,⊢⍤/⍨1(⊢∨⌽)0,≠)Dv
'ab c'

⍝ aplcart/table.tsv:1529 — Arithmetic-harmonic mean
Nv←1 4 ⋄ (↑((+⌿÷≢),≢÷1⊥÷)⍣≡)Nv   ⍝ 2

⍝ aplcart/table.tsv:1530 — Add line numbers to table Xm; Format exact counts as ordinary numbers
Ym←2 3⍴⍳6 ⋄ ((3⌽']  [',⍕)⍤0∘⍳∘(0+≢),⍕)Ym
2 10⍴'[1]  1 2 3[2]  4 5 6'

⍝ aplcart/table.tsv:1531 — Syllabisation of a Finnish word Dv
Dv←'Hello, world! 123'  ⋄ Dv⊂⍨1,1↓(≢Dv)↑</2↕(•C Dv)∊'aeiouyäö'
('Hel' ⋄ 'lo, ' ⋄ 'world! 123')

⍝ aplcart/table.tsv:1532 — Groups of ones in Bv pointed to by at least one 1 in Av
Av←0 1 0 0 0 0 0 1 ⋄ Bv←0 1 1 0 1 1 0 1 ⋄ Av(∧{⍵∧s∊⍺/s←+\(</∘(2∘↕))0,⍵}⊢)Bv
0 1 1 0 0 0 0 1

⍝ aplcart/table.tsv:1534 — Date (⎕TS format) to M/D/YY
Jv←2026 9 18 ⋄ ('/'@(' '∘=)∘⍕100|1⌽3∘↑)Jv
'9/18/26'

⍝ aplcart/table.tsv:1535 — Date (⎕TS format) to D.M.YYYY
Jv←2026 9 18 ⋄ ('.'@(' '∘=)∘⍕∘⌽3∘↑)Jv   ⍝ '18.9.2026'

⍝ aplcart/table.tsv:1536 — Mask Operator: Merge X and Y using Bv (1 for X's item, 0 for Yv's item)
X←10 20 30 ⋄ Bv←1 0 1 ⋄ Y←1 2 3 ⋄ X(Bv{(⍶⌿⍺)@(⍸⍶)⊢⍵})Y
10 2 30

⍝ aplcart/table.tsv:1537 — Diagonal ravel
Ym←2 3⍴⍳6 ⋄ (,⌷⍨∘⊂∘⍋1⊥⍴⊤¯1+∘⍳×/∘⍴)Ym   ⍝ 1 2 4 3 5 6

⍝ aplcart/table.tsv:1538 — Vector (cross) product of vectors
Mv←1 2 3 ⋄ Nv←4 5 6 ⋄ Mv((1∘⌽⍤⊣×¯1⌽⊢)-¯1∘⌽⍤⊣×1⌽⊢)Nv
¯3 6 ¯3

⍝ aplcart/table.tsv:1539 — Are Is and Js betrothed numbers?
Is←48 ⋄ Js←75 ⋄ Is(∧/+=¯1+∘(+/∘∪⍳⍛∨)¨,)Js
1

⍝ aplcart/table.tsv:1541 — Doubling quotes for execution
Dv← 'don''t'  ⋄ ''''∘(⊣,⊣,⍨⊢⊢⍤/⍨1+=)Dv   ⍝ '''don''''t'''

⍝ aplcart/table.tsv:1542 — Histogram (distribution barchart, down the page)
Jv←1 3 3 4 ⋄ ({'⎕'/⍨¯1+≢⍵}⌸⌈/⍳⍛,⊢)Jv   ⍝ 4 2⍴'⎕   ⎕⎕⎕ '

⍝ aplcart/table.tsv:1544 — Truth table: All possibilities of Boolean primitive Ds; Reviewed Execute example checked through the Rust reference worker
Ds←'∧' ⋄ 0 1∘{(⍵,⍺)⍪⍺, (⍎⍵)⌝ ⍨⍺}Ds   ⍝ 3 3⍴'∧' 0 1 0 0 0 1 0 1

⍝ aplcart/table.tsv:1545 — Is'th moment of Nv
Is←2 ⋄ Nv←1 2 4 ⋄ Is(⊢∘≢÷⍨1⊥⊣*⍨⊢-⊢∘≢÷⍨1⊥⊢)Nv
1.555555555555556

⍝ aplcart/table.tsv:1546 — Theoretical standard deviation
Nv←1 2 4 ⋄ ((2*∘÷⍨+⌿÷≢)2*⍨⊢-+⌿÷≢)Nv   ⍝ 1.247219128924647

⍝ aplcart/table.tsv:1547 — Cumulative sum (+\) in each group of ones
Bv←0 1 1 0 1 1 1 0 ⋄ 0 {⍵×1+⍺}\Bv   ⍝ 0 1 2 0 1 2 3 0

⍝ aplcart/table.tsv:1548 — Shortest path length matrix from weighted adjacency matrix n2, using Floyd-Warshall
Nm←3 3⍴0 2 9 2 0 3 9 3 0 ⋄ (⍳⍤≢(⊢⌊⌷⍤1 +⌝ ⌷)/⍤,⊂)Nm
3 3⍴0 2 5 2 0 3 5 3 0

⍝ aplcart/table.tsv:1549 — Component of Mv in direction of Nv
Mv←1 2 3 ⋄ Nv←1 1 0 ⋄ Mv(⊢×+.×)∘(⊢÷2*∘÷⍨+.×⍨)Nv
1.5 1.5 0

⍝ aplcart/table.tsv:1550 — Arithmetic-geometric mean
Nv←1 4 ⋄ (↑((+⌿÷≢),×⌿*∘÷≢)⍣≡)Nv   ⍝ 2.243028580287603

⍝ aplcart/table.tsv:1551 — Product of polynomials with descending coefficients
Mv←1 2 ⋄ Nv←1 3 2 ⋄ Mv(+⌿∘⊃(,\0×⊣)(1↓,)¨×∘⊂)Nv
1 5 8 4

⍝ aplcart/table.tsv:1552 — Geometric-harmonic mean
Nv←1 4 ⋄ (↑((×⌿*∘÷≢),≢÷1⊥÷)⍣≡)Nv   ⍝ 1.783303179974246

⍝ aplcart/table.tsv:1553 — Median of non-empty Nv
Nv←7 1 4 2 ⋄ (2÷⍨1⊥⊢⌷⍨∘⊂⍋⌷⍨∘⊂∘⌈2÷⍨0 1+≢)Nv
3

⍝ aplcart/table.tsv:1554 — Sample standard deviation
Nv←1 2 4 ⋄ ((2*∘÷⍨+⌿÷¯1+≢)2*⍨⊢-+⌿÷≢)Nv   ⍝ 1.527525231651947

⍝ aplcart/table.tsv:1555 — Theoretical variance
Nv←1 2 4 ⋄ (≢÷⍨≢÷⍨(≢×+.*∘2)-2*⍨+⌿)Nv   ⍝ 1.555555555555556

⍝ aplcart/table.tsv:1556 — Js-bit reflected Gray code
Js←3 ⋄ (2∘*↑(⌽2*⍳)⊖∘⍉⍴∘2⊤2/∘⍳2∘*)Js
8 3⍴1 1 0 1 1 0 1 0 1 1 0 1 1 0 0 1 0 0 0 1 1 0 1 1

⍝ aplcart/table.tsv:1557 — Is Dv a valid Finnish social security number? (10=≢Dv); Concrete checksum recipe with matching and mismatching check characters; independent modulo-31 checksum T and Dyalog result 1 0. No external identity lookup
valid←(⊢/=(•D,•A~'GIOQ')⊃⍨1+31|∘⍎9∘↑) ⋄ valid¨'131052308T' '131052308A'
1 0

⍝ aplcart/table.tsv:1558 — Ordinal suffix for positive integer Js
Ord ← ⍕,{2↑'thstndrd'↓⍨2×↑⍵⌽∊1 0 8\⊂10↑0,⍳3} ⋄ (Ord 0 ⋄ Ord¨⍳121 ⋄ Ord 1000000)
('0th' ⋄ ('1st' ⋄ '2nd' ⋄ '3rd' ⋄ '4th' ⋄ '5th' ⋄ '6th' ⋄ '7th' ⋄ '8th' ⋄ '9th' ⋄ '10th' ⋄ '11th' ⋄ '12th' ⋄ '13th' ⋄ '14th' ⋄ '15th' ⋄ '16th' ⋄ '17th' ⋄ '18th' ⋄ '19th' ⋄ '20th' ⋄ '21st' ⋄ '22nd' ⋄ '23rd' ⋄ '24th' ⋄ '25th' ⋄ '26th' ⋄ '27th' ⋄ '28th' ⋄ '29th' ⋄ '30th' ⋄ '31st' ⋄ '32nd' ⋄ '33rd' ⋄ '34th' ⋄ '35th' ⋄ '36th' ⋄ '37th' ⋄ '38th' ⋄ '39th' ⋄ '40th' ⋄ '41st' ⋄ '42nd' ⋄ '43rd' ⋄ '44th' ⋄ '45th' ⋄ '46th' ⋄ '47th' ⋄ '48th' ⋄ '49th' ⋄ '50th' ⋄ '51st' ⋄ '52nd' ⋄ '53rd' ⋄ '54th' ⋄ '55th' ⋄ '56th' ⋄ '57th' ⋄ '58th' ⋄ '59th' ⋄ '60th' ⋄ '61st' ⋄ '62nd' ⋄ '63rd' ⋄ '64th' ⋄ '65th' ⋄ '66th' ⋄ '67th' ⋄ '68th' ⋄ '69th' ⋄ '70th' ⋄ '71st' ⋄ '72nd' ⋄ '73rd' ⋄ '74th' ⋄ '75th' ⋄ '76th' ⋄ '77th' ⋄ '78th' ⋄ '79th' ⋄ '80th' ⋄ '81st' ⋄ '82nd' ⋄ '83rd' ⋄ '84th' ⋄ '85th' ⋄ '86th' ⋄ '87th' ⋄ '88th' ⋄ '89th' ⋄ '90th' ⋄ '91st' ⋄ '92nd' ⋄ '93rd' ⋄ '94th' ⋄ '95th' ⋄ '96th' ⋄ '97th' ⋄ '98th' ⋄ '99th' ⋄ '100th' ⋄ '101st' ⋄ '102nd' ⋄ '103rd' ⋄ '104th' ⋄ '105th' ⋄ '106th' ⋄ '107th' ⋄ '108th' ⋄ '109th' ⋄ '110th' ⋄ '111th' ⋄ '112th' ⋄ '113th' ⋄ '114th' ⋄ '115th' ⋄ '116th' ⋄ '117th' ⋄ '118th' ⋄ '119th' ⋄ '120th' ⋄ '121st') ⋄ '1000000th')

⍝ aplcart/table.tsv:1561 — A magic square, odd side Js
Js←3 ⋄ ((⍳-∘⌈÷∘2)(⊣⊖⌽),⍨⍴∘⍳×⍨)Js   ⍝ 3 3⍴8 1 6 3 5 7 4 9 2

⍝ aplcart/table.tsv:1562 — Scatter plot of two series (one per row of Jm)
Jm←2 3⍴1 2 3 3 1 2 ⋄ {⍉' +○⍟'[1+2⊥⍵ =⌝ ⌽⍳⌈/,⍵]}Jm
3 3⍴'+ ○ ○+○+ '

⍝ aplcart/table.tsv:1563 — Start and length of groups of 1s in Bv
Bv←0 1 1 0 1 1 1 0 ⋄ p←⍸≠/2↕0,Bv,0 ⋄ -⍨\((≢p)÷2) 2⍴p
2 2⍴2 2 5 3

⍝ aplcart/table.tsv:1564 — Number of days in months I of years J
I←2 2 4 ⋄ J←2000 1900 2026 ⋄ I((31-2|7|¯1+⊣)-2∘=⍤⊣×2-0≠.=400 100 4 |⌝ ⊢)J
29 28 30

⍝ aplcart/table.tsv:1565 — Sample variance
Nv←1 2 4 ⋄ (((≢×+.*∘2)-2*⍨+⌿)÷≢×1⌈¯1+≢)Nv
2.333333333333333

⍝ aplcart/table.tsv:1566 — Sample Pearson correlation coefficient
Mv←1 2 3 4 ⋄ Nv←2 4 6 8 ⋄ Mv+.×⍥((⊢÷2*∘÷⍨+.×⍨)⊢-+⌿÷≢)Nv
1

⍝ aplcart/table.tsv:1568 — Solutions of quadratic equation Nv₁x²+Nv₂x+Nv₃=0
Nv←1 ¯5 6 ⋄ (↑÷¯2÷2∘⊃-¯1 1×2*∘÷⍨(×⍨2∘⊃)-(×/4@2))Nv
2 3

⍝ aplcart/table.tsv:1569 — Convert bits Bv representing a signed integer of As-endianess (0:big, 1:little) into a number
(1 {((¯1*↑∘,)×2⊥↑∘,≠,)⊖⍣⍺⊢(8÷⍨≢⍵)8⍴⍵} 64↑1 1 ⋄ 1 {((¯1*↑∘,)×2⊥↑∘,≠,)⊖⍣⍺⊢(8÷⍨≢⍵)8⍴⍵} ~64↑1 1)
192 ¯192

⍝ aplcart/table.tsv:1571 — Numeric matrix of all unordered combinations of Is out of Js without replacement
Is←2 ⋄ Js←4 ⋄ Is({⍵/⍨∧⌿(</[2]∘(2∘↕))⍵}1+-⍛↑∘⍳⊤∘⍳!×∘!⊣)Js
2 6⍴1 1 1 2 2 3 2 3 4 3 4 4

⍝ aplcart/table.tsv:1573 — Value of saddle point; First-true masks use cumulative counts under basedpl left scan
Nm←2 2⍴3 4 1 2 ⋄ (,⊢⍤/⍨(⊢=⍴⍴⌈⌿){⍵∧1=+\⍵}⍤,⍤∧⊢=∘⍉⌽∘⍴⍴⌊/)Nm
1⍴3

⍝ aplcart/table.tsv:1574 — Convert inverted table to table (character data as matrices; remove trailing spaces); dfns display import/wrappers omitted to test underlying arrays
(⍉∘⊃{(+/∨\' '≠⌽⍵)↑¨↓⍵}¨@(2=≢∘⍴¨)) (2 4⍴'Ab  Cdef' ⋄ 1 2 ⋄ 7 3)
2 3⍴('Ab') 1 7 ('Cdef') 2 3

⍝ aplcart/table.tsv:1575 — Multiplicative inverse of Js modulo Is (fast)
Is←7 ⋄ Js←3 ⋄ Is(⊣|∘↑{0=⍵:1 0 ⋄ (⍵∇⍵|⍺)+.×0 1,⍪1,-⌊⍺÷⍵}⍨)Js
5

⍝ aplcart/table.tsv:1580 — Determinant of any square matrix
Nm←3 3⍴1 2 3 0 4 5 1 0 6 ⋄ (↑∘,({-⍺+.×⍨(+\-+/)@( =⌝ ⍨⍳∘≢)⍵× ≤⌝ ⍨⍳≢⍵}/≢⍴⊂))Nm
22

⍝ aplcart/table.tsv:1583 — ⍵-permutation of vertices of graph ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:32. Local definitions replace the dfns namespace; no namespace feature implied
gperm ← { (⊂⍵)⍳¨⍺[⍵] }
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←g gperm 2 1 3 4 5
⍝ =>
3 (1 3) (1 4) (2 5) 3

⍝ aplcart/table.tsv:1584 — Insert vertex ⍵ in graph ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:34. Local definitions replace the dfns namespace; no namespace feature implied
insnode ← {
  (⍵⌈⍴⍺)↑⍺,⍵⍴⊂⍬
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←g insnode 10
⍝ =>
(2 3) 3 (2 4) (1 5) 3 (⍬) (⍬) (⍬) (⍬) (⍬)

⍝ aplcart/table.tsv:1585 — Graph ⍺ with vertex ⍵ removed; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:35. Local definitions replace the dfns namespace; no namespace feature implied
remnode ← {
  new←(⍵≠⍳⍴⍺)/⍺~¨⍵
  new-new>⍵
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←g remnode 3
⍝ =>
(1⍴2 ⋄ ⍬ ⋄ 1 4 ⋄ ⍬)

⍝ aplcart/table.tsv:1587 — Hypersphere surface area; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:265. Local definitions replace the dfns namespace; no namespace feature implied
ksphere ← {
  n←⍺+1
  pi←(π1)*n÷2
  n×(⍵*⍺)×pi÷!n÷2
}
result←3 ksphere 2
⍝ =>
157.9136704174297

⍝ aplcart/table.tsv:1589 — Extract html segments; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:183. Local definitions replace the dfns namespace; no namespace feature implied
htx ← {
  1≠≡,⍵:⍺ ∇{⍺,' ',⍵}/⍵
  xtags←{seg sep cmb vec ⍵}
  seg←{(1=2|⍳⍴⍵)/⍵}
  sep←{((fm⍷⍵)∨to⍷⍵)⊂⍵}
  cmb←{(~'  '⍷⍵)/⍵}
  vec←{(~⍵∊•UCS 8 10 13){⍺\⍺/⍵}⍵}
  rlt←{(1++/∧\'>'≠⍵)↓⍵}
  att←{⍵,to,'>'}
  fm to←'<' '</',¨⊂⍺~'<>'
  '<'=↑⍺:att¨xtags,⍵
         rlt¨xtags,⍵
}
newl←•ucs 13
    htm←,'<html>                                                    ',newl
    htm,←'  <body>                                                  ',newl
    htm,←'    <table>                                               ',newl
    htm,←'      <tr><td>%</td><td>Eye Poke</td><td>Kumquat</td></tr>',newl
    htm,←'      <tr><td>Guys</td><td>60</td><td>40</td></tr>        ',newl
    htm,←'      <tr><td>Dolls</td><td>20</td><td>80</td></tr>       ',newl
    htm,←'    </table>                                              ',newl
    htm,←' </body>                                                  ',newl
    htm,←'</html>                                                   ',newl
result←'td' htx htm
⍝ =>
(1⍴'%' ⋄ 'Eye Poke' ⋄ 'Kumquat' ⋄ 'Guys' ⋄ '60' ⋄ '40' ⋄ 'Dolls' ⋄ '20' ⋄ '80')

⍝ aplcart/table.tsv:1590 — Kaprekar's operation; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:48. Local definitions replace the dfns namespace; no namespace feature implied
k6174 ← {
  enco←(4/10)∘⊤
  deco←enco⍣¯1
  1=⍴∪enco ⍵:'error'
  ⍬{
    ⍵=↑⌽⍺:⍺
    v←{⍵[⍒⍵]}enco ⍵
    (⍺,⍵)∇(deco v)-deco⌽v
  }⍵
}
result←k6174 3524
⍝ =>
3524 3087 8352 6174

⍝ aplcart/table.tsv:1594 — Value for key ⍵, and reduced list ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:15. Local definitions replace the dfns namespace; no namespace feature implied
alpop ← {
  keys vals←⍺
  indx←keys⍳⊂⍵
  val←indx⊃vals
  list←(⊂indx≠⍳⍴keys)/¨⍺
  val list
}
list ← { {⍺ ⍵}/⍵,'∘' }
found ← ('milly' 'molly' 'may' ⋄ 'star' 'thing' 'stone')
result←found alpop 'molly' 
⍝ =>
('thing' ⋄ ('milly' 'may' ⋄ 'star' 'stone'))

⍝ aplcart/table.tsv:1595 — Value for key ⍵ from list ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:14. Local definitions replace the dfns namespace; no namespace feature implied
alget ← {
  keys vals←⍺
  (keys⍳⊂⍵)⊃vals
}
found ← ('milly' 'molly' 'may' ⋄ 'star' 'thing' 'stone')
result←found alget 'may' 
⍝ =>
'stone'

⍝ aplcart/table.tsv:1596 — Breadth-first search of graph ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:41. Local definitions replace the dfns namespace; no namespace feature implied
search ← {
  graph←⍺
  ⍵{
    ⍵≡⍬:⍺
    adjv←⍵⊃¨⊂graph
    next←∪(⊃,/adjv)~⍺
    (⍺,next)∇ next
  }⍵
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←g search 3
⍝ =>
3 2 4 1 5

⍝ aplcart/table.tsv:1597 — Path through spanning tree ⍺ to vertex ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:73. Local definitions replace the dfns namespace; no namespace feature implied
span ← {
  graph←⍺
  (¯2+(⍳⍴⍺)∊⍵){
    ⍵≡⍬:⍺
    next←graph[⍵]∩¨⊂⍸⍺=¯2
    back←⍵+0×next
    tree←(∊back)@(∊next)⊢⍺
    tree ∇∪∊next
  }⍵
}
stpath ← {
  tree←⍺
  ⍬{
    ⍵<0:(⍵=¯2)↓⍺
    (⍵,⍺)∇ ⍵⊃tree
  }⍵
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←(g span 3)stpath 5
⍝ =>
3 4 5

⍝ aplcart/table.tsv:1601 — Numeric matrix of all permutations of length Js; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:208. Local definitions replace the dfns namespace; no namespace feature implied
pmat ← {
  {
    1≥⍴⍵:⊃,↓⍵ ⋄ ⊃⍪/⍵,∘∇¨⍵∘~¨⍵
  }⍳⍵
}
result←pmat 3
⍝ =>
6 3⍴1 2 3 1 3 2 2 1 3 2 3 1 3 1 2 3 2 1

⍝ aplcart/table.tsv:1603 — Hungarian method cost assignment; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:14. Local definitions replace the dfns namespace; no namespace feature implied; First-true masks use cumulative counts under basedpl left scan
assign ← {
  step0←{step1(⌽⌈\⌽⍴⍵)↑⍵}
  step1←{step2⊃(↓⍵)-⌊/⍵}
  step2←{
    stars←{
      ~1∊⍵:⍺
      next←{⍵∧1=+\⍵}{⍵∧1=+⍀⍵}⍵
      mask←(rows next)∨cols next
      (⍺∨next)∇ ⍵>mask
    }
    zeros←{⍵+0 stars ⍵}⍵=0
    step3 ⍵ zeros
  }
  step3←{costs zeros←⍵
    stars←zeros=2
    covers←2×cols stars
    ~0∊covers:stars
    step4 costs zeros covers
  }
  step4←{costs zeros covers←⍵
    mask←covers=0
    open←1=mask×zeros
    ~1∊open:(⌊/(,mask)/,costs)step6 ⍵
    prime←first open
    prow←rows prime
    star←2=zeros×prow
    ~1∊star:prime step5{
      costs ⍵ prime
    }zeros+2×prime
    cnext←covers+prow-2×cols star
    znext←zeros⌈3×prime
    ∇ costs znext cnext
  }
  step5←{costs zeros prime←⍵
    star←(cols prime)∧zeros=2
    ~1∊star:step3 ⍺{
      {costs ⍵}{⍵-2×⍵=3}⍵-⍺∧⍵>1
    }zeros
    pnext←(rows star)∧zeros=3
    (⍺∨pnext∨star)∇ costs zeros pnext
  }
  step6←{costs zeros covers←⍵
    cnext←costs+⍺×¯1 1+.×0 3 =⌝ covers
    znext←zeros+(×costs)-×cnext
    step4 cnext znext covers
  }
  rows←{(⍴⍵)⍴(↑⌽⍴⍵)/∨/⍵}
  cols←{(⍴⍵)⍴∨⌿⍵}
  first←{(⍴⍵)⍴{⍵∧1=+\⍵},⍵}
  (⍴⍵)↑step0 ⍵
}
costs1 ← ⊃(72 99 88⋄ 23 30 35⋄ 51 59 84)
result←assign costs1
⍝ =>
3 3⍴1 0 0 0 0 1 0 1 0

⍝ aplcart/table.tsv:1607 — Lower-casification; Concrete APLcart library call with self-contained April dfns definitions and standard operand names; no namespace support implied
lcase ← {
  lc←'abcdefghijklmnopqrstuvwxyzåäöàæéñøü'
  uc←'ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖÀÆÉÑØÜ'
  (⍴⍵)⍴(lc,,⍵)[(uc,,⍵)⍳⍵]
}
result←lcase 'HELLO ÅÄÖ' 
⍝ =>
'hello åäö'

⍝ aplcart/table.tsv:1608 — Upper-casification; Concrete APLcart library call with self-contained April dfns definitions and standard operand names; no namespace support implied
ucase ← {
  lc←'abcdefghijklmnopqrstuvwxyzåäöàæéñøü'
  uc←'ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖÀÆÉÑØÜ'
  (⍴⍵)⍴(uc,,⍵)[(lc,,⍵)⍳⍵]
}
result←ucase 'Hello åäö' 
⍝ =>
'HELLO ÅÄÖ'

⍝ aplcart/table.tsv:1610 — Compress multiple blanks; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:171. Local definitions replace the dfns namespace; no namespace feature implied
squeeze ← { (~'  '⍷⍵)/⍵ }
result←squeeze '   oranges    and     lemons' 
⍝ =>
' oranges and lemons'

⍝ aplcart/table.tsv:1611 — Remove trailing blanks; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:290. Local definitions replace the dfns namespace; no namespace feature implied
vtrim ← { 
  lf sp←•UCS 10 32
  1↓¯1↓{
    types←¯1+lf sp⍳⍵
    mask←~1 1⍷types
    comp←mask/types
    csl←2 1 0⍷comp
    lsl←0 1 0⍷comp
    from←mask\csl∨lsl
    upto←1 0⍷types
    (~¯1⌽≠\from∨upto)/⍵
  }lf,⍵,lf
}
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
text←(•ucs 10) {⊃⍶{⍺,⍶,⍵}/⍵} 'Where Alph, the sacred river, ran  ' 'Through caverns measureless to man    ' '  Down to a sunless sea.           '
show←' ·'∘subs
result←vtrim 'some text   ' 
⍝ =>
'some text'

⍝ aplcart/table.tsv:1613 — Replace [LF] with blanks; Concrete APLcart library call with self-contained April dfns definitions and standard operand names; no namespace support implied
unwrap ← { (~⍵∊•UCS 10 13 133){⍺\⍺/⍵}⍵ }
result←unwrap 'one',(•UCS 10),'two',(•UCS 13),'three'
⍝ =>
'one two three'

⍝ aplcart/table.tsv:1615 — Cost vector for path ⍵ through weighted graph ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:97. Local definitions replace the dfns namespace; no namespace feature implied
wcost ← {
  graph costs←↓⍺
  2>≢⍵:0
  {
    node←,⍺⊃graph
    indx←node⍳⍵
    indx⊃,⍺⊃costs
  }/2↕⍵
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
w←(1 3) 1 (4 1) (1 1) 1
result←(2 5⍴g,w)wcost 1 3 4 5 3 2 3 2
⍝ =>
3 1 1 1 4 1 4

⍝ aplcart/table.tsv:1617 — Trim off trailing blank cols; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:149. Local definitions replace the dfns namespace; no namespace feature implied
display ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t} ⋄ 
  box←{
    vrt hrz←(¯1+⍴⍵)⍴¨'│─'
    top←'─⊖→'[1+¯1↑⍺],hrz
    bot←(↑⍺),hrz
    rgt←'┐│',vrt,'┘'
    lax←'│⌽↓'[1+¯1↓1↓⍺],¨⊂vrt
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪⍵⍪bot),rgt
  }
  deco←{⍺←type open ⍵ ⋄ ⍺,axes ⍵}
  axes←{(-2⌈⍴⍴⍵)↑1+×⍴⍵}
  open←{(1⌈⍴⍵)⍴⍵}
  trim←{(~1 1⍷∧⌿⍵=' ')/⍵}
  char←{⍬≡⍴⍵:'─' ⋄ (1+↑⍵∊'¯',•D)⊃'#~'}∘⍕
  type←{{(1+1=⍴⍵)⊃'+'⍵}∪,char¨⍵}
  line←{(1+''≡0⍴⍵)⊃' -'}
  { 
    0=≡⍵:' '⍪(open format ⍵)⍪line ⍵
    1 ⍬≡(≡⍵)(⍴⍵):'∇' 0 0 box format ⍵
    1=≡⍵:(deco ⍵)box open format open ⍵
    ('∊'deco ⍵)box trim format ∇¨open ⍵
  }⍵
}
mtrim ← { (⌽∨\⌽∨⌿⍵≠' ')/⍵ }
result←mtrim 3 5⍴'abc  def  ghi  ' 
⍝ =>
3 3⍴'abcdefghi'

⍝ aplcart/table.tsv:1618 — Boxed display of array; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:53. Local definitions replace the dfns namespace; no namespace feature implied
display ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t} ⋄ 
  box←{
    vrt hrz←(¯1+⍴⍵)⍴¨'│─'
    top←'─⊖→'[1+¯1↑⍺],hrz
    bot←(↑⍺),hrz
    rgt←'┐│',vrt,'┘'
    lax←'│⌽↓'[1+¯1↓1↓⍺],¨⊂vrt
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪⍵⍪bot),rgt
  }
  deco←{⍺←type open ⍵ ⋄ ⍺,axes ⍵}
  axes←{(-2⌈⍴⍴⍵)↑1+×⍴⍵}
  open←{(1⌈⍴⍵)⍴⍵}
  trim←{(~1 1⍷∧⌿⍵=' ')/⍵}
  char←{⍬≡⍴⍵:'─' ⋄ (1+↑⍵∊'¯',•D)⊃'#~'}∘⍕
  type←{{(1+1=⍴⍵)⊃'+'⍵}∪,char¨⍵}
  line←{(1+''≡0⍴⍵)⊃' -'}
  { 
    0=≡⍵:' '⍪(open format ⍵)⍪line ⍵
    1 ⍬≡(≡⍵)(⍴⍵):'∇' 0 0 box format ⍵
    1=≡⍵:(deco ⍵)box open format open ⍵
    ('∊'deco ⍵)box trim format ∇¨open ⍵
  }⍵
}
result←display 1 'a' 'abc' (2 3⍴⍳6)
⍝ =>
6 21⍴'┌→──────────────────┐│     ┌→──┐ ┌→────┐ ││ 1 a │abc│ ↓1 2 3│ ││   - └───┘ │4 5 6│ ││           └~────┘ │└∊──────────────────┘'

⍝ aplcart/table.tsv:1619 — Boxed display of array with axis lengths; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:80. Local definitions replace the dfns namespace; no namespace feature implied; Format exact counts as ordinary numbers
displays ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t}
  box←{
    shp w←open\⍵
    vrt hrz←(¯1+⍴w)⍴¨'│─'
    top←('─⊖→')[1+¯1↑⍺],hrz
    ok←(⍴shp)<⍴hrz
    top←(⍴top)↑(2↑top),(ok/shp),(2+ok×⍴shp)↓top
    bot←(↑⍺),hrz
    rgt←'┐│',vrt,'┘'
    lax←('│⌽↓')[1+¯1↓1↓⍺],¨⊂vrt
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪w⍪bot),rgt
  }
  deco←{⍺←type open ⍵ ⋄ ⍺,axes ⍵}
  axes←{(-2⌈⍴⍴⍵)↑1+×⍴⍵}
  open←{(1⌈⍴⍵)⍴⍵}
  trim←{(1⊃⍵)((~1 1⍷∧⌿(2⊃⍵)=' ')/(2⊃⍵))}
  char←{⍬≡⍴⍵:'─' ⋄ (1+↑⍵∊'¯',•D)⊃'#~'}∘⍕
  type←{{(1+1=⍴⍵)⊃'+'⍵}∪,char¨⍵}
  qfmt←{(⍕0+⍴⍺)(format open ⍵)}
  {
    0=≡⍵:' '⍪(format ⍵)⍪(1+' '≡↑0⍴⍵)⊃' -'
    1 ⍬≡(≡⍵)(⍴⍵):'∇' 0 0 box(,'─')(format ⍵)
    1=≡⍵:(deco ⍵)box open ⍵ qfmt ⍵
    ('∊'deco ⍵)box trim ⍵ qfmt ∇¨open ⍵
  }⍵
}
result←displays 1 'a' 'abc' (2 3⍴⍳6)
⍝ =>
6 21⍴'┌→─4────────────────┐│     ┌→─3┐ ┌→─2 3┐ ││ 1 a │abc│ ↓1 2 3│ ││   - └───┘ │4 5 6│ ││           └~────┘ │└∊──────────────────┘'

⍝ aplcart/table.tsv:1620 — Boxed display of array with axis lengths and subarray depths; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:86. Local definitions replace the dfns namespace; no namespace feature implied; Format exact counts as ordinary numbers
displayr ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t}
  box←{
    vrt hrz←(¯1+⍴⍵)⍴¨'│─'
    top←(1+⍴hrz)↑(↑(1+¯1↑⍺)⌷'─⊖',⊂⍕¯1↑2⊃⍺),hrz
    bot←(⍴top)↑(↑2↓⍺),hrz
    rgt←'┐│',vrt,'┘'
    lax←(↑¨(1+¯1↓3↓⍺)⌷¨(-1⌈¯1+⍴2⊃⍺)↑(⊂'│⌽'),¨⊂∘⍕¨¯1↓0,2⊃⍺),¨⊂vrt
    lax←(⊂1+⍴vrt)↑¨(lax~¨⊂' '),¨'│'
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪⍵⍪bot),rgt
  }
  deco←{⍺←type open ⍵ ⋄ (⍴⍴⍵),(⊂0+⍴⍵),⍺,axes ⍵}
  axes←{(-2⌈⍴⍴⍵)↑1+×⍴⍵}
  open←{(1⌈⍴⍵)⍴⍵}
  trim←{(~1 1⍷∧⌿⍵=' ')/⍵}
  char←{⍬≡⍴⍵:'─' ⋄ (1+↑⍵∊'¯',•D)⊃'#~'}∘⍕
  type←{{(1+1=⍴⍵)⊃'+'⍵}∪,char¨⍵}
  {
    0=≡⍵:' '⍪(open format ⍵)⍪(1+' '=↑0⍴⍵)⊃' -'
    1 ⍬≡(≡⍵)(⍴⍵):''(0 0)'∇' 0 0 box format ⍵
    1=≡⍵:(deco ⍵)box open' ',format open ⍵
    ((⊂⍕0+≡⍵)deco ⍵)box trim' ',format ∇¨open ⍵
  }⍵
}
result←displayr 1 'a' 'abc' (2 3⍴⍳6)
⍝ =>
6 23⍴'┌4────────────────────┐│     ┌3───┐ ┌3─────┐ ││ 1 a │ abc│ 2 1 2 3│ ││   - └────┘ │ 4 5 6│ ││            └~─────┘ │└¯2───────────────────┘'

⍝ aplcart/table.tsv:1621 — Character matrix from tree(s); Concrete APLcart library call; self-contained setup from april/libraries/dfns/tree/demo.lisp:907. Local definitions replace the dfns namespace; no namespace feature implied
tfmt ← {
  ⍺←''
  1=≡,⍵:⊃,↓⍺,⍵
  node←⍺,↑⍵
  subs←(⍺,4↑'·')∘∇¨1↓⍵
  ⊃(⊂node),,/↓¨subs
}
result←tfmt 'hot' 'tea' 'coffee' 
⍝ =>
3 10⍴'hot       ·   tea   ·   coffee'

⍝ aplcart/table.tsv:1626 — Levenshtein distance; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:92. Local definitions replace the dfns namespace; no namespace feature implied
dist ← {
  a←(n+1)⍴(⍴⍺)+n←⍴⍵
  f←⍵{⌊\⍵⌊(↑⍵),(¯1↓⍵)-1+⍺=⍶}
  z←f/(⌽⍺),⊂a
  ↑⌽z
}
result←'Sunday' dist 'Saturday' 
⍝ =>
3

⍝ aplcart/table.tsv:1627 — Spanning-tree path lengths; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:72. Local definitions replace the dfns namespace; no namespace feature implied
span ← {
  graph←⍺
  (¯2+(⍳⍴⍺)∊⍵){
    ⍵≡⍬:⍺
    next←graph[⍵]∩¨⊂⍸⍺=¯2
    back←⍵+0×next
    tree←(∊back)@(∊next)⊢⍺
    tree ∇∪∊next
  }⍵
}
stdists ← {
  tree←⍵
  0{
    next dvec←⍵
    next≡⍬:dvec
    ∆dvec←⍺@next⊢dvec
    ∆next←⍸tree∊next
    (⍺+1)∇ ∆next ∆dvec
  }(⍵⍳¯1)(⍵⊢¨¯1)
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←stdists g span 3
⍝ =>
2 1 0 1 2

⍝ aplcart/table.tsv:1628 — Bijective base-⍺ numeration; Concrete APLcart recipe; local definitions from april/libraries/dfns/numeric/demo.lisp:13 replace the dfns namespace; Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
adic ← { 
  b←⍬⍴⍴a←,⍺
  1=⍴⍴⍵:b⊥a⍳⍵
  1=b:⍵/⍺
  n←⌊b⍟1+⍵×b-1
  z←(¯1+b*n)÷b-1
  a[1+(n/b)⊤⍵-z]
}
to ← { 
  from step←1 ¯1×-\2↑⍺,⍺+×⍵-⍺
  from+step×¯1+⍳0⌈1+⌊(⍵-from)÷step+step=0
}
result←•A adic 703
⍝ =>
'AAA'

⍝ aplcart/table.tsv:1629 — Fast multi-digit power using FFT; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:317. Local definitions replace the dfns namespace; no namespace feature implied; Square the accumulator in basedpl left scan
xtimes ← { m←0
  xroots    ← {×\1,1↓(⍵÷2)⍴¯1*2÷⍵}
  cube      ← {⍵⍴⍨2⍴⍨⌊2⍟⍴⍵}
  extend    ← {(2*⌈2⍟¯1+(⍴⍺)+⍴⍵)↑¨⍺ ⍵}
  floop     ← {(⊣/⍺)∇⍣(×m)⊢(+⌿⍵),[m+0.5]⍺×[⍳m←≢⍴⍺]-⌿⍵}
  FFT       ← {,(cube xroots⍴⍵)floop cube ⍵}
  iFFT      ← {(⍴⍵)÷⍨,(cube+xroots⍴⍵)floop cube ⍵}
  rconvolve ← {(¯1+(⍴⍺)+⍴⍵)↑iFFT×/FFT¨⍺ extend ⍵}
  carry     ← {1↓+⌿1 0⌽0,0 10⊤⍵}
  (+/∧\0=t)↓t←carry⍣≡0,⌊0.5+9○⍺ rconvolve ⍵
}
xpower ← {
  xt←{(0,⍺)xtimes 0,⍵} ⋄ b←⌽2⊥⍣¯1+10⊥⍵
  ⊃,/xt/b/{xt⍨⍺}\(⊂,10⊥⍣¯1+⍺)⍴⍨⍴b
}
result←2 xpower 16
⍝ =>
6 5 5 3 6

⍝ aplcart/table.tsv:1631 — Shannon entropy of message ⍵; Concrete APLcart recipe; local definitions from april/libraries/dfns/array/demo.lisp:352 replace the dfns namespace; Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
shannon ← { -+/(2∘⍟×⊣)¨({≢⍵}⌸÷≢)⍵ }
result←shannon 'banana' 
⍝ =>
1.459147917027245

⍝ aplcart/table.tsv:1632 — Signed from unsigned integers; Concrete APLcart library call with self-contained April dfns definitions and standard operand names; no namespace support implied
int ← { ⊃⍵{(⍺|⍶+⍵)-⍵}/2*⍺-0 1 }
result←8 int 0 127 128 255
⍝ =>
0 127 ¯128 ¯1

⍝ aplcart/table.tsv:1633 — Numeric range classification; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:130. Local definitions replace the dfns namespace; no namespace feature implied
range ← { (⍴⍵)⍴((⍴⍺)↓⍋⍋⍺,,⍵)-⍋⍋,⍵ }
result←0 5 10 15 range ¯2+⍳18
⍝ =>
0 1 1 1 1 1 2 2 2 2 2 3 3 3 3 3 4 4

⍝ aplcart/table.tsv:1636 — List from vector ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:179. Local definitions replace the dfns namespace; no namespace feature implied
list ← { {⍺ ⍵}/⍵,'∘' }
result←list 'hello' 
⍝ =>
'h' ('e' ('l' ('l' ('o∘'))))

⍝ aplcart/table.tsv:1637 — List ⍺ with key-value ⍵ replacement; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:16. Local definitions replace the dfns namespace; no namespace feature implied
alset ← {
  key val←⍵
  {val@(⍺⍳⊂key)⊢⍵}\⍺
}
found ← ('milly' 'molly' 'may' ⋄ 'star' 'thing' 'stone')
result←found alset 'may' 'pebble' 
⍝ =>
(('milly' ⋄ 'molly' ⋄ 'may') ⋄ ('star' ⋄ 'thing' ⋄ 'pebble'))

⍝ aplcart/table.tsv:1638 — List ⍺ extended with key-value pair ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:17. Local definitions replace the dfns namespace; no namespace feature implied
alpush ← {
  ⍺,⍨∘⊂¨⍵
}
found ← ('milly' 'molly' 'may' ⋄ 'star' 'thing' 'stone')
result←found alpush 'may' 'rock' 
⍝ =>
(('may' ⋄ 'milly' ⋄ 'molly' ⋄ 'may') ⋄ ('rock' ⋄ 'star' ⋄ 'thing' ⋄ 'stone'))

⍝ aplcart/table.tsv:1640 — Number of display lines for simple array; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:202. Local definitions replace the dfns namespace; no namespace feature implied
nlines ← {
  {
    (×/⍵)+{
      +/+\0⌈⍵-1,¯1↓⍵
    }×\¯1↓⍵
  }¯1↓⍴⍵
}
result←nlines 2 3 4⍴⍳24
⍝ =>
7

⍝ aplcart/table.tsv:1641 — Egyptian fractions; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:35. Local definitions replace the dfns namespace; no namespace feature implied
efract ← {
  ⍬{
    (p q)←⍵÷∨/⍵
    p=1:⍺,q
    r←p|q ⋄ s←(q-r)÷p
    (⍺,s+1)∇(p-r)(q×s+1)
  }⍺ ⍵
}
result←2 efract 11
⍝ =>
6 66

⍝ aplcart/table.tsv:1642 — Sequence ⍺ … ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:65. Local definitions replace the dfns namespace; no namespace feature implied
hex ← { 
  ⍺←⊢
  1≠≡,⍵:⍺ ∇¨⍵
  0∊⍵-1+⍵:'Too big'
  n←⍬⍴⍺,2*⌈2⍟2⌈16⍟1+⌈/|⍵
  ↓[1]'0123456789abcdef'[1+(n/16)⊤⍵]
}
dec ← { 
  ⍺←0
  1<⍴⍴⍵:⍺∘∇⍤1⊢⍵
  0≡≢⍵:0
  1≠≡,⍵:⍺ ∇¨⍵
  ws←∊∘(•UCS 9 10 13 32 133 160)
  ws↑⍵:⍺ ∇ 1↓⍵
  ws↑⌽⍵:⍺ ∇ ¯1↓⍵
  ∨/ws ⍵:⍺ ∇¨(1+ws ⍵)⊆⍵
  v←16|¯1+'0123456789abcdef0123456789ABCDEF'⍳⍵
  (16⊥v)-⍺×(8≤↑v)×16*≢v
}
to ← { 
  from step←1 ¯1×-\2↑⍺,⍺+×⍵-⍺
  from+step×¯1+⍳0⌈1+⌊(⍵-from)÷step+step=0
}
result←¯3 to 3
⍝ =>
¯3 ¯2 ¯1 0 1 2 3

⍝ aplcart/table.tsv:1643 — Prime factors of ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:45. Local definitions replace the dfns namespace; no namespace feature implied
factors ← { ⍵{
    ⍵,(⍺÷×/⍵)~1
  }∊⍵{
    (0=(⍵*⍳⌊⍵⍟⍺)|⍺)/⍵
  }¨⍬{
    nxt←↑⍵
    msk←0≠nxt|⍵
    ∧/1↓msk:⍺,⍵
    (⍺,nxt)∇ msk/⍵
  }⍵{
    (0=⍵|⍺)/⍵
  }2,(1+2×⍳⌊0.5×⍵*÷2),⍵
}
result←factors 441256830030
⍝ =>
2 3 3 5 71 73 945949

⍝ aplcart/table.tsv:1644 — Tail-recursive Fibonacci; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:39. Local definitions replace the dfns namespace; no namespace feature implied
fibonacci ← { ⍺←0 1
  ⍵=0:↑⍺ ⋄ (1↓⍺,+/⍺)∇ ⍵-1
}
result←fibonacci 10
⍝ =>
55

⍝ aplcart/table.tsv:1645 — Sieve of Eratosthenes; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:161. Local definitions replace the dfns namespace; no namespace feature implied
sieve ← {
  ⍺←⍬
  nxt←1↑⍵
  msk←0≠nxt|⍵
  ∧/1↓msk:⍺,⍵
  (⍺,nxt)∇ msk/⍵
}
to ← { 
  from step←1 ¯1×-\2↑⍺,⍺+×⍵-⍺
  from+step×¯1+⍳0⌈1+⌊(⍵-from)÷step+step=0
}
result←sieve 2 to 100
⍝ =>
2 3 5 7 11 13 17 19 23 29 31 37 41 43 47 53 59 61 67 71 73 79 83 89 97

⍝ aplcart/table.tsv:1647 — Real roots of quadratic; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:302. Local definitions replace the dfns namespace; no namespace feature implied
roots ← {
  a b c←⍵
  d←(b*2)-4×a×c
  (-b+¯1 1×d*0.5)÷2×a
}
result←roots 1 ¯3 2
⍝ =>
2 1

⍝ aplcart/table.tsv:1648 — ⍵ similar integers with sum ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:115. Local definitions replace the dfns namespace; no namespace feature implied
nicediv ← {
  q←⍵⍴⌊⍺÷⍵
  d←+\(⍵|⍺)÷⍵⍴⍵
  i←</2↕0,⌊0.5+d
  q+i
}
result←100 nicediv 7
⍝ =>
14 15 14 14 14 15 14

⍝ aplcart/table.tsv:1649 — Tail recursive factorial; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:38. Local definitions replace the dfns namespace; no namespace feature implied
factorial←{⍺←1 ⋄ ⍵=0:⍺ ⋄ (⍺×⍵)∇⍵-1}
result←factorial 10
⍝ =>
3628800

⍝ aplcart/table.tsv:1650 — Arithmetic mean; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:270. Local definitions replace the dfns namespace; no namespace feature implied
mean ← { sum←+/⍵ ⋄ num←⍴⍵ ⋄ sum÷num }
result←mean 1 2 3 4
⍝ =>
1⍴2.5

⍝ aplcart/table.tsv:1651 — Determinant of square matrix; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:219. Local definitions replace the dfns namespace; no namespace feature implied
det ← { 
  ⍺←1
  0=n←≢⍵:⍺
  i j←1+(⍴⍵)⊤¯1+{⍵⍳⌈/⍵}|,⍵
  k←⍳n
  (⍺×⍵[i;j]×¯1*i+j)∇ ⍵[k~i;k~j]-⍵[k~i;j] ×⌝ ⍵[i;k~j]÷⍵[i;j]
}
result←det 2 2⍴2 3 4 5
⍝ =>
¯2.000000000000002

⍝ aplcart/table.tsv:1652 — Oscillate - probably returns 1; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:129. Local definitions replace the dfns namespace; no namespace feature implied
osc ← { 1=⍵ : 1 ⋄ 2|⍵ : ∇ 1+3×⍵ ⋄ ∇ ⍵÷2 }
result←osc 10
⍝ =>
1

⍝ aplcart/table.tsv:1653 — Greatest Common Divisor; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:47. Local definitions replace the dfns namespace; no namespace feature implied
gcd ← { ⍵=0 : |⍺ ⋄ ⍵∇⍵|⍺ }
result←105 gcd 330
⍝ =>
15

⍝ aplcart/table.tsv:1655 — Locations of item ⍺ in array ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:175. Local definitions replace the dfns namespace; no namespace feature implied
in ← {
  D←|≡item←⍺
  ⍬{
    item≡⍵:,⊂⍺
    D≥|≡⍵:⍬
    paths←⍺∘,∘⊂¨⍳⍴⍵
    ,/,paths ∇¨⍵
  }⍵
}
result←1 in 3 1 4 1 5
⍝ =>
(1⍴2 ⋄ 1⍴4)

⍝ aplcart/table.tsv:1656 — Spanning tree paths; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:78. Local definitions replace the dfns namespace; no namespace feature implied
span ← {
  graph←⍺
  (¯2+(⍳⍴⍺)∊⍵){
    ⍵≡⍬:⍺
    next←graph[⍵]∩¨⊂⍸⍺=¯2
    back←⍵+0×next
    tree←(∊back)@(∊next)⊢⍺
    tree ∇∪∊next
  }⍵
}
stpaths ← {
  tree←⍵
  root←⍵⍳¯1
  paths←(root=⍳⍴⍵)↑¨root
  paths{
    next←(⍵=⊂tree)/¨⊂⍳⍴tree
    (⊂⍬)∧.≡next:⍺
    exts←(⊂¨⍵⊃¨⊂⍺),¨¨next
    indx←,/next
    paths←(,/exts)@indx⊢⍺
    paths ∇ indx
  }root
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←stpaths g span 3
⍝ =>
(3 4 1 ⋄ 3 2 ⋄ 1⍴3 ⋄ 3 4 ⋄ 3 4 5)

⍝ aplcart/table.tsv:1658 — Bayes' formula; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:202. Local definitions replace the dfns namespace; no namespace feature implied
bayes ← { ⍺(×÷+.×)⍵ }
result←(3 2 4÷11 11 16)bayes 11 11 16÷38
⍝ =>
0.3333333333333333 0.2222222222222223 0.4444444444444444

⍝ aplcart/table.tsv:1660 — Select (1↓⍴⍵)-cells from array ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:134. Local definitions replace the dfns namespace; no namespace feature implied
from ← {
  ~(≢⍺)≡≢⍴⍵:'error'
  indx←⍺
  axes←1++\0,¯1↓{↑⍴⍴⍵}¨⍺
  {
    indx axis←⍺
    indx≡,⊂⍬:⍵
    vec←⊂[(⍳⍴⍴⍵)~axis]⍵
    sel←⊃indx⊃¨⊂vec
    pos←(axis-1)+⍳⍴⍴indx
    (pos,(⍳⍴⍴sel)~pos)⍉sel
  }/⌽(⊂⍵),↓⍉⊃indx axes
}
ta1 ta2 ta3 ta4 ← {10⊥¨⍳⌽⍵}¨,\5 4 3 2
result←(2 1⋄ ,⊂⍬)from ta2
⍝ =>
2 5⍴21 22 23 24 25 11 12 13 14 15

⍝ aplcart/table.tsv:1661 — ⍺-selection of items from vector ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:335. Local definitions replace the dfns namespace; no namespace feature implied
select ← { ⍺⊃¨,¨/⊂¨¨⍵ }
result←2 1 2 2 1 select (1 2 3 4 5⋄ 10 20 30 40 50)
⍝ =>
10 2 30 40 5

⍝ aplcart/table.tsv:1668 — Breadth-first span tree for graph ⍺ from vertex ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:48. Local definitions replace the dfns namespace; no namespace feature implied
span ← {
  graph←⍺
  (¯2+(⍳⍴⍺)∊⍵){
    ⍵≡⍬:⍺
    next←graph[⍵]∩¨⊂⍸⍺=¯2
    back←⍵+0×next
    tree←(∊back)@(∊next)⊢⍺
    tree ∇∪∊next
  }⍵
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←g span 1
⍝ =>
¯1 1 1 3 4

⍝ aplcart/table.tsv:1669 — Depth-first spanning tree: graph ⍺ from vertex ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:51. Local definitions replace the dfns namespace; no namespace feature implied
dfspan ← {
  graph←⍺
  trav←{
    ¯2≠⍺⊃⍵:⍵
    next←⌽⍺⊃graph
    tree←⍶@⍺⊢⍵
    ⍺ ⍢/next,⊂tree
  }
  ⍵(¯1 trav)¯2⊣¨⍺
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←g dfspan 1
⍝ =>
¯1 1 2 3 4

⍝ aplcart/table.tsv:1670 — Spanning tree for weighted graph ⍺ from ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:103. Local definitions replace the dfns namespace; no namespace feature implied
wspan ← {
  graph costs←↓⍺
  tree←¯1⊣¨graph
  cost←0@⍵⊢(⍴costs)⍴⌊/⍬
  I←⊃¨∘⊂
  ⍵{
    tree cost←⍵
    ⍺≡⍬:tree
    adjv←⍺ I graph
    accm←⍺ I cost+costs
    mask←accm<adjv I¨⊂cost
    cvec←,/mask/¨accm
    next←mask/¨adjv
    back←,/⍺+0×next
    decr←(⍒cvec)I⊢
    wave←decr,/next
    new←back cvec decr⍨@wave¨⍵
    (∪wave)∇ new
  }tree cost
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
w←(1 3) 1 (4 1) (1 1) 1
result←(⊃g w)wspan 1
⍝ =>
¯1 1 2 3 4

⍝ aplcart/table.tsv:1671 — Minimum Spanning Tree for weighted graph ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:105. Local definitions replace the dfns namespace; no namespace feature implied
wmst ← {
  graph costs←↓⍺
  xvec←⍳⍴graph
  ⍵{
    tree todo←⍵
    todo≡⍬:tree
    edges←(graph∊¨⊂todo)∧xvec∊⍺
    min←⌊/⌊/¨edges/¨costs
    masks←edges∧min=costs
    fm to←{⊃,/masks/¨⍵}¨xvec graph
    fm≡⍬:tree
    (⍺,to)∇(fm@to⊢tree)(todo~to)
  }(¯1⊣¨graph)(xvec~⍵)
}
aa←⊃((2 3 4⋄ 1 3⋄ 1 2 4 5⋄ 1 3 5⋄ 3 4)⋄ (1 3 1⋄ 1 2⋄ 3 2 1 1⋄ 1 1 1⋄ 1 1))
result←aa wmst 2
⍝ =>
2 ¯1 4 1 4

⍝ aplcart/table.tsv:1672 — Unsigned from signed integers; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:114. Local definitions replace the dfns namespace; no namespace feature implied
uns ← { (2*⍺)|⍵ }
to ← { 
  from step←1 ¯1×-\2↑⍺,⍺+×⍵-⍺
  from+step×¯1+⍳0⌈1+⌊(⍵-from)÷step+step=0
}
result←8 uns ¯3 to 3
⍝ =>
253 254 255 0 1 2 3

⍝ aplcart/table.tsv:1673 — Decomposition of Hermitian positive-definite matrix; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:216. Local definitions replace the dfns namespace; no namespace feature implied
Cholesky ← {
  1≥n←≢⍵:⍵*0.5
  p←⌈n÷2
  q←⌊n÷2
  X←(p,p)↑⍵⊣Y←(p,-q)↑⍵⊣Z←(-q,q)↑⍵
  L0←∇ X
  L1←∇ Z-(TT←(+⍉Y)+.×⌹X)+.×Y
  ((p,n)↑L0)⍪(TT+.×L0),L1
}
result←Cholesky 3 3⍴4 12 ¯16 12 37 ¯43 ¯16 ¯43 98
⍝ =>
3 3⍴2 0 0 6 1 0 ¯8 5 3

⍝ aplcart/table.tsv:1674 — Is ⍺ a subvector of ⍵?; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:355. Local definitions replace the dfns namespace; no namespace feature implied
subvec ← { 0∊⍴⍺:1
  0∊⍴⍵:0
  (1↓⍺)∇(⍵⍳1↑⍺)↓⍵
}
result←'abba' subvec 'babba' 
⍝ =>
1

⍝ aplcart/table.tsv:1676 — Fast multi-digit product using FFT; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:315. Local definitions replace the dfns namespace; no namespace feature implied; Square the accumulator in basedpl left scan
xtimes ← { m←0
  xroots    ← {×\1,1↓(⍵÷2)⍴¯1*2÷⍵}
  cube      ← {⍵⍴⍨2⍴⍨⌊2⍟⍴⍵}
  extend    ← {(2*⌈2⍟¯1+(⍴⍺)+⍴⍵)↑¨⍺ ⍵}
  floop     ← {(⊣/⍺)∇⍣(×m)⊢(+⌿⍵),[m+0.5]⍺×[⍳m←≢⍴⍺]-⌿⍵}
  FFT       ← {,(cube xroots⍴⍵)floop cube ⍵}
  iFFT      ← {(⍴⍵)÷⍨,(cube+xroots⍴⍵)floop cube ⍵}
  rconvolve ← {(¯1+(⍴⍺)+⍴⍵)↑iFFT×/FFT¨⍺ extend ⍵}
  carry     ← {1↓+⌿1 0⌽0,0 10⊤⍵}
  (+/∧\0=t)↓t←carry⍣≡0,⌊0.5+9○⍺ rconvolve ⍵
}
xpower ← {
  xt←{(0,⍺)xtimes 0,⍵} ⋄ b←⌽2⊥⍣¯1+10⊥⍵
  ⊃,/xt/b/{xt⍨⍺}\(⊂,10⊥⍣¯1+⍺)⍴⍨⍴b
}
result←9 3 5 8 1 0 5 xtimes 6 2 3 7 4
⍝ =>
5 8 3 7 0 2 4 4 1 2 7 0

⍝ aplcart/table.tsv:1677 — Fast: X and Y are ignored (sink; no result produced)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X{}Y   ⍝ {}0

⍝ aplcart/table.tsv:1678 — Fast: Y is ignored (sink; no result produced)
Y←3 1 3 2 ⋄ {}Y   ⍝ {}0

⍝ aplcart/table.tsv:1679 — Inverted Table Tally (≢Y where Y is unverted Yv)
Yv← (1 2 3⋄ 3 2⍴'abcdef')  ⋄ ≢⍤↑Yv   ⍝ 3

⍝ aplcart/table.tsv:1680 — Ternary: if Bs then return X else return Y
X←3 1 2 1 ⋄ Bs←0 ⋄ Y←3 1 3 2 ⋄ X⊣⍣Bs⊢Y   ⍝ 3 1 3 2

⍝ aplcart/table.tsv:1691 — Partitioned enclose of Y according (along last axis) beginning enclosures at indices Iv
Iv←1 3 5 ⋄ Y←⍳6 ⋄ Iv⍸⍣¯1⍛⊂Y   ⍝ (1 2 ⋄ 3 4 ⋄ 5 6)

⍝ aplcart/table.tsv:1692 — Count number of trailing elements that are equal between two vectors of equal length
'Cloud' (1⊥=) 'Proud'   ⍝ 3

⍝ aplcart/table.tsv:1693 — Count number of trailing elements that are unequal between two vectors of equal length
'Analyst' (1⊥≠) 'Anatomy'   ⍝ 4

⍝ aplcart/table.tsv:1694 — Insert edge ⍵ in graph ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:39. Local definitions replace the dfns namespace; no namespace feature implied
inslink ← {
  fm to←⍵
  ∪∘to¨@fm⊢⍺
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←g inslink 5 1
⍝ =>
(2 3) 3 (2 4) (1 5) (3 1)

⍝ aplcart/table.tsv:1695 — Remove edge ⍵ from graph ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:40. Local definitions replace the dfns namespace; no namespace feature implied
remlink ← {
  fm to←⍵
  ~∘to¨@fm⊢⍺
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←g remlink 2 3
⍝ =>
(2 3) (⍬) (2 4) (1 5) 3

⍝ aplcart/table.tsv:1696 — Vector substitution; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:369. Local definitions replace the dfns namespace; no namespace feature implied
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
result←(6 7⋄ 666 777)subs 2 3 4⍴⍳12
⍝ =>
2 3 4⍴1 2 3 4 5 666 777 8 9 10 11 12 1 2 3 4 5 666 777 8 9 10 11 12

⍝ aplcart/table.tsv:1698 — Shortest path between ⍵ in graph ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:42. Local definitions replace the dfns namespace; no namespace feature implied
path ← {
  graph(fm to)←⍺ ⍵
  fm{
    ⍺≡⍬:⍬
    ∨/to∊⍺:⍬(⊃∘⍵){
      ⍵<0:⍺
      (⍵,⍺)∇ ⍶ ⍵
    }1↑⍺∩to
    next←graph[,⍺]∩¨⊂⍸⍵=¯2
    back←,/⍺+0×next
    wave←,/next
    (∪wave)∇ back@wave⊢⍵
  }¯2+(⍳⍴⍺)∊fm
}
g ← (2 3 ⋄ 3 ⋄ 2 4 ⋄ 1 5 ⋄ 3)
result←g path 2 1
⍝ =>
2 3 4 1

⍝ aplcart/table.tsv:1701 — Approximate alternative to xutils' ss; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:158. Local definitions replace the dfns namespace; no namespace feature implied
ss ← {
  srce find repl←,¨⍵
  mask←find⍷srce
  prem←(⍴find)↑1
  cvex←(prem,mask)⊂find,srce
  (⍴repl)↓∊{repl,(⍴find)↓⍵}¨cvex
}
result←ss 'Banana' 'an' 'AN' 
⍝ =>
'BANANa'

⍝ aplcart/table.tsv:1703 — Hexadecimal from decimal; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:63. Local definitions replace the dfns namespace; no namespace feature implied
hex ← { 
  ⍺←⊢
  1≠≡,⍵:⍺ ∇¨⍵
  0∊⍵-1+⍵:'Too big'
  n←⍬⍴⍺,2*⌈2⍟2⌈16⍟1+⌈/|⍵
  ↓[1]'0123456789abcdef'[1+(n/16)⊤⍵]
}
dec ← { 
  ⍺←0
  1<⍴⍴⍵:⍺∘∇⍤1⊢⍵
  0≡≢⍵:0
  1≠≡,⍵:⍺ ∇¨⍵
  ws←∊∘(•UCS 9 10 13 32 133 160)
  ws↑⍵:⍺ ∇ 1↓⍵
  ws↑⌽⍵:⍺ ∇ ¯1↓⍵
  ∨/ws ⍵:⍺ ∇¨(1+ws ⍵)⊆⍵
  v←16|¯1+'0123456789abcdef0123456789ABCDEF'⍳⍵
  (16⊥v)-⍺×(8≤↑v)×16*≢v
}
to ← { 
  from step←1 ¯1×-\2↑⍺,⍺+×⍵-⍺
  from+step×¯1+⍳0⌈1+⌊(⍵-from)÷step+step=0
}
result←4 hex 1234 5678
⍝ =>
('04d2' ⋄ '162e')

⍝ aplcart/table.tsv:1704 — Matrix search/replace; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:164. Local definitions replace the dfns namespace; no namespace feature implied
ss ← {
  srce find repl←,¨⍵
  mask←find⍷srce
  prem←(⍴find)↑1
  cvex←(prem,mask)⊂find,srce
  (⍴repl)↓∊{repl,(⍴find)↓⍵}¨cvex
}
ssmat ← {
  cmat find repl←⍵
  ⊃{ss ⍵ find repl}¨↓cmat
}
result←ssmat (3 5⍴⍳15⋄ 7 8 9⋄ 70 80 90)
⍝ =>
3 5⍴1 2 3 4 5 6 70 80 90 10 11 12 13 14 15

⍝ aplcart/table.tsv:1707 — Gauss-Jordan elimination; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:230. Local definitions replace the dfns namespace; no namespace feature implied
gauss_jordan ← { 
  elim←{
    p←(⍺-1)+{⍵⍳⌈/⍵}|(⍺-1)↓⍵[;⍺]
    swap←⊖@⍺ p⊢⍵
    mat←swap[⍺;⍺]÷⍨@⍺⊢swap
    mat-(mat[;⍺]×⍺≠⍳≢⍵) ×⌝ mat[⍺;]
  }
  ⍺←=/⊃⍳⍴⍵
  (⍴⍺)⍴(0 1×⍴⍵)↓elim/(⌽⍳⌊/⍴⍵),⊂⍵,⍺
}
hil ← {÷1+ +⌝ ⍨(⍳⍵)-1}
to ← { 
  from step←1 ¯1×-\2↑⍺,⍺+×⍵-⍺
  from+step×¯1+⍳0⌈1+⌊(⍵-from)÷step+step=0
}
result←gauss_jordan 3 3⍴2 1 0 1 2 1 0 1 2
⍝ =>
3 3⍴0.75 ¯0.4999999999999999 0.2499999999999999 ¯0.4999999999999999 0.9999999999999999 ¯0.4999999999999999 0.25 ¯0.4999999999999999 0.7499999999999999

⍝ aplcart/table.tsv:1708 — Nested vector to lines; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:209. Local definitions replace the dfns namespace; no namespace feature implied
ltov ← {
  ⍺←•UCS 10 13 133
  1↓¨⍺{
    (⍵∊⍺)⊂⍵
  }¯1⌽⍵,(~(¯1↑⍵)∊⍺)/1↑⍺
}
vtol ← {
  ⍺←•UCS 10
  ,/⍵,¨⊂⍺
}
result←0 vtol (1 2 3⋄ 4 5 6⋄ 7 8 9)
⍝ =>
1 2 3 0 4 5 6 0 7 8 9 0

⍝ aplcart/table.tsv:1709 — Justify line-vector to ⍺ columns; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:283. Local definitions replace the dfns namespace; no namespace feature implied
justify ← { 
  segs←{¯1+⍵{(⍵,⍴⍺)-¯1,⍵}¯1+⍸⍵}
  split←{((⍵|⍺)>¯1+⍳⍵)+⌊⍺÷1⌈⍵}
  lf sp←(•UCS 10 32)=⊂⍵
  sizes←segs lf
  ⍺←⌈/sizes
  blanks←segs~(lf∨sp)/sp
  required←blanks+⍺-sizes
  breps←required split¨blanks
  last←1⌈¯1+⍴sizes
  brep←∊(last↑breps),×last↓breps
  ((~sp)+sp\∊brep)\⍵
}
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
result←20 justify 'short line',(•UCS 10),'and another'
⍝ =>
•UCS 115 104 111 114 116 32 32 32 32 32 32 32 32 32 32 32 108 105 110 101 10 97 110 100 32 97 110 111 116 104 101 114

⍝ aplcart/table.tsv:1710 — List ⍺-leaves of nested array; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:120. Local definitions replace the dfns namespace; no namespace feature implied
enlist ← {
  ⍺←0
  ⍺≥¯1+|≡⍵:,⍵
  1↓,/(⊂⊂↑↑⍵),⍺ ∇¨,⍵
}
vecs←(('hello' 'world'⋄ 'bonjour' 'monde')⋄ ('good' 'night'⋄ 'bon' 'soir'))
result←1 enlist vecs
⍝ =>
('hello' ⋄ 'world' ⋄ 'bonjour' ⋄ 'monde' ⋄ 'good' ⋄ 'night' ⋄ 'bon' ⋄ 'soir')

⍝ aplcart/table.tsv:1711 — Minus scan; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:328. Local definitions replace the dfns namespace; no namespace feature implied
mscan ← {
  ⍺←⍬⍴⌽⍳⍴⍴⍵
  +\[⍺]⍵×[⍺](⍺⊃⍴⍵)⍴1,-1
}
result←mscan ⍳10
⍝ =>
1 ¯1 2 ¯2 3 ¯3 4 ¯4 5 ¯5

⍝ aplcart/table.tsv:1712 — Divide scan; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:329. Local definitions replace the dfns namespace; no namespace feature implied
dscan ← {
  ⍺←⍬⍴⌽⍳⍴⍴⍵
  ×\[⍺]⍵*[⍺](⍺⊃⍴⍵)⍴1,-1
}
result←dscan ⍳10
⍝ =>
1 0.5 1.5 0.375 1.875 0.3125 2.1875 0.2734375 2.4609375 0.24609375

⍝ aplcart/table.tsv:1713 — Lines to nested vector; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:204. Local definitions replace the dfns namespace; no namespace feature implied
ltov ← {
  ⍺←•UCS 10 13 133
  1↓¨⍺{
    (⍵∊⍺)⊂⍵
  }¯1⌽⍵,(~(¯1↑⍵)∊⍺)/1↑⍺
}
lvec←{'fooling around', ⍵, 'with barrels', ⍵, 'in alleys'} •ucs 10
result←ltov lvec
⍝ =>
('fooling around' ⋄ 'with barrels' ⋄ 'in alleys')

⍝ aplcart/table.tsv:1715 — Quickest path from/to ⍵ in weighted graph ⍺; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:101. Local definitions replace the dfns namespace; no namespace feature implied
wpath ← {
  graph costs←↓⍺
  fm to←⍵
  tree←¯1⊣¨graph
  cost←0@fm⊢(⍴costs)⍴⌊/⍬
  I←⊃¨∘⊂
  fm{
    acc to←⍵
    to<0:(to=¯2)↓acc
    ⍺ ∇(to,acc)(to⊃⍺)
  }{
    tree cost←⍵
    ⍺≡⍬:tree ⍶ ⍬ to
    adjv←⍺ I graph
    accm←⍺ I cost+costs
    best←adjv I¨⊂cost
    mask←accm<best⌊to⊃cost
    next←mask/¨adjv
    back←,/⍺+0×next
    cvec←,/mask/¨accm
    decr←{(⍒cvec)I ⍵}
    wave←decr,/next
    new←back cvec decr⍨@wave¨⍵
    (∪wave)∇ new
  }tree cost
}
md1←,¨⊃((2 3) 4 4 ⍬ ⋄ (2 3) 1 2 ⍬)
result←md1 wpath 1 4
⍝ =>
1 2 4

⍝ aplcart/table.tsv:1718 — Drop All Blanks; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:320. Local definitions replace the dfns namespace; no namespace feature implied
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
dab←{
  ⍺←' ' ⋄ 1<≡⍵:(⊂⍺)∇¨⍵
  1≥⍴⍴⍵:⍵~⍺
  2=⍴⍴⍵:⊃(↓⍵)~¨⊂⍺
  (¯1↓⍴⍵){(⍺,1↓⍴⍵)⍴⍵}⍺ ∇,[¯1↓⍳⍴⍴⍵]⍵
}
show←' ·'∘subs
cvec←'  twas  ever  thus  '
result←dab cvec
⍝ =>
'twaseverthus'

⍝ aplcart/table.tsv:1719 — Drop Multiple Blanks; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:318. Local definitions replace the dfns namespace; no namespace feature implied
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
dmb ← {
  ⍺←' ' ⋄ 1<|≡⍵:(⊂⍺)∇¨⍵
  2<⍴⍴⍵:(¯1↓⍴⍵){(⍺,1↓⍴⍵)⍴⍵}⍺ ∇,[¯1↓⍳⍴⍴⍵]⍵
  2>⍴⍴⍵:((∨/∘(2∘↕))(~⍵∊⍺),1)/⍵
  ((∨/∘(2∘↕))(,∨⌿~⍵∊⍺),1)/⍵
}
show←' ·'∘subs
cvec←'  twas  ever  thus  '
result←dmb cvec
⍝ =>
' twas ever thus '

⍝ aplcart/table.tsv:1720 — Drop Ending Blanks; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:317. Local definitions replace the dfns namespace; no namespace feature implied
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
deb ← {
  ⍺←' ' ⋄ 1<|≡⍵:(⊂⍺)∇¨⍵
  2<⍴⍴⍵:(¯1↓⍴⍵){(⍺,1↓⍴⍵)⍴⍵}⍺ ∇,[¯1↓⍳⍴⍴⍵]⍵
  b←⍵∊⍺
  1≥⍴⍴⍵:((∧\b)⍱⌽∧\⌽b)/⍵
  b←∧⌿b ⋄ ((∧\b)⍱⌽∧\⌽b)/⍵
}
show←' ·'∘subs
cvec←'  twas  ever  thus  '
result←deb cvec
⍝ =>
'twas  ever  thus'

⍝ aplcart/table.tsv:1721 — Drop Leading Blanks; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:315. Local definitions replace the dfns namespace; no namespace feature implied
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
dlb ← {
  ⍺←' ' ⋄ 1<|≡⍵:(⊂⍺)∇¨⍵
  2<⍴⍴⍵:(¯1↓⍴⍵){(⍺,1↓⍴⍵)⍴⍵}⍺ ∇,[¯1↓⍳⍴⍴⍵]⍵
  1≥⍴⍴⍵:(+/∧\⍵∊⍺)↓⍵
  (∨\∨⌿~⍵∊⍺)/⍵
}
show←' ·'∘subs
cvec←'  twas  ever  thus  '
result←dlb cvec
⍝ =>
'twas  ever  thus  '

⍝ aplcart/table.tsv:1722 — Drop Trailing Blanks; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:316. Local definitions replace the dfns namespace; no namespace feature implied
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
dtb ← {
  ⍺←' ' ⋄ 1<|≡⍵:(⊂⍺)∇¨⍵
  2<⍴⍴⍵:(¯1↓⍴⍵){(⍺,1↓⍴⍵)⍴⍵}⍺ ∇,[¯1↓⍳⍴⍴⍵]⍵
  1≥⍴⍴⍵:(-+/∧\⌽⍵∊⍺)↓⍵
  (~⌽∧\⌽∧⌿⍵∊⍺)/⍵
}
show←' ·'∘subs
cvec←'  twas  ever  thus  '
result←dtb cvec
⍝ =>
'  twas  ever  thus'

⍝ aplcart/table.tsv:1723 — Drop eXtraneous Blanks; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:319. Local definitions replace the dfns namespace; no namespace feature implied
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
dxb ← {
  ⍺←' ' ⋄ 1<|≡⍵:(⊂⍺)∇¨⍵
  2<⍴⍴⍵:(¯1↓⍴⍵){(⍺,1↓⍴⍵)⍴⍵}⍺ ∇,[¯1↓⍳⍴⍴⍵]⍵
  b←⍵∊⍺
  1≥⍴⍴⍵:(1↑b)↓(b⍲1↓b,1)/⍵
  b←∧⌿b ⋄ (0,1↑b)↓(b⍲1↓b,1)/⍵
}
show←' ·'∘subs
cvec←'  twas  ever  thus  '
result←dxb cvec
⍝ =>
'twas ever thus'

⍝ aplcart/table.tsv:1724 — Array from TreeView style tree; Concrete APLcart library call; self-contained setup from april/libraries/dfns/tree/demo.lisp:919. Local definitions replace the dfns namespace; no namespace feature implied
disp ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t} ⋄  ⋄ ⍺←⍬
  dec ctd←2↑⍺
  box←{
    isor ⍵:format⊂⍵
    1=≡,⍵:dec open format dec open ⍵
    mat←matr 1/dec open ⍵
    r c←×⍴mat
    dec<0∊r c:c/r⌿∇ 1 open mat
    subs←aligned ∇¨mat
    (≢⍴⍵)gaps ⍵ plane subs
  }
  aligned←{
    rows cols←sepr⍴¨⍵
    sizes←(⌈/rows) ,⌝ ⌈⌿cols
    ctd=0:sizes↑¨⍵
    v h←sepr⌈0.5×⊃(⍴¨⍵)-sizes
    v⊖¨h⌽¨sizes↑¨⍵
  }
  gaps←{
    ⍺≤2:⍵
    subs←(⍺-1)∇¨⍵
    width←↑⌽⍴↑subs
    fill←(⍺ width-3 0)⍴' '
    ⊃{⍺⍪fill⍪⍵}/1 open subs
  }
  plane←{
    2<⍴⍴⍺:⍺ join ⍵
    odec←(dec shape ⍺)outer ⍵
    idec←inner ⍺
    (odec,idec)collect ⍵
  }
  join←{
    sep←(≢⍵)÷1⌈≢⍺
    split←(0=sep|¯1+⍳≢⍵)⊂[1]⍵
    (⊂⍤¯1⊢⍺)plane¨split
  }
  outer←{
    sizes←1 0{↑↓(⍉⍣⍺)⍵}¨sepr⍴¨⍵
    sides←sizes/¨¨'│─'
    bords←dec↓¨'├┬'glue¨sides
    ,¨/('┌' '')⍺ bords'└┐'
  }
  inner←{
    deco←{(type ⍵),1 shape ⍵}
    sepr deco¨matr dec open ⍵
  }
  collect←{
    lft top tt vv hh←⍺
    cells←vv right 1 open tt hh lower ⍵
    boxes←(dec∨0∊⍴⍵)open cells
    lft,top⍪⍪⌿,/boxes
  }
  right←{
    types←2⊥¨(⍳⍴⍵)=⊂⍴⍵
    chars←'┼┤┴┘'[1+types]
    rgt←{⍵,(-≢⍵)↑(≢⍵)1 1/'│',⍺}
    ((matr 1 open ⍺),¨chars)rgt¨⍵
  }
  lower←{
    split←{((¯2+2⊃⍴⍵)/'─')glue ⍺}
    bot←{⍵⍪(-2⊃⍴⍵)↑⍺ split ⍵}
    (matr,¨/⍺)bot¨matr ⍵
  }
  type←{
    dec<|≡⍵:'─'
    isor ⍵:'∇'
    sst←{
      0=dec×⍴⍴⍵:'─'
      (1+↑⍵∊'¯',•D)⊃'#~'
    }∘⍕
    0=≡⍵:sst ⍵
    {(1+1=⍴⍵)⊃'+'⍵}∪,sst¨dec open ⍵
  }
  shape←{     
    dec≤0=⍴⍴⍵:⍺/¨'│─'
    cols←(1+×¯1↑⍴⍵)⊃'⊖→'
    rsig←(1+××/¯1↓⍴⍵)⊃'⌽↓'
    rows←(3⌊⍴⍴⍵)⊃'│'rsig'⍒'
    rows cols
  }
  matr←{⊃,↓⍵}
  sepr←{+/¨1⊂⊃⍵}
  open←{(⍺⌈⍴⍵)↑⍵}
  isor←{1 ⍬≡(≡⍵)(⍴⍵)}
  glue←{0=⍴⍵ : ⍵ ⋄ ⍺{⍺,⍶,⍵}/⍵}
  isor ⍵:format⊂⍵
  1=≡,⍵:format ⍵
  box ⍵
}
tnest ← {
  dvec ivec←⍵
  1=≢dvec:↑ivec
  node←1↑ivec
  dsub isub←(1=dvec)∘⊂¨⍵
  node,∇¨↓⍉⊃(dsub-1)isub
}
result←tnest (0 1 1⋄ 'hot' 'tea' 'coffee')
⍝ =>
('hot' ⋄ 'tea' ⋄ 'coffee')

⍝ aplcart/table.tsv:1726 — TreeView style from nested array; Concrete APLcart library call; self-contained setup from april/libraries/dfns/tree/demo.lisp:963. Local definitions replace the dfns namespace; no namespace feature implied
disp ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t} ⋄  ⋄ ⍺←⍬
  dec ctd←2↑⍺
  box←{
    isor ⍵:format⊂⍵
    1=≡,⍵:dec open format dec open ⍵
    mat←matr 1/dec open ⍵
    r c←×⍴mat
    dec<0∊r c:c/r⌿∇ 1 open mat
    subs←aligned ∇¨mat
    (≢⍴⍵)gaps ⍵ plane subs
  }
  aligned←{
    rows cols←sepr⍴¨⍵
    sizes←(⌈/rows) ,⌝ ⌈⌿cols
    ctd=0:sizes↑¨⍵
    v h←sepr⌈0.5×⊃(⍴¨⍵)-sizes
    v⊖¨h⌽¨sizes↑¨⍵
  }
  gaps←{
    ⍺≤2:⍵
    subs←(⍺-1)∇¨⍵
    width←↑⌽⍴↑subs
    fill←(⍺ width-3 0)⍴' '
    ⊃{⍺⍪fill⍪⍵}/1 open subs
  }
  plane←{
    2<⍴⍴⍺:⍺ join ⍵
    odec←(dec shape ⍺)outer ⍵
    idec←inner ⍺
    (odec,idec)collect ⍵
  }
  join←{
    sep←(≢⍵)÷1⌈≢⍺
    split←(0=sep|¯1+⍳≢⍵)⊂[1]⍵
    (⊂⍤¯1⊢⍺)plane¨split
  }
  outer←{
    sizes←1 0{↑↓(⍉⍣⍺)⍵}¨sepr⍴¨⍵
    sides←sizes/¨¨'│─'
    bords←dec↓¨'├┬'glue¨sides
    ,¨/('┌' '')⍺ bords'└┐'
  }
  inner←{
    deco←{(type ⍵),1 shape ⍵}
    sepr deco¨matr dec open ⍵
  }
  collect←{
    lft top tt vv hh←⍺
    cells←vv right 1 open tt hh lower ⍵
    boxes←(dec∨0∊⍴⍵)open cells
    lft,top⍪⍪⌿,/boxes
  }
  right←{
    types←2⊥¨(⍳⍴⍵)=⊂⍴⍵
    chars←'┼┤┴┘'[1+types]
    rgt←{⍵,(-≢⍵)↑(≢⍵)1 1/'│',⍺}
    ((matr 1 open ⍺),¨chars)rgt¨⍵
  }
  lower←{
    split←{((¯2+2⊃⍴⍵)/'─')glue ⍺}
    bot←{⍵⍪(-2⊃⍴⍵)↑⍺ split ⍵}
    (matr,¨/⍺)bot¨matr ⍵
  }
  type←{
    dec<|≡⍵:'─'
    isor ⍵:'∇'
    sst←{
      0=dec×⍴⍴⍵:'─'
      (1+↑⍵∊'¯',•D)⊃'#~'
    }∘⍕
    0=≡⍵:sst ⍵
    {(1+1=⍴⍵)⊃'+'⍵}∪,sst¨dec open ⍵
  }
  shape←{     
    dec≤0=⍴⍴⍵:⍺/¨'│─'
    cols←(1+×¯1↑⍴⍵)⊃'⊖→'
    rsig←(1+××/¯1↓⍴⍵)⊃'⌽↓'
    rows←(3⌊⍴⍴⍵)⊃'│'rsig'⍒'
    rows cols
  }
  matr←{⊃,↓⍵}
  sepr←{+/¨1⊂⊃⍵}
  open←{(⍺⌈⍴⍵)↑⍵}
  isor←{1 ⍬≡(≡⍵)(⍴⍵)}
  glue←{0=⍴⍵ : ⍵ ⋄ ⍺{⍺,⍶,⍵}/⍵}
  isor ⍵:format⊂⍵
  1=≡,⍵:format ⍵
  box ⍵
}
tview ← {
  ⍺←0
  1=≡,⍵:⍺,⊂,⊂⍵
  node←⍺,⊂1↑⍵
  subs←(⍺+1)∇¨1↓⍵
  ,⌿node⍪⊃subs
}
result←tview 'hot' 'tea' 'coffee' 
⍝ =>
(0 1 1 ⋄ ('hot' ⋄ 'tea' ⋄ 'coffee'))

⍝ aplcart/table.tsv:1730 — Fast: X and Y as a two item vector (X Y)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X{⍺⍵}Y   ⍝ (3 1 2 1 ⋄ 3 1 3 2)

⍝ aplcart/table.tsv:1731 — Fast: Y sorted into descending order
Y←3 1 3 2 ⋄ ⊂⍤⍒⍛⌷Y   ⍝ 3 3 2 1

⍝ aplcart/table.tsv:1733 — Strongly connected components of directed graph ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/graph/demo.lisp:64. Local definitions replace the dfns namespace; no namespace feature implied; First-true masks use cumulative counts under basedpl left scan
scc ← {
  TT←(3/⊂0⊣¨G←⍵),1 ⍬
  C L X x S←⍳⍴TT
  put←{(⍹⊃⍵)@(⊂⍶ ⍺)⊢⍵}
  Lx←L put x
  Xx←X put x
  x1←{1+@x⊢⍵}
  push←,@S
  ⍺←0 ⋄ trace←{⍵⊣⎕←0 dsp ⍺,⍵}⍣(⍺≢0)
  comp←{ v←⍺
    pops←1++/∧\v≠stk←S⊃⍵
    C∆←((1+⌈/C⊃⍵))@(pops↑stk)⊢C⊃⍵
    ((pops↓stk)C∆)@S C⊢⍵
  }
  conn←{ v←⍺
    T0←v trace ⍵
    T1←x1 v push v Lx v Xx T0
    T2←{w←⍺
      min_L←{(w⊃⍺⊃⍵)⌊@(⊂L v)⊢⍵}
      0=w⊃X⊃⍵:L min_L w conn ⍵
      X min_L⍣(w∊S⊃⍵)⊢⍵
    }/(⌽v⊃G),⊂T1
    root←(v⊃L⊃T2)=v⊃X⊃T2
    v comp⍣root⊢T2
  }
  loop←{
    vert←{⍺ conn⍣(0=⍺⊃X⊃⍵)⊢⍵}
    vert/(⌽⍳⍴G),⊂⍵
  }
  (∪⍳⊢)C⊃loop TT
}
X ← {
  ⍺←1∨.∨⍵
  x←⍳⍴⍺
  d←(x~⍺/x) =⌝ x
  z←{
    r c←⍴⍵
    c=0:r⍴0
    n←+⌿⍵
    f←({⍵∧1=+\⍵}n=⌊/n)/⍵
    ⍵ ∇{
      ~1∊⍵:0
      f←{⍵∧1=+\⍵}⍵
      c←,f⌿⍺
      r←∨/c/⍺
      s←⍶(~c)/(~r)⌿⍺
      s≡0:⍺ ∇ f<⍵
      f∨(~r)\s
    },f
  }⍵⍪d
  z≡0:0
  (-+/~⍺)↓z
}
scg1 ← ,¨1(2 4 5)(3 6)(2 7)(0 5)6 5(3 6)
result←scc scg1+1
⍝ =>
1 1 2 2 1 3 3 2

⍝ aplcart/table.tsv:1734 — Huntington-Hill apportionment; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:23. Local definitions replace the dfns namespace; no namespace feature implied
apportion ← {
  ⍺←435
  ⍵{
    d←(⍵×⍵+1)*0.5
    cs←⍺÷d
    ⍵+cs=⌈/cs
  }⍣(⍺-≢⍵),1
}
p1790 ← 236841 55540 70835 68705 278514 475327 141822 179570 331589 353523 432879 68446 206236 85533 630560
result←105 apportion p1790
⍝ =>
7 2 2 2 8 14 4 5 10 10 12 2 6 3 18

⍝ aplcart/table.tsv:1737 — Sum of (default decimal) digit columns with carry; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:30. Local definitions replace the dfns namespace; no namespace feature implied
colsum ← {
  ⍺←10 ⋄ ⍺{{(0=⍬⍴⍵)↓⍵}+⌿1 0⌽0,0 ⍺⊤⍵}⍣≡+⌿⍵
}
result←colsum 10 10⍴⍳9
⍝ =>
5 1 2 3 4 5 6 7 8 8 6

⍝ aplcart/table.tsv:1738 — Expand/compress HT chars; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:306. Local definitions replace the dfns namespace; no namespace feature implied
xtabs ← { 
  ⍺←8
  ⍺=0:⍵
  chs←~⍵∊•UCS 10 13 133
  ⍺>0:⍺{
    tabs nabs←1 0=⊂⍵∊•UCS 9
    sync←tabs≥chs
    segs←¯1+{⍵-¯1,¯1↓⍵}¯1+⍸sync
    pads←0⌈⍺-⍺|(sync/tabs)/segs
    (nabs+tabs\pads)/nabs\nabs/⍵
  }⍵
  ⍺<0:(-⍺){
    bks nks←1 0=⊂⍵=' '
    runs←{⍵{⍵-⌈\⍵×~⍺}+\⍵}
    tabs←bks∧chs∧0=⍺|runs chs
    onoff←{(⍺≠⍵){≠\⍺\(≠/∘(2∘↕))¯1,⍺/⍵}⍺-⍵}
    pretab←⌽(⌽tabs)onoff⌽nks
    (pretab≤tabs)/(•UCS 9)@{tabs}⍵
  }⍵
}
subs ← {
  fs ts←≢¨fm to←⍺
  1≡≡⍺:to@(fm∘=)⍵
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1
  (fm⍷⍵){
    ~1∊⍺:⍵
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵
}
tabText←'whistles        far     and wee'
result←¯8 xtabs tabText
⍝ =>
•UCS 119 104 105 115 116 108 101 115 9 102 97 114 9 97 110 100 32 119 101 101

⍝ aplcart/table.tsv:1741 — Reduced version of disp; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:102. Local definitions replace the dfns namespace; no namespace feature implied
dsp ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t}
  (1=≡,⍵)∨0∊⍴⍵:format ⍵
  ⍺←1 ⋄ top←'─'∘⍪⍣⍺
  1≥⍴⍴⍵:{
    bars←{⍪(⌊/≢¨⍺ ⍵)/'│'}/2↕⍵,0
    join←{⊃,/(⌈/≢¨⍵)↑¨⍵}
    0 ¯1↓join top¨join¨↓⍉⊃⍵ bars
  }1 ∇¨⍵
  subs←⍺ ∇¨⍵
  rs cs←+/¨1⊂⊃⍴¨subs
  dims←(mrs←⌈/rs) ,⌝ mcs←⌈/⍪⍉cs
  join←{⍺{⍺,⍶,⍵}/⍵}
  rows←(mrs/¨'│')join¨↓dims↑¨subs
  hzs←'┼'join mcs/¨'─'
  cells←{⍺⍪hzs⍪⍵}/rows
  gaps←(⌽⍳¯2+⍴⍴⍵)/¨' '
  cjoin←{⍪/(⊂⍺),⍶,⊂⍵}
  top{⍺ cjoin⌿⍵}/gaps,⊂cells
}
result←dsp 'hello' 'world' 
⍝ =>
2 11⍴'───────────hello│world'

⍝ aplcart/table.tsv:1758 — Postage stamps for ⍵; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:164. Local definitions replace the dfns namespace; no namespace feature implied
path ← {
  graph(fm to)←⍺ ⍵
  fm{
    ⍺≡⍬:⍬
    ∨/to∊⍺:⍬(⊃∘⍵){
      ⍵<0:⍺
      (⍵,⍺)∇ ⍶ ⍵
    }1↑⍺∩to
    next←graph[,⍺]∩¨⊂⍸⍵=¯2
    back←,/⍺+0×next
    wave←,/next
    (∪wave)∇ back@wave⊢⍵
  }¯2+(⍳⍴⍺)∊fm
}
stamps ← {
  ⍺←1 5 6 10 26 39 43
  graph←⍺{⍵∘∩¨⍵+⊂⍺}⍳⍵+|⌊/⍺
  spath←graph path 1+0 ⍵
  (-/∘⌽∘(2∘↕))spath
}
result←stamps 53
⍝ =>
43 10

⍝ aplcart/table.tsv:1759 — ⍺'th root; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:208. Local definitions replace the dfns namespace; no namespace feature implied
cfract ← {
  ,{
    ⍵=1:⍺
    n r←0 ⍵⊤⍺
    n,⍵∇r
  }/⌊1E¯14+⍵ 1÷1∨⍵
}
root ← { ⍺←2 ⋄ ⍵*÷⍺ }
result←root 5
⍝ =>
2.23606797749979

⍝ aplcart/table.tsv:1760 — Decimal from hexadecimal; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:95. Local definitions replace the dfns namespace; no namespace feature implied
hex ← { 
  ⍺←⊢
  1≠≡,⍵:⍺ ∇¨⍵
  0∊⍵-1+⍵:'Too big'
  n←⍬⍴⍺,2*⌈2⍟2⌈16⍟1+⌈/|⍵
  ↓[1]'0123456789abcdef'[1+(n/16)⊤⍵]
}
dec ← { 
  ⍺←0
  1<⍴⍴⍵:⍺∘∇⍤1⊢⍵
  0≡≢⍵:0
  1≠≡,⍵:⍺ ∇¨⍵
  ws←∊∘(•UCS 9 10 13 32 133 160)
  ws↑⍵:⍺ ∇ 1↓⍵
  ws↑⌽⍵:⍺ ∇ ¯1↓⍵
  ∨/ws ⍵:⍺ ∇¨(1+ws ⍵)⊆⍵
  v←16|¯1+'0123456789abcdef0123456789ABCDEF'⍳⍵
  (16⊥v)-⍺×(8≤↑v)×16*≢v
}
to ← { 
  from step←1 ¯1×-\2↑⍺,⍺+×⍵-⍺
  from+step×¯1+⍳0⌈1+⌊(⍵-from)÷step+step=0
}
result←dec 'DEAD' 'beef' 
⍝ =>
57005 48879

⍝ aplcart/table.tsv:1761 — Relationship between point and k-ball; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:251. Local definitions replace the dfns namespace; no namespace feature implied
kball ← { ⍺←1
  r←↑⍺ ⋄ p←1/⍵
  c←(≢p)↑1↓⍺
  ×-/(⍉p-[1]c)r+.*¨2
}
result←2 1 kball 1 3⍴2 3 4
⍝ =>
¯1 0 1

⍝ aplcart/table.tsv:1763 — Justify text array; Concrete APLcart library call; self-contained setup from april/libraries/dfns/string/demo.lisp:72. Local definitions replace the dfns namespace; no namespace feature implied
display ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t} ⋄ 
  box←{
    vrt hrz←(¯1+⍴⍵)⍴¨'│─'
    top←'─⊖→'[1+¯1↑⍺],hrz
    bot←(↑⍺),hrz
    rgt←'┐│',vrt,'┘'
    lax←'│⌽↓'[1+¯1↓1↓⍺],¨⊂vrt
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪⍵⍪bot),rgt
  }
  deco←{⍺←type open ⍵ ⋄ ⍺,axes ⍵}
  axes←{(-2⌈⍴⍴⍵)↑1+×⍴⍵}
  open←{(1⌈⍴⍵)⍴⍵}
  trim←{(~1 1⍷∧⌿⍵=' ')/⍵}
  char←{⍬≡⍴⍵:'─' ⋄ (1+↑⍵∊'¯',•D)⊃'#~'}∘⍕
  type←{{(1+1=⍴⍵)⊃'+'⍵}∪,char¨⍵}
  line←{(1+''≡0⍴⍵)⊃' -'}
  { 
    0=≡⍵:' '⍪(open format ⍵)⍪line ⍵
    1 ⍬≡(≡⍵)(⍴⍵):'∇' 0 0 box format ⍵
    1=≡⍵:(deco ⍵)box open format open ⍵
    ('∊'deco ⍵)box trim format ∇¨open ⍵
  }⍵
}
just ← {
  ⍺←¯1
  ⍺=¯1: ( +/∧\' '= ⍵)            ⌽⍵
  ⍺= 1: (-+/∧\' '=⌽⍵)            ⌽⍵
  (⌈0.5×( +/∧\' '= ⍵)-+/∧\' '=⌽⍵)⌽⍵
}
result←0 just '  hello world  ' 
⍝ =>
'  hello world  '

⍝ aplcart/table.tsv:1767 — Polar from/to cartesian coordinates; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:303. Local definitions replace the dfns namespace; no namespace feature implied
polar ← {
  pol_car←{
    radius←{(+⌿⍵*2)*0.5}
    angle←{
      x y←⊂⍤¯1⊢⍵
      x0 xn←1 0=⊂0=x
      atan←¯3○y÷x+x0
      qne←(xn×atan)+x0×π0.5×2-×y
      nsw←πx<0
      qse←π2×(x>0)∧y<0
      nsw+qse+qne
    }
    (radius ⍵)lam angle ⍵
  }
  car_pol←{
    r o←⊂⍤¯1⊢⍵
    (r×2○o)lam r×1○o
  }
  lam←,[1-÷2]
  ⍺←1
  ⍺=+1:pol_car ⍵
  ⍺=-1:car_pol ⍵
}
result←polar 2 3⍴3 0 ¯3 4 1 4
⍝ =>
2 3⍴5 1 5 0.9272952180016122 1.570796326794897 2.214297435588181

⍝ aplcart/table.tsv:1769 — Changing an index origin dependent argument to act as ⎕IO=0; Pure glyph recipe; quoted quad is data or origin lookup is replaced with fixed one; Supply small operands and use fixed origin one where the recipe reads index origin; Quoted quad remains character data
J←¯1 0 2 ⋄ {1+⍵}J   ⍝ 0 1 3

⍝ aplcart/table.tsv:1772 — Ternary: if Bs then execute and return X else execute and return Y
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ Bs←1 ⋄ {⍵:X ⋄ Y}Bs
3 1 2 1

⍝ aplcart/table.tsv:1773 — Two-row matrix from two vectors (pad shorter vector)
Xv←1 2 ⋄ Yv←3 4 5 ⋄ Xv{⊃⍺⍵}Yv   ⍝ 2 3⍴1 2 0 3 4 5

⍝ aplcart/table.tsv:1774 — Integer quotient and remainder (new leading length-2 axis) of N divided by Ms
3 {0 ⍺⊤⍵} 10 11 12 13   ⍝ 2 4⍴3 3 4 4 1 2 0 1

⍝ aplcart/table.tsv:1777 — Matrix Trace: Sum of main diagonal
(A ← 3 3⍴⍳9 ⋄ (+/1 1∘⍉) A)   ⍝ (3 3⍴1 2 3 4 5 6 7 8 9) 15

⍝ aplcart/table.tsv:1788 — Changing an index origin dependent argument to act as ⎕IO=1; Pure glyph recipe; quoted quad is data or origin lookup is replaced with fixed one; Supply small operands and use fixed origin one where the recipe reads index origin; Quoted quad remains character data
J←1 2 3 ⋄ {⍵+1-1}J   ⍝ 1 2 3

⍝ aplcart/table.tsv:1789 — Changing an index origin dependent result to be as ⎕IO=0; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1. Specialized ⎕IO to the fixed origin 1
{-1-⍵}¯1 0 1 4   ⍝ ¯2 ¯1 0 3

⍝ aplcart/table.tsv:1790 — Changing an index origin dependent result to be as ⎕IO=1; Pure glyph recipe; quoted quad is data or origin lookup is replaced with fixed one; Supply small operands and use fixed origin one where the recipe reads index origin; Quoted quad remains character data
J←1 2 3 ⋄ {⍵+~1}J   ⍝ 1 2 3

⍝ aplcart/table.tsv:1792 — Exact cover: Knuth's Algorithm X; Concrete APLcart recipe; local definitions from april/libraries/dfns/graph/demo.lisp:83 replace the dfns namespace; Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1; First-true masks use cumulative counts under basedpl left scan
X ← {
  ⍺←1∨.∨⍵
  x←⍳⍴⍺
  d←(x~⍺/x) =⌝ x
  z←{
    r c←⍴⍵
    c=0:r⍴0
    n←+⌿⍵
    f←({⍵∧1=+\⍵}n=⌊/n)/⍵
    ⍵ ∇{
      ~1∊⍵:0
      f←{⍵∧1=+\⍵}⍵
      c←,f⌿⍺
      r←∨/c/⍺
      s←⍶(~c)/(~r)⌿⍺
      s≡0:⍺ ∇ f<⍵
      f∨(~r)\s
    },f
  }⍵⍪d
  z≡0:0
  (-+/~⍺)↓z
}
M←6 7⍴1 0 0 1 0 0 1 1 0 0 1 0 0 0 0 0 0 1 1 0 1 0 0 1 0 1 1 0 0 1 1 0 0 1 1 0 1 0 0 0 0 1
result←X M
⍝ =>
0 1 0 1 0 1

⍝ aplcart/table.tsv:1799 — Xs-separated vector constructed from the vector of vectors Yv (which must be of depth 2)
Xs← ','  ⋄ Yv← 'ab' '' 'cd'  ⋄ Xs(1↓∘∊,¨)Yv
'ab,,cd'

⍝ aplcart/table.tsv:1800 — Is Nm traceless?
(A ← 3 3⍴⍳9 ⋄ (0∧.=1 1∘⍉) A ⋄ B ← 3 3⍴0 1 2 3 ⋄ (0∧.=1 1∘⍉) B)
(3 3⍴1 2 3 4 5 6 7 8 9) 0 (3 3⍴0 1 2 3 0 1 2 3 0) 1

⍝ aplcart/table.tsv:1801 — Reverse Y on condition As
As←1 ⋄ Y←2 3⍴⍳6 ⋄ As{⊖⍣⍺⊢⍵}Y   ⍝ 2 3⍴4 5 6 1 2 3

⍝ aplcart/table.tsv:1802 — Is X lexically less than or equal to Y?
('Anna' {</⍋⍺⍵} 'Bob' ⋄ 'Anna' {</⍋⍺⍵} 'Anna' ⋄ 'Bob' {</⍋⍺⍵} 'Anna' ⋄ 'Anna ' {</⍋⍺⍵} 'Anna')
1 1 0 0

⍝ aplcart/table.tsv:1803 — Is X lexically greater than Y?
('Anna' {>/⍋⍺⍵} 'Bob' ⋄ 'Anna' {>/⍋⍺⍵} 'Anna' ⋄ 'Bob' {>/⍋⍺⍵} 'Anna' ⋄ 'Anna' {>/⍋⍺⍵} 'Anna ')
0 0 1 0

⍝ aplcart/table.tsv:1804 — Is X lexically greater than or equal to Y?
('Anna' {</⍒⍺⍵} 'Bob' ⋄ 'Anna' {</⍒⍺⍵} 'Anna' ⋄ 'Bob' {</⍒⍺⍵} 'Anna' ⋄ 'Anna ' {</⍒⍺⍵} 'Anna')
0 1 1 1

⍝ aplcart/table.tsv:1805 — Is X lexically less than Y?
('Anna' {>/⍒⍺⍵} 'Bob' ⋄ 'Anna' {>/⍒⍺⍵} 'Anna' ⋄ 'Bob' {>/⍒⍺⍵} 'Anna' ⋄ 'Anna' {>/⍒⍺⍵} 'Anna ')
1 0 0 1

⍝ aplcart/table.tsv:1806 — Indexing two-element vector Xv with Boolean values B
Xv← 'no' 'yes'  ⋄ B←0 1 1 0 ⋄ Xv{⍺[1+⍵]}B
('no' ⋄ 'yes' ⋄ 'yes' ⋄ 'no')

⍝ aplcart/table.tsv:1807 — Integer quotient and remainder (new leading length-2 axis) of all combinations of an element from N divided by an element from M
2 3 4 5 6 {⍵⊤⍨⊃⍬⍺} 10   ⍝ 2 5⍴5 3 2 2 1 0 1 2 0 4

⍝ aplcart/table.tsv:1812 — Replacing major cells of Y not satisfying Bv with prototypical cells
(1 1 0 1 1 1 0 1 {⍺⍀⍺⌿⍵} 'Amazing!' ⋄ 1 1 0 {⍺⍀⍺⌿⍵} (3 1 4⋄ 2 7 18⋄ 1 6 18))
('Am zin !' ⋄ (3 1 4 ⋄ 2 7 18 ⋄ 0 0 0))

⍝ aplcart/table.tsv:1813 — Count number of leading elements that are equal between two vectors of equal length
Xv←1 2 4 5 ⋄ Yv←1 2 3 5 ⋄ Xv(+/∧\⍤=)Yv   ⍝ 2

⍝ aplcart/table.tsv:1814 — Count number of leading elements that are unequal between two vectors of equal length
Xv←1 2 4 5 ⋄ Yv←0 3 4 5 ⋄ Xv(+/∧\⍤≠)Yv   ⍝ 2

⍝ aplcart/table.tsv:1816 — Determinant of two-row matrix
Nm←2 2⍴1 2 3 4 ⋄ {-/×⌿0 1⌽⍵}Nm   ⍝ ¯2

⍝ aplcart/table.tsv:1817 — Inverted Table Transpose (⍉Y where Y is unverted Yv)
Yv← (1 2 3⋄ 3 2⍴'abcdef')  ⋄ {⍉⊃⊂⍤¯1¨⍵}Yv
3 2⍴1 ('ab') 2 ('cd') 3 ('ef')

⍝ aplcart/table.tsv:1818 — Bit-wise NOT for positive integers
J←1 2 3 7 8 ⋄ {2⊥~2⊥⍣¯1⊢⍵}J   ⍝ 14 13 12 8 7

⍝ aplcart/table.tsv:1819 — Alternating sequence of Jv[1] ones, Jv[2] zeros, Jv[3] ones, …
Jv←1 2 3 ⋄ {⍵/1 0⍴⍨≢⍵}Jv   ⍝ 1 0 0 1 1 1

⍝ aplcart/table.tsv:1820 — Locate fill elements formed by replicating Y by Iv
Iv←2 ¯1 1 ⋄ Y←10 20 30 ⋄ Iv{~⍺⌿1⍨¨⍵}Y   ⍝ 0 0 1 0

⍝ aplcart/table.tsv:1821 — First Js figurate numbers of order Is
Is←2 ⋄ Js←5 ⋄ Is{+\⍣⍺⍳⍵}Js   ⍝ 1 4 10 20 35

⍝ aplcart/table.tsv:1822 — Locate fill elements formed by expanding Y by Iv
Iv←1 0 1 0 1 ⋄ Y←10 20 30 ⋄ Iv{~⍺⍀1⍨¨⍵}Y
0 1 0 1 0

⍝ aplcart/table.tsv:1823 — Find with wildcards; Concrete APLcart recipe; local definitions from april/libraries/dfns/array/demo.lisp:201 replace the dfns namespace; Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
match ← {
  p x←{⍵'*'}⍣(1=≡,⍺),⍺
  v←1↓¨{(x∘≡¨⍵)⊂⍵}(⊂x),p
  h←⊃v⍷¨⊂⍵
  r←0,¯1↓,⊃⍴¨v
  sl←{
    a←¯1↓⍴⍵
    x←⌈/⍺
    p←⍵,(a,x)⍴0
    s←⍉a⍴⍺
    (-(0×a),x)↓s⌽p
  }
  m←⌽∨\⌽r sl h
  ↑{
    (lh lm)(und rm)←⍺ ⍵
    (lh∧rm)lm
  }/↓⍉⊃⊂⍤¯1¨h m
}
result←'a*b*d' match 'aaaabbbccd' 
⍝ =>
1 1 1 1 0 0 0 0 0 0

⍝ aplcart/table.tsv:1825 — Boxed sketch of nested array; Concrete APLcart recipe; local definitions from april/libraries/dfns/array/demo.lisp:24 replace the dfns namespace; Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
disp ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t} ⋄  ⋄ ⍺←⍬
  dec ctd←2↑⍺
  box←{
    isor ⍵:format⊂⍵
    1=≡,⍵:dec open format dec open ⍵
    mat←matr 1/dec open ⍵
    r c←×⍴mat
    dec<0∊r c:c/r⌿∇ 1 open mat
    subs←aligned ∇¨mat
    (≢⍴⍵)gaps ⍵ plane subs
  }
  aligned←{
    rows cols←sepr⍴¨⍵
    sizes←(⌈/rows) ,⌝ ⌈⌿cols
    ctd=0:sizes↑¨⍵
    v h←sepr⌈0.5×⊃(⍴¨⍵)-sizes
    v⊖¨h⌽¨sizes↑¨⍵
  }
  gaps←{
    ⍺≤2:⍵
    subs←(⍺-1)∇¨⍵
    width←↑⌽⍴↑subs
    fill←(⍺ width-3 0)⍴' '
    ⊃{⍺⍪fill⍪⍵}/1 open subs
  }
  plane←{
    2<⍴⍴⍺:⍺ join ⍵
    odec←(dec shape ⍺)outer ⍵
    idec←inner ⍺
    (odec,idec)collect ⍵
  }
  join←{
    sep←(≢⍵)÷1⌈≢⍺
    split←(0=sep|¯1+⍳≢⍵)⊂[1]⍵
    (⊂⍤¯1⊢⍺)plane¨split
  }
  outer←{
    sizes←1 0{↑↓(⍉⍣⍺)⍵}¨sepr⍴¨⍵
    sides←sizes/¨¨'│─'
    bords←dec↓¨'├┬'glue¨sides
    ,¨/('┌' '')⍺ bords'└┐'
  }
  inner←{
    deco←{(type ⍵),1 shape ⍵}
    sepr deco¨matr dec open ⍵
  }
  collect←{
    lft top tt vv hh←⍺
    cells←vv right 1 open tt hh lower ⍵
    boxes←(dec∨0∊⍴⍵)open cells
    lft,top⍪⍪⌿,/boxes
  }
  right←{
    types←2⊥¨(⍳⍴⍵)=⊂⍴⍵
    chars←'┼┤┴┘'[1+types]
    rgt←{⍵,(-≢⍵)↑(≢⍵)1 1/'│',⍺}
    ((matr 1 open ⍺),¨chars)rgt¨⍵
  }
  lower←{
    split←{((¯2+2⊃⍴⍵)/'─')glue ⍺}
    bot←{⍵⍪(-2⊃⍴⍵)↑⍺ split ⍵}
    (matr,¨/⍺)bot¨matr ⍵
  }
  type←{
    dec<|≡⍵:'─'
    isor ⍵:'∇'
    sst←{
      0=dec×⍴⍴⍵:'─'
      (1+↑⍵∊'¯',•D)⊃'#~'
    }∘⍕
    0=≡⍵:sst ⍵
    {(1+1=⍴⍵)⊃'+'⍵}∪,sst¨dec open ⍵
  }
  shape←{     
    dec≤0=⍴⍴⍵:⍺/¨'│─'
    cols←(1+×¯1↑⍴⍵)⊃'⊖→'
    rsig←(1+××/¯1↓⍴⍵)⊃'⌽↓'
    rows←(3⌊⍴⍴⍵)⊃'│'rsig'⍒'
    rows cols
  }
  matr←{⊃,↓⍵}
  sepr←{+/¨1⊂⊃⍵}
  open←{(⍺⌈⍴⍵)↑⍵}
  isor←{1 ⍬≡(≡⍵)(⍴⍵)}
  glue←{0=⍴⍵ : ⍵ ⋄ ⍺{⍺,⍶,⍵}/⍵}
  isor ⍵:format⊂⍵
  1=≡,⍵:format ⍵
  box ⍵
}
result←disp (1 2⋄ 3 4 5)
⍝ =>
3 11⍴'┌───┬─────┐│1 2│3 4 5│└───┴─────┘'

⍝ aplcart/table.tsv:1829 — Locate indices Iv in array Y
Iv←1 3 ⋄ Y←2 3 4 5 ⋄ Iv{1@⍺{0}¨⍵}Y   ⍝ 1 0 1 0

⍝ aplcart/table.tsv:1832 — Sum of list of polynomials with descending coefficients
Nv←(1 2 3⋄ 4 5⋄ ,6) ⋄ {⌽+⌿⊃⌽¨⍵}Nv   ⍝ 1 6 14

⍝ aplcart/table.tsv:1833 — Subdiagonal matrix of size Jv (n or m,n)
{1=-/¨⍳2⍴⍵} 6 7
6 7⍴0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0

⍝ aplcart/table.tsv:1834 — Superdiagonal matrix of size Jv (n or m,n)
{¯1=-/¨⍳2⍴⍵} 6 7
6 7⍴0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1

⍝ aplcart/table.tsv:1835 — All binary representations up to Js (truth table)
Js←5 ⋄ {⍉2⊥⍣¯1⊢0,⍳⍵}Js   ⍝ 6 3⍴0 0 0 0 0 1 0 1 0 0 1 1 1 0 0 1 0 1

⍝ aplcart/table.tsv:1836 — Binary format of decimal number Js
Js←13 ⋄ {∊⍕¨2⊥⍣¯1⊢⍵}Js   ⍝ '1101'

⍝ aplcart/table.tsv:1838 — Adjacency matrix of size Iv (n or m,n) from list of (a,b) arcs Jv
4 7 {1@⍵⊢0⍴⍨2⍴⍺} (1 6⋄ 2 7⋄ 3 1)
4 7⍴0 0 0 0 0 1 0 0 0 0 0 0 0 1 1 0 0 0 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/table.tsv:1839 — Remove consecutive duplicates from an ordered vector
Yv←1 1 2 2 2 3 ⋄ {⍵/⍨1,(≢/[2]∘(2∘↕))⍵}Yv
1 2 3

⍝ aplcart/table.tsv:1840 — Is differences of differences of adjacents
Is←2 ⋄ N←1 4 9 16 25 ⋄ {-/⌽2↕⍵}⍣Is⊢N   ⍝ 2 2 2

⍝ aplcart/table.tsv:1841 — Inverted Table Select: records Iv from inverted table Yv ((⊂Iv)⌷Y where Y is unverted Yv)
Iv←3 1 ⋄ Yv← (1 2 3⋄ 3 2⍴'abcdef')  ⋄ Iv{⍵⌷¨⍨⊂⊂⍺}Yv
(3 1 ⋄ 2 2⍴'efab')

⍝ aplcart/table.tsv:1844 — Execute Alternate: Execute Dv but if it errors, execute Cv; Reviewed Execute example checked through the Rust reference worker
Cv←'42' ⋄ Dv←'1÷0' ⋄ Cv{0::⍎⍺ ⋄ ⍎⍵}Dv   ⍝ 42

⍝ aplcart/table.tsv:1851 — Tridiagonal matrix of size Jv (n or m,n)
{1≥|-/¨⍳2⍴⍵} 6 7
6 7⍴1 1 0 0 0 0 0 1 1 1 0 0 0 0 0 1 1 1 0 0 0 0 0 1 1 1 0 0 0 0 0 1 1 1 0 0 0 0 0 1 1 1

⍝ aplcart/table.tsv:1852 — Pentadiagonal matrix of size Jv (n or m,n)
{2≥|-/¨⍳2⍴⍵} 6 7
6 7⍴1 1 1 0 0 0 0 1 1 1 1 0 0 0 1 1 1 1 1 0 0 0 1 1 1 1 1 0 0 0 1 1 1 1 1 0 0 0 1 1 1 1

⍝ aplcart/table.tsv:1853 — Heptadiagonal matrix of size Jv (n or m,n)
{3≥|-/¨⍳2⍴⍵} 6 7
6 7⍴1 1 1 1 0 0 0 1 1 1 1 1 0 0 1 1 1 1 1 1 0 1 1 1 1 1 1 1 0 1 1 1 1 1 1 0 0 1 1 1 1 1

⍝ aplcart/table.tsv:1854 — Coefficients of the binomial (approximated, fastest above 10)
Js←5 ⋄ {1,×\⌽⍛÷⍳⍵}Js   ⍝ 1 5 10 10 5 1

⍝ aplcart/table.tsv:1855 — Derangement
N←0 1 2 3 4 5 ⋄ {⌊0.5+⍵!⍛÷*1}N   ⍝ 0 0 1 2 9 44

⍝ aplcart/table.tsv:1856 — Convert from hexadecimal; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
{16⊥¯1+⍵⍳⍨•D,•A}'ABCD'   ⍝ 43981

⍝ aplcart/table.tsv:1857 — Sum of polynomials with descending coefficients
Mv←1 2 3 ⋄ Nv←4 5 ⋄ Mv{⌽+⌿⊃⌽¨⍺ ⍵}Nv   ⍝ 1 6 8

⍝ aplcart/table.tsv:1858 — Difference between polynomials with descending coefficients
Mv←1 2 3 ⋄ Nv←4 5 ⋄ Mv{⌽-⌿⊃⌽¨⍺ ⍵}Nv   ⍝ 1 ¯2 ¯2

⍝ aplcart/table.tsv:1859 — All-zero matrix of shape Jv with ones along edges
((⌽∨⊖)1∊¨⍳) 4 5
4 5⍴1 1 1 1 1 1 0 0 0 1 1 0 0 0 1 1 1 1 1 1

⍝ aplcart/table.tsv:1860 — Count number of leading major cells that match between two arrays of equal length
X←3 2⍴1 2 3 4 5 6 ⋄ Y←3 2⍴1 2 3 4 7 8 ⋄ X(+/(∧\≡⍤¯1))Y
2

⍝ aplcart/table.tsv:1861 — Count number of leading major cells that differ between two arrays of equal length
X←3 2⍴1 2 3 4 5 6 ⋄ Y←3 2⍴9 2 3 4 5 6 ⋄ X(+/(∧\≢⍤¯1))Y
1

⍝ aplcart/table.tsv:1862 — Reverse order of partitions in Y as indicated by Av (Fast ∊⌽Av⊂Yv)
1 0 0 0 1 1 0 0 {⍵⌷⍨⊂⍒+\⍺} 'abcd-XYZ'   ⍝ 'XYZ-abcd'

⍝ aplcart/table.tsv:1863 — Is N Strictly Increasing along axis Is?
Is←1 ⋄ N←3 2⍴1 2 3 4 5 6 ⋄ ~0∊</[2]2↕N   ⍝ 1

⍝ aplcart/table.tsv:1864 — Is N Non-decreasing along axis Is?
Is←2 ⋄ N←2 3⍴1 1 2 3 4 4 ⋄ ~0∊(≤/∘(2∘↕)⍤1)N
1

⍝ aplcart/table.tsv:1865 — Is N Non-increasing along axis Is?
Is←2 ⋄ N←2 3⍴3 2 2 6 5 4 ⋄ Is{~0∊2≥/[⍺]⍵}N
1

⍝ aplcart/table.tsv:1866 — Is N Strictly Decreasing along axis Is?
Is←1 ⋄ N←3 2⍴5 6 3 4 1 2 ⋄ Is{~0∊2>/[⍺]⍵}N
1

⍝ aplcart/table.tsv:1870 — Stretch Y to length Is repeating the last major cell as necessary
Is←5 ⋄ Y←2 2⍴1 2 3 4 ⋄ Is{⍺↑⍵⍪⍺⌿¯1↑⍵}Y   ⍝ 5 2⍴1 2 3 4 3 4 3 4 3 4

⍝ aplcart/table.tsv:1874 — Zero-padded character matrix from vector of integers; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
{•D[1+⍉10⊥⍣¯1⊢⍵]}31 0 2718   ⍝ 3 4⍴'003100002718'

⍝ aplcart/table.tsv:1875 — Remove consecutive duplicate rows from ordered matrix Ym
Ym←4 2⍴1 2 1 2 3 4 3 4 ⋄ {⍵⌿⍨1,∨/(≢/[2]∘(2∘↕))⍵}Ym
2 2⍴1 2 3 4

⍝ aplcart/table.tsv:1876 — Is-diagonal matrix of size Jv (n or m,n)
¯2 {⍺=-⍨/¨⍳2⍴⍵} 6 7
6 7⍴0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0

⍝ aplcart/table.tsv:1877 — Spacing out text; Double the expansion-mask length so its one-count matches the input length; upstream recipe raises LENGTH ERROR. Concrete character vector checked in Dyalog
D←'abcd' ⋄ {⍵\⍨1 0⍴⍨2×⊃⌽⍴⍵}D   ⍝ 'a b c d '

⍝ aplcart/table.tsv:1878 — Reversal (⊖) of each subvector of Y indicated by Av (fast ∊⌽¨Av⊂Yv)
Av←1 0 1 0 0 ⋄ Y←1 2 3 4 5 ⋄ Av{⍵⌷⍨⊂⌽⍒+\⍺}Y
2 1 5 4 3

⍝ aplcart/table.tsv:1880 — Taking every Is'th major cell of Y
Is←2 ⋄ Y←5 2⍴⍳10 ⋄ Is{⍵⌿⍨0=⍺|⍳≢⍵}Y   ⍝ 2 2⍴3 4 7 8

⍝ aplcart/table.tsv:1881 — Remove every Is'th cell of Y
Is←2 ⋄ Y←5 2⍴⍳10 ⋄ Is{⍵⌿⍨0≠⍺|⍳≢⍵}Y   ⍝ 3 2⍴1 2 5 6 9 10

⍝ aplcart/table.tsv:1882 — Rows of non-empty matrix Y starting with an element in X
m ← ⊃'Lorem' 'ipsum' 'dolor' 'sit' 'amet' 'consectetur' 'adipiscing' 'elit' ⋄ 'aeiou' {⍵⌿⍨⍺∊⍨⊣/⍵} m
4 11⍴'ipsum      amet       adipiscing elit       '

⍝ aplcart/table.tsv:1883 — Move major cells at positions Iv to the front/top
3 2 {⍵⌷⍨⊂⍺∪⍳≢⍵} 'Hello'   ⍝ 'leHlo'

⍝ aplcart/table.tsv:1884 — Permutation Iv to the power of Js
Iv←2 3 1 ⋄ Js←2 ⋄ Iv{⍺⊂⍛⌷⍣⍵⍳≢⍺}Js   ⍝ 3 1 2

⍝ aplcart/table.tsv:1885 — Compression vector Av for partitioned array indicated by Bv (Fast Av/⍨≢¨⊆⍨Bv)
1 0 1 0 {≠\⍵\⍺≠¯1↓0,⍺} 1 0 0 1 0 1 0 1 0
1 1 1 0 0 1 1 0 0

⍝ aplcart/table.tsv:1886 — 1-rotate on each subvector of Bv indicated by Av (Fast ∊1⌽¨Av⊂Bv)
1 0 0 0 1 0 0 {⍵[⍋⍺++\⍺]} 'abcdEFG'   ⍝ 'bcdaFGE'

⍝ aplcart/table.tsv:1887 — Cantor set iteration Js
Js←2 ⋄ {1 0 1(, ∧⌝ )⍣⍵⊢1}Js   ⍝ 1 0 1 0 0 0 1 0 1

⍝ aplcart/table.tsv:1888 — Remove elements in X from end of vector Yv
X←0 9 ⋄ Yv←0 9 1 2 0 9 ⋄ X{⍵↓⍨-⊥⍨⍵∊⍺}Yv   ⍝ 0 9 1 2

⍝ aplcart/table.tsv:1889 — Remove elements in X from beginning of vector Yv
X←0 9 ⋄ Yv←0 9 1 2 0 9 ⋄ X{⍵↓⍨⊥⍨⌽⍵∊⍺}Yv   ⍝ 1 2 0 9

⍝ aplcart/table.tsv:1890 — Cs or linefeed-delimited character vector constructed from the vector of character vectors Dv (which must be of depth 2)
Cs←'|'  ⋄ Dv←'Hello, world! 123'  ⋄ Cs{⍺←•UCS 10 ⋄ ∊⍺,¨⍵}Dv
'|H|e|l|l|o|,| |w|o|r|l|d|!| |1|2|3'

⍝ aplcart/table.tsv:1892 — Interchange first and last major cells
Y←3 2⍴⍳6 ⋄ {⊖@(1,≢⍵)⊢⍵}Y   ⍝ 3 2⍴5 6 3 4 1 2

⍝ aplcart/table.tsv:1894 — Outstanding balances on rule of 78s
Js←4 ⋄ {÷∘↑⍨⌽+\0,⍳⍵}Js   ⍝ 1 0.6 0.3 0.1 0

⍝ aplcart/table.tsv:1896 — Perfect Ripple Shuffle
Y←1 2 3 4 5 6 ⋄ {⍵⌷⍨⊂⍋⍒1 0⍴⍨≢⍵}Y   ⍝ 1 4 2 5 3 6

⍝ aplcart/table.tsv:1897 — Count number of trailing major cells that match between two arrays of equal length
X←3 2⍴1 2 3 4 5 6 ⋄ Y←3 2⍴9 2 3 4 5 6 ⋄ X(+/(∧\∘⌽≡⍤¯1))Y
2

⍝ aplcart/table.tsv:1898 — Count number of trailing major cells that differ between two arrays of equal length
X←3 2⍴1 2 3 4 5 6 ⋄ Y←3 2⍴1 2 3 4 7 8 ⋄ X(+/(∧\∘⌽≢⍤¯1))Y
1

⍝ aplcart/table.tsv:1899 — FIFO stock Nv decremented by Ms
Ms←5 ⋄ Nv←3 4 2 ⋄ Ms{(-/∘⌽∘(2∘↕))0,0⌈⍺-⍨+\⍵}Nv
0 2 2

⍝ aplcart/table.tsv:1900 — Matricise (like ⍪ but preserves trailing instead of leading shape)
Y←2 3 4⍴⍳24 ⋄ {,[¯1↓⍳≢⍴⍵]1/⍵}Y
6 4⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24

⍝ aplcart/table.tsv:1901 — Non-unique major cells
(∪(⌿⍨)1≠⊢∘≢⌸) 'abracadabra!'   ⍝ 'abr'

⍝ aplcart/table.tsv:1902 — Valid siteswap pattern?
Jv←5 3 1 ⋄ {n=≢∪n|⍵+⍳n←≢⍵}Jv   ⍝ 1

⍝ aplcart/table.tsv:1903 — Rounding N to I decimal places (Fast ⍎Is⍕N)
I←2 ⋄ N←1.234 ¯2.345 0.005 ⋄ I{p÷⍨⌊0.5+⍵×p←10*⍺}N
1.23 ¯2.34 0.01

⍝ aplcart/table.tsv:1904 — Rotate first major cells (1⊖) of each subvector of Y indicated by Av (fast ∊1⌽¨Av⊂Yv)
Av←1 0 1 0 0 ⋄ Y←3 1 5 2 4 ⋄ Av{⍵⌷⍨⊂⍋⍺++\⍺}Y
1 3 2 4 5

⍝ aplcart/table.tsv:1905 — Sort rows of matrix Ym according to column(s) I
I←2 ⋄ Ym←3 2⍴1 30 2 10 3 20 ⋄ I{⍵[⍋⍵[;⍺];]}Ym
3 2⍴2 10 3 20 1 30

⍝ aplcart/table.tsv:1906 — Deal: Is random items from ⍳Jv (without replacement); Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Is←4 ⋄ Jv←2 3 ⋄ r←Is{,⌿1+⍵⊤⍺?×/⍵}Jv ⋄ (Is=≢r)∧(r≡∪r)∧∧/r∊,⍳Jv
1

⍝ aplcart/table.tsv:1909 — Increase rank of Y to Is
Is←4 ⋄ Y←2 3⍴⍳6 ⋄ Is{⍺-⍛↑(99⍴1),⍴⍵}Y   ⍝ 1 1 2 3

⍝ aplcart/table.tsv:1910 — Remove leading and multiple blanks
( {⍵/⍨(∨/∘(2∘↕))0,' '≠⍵} '      If     only I    knew  how to   do   proper word   spacing    ' ), '.'
'If only I knew how to do proper word spacing .'

⍝ aplcart/table.tsv:1911 — Intersection (∩) on major cells for any rank
X←3 2⍴1 2 3 4 5 6 ⋄ Y←2 2⍴3 4 9 0 ⋄ X{⍺⌿⍨⍵≢⍛≥⍵⍳⍺}Y
1 2⍴3 4

⍝ aplcart/table.tsv:1912 — Remove trailing blanks
Dv← ' ab  '  ⋄ {⍵↓⍨-⊥⍨' '=⍵}Dv   ⍝ ' ab'

⍝ aplcart/table.tsv:1919 — Fast: Dv without any leading blank spaces
Dv← '  ab c '  ⋄ {(∨\' '≠⍵)/⍵}Dv   ⍝ 'ab c '

⍝ aplcart/table.tsv:1922 — Sum of digits of the first 10*Is numbers
Is←2 ⋄ {,⊃ +⌝ /⍵⍴⊂¯1+⍳10}Is
0 1 2 3 4 5 6 7 8 9 1 2 3 4 5 6 7 8 9 10 2 3 4 5 6 7 8 9 10 11 3 4 5 6 7 8 9 10 11 12 4 5 6 7 8 9 10 11 12 13 5 6 7 8 9 10 11 12 13 14 6 7 8 9 10 11 12 13 14 15 7 8 9 10 11 12 13 14 15 16 8 9 10 11 12 13 14 15 16 17 9 10 11 12 13 14 15 16 17 18

⍝ aplcart/table.tsv:1923 — Join vector of vectors Yv using separator Xv
Xv← ', '  ⋄ Yv← 'ab' '' 'cd'  ⋄ Xv{⍪/1↓,⍺⊂⍛,⍪⍵}Yv
'ab, , cd'

⍝ aplcart/table.tsv:1924 — Caesar's cipher for uppercase D (Is:encryption, -Is:decryption); Concrete APLcart Caesar cipher; original right-to-left subtraction is retained; HELLO shifted by 3 is KHOOR, independently checked in Dyalog 20.0.53963.0 with IO=1 CT=1E¯14 DIV=0 ML=1
3{•A[1+26|⍺-1-•A⍳⍵]}'HELLO'   ⍝ 'KHOOR'

⍝ aplcart/table.tsv:1925 — Are circular lists Xv and Yv identical (excluding phase)
Xv←1 2 3 ⋄ Yv←2 3 1 ⋄ Xv{⍺⊂⍛∊⌽∘⍵¨⍳≢⍵}Yv   ⍝ 1

⍝ aplcart/table.tsv:1926 — Adding an empty cell into Y at fractional position Ms
Ms←2.5 ⋄ Y←3 2⍴⍳6 ⋄ Ms{⍵⍀⍨⍺⌈⍛≠⍳1+≢⍵}Y   ⍝ 4 2⍴1 2 3 4 0 0 5 6

⍝ aplcart/table.tsv:1927 — Cut Y into partitions of length I (last partition can be shorter)
I←2 ⋄ Y←5 2⍴⍳10 ⋄ I{⍵⊂[1]⍨⍵≢⍛⍴⍺↑1}Y   ⍝ (2 2⍴1 2 3 4 ⋄ 2 2⍴5 6 7 8 ⋄ 1 2⍴9 10)

⍝ aplcart/table.tsv:1928 — ¯1-rotate on each subvector of Bv indicated by Av (Fast ∊¯1⌽¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ Bv←1 1 1 0 1 ⋄ Av{⍵[⍋⍺+\⍛-1⌽⍺]}Bv
1 1 1 1 0

⍝ aplcart/table.tsv:1929 — Value of polynomial with coefficients Nv at point Ms
Ms←2 ⋄ Nv←1 2 3 ⋄ Ms{⍵+.×⍨⍺*¯1+⍳≢⍵}Nv   ⍝ 17

⍝ aplcart/table.tsv:1931 — Remove multiple and trailing blanks
{⍵/⍨(∨/∘(2∘↕))0,⍨' '≠⍵} '      If     only I    knew  how to   do   proper word   spacing    '
' If only I knew how to do proper word spacing'

⍝ aplcart/table.tsv:1932 — Justifying left
D← 2 4⍴'  ab c  '  ⋄ {⍵⌽⍨+/∧\' '=⍵}D   ⍝ 2 4⍴'ab  c   '

⍝ aplcart/table.tsv:1933 — Sum (+/) of each subvector of N indicated by Av (fast +/¨Av⊂Nv)
Av←1 0 1 0 0 ⋄ N←1 2 3 4 5 ⋄ Av{(-/[2]∘(⌽[2])∘(2∘↕))0⍪(1⌽⍺)⌿+⍀⍵}N
3 12

⍝ aplcart/table.tsv:1934 — Any-true or any-positive (∨/×) of each subvector of non-negative N indicated by Av (fast ∨/¨Av⊂×Nv)
Av←1 0 1 0 0 ⋄ N←1 2 3 4 5 ⋄ Av{(</[2]∘(2∘↕))0⍪(1⌽⍺)⌿+⍀⍵}N
1 1

⍝ aplcart/table.tsv:1935 — Reverse parity (=⌿) in each subvector of B as indicated by Av (fast =/¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ B←1 0 1 1 0 ⋄ Av{(=/[2]∘(2∘↕))1⍪(1⌽⍺)⌿=⍀⍵}B
0 0

⍝ aplcart/table.tsv:1936 — Parity (≠⌿) in each subvector of B indicated by Av (fast ≠/¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ B←1 0 1 1 0 ⋄ Av{(≠/[2]∘(2∘↕))0⍪(1⌽⍺)⌿≠⍀⍵}B
1 0

⍝ aplcart/table.tsv:1937 — Scale Nv so the maximum element is Ms
5 {⍵×⍺÷⍵[↑⍒|⍵]} ¯2 ¯1   ⍝ 5 2.5

⍝ aplcart/table.tsv:1938 — Remove leading blank columns
Dm← 2 4⍴'  ab  c '  ⋄ {⍵/⍨∨\' '∨.≠⍵}Dm   ⍝ 2 2⍴'abc '

⍝ aplcart/table.tsv:1939 — Remove leading blank rows
Dm← 3 3⍴'   ab c d'  ⋄ {⍵⌿⍨∨\⍵∨.≠' '}Dm   ⍝ 2 3⍴'ab c d'

⍝ aplcart/table.tsv:1940 — Sorting words in list Dm according to word length
Dm← 3 3⍴'abcde f  '  ⋄ {⍵[⍋⍵+.≠' ';]}Dm   ⍝ 3 3⍴'f  de abc'

⍝ aplcart/table.tsv:1941 — The Is smallest cells of Y in order of occurrence
Is←2 ⋄ Y←3 1 5 2 4 ⋄ Is{⍵⌿⍨(⍋⍋⍵)∊⍳⍺}Y   ⍝ 1 2

⍝ aplcart/table.tsv:1942 — The Is largest cells of Y in order of occurrence
Is←2 ⋄ Y←3 1 5 2 4 ⋄ Is{⍵⌿⍨(⍋⍒⍵)∊⍳⍺}Y   ⍝ 5 4

⍝ aplcart/table.tsv:1943 — Locations of texts between and including quotes
Dv← 'a+''hello''+b'  ⋄ {(∨/∘(2∘↕))0,≠\⍵=''''}Dv
0 0 1 1 1 1 1 1 1 0 0

⍝ aplcart/table.tsv:1944 — Locations of texts between and excluding quotes
Dv← 'a+''hello''+b'  ⋄ {(∧/∘(2∘↕))0,≠\⍵=''''}Dv
0 0 0 1 1 1 1 1 0 0 0

⍝ aplcart/table.tsv:1947 — Extending Y with last cell of Y to length Is
Is←5 ⋄ Y←2 2⍴⍳4 ⋄ Is{⍵⍪(⍺-≢⍵)⌿¯1↑⍵}Y   ⍝ 5 2⍴1 2 3 4 3 4 3 4 3 4

⍝ aplcart/table.tsv:1950 — Modes: most frequently occuring major cells
Y←1 2 1 3 2 ⋄ {⍵⌷⍨⊂0~⍨⊢/0,⊢⌸⍵}Y   ⍝ 1 2

⍝ aplcart/table.tsv:1953 — Remove columns Iv from array Y
Iv←2 ⋄ Y←2 3⍴⍳6 ⋄ Iv{⍵/⍨~⍺∊⍨⍳↑⌽⍴⍵}Y   ⍝ 2 2⍴1 3 4 6

⍝ aplcart/table.tsv:1954 — Cut Y into partitions of lengths Iv
Iv←2 1 3 ⋄ Y←⍳6 ⋄ Iv{⍺-⍛↑¨↑∘⍵¨+\⍺}Y   ⍝ (1 2 ⋄ 1⍴3 ⋄ 4 5 6)

⍝ aplcart/table.tsv:1955 — Copying each cell of Y until before next 1 in Av
Av←1 0 1 0 0 ⋄ Y←3 1 5 2 4 ⋄ Av{⍵⌷⍨⊂1⌈⌈\⍺×⍳≢⍺}Y
3 3 5 5 5

⍝ aplcart/table.tsv:1956 — Work done for demand N with capacity M
Mv←3 3 3 3 ⋄ Nv←1 5 2 6 ⋄ Mv{⍺+(-/[2]∘(2∘↕))⌈⍀0⍪+⍀⍺-⍵}Nv
1 3 3 3

⍝ aplcart/table.tsv:1957 — Convert to hexadecimal; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
{(•D,•A)[1+16⊥⍣¯1⊢⍵]}65535   ⍝ 'FFFF'

⍝ aplcart/table.tsv:1958 — Maxima of elements of subsets of Nv specified by A (one mask per column)
A←3 2⍴1 0 0 1 1 1 ⋄ Nv←2 7 4 ⋄ A{m+⍺⌈.×⍨⍵-m←⌊/⍵}Nv
4 7

⍝ aplcart/table.tsv:1959 — The Is'th subvector of Yv (subvectors separated by Yv[1])
Is←2 ⋄ Yv←0 1 2 0 3 4 0 5 ⋄ Is{1↓⍵/⍨⍺=+\⍵=↑⍵}Yv
3 4

⍝ aplcart/table.tsv:1960 — Extracting field number Is from Dv (field separated by first element of Dv)
Is←2 ⋄ Dv← '/ab/cde/f'  ⋄ Is{1↓⍵/⍨⍺=+\⍵=↑⍵}Dv
'cde'

⍝ aplcart/table.tsv:1961 — Integral of polynomial Nv with descending coefficients and optional constant Ms
Ms←7 ⋄ Nv←3 4 5 ⋄ Ms{⍺←0 ⋄ ⍺,⍨⍵÷⌽⍳≢⍵}Nv   ⍝ 1 2 5 7

⍝ aplcart/table.tsv:1963 — Union (∪) on major cells of any rank
X←2 2⍴1 2 3 4 ⋄ Y←2 2⍴3 4 5 6 ⋄ X{⍺⍪⍵⌿⍨⍺≢⍛<⍺⍳⍵}Y
3 2⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:1964 — Average (mean value) of elements of N along direction Is
Is←2 ⋄ N←2 3⍴⍳6 ⋄ Is{(+/[⍺]÷⍺⊃⍴)⍵}N   ⍝ 2 5

⍝ aplcart/table.tsv:1965 — Replacing elements of Y in set X with prototypical elements
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X{(↑0⍴⊂)@(∊∘⍺)⍵}Y   ⍝ 1 0 3 0 5

⍝ aplcart/table.tsv:1966 — Ordinal numbers of words in Dv that indices I point to
I←1 3 5 7 ⋄ Dv← 'ab cd ef'  ⋄ I{(1++\' '=⍵)[⍺]}Dv
1 2 2 3

⍝ aplcart/table.tsv:1967 — Changing connectivity matrix Jm to a connectivity list
Jm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ {⍵,⍛/1+s⊤¯1+⍳×/s←⍴⍵}Jm
2 3⍴1 2 3 2 3 1

⍝ aplcart/table.tsv:1968 — Value of Taylor series with coefficients Nv at point Ns
Ns←2 ⋄ Nv←1 1 1 1 ⋄ Ns{+/⍵××\1,⍺÷⍳¯1+≢⍵}Nv
6.333333333333333

⍝ aplcart/table.tsv:1969 — Not leading zeroes (∨\) in each subvector of Bv indicated by Av (fast ∊∨\¨Av⊂Nv)
Av←1 0 1 0 0 ⋄ Bv←1 1 1 0 1 ⋄ Av{≠⍀b⍀(≠/[2]∘(2∘↕))0⍪⍵⌿⍨b←⍺∨⍵}Bv
1 1 1 1 1

⍝ aplcart/table.tsv:1970 — Date (⎕TS format) to hh:mm:ss
Jv←2026 9 18 12 34 56 0 ⋄ {1↓∊':'@1∘⍕¨100+3↑3↓⍵}Jv
'12:34:56'

⍝ aplcart/table.tsv:1971 — Sum elements of Nv marked by successive identicals in Mv
Mv←1 1 2 2 2 ⋄ Nv←1 2 3 4 5 ⋄ Mv{(-/[2]∘(⌽[2])∘(2∘↕))0⍪((≠/∘(2∘↕))⍺⍪0)⌿+⍀⍵}Nv
3 12

⍝ aplcart/table.tsv:1972 — Sums over (+/) subvectors of N, lengths in Iv
Iv←2 3 ⋄ N←1 2 3 4 5 ⋄ Iv{(-/[2]∘(⌽[2])∘(2∘↕))0⍪(⊂+\⍺)⌷+⍀⍵}N
3 12

⍝ aplcart/table.tsv:1973 — Remove trailing blank columns
(( 2 10⍴' 2    a       a a    ' ), '||' ⋄ ( {⍵/⍨⌽∨\⌽' '∨.≠⍵} 2 10⍴' 2    a       a a    ' ), '||')
(2 11⍴' 2    a   |    a a   |' ⋄ 2 8⍴' 2    a|    a a|')

⍝ aplcart/table.tsv:1974 — Grade up (⍋) for sorting each subarray of Y indicated by Av
Av←1 0 1 0 0 ⋄ Y←3 1 5 2 4 ⋄ Av{g[⍋(+\⍺)[g←⍋⍵]]}Y
2 1 4 5 3

⍝ aplcart/table.tsv:1975 — Grade down (⍒) for sorting each subarray of Y indicated by Av
Av←1 0 1 0 0 ⋄ Y←3 1 5 2 4 ⋄ Av{g[⍋(+\⍺)[g←⍒⍵]]}Y
1 2 3 5 4

⍝ aplcart/table.tsv:1976 — Largest sum of any contiguous subvector
Nv←¯2 3 ¯1 4 ¯5 ⋄ {s←0 ⋄ ⌈/{s⊢←0⌈s+⍵}¨⍵}Nv
6

⍝ aplcart/table.tsv:1977 — Sign (side) of point Ns relative to bisections of complex plane by directed edges of Mv
(0J¯1 0J1 {×11○(⍵-1↓⍺)×+(-/∘(2∘↕))⍺} 1J0 ⋄ 0J¯1 0J1 {×11○(⍵-1↓⍺)×+(-/∘(2∘↕))⍺} 0J0 ⋄ 0J1 0J¯1 {×11○(⍵-1↓⍺)×+(-/∘(2∘↕))⍺} 1J0)
(1⍴1 ⋄ 1⍴0 ⋄ 1⍴¯1)

⍝ aplcart/table.tsv:1978 — Iv-shaped array of random numbers in range Jv[1]…Jv[2] (inclusively, with replacement); Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Iv←2 3 ⋄ Jv←¯3 4 ⋄ r←Iv{(¯1+↑⍵)+?⍺⍴1--/⍵}Jv ⋄ (Iv≡⍴r)∧∧/∊(¯3≤r)∧r≤4
1

⍝ aplcart/table.tsv:1979 — Last indices in X of major cells Y
X←1 2 3 2 ⋄ Y←2 4 ⋄ X{l-(l←1+≢⍺)|⍺⊖⍛⍳⍵}Y
4 5

⍝ aplcart/table.tsv:1980 — Increase rank of Y to rank of X
X←2 3 4⍴0 ⋄ Y←1 2 ⋄ X{(-≢⍴⍺)↑(99⍴1),⍴⍵}Y
1 1 2

⍝ aplcart/table.tsv:1981 — Replacing elements of Y not in set X with prototypical elements
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X{(↑0⍴⊂)@(~∊∘⍺)⍵}Y
0 2 0 4 0

⍝ aplcart/table.tsv:1982 — Without (~) on major cells for any rank
X←3 2⍴1 2 3 4 5 6 ⋄ Y←2 2⍴3 4 9 0 ⋄ X{⍺⌿⍨~(⍳≢⍺)∊⍺⍳⍵}Y
2 2⍴1 2 5 6

⍝ aplcart/table.tsv:1984 — Extract the anti-diagonals of a matrix (without wrap-around) as vector of vectors
Ym←2 3⍴⍳6 ⋄ {⍵⊂⍤⊢⌸⍥,⍨+/⊃⍳⍴⍵}Ym   ⍝ (1⍴1 ⋄ 2 4 ⋄ 3 5 ⋄ 1⍴6)

⍝ aplcart/table.tsv:1985 — Powerset: All subsets of Y, excluding the empty set (0⌿Y) but including Y itself
({⌿∘⍵¨↓⌽⍉2⊥⍣¯1⍳¯1+2*≢⍵} 'ABCD' ⋄ {⌿∘⍵¨↓⌽⍉2⊥⍣¯1⍳¯1+2*≢⍵} 3 2⍴'AaBbCc')
((1⍴'A' ⋄ 1⍴'B' ⋄ 'AB' ⋄ 1⍴'C' ⋄ 'AC' ⋄ 'BC' ⋄ 'ABC' ⋄ 1⍴'D' ⋄ 'AD' ⋄ 'BD' ⋄ 'ABD' ⋄ 'CD' ⋄ 'ACD' ⋄ 'BCD' ⋄ 'ABCD') ⋄ (1 2⍴'Aa' ⋄ 1 2⍴'Bb' ⋄ 2 2⍴'AaBb' ⋄ 1 2⍴'Cc' ⋄ 2 2⍴'AaCc' ⋄ 2 2⍴'BbCc' ⋄ 3 2⍴'AaBbCc'))

⍝ aplcart/table.tsv:1987 — Accumulate deposits Nv at rate Ms
Ms←1.05 ⋄ Nv←100 200 300 ⋄ Ms{r×+\⍵÷r←×\1@1⍴∘⍺≢⍵}Nv
100 305 620.25

⍝ aplcart/table.tsv:1988 — Trim groups of ones in B to begin only where first pointed to by a 1 in A
A←0 1 0 0 1 ⋄ B←1 1 0 1 1 ⋄ A{⍵∧s=⌈⍀⍺×s←+⍀(</[2]∘(2∘↕))0⍪⍵}B
0 1 0 0 1

⍝ aplcart/table.tsv:1989 — Count of occurrences of the cells of Y
Y←1 2 1 3 1 2 ⋄ {--⌿(2,≢⍵)⍴⍋⍋⍪⍨⍵}Y   ⍝ 3 2 3 1 3 2

⍝ aplcart/table.tsv:1990 — Cardinals Up (ranking, shareable)
Y←3 1 2 1 ⋄ {⌊2÷⍨(⍋⍋⍵)+⌽⍋⍋⌽⍵}Y   ⍝ 4 1 3 1

⍝ aplcart/table.tsv:1991 — Cardinals Down (ranking, shareable)
Y←3 1 2 1 ⋄ {⌊2÷⍨(⍋⍒⍵)+⌽⍋⍒⌽⍵}Y   ⍝ 1 3 2 3

⍝ aplcart/table.tsv:1992 — Levi-Civita symbol
Jv←2 3 1 ⋄ {⍵≢∪⍵:0⋄¯1*1⊥∊⍵<,\⍵}Jv   ⍝ 1

⍝ aplcart/table.tsv:1993 — Convolution
Mv←1 2 3 ⋄ Nv←4 5 6 ⋄ Mv{⍵+.×⍨(⍳≢⍺)⌽⍤0 1⌽⍺}Nv
31 31 28

⍝ aplcart/table.tsv:1994 — Matrix power: Mm raised to the power Js (even for negative Js)
Mm←2 2⍴1 1 0 1 ⋄ Js←3 ⋄ Mm{⍺+.×⍣⍵(⍴⍴1,0⍨¨)⍺}Js
2 2⍴1 3 0 1

⍝ aplcart/table.tsv:1995 — Format with leading zeroes for non-negative Jv in fields of width Is
Is←4 ⋄ Jv←0 12 345 ⋄ Is{0 1↓(2↑1+⍺)⍕⍵ +⌝ ,10*⍺}Jv
3 4⍴'000000120345'

⍝ aplcart/table.tsv:1997 — Compress delimited string Dv (where the first character is the delimiter) using compression vector Av
1 0 1 {⍵/⍨(+\⍵=↑⍵)∊⍸⍺} '/here/be/dragons'
'/here/dragons'

⍝ aplcart/table.tsv:1999 — Execution of expression Dv with default value X; Reviewed Execute example checked through the Rust reference worker
X←42 ⋄ Dv←' ' ⋄ X{⍎⍵,'⍺'⍴⍨' '∧.=⍵}Dv   ⍝ 42

⍝ aplcart/table.tsv:2002 — Sorting Y in case-insensitive alphabetical order
Y←42 'Pete' 'Πέτρος'  ⋄ {⍵⌷⍨⊂⍋{(•C⍵)⍵}¨⍵}Y
42 ('Pete') ('Πέτρος')

⍝ aplcart/table.tsv:2004 — Powerset: All subsets of Y, excluding the empty set (0⌿Y) and Y itself
({⌿∘⍵¨↓⌽⍉2⊥⍣¯1⊢⍳¯2+2*≢⍵} 'ABCD' ⋄ {⌿∘⍵¨↓⌽⍉2⊥⍣¯1⊢⍳¯2+2*≢⍵} 3 2⍴'AaBbCc')
((1⍴'A' ⋄ 1⍴'B' ⋄ 'AB' ⋄ 1⍴'C' ⋄ 'AC' ⋄ 'BC' ⋄ 'ABC' ⋄ 1⍴'D' ⋄ 'AD' ⋄ 'BD' ⋄ 'ABD' ⋄ 'CD' ⋄ 'ACD' ⋄ 'BCD') ⋄ (1 2⍴'Aa' ⋄ 1 2⍴'Bb' ⋄ 2 2⍴'AaBb' ⋄ 1 2⍴'Cc' ⋄ 2 2⍴'AaCc' ⋄ 2 2⍴'BbCc'))

⍝ aplcart/table.tsv:2005 — Powerset: All subsets of Y, including the empty set (0⌿Y) and Y itself
({⌿∘⍵¨↓⌽⍉2⊥⍣¯1⊢¯1+⍳2*≢⍵} 'ABCD' ⋄ {⌿∘⍵¨↓⌽⍉2⊥⍣¯1⊢¯1+⍳2*≢⍵} 3 2⍴'AaBbCc')
(('' ⋄ 1⍴'A' ⋄ 1⍴'B' ⋄ 'AB' ⋄ 1⍴'C' ⋄ 'AC' ⋄ 'BC' ⋄ 'ABC' ⋄ 1⍴'D' ⋄ 'AD' ⋄ 'BD' ⋄ 'ABD' ⋄ 'CD' ⋄ 'ACD' ⋄ 'BCD' ⋄ 'ABCD') ⋄ (0 2⍴' ' ⋄ 1 2⍴'Aa' ⋄ 1 2⍴'Bb' ⋄ 2 2⍴'AaBb' ⋄ 1 2⍴'Cc' ⋄ 2 2⍴'AaCc' ⋄ 2 2⍴'BbCc' ⋄ 3 2⍴'AaBbCc'))

⍝ aplcart/table.tsv:2006 — Cumulative all-true (∧\) in each subvectors of Bv indicated by Av (fast ∊∧\¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ Bv←1 1 1 0 1 ⋄ Av{~≠\z\(≠/∘(2∘↕))0,~⍵/⍨z←⍺≥⍵}Bv
1 1 1 0 0

⍝ aplcart/table.tsv:2007 — Mesh major cells of elements of Yv
({(⊂⍋∊⍳∘≢¨⍵)⌷⊃⍪/⍵} 'abc'•D'ABCDEF' ⋄ {(⊂⍋∊⍳∘≢¨⍵)⌷⊃⍪/⍵} (4 2⍴'aabbccdd'⋄ 2 2⍴'AABB'))
('a0Ab1Bc2C3D4E5F6789' ⋄ 6 2⍴'aaAAbbBBccdd')

⍝ aplcart/table.tsv:2008 — Determinant of three-row matrix
Nm←3 3⍴1 2 3 0 4 5 1 0 6 ⋄ {-/+/×/[2](2 3⍴0 1 2 0 2 1)⌽⊃⍵ ⍵}Nm
22

⍝ aplcart/table.tsv:2009 — Vectors as row matrices in catenation upon each other
X←1 2 3 ⋄ Y←4 5 6 ⋄ X{⍺,[2÷⍨⌈/≢∘⍴¨⍺⍵]⍵}Y
2 3⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:2010 — Remove trailing blank rows
(mat ← 10 2⍴'  2    a  aa        ' ⋄ '--' ⋄ {⍵↓⍨-+/∧\⌽⍵∧.=' '} mat)
(10 2⍴'  2    a  aa        ' ⋄ '--' ⋄ 6 2⍴'  2    a  aa')

⍝ aplcart/table.tsv:2011 — Rows of matrix Ym starting with Xv
Xv←1 2 ⋄ Ym←3 3⍴1 2 3 1 4 5 1 2 6 ⋄ Xv{⍵⌿⍨⍺∧.=⍨⍵↑[2]⍨≢⍺}Ym
2 3⍴1 2 3 1 2 6

⍝ aplcart/table.tsv:2012 — Taylor series at point Mv, coefficients Nv
Mv←,2 ⋄ Nv←1 1 1 1 ⋄ Mv{⍵+.×(⍺∘*÷!)¯1+⍳≢⍵}Nv
6.333333333333333

⍝ aplcart/table.tsv:2013 — Cross-correlation
Mv←1 2 3 ⋄ Nv←4 5 6 ⋄ Mv{⍵+.×⍨(⍳≢⍺)⌽⍤0 1+⌽⍺}Nv
31 31 28

⍝ aplcart/table.tsv:2014 — Auto-correlation; Use ⍵ for the length in this monadic autocorrelation; upstream ⍺ gives VALUE ERROR. Concrete vector independently checked in Dyalog
Nv←1 2 3 ⋄ {⍵+.×⍨(⍳≢⍵)⌽⍤0 1+⌽⍵}Nv   ⍝ 13 13 10

⍝ aplcart/table.tsv:2016 — Surround any-rank array Y with scalar Xs
Xs←9 ⋄ Y←2 3⍴⍳6 ⋄ Xs{⍵@(1+⍳⍴⍵)⊢⍺⍴⍨2+⍴⍵}Y
4 5⍴9 9 9 9 9 9 1 2 3 9 9 4 5 6 9 9 9 9 9 9

⍝ aplcart/table.tsv:2017 — Fast: A nested vector comprising simple character vectors constructed from the rows of Dm (which must be of depth 1) with trailing spaces removed
Dm← 3 3⍴'ab c  def'  ⋄ {(+/∨\' '≠⌽⍵)↑¨↓⍵}Dm
('ab' ⋄ 1⍴'c' ⋄ 'def')

⍝ aplcart/table.tsv:2018 — Sorting rows of Ym according to key Xv (alphabetising)
Xv← 'cba'  ⋄ Ym← 3 2⍴'abacba'  ⋄ Xv{⍵[⍋(1+≢⍺)⊥⍺⍳⍉⍵;]}Ym
3 2⍴'baacab'

⍝ aplcart/table.tsv:2019 — Vector of character vectors constructed from the character vector Dv where the first character is the delimiter
Dv← '/ab/cde/f/'  ⋄ {⍺←↑⌽⍵ ⋄ 1↓¨⍺(=⊂⊢)⍵}Dv
('ab' ⋄ 'cde' ⋄ 1⍴'f' ⋄ '')

⍝ aplcart/table.tsv:2020 — All-true (∧/) in each subvector of Bv indicated by Av (fast ∧/¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ Bv←1 1 1 0 1 ⋄ Av{(⍺/⍵)∧z/1⌽z←⍺/⍨⍺≥⍵}Bv
1 0

⍝ aplcart/table.tsv:2021 — All elements true (∧⌿) on each subvector of Bv partioned by Av (fast ∧⌿¨Av⊂[1]Bv)
Av←1 0 1 0 0 ⋄ Bv←1 1 1 0 1 ⋄ Av{(⍺⌿⍵)∧a⌿1⊖a←⍺⌿⍨⍵≤⍺}Bv
1 0

⍝ aplcart/table.tsv:2022 — Any element true (∨⌿) on each subvector of Bv partitioned by Av (fast ∨⌿¨Av⊂[1]Bv)
Av←1 0 1 0 0 ⋄ Bv←1 1 1 0 1 ⋄ Av{(⍺⌿⍵)≥a⌿1⊖a←⍺⌿⍨⍵∨⍺}Bv
1 1

⍝ aplcart/table.tsv:2023 — Generalised mean
Ms←2 ⋄ N←1 2 3 ⋄ Ms{(+⌿(⍵*⍺)÷≢⍵)*÷⍺}N   ⍝ 2.160246899469286

⍝ aplcart/table.tsv:2024 — Powerset: All subsets of Y, including the empty set (0⌿Y) but excluding Y itself
({⌿∘⍵¨↓⌽⍉2⊥⍣¯1⊢¯1+⍳¯1+2*≢⍵} 'ABCD' ⋄ {⌿∘⍵¨↓⌽⍉2⊥⍣¯1⊢¯1+⍳¯1+2*≢⍵} 3 2⍴'AaBbCc')
(('' ⋄ 1⍴'A' ⋄ 1⍴'B' ⋄ 'AB' ⋄ 1⍴'C' ⋄ 'AC' ⋄ 'BC' ⋄ 'ABC' ⋄ 1⍴'D' ⋄ 'AD' ⋄ 'BD' ⋄ 'ABD' ⋄ 'CD' ⋄ 'ACD' ⋄ 'BCD') ⋄ (0 2⍴' ' ⋄ 1 2⍴'Aa' ⋄ 1 2⍴'Bb' ⋄ 2 2⍴'AaBb' ⋄ 1 2⍴'Cc' ⋄ 2 2⍴'AaCc' ⋄ 2 2⍴'BbCc'))

⍝ aplcart/table.tsv:2025 — Centering flush left character array
Dm← 2 6⍴'ab    cde   '  ⋄ {⍵⌽⍨-⌊2÷⍨+/∧\' '=⌽⍵}Dm
2 6⍴'  ab   cde  '

⍝ aplcart/table.tsv:2026 — Cumulative sum (+⍀) in each subvector of N indicated by Av (fast ∊+\¨Av⊂Nv)
Av←1 0 1 0 0 ⋄ N←1 2 3 4 5 ⋄ Av{+⍀⍵-⍺⍀(-/[2]∘(⌽[2])∘(2∘↕))0⍪⍺⌿+⍀¯1↓0⍪⍵}N
1 3 3 7 12

⍝ aplcart/table.tsv:2027 — Running parity (≠⍀) in each subvector of B indicated by Av (fast ∊≠\¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ B←1 0 1 1 0 ⋄ Av{≠⍀⍵≠⍺⍀(≠/[2]∘(2∘↕))0⍪⍺⌿≠⍀¯1↓0⍪⍵}B
1 1 1 0 0

⍝ aplcart/table.tsv:2028 — Vectors as column matrices in catenation beneath each other
X←1 2 3 ⋄ Y←4 5 6 ⋄ X{⍺,[1+2÷⍨⌈/≢∘⍴¨⍺⍵]⍵}Y
3 2⍴1 4 2 5 3 6

⍝ aplcart/table.tsv:2029 — Playing order in a cup for Js ranked players
Js←5 ⋄ {,⍉2(⍴⍨⍴*↑∘⍳⍵⍨)⌈2⍟⍵}Js   ⍝ 1 5 3 0 2 0 4 0

⍝ aplcart/table.tsv:2030 — Remove duplicate blank columns
Dm← 2 5⍴'a  ba   c '  ⋄ {⍵/⍨¯1↓1,1(⊢∨⌽)' '∨.≠⍵}Dm
2 4⍴'a ba  c '

⍝ aplcart/table.tsv:2031 — Remove duplicate blank rows
Dm← 4 2⍴'ab    cd'  ⋄ {⍵⌿⍨¯1↓1,1(⊢∨⌽)⍵∨.≠' '}Dm
3 2⍴'ab  cd'

⍝ aplcart/table.tsv:2032 — Character matrix constructed from the linefeed-separated character vector Dv (which must be of depth 1) padded with trailing spaces
Dv←'Hello, world! 123'  ⋄ {⍺←•UCS 10 ⋄ ⊃1↓¨⍺(=⊂⊢)⍺,⍵}Dv
1 17⍴'Hello, world! 123'

⍝ aplcart/table.tsv:2034 — Cumulative maximum (⌈⍀) in each subvector of Y indicated by Av (fast ∊⌈\¨Av⊂Yv)
Av←1 0 1 0 0 ⋄ Y←3 1 5 2 4 ⋄ Av{⍵⌷⍨⊂z⍳⌈\z←⍋⍋⍺+\⍛,⍪⍵}Y
3 3 5 5 5

⍝ aplcart/table.tsv:2035 — Cumulative minimum (⌊⍀) in each subvector of Y indicated by Av (fast ∊⌊\¨Av⊂Yv)
Av←1 0 1 0 0 ⋄ Y←3 1 5 2 4 ⋄ Av{⍵⌷⍨⊂z⍳⌊\z←⍋⍋⍺+\⍛,⍪⍵}Y
3 1 1 1 1

⍝ aplcart/table.tsv:2036 — Cumulative reverse parity (=⍀) in each subvector B as indicated by Av (fast ∊=\¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ B←1 0 1 1 0 ⋄ Av{=⍀⍵≠⍺⍀(≠/[2]∘(2∘↕))0⍪~⍺⌿=⍀¯1↓1⍪⍵}B
1 0 1 1 0

⍝ aplcart/table.tsv:2037 — Rounding N to I significant digits (Fast ⍎(-Is)⍕N)
I←3 ⋄ N←12.345 0.012345 12345 ⋄ I{s×⌊0.5+⍵÷s←10*⍺-⍨⌈10⍟⍵+0=⍵}N
12.3 0.0123 12300

⍝ aplcart/table.tsv:2038 — Area of pyramid with height and width Mv and length Ns (excluding base)
Mv←3 4 ⋄ Ns←6 ⋄ Mv{⍵×2+.*∘÷⍨(⍺*2)+×⍨⍵÷2}Ns
55.4558441227157

⍝ aplcart/table.tsv:2039 — Remove elements in X from beginning and end of vector Yv
X←0 9 ⋄ Yv←0 9 1 2 0 9 ⋄ X{(⊥⍨∘⌽↓⍵↓⍨∘-⊥⍨)⍵∊⍺}Yv
1 2

⍝ aplcart/table.tsv:2040 — Deal: Iv-shaped array of random items from ⍳Jv (without replacement); Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Is←2 2 ⋄ Jv←2 3 ⋄ r←Is{⍺⍴,⌿1+⍵,⍛⊤⍺?⍥(×/)⍵}Jv ⋄ (Is≡⍴r)∧((,r)≡∪,r)∧∧/∊r∊,⍳Jv
1

⍝ aplcart/table.tsv:2041 — Interpolate Iv values between major cells of N
Iv←1 2 ⋄ N←0 4 10 ⋄ Iv{+⍀(1↑⍵),i⌿((-/[2]∘(⌽[2])∘(2∘↕))⍵)÷i←1+⍺}N
0 2 4 6 8 10

⍝ aplcart/table.tsv:2042 — Sierpiński triangle of size Js; Pure glyph recipe; quoted quad is data or origin lookup is replaced with fixed one; Supply small operands and use fixed origin one where the recipe reads index origin; Quoted quad remains character data; Generate each row from the scan accumulator
Js←3 ⋄ {' ⎕'[1+⊃≠\⍤⊣\↓1⍴⍨2⍴2*⍵]}Js
8 8⍴'⎕⎕⎕⎕⎕⎕⎕⎕⎕ ⎕ ⎕ ⎕ ⎕⎕  ⎕⎕  ⎕   ⎕   ⎕⎕⎕⎕    ⎕ ⎕     ⎕⎕      ⎕       '

⍝ aplcart/table.tsv:2043 — Inverse of monadic ⍸ with optional result shape as left argument
Yv←1 3 3 4 ⋄ {⍺←0⌈⊃⌈/⍵ ⋄ r←⍺⍴0 ⋄ r[⍵]+←1 ⋄ r}Yv
1 0 2 1

⍝ aplcart/table.tsv:2044 — Reversal (⊖) of subvectors of Y having lengths Iv
Iv←2 3 ⋄ Y←1 2 3 4 5 ⋄ Iv{⍵⌷⍨⊂⌽⍒+\(⍳≢⍵)∊+\1,⍺}Y
2 1 5 4 3

⍝ aplcart/table.tsv:2045 — Number of decimals (up to Is) of elements of N
Is←3 ⋄ N←1 1.2 1.23 1.234 ⋄ Is{0+.≠(10*⌽0,⍳⍺) |⌝ ⌊⍵×10*⍺}N
0 1 2 3

⍝ aplcart/table.tsv:2046 — Conversion of characters to hexadecimal byte representation
Dv←'Hello, world! 123'  ⋄ {,⍉3↑(•D,•A)[1+16 16⊤¯1+'UTF-8'•UCS ⍵]}Dv
'47 64 6B 6B 6E 2B 1F 76 6E 71 6B 63 20 1F 30 31 32 '

⍝ aplcart/table.tsv:2047 — Increasing rank of Y to rank of X
X←2 3 4⍴0 ⋄ Y←1 2 ⋄ X{⍵⍴⍨(1⍴⍨-/≢∘⍴¨⍺⍵),⍴⍵}Y
1 1 2⍴1 2

⍝ aplcart/table.tsv:2048 — Remove leading and trailing blanks
{⍵/⍨(∨\∧∘⌽∨\∘⌽)' '≠⍵} '      If     only I    knew  how to   do   proper word   spacing    '
'If     only I    knew  how to   do   proper word   spacing'

⍝ aplcart/table.tsv:2049 — Only final one (</) in each subvector of Bv indicated by Av (fast </¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ Bv←1 1 1 0 1 ⋄ Av{(⍺/y)∧z/1⌽z←⍺/⍨⍺≥y←⍵=1⌽⍺}Bv
0 0

⍝ aplcart/table.tsv:2050 — Cumulative less than or equal scan on each subvector of Bv indicated by Av (Fast ∊≤\¨Av⊂Bv)
1 0 0 0 1 0 0 {(⍺≤⍵)>b\z<¯1↓1,z←⍵/⍨b←⍵≤⍺} 1 0 1 0 1 1 0
1 0 1 1 1 1 0

⍝ aplcart/table.tsv:2051 — Not first zero (≤\) in each subvector of B indicated by Av (fast ∊≤\¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ B←1 0 1 1 0 ⋄ Av{(b∧⍺)⍱c⍀(</[2]∘(2∘↕))0⍪(c←b∨⍺)⌿b←~⍵}B
1 0 1 1 0

⍝ aplcart/table.tsv:2054 — Discrete Fourier Transformation
{⍵+.×⍨*π0J¯2×n÷⍨ ×⌝ ⍨¯1+⍳n←≢⍵} 1 2j¯1 0j¯1 ¯1j2
2 ¯2j¯2 0j¯2 4j4

⍝ aplcart/table.tsv:2055 — Progressive index of (⍳) without replacement
X←1 2 1 ⋄ Y←1 1 1 2 ⋄ X{⍺(R⍨⍳R←≢⍤⊢⍴∘⍋∘⍋⍺⍳⍪⍨)⍵}Y
1 3 4 2

⍝ aplcart/table.tsv:2056 — Progressive member of (∊) without replacement
X←1 2 1 ⋄ Y←1 1 1 2 ⋄ X{⍺(R⍨∊R←≢⍤⊢⍴∘⍋∘⍋⍺⍳⍪⍨)⍵}Y
1 1 1

⍝ aplcart/table.tsv:2057 — Minimum (⌊/) in each subvector of Y indicated by Av (fast ⌊/¨Av⊂Yv)
Av←1 0 1 0 0 ⋄ Y←3 1 5 2 4 ⋄ Av{⍵⌷⍨⊂g[⍺/⍋(+\⍺)[g←⍋⍵]]}Y
1 2

⍝ aplcart/table.tsv:2058 — Maximum (⌈/) in each subvector of Y indicated by Av (fast ⌈/¨Av⊂Yv)
Av←1 0 1 0 0 ⋄ Y←3 1 5 2 4 ⋄ Av{⍵⌷⍨⊂g[⍺/⍋(+\⍺)[g←⍒⍵]]}Y
3 5

⍝ aplcart/table.tsv:2060 — Occurrence Count: Enumerate each unique major cells separately
Y←1 2 1 3 2 1 ⋄ {1+o-(m⌿o←⍋⍋⍵)[⍵⍳⍨⍵⌿⍨m←≠⍵]}Y
1 1 2 1 2 3

⍝ aplcart/table.tsv:2063 — Adding an empty cell into Y at fractional positions Mv
Mv←0.5 2.5 ⋄ Y←1 2 3 ⋄ Mv{⍵⍀⍨~(⍳⍺+⍥≢⍵)∊⍺⌊⍛+⍳≢⍺}Y
0 1 2 0 3

⍝ aplcart/table.tsv:2064 — Inverted Table Grade Up (⍋Y where Y is unverted Yv)
Yv← (2 1 2⋄ 3 2⍴'bacadb')  ⋄ {{⍵[⍋⍺⌷⍨⊂⍵]}/⍵,⊂⍳≢↑⍵}Yv
2 1 3

⍝ aplcart/table.tsv:2065 — Inverse Discrete Fourier Transformation
{n÷⍨⍵+.×⍨*π0J2×n÷⍨ ×⌝ ⍨¯1+⍳n←≢⍵} 2 ¯2J¯2 0J¯2 4J4
1 2j¯1 0j¯1 ¯1j2

⍝ aplcart/table.tsv:2066 — Changing connectivity list Jm to a connectivity matrix
Jm←2 3⍴1 2 3 2 3 1 ⋄ {s⍴1@(1+s[1]⊥⍵-1)⊢0⍴⍨×/s←0 0+⌈/,⍵}Jm
3 3⍴0 1 0 0 0 1 1 0 0

⍝ aplcart/table.tsv:2067 — Expansion vector (left argument for \ or ⍀) to insert Iv[i] elements after i'th subvector (subvectors indicated by Bv)
Iv←2 1 ⋄ Bv←1 0 1 0 0 ⋄ Iv{(⍳⍵≢⍛++/⍺)∊+\1+¯1↓0,⍺\⍨1⌽⍵}Bv
1 1 0 0 1 1 1 0

⍝ aplcart/table.tsv:2068 — Sum of digit columns with carry
Is←10 ⋄ Jm←2 3⍴9 9 9 0 0 1 ⋄ Is(⊣{⍬(⊢↓⍨0=⍴)+⌿1 0⌽0,0 ⍺⊤⍵}⍣≡1⊥⊢)Jm
1 0 0 0

⍝ aplcart/table.tsv:2069 — Count partitions of a set of Js objects into Is non-empty subsets: S(Js,Is)
Is←2 ⋄ Js←4 ⋄ Is{⍺>⍵:0 ⋄ ↑⌽1(+\×)⍣(⍵-⍺)⍨⍳⍺}Js
7

⍝ — 2's-complement bit-wise OR; positive operands (aplcart/table.tsv:2070)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(-∨/0>b)⍪∨/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
3 7 3

⍝ — 2's-complement bit-wise AND; positive operands (aplcart/table.tsv:2071)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(-∧/0>b)⍪∧/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
2 0 0

⍝ — 2's-complement bit-wise converse nonimplication; positive operands (aplcart/table.tsv:2072)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(-</0>b)⍪</2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
0 5 2

⍝ — 2's-complement bit-wise implication; positive operands (aplcart/table.tsv:2073)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(-≤/0>b)⍪≤/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
¯2 ¯3 ¯2

⍝ — 2's-complement bit-wise XNOR; positive operands (aplcart/table.tsv:2074)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(-=/0>b)⍪=/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
¯2 ¯8 ¯4

⍝ — 2's-complement bit-wise converse implication; positive operands (aplcart/table.tsv:2075)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(-≥/0>b)⍪≥/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
¯1 ¯6 ¯3

⍝ — 2's-complement bit-wise nonimplication; positive operands (aplcart/table.tsv:2076)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(->/0>b)⍪>/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
1 2 1

⍝ — 2's-complement bit-wise XOR; positive operands (aplcart/table.tsv:2077)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(-≠/0>b)⍪≠/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
1 7 3

⍝ — 2's-complement bit-wise NOR; positive operands (aplcart/table.tsv:2078)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(-⍱/0>b)⍪⍱/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
¯4 ¯8 ¯4

⍝ — 2's-complement bit-wise NAND; positive operands (aplcart/table.tsv:2079)
I←3 2 1 ⋄ J←2 5 2 ⋄ I{⍉2⊥(-⍲/0>b)⍪⍲/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
¯3 ¯1 ¯1

⍝ aplcart/table.tsv:2080 — Count permutations of a set of Js objects that have Is cycles: s(Js,Is)
Is←2 ⋄ Js←4 ⋄ Is{⍺>⍵:0 ⋄ ↑⌽⊃+\⍤×/1,⍨⌽,/⍺↕⍳⍵-1}Js
11

⍝ aplcart/table.tsv:2081 — Reshape (⍴) Y to shape Iv, allowing ¯1 to automatically determine missing length
Iv←2 ¯1 ⋄ Y←⍳6 ⋄ Iv{⍵⍴⍨⍺×@(<∘0)⍨⍵×/⍤⍴⍛÷×/⍺~0}Y
2 3⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:2082 — Iv (k or k₁,k₂) band matrix of size Jv (n or m,n)
1 ¯2 {⍺(≥∘(2⊃⊢,-)∧≤∘↑)⍨-/¨⍳2⍴⍵} 5 6
5 6⍴1 1 1 0 0 0 1 1 1 1 0 0 0 1 1 1 1 0 0 0 1 1 1 1 0 0 0 1 1 1

⍝ aplcart/table.tsv:2083 — Number Spiral of order Js
{⍵ ⍵⍴⍋+\a/(≢a←1↓⌽2/⍳⍵)⍴,∘-⍨1⍵} 5
5 5⍴1 2 3 4 5 16 17 18 19 6 15 24 25 20 7 14 23 22 21 8 13 12 11 10 9

⍝ aplcart/table.tsv:2084 — First Js Catalan numbers (number of ways to nest Js pairs of parentheses)
{⌽↑¨{⍵,⍨⊂1↓+\↑⍵}⍣(⍵-1)⊂⍵↑1} 10   ⍝ 1 1 2 5 14 42 132 429 1430 4862

⍝ aplcart/table.tsv:2085 — “Transpose” of matrix Ym with column fields of width Is
Is←2 ⋄ Ym←3 4⍴⍳12 ⋄ Is{((a×⌽)⍴2 1 3⍉⍵⍴⍨1⌽⍺,⊢)⍵⍴⍛÷a←1,⍺}Ym
2 6⍴1 2 5 6 9 10 3 4 7 8 11 12

⍝ aplcart/table.tsv:2086 — “Transpose” of matrix Ym with column fields of width Is
Is←2 ⋄ Ym←3 4⍴⍳12 ⋄ Is{((a×⌽)⍴2 1 3⍉⍵⍴⍨1⌽⍺,⊢)⍵⍴⍛÷a←1,⍺}Ym
2 6⍴1 2 5 6 9 10 3 4 7 8 11 12

⍝ aplcart/table.tsv:2087 — “Transpose” of matrix Ym with column fields of width Is
Is←2 ⋄ Ym←3 4⍴⍳12 ⋄ Is{((a×⌽)⍴2 1 3⍉⍵⍴⍨1⌽⍺,⊢)⍵⍴⍛÷a←1,⍺}Ym
2 6⍴1 2 5 6 9 10 3 4 7 8 11 12

⍝ aplcart/table.tsv:2088 — “Transpose” of matrix Ym with column fields of width Is
Is←2 ⋄ Ym←3 4⍴⍳12 ⋄ Is{((a×⌽)⍴2 1 3⍉⍵⍴⍨1⌽⍺,⊢)⍵⍴⍛÷a←1,⍺}Ym
2 6⍴1 2 5 6 9 10 3 4 7 8 11 12

⍝ aplcart/table.tsv:2089 — “Transpose” of matrix Ym with column fields of width Is
Is←2 ⋄ Ym←3 4⍴⍳12 ⋄ Is{((a×⌽)⍴2 1 3⍉⍵⍴⍨1⌽⍺,⊢)⍵⍴⍛÷a←1,⍺}Ym
2 6⍴1 2 5 6 9 10 3 4 7 8 11 12

⍝ aplcart/table.tsv:2090 — “Transpose” of matrix Ym with column fields of width Is
Is←2 ⋄ Ym←3 4⍴⍳12 ⋄ Is{((a×⌽)⍴2 1 3⍉⍵⍴⍨1⌽⍺,⊢)⍵⍴⍛÷a←1,⍺}Ym
2 6⍴1 2 5 6 9 10 3 4 7 8 11 12

⍝ aplcart/table.tsv:2091 — “Transpose” of matrix Ym with column fields of width Is
Is←2 ⋄ Ym←3 4⍴⍳12 ⋄ Is{((a×⌽)⍴2 1 3⍉⍵⍴⍨1⌽⍺,⊢)⍵⍴⍛÷a←1,⍺}Ym
2 6⍴1 2 5 6 9 10 3 4 7 8 11 12

⍝ aplcart/table.tsv:2092 — “Transpose” of matrix Ym with column fields of width Is
Is←2 ⋄ Ym←3 4⍴⍳12 ⋄ Is{((a×⌽)⍴2 1 3⍉⍵⍴⍨1⌽⍺,⊢)⍵⍴⍛÷a←1,⍺}Ym
2 6⍴1 2 5 6 9 10 3 4 7 8 11 12

⍝ aplcart/table.tsv:2093 — Linefeed-separated character vector constructed from the rows of Dm (which must be of depth 1) with trailing spaces removed
Cm←2 4⍴'ab  cde '  ⋄ {⍺←•UCS 10 ⋄ 1↓(,1,⌽∨\⌽' '≠⍵)/,⍺,⍵}Cm
•UCS 97 98 10 99 100 101

⍝ aplcart/table.tsv:2094 — Collatz sequence for positive integer Js
{r⊣{2|⍵:1+3×⍵ ⋄ ⍵÷2}⍣{1=r,←⍵}⍵⊣r←⍬} 27
27 82 41 124 62 31 94 47 142 71 214 107 322 161 484 242 121 364 182 91 274 137 412 206 103 310 155 466 233 700 350 175 526 263 790 395 1186 593 1780 890 445 1336 668 334 167 502 251 754 377 1132 566 283 850 425 1276 638 319 958 479 1438 719 2158 1079 3238 1619 4858 2429 7288 3644 1822 911 2734 1367 4102 2051 6154 3077 9232 4616 2308 1154 577 1732 866 433 1300 650 325 976 488 244 122 61 184 92 46 23 70 35 106 53 160 80 40 20 10 5 16 8 4 2 1

⍝ aplcart/table.tsv:2095 — Vector (Iv[1]⍴1),(Jv[1]⍴0),(Iv[2]⍴1),…
Iv←2 1 3 ⋄ Jv←1 2 0 ⋄ Iv{(⍳+/Iv,Jv)∊+\1+¯1↓0,⍵\⍨(⍳+/Iv)∊+\Iv}Jv
1 1 0 1 0 0 1 1 1

⍝ aplcart/table.tsv:2096 — Grade up in each subvector of Bv indicated by Av (Fast ∊⍋¨Av⊂Nv)
1 0 0 0 1 0 0 {(⍋⍵++\⍺\2×⌈/|0,⍵)-⌈\⍺\¯1+⍸⍺} 10 20 40 30 ¯5 ¯10 ¯7
1 2 4 3 2 3 1

⍝ aplcart/table.tsv:2097 — Evaluate Roman numeral
Dv← 'MCMXCIV'  ⋄ {v←(×\1,6⍴5 2)['IVXLCDM'⍳⍵] ⋄ v+.×¯1*</2↕v,0}Dv
1994

⍝ aplcart/table.tsv:2098 — Cumulative sum (+\) of each subvector of Bv indicated by Av (fast ∊+\¨Av⊂Nv)
Av←1 0 1 0 0 ⋄ Bv←1 1 1 0 1 ⋄ Av{(1↓(-/∘(2∘↕))0,z/⍳≢z←1,⍨⍺/⍨⍺∨⍵)-~⍺/⍵}Bv
¯2 ¯2

⍝ aplcart/table.tsv:2099 — Classification of elements Nv into Is ranges of equal size
Is←3 ⋄ Nv←1 2 4 6 7 ⋄ Is{+/((⊢×⍺÷⌈/)⍵-⌊/⍵) ≥⌝ ¯1+⍳⍺}Nv
1 1 2 3 3

⍝ aplcart/table.tsv:2103 — Grade down in each subvector of Bv indicated by Av (Fast ∊⍒¨Av⊂Nv)
1 0 0 0 1 0 0 {(⍋⍵-⍨+\⍺\2×⌈/|0,⍵)-⌈\⍺\¯1+⍸⍺} 10 20 40 30 ¯5 ¯10 ¯7
3 4 2 1 1 3 2

⍝ aplcart/table.tsv:2104 — Grade up (⍋) for sorting subarrays of Y having lengths Iv (≢Y ↔ +/Iv)
Iv←2 3 ⋄ Y←3 1 5 2 4 ⋄ Iv{g⌷⍨⊂⍋(+\(⍳≢⍵)∊+\1,⍺)⌷⍨⊂g←⍋⍵}Y
2 1 4 5 3

⍝ aplcart/table.tsv:2105 — Grade down (⍒) for sorting subarrays of Y having lengths Iv (≢Y ↔ +/Iv)
Iv←2 3 ⋄ Y←3 1 5 2 4 ⋄ Iv{g⌷⍨⊂⍋(+\(⍳≢⍵)∊+\1,⍺)⌷⍨⊂g←⍒⍵}Y
1 2 3 5 4

⍝ aplcart/table.tsv:2106 — Date and time (⎕TS format) to YYYY-MM-DD hh:mm:ss
Jv←2026 9 18 12 34 56 0 ⋄ {'-- ::'@(2+3×⍳5)∊⍕¨(↑,100+1∘↓)6↑⍵}Jv
'2026-09-18 12:34:56'

⍝ aplcart/table.tsv:2107 — Inverted Table Sort (Y[⍋Y;] where Y is unverted Yv)
Yv← (2 1 2⋄ 3 2⍴'bacadb')  ⋄ {⍵⌷¨⍨⊂⊂{⍵[⍋⍺⌷⍨⊂⍵]}/⍵,⊂⍳≢↑⍵}Yv
(1 2 2 ⋄ 3 2⍴'cabadb')

⍝ aplcart/table.tsv:2108 — Less than or equal reduction on each subvector of Bv indicated by Av (Fast ≤/¨Av⊂Bv)
Av←1 0 1 0 0 ⋄ Bv←1 1 1 0 1 ⋄ Av{⍵×/⍛≥(z/1⌽z←⍺/⍨⍺≥z)∧⍺/z←⍵∨x←1⌽⍺}Bv
0 1

⍝ aplcart/table.tsv:2110 — Cut Y into I partitions of equal lengths (with even distribution of variation)
I←3 ⋄ Y←⍳8 ⋄ I{⍵⊂[1]⍨(-/∘⌽∘(2∘↕))0,(⌊1.5+l×⍺÷⍨¯1+⍳⍺)⍸⍳l←≢⍵}Y
(1 2 3 ⋄ 4 5 ⋄ 6 7 8)

⍝ aplcart/table.tsv:2111 — Normally distributed numbers with mean M and standard deviation N; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Zero standard deviation must return the supplied mean vector; tests the degenerate normal-distribution recipe without probabilistic thresholds
M←1 2 3 ⋄ N←0 0 0 ⋄ r←M{⍺+⍵×(0.5*⍨¯2×⍟?0×⍺+⍵)×1○π2×?0×⍺+⍵}N ⋄ M≡r
1

⍝ aplcart/table.tsv:2112 — Is Ns inside closed polygon Mv (using complex points)?
Mv←0 2 2j2 0j2 ⋄ Ns←1j1 ⋄ Mv{⍵∊⍺:1 ⋄ (¯1∊×d)∨1<|+/⍟d←(⊢÷1∘⌽)⍺-⍵}Ns
1

⍝ aplcart/table.tsv:2113 — Vector (Jv[1]+⍳Iv[1]),(Jv[2]+⍳Iv[2]),(Jv[3]+⍳Iv[3]),… (≢Iv ↔ ≢Jv)
Iv←2 3 1 ⋄ Jv←10 20 30 ⋄ Iv{1++\1+((⍳+/⍺)∊+\1,⍺)\⍵-¯1↓1,⍺+⍵}Jv
11 12 21 22 23 31

⍝ aplcart/table.tsv:2114 — Number of areas intersecting areas in N (⍴N ↔ (n × 2 × dim))
N←3 2 2⍴0 0 2 2 1 1 3 3 4 4 5 5 ⋄ {+/∧∘⍉⍨∧/⍵[;a⍴1;]≤2 1 3⍉⍵[;2⍴⍨a←≢⍵;]}N
2 2 1

⍝ aplcart/table.tsv:2115 — Drop first Is (if negative: last |Is) segments from delimited string Dv where the first character is the delimiter
Is←1 ⋄ Dv← '/ab/cd/ef'  ⋄ Is{r←+\⍵=d←↑⍵ ⋄ 0<⍺:⍵/⍨r>⍺ ⋄ ⍵/⍨r≤⍺+⊃⌽r}Dv
'/cd/ef'

⍝ aplcart/table.tsv:2119 — Numeric matrix of all permutations of length Js in lexicographical order
{0=⍵:1 0⍴0 ⋄ ,[⍳2](⍒⍤1 =⌝ ⍨⍳⍵)[;1,1+∇ ⍵-1]} 3
6 3⍴1 2 3 1 3 2 2 1 3 2 3 1 3 1 2 3 2 1

⍝ aplcart/table.tsv:2120 — Sorting rows of matrix Y into ascending order (Fast (⊂∘⍋⌷⊢)⍤1)
Y←2 3⍴3 1 2 6 4 5 ⋄ {s⍴r[g[⍋(,⍉(⌽s←⍴⍵)⍴⍳≢⍵)[g←⍋r←,⍵]]]}Y
2 3⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:2122 — Permutation vector for Is-rotate on each subvector indicated by Bv (Fast ∊Is⌽¨⍵⊂⍳≢Bv)
(p ← 2 {⍋j+(i|⍺)[j←+\⍵]≥+\1-⍵\¯1↓0,i←i-¯1↓0,i←⍸1⌽⍵} 1 0 0 0 0 1 0 0 0 ⋄ 'abcdeABCD'[p])
(3 4 5 1 2 8 9 6 7 ⋄ 'cdeabCDAB')

⍝ aplcart/table.tsv:2123 — Clamp non-negative N to fit in ⍕ field Iv[1 2]
Iv←5 2 ⋄ N←0 0.01 12.34 123.45 ⋄ Iv{1(⊃⌊⍵⌈↓)(2 2⍴¯1 1 1 ¯0.1)+.×10*(-↑⌽⍺),-/⍺+⍺>99 0}N
0 0.01 12.34 99.99

⍝ aplcart/table.tsv:2124 — Progressive without (~) without replacement
X←1 2 1 3 ⋄ Y←1 1 1 2 ⋄ X{⍺⌷⍨⊂⍺⍳⍤≢⍛~⍺(R⍨⍳R←≢⍤⊢⍴∘⍋∘⍋⍺⍳⍪⍨)⍵}Y
1⍴3

⍝ aplcart/table.tsv:2125 — Progressive intersection (∩) without replacement
X←1 2 1 3 ⋄ Y←1 1 1 2 ⋄ X{⍺⌷⍨⊂⍺⍳⍤≢⍛∩⍺(R⍨⍳R←≢⍤⊢⍴∘⍋∘⍋⍺⍳⍪⍨)⍵}Y
1 2 1

⍝ aplcart/table.tsv:2126 — Intersperse: Insert cell X between major cells in Y
X←9 ⋄ Y←1 2 3 ⋄ X{⍺⍴⍨∘⍴@(⍸m|⍛/m<0)⊢⍵⍀⍨m←(¯1+2×≢⍵)⍴1,-≢⍺}Y
1 9 2 9 3

⍝ aplcart/table.tsv:2127 — Length of longest common substring
Cv← 'abcde'  ⋄ Dv← 'abfde'  ⋄ Cv{↑⌽⊃(⊢⌈(⌈\(⍵=⊣)+0,¯1↓⊢))/⍺⌽⍛,⊂0⍨¨⍵}Dv
4

⍝ aplcart/table.tsv:2129 — Multiplicative inverse of Js modulo Is (fast)
Is←7 ⋄ Js←3 ⋄ Is{⍺|↑⍵{0=⍵:1 0 ⋄ (⍵∇⍵|⍺)+.×0 1,⍪1,-⌊⍺÷⍵}⍺}Js
5

⍝ aplcart/table.tsv:2131 — Moving all blanks to end of each row (fast (~,∩)∘' '⍤1)
D← 2 4⍴'a b  cd '  ⋄ {⍵⍴⍛⍴(,(+/b) >⌝ ¯1+⍳¯1↑⍴⍵)\⍵/⍨⍥,b←⍵≠' '}D
2 4⍴'ab  cd  '

⍝ aplcart/table.tsv:2132 — Tesselate: Cut Y into tiles of size Iv (padding Y if necessary); dfns display import/wrappers omitted to test underlying arrays
2 3 {⊂[2×⍳k](,s,⍪r)⍴⍵↑⍨r×s←⌈R÷r←1+(-k←≢R←⍴⍵)↑⍺-1} 4 5⍴•A
2 2⍴(2 3⍴'ABCFGH' ⋄ 2 3⍴'DE IJ ' ⋄ 2 3⍴'KLMPQR' ⋄ 2 3⍴'NO ST ')

⍝ aplcart/table.tsv:2133 — ASCII frame a matrix
Dm← 2 3⍴'abcdef'  ⋄ {'+'@(⊂1 1)∘⌽∘⍉⍣4⊃(⍪∘⌽∘⍉)/'-|-|',⊂⍕⍵}Dm
4 5⍴'+---+|abc||def|+---+'

⍝ aplcart/table.tsv:2134 — Tesselate: Distribute Y's elements into evenly-sized tiles in an array of shape Iv (padding Y if necessary); dfns display import/wrappers omitted to test underlying arrays
2 3 {⊂[¯1+2×⍳k](,s,⍪r)⍴⍵↑⍨r×s←⌈R÷r←1+(-k←≢R←⍴⍵)↑⍺-1} 4 5⍴•A
2 3⍴(2 2⍴'ADKN' ⋄ 2 2⍴'BELO' ⋄ 2 2⍴'C M ' ⋄ 2 2⍴'FIPS' ⋄ 2 2⍴'GJQT' ⋄ 2 2⍴'H R ')

⍝ aplcart/table.tsv:2135 — Unicode frame a matrix
Dm← 2 3⍴'abcdef'  ⋄ {⊣@(⊂1 1)∘⌽∘⍉/'┌┐┘└',⊂⍪∘⌽∘⍉/'─│─│',⊂⍕⍵}Dm
4 5⍴'┌───┐│abc││def│└───┘'

⍝ aplcart/table.tsv:2136 — Indices of smallest Is elements of Nv (Fast Is↑⍋Nv); Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Is←3 ⋄ Yv←8 1 6 1 9 2 7 2 5 ⋄ r←Is{i←⍸⍵≤⍺⊃⊂⍤⍋⍛⌷⍵[(⌈0.5*⍨⍺×n)?n←≢⍵] ⋄ i[⍺↑⍋⍵[i]]}Yv ⋄ r≡Is↑⍋Yv
1

⍝ aplcart/table.tsv:2137 — Indices of largest Is elements of Nv (Fast Is↑⍒Nv); Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Is←3 ⋄ Yv←8 1 6 1 9 2 7 2 5 ⋄ r←Is{i←⍸⍵≥⍺⊃⊂⍤⍒⍛⌷⍵[(⌈0.5*⍨⍺×n)?n←≢⍵] ⋄ i[⍺↑⍒⍵[i]]}Yv ⋄ r≡Is↑⍒Yv
1

⍝ aplcart/table.tsv:2138 — Decommenting a matrix representation of a function (⎕CR)
Dm← 2 8⍴'x←1 ⍝abc''⍝'' ⍝def'  ⋄ {a∨/⍛⌿⍵⍴⍛⍴a,⍛\⍵/⍨⍥,a←∧\('⍝'≠⍵)∨≠\⍵=''''}Dm
2 8⍴'x←1     ''⍝''     '

⍝ aplcart/table.tsv:2139 — Descending coefficients of polynomial with roots Nv
Nv←1 2 3 ⋄ {⌽((0,⍳≢⍵) =⌝ +⌿~b)+.×(-⍵)×.*b←t⊤¯1+⍳×/t←2⍴⍨≢⍵}Nv
1 ¯6 11 ¯6

⍝ aplcart/table.tsv:2140 — Accurately sum a vector of floating point numbers
Nv←1e16 1 ¯1e16 ⋄ {(⊢/t)++/((M~b)-1↓t)+(M←⍵∘×+c×~)b←⍵≤⍥|c←¯1↓t←+\0,⍵}Nv
1

⍝ aplcart/table.tsv:2141 — Numeric matrix of all permutations of length Js in unspecified order (faster than generating in lexicographical order)
Js←3 ⋄ {1+⍉⊃{1=⍵:,⊂,0 ⋄ (,/)¨(⍳⍵)⌽¨⊂(∇,∘⊂!⍴⊢)⍵-1}⍵}Js
6 3⍴1 3 2 2 3 1 3 2 1 3 1 2 2 1 3 1 2 3

⍝ aplcart/table.tsv:2142 — Editing Dv with Cv ('/' to delete and ',' to insert)
Cv← ' /,xy'  ⋄ Dv← 'abcd'  ⋄ Cv{((~(≢b↑⍵)↑'/'=⍺)/b↑⍵),(1↓b↓⍺),⍵↓⍨b←+/∧\⍺≠','}Dv
'axycd'

⍝ aplcart/table.tsv:2143 — Concatenate same-rank arrays X and Y along axis Is (padding if necessary)
X←2 2⍴⍳4 ⋄ Is←2 ⋄ Y←3 1⍴5 6 7 ⋄ X(Is{(⍺↑⍨a[⍶]@⍶⊢s),[⍶]⍵↑⍨w[⍶]@⍶⊢s←(a←⍴⍺)⌈w←⍴⍵})Y
3 3⍴1 2 5 3 4 6 0 0 7

⍝ aplcart/table.tsv:2145 — Take first Is (if negative: last |Is) segments from delimited string Dv where the first character is the delimiter
Is←2 ⋄ Dv← '/ab/cd/ef'  ⋄ Is{s←↑⌽r←+\⍵=d←↑⍵ ⋄ 0<⍺:(⍵/⍨r≤⍺),d⍴⍨0⌈⍺-s ⋄ (d⍴⍨0⌈-⍺+s),⍵/⍨r>⍺+s}Dv
'/ab/cd'

⍝ aplcart/table.tsv:2146 — Determinant of any square matrix
Nm←3 3⍴1 2 3 0 4 5 1 0 6 ⋄ {↑,{-((+\-+/)@{ =⌝ ⍨⍳≢⍵}⍵× ≤⌝ ⍨⍳≢⍵)+.×⍺}/(≢⍴⊂)⍵}Nm
22

⍝ aplcart/table.tsv:2147 — Chinese Remainder Theorem for moduli Iv and desired remainders Jv
(3 5 7 {m|⍵+.×⍺(⊣×⊢|∘↑{0=⍵:1 0 ⋄ (⍵∇⍵|⍺)+.×0 1,⍪1,-⌊⍺÷⍵})¨⍨⍺÷⍨m←×/⍺} 2 3 2 ⋄ 11 12 13 {m|⍵+.×⍺(⊣×⊢|∘↑{0=⍵:1 0 ⋄ (⍵∇⍵|⍺)+.×0 1,⍪1,-⌊⍺÷⍵})¨⍨⍺÷⍨m←×/⍺} 10 4 12)
23 1000

⍝ aplcart/table.tsv:2148 — Fast Fourier Transformation; Move the misplaced opening brace after the reshape dfn so it becomes the operand of the intended dop; Eight-point transform; original recipe retained
Nv←1 2 3 4 5 6 7 8 ⋄ {⍵⍴⍨2⍴⍨2⍟⍴⍵}{,(⍶×\1,1↓(2÷⍨⍴⍵)⍴¯1*2÷⍴⍵){(⊣/⍺)∇⍣(×m)⊢(+⌿⍵),[m+0.5]⍺×[⍳m←≢⍴⍺]-⌿⍵}⍶ ⍵}Nv
36 ¯4.000000000000002j¯9.65685424949238 ¯4.000000000000001j¯4 ¯4.000000000000001j¯1.65685424949238 ¯4 ¯4j1.656854249492381 ¯3.999999999999999j4 ¯3.999999999999997j9.65685424949238

⍝ aplcart/table.tsv:2149 — Inverse Fast Fourier Transformation; Move the misplaced opening brace after the reshape dfn so it becomes the operand of the intended dop; Eight-point transform; original recipe retained
Nv←1 2 3 4 5 6 7 8 ⋄ {⍵⍴⍨2⍴⍨2⍟⍴⍵}{⍵÷∘⍴⍨,(⍶+×\1,1↓(2÷⍨⍴⍵)⍴¯1*2÷⍴⍵){(⊣/⍺)∇⍣(×m)⊢(+⌿⍵),[m+0.5]⍺×[⍳m←≢⍴⍺]-⌿⍵}⍶ ⍵}Nv
4.5 ¯0.5000000000000002j1.207106781186547 ¯0.5000000000000001j0.5 ¯0.5000000000000001j0.2071067811865475 ¯0.5 ¯0.5j¯0.2071067811865476 ¯0.4999999999999999j¯0.5 ¯0.4999999999999996j¯1.207106781186547

⍝ aplcart/table.tsv:2151 — Where: Execute f on condition B mask
f←- ⋄ B←1 0 1 0 1 ⋄ Y←3 1 5 2 4 ⋄ f@{B}Y
¯3 1 ¯5 2 ¯4

⍝ aplcart/table.tsv:2152 — Function limit 'trajectory'; Concrete APLcart library call; self-contained setup from april/libraries/dfns/power/demo.lisp:41. Local definitions replace the dfns namespace; no namespace feature implied
traj ← {
  ⍺←⍬
  (⊂⍵)∊⍺:⍺
  (⍺,⊂⍵)∇ ⍶ ⍵
}
result←{2|⍵:1+3×⍵ ⋄ ⍵÷2}traj 7
⍝ =>
7 22 11 34 17 52 26 13 40 20 10 5 16 8 4 2 1

⍝ aplcart/table.tsv:2153 — Associative vector scan; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:330. Local definitions replace the dfns namespace; no namespace feature implied
ascan ← {
  2>0⊥⍴⍵:⍵
  ⌽⊃⍶{(⊂(↑⍵)⍶ ⍺),⍵}/⌽(⊂∘↑¨↓⍵),⊃1↓¨↓⍵
}
result←+ascan ⍳10
⍝ =>
1 3 6 10 15 21 28 36 45 55

⍝ aplcart/table.tsv:2155 — Binary search; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:27. Local definitions replace the dfns namespace; no namespace feature implied
bsearch ← {
  ¯1≤-/⍵:1↑⍵+2-+/⍶¨⍵
  mid←⌈0.5×+/⍵
  ⍶ mid:∇(1↑⍵),mid
         ∇ mid ,1↓⍵
}
result←{⍵≥5}bsearch 3 7
⍝ =>
1⍴5

⍝ aplcart/table.tsv:2156 — Relationship between point and k-cell; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:236. Local definitions replace the dfns namespace; no namespace feature implied
kcell ← {
  ⍺←(≢⍵)/2 1⍴0 1
  b←,[(2=⍴⍴⍺)/1]⍺
  p←((2⌊⍴⍴⍺)↓1 1,⍴⍵)⍴⍵
  d←⊃,¨⍶⌿×-b,.-p
  ⍶/⍬:⌈/d ⋄ 5⊥⍉d
}
result←3 ×kcell ⍳5
⍝ =>
¯1 ¯1 0 1 1

⍝ aplcart/table.tsv:2157 — Function power; Concrete APLcart library call; self-contained setup from april/libraries/dfns/power/demo.lisp:37. Local definitions replace the dfns namespace; no namespace feature implied
pow ← { (⍶⍣⍺)⍵ }
result←6 {'<',⍵,'>'}pow 'wow' 
⍝ =>
'<<<<<<wow>>>>>>'

⍝ aplcart/table.tsv:2158 — Fold (reduce) from the left; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:129. Local definitions replace the dfns namespace; no namespace feature implied
foldl ← { ⍶⍨/(⌽⍵),⊂⍺ }
result←'abracadabra' {⍺~⍵⊃⍺}foldl 1 2
⍝ =>
'bcdb'

⍝ aplcart/table.tsv:2159 — Generic depth-first parent-last tree; Concrete APLcart library call with self-contained April dfns definitions and standard operand names; no namespace support implied
ravt ← {
  (∇⍨/⌽(⊂⍺),⍺ ⍹ ⍵)⍶ ⍵
}
tree←1(2 3)(4(5 6)) ⋄ acc←{⍺,⊂⍵} ⋄ subs←{1≥|≡⍵:⍬ ⋄ ⍵}
result←⍬ (acc ravt subs)tree
⍝ =>
1 (2 3) 4 (5 6) (4 (5 6)) (1 (2 3) (4 (5 6)))

⍝ aplcart/table.tsv:2160 — Generic depth-first parent-first tree; Concrete APLcart library call with self-contained April dfns definitions and standard operand names; no namespace support implied
trav ← {
  ∇⍨/⌽(⊂⍺ ⍶ ⍵),⍺ ⍹ ⍵
}
tree←1(2 3)(4(5 6)) ⋄ acc←{⍺,⊂⍵} ⋄ subs←{1≥|≡⍵:⍬ ⋄ ⍵}
result←⍬ (acc trav subs)tree
⍝ =>
(1 (2 3) (4 (5 6))) 1 (2 3) (4 (5 6)) 4 (5 6)

⍝ aplcart/table.tsv:2161 — List traversal; Concrete APLcart library call with self-contained April dfns definitions and standard operand names; no namespace support implied
ltrav ← {
  '∘'≡head tail←⍵:⍺
  (⍺ ⍶ head)∇ tail
}
list ← { {⍺ ⍵}/⍵,'∘' }
result←0 {⍺+1}ltrav list 'abc' 
⍝ =>
3

⍝ aplcart/table.tsv:2163 — Partitioned reduction; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:209. Local definitions replace the dfns namespace; no namespace feature implied
pred ← { ⊃⍶/¨(⍺/⍳⍴⍺)⊆⍵ }
result←2 3 3 2 +pred ⍳10
⍝ =>
3 12 21 19

⍝ aplcart/table.tsv:2164 — Alternant; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:194. Local definitions replace the dfns namespace; no namespace feature implied
alt ← {
  r c←⍴⍵
  0=r:⍹⌿,⍵
  1≥c:⍶⌿,⍵
  M←~⍤1 0⍨⍳r
  ⍵[;1]⍶.⍹∇¨⊂[2 3]⍵[M;1↓⍳c]
}
result←-alt×2 2⍴2 3 4 5
⍝ =>
¯2

⍝ aplcart/table.tsv:2169 — Function power limit (fixpoint); Concrete APLcart library call; self-contained setup from april/libraries/dfns/power/demo.lisp:29. Local definitions replace the dfns namespace; no namespace feature implied
limit ← {
  ⍵ ⍶{
    ⍺≡⍵:⍵
    ⍵ ∇ ⍶ ⍵
  }⍶ ⍵
}
result←{1⌈⍵÷2}limit 16
⍝ =>
1

⍝ aplcart/table.tsv:2170 — Accumulating reduction; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:19. Local definitions replace the dfns namespace; no namespace feature implied
acc ← { ⍶{(⊂⍺ ⍶↑⍬⍴⍵),⍵}/1↓{⍵,⊂⍬⍴⍵}¯1⌽⍵ }
result←+acc ⍳4
⍝ =>
10 9 7 4

⍝ aplcart/table.tsv:2171 — Conditional function power; Concrete APLcart library call; self-contained setup from april/libraries/dfns/power/demo.lisp:89. Local definitions replace the dfns namespace; no namespace feature implied
until ← {
  ⍶{
    ⍹ ⍵:⍵
    ∇ ⍶ ⍵
  }⍹ ⍶ ⍵
}
result←{⍵,'.'}until{12=⍴⍵}'Note' 
⍝ =>
'Note........'

⍝ aplcart/table.tsv:2172 — Conditional function power; Concrete APLcart library call; self-contained setup from april/libraries/dfns/power/demo.lisp:78. Local definitions replace the dfns namespace; no namespace feature implied
while ← {
  ⍹ ⍵:∇ ⍶ ⍵
  ⍵
}
result←,∘'.'while(12>⍴)'Note' 
⍝ =>
'Note........'

⍝ aplcart/table.tsv:2180 — Associative higher rank scan; Concrete APLcart library call with self-contained April dfns definitions and standard operand names; no namespace support implied
ascan ← {
  2>0⊥⍴⍵:⍵
  ⌽⊃⍶{(⊂(↑⍵)⍶ ⍺),⍵}/⌽(⊂∘↑¨↓⍵),⊃1↓¨↓⍵
}
ascana ← {
  ⍺←⍬⍴⌽⍳⍴⍴⍵
  ⊃[⍺-0.1](⍶ ascan)¨↓[⍺]⍵
}
result←+ascana 2 3⍴⍳6
⍝ =>
2 3⍴1 3 6 4 9 15

⍝ aplcart/table.tsv:2183 — Select and modify; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:227. Local definitions replace the dfns namespace; no namespace feature implied
disp ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t} ⋄  ⋄ ⍺←⍬
  dec ctd←2↑⍺
  box←{
    isor ⍵:format⊂⍵
    1=≡,⍵:dec open format dec open ⍵
    mat←matr 1/dec open ⍵
    r c←×⍴mat
    dec<0∊r c:c/r⌿∇ 1 open mat
    subs←aligned ∇¨mat
    (≢⍴⍵)gaps ⍵ plane subs
  }
  aligned←{
    rows cols←sepr⍴¨⍵
    sizes←(⌈/rows) ,⌝ ⌈⌿cols
    ctd=0:sizes↑¨⍵
    v h←sepr⌈0.5×⊃(⍴¨⍵)-sizes
    v⊖¨h⌽¨sizes↑¨⍵
  }
  gaps←{
    ⍺≤2:⍵
    subs←(⍺-1)∇¨⍵
    width←↑⌽⍴↑subs
    fill←(⍺ width-3 0)⍴' '
    ⊃{⍺⍪fill⍪⍵}/1 open subs
  }
  plane←{
    2<⍴⍴⍺:⍺ join ⍵
    odec←(dec shape ⍺)outer ⍵
    idec←inner ⍺
    (odec,idec)collect ⍵
  }
  join←{
    sep←(≢⍵)÷1⌈≢⍺
    split←(0=sep|¯1+⍳≢⍵)⊂[1]⍵
    (⊂⍤¯1⊢⍺)plane¨split
  }
  outer←{
    sizes←1 0{↑↓(⍉⍣⍺)⍵}¨sepr⍴¨⍵
    sides←sizes/¨¨'│─'
    bords←dec↓¨'├┬'glue¨sides
    ,¨/('┌' '')⍺ bords'└┐'
  }
  inner←{
    deco←{(type ⍵),1 shape ⍵}
    sepr deco¨matr dec open ⍵
  }
  collect←{
    lft top tt vv hh←⍺
    cells←vv right 1 open tt hh lower ⍵
    boxes←(dec∨0∊⍴⍵)open cells
    lft,top⍪⍪⌿,/boxes
  }
  right←{
    types←2⊥¨(⍳⍴⍵)=⊂⍴⍵
    chars←'┼┤┴┘'[1+types]
    rgt←{⍵,(-≢⍵)↑(≢⍵)1 1/'│',⍺}
    ((matr 1 open ⍺),¨chars)rgt¨⍵
  }
  lower←{
    split←{((¯2+2⊃⍴⍵)/'─')glue ⍺}
    bot←{⍵⍪(-2⊃⍴⍵)↑⍺ split ⍵}
    (matr,¨/⍺)bot¨matr ⍵
  }
  type←{
    dec<|≡⍵:'─'
    isor ⍵:'∇'
    sst←{
      0=dec×⍴⍴⍵:'─'
      (1+↑⍵∊'¯',•D)⊃'#~'
    }∘⍕
    0=≡⍵:sst ⍵
    {(1+1=⍴⍵)⊃'+'⍵}∪,sst¨dec open ⍵
  }
  shape←{     
    dec≤0=⍴⍴⍵:⍺/¨'│─'
    cols←(1+×¯1↑⍴⍵)⊃'⊖→'
    rsig←(1+××/¯1↓⍴⍵)⊃'⌽↓'
    rows←(3⌊⍴⍴⍵)⊃'│'rsig'⍒'
    rows cols
  }
  matr←{⊃,↓⍵}
  sepr←{+/¨1⊂⊃⍵}
  open←{(⍺⌈⍴⍵)↑⍵}
  isor←{1 ⍬≡(≡⍵)(⍴⍵)}
  glue←{0=⍴⍵ : ⍵ ⋄ ⍺{⍺,⍶,⍵}/⍵}
  isor ⍵:format⊂⍵
  1=≡,⍵:format ⍵
  box ⍵
}
sam ← {
  ⍺←⊢
  array←⍵
  (⍺ ⍶ array)←⍹ ⍺ ⍶ array
  array
}
ucase ← {
  lc←'abcdefghijklmnopqrstuvwxyzåäöàæéñøü'
  uc←'ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖÀÆÉÑØÜ'
  (⍴⍵)⍴(uc,,⍵)[(lc,,⍵)⍳⍵]
}
vex←('one' 'two' 'three'⋄ 'alpha' 'beta' 'gamma'⋄ 'red' 'blue' 'green')
result←2 ↑sam⌽ vex
⍝ =>
(('alpha' ⋄ 'beta' ⋄ 'gamma') ⋄ ('one' ⋄ 'two' ⋄ 'three') ⋄ ('red' ⋄ 'blue' ⋄ 'green'))

⍝ aplcart/table.tsv:2184 — Operand function applied to argument rows; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:218. Local definitions replace the dfns namespace; no namespace feature implied
rows ← {
  1<|≡⍵:∇¨⍵
  ⍶⍤1⊢⍵
}
result←{'<',⍵,'>'}rows 'ten' 'a' 'penny' 
⍝ =>
('<ten>' ⋄ '<a>' ⋄ '<penny>')

⍝ aplcart/table.tsv:2185 — Scalar pervasion; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:207. Local definitions replace the dfns namespace; no namespace feature implied
perv ← { ⍺←⊢
  1=≡⍺ ⍵ ⍵:⍺ ⍶ ⍵
           ⍺ ∇¨⍵
}
result←1(2 3),perv(4 5)6
⍝ =>
((1 4 ⋄ 1 5) ⋄ (2 6 ⋄ 3 6))

⍝ aplcart/table.tsv:2190 — Simple Binary Search Trees; Concrete APLcart library call; self-contained setup from april/libraries/dfns/tree/demo.lisp:285. Local definitions replace the dfns namespace; no namespace feature implied
sbst ← { 
  put←{
    ⍺≡0:(⍵(0 0))0
    ((nxt _)subs)(key _)←⍺ ⍵
    nxt≡key:(⍵ subs)0
    ⍺ ∇ search ⍵
  }
  get←{
    ⍺≡0:⍺'?'
    ((nxt val)_)(key _)←⍺ ⍵
    nxt≡key:⍺ val
    ⍺ ∇ search ⍵
  }
  rem←{
    ⍺≡0:⍺ 0
    ((nxt _)subs)(key _)←⍺ ⍵
    ~nxt≡key:⍺ ∇ search ⍵
    0 0≡subs:0 0
    0∊subs:((subs⍳0)⊃⌽subs)0
    (rrot ⍺)∇ ⍵
  }
  rrot←{
    B((A(p q))r)←⍵
    A(p(B(q r)))
  }
  search←{
    inf(lft rgt)←⍺
    dir←1-2×>/⍋⊃↑¨inf ⍵
    wise←{(2×⍺)↑3⍴⍵}
    _ nxt←dir wise lft rgt
    sub val←nxt ⍶ ⍵
    subs←dir wise lft sub rgt
    (inf subs)val
  }
  fmt←{
    null←0 0⍴''
    ⍵≡0:null
    (key val)subs←⍵
    key_val←⊃,/⍕¨key'='val
    fmts←{⊖⍵}\'┌└'{
      0 0≡⍴⍵:⍵
      mask←∧\' '=↑↓⌽⍉⍵
      ⍉⌽⊃(⊂⌽⍺,mask/'│'),↓⌽⍉⍵
    }¨{⊖⍵}\∇¨subs
    case←~null null≡¨fmts
    join←(1+2⊥case)⊃'∘┐┘┤'
    join≡'∘':⊃,↓key_val
    dent←' '⊣¨key_val
    pads←{↓,/dent,⊂⍵}¨fmts
    ⊃{⍺,(↓key_val,join),⍵}/pads
  }
  vec←{
    ⍵≡0:⍬
    key_val(lft rgt)←⍵
    (∇ lft),(⊂key_val),∇ rgt
  }
  chk←{
    0=≡⍵:(0≡⍵)0 0 0 ⍬
    (key _)subs←⍵
    stats←(⍺+1)∇¨subs
    oks szs dps hts krs←↓⍉⊃stats
    keys←key{⍺,(⊂⍶),⍵}/krs
    okkey←{⍵≡⍳⍴⍵}⍋⊃keys
    okstr←2 2≡(⍴⍵),⍴↑⌽⍵
    ok←okkey∧okstr∧∧/oks
    sz←1++/szs
    dp←⍺++/dps
    ht←1+⌈/hts
    kr←⌽2⍴¯1⌽keys
    ⍺>0:ok sz dp ht kr
    ok sz(⌊0.5+dp÷sz)ht
  }
  bal←{
    vine size←0 0 list ⍵
    log←⌊2⍟size+1
    rem←1+size-2*log
    cmps←¯2+2*⍳log
    cmp/(1↓cmps,2×rem),⊂vine
  }
  cmp←{
    ⍺=0:⍵
    inf(lft rgt)←⍵
    lev←(⍺-1)∇ lft
    2|⍺:inf(lev rgt)
    rrot inf(lev rgt)
  }
  list←{
    0≡⍵:⍺
    inf(lft rgt)←⍵
    lev s←⍺ ∇ lft
    (inf(lev 0))(s+1)∇ rgt
  }
  op←⍶
  '∪'≡op:↑⍺ put ⍵
  '⍎'≡op:↑⌽⍵ get ⍺ 0
  '~'≡op:↑⍺ rem ⍵ 0
  '⍕'≡op:fmt ⍵
  '∊'≡op:vec ⍵
  '?'≡op:4↑0 chk ⍵
  '='≡op:bal ⍵
}
foldl ← { ⍶⍨/(⌽⍵),⊂⍺ }
put←'∪' sbst ⋄ get←'⍎' sbst ⋄ rem←'~' sbst ⋄ fmt←'⍕' sbst ⋄ chk←'?' sbst ⋄ vec←'∊' sbst ⋄ bal←'=' sbst ⋄ tree←0∘(put foldl)
result←'?' sbst tree ⍳7
⍝ =>
1 7 3 7

⍝ aplcart/table.tsv:2191 — Splay trees; Concrete APLcart library call; self-contained setup from april/libraries/dfns/tree/demo.lisp:893. Local definitions replace the dfns namespace; no namespace feature implied
splay ← { 
  wise←{(2×⍶)↑3⍴⍵}
  put←{
    ⍺≡0:⍵(0 0)
    ((nxt _)subs)(key _)←⍺ ⍵
    nxt≡key:⍵ subs
    ⍺ ∇ search ⍵
  }
  rem←{
    ⍺≡0:0
    ((nxt _)subs)(key _)←⍺ ⍵
    ~nxt≡key:⍺ ∇ search ⍵
    0 0≡subs:0
    0∊subs:(subs⍳0)⊃⌽subs
    (⍺ rot 1)∇ ⍵
  }
  search←{
    inf(lft rgt)←⍺
    dir←1-2×>/⍋⊃↑¨inf ⍵
    _ nxt←dir wise lft rgt
    sub←nxt ⍶ ⍵
    inf(dir wise lft sub rgt)
  }
  get←{
    ⍺≡0:0 0 0
    (key val)(lft rgt)←⍺
    key≡⍵:val ⍺ ⍬
    dir←1-2×>/⍋⊃key ⍵
    _ nxt←dir wise lft rgt
    rslt sub path←nxt ∇ ⍵
    ∆path←dir,path
    cand←(key val)(lft sub rgt)
    tree←(dir wise\cand)bal ∆path
    rslt tree ∆path
  }
  bal←{
    2≠⍴⍵:⍺
    pos neg←1 ¯1×↑⍵
    =/⍵:(⍺ rot neg)rot neg
    C(BpA s)←neg wise\⍺
    ∆C←neg wise\C((BpA rot pos)s)
    ∆C rot neg
  }
  rot←{
    B(Apq r)←⍵ wise\⍺
    A(p q)←⍵ wise\Apq
    Bqr←⍵ wise\B(q r)
    ⍵ wise\A(p Bqr)
  }
  vec←{
    ⍵≡0:⍬
    key_val(lft rgt)←⍵
    (∇ lft),(⊂key_val),∇ rgt
  }
  lift←{
    val root path←⍵
    0∊path:
    1≠⍴path:val root
    val(root rot-↑path)
  }
  fmt←{
    null←0 0⍴''
    ⍵≡0:null
    (key val)subs←⍵
    key_val←⊃,/⍕¨key'='val
    fmts←{⊖⍵}\'┌└'{
      0 0≡⍴⍵:⍵
      mask←∧\' '=↑↓⌽⍉⍵
      ⍉⌽⊃(⊂⌽⍺,mask/'│'),↓⌽⍉⍵
    }¨{⊖⍵}\∇¨subs
    case←~null null≡¨fmts
    join←(1+2⊥case)⊃'∘┐┘┤'
    join≡'∘':⊃,↓key_val
    dent←' '⊣¨key_val
    pads←{↓,/dent,⊂⍵}¨fmts
    ⊃{⍺,(↓key_val,join),⍵}/pads
  }
  dep←{
    ⍺≡0:0
    (key val)subs←⍺
    key≡⍵:1
    dir←1-2×>/⍋⊃key ⍵
    _ sub←dir wise subs
    {⍵+×⍵}sub ∇ ⍵
  }
  chk←{
    0=≡⍵:(0≡⍵)0 0 0 ⍬
    (key _)subs←⍵
    stats←(⍺+1)∇¨subs
    oks szs dps hts krs←↓⍉⊃stats
    keys←key{⍺,(⊂⍶),⍵}/krs
    okkey←{⍵≡⍳⍴⍵}⍋⊃keys
    okstr←2 2≡(⍴⍵),⍴↑⌽⍵
    ok←okkey∧okstr∧∧/oks
    sz←1++/szs
    dp←⍺++/dps
    ht←1+⌈/hts
    kr←⌽2⍴¯1⌽keys
    ⍺>0:ok sz dp ht kr
    ok sz(⌊0.5+dp÷sz)ht
  }
  op←⍶
  '∪'≡op:⍺ put ⍵
  '⍎'≡op:lift ⍵ get ⍺
  '~'≡op:⍺ rem ⍵ 0
  '?'≡op:4↑0 chk ⍵
  '⍕'≡op:fmt ⍵
  '∊'≡op:vec ⍵
  '≡'≡op:⍵ dep ⍺
}
foldl ← { ⍶⍨/(⌽⍵),⊂⍺ }
put←'∪' splay ⋄ get←'⍎' splay ⋄ rem←'~' splay ⋄ fmt←'⍕' splay ⋄ chk←'?' splay ⋄ vec←'∊' splay ⋄ dep←'≡' splay ⋄ tree←0∘(put foldl)
result←'?' splay tree ⍳7
⍝ =>
1 7 3 7

⍝ aplcart/table.tsv:2192 — Red-black trees; Concrete APLcart library call; self-contained setup from april/libraries/dfns/tree/demo.lisp:518. Local definitions replace the dfns namespace; no namespace feature implied
redblack ← { 
  ins←{
    ⍺≡0:base ⍵ 1(0 0)
    ((nxt _)red subs)(key _)←⍺ ⍵
    nxt≡key:done ⍵ red subs
    node path←⍺ ∇ search ⍵
    2≠⍴path:node path
    p c←path
    ~node isred p:node path
    node isred-p:node insRR p
    p=c:node insRBo p
    (node rot∘p sub p)insRBo p
  }
  insRR←{
    g←flip ⍺
    u←g flip sub ⍵
    base u flip sub-⍵
  }
  insRBo←{
    g←flip ⍺
    p←g rot-⍵
    done flip p
  }
  get←{
    ⍺≡0:
    (nxt val)_ subs←⍺
    nxt≡⍵:val
    ((1+>/⍋⊃⍵ nxt)⊃subs)∇ ⍵
  }
  rem←{
    ⍺≡0:done 0
    (nxt _)red(lft rgt)←⍺
    ~nxt≡↑⍵:bal ⍺ ∇ search ⍵
    0≡lft:bal rgt rep red
    0≡rgt:bal lft rep red
    kv←left rgt
    sub path←rgt ∇ kv
    bal(kv red(lft sub))(1,path)
  }
  left←{
    kv _(lft _)←⍵
    0≡lft:kv
    ∇ lft
  }
  rep←{
    ⍵:done ⍺
    ⍺≡0:dblk 0
    inf red subs←⍺
    red:done inf 0 subs
    dblk ⍺
  }
  bal←{
    sub path←⍵
    2≠⍴path:⍵
    dir ddblk←path×¯1 1
    ddblk≠0:⍵
    sub isred dir:sub balR dir
    sub balB dir
  }
  balR←{
    p0←flip ⍺
    p1←p0 rot-⍵
    p2←flip p1
    Pinf Pred Psubs←p2
    N S←⍵ wise Psubs
    N_ path←N balB ⍵
    P_subs←⍵ wise N_ S
    (Pinf Pred P_subs)path
  }
  balB←{
    near far←⍺∘isred¨⍵×1,¨¯1 1
    near⍱far:⍺ balBbb ⍵
    far:⍺ balB_r ⍵
    ⍺ balBrb ⍵
  }
  balBbb←{
    pred←⍺ isred ⍬
    p0←⍺ flip sub ⍵
    pred:done flip p0
    dblk p0
  }
  balB_r←{
    colr←{kv _ lr←⍺ ⋄ kv ⍵ lr}
    pred←⍺ isred ⍬
    p0←⍺ colr 0
    p1←p0 rot-⍵
    p2←p1 flip sub ⍵
    p3←p2 colr pred
    done p3
  }
  balBrb←{
    p0←⍺ flip sub ⍵
    p1←p0 rot∘⍵ sub ⍵
    p2←p1 flip sub ⍵
    p2 balB_r ⍵
  }
  root←{
    0≡↑⍵:0
    (inf _ subs)_←⍵
    inf 0 subs
  }
  fmt←{
    null←⊃,↓'[∘]'
    ⍵≡0:null
    (key val)red subs←⍵
    l r←(1+red)⊃'[]' '<>'
    key_val←⊃,/⍕¨l key'='val r
    fmts←{⊖⍵}\'┌└'{
      0 0≡⍴⍵:⍵
      mask←∧\' '=↑↓⌽⍉⍵
      ⍉⌽⊃(⊂⌽⍺,mask/'│'),↓⌽⍉⍵
    }¨{⊖⍵}\∇¨subs
    dent←' '⊣¨key_val
    pads←{↓,/dent,⊂⍵}¨fmts
    ⊃{⍺,(↓key_val,'┤'),⍵}/pads
  }
  vec←{
    0≡⍵:⍬
    key_val bal(lft rgt)←⍵
    (∇ lft),(⊂key_val),∇ rgt
  }
  chk←{
    0=≡⍵:(0≡⍵)0 0 0,⍬ 1 1
    (key _)red subs←⍵
    blk←~red
    stats←(⍺+1)∇¨subs
    oks ss ds hs ks bs bks←↓⍉⊃stats
    keys←key{⍺,(⊂⍶),⍵}/ks
    okK←{⍵≡⍳⍴⍵}⍋⊃keys
    okR←blk∨∧/bks
    okB←=/bs
    ok←okK∧okR∧okB∧∧/oks
    kr←⌽2⍴¯1⌽keys
    blks←blk+⌈/bs
    ht sz←1+(⌈/hs),+/ss
    dp←⍺++/ds
    ⍺>0:ok sz dp ht,kr blks blk
    ok sz(⌊0.5+dp÷sz)ht
  }
  search←{
    inf red(lft rgt)←⍺
    dir←1-2×>/⍋⊃↑¨inf ⍵
    _ nxt←dir wise lft rgt
    sub path←nxt ⍶ ⍵
    subs←dir wise lft sub rgt
    (inf red subs)(dir,path)
  }
  rot←{
    Ninf Nred Nsubs←⍺
    (Linf Lred Lsubs)R←⍵ wise Nsubs
    ll lr←⍵ wise Lsubs
    N_←Ninf Nred(⍵ wise lr R)
    Linf Lred(⍵ wise ll N_)
  }
  sub←{
    inf red subs←⍺
    lft rgt←⍵ wise subs
    sub←⍶ rgt
    inf red(⍵ wise lft sub)
    }
  isred←{
    inf col(lft rgt)←⍺
    ⍵≡⍬:col
    (↑⌽(↑⍵)wise lft rgt)∇ 1↓⍵
  }
  wise←{(2×⍺)↑3⍴⍵}
  flip←{kv b lr←⍵ ⋄ kv(~b)lr}
  done←{⍺ ⍵}∘0 0 0
  base←{⍺ ⍵}∘⍬
  dblk←{⍺ ⍵}∘(,0)
  op←⍶
  '∪'≡op:root ⍺ ins ⍵
  '~'≡op:root ⍺ rem ⍵ 0
  '⍎'≡op:⍵ get ⍺
  '⍕'≡op:fmt ⍵
  '∊'≡op:vec ⍵
  '?'≡op:4↑0 chk ⍵
}
foldl ← { ⍶⍨/(⌽⍵),⊂⍺ }
put←'∪' redblack ⋄ get←'⍎' redblack ⋄ rem←'~' redblack ⋄ fmt←'⍕' redblack ⋄ chk←'?' redblack ⋄ vec←'∊' redblack ⋄ tree←0∘(put foldl)
pairs←('one'1⋄ 'two'2⋄ 'three'3⋄ 'four'4⋄ 'five'5⋄ 'six'6⋄ 'seven'7) ⋄ tt←tree pairs
result←'?' redblack tree ⍳7
⍝ =>
1 7 2 4

⍝ aplcart/table.tsv:2193 — Adelson-Velskii, Landis (AVL) trees; Concrete APLcart library call; self-contained setup from april/libraries/dfns/tree/demo.lisp:209. Local definitions replace the dfns namespace; no namespace feature implied
avl ← { 
  get←{
    ⍺≡0:
    (k v)_ subs←⍺
    k≡⍵:v
    dir←-/⍋⊃⍵ k
    sub sib←dir wise subs
    sub ∇ ⍵
  }
  put←{
    ⍺≡0:(⍵ 0(0 0))1
    (kv obal subs)(key val)←⍺ ⍵
    key≡↑kv:(⍵ obal subs)0
    dir obv←1 ¯1×-/⍋⊃key(↑kv)
    sub sib←dir wise subs
    nsub inc←sub ∇ ⍵
    new←obv proj kv obal(nsub sib)
    inc=0:new 0
    (dir balance new)(obal=0)
  }
  rem←{
    ⍺≡0:0 0
    ⍵≡↑↑⍺:rmnode ⍺
    dir obv←1 ¯1×-/⍋⊃⍵(↑↑⍺)
    kv obal(sub sib)←obv proj ⍺
    nsub inc←sub ∇ ⍵
    new←obv proj kv obal(nsub sib)
    inc=0:new 0
    nkv nbal nsubs←obv balance new
    (nkv nbal nsubs)(-nbal=0)
  }
  rmnode←{
    kv obal subs←⍵
    0∊subs:((subs⍳0)⊃⌽subs)¯1
    lft rgt←subs
    (sk sv)_ _←rgt limt ¯1
    rm inc←rgt rem sk
    new←(sk sv)obal(lft rm)
    inc=0:new 0
    nkv nbal nsubs←¯1 balance new
    (nkv nbal nsubs)(-nbal=0)
  }
  limt←{
    sub←↑⍵ wise↑⌽⍺
    sub≡0:⍺
    sub ∇ ⍵
  }
  balance←{
    kv obal subs←⍵
    new←⍺+obal
    0∊obal new:kv new subs
    (_ Bbal _)_←obal wise subs
    Bbal≠-obal:(-obal)rot1 ⍵
               (-obal)rot2 ⍵
  }
  rot1←{
    Akv Abal(B r)←⍺ proj ⍵
    Bkv Bbal(p q)←⍺ proj B
    AAbal←-⍺×Bbal=0
    BBbal←+⍺×Bbal=0
    AA←⍺ proj Akv AAbal(q r)
    ⍺ proj Bkv BBbal(p AA)
  }
  rot2←{
    Akv Abal(B s)←⍺ proj ⍵
    Bkv Bbal(p C)←⍺ proj B
    Ckv Cbal(q r)←⍺ proj C
    AAbal←⍺×Cbal=-⍺
    BBbal←-⍺×Cbal=⍺
    BB←⍺ proj Bkv BBbal(p q)
    AA←⍺ proj Akv AAbal(r s)
    ⍺ proj Ckv 0(BB AA)
  }
  vec←{
    0≡⍵:⍬
    key_val bal(lft rgt)←⍵
    (∇ lft),(⊂key_val),∇ rgt
  }
  chk←{
    0=≡⍵:(⍵≡0)0 0 0 ⍬
    (key _)bal subs←⍵
    stats←(⍺+1)∇¨subs
    oks szs dps hts krs←↓⍉⊃stats
    keys←key{⍺,(⊂⍶),⍵}/krs
    okkey←{⍵≡⍳⍴⍵}⍋⊃keys
    okhgt←bal=--/hts
    okbal←bal∊¯1 0 1
    ok←okkey∧okbal∧okhgt∧∧/oks
    sz←1++/szs
    dp←⍺++/dps
    ht←1+⌈/hts
    kr←⌽2⍴¯1⌽keys
    ⍺>0:ok sz dp ht kr
    ok sz(⌊0.5+dp÷sz)ht
  }
  fmt←{
    null←0 0⍴''
    ⍵≡0:null
    (key val)bal subs←⍵
    key_val←⍺,,/⍕¨key'='val
    deco←(2+bal)⊃'><' '─' '<>'
    fmts←{⊖⍵}\'┌└'{
      0 0≡⍴⍵:⍵
      mask←∧\' '=↑↓⌽⍉⍵
      ⍉⌽⊃(⊂⌽⍺,mask/'│'),↓⌽⍉⍵
    }¨{⊖⍵}\deco ∇¨subs
    case←~null null≡¨fmts
    join←(1+2⊥case)⊃'∘┐┘┤'
    join≡'∘':⊃,↓key_val
    dent←' '⊣¨key_val
    pads←{↓,/dent,⊂⍵}¨fmts
    ⊃{⍺,(↓key_val,join),⍵}/pads
  }
  proj←{(⍺=0 0 ¯1)⌽¨⍵}
  wise←{(⍺=1)⌽⍵}
  op←⍶
  '∪'≡op:↑⍺ put ⍵
  '⍎'≡op:⍺ get ⍵
  '~'≡op:↑⍺ rem ⍵
  '⍕'≡op:''fmt ⍵
  '∊'≡op:vec ⍵
  '?'≡op:4↑0 chk ⍵
}
foldl ← { ⍶⍨/(⌽⍵),⊂⍺ }
put←'∪' avl ⋄ get←'⍎' avl ⋄ rem←'~' avl ⋄ fmt←'⍕' avl ⋄ chk←'?' avl ⋄ vec←'∊' avl ⋄ tree←0∘(put foldl)
tt←0 put foldl ('one'1⋄ 'two'2⋄ 'three'3⋄ 'four'4⋄ 'five'5⋄ 'six'6⋄ 'seven'7) ⋄ tt←tt put foldl ('one'11⋄ 'two'22⋄ 'three'33)
result←'?' avl tree ⍳7
⍝ =>
1 7 1 3

⍝ aplcart/table.tsv:2194 — Apply function Simple-Array-Wise; Concrete APLcart library call; self-contained setup from april/libraries/dfns/array/demo.lisp:324. Local definitions replace the dfns namespace; no namespace feature implied
saw ← {
  ⍺←⊢
  2≥|≡⍺ ⍵ ⍵:⍺ ⍶ ⍵
  1≥|≡⍵    :⍺ ∇¨⊂⍵
  2≥|≡⍺ 1  :⍺∘∇¨⍵
  ⍺ ∇¨⍵
}
eng←'One' '' '' '' 'Five'
esp←'Uno' 'Dos' 'Tres' '' ''
result←⌽saw eng esp
⍝ =>
(('enO' ⋄ '' ⋄ '' ⋄ '' ⋄ 'eviF') ⋄ ('onU' ⋄ 'soD' ⋄ 'serT' ⋄ '' ⋄ ''))

⍝ aplcart/table.tsv:2197 — Roman numeral arithmetic; Concrete APLcart library call; self-contained setup from april/libraries/dfns/numeric/demo.lisp:137. Local definitions replace the dfns namespace; no namespace feature implied
roman ← {
  num←{{⍵+.××0.5+×⍵-1↓⍵,0}(,⍉1 5 ×⌝ 10*¯1+⍳4)[1+7|¯1+'IVXLCDMivxlcdm'⍳⍵]}
  fmt←{~∘' ',2 1 1⍉(' '⍪3 4⍴'MCXI DLV ')[1+(0 4 2 2⊤0 16 20 22 24 32 36 38 39 28)[;1+⍵⊤⍨4⍴10];]}
  depth←{⍹≥|≡⍵ : ⍶ ⍵ ⋄ ∇¨⍵}
  nums←num depth 1
  fmts←fmt depth 0
  ⍺←⍬ ⋄ ⍬≡⍺:fmts ⍶ ⌊nums ⍵
  fmts(⌊nums ⍺)⍶ ⌊nums ⍵
}
result←'IX' +roman 'IX' 
⍝ =>
'XVIII'

⍝ aplcart/table.tsv:2201 — Apply monadic function taking two-element argument as a dyadic function
Sum ← +/ ⋄ (Sum 3 4 ⋄ 3 Sum{⍶ ⍺ ⍵} 4)   ⍝ 7 7

⍝ aplcart/table.tsv:2204 — Reduction with f without respect to shape; Supply + as the missing operator operand; reduce a concrete 2×3 matrix
Y←2 3⍴⍳6 ⋄ +{⍶/,⍵}Y   ⍝ 21

⍝ aplcart/table.tsv:2205 — Apply dyadic function as a monadic function on a two-element vector of arguments
('Hello' ~ 'World' ⋄ ~{⍶/⍵} 'Hello' 'World')
('He' ⋄ 'He')

⍝ aplcart/table.tsv:2206 — Applying to columns action f defined on rows
f←⌽ ⋄ Y←2 3⍴⍳6 ⋄ f{⍉⍶⍉⍵}Y   ⍝ 2 3⍴4 5 6 1 2 3

⍝ aplcart/table.tsv:2207 — Normalise to norm given by f equal to 1
+⌿{⍵÷⍶ ⍵} 3 1 4 1
0.3333333333333333 0.1111111111111111 0.4444444444444444 0.1111111111111111

⍝ aplcart/table.tsv:2209 — Join matrix of matrixes to single matrix; dfns display import/wrappers omitted to test underlying arrays
mat ← 6 6⍴⍳36 ⋄ (1 1 0 1 0 0 {⊃(⊂⊢/⊆⍺)⊂¨(↑⊆⍺)⊂[1]⍵} mat ⋄ (1 1 0 1 0 0⋄ 1 0 1 0 1 0) {⊃(⊂⊢/⊆⍺)⊂¨(↑⊆⍺)⊂[1]⍵} mat)
(3 3⍴(1 1⍴1 ⋄ 1 2⍴2 3 ⋄ 1 3⍴4 5 6 ⋄ 2 1⍴7 13 ⋄ 2 2⍴8 9 14 15 ⋄ 2 3⍴10 11 12 16 17 18 ⋄ 3 1⍴19 25 31 ⋄ 3 2⍴20 21 26 27 32 33 ⋄ 3 3⍴22 23 24 28 29 30 34 35 36) ⋄ 3 3⍴(1 2⍴1 2 ⋄ 1 2⍴3 4 ⋄ 1 2⍴5 6 ⋄ 2 2⍴7 8 13 14 ⋄ 2 2⍴9 10 15 16 ⋄ 2 2⍴11 12 17 18 ⋄ 3 2⍴19 20 25 26 31 32 ⋄ 3 2⍴21 22 27 28 33 34 ⋄ 3 2⍴23 24 29 30 35 36))

⍝ aplcart/table.tsv:2210 — Scan from end with f; basedpl left scan
f←- ⋄ Y←1 2 3 ⋄ f{⌽⍶\⌽⍵}Y   ⍝ 0 1 3

⍝ aplcart/table.tsv:2211 — Plot of scalaroid function f for data Nv
f←×⍨ ⋄ Nv←¯2 ¯1 0 1 2 ⋄ f{⍵+¯11○⍶ ⍵}Nv   ⍝ ¯2j4 ¯1j1 0 1j1 2j4

⍝ aplcart/table.tsv:2212 — Each-Right: Pair up the entirety of X with each element of Y
1 2 3 ,{⍺∘⍶¨⍵} 10 20 30   ⍝ (1 2 3 10 ⋄ 1 2 3 20 ⋄ 1 2 3 30)

⍝ aplcart/table.tsv:2213 — Each-Left: Pair up each element of X with the entirety of Y
1 2 3 ,{⍶∘⍵¨⍺} 10 20 30   ⍝ (1 10 20 30 ⋄ 2 10 20 30 ⋄ 3 10 20 30)

⍝ aplcart/table.tsv:2214 — Filter to only those elements of Yv that satisfy scalar criterion criteria f
f←0∘< ⋄ Yv←¯1 0 2 3 ⋄ f{⍵/⍨⍶ ⍵}Yv   ⍝ 2 3

⍝ aplcart/table.tsv:2215 — Index of first element where X f Y (f returns Boolean result)
X←1 2 3 4 ⋄ f←= ⋄ Y←0 2 3 9 ⋄ X f{↑⍸⍺ ⍶ ⍵}Y
2

⍝ aplcart/table.tsv:2216 — Function power: apply f on Is repeated Is times
Is←3 ⋄ f←1∘+ ⋄ Y←1 2 3 ⋄ Is f{⍶⍣⍺⊢⍵}Y   ⍝ 4 5 6

⍝ aplcart/table.tsv:2219 — Multiple selection of function list; Concrete APLcart library call with self-contained April dfns definitions and standard operand names; no namespace support implied
for ← {
  (¯1↓⍺)⍶{1≠⍴⍺:⍺ ⍶ ⍵
    (⍶⍣(↑⍺))⍵
  }(⍹⍣(↑⌽⍺))⍵
}
result←1 2 1 (-for{⍵+1}for{⍵×2})3
⍝ =>
¯8

⍝ aplcart/table.tsv:2222 — Inverse of real-valued function; Concrete APLcart library call; self-contained setup from april/libraries/dfns/power/demo.lisp:19. Local definitions replace the dfns namespace; no namespace feature implied
invr ← { 
  ⍺←1+1e¯14+0×⍵
  ∆x←1e¯14*÷2
  -∘⍵∘⍶{
    ⍹ ⍵:⍵
    y y∆←⍶¨0 ∆x+⊂⍵
    ∇ ⍵-y×∆x÷y∆-y
  }(⍵∘≡∘⍶)⍺
}
result←{⍵*2}invr 49
⍝ =>
7.000000000000002

⍝ aplcart/table.tsv:2224 — The number of elements where X f Y (f returns Boolean result)
X←1 2 3 4 ⋄ f←= ⋄ Y←0 2 3 9 ⋄ X f{+/,⍺ ⍶ ⍵}Y
2

⍝ aplcart/table.tsv:2225 — Index of last element where Xv f Y (f returns Boolean result)
X←1 2 3 4 ⋄ f←= ⋄ Y←0 2 3 9 ⋄ X f{↑⌽⍸⍺ ⍶ ⍵}Y
3

⍝ aplcart/table.tsv:2226 — Index of first element where not X f Y (f returns Boolean result)
X←1 2 3 4 ⋄ f←= ⋄ Y←0 2 3 9 ⋄ X f{↑⍸~⍺ ⍶ ⍵}Y
1

⍝ aplcart/table.tsv:2230 — Reverse composition (Q-combinator): g on f X and Y, that is, (f X) g Y
X←2 ⋄ f←- ⋄ g←× ⋄ Y←3 ⋄ X f{⍵ ⍹⍨⍶ ⍺}g Y
¯6

⍝ aplcart/table.tsv:2231 — Bit-wise application of f over positive integers Jv
f←∨ ⋄ J←3 5 6 ⋄ f{2⊥⍶/2⊥⍣¯1⍉⍵}J   ⍝ 7

⍝ aplcart/table.tsv:2232 — Index of last element where not X f Y (f returns Boolean result)
X←1 2 3 4 ⋄ f←= ⋄ Y←0 2 3 9 ⋄ X f{↑⌽⍸~⍺ ⍶ ⍵}Y
4

⍝ aplcart/table.tsv:2233 — Selection of X or Y depending on condition As
X←10 20 ⋄ As←0 ⋄ Y←1 2 ⋄ X(As{↑⍶↓⍺⍵})Y   ⍝ 10 20

⍝ aplcart/table.tsv:2234 — Sequential OR test
f←0∘< ⋄ g←2∘= ⋄ Y←3 ⋄ f{⍶ ⍵:1 ⋄ ⍹ ⍵}g Y
1

⍝ aplcart/table.tsv:2235 — Sequential AND test
f←0∘< ⋄ g←2∘= ⋄ Y←3 ⋄ f{⍶ ⍵:⍹ ⍵ ⋄ 0}g Y
0

⍝ aplcart/table.tsv:2236 — Conditionally multiply (where A=0) or divide (where A=1)
M←10 20 30 ⋄ A←0 1 0 ⋄ N←2 4 5 ⋄ M(A{⍺×⍵*¯1*⍶})N
20 5 150

⍝ aplcart/table.tsv:2237 — Conditional drop of Iv elements from array Y
Iv←1 2 ⋄ A←1 0 ⋄ Y←3 3⍴⍳9 ⋄ Iv(A{⍵↓⍨⍶×⍺})Y
2 3⍴4 5 6 7 8 9

⍝ aplcart/table.tsv:2238 — Force N to range Ms≤N≤Ns
Ms←1 ⋄ Ns←3 ⋄ N←0 1 2 3 4 ⋄ (Ms{⍶⌈⍹⌊⍵}Ns)N
1 1 2 3 3

⍝ aplcart/table.tsv:2240 — Ternary: if As then apply f to Y else apply g to Y1
As←0 ⋄ f←- ⋄ g←⌽ ⋄ Y←1 2 3 ⋄ As f{⍺:⍶ ⍵ ⋄ ⍹ ⍵}g Y
3 2 1

⍝ aplcart/table.tsv:2241 — Church Boolean AND; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
X←1 ⋄ Y←0 ⋄ X(⊣{⍺(⍶ ⍹ ⊢)⍵}⊢)Y   ⍝ 0

⍝ aplcart/table.tsv:2242 — Church Boolean NAND; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
X←1 ⋄ Y←0 ⋄ X(⊣{⍵(⍶ ⍹ ⊢)⍺}⊢)Y   ⍝ 1

⍝ aplcart/table.tsv:2243 — Church Boolean OR; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
X←1 ⋄ Y←0 ⋄ X(⊣{⍺(⊣ ⍶ ⍹)⍵}⊢)Y   ⍝ 1

⍝ aplcart/table.tsv:2244 — Church Boolean NOR; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
X←1 ⋄ Y←0 ⋄ X(⊣{⍵(⊣ ⍶ ⍹)⍺}⊢)Y   ⍝ 0

⍝ aplcart/table.tsv:2245 — Church Boolean Implication; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
X←1 ⋄ Y←0 ⋄ X(⊣{⍺(⍹ ⍶ ⊣)⍵}⊢)Y   ⍝ 0

⍝ aplcart/table.tsv:2246 — Church Boolean Nonimplication; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
X←1 ⋄ Y←0 ⋄ X(⊣{⍵(⍹ ⍶ ⊣)⍺}⊢)Y   ⍝ 1

⍝ aplcart/table.tsv:2247 — Church Boolean Converse Implication; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
X←1 ⋄ Y←0 ⋄ X(⊣{⍺(⍶ ⍹ ⊣)⍵}⊢)Y   ⍝ 1

⍝ aplcart/table.tsv:2248 — Church Boolean Converse Nonimplication; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
X←1 ⋄ Y←0 ⋄ X(⊣{⍵(⍶ ⍹ ⊣)⍺}⊢)Y   ⍝ 0

⍝ aplcart/table.tsv:2250 — Segmented reduction: like f\ but starting over whenever indicated by Av
Av←1 0 1 0 0 ⋄ f←+ ⋄ Yv←1 2 3 4 5 ⋄ Av f{,/⍶/¨⍺⊂⍵}Yv
3 12

⍝ aplcart/table.tsv:2251 — Segmented scan: like f\ but starting over whenever indicated by Av; basedpl left scan
Av←1 0 1 0 0 ⋄ f←- ⋄ Yv←1 2 3 4 5 ⋄ Av f{,/⍶\¨⍺⊂⍵}Yv
1 ¯1 3 ¯1 ¯6

⍝ aplcart/table.tsv:2252 — Exclusive Scan First: scan with identity element as seed value
(+{⍶⍀¯1↓⍵⍪⍨⍶⌿⍬}1 2 3 4 5 ⋄ ×{⍶⍀¯1↓⍵⍪⍨⍶⌿⍬}1 2 3 4 5)
(0 1 3 6 10 ⋄ 1 1 2 6 24)

⍝ aplcart/table.tsv:2253 — Atop: f X g Y
X←2 ⋄ f←- ⋄ g←× ⋄ Y←3 ⋄ X f{⍺←⊢ ⋄ ⍶ ⍺ ⍹ ⍵}g Y
¯6

⍝ aplcart/table.tsv:2254 — Power: Iterating f on Y until condition g Y is true
(,∘0 {⍹ ⍵:⍵ ⋄ ∇⍶ ⍵} (7<≢) 1 2 3 ⋄ ,∘0 {⍹ ⍵:⍵ ⋄ ∇⍶ ⍵} (2<≢) 1 2 3)
(1 2 3 0 0 0 0 0 ⋄ 1 2 3)

⍝ aplcart/table.tsv:2255 — Apply f to Y, g Y times
f←1∘+ ⋄ g←≢ ⋄ Y←1 2 3 ⋄ f{⍶⍣(⍹ ⍵)⊢⍵}g Y
4 5 6

⍝ aplcart/table.tsv:2256 — Fold (reduce) from the left; Supply the initial left argument 0 for the left fold
f←- ⋄ Y←1 2 3 ⋄ 0 f{⍶⍨/⍵⌽⍛,⊂⍺}Y   ⍝ ¯6

⍝ aplcart/table.tsv:2257 — Stable bubble sort using custom comparison function f (true:left precedes right)
f←< ⋄ Y←3 1 2 1 ⋄ f{⍵⌷⍨⊂⍒ ⍶⌝ ⍨⍵}Y   ⍝ 1 1 2 3

⍝ aplcart/table.tsv:2258 — Position of the Is'th Y in X
X←1 2 1 3 ⋄ Is←2 ⋄ Y←1 ⋄ X(Is{⍶⊃⍸⍺≡¨⊂⍵})Y
3

⍝ aplcart/table.tsv:2259 — The Is'th subvector of Yv (subvectors indicated by Bv)
Bv←1 0 1 0 0 ⋄ Is←2 ⋄ Yv←1 2 3 4 5 ⋄ Bv(Is{⍵⌿⍨⍶=+\⍺})Yv
3 4 5

⍝ aplcart/table.tsv:2260 — Arithmetic progression vector: Js elements starting at Ms with step Ns
Ms←2 ⋄ Ns←3 ⋄ Js←4 ⋄ (Ms{⍶+⍹×¯1+⍳⍵}Ns)Js
2 5 8 11

⍝ aplcart/table.tsv:2261 — Apply no-result function “en passant”; Reviewed Execute example checked through the Rust reference worker
f←{} ⋄ Y←1 2 3 ⋄ f{⍎'⍶ ⍵ ⋄ ⍵' ⋄ ⍶}Y   ⍝ 1 2 3

⍝ aplcart/table.tsv:2262 — Church Boolean XNOR; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
1(⊣{⍺(⍶ ⍹ ⍶⍨)⍵}⊢)0   ⍝ 0

⍝ aplcart/table.tsv:2263 — Church Boolean XOR; Supply ⊣ and ⊢ as the two Church-Boolean operands; apply the derived function to 1 and 0
1(⊣{⍺(⍶⍨ ⍹ ⍶)⍵}⊢)0   ⍝ 1

⍝ aplcart/table.tsv:2264 — Apply costly monadic function f on repetitive arguments
f←×⍨ ⋄ Y←2 1 2 3 ⋄ f{(⊂∪⍛⍳⍵)⌷⍶∪⍵}Y   ⍝ 4 1 4 9

⍝ aplcart/table.tsv:2265 — For each: f on items of Y unless Y is empty
f←≢ ⋄ Y←(1 2⋄ 3 4 5) ⋄ f{⍶¨⍣(~0∊⍴⍵)⊢⍵}Y
2 3

⍝ aplcart/table.tsv:2266 — Sums of N according to codes X for lookup table Y
X←1 2 3 ⋄ Y←1 2 1 3 ⋄ N←10 20 30 40 ⋄ X(Y{⍺ =⌝ ⍶+.×⍵})N
0 0 0

⍝ aplcart/table.tsv:2268 — Row-by-row formatting (width Is) of Nm with Iv decimals per row
Is←6 ⋄ Iv←1 2 ⋄ Nm←2 2⍴1.25 2.375 3.25 4.625 ⋄ (Is{⍵⍕⍤1⍨⍶,⍪⍹}Iv)Nm
2 12⍴'   1.3   2.4  3.25  4.63'

⍝ aplcart/table.tsv:2269 — Inserting X into Y after major cell Is
X←8 9 ⋄ Is←2 ⋄ Y←1 2 3 4 ⋄ X(Is{⍶(↑⍪⍺⍪↓)⍵})Y
1 2 8 9 3 4

⍝ aplcart/table.tsv:2270 — Replacing elements of Y satisfying Bv with Xs
Xs←9 ⋄ Bv←1 0 1 ⋄ Y←1 2 3 ⋄ (Xs{⍶@(⍹∘⊣)⍵}Bv)Y
9 2 9

⍝ aplcart/table.tsv:2271 — Apply X∘f to Y, X g Y times
X←2 ⋄ f←+ ⋄ g←⊣ ⋄ Y←1 2 3 ⋄ X f{⍺ ⍶⍣(⍺ ⍹ ⍵)⊢⍵}g Y
5 6 7

⍝ aplcart/table.tsv:2272 — Descending coefficients of Is-degree polynomial fit given x-values Mv and y-values Nv
Mv←0 1 2 3 ⋄ Is←2 ⋄ Nv←1 3 7 13 ⋄ Mv(Is{⌽⍵⌹⍺ *⌝ 0,⍳⍶})Nv
1 1 1

⍝ aplcart/table.tsv:2273 — Bit-wise application of f between positive integers I and J
I←3 5 ⋄ f←≠ ⋄ J←6 2 ⋄ I f{⍉2⊥⍶/2⊥⍣¯1⍉⍺,[0.5]⍵}J
5 7

⍝ aplcart/table.tsv:2274 — Justifying left fields of Yv (lengths Iv) to length Is
Iv←2 1 ⋄ Is←3 ⋄ Yv← 'abc'  ⋄ Iv(Is{⍵\⍨,⍺ >⌝ ¯1+⍳⍶})Yv
'ab c  '

⍝ aplcart/table.tsv:2275 — Left Scan with initial value: f⍨/⌽(⊂X),Yv with intermediate values
10-{c←⍺ ⋄ ⍶{c⊢←c ⍶ ⍵}¨⍵}3 1 4   ⍝ 7 6 2

⍝ aplcart/table.tsv:2276 — Selection of elements of M and N depending on condition A
M←10 20 30 ⋄ A←1 0 1 ⋄ N←1 2 3 ⋄ M(A{(⍺×⍶)+⍵×~⍶})N
10 2 30

⍝ aplcart/table.tsv:2277 — Across: apply f between (Y g X) and its commute, that is (Y g X) f (X g Y)
X←2 ⋄ f←- ⋄ g←÷ ⋄ Y←3 ⋄ X f{(⍵ ⍹ ⍺)⍶(⍺ ⍹ ⍵)}g Y
0.8333333333333334

⍝ aplcart/table.tsv:2278 — Justifying right fields of Yv (lengths Iv) to length Is
Iv←2 1 ⋄ Is←3 ⋄ Yv← 'abc'  ⋄ Iv(Is{⍵\⍨,⍺ >⌝ ⌽¯1+⍳⍶})Yv
' ab  c'

⍝ aplcart/table.tsv:2279 — Multi-dimensional arithmetic progression with dimensions Jv starting at Mv with steps Nv
Mv←10 20 ⋄ Nv←2 3 ⋄ Jv←2 3 ⋄ (Mv{⍶∘+¨⍹∘×¨¯1+⍳⍵}Nv)Jv
2 3⍴(10 20 ⋄ 10 23 ⋄ 10 26 ⋄ 12 20 ⋄ 12 23 ⋄ 12 26)

⍝ aplcart/table.tsv:2280 — Is-replicating along new dimension at fractional axis Ms
(2 (0.5{⍺/[⌈⍶],[⍶]⍵}) 2 3⍴⍳6 ⋄ 2 (2.5{⍺/[⌈⍶],[⍶]⍵}) 2 3⍴⍳6 ⋄ 2 (1.5{⍺/[⌈⍶],[⍶]⍵}) 2 3⍴⍳6)
(2 2 3⍴1 2 3 4 5 6 1 2 3 4 5 6 ⋄ 2 3 2⍴1 1 2 2 3 3 4 4 5 5 6 6 ⋄ 2 2 3⍴1 2 3 1 2 3 4 5 6 4 5 6)

⍝ aplcart/table.tsv:2281 — Giving a default value X for indices beyond end of Y
I←1 4 2 ⋄ X←99 ⋄ Y←10 20 30 ⋄ I(X{(⊂⍺⌊1+≢⍵)⌷⍵⍪⍶})Y
10 99 20

⍝ aplcart/table.tsv:2282 — Under: preprocess (g) argument(s) before applying main function (f), then undo preprocessing
X←2 ⋄ f←+ ⋄ g←⍟ ⋄ Y←3 ⋄ X f{⍺←⊢ ⋄ ⍹⍣¯1⊢⍺ ⍶⍥⍹ ⍵}g Y
6

⍝ aplcart/table.tsv:2284 — Inverse of outer product selfie (∘.f⍨⍣¯1)
f←+ ⋄ Y←3 3⍴2 3 4 3 4 5 4 5 6 ⋄ f{⍶⍨⍣¯1⊢⍵⍉⍨r⍴⍳2÷⍨r←⍴⍴⍵}Y
1 2 3

⍝ aplcart/table.tsv:2286 — Generalised convolution
Mv←1 2 3 ⋄ f←+ ⋄ Nv←4 5 6 ⋄ Mv f{⍵+.×⍨(⍳≢⍺)⌽⍤0 1⍶⌽⍺}Nv
31 31 28

⍝ aplcart/table.tsv:2287 — Interpolated value of series Mv=f(Nv) at Ms
Mv←1 3 7 ⋄ Ms←1.5 ⋄ Nv←0 1 2 ⋄ Mv(Ms{⍶⊥⍺⌹⍵ *⌝ ⌽¯1+⍳≢⍵})Nv
4.75

⍝ aplcart/table.tsv:2288 — Run f on axes of Y; Use implicit ⍺ and ⍶ rather than the free/undefined AX and f names in the operator body; test adding a vector along matrix axis 1
X←10 20 ⋄ f←+ ⋄ ax←1 ⋄ Y←2 3⍴⍳6 ⋄ X(f{⊃[⍹](⊂⍺)⍶⊂[⍹]⍵}ax)Y
2 3⍴11 12 13 24 25 26

⍝ aplcart/table.tsv:2289 — Apply f with leading axis agreement
10 20 30 +{⍺ ⍶⍤(-⌊/≢∘⍴¨⍺⍵)⊢⍵} 3 2 4⍴8/⍳3
3 2 4⍴11 11 11 11 11 11 11 11 22 22 22 22 22 22 22 22 33 33 33 33 33 33 33 33

⍝ aplcart/table.tsv:2290 — Mesh arrays X and Y under control of A () (0:element from X, 1:cell from Y, …)
X←1 2 3 ⋄ A←0 1 0 ⋄ Y←10 20 30 ⋄ X(A{(⍶/⍥,⍵)@(⍶⍨)⍺})Y
1 20 3

⍝ aplcart/table.tsv:2291 — Partition (⊂) Ym along both axes (Av can be one or two partitioning vectors); dfns display import/wrappers omitted to test underlying arrays
mat ← 6 6⍴⍳36 ⋄ (1 1 0 1 0 0 {⊃(⊂⊢/⊆⍺)⊂¨(↑⊆⍺)⊂[1]⍵} mat ⋄ (1 1 0 1 0 0⋄ 1 0 1 0 1 0) {⊃(⊂⊢/⊆⍺)⊂¨(↑⊆⍺)⊂[1]⍵} mat)
(3 3⍴(1 1⍴1 ⋄ 1 2⍴2 3 ⋄ 1 3⍴4 5 6 ⋄ 2 1⍴7 13 ⋄ 2 2⍴8 9 14 15 ⋄ 2 3⍴10 11 12 16 17 18 ⋄ 3 1⍴19 25 31 ⋄ 3 2⍴20 21 26 27 32 33 ⋄ 3 3⍴22 23 24 28 29 30 34 35 36) ⋄ 3 3⍴(1 2⍴1 2 ⋄ 1 2⍴3 4 ⋄ 1 2⍴5 6 ⋄ 2 2⍴7 8 13 14 ⋄ 2 2⍴9 10 15 16 ⋄ 2 2⍴11 12 17 18 ⋄ 3 2⍴19 20 25 26 31 32 ⋄ 3 2⍴21 22 27 28 33 34 ⋄ 3 2⍴23 24 29 30 35 36))

⍝ aplcart/table.tsv:2292 — 2's-complement bit-wise application of f over Jv
f←∨ ⋄ J←3 5 6 ⋄ f{⍉2⊥(-⍶/0>b)⍪⍶/2⊥⍣¯1⊢b←⍵}J
7

⍝ aplcart/table.tsv:2293 — Is-point spline of Nm Bezier matrix with Mv control points
Nm←2 2⍴¯1 1 1 0 ⋄ Is←4 ⋄ Mv←0 8 ⋄ Nm(Is{(⍪⍶÷⍨⍳⍶)⊥⍺+.×⍵})Mv
2 4 6 8

⍝ aplcart/table.tsv:2294 — If: replace/apply if Bs
X←2 ⋄ f←+ ⋄ Bs←1 ⋄ Y←1 2 3 ⋄ X(f{⍺←⊢ ⋄ ⍺(⍶⊣⊢)⍣⍹⊢⍵}Bs)Y
3 4 5

⍝ aplcart/table.tsv:2295 — Power: apply f on Y J times
f←1∘+ ⋄ J←0 1 3 ⋄ Y←10 ⋄ (f{⍶{⍶⍣⍵⊢⍹}⍵⍤0⊢⍹}J)Y
10 11 13

⍝ aplcart/table.tsv:2296 — Reduction (/) with f in dimension Is (default: last), rank unchanged
Is←1 ⋄ f←+ ⋄ Y←2 3⍴⍳6 ⋄ Is f{⍺←≢⍴⍵ ⋄ s←⍴⍵ ⋄ s[⍺]←1 ⋄ s⍴⍶/[⍺]⍵}Y
1 3⍴5 7 9

⍝ aplcart/table.tsv:2297 — Run f on axes of X
X←2 3⍴⍳6 ⋄ f←+ ⋄ ax←1 ⋄ Y←10 20 ⋄ X(f{⊃[⍹](⊂[⍹]⍺)⍶⊂⍵}ax)Y
2 3⍴11 12 13 24 25 26

⍝ aplcart/table.tsv:2298 — Adverse: Apply f but if it errors, apply g; Use the optional left argument X=2 and division by zero to exercise fallback to +
X←2 ⋄ f←÷ ⋄ g←+ ⋄ Y←0 ⋄ X(f{⍺←⊢ ⋄ 0::⍺ ⍹ ⍵ ⋄ ⍺ ⍶ ⍵}g)Y
2

⍝ aplcart/table.tsv:2299 — Stable bubble sort using custom comparison function f (true:left precedes right); First-true masks use cumulative counts under basedpl left scan
f←< ⋄ Y←3 1 4 2 ⋄ f{⌽@(1(⌽∨⊢)0{⍵∧1=+\⍵}⍤,(⍶/∘⌽∘(2∘↕)))⍣≡⍵}Y
1 2 3 4

⍝ aplcart/table.tsv:2300 — Insert X at fractional positions Nv in Y (≢Nv)=≢X
X←8 9 ⋄ Nv←1.5 3.5 ⋄ Y←1 2 3 4 ⋄ (X{(⊂⍋(⍳≢⍵),⌊⍶)⌷⍵,⍶}Nv)Y
(1.5 3.5 8 9 ⋄ 1 2 3 4)

⍝ aplcart/table.tsv:2301 — Power: apply X∘f on Y J times
X←2 ⋄ f←+ ⋄ J←0 1 3 ⋄ Y←10 ⋄ X(f{⍺ ⍶{⍺ ⍶⍣⍵⊢⍹}⍵⍤0⊢⍹}J)Y
10 12 16

⍝ aplcart/table.tsv:2302 — Power: Iterating f on Y while condition g Y is true but at most I times
(2 ,∘0 {⍶{⍶⍣(⍹ ⍵)⊢⍵}⍹⍣⍺⊢⍵} (7>≢) 1 2 3 ⋄ 9 ,∘0 {⍶{⍶⍣(⍹ ⍵)⊢⍵}⍹⍣⍺⊢⍵} (7>≢) 1 2 3)
(1 2 3 0 0 ⋄ 1 2 3 0 0 0 0)

⍝ aplcart/table.tsv:2303 — Left Scan: f⍨/⌽Yv with intermediate values
-{c←↑⍵ ⋄ (1↑⍵),⍶{c⊢←c ⍶ ⍵}¨1↓⍵}10 3 1 4
10 7 6 2

⍝ aplcart/table.tsv:2304 — Replacing elements of Z that appear in Xv with the corresponding element from Yv
Xv←2 4 ⋄ Yv←20 40 ⋄ Z←1 2 3 4 2 ⋄ (Xv{⍶(⍳⊂⍛⌷⍹⍨)@(∊∘⍶)⍵}Yv)Z
1 20 3 40 20

⍝ aplcart/table.tsv:2306 — Formatted function table for function Cv with Xv down and Yv across; Reviewed Execute example checked through the Rust reference worker
Xv←1 2 ⋄ Cv←'+' ⋄ Yv←3 4 ⋄ Xv(Cv{(⍵,⍨⊂⍶)⍪⍺,⍺ (⍎⍶)⌝ ⍵})Yv
3 3⍴'+' 3 4 1 4 5 2 5 6

⍝ aplcart/table.tsv:2307 — Under: apply main function (f) to selection (g) of argument(s)
X←2 ⋄ f←+ ⋄ g←1∘↑ ⋄ Y←10 20 30 ⋄ X f{⍺←⊢ ⋄ w←⍵ ⋄ ((⍹)w)←⍺ ⍶⍥⍹ ⍵ ⋄ w}g Y
12 20 30

⍝ aplcart/table.tsv:2308 — Definite integral of scalar function f in range Nv[1]…Nv[2] with Is steps
Is←4 ⋄ f←×⍨ ⋄ Nv←0 1 ⋄ Is f{a+.×⍶⍵↑⍛-(a←-/⍵÷-⍺)×0.5-⍳⍺}Nv
0.328125

⍝ aplcart/table.tsv:2309 — 2's-complement bit-wise application of f between I and J
I←3 5 ⋄ f←≠ ⋄ J←6 2 ⋄ I f{⍉2⊥(-⍶/0>b)⍪⍶/2⊥⍣¯1⊢b←⍉⍺,[0.5]⍵}J
5 7

⍝ aplcart/table.tsv:2310 — Probabilistic function corresponding to Boolean function f
M←0.25 ⋄ f←∨ ⋄ N←0.5 ⋄ M f{↑+/,( ⍶⌝ ⍨0 1)×(1-⍺)⍺ ×⌝ (1-⍵)⍵}N
0.625

⍝ aplcart/table.tsv:2311 — Mesh vectors Xv and Yv under control of Av (0:cell from Xv, 1:cell from Yv, …)
Xv←10 20 ⋄ Av←0 1 0 1 1 ⋄ Yv←1 2 3 ⋄ Xv(Av{(1+⍶)⊃¨(⍶~⍛\⍺),¨(⍶\⍵)})Yv
10 1 20 2 3

⍝ aplcart/table.tsv:2312 — Accumulating reduction
f←- ⋄ Y←1 2 3 ⋄ f{⍶{⍵,⍨⊂⍺ ⍶↑⍵}/1↓⍬(⊢,∘⊂⍴)¯1⌽⍵}Y
2 ¯1 3

⍝ aplcart/table.tsv:2313 — Modulo power (fast non-overflowing Ns|Mv*Jv)
Mv←2 3 ⋄ Ns←7 ⋄ Jv←5 4 ⋄ Mv(Ns{⍶{⍶|⍺×⍶|×⍨⍵}⌿⍺*⍤1⊖0⍪2⊥⍣¯1⊢⍵})Jv
4 4

⍝ aplcart/table.tsv:2314 — Mesh matrices Xm and Ym along rows under control of Av (0:cell from Xm, 1:cell from Ym, …)
Xm←2 2⍴10 20 30 40 ⋄ Av←0 1 0 1 1 ⋄ Ym←2 3⍴⍳6 ⋄ Xm(Av{(1+⍶)⊃¨⍤1⊢(⍶~⍛\⍺),¨(⍶\⍵)})Ym
2 5⍴10 1 20 2 3 30 4 40 5 6

⍝ aplcart/table.tsv:2315 — Mesh matrices Xm and Ym along columns under control of Av (0:cell from Xm, 1:cell from Ym, …)
Xm←2 2⍴10 20 30 40 ⋄ Av←0 1 0 1 1 ⋄ Ym←3 2⍴⍳6 ⋄ Xm(Av{(1+⍶)⊃¨⍤0 1⊢(⍶~⍛⍀⍺),¨(⍶⍀⍵)})Ym
5 2⍴10 20 1 2 30 40 3 4 5 6

⍝ aplcart/table.tsv:2316 — Boolean/probabilistic functions I (0-15)
M←0.25 ⋄ I←1 6 7 ⋄ N←0.5 ⋄ M(I{+⌿(⍶⊤⍨4⍴2)×⍤¯1⊃,(1-⍺)⍺ ×⌝ (1-⍵)⍵})N
0.125 0.5 0.625

⍝ aplcart/table.tsv:2317 — Replacing major cells of Z that appear in X with the corresponding major cell from Y
X←2 4 ⋄ Y←20 40 ⋄ Z←1 2 3 4 2 ⋄ (X{⍶(⍳⊂⍛⌷⍹⍨)@(⍸⍶≢⍛≥⍶⍳⍵)⊢⍵}Y)Z
1 20 3 40 20

⍝ aplcart/table.tsv:2318 — Ravel of a matrix to Is columns with a gap of Js
Is←2 ⋄ Js←1 ⋄ Ym←3 4⍴⍳12 ⋄ (Is{(⍶*¯1 1)(×⍴⊢↑⍵⍨)⍵⍴⍛+⍹,⍨⍶|-≢⍵}Js)Ym
2 10⍴1 2 3 4 0 5 6 7 8 0 9 10 11 12 0 0 0 0 0 0

⍝ aplcart/table.tsv:2319 — Stirling number of the As'th kind (0:first, 1:second): S(n,k)
Is←2 ⋄ As←1 ⋄ Js←4 ⋄ Is(As{⍺>⍵:0 ⋄ ↑⌽⊃+\⍤×/1,⍨⌽⊣\⍣⍶⊢,/⍺↕⍳⍵-1})Js
7

⍝ aplcart/table.tsv:2321 — Open a gap of Iv[i] before Y[Jv[i]] (for all i)
Iv←2 1 ⋄ Jv←1 3 ⋄ Y←10 20 30 ⋄ (Iv{⍵⍀⍨(⍳⍵≢⍛++/⍶)∊+\1+⍶\⍨⍹∊⍨⍳≢⍵}Jv)Y
0 0 10 20 0 30

⍝ aplcart/table.tsv:2322 — Mesh vectors Xv and Yv in multiple ways under control of Am (0:cell from Xv, 1:cell from Yv, …)
Xv←10 20 ⋄ Am←2 4⍴0 1 0 1 1 0 1 0 ⋄ Yv←1 2 ⋄ Xv(Am{(1+⍶)⊃¨⍤1⊢(⍶~⍛\⍤1⊢⍺),¨(⍶\⍤1⊢⍵)})Yv
2 4⍴10 1 20 2 1 10 2 20

⍝ aplcart/table.tsv:2323 — Mesh matrices Xm and Ym differently for each row under control of Am (0:cell from Xm, 1:cell from Ym, …)
Xm←2 2⍴10 20 30 40 ⋄ Am←2 4⍴0 1 0 1 1 0 1 0 ⋄ Ym←2 2⍴1 2 3 4 ⋄ Xm(Am{(1+⍶)⊃¨⍤1⊢(⍶~⍛\⍤1⊢⍺),¨(⍶\⍤1⊢⍵)})Ym
2 4⍴10 1 20 2 3 30 4 40

⍝ aplcart/table.tsv:2324 — Iterate: ⍣ but with intermediary results
X←2 ⋄ f←+ ⋄ Y←3 ⋄ Z←10 ⋄ X(f{⍺←⊢ ⋄ r⊣⍺ ⍶{⍺←⊢ ⋄ r,∘⊂←⍺ ⍶ ⍵}⍣⍹↑r←⊂⍵}Y)Z
10 12 14 16

⍝ aplcart/table.tsv:2325 — Inserting Is Y's into Z after indices Iv
Iv←1 3 ⋄ Is←2 ⋄ Y←9 ⋄ Z←1 2 3 4 ⋄ Iv(Is{(⊂(1+≢⍵)⌊⍋(⍳≢⍵),⍺⍴⍨⍶×≢⍺)⌷⍵⍪⍹}Y)Z
1 9 9 2 3 9 9 4

⍝ aplcart/table.tsv:2326 — Mesh vectors Xv and Yv differently in each column under control of Am (0:cell from Xv, 1:cell from Yv, …)
Xv←10 20 ⋄ Am←2 4⍴0 1 0 1 1 0 1 0 ⋄ Yv←1 2 ⋄ Xv(Am{⍉(1+⍶)⊃¨⍤1⊢(⍶~⍛\⍤1⍉⍺),¨(⍶\⍤1⍉⍵)})Yv
4 2⍴10 1 1 10 20 2 2 20

⍝ aplcart/table.tsv:2327 — Mesh matrices Xm and Ym differently for each column under control of Am (0:cell from Xm, 1:cell from Ym, …)
Xm←2 2⍴10 20 30 40 ⋄ Am←2 4⍴0 1 0 1 1 0 1 0 ⋄ Ym←2 2⍴1 2 3 4 ⋄ Xm(Am{⍉(1+⍶)⊃¨⍤1⊢(⍶~⍛\⍤1⍉⍺),¨(⍶\⍤1⍉⍵)})Ym
4 2⍴10 2 1 20 30 4 3 40

⍝ aplcart/table.tsv:2328 — Graph of scalar function f at points Nv; Reviewed Execute example checked through the Rust reference worker
f←×⍨ ⋄ Nv←¯2 ¯1 0 1 2 ⋄ f{' ∘'[1+(⊢ =⌝ ⍨∘⌽¯1+⌊/+∘⍳1+⌈/-⌊/)⌊0.5+⍶ ⍵]}Nv
5 5⍴'∘   ∘           ∘ ∘   ∘  '

⍝ aplcart/table.tsv:2329 — Open a gap of Iv[i] after Y[Jv[i]] (for all i)
Iv←2 1 ⋄ Jv←1 3 ⋄ Y←10 20 30 ⋄ (Iv{⍵⍀⍨(⍳⍵≢⍛++/⍶)∊+\1+¯1↓0,⍶\⍨⍹∊⍨⍳≢⍵}Jv)Y
10 0 0 20 30 0

⍝ aplcart/table.tsv:2330 — Stable quicksort using custom comparison function f (negative:left precedes right, zero:keep ordering, positive:right precedes left); Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
f←{(↑⍺)-↑⍵} ⋄ Y←5 2⍴2 30 1 90 2 10 1 70 2 20 ⋄ r←f{1≥≢⍵:⍵ ⋄ 0((∇⍵⌿⍨>)⍪(⍵⌿⍨=)⍪∘∇⍵⌿⍨<)⍵(⍶-⍶⍨)⍤¯1 99?∘≢⍛⌷⍵}Y ⋄ r≡Y[⍋Y[;1];]
1

⍝ aplcart/table.tsv:2331 — Delta; assignment recipes also read the target, and definition recipes call the defined function/operator
∆←42 ⋄ ∆   ⍝ 42

⍝ aplcart/table.tsv:2333 — Scaled Notation
(1000 = 1E3 ⋄ 10*23)   ⍝ 1 1e23

⍝ aplcart/table.tsv:2334 — Complex Notation
(¯1 * 0.5 ⋄ 0J2 * 2)   ⍝ 0j1 ¯4

⍝ aplcart/table.tsv:2335 — Negative number indicator
(¯1 + 3 ⋄ 1 - 3)   ⍝ 2 ¯2

⍝ aplcart/table.tsv:2336 — tau (2 pi)
π2   ⍝ 6.283185307179586

⍝ aplcart/table.tsv:2337 — Identity element for function f
(+/⍬ ⋄ ×/⍬)   ⍝ 0 1

⍝ aplcart/table.tsv:2339 — Decimal Point
1.2 × 3.4   ⍝ 4.08

⍝ aplcart/table.tsv:2341 — zero-by-zero numeric matrix
(0 0⍴0) ≡ ⍬⊤⍬   ⍝ 1

⍝ aplcart/table.tsv:2419 — Null character (NUL)
•UCS 0   ⍝ •UCS 0

⍝ aplcart/table.tsv:2420 — Line Feed (LF)
•UCS 10   ⍝ •UCS 10

⍝ aplcart/table.tsv:2421 — Vertical Tab character (VT)
•UCS 11   ⍝ •UCS 11

⍝ aplcart/table.tsv:2422 — Form Feed character (FF)
•UCS 12   ⍝ •UCS 12

⍝ aplcart/table.tsv:2423 — Delete character (DEL)
•UCS 127   ⍝ •UCS 127

⍝ aplcart/table.tsv:2424 — Carriage Return character (CR)
•UCS 13   ⍝ •UCS 13

⍝ aplcart/table.tsv:2425 — Carriage Return-Line Feed pair (CRLF)
•UCS 13 10   ⍝ •UCS 13 10

⍝ aplcart/table.tsv:2426 — End-Of-File character (EOF)
•UCS 26   ⍝ •UCS 26

⍝ aplcart/table.tsv:2427 — Escape character (ESC)
•UCS 27   ⍝ •UCS 27

⍝ aplcart/table.tsv:2428 — Bell character (BEL)
•UCS 7   ⍝ •UCS 7

⍝ aplcart/table.tsv:2429 — Backspace character (BS)
•UCS 8   ⍝ •UCS 8

⍝ aplcart/table.tsv:2430 — Horizontal Tab character (HT)
•UCS 9   ⍝ •UCS 9

⍝ aplcart/table.tsv:2431 — Statement Separator; assignment recipes also read the target, and definition recipes call the defined function/operator
⋄   ⍝ {}0

⍝ aplcart/table.tsv:2457 — Dfn/dop Guard (conditional result)
f←{⍵>0:⍵ ⋄ -⍵} ⋄ f ¯3   ⍝ 3

⍝ aplcart/table.tsv:2460 — Character delimiter
('abc' ⋄ '5' = 5)   ⍝ ('abc') 0

⍝ aplcart/table.tsv:2461 — Comment symbol (disables rest of line)
'yes'   ⍝ 'yes'

⍝ aplcart/table.tsv:2470 — Fast: Is Y a Simple Scalar?; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←7 ⋄ 0=≡Y   ⍝ 1

⍝ aplcart/table.tsv:2471 — Fast: Is Y a Simple Non-scalar?; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←1 2 ⋄ 1=≡Y   ⍝ 1

⍝ aplcart/table.tsv:2472 — Fast: The rank of Y as a scalar; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←2 3⍴⍳6 ⋄ ≢⍴Y   ⍝ 2

⍝ aplcart/table.tsv:2473 — Fast: The rank of Y as a 1-element vector; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←2 3⍴⍳6 ⋄ ⍴⍴Y   ⍝ 1⍴2

⍝ aplcart/table.tsv:2474 — Fast: Euler's idiom (accurate when N is a multiple of 0J0.5); assignment recipes also read the target, and definition recipes call the defined function/operator
N←0j0.5 0j1 0j1.5 0j2 ⋄ *πN   ⍝ 0j1 ¯1 0j¯1 1

⍝ aplcart/table.tsv:2475 — Negative “infinity” (the smallest representable value); Agreed real infinity literal/empty-reduction identity
⌈/⍬   ⍝ ¯∞

⍝ aplcart/table.tsv:2476 — Positive “infinity” (the largest representable value); Agreed real infinity literal/empty-reduction identity
⌊/⍬   ⍝ ∞

⍝ aplcart/table.tsv:2477 — Fast: The item in the top right of Y; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←2 3⍴⍳6 ⋄ ↑↑⌽Y   ⍝ 3

⍝ aplcart/table.tsv:2478 — Fast: The subset of ⍳Js corresponding to the 1s in Bv; assignment recipes also read the target, and definition recipes call the defined function/operator
Bv←1 0 1 0 ⋄ Js←4 ⋄ Bv/⍳Js   ⍝ 1 3

⍝ aplcart/table.tsv:2479 — Fast: Is Y empty?; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←2 0⍴0 ⋄ 0∊⍴Y   ⍝ 1

⍝ aplcart/table.tsv:2480 — f between X and Y along axis ax; assignment recipes also read the target, and definition recipes call the defined function/operator
X←10 20 30 ⋄ f←+ ⋄ ax←2 ⋄ Y←2 3⍴⍳6 ⋄ X f[ax]Y
2 3⍴11 22 33 14 25 36

⍝ aplcart/table.tsv:2481 — f on Y along axis/axes ax; assignment recipes also read the target, and definition recipes call the defined function/operator
f←⌽ ⋄ ax←1 ⋄ Y←2 3⍴⍳6 ⋄ f[ax]Y   ⍝ 2 3⍴4 5 6 1 2 3

⍝ aplcart/table.tsv:2577 — Unit tesseract; assignment recipes also read the target, and definition recipes call the defined function/operator
2 2 2 2⊤⍳16
4 16⍴0 0 0 0 0 0 0 1 1 1 1 1 1 1 1 0 0 0 0 1 1 1 1 0 0 0 0 1 1 1 1 0 0 1 1 0 0 1 1 0 0 1 1 0 0 1 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0

⍝ aplcart/table.tsv:2578 — Unit cube; assignment recipes also read the target, and definition recipes call the defined function/operator
2 2 2⊤⍳8
3 8⍴0 0 0 1 1 1 1 0 0 1 1 0 0 1 1 0 1 0 1 0 1 0 1 0

⍝ aplcart/table.tsv:2579 — Unit square; assignment recipes also read the target, and definition recipes call the defined function/operator
2 2⊤⍳4   ⍝ 2 4⍴0 1 1 0 1 0 1 0

⍝ aplcart/table.tsv:2587 — Output assigned value; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←1 2 ⋄ ⊢name←Y ⋄ name   ⍝ 1 2

⍝ aplcart/table.tsv:2588 — Update (in dfns/dops) a variable in closest scope where localised or in global scope if not localised anyhere; assignment recipes also read the target, and definition recipes call the defined function/operator
name←1 2 ⋄ Y←3 4 ⋄ name⊢←Y ⋄ name   ⍝ 3 4

⍝ aplcart/table.tsv:2589 — Fast: 'name' redefined to be its value with Y catenated along its last axis; assignment recipes also read the target, and definition recipes call the defined function/operator
name←1 2 ⋄ Y←3 4 ⋄ name,←Y ⋄ name   ⍝ 1 2 3 4

⍝ aplcart/table.tsv:2590 — Fast: 'name' redefined to be its value with Y catenated along its first axis; assignment recipes also read the target, and definition recipes call the defined function/operator
name←2 2⍴⍳4 ⋄ Y←5 6 ⋄ name⍪←Y ⋄ name   ⍝ 3 2⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:2591 — Output x to the session via stdout (with trailing line break); assignment recipes also read the target, and definition recipes call the defined function/operator
x←42 ⋄ ⎕←x   ⍝ 42

⍝ aplcart/table.tsv:2599 — Covers all errors (errors 1–999); Concrete executable example for the APLcart error-guard catalogue; trigger the named language error and return from its handler
{0::99 ⋄ 1÷0}0   ⍝ 99

⍝ aplcart/table.tsv:2601 — A system limit is exceeded; Concrete executable example for the APLcart error-guard catalogue; trigger the named language error and return from its handler
{10::10 ⋄ (129⍴0)⍴⍬}0   ⍝ 10

⍝ aplcart/table.tsv:2611 — Type or value not permitted for the function/operator/system variable or unrepresentable numeric value; Concrete executable example for the APLcart error-guard catalogue; trigger the named language error and return from its handler
{11::11 ⋄ 1÷0}0   ⍝ 11

⍝ aplcart/table.tsv:2616 — A line of characters does not constitute a meaningful statement; Concrete executable example for the APLcart error-guard catalogue; trigger the named language error and return from its handler
{2::2 ⋄ ⍎'('}0   ⍝ 2

⍝ aplcart/table.tsv:2625 — Index or axis is not in ⍳⍴Y or not in ⍳≢⍴Y; Concrete executable example for the APLcart error-guard catalogue; trigger the named language error and return from its handler
{3::3 ⋄ 4⊃1 2 3}0   ⍝ 3

⍝ aplcart/table.tsv:2631 — Array rank invalid for function/operator, or ranks of arguments do not conform; Concrete executable example for the APLcart error-guard catalogue; trigger the named language error and return from its handler
{4::4 ⋄ ⍳2 2⍴3}0   ⍝ 4

⍝ aplcart/table.tsv:2632 — The shape of the arguments of a function do not conform, but the ranks do conform; Concrete executable example for the APLcart error-guard catalogue; trigger the named language error and return from its handler
{5::5 ⋄ 1 2+1 2 3}0   ⍝ 5

⍝ aplcart/table.tsv:2639 — Undefined name in this scope, or function does not return result while required; Concrete executable example for the APLcart error-guard catalogue; trigger the named language error and return from its handler
{6::6 ⋄ missing}0   ⍝ 6

⍝ aplcart/table.tsv:2661 — Default left argument in dfn/dop
root←{⍺←2 ⋄ ⍵*÷⍺} ⋄ (2 root 9 ⋄ root 9)   ⍝ 3 3

⍝ aplcart/table.tsv:2665 — Dyadic operator (DOP) taking an array operand (W) to derive a monadic operator (which in turn takes a function operand to derive a dyadic function)
DOP←{⍺ ⍶ ⍵+⍹} ⋄ X←10 ⋄ f←- ⋄ Z←3 ⋄ Y←1 2 ⋄ X f(DOP Z)Y
6 5

⍝ aplcart/table.tsv:2666 — Dyadic operator (DOP) taking a function operand to derive a monadic operator (which in turn takes a function operand to derive a dyadic function)
DOP←{⍺ ⍶ ⍹ ⍵} ⋄ X←10 ⋄ f←- ⋄ g←⌽ ⋄ Y←1 2 ⋄ X f(DOP g)Y
8 9

⍝ aplcart/table.tsv:2667 — Stranding: (⊂X),⊂(⊂Y),(⊂Z); assignment recipes also read the target, and definition recipes call the defined function/operator
X←3 1 2 1 ⋄ Y←3 1 5 2 4 ⋄ Z←42 ⋄ X (Y Z)
(3 1 2 1) ((3 1 5 2 4) 42)

⍝ aplcart/table.tsv:2668 — Fork (E-combinator): Z g X h Y; assignment recipes also read the target, and definition recipes call the defined function/operator
X←2 ⋄ Z←3 ⋄ g←+ ⋄ h←× ⋄ Y←4 ⋄ X(Z g h)Y   ⍝ 11

⍝ aplcart/table.tsv:2669 — Fork (Φ₁-combinator): (X f Y)g(X h Y); assignment recipes also read the target, and definition recipes call the defined function/operator
X←2 ⋄ f←+ ⋄ g←- ⋄ h←× ⋄ Y←4 ⋄ X(f g h)Y   ⍝ ¯2

⍝ aplcart/table.tsv:2670 — Atop (B₁-combinator): f X g Y; assignment recipes also read the target, and definition recipes call the defined function/operator
X←2 ⋄ f←- ⋄ g←× ⋄ Y←4 ⋄ X(f g)Y   ⍝ ¯8

⍝ aplcart/table.tsv:2671 — Dyadic operator (DOP) taking an array operand (W) to derive a monadic operator (which in turn takes a function operand to derive a monadic function)
DOP←{⍶+⍵×⍹} ⋄ W←10 ⋄ Z←3 ⋄ Y←1 2 ⋄ W(DOP Z)Y
13 16

⍝ aplcart/table.tsv:2672 — Dyadic operator (DOP) taking a function operand to derive a monadic operator (which in turn takes an array operand to derive a monadic function)
DOP←{⍶+⍹ ⍵} ⋄ W←10 ⋄ g←- ⋄ Y←1 2 ⋄ W(DOP g)Y
9 8

⍝ aplcart/table.tsv:2673 — Dyadic operator (DOP) taking an array operand (W) to derive a monadic operator (which in turn takes an array operand to derive a monadic function)
DOP←{⍶ ⍵+⍹} ⋄ f←- ⋄ Z←3 ⋄ Y←1 2 ⋄ f(DOP Z)Y
¯4 ¯5

⍝ aplcart/table.tsv:2674 — Dyadic operator (DOP) taking a function operand to derive a monadic operator (which in turn takes a function operand to derive a monadic function)
DOP←{⍶ ⍹ ⍵} ⋄ f←+/ ⋄ g←⌽ ⋄ Y←1 2 3 ⋄ f(DOP g)Y
6

⍝ aplcart/table.tsv:2675 — Fork (D-combinator): X g h Y; assignment recipes also read the target, and definition recipes call the defined function/operator
X←3 ⋄ g←+ ⋄ h←× ⋄ Y←4 ⋄ (X g h)Y   ⍝ 4

⍝ aplcart/table.tsv:2676 — Fork (Φ-combinator): (f Y)g(h Y); assignment recipes also read the target, and definition recipes call the defined function/operator
f←- ⋄ g←+ ⋄ h←× ⋄ Y←2 ⋄ (f g h)Y   ⍝ ¯1

⍝ aplcart/table.tsv:2677 — Atop (B-combinator): f g Y; assignment recipes also read the target, and definition recipes call the defined function/operator
f←- ⋄ g←× ⋄ Y←2 ⋄ (f g)Y   ⍝ ¯1

⍝ aplcart/table.tsv:2679 — Fast: Is Y a Scalar?; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←1 ⋄ 0=⍴⍴Y   ⍝ 1⍴1

⍝ aplcart/table.tsv:2680 — Fast: Is Y a Simple Array?; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←1 2 ⋄ 1=≡,Y   ⍝ 1

⍝ aplcart/table.tsv:2681 — Fast: Does Y have an empty first dimension?; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←0 3⍴0 ⋄ 0=↑⍴Y   ⍝ 1

⍝ aplcart/table.tsv:2682 — Fast: The item in the bottom right of Y; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←2 3⍴⍳6 ⋄ ↑⌽,Y   ⍝ 6

⍝ aplcart/table.tsv:2683 — Fast: The subset of Yv in the index positions defined by M (equivalent to Yv[M]); assignment recipes also read the target, and definition recipes call the defined function/operator
M←3 1 ⋄ Yv←10 20 30 ⋄ M⊃¨⊂Yv   ⍝ 30 10

⍝ aplcart/table.tsv:2684 — Fast: The positions in Yv corresponding to the 1s in Av; assignment recipes also read the target, and definition recipes call the defined function/operator
Av←1 0 1 ⋄ Yv←10 20 30 ⋄ Av/⍳⍴Yv   ⍝ 1 3

⍝ aplcart/table.tsv:2685 — Fast: Is Y non-empty?; assignment recipes also read the target, and definition recipes call the defined function/operator
Y←3 1 5 2 4 ⋄ ~0∊⍴Y   ⍝ 1

⍝ aplcart/table.tsv:2688 — Fast: A nested vector comprising vectors that each correspond to a position in the original vectors of Yv – the first vector contains the first item from each vector in Yv, padded to be the same length as the largest vector, and so on; assignment recipes also read the target, and definition recipes call the defined function/operator
Yv←(1 2⋄ 3 4 5) ⋄ ↓⍉⊃Yv   ⍝ (1 3 ⋄ 2 4 ⋄ 0 5)

⍝ aplcart/table.tsv:2689 — Fast: Round to nearest integer; assignment recipes also read the target, and definition recipes call the defined function/operator
N←¯1.5 0.5 1.6 ⋄ ⌊0.5+N   ⍝ ¯1 1 2

⍝ aplcart/table.tsv:2692 — Printable ASCII
31↓•UCS⍳126
' !"#$%&''()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_`abcdefghijklmnopqrstuvwxyz{|}~'

⍝ aplcart/table.tsv:2694 — Fast: 'name' redefined to be its value without the -Js trailing major cells (only fast when Js is negative); assignment recipes also read the target, and definition recipes call the defined function/operator
name←1 2 3 4 ⋄ Js←¯2 ⋄ name↓⍨←Js ⋄ name   ⍝ 1 2

⍝ aplcart/table.tsv:2695 — Modified Assignment (also dfns/dops)
Foo←{a←⍵ ⋄ b←⍵ ⋄ Plus←+ ⋄ a Plus∘⊢←1 ⋄ b Plus←1 ⋄ a b} ⋄ Foo 3
4 1

⍝ aplcart/table.tsv:2702 — Indexing (scatter-point)
X←3 4⍴⍳12 ⋄ Iv←1 2 ⋄ Jv←3 4 ⋄ X[Iv Jv]   ⍝ 2 12

⍝ aplcart/table.tsv:2730 — Multiple assignment
(a b)←1 2 ⋄ a+b   ⍝ 3

⍝ aplcart/table.tsv:2731 — Swap variable values
(a b)←5 8 ⋄ (a b)←b a ⋄ a-b   ⍝ 3

⍝ aplcart/table.tsv:2762 — Golden ratio (direct formula)
2÷¯1+5*÷2   ⍝ 1.618033988749895

⍝ aplcart/table.tsv:2763 — Fast: The number of leading 1s in each row of B; assignment recipes also read the target, and definition recipes call the defined function/operator
B←2 4⍴1 1 0 1 1 0 1 1 ⋄ +/∧\B   ⍝ 2 1

⍝ aplcart/table.tsv:2766 — High-rank array: ↑(⊂X),(⊂Y),(⊂Z); assignment recipes also read the target, and definition recipes call the defined function/operator
X←1 2 ⋄ Y←3 4 ⋄ Z←5 6 ⋄ [X ⋄ Y ⋄ Z]   ⍝ 3 2⍴1 2 3 4 5 6

⍝ aplcart/table.tsv:2768 — Precedence: Z×X+Y; assignment recipes also read the target, and definition recipes call the defined function/operator
X←1 2 ⋄ Y←3 4 ⋄ Z←2 ⋄ (X+Y)×Z   ⍝ 8 12

⍝ aplcart/table.tsv:2775 — Reassign main diagonal of matrix; assignment recipes also read the target, and definition recipes call the defined function/operator
Xm←3 3⍴⍳9 ⋄ Yv←0 ⋄ (1 1⍉Xm)←Yv ⋄ Xm   ⍝ 3 3⍴0 2 3 4 0 6 7 8 0

⍝ aplcart/table.tsv:2776 — Vector: (⊂X),(⊂Y),(⊂Z); assignment recipes also read the target, and definition recipes call the defined function/operator
X←1 2 ⋄ Y←3 4 ⋄ Z←5 6 ⋄ (X ⋄ Y ⋄ Z)   ⍝ (1 2 ⋄ 3 4 ⋄ 5 6)

⍝ aplcart/table.tsv:2781 — Prototypical monadic dfn; assignment recipes also read the target, and definition recipes call the defined function/operator
f←{⍵} ⋄ f 3   ⍝ 3

⍝ aplcart/table.tsv:2782 — Prototypical dyadic dfn; assignment recipes also read the target, and definition recipes call the defined function/operator
f←{⍺} ⋄ 2 f 3   ⍝ 2

⍝ aplcart/table.tsv:2783 — Shy dfn/dop result
f←{_←⍵+1} ⋄ f 2   ⍝ 3

⍝ aplcart/table.tsv:2784 — Dyadic operator (DOP) taking an array operand (W) to derive a monadic operator (which in turn takes an array operand to derive a dyadic function)
DOP←{⍺+⍶+⍵×⍹} ⋄ X←100 ⋄ W←10 ⋄ Z←3 ⋄ Y←1 2 ⋄ X(W(DOP Z))Y
113 116

⍝ aplcart/table.tsv:2785 — Dyadic operator (DOP) taking a function operand to derive a monadic operator (which in turn takes an array operand to derive a dyadic function)
DOP←{⍺+⍶+⍹ ⍵} ⋄ X←100 ⋄ W←10 ⋄ g←- ⋄ Y←1 2 ⋄ X(W(DOP g))Y
109 108

⍝ aplcart/table.tsv:2786 — Quote character
('''' ⋄ 'can''t')   ⍝ '''' ('can''t')

⍝ aplcart/table.tsv:2788 — Indexing (slicing)
X←3 4⍴⍳12 ⋄ I←1 3 ⋄ J←2 4 ⋄ X[I;J]   ⍝ 2 2⍴2 4 10 12

⍝ aplcart/table.tsv:2789 — Fast: A Boolean mask indicating the leading blank spaces in each row of D; assignment recipes also read the target, and definition recipes call the defined function/operator
D←2 4⍴'  ab c d'  ⋄ ∧\' '=D   ⍝ 2 4⍴1 1 0 0 1 0 0 0

⍝ aplcart/table.tsv:2793 — Structural assignment: Replace all items, shape unchanged; assignment recipes also read the target, and definition recipes call the defined function/operator
name←2 2⍴⍳4 ⋄ Y←9 8 ⋄ (,name)←⊂Y ⋄ name   ⍝ 2 2⍴(9 8 ⋄ 9 8 ⋄ 9 8 ⋄ 9 8)

⍝ aplcart/table.tsv:2900 — Golden ratio (as a limit)
+∘÷⍣=⍨1   ⍝ 1.618033988749897

⍝ aplcart/table.tsv:2901 — Fast: A nested vector comprising simple character vectors constructed from the rows of Dm (which must be of depth 1) with all blank spaces removed; assignment recipes also read the target, and definition recipes call the defined function/operator
Dm←3 3⍴'abc   de '  ⋄ ~∘' '¨↓Dm   ⍝ ('abc' ⋄ '' ⋄ 'de')

⍝ aplcart/table.tsv:2941 — Prototypical monadic dop deriving monadic functions; assignment recipes also read the target, and definition recipes call the defined function/operator
op←{⍶ ⍵} ⋄ +op 3   ⍝ 3

⍝ aplcart/table.tsv:2942 — Prototypical dyadic dop deriving monadic functions; assignment recipes also read the target, and definition recipes call the defined function/operator
op←{⍹ ⍵} ⋄ +op- 3   ⍝ ¯3

⍝ aplcart/table.tsv:2943 — Prototypical monadic dop deriving dyadic functions; assignment recipes also read the target, and definition recipes call the defined function/operator
op←{⍶ ⍺} ⋄ 2 +op 3   ⍝ 2

⍝ aplcart/table.tsv:2944 — Prototypical dyadic dop deriving dyadic functions; assignment recipes also read the target, and definition recipes call the defined function/operator
op←{⍹ ⍺} ⋄ 2 +op- 3   ⍝ ¯2

⍝ aplcart/table.tsv:2945 — Meaning of life (modern); pure Execute recipe checked against Dyalog with fixed settings
⍎⌽⍕⌈*π≡⍬   ⍝ 42

⍝ aplcart/table.tsv:2946 — Fast: The number of leading blank spaces in each row of D; assignment recipes also read the target, and definition recipes call the defined function/operator
D←2 4⍴'  ab c d'  ⋄ +/∧\' '=D   ⍝ 2 1

⍝ aplcart/table.tsv:3768 — An expression giving itself; assignment recipes also read the target, and definition recipes call the defined function/operator
1⌽,⍨9⍴'''1⌽,⍨9⍴'''   ⍝ '1⌽,⍨9⍴''''''1⌽,⍨9⍴'''''''

⍝ aplcart/table.tsv:3774 — Meaning of life (traditional); pure Execute recipe checked against Dyalog with fixed settings
⍎⊖⍕⊃⊂|⌊-*+π⌈×÷!⌽⍉⌹~⍴⍋⍒,⍟?⍳0   ⍝ 42

⍝ aplcart/table.tsv:3775 — Generate random UUIDv4; Original UUIDv4 generator; assert length, hyphen positions, version 4, variant bits and lowercase hexadecimal alphabet, not a random sample; Independent expected truth and Dyalog check
u←'-'@(4+5×⍳4)⊢(•D,•C•A)[4(9+|)@20⊢5@15?36⍴16] ⋄ (36=≢u)∧(∧/'-'=u[9 14 19 24])∧('4'=u[15])∧(u[20]∊'89ab')∧∧/(u~'-')∊•D,•C•A
1

⍝ aplcart/tt.tsv:1 — *N×0J1
N←0 0.5 1 ⋄ ¯12∘○N
1 0.8775825618903728j0.479425538604203 0.5403023058681398j0.8414709848078965

⍝ aplcart/tt.tsv:2 — *N×0J1; Reuse concrete inputs from aplcart/table.tsv:574; execute this alternate recipe independently
N←0 0.5 1 ⋄ {¯12○⍵}N
1 0.8775825618903728j0.479425538604203 0.5403023058681398j0.8414709848078965

⍝ aplcart/tt.tsv:3 — +N (complex conjugate)
N←3j4 0j2 ⋄ ¯10∘○N   ⍝ 3j¯4 0j¯2

⍝ aplcart/tt.tsv:4 — +N (complex conjugate); Reuse concrete inputs from aplcart/table.tsv:572; execute this alternate recipe independently
N←3j4 0j2 ⋄ {¯10○⍵}N   ⍝ 3j¯4 0j¯2

⍝ aplcart/tt.tsv:5 — 2's-complement bit-wise NOT
J←¯3 0 1 7 ⋄ ¯1∘-J   ⍝ 2 ¯1 ¯2 ¯8

⍝ aplcart/tt.tsv:6 — 2's-complement bit-wise NOT; Reuse concrete inputs from aplcart/table.tsv:533; execute this alternate recipe independently
J←¯3 0 1 7 ⋄ {¯1-⍵}J   ⍝ 2 ¯1 ¯2 ¯8

⍝ aplcart/tt.tsv:7 — 2-argument arctangent (M:x, N:y)
M←1 ¯1 ¯1 1 ⋄ N←1 1 ¯1 ¯1 ⋄ M(12○⊣+0J1×⊢)N
0.7853981633974483 2.356194490192345 ¯2.356194490192345 ¯0.7853981633974483

⍝ aplcart/tt.tsv:8 — 2-argument arctangent (M:x, N:y); Reuse concrete inputs from aplcart/table.tsv:1098; execute this alternate recipe independently
M←1 ¯1 ¯1 1 ⋄ N←1 1 ¯1 ¯1 ⋄ M{12○⍺+0J1×⍵}N
0.7853981633974483 2.356194490192345 ¯2.356194490192345 ¯0.7853981633974483

⍝ aplcart/tt.tsv:9 — A magic square, odd side Js
Js←3 ⋄ ((⍳-∘⌈÷∘2)(⊣⊖⌽),⍨⍴∘⍳×⍨)Js   ⍝ 3 3⍴8 1 6 3 5 7 4 9 2

⍝ aplcart/tt.tsv:10 — A magic square, odd side Js; Reuse concrete inputs from aplcart/table.tsv:1561; execute this alternate recipe independently
Js←3 ⋄ {r⊖(r←(⍳⍵)-⌈⍵÷2)⌽⍵ ⍵⍴⍳⍵*2}Js   ⍝ 3 3⍴8 1 6 3 5 7 4 9 2

⍝ aplcart/tt.tsv:11 — Absolute distance between X and Y
M←2 3 4 ⋄ N←4 9 16 ⋄ M(|-)N   ⍝ 2 6 12

⍝ aplcart/tt.tsv:12 — Absolute distance between X and Y; Reuse concrete inputs from aplcart/table.tsv:778; execute this alternate recipe independently
M←2 3 4 ⋄ N←4 9 16 ⋄ M{|⍺-⍵}N   ⍝ 2 6 12

⍝ aplcart/tt.tsv:15 — Add line numbers to table Xm; Format exact counts as ordinary numbers
Ym←2 3⍴⍳6 ⋄ ((3⌽']  [',⍕)⍤0∘⍳∘(0+≢),⍕)Ym
2 10⍴'[1]  1 2 3[2]  4 5 6'

⍝ aplcart/tt.tsv:16 — Add line numbers to table Xm; Reuse concrete inputs from aplcart/table.tsv:1530; execute this alternate recipe independently; Format exact counts as ordinary numbers
Ym←2 3⍴⍳6 ⋄ {({3⌽']  [',⍕⍵}⍤0⍳0+≢⍵),⍕⍵}Ym
2 10⍴'[1]  1 2 3[2]  4 5 6'

⍝ aplcart/tt.tsv:17 — Addition Table for Iv down and Jv across
Iv←¯1 0 1 ⋄ Jv←2 3 ⋄ Iv +⌝ Jv   ⍝ 3 2⍴1 2 2 3 3 4

⍝ aplcart/tt.tsv:18 — Addition Table for Numbers up to Js; optional {X} instantiated as dyadic use
Js←4 ⋄  +⌝ ⍨∘⍳Js   ⍝ 4 4⍴2 3 4 5 3 4 5 6 4 5 6 7 5 6 7 8

⍝ aplcart/tt.tsv:19 — Addition Table for Numbers up to Js; Reuse concrete inputs from aplcart/table.tsv:1004; execute this alternate recipe independently
Js←4 ⋄ {i +⌝ i←⍳⍵}Js   ⍝ 4 4⍴2 3 4 5 3 4 5 6 4 5 6 7 5 6 7 8

⍝ aplcart/tt.tsv:20 — Adjust Ym to width Is (positive Is to pad/chop on right, negative Is to pad/chop on left)
Is←4 ⋄ Ym←2 3⍴⍳6 ⋄ Is(⊢↑⍨⊢∘≢,⊣)Ym   ⍝ 2 4⍴1 2 3 0 4 5 6 0

⍝ aplcart/tt.tsv:21 — Adjust Ym to width Is (positive Is to pad/chop on right, negative Is to pad/chop on left)
Is←4 ⋄ Ym←2 3⍴⍳6 ⋄ Is{((≢⍵),⍺)↑⍵}Ym   ⍝ 2 4⍴1 2 3 0 4 5 6 0

⍝ aplcart/tt.tsv:22 — Align the diagonals of a matrix into columns (with wrap-around)
Ym←2 3⍴⍳6 ⋄ (⊢⌽⍨¯1+⍳∘≢)Ym   ⍝ 2 3⍴1 2 3 5 6 4

⍝ aplcart/tt.tsv:23 — Align the diagonals of a matrix into columns (with wrap-around); Reuse concrete inputs from aplcart/table.tsv:1256; execute this alternate recipe independently
Ym←2 3⍴⍳6 ⋄ {(¯1+⍳≢⍵)⌽⍵}Ym   ⍝ 2 3⍴1 2 3 5 6 4

⍝ aplcart/tt.tsv:24 — Aliquot sum (sum of proper divisors)
Js←12 ⋄ (+/∘∪⊢∨¯1↓⍳)Js   ⍝ 16

⍝ aplcart/tt.tsv:25 — Aliquot sum (sum of proper divisors); Reuse concrete inputs from aplcart/table.tsv:1320; execute this alternate recipe independently
Js←12 ⋄ {+/∪⍵∨¯1↓⍳⍵}Js   ⍝ 16

⍝ aplcart/tt.tsv:26 — All; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (~0∘∊)B   ⍝ 0

⍝ aplcart/tt.tsv:27 — All; Reuse concrete inputs from aplcart/table.tsv:951; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ {~0∊⍵}B   ⍝ 0

⍝ aplcart/tt.tsv:28 — All axes of array Y
Y←3 1 3 2 ⋄ ⍳∘≢∘⍴Y   ⍝ 1⍴1

⍝ aplcart/tt.tsv:29 — All axes of array Y; Reuse concrete inputs from aplcart/table.tsv:868; execute this alternate recipe independently
Y←3 1 3 2 ⋄ {⍳≢⍴⍵}Y   ⍝ 1⍴1

⍝ aplcart/tt.tsv:30 — All binary representations with Js bits (truth table with Js variables, matrix for choosing all subsets)
2∘(⍉⍴⍨⊤¯1+∘⍳*) 3
8 3⍴0 0 0 0 0 1 0 1 0 0 1 1 1 0 0 1 0 1 1 1 0 1 1 1

⍝ aplcart/tt.tsv:31 — All binary representations with Js bits (truth table with Js variables, matrix for choosing all subsets)
Js←3 ⋄ {⍉(⍵⍴2)⊤¯1+⍳2*⍵}Js
8 3⍴0 0 0 0 0 1 0 1 0 0 1 1 1 0 0 1 0 1 1 1 0 1 1 1

⍝ aplcart/tt.tsv:32 — All column indices of array Ym
Ym←2 3⍴⍳6 ⋄ (⍳0⊥⍴)Ym   ⍝ 1 2 3

⍝ aplcart/tt.tsv:33 — All column indices of array Ym
Ym←2 3⍴⍳6 ⋄ {⍳0⊥⍴⍵}Ym   ⍝ 1 2 3

⍝ aplcart/tt.tsv:34 — All divisors of Js; Reuse concrete inputs from aplcart/table.tsv:1058; execute this alternate recipe independently
Js←4 ⋄ (∪⊢∨⍳)Js   ⍝ 1 2 4

⍝ aplcart/tt.tsv:35 — All divisors of Js; Reuse concrete inputs from aplcart/table.tsv:1058; execute this alternate recipe independently
Js←4 ⋄ {∪⍵∨⍳⍵}Js   ⍝ 1 2 4

⍝ aplcart/tt.tsv:36 — All indices of Y
Y←3 1 3 2 ⋄ (,∘⍳⍴)Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:37 — All indices of Y; Reuse concrete inputs from aplcart/table.tsv:1048; execute this alternate recipe independently
Y←3 1 3 2 ⋄ {,⍳⍴⍵}Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:38 — All indices of array Y
Y←3 1 3 2 ⋄ (⍳⍴)Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:39 — All indices of array Y; Reuse concrete inputs from aplcart/table.tsv:828; execute this alternate recipe independently
Y←3 1 3 2 ⋄ {⍳⍴⍵}Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:40 — All possible subvectors grouped by length (Yv must be simple)
Yv←1 2 3 4 ⋄ (⍳≢Yv){,/⍺↕⍵}¨⊂Yv
(1 2 3 4 ⋄ (1 2 ⋄ 2 3 ⋄ 3 4) ⋄ (1 2 3 ⋄ 2 3 4) ⋄ 1⍴⊂(1 2 3 4))

⍝ aplcart/tt.tsv:41 — All possible subvectors grouped by length (Yv must be simple); Reuse concrete inputs from aplcart/table.tsv:1259; execute this alternate recipe independently
Yv←1 2 3 4 ⋄ {⍳≢,/¨⊂⍵}Yv   ⍝ 1⍴1

⍝ aplcart/tt.tsv:42 — All possible subvectors of length Is (Yv must be simple); dfns display import/wrappers omitted to test underlying arrays
(2 4 6 8 10 12 ⋄ ,/2↕  2 4 6 8 10 12 ⋄ ,/4↕  2 4 6 8 10 12)
(2 4 6 8 10 12 ⋄ (2 4 ⋄ 4 6 ⋄ 6 8 ⋄ 8 10 ⋄ 10 12) ⋄ (2 4 6 8 ⋄ 4 6 8 10 ⋄ 6 8 10 12))

⍝ aplcart/tt.tsv:43 — All row indices of matrix Ym
(4 3⍴0.25 ⋄ ⍳∘≢ 4 3⍴0.25)
(4 3⍴0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 0.25 ⋄ 1 2 3 4)

⍝ aplcart/tt.tsv:44 — All row indices of matrix Ym
Ym←2 3⍴⍳6 ⋄ {⍳≢⍵}Ym   ⍝ 1 2

⍝ aplcart/tt.tsv:45 — All tuples of corresponding elements of ⍳¨Jv (for large Jv even above length 15)
Jv←2 3 2 ⋄ (⍉1+⊢⊤¯1+∘⍳×/)Jv
12 3⍴1 1 1 1 1 2 1 2 1 1 2 2 1 3 1 1 3 2 2 1 1 2 1 2 2 2 1 2 2 2 2 3 1 2 3 2

⍝ aplcart/tt.tsv:46 — All tuples of corresponding elements of ⍳¨Jv (for large Jv even above length 15); Reuse concrete inputs from aplcart/table.tsv:1387; execute this alternate recipe independently
Jv←2 3 2 ⋄ {⍉1+⍵⊤¯1+⍳×/⍵}Jv
12 3⍴1 1 1 1 1 2 1 2 1 1 2 2 1 3 1 1 3 2 2 1 1 2 1 2 2 2 1 2 2 2 2 3 1 2 3 2

⍝ aplcart/tt.tsv:47 — All tuples of corresponding elements of ⍳¨Jv (for small Jv of max length 15)
,∘⍳ 3 6
(1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 1 4 ⋄ 1 5 ⋄ 1 6 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3 ⋄ 2 4 ⋄ 2 5 ⋄ 2 6 ⋄ 3 1 ⋄ 3 2 ⋄ 3 3 ⋄ 3 4 ⋄ 3 5 ⋄ 3 6)

⍝ aplcart/tt.tsv:48 — All tuples of corresponding elements of ⍳¨Jv (for small Jv of max length 15)
Jv←2 3 ⋄ {,⍳⍵}Jv   ⍝ (1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3)

⍝ aplcart/tt.tsv:49 — Alternating series (1,-1,2,-2, …) of length Js; optional {X} instantiated as dyadic use; Reduce each prefix to preserve right association
Js←4 ⋄ {-/¨,\⍳⍵}Js   ⍝ 1 ¯1 2 ¯2

⍝ aplcart/tt.tsv:50 — Alternating series (1,-1,2,-2, …) of length Js; Reuse concrete inputs from aplcart/table.tsv:969; execute this alternate recipe independently; Reduce each prefix to preserve right association
Js←4 ⋄ {-/¨,\⍳⍵}Js   ⍝ 1 ¯1 2 ¯2

⍝ aplcart/tt.tsv:51 — An array that begins with 1↑N and has pair-wise sums 1↓N
((¯1⊥¨,\) 3,4 5 5 6 ⋄ (+/∘(2∘↕)) (¯1⊥¨,\) 3,4 5 5 6)
(3 1 4 1 5 ⋄ 4 5 5 6)

⍝ aplcart/tt.tsv:52 — An array that begins with 1↑N and has pair-wise sums 1↓N
N←1 2 3 4 ⋄ {¯1⊥¨⍪⍀⍵}N   ⍝ 1 1 2 2

⍝ aplcart/tt.tsv:53 — Angle of right triangle with height M and width N
M←1 ⋄ N←1 2 ⋄ M(¯3○÷)N   ⍝ 0.7853981633974483 0.4636476090008061

⍝ aplcart/tt.tsv:54 — Angle of right triangle with height M and width N; Reuse concrete inputs from aplcart/table.tsv:794; execute this alternate recipe independently
M←1 ⋄ N←1 2 ⋄ M{¯3○⍺÷⍵}N   ⍝ 0.7853981633974483 0.4636476090008061

⍝ aplcart/tt.tsv:55 — Annual rate to modal rate
M←12 ⋄ N←0.05 ⋄ M(¯1+⊣*∘÷⍨1+⊢)N   ⍝ 0.004074123783648353

⍝ aplcart/tt.tsv:56 — Annual rate to modal rate; Reuse concrete inputs from aplcart/table.tsv:1302; execute this alternate recipe independently
M←12 ⋄ N←0.05 ⋄ M{¯1+(1+⍵)*÷⍺}N   ⍝ 0.004074123783648353

⍝ aplcart/tt.tsv:57 — Annuity coefficient: I periods at interest N
I←12 ⋄ N←0.05 ⋄ I(⊢÷∘⍉1+⊣ ×⌝ 1+⊢)N   ⍝ 0.003676470588235294

⍝ aplcart/tt.tsv:58 — Annuity coefficient: I periods at interest N; Reuse concrete inputs from aplcart/table.tsv:1456; execute this alternate recipe independently
I←12 ⋄ N←0.05 ⋄ I{⍵÷⍉1+⍺ ×⌝ 1+⍵}N   ⍝ 0.003676470588235294

⍝ aplcart/tt.tsv:59 — Anti-diagonal of any rank array
Y←2 3 4⍴⍳24 ⋄ (⌽⍉⍨1*⍴)Y   ⍝ 4 19

⍝ aplcart/tt.tsv:60 — Anti-diagonal of any rank array; Reuse concrete inputs from aplcart/table.tsv:1147; execute this alternate recipe independently; Add the missing ⍵ arguments in the upstream anti-diagonal dfn
Y←2 3 4⍴⍳24 ⋄ {(1*⍴⍵)⍉⌽⍵}Y   ⍝ 4 19

⍝ aplcart/tt.tsv:61 — Any true?
(1∘∊ 0 0 0 0 ⋄ 1∘∊ 0 1 1 0)   ⍝ 0 1

⍝ aplcart/tt.tsv:62 — Any true?
B←1 1 0 1 0 1 0 ⋄ {1∊⍵}B   ⍝ 1

⍝ aplcart/tt.tsv:63 — Append a column of 1s
Y←3 1 3 2 ⋄ (,∘1)Y   ⍝ 3 1 3 2 1

⍝ aplcart/tt.tsv:64 — Append a column of 1s; Reuse concrete inputs from aplcart/table.tsv:810; execute this alternate recipe independently
Y←3 1 3 2 ⋄ {⍵,1}Y   ⍝ 3 1 3 2 1

⍝ aplcart/tt.tsv:65 — Append a row of 1s
Y←3 1 3 2 ⋄ (⍪∘1)Y   ⍝ 3 1 3 2 1

⍝ aplcart/tt.tsv:66 — Append a row of 1s; Reuse concrete inputs from aplcart/table.tsv:815; execute this alternate recipe independently
Y←3 1 3 2 ⋄ {⍵⍪1}Y   ⍝ 3 1 3 2 1

⍝ aplcart/tt.tsv:67 — Appending X as additional major cell of Y, adjusting dimensions of both as necessary
X←1 2 3 4 ⋄ Y←2 3⍴⍳6 ⋄ X(⊃⊂⍤¯1⍤⊢,⊂⍤⊣)Y   ⍝ 3 4⍴1 2 3 0 4 5 6 0 1 2 3 4

⍝ aplcart/tt.tsv:68 — Appending X as additional major cell of Y, adjusting dimensions of both as necessary; Reuse concrete inputs from aplcart/table.tsv:1367; execute this alternate recipe independently
X←1 2 3 4 ⋄ Y←2 3⍴⍳6 ⋄ X{⊃(⊂⍤¯1⊢⍵),⊂⍺}Y   ⍝ 3 4⍴1 2 3 0 4 5 6 0 1 2 3 4

⍝ aplcart/tt.tsv:69 — Arccosecant
N←4 9 16 ⋄ (¯1○÷)N
0.2526802551420786 0.1113410143409639 0.06254076179649139

⍝ aplcart/tt.tsv:70 — Arccosecant; Reuse concrete inputs from aplcart/table.tsv:795; execute this alternate recipe independently
N←4 9 16 ⋄ {¯1○÷⍵}N
0.2526802551420786 0.1113410143409639 0.06254076179649139

⍝ aplcart/tt.tsv:71 — Arccosine N
N←0.25 0.5 0.75 ⋄ ¯2∘○N
1.318116071652818 1.047197551196598 0.7227342478134157

⍝ aplcart/tt.tsv:72 — Arccosine N; Reuse concrete inputs from aplcart/table.tsv:575; execute this alternate recipe independently
N←0.25 0.5 0.75 ⋄ {¯2○⍵}N
1.318116071652818 1.047197551196598 0.7227342478134157

⍝ aplcart/tt.tsv:73 — Arccotangent
N←4 9 16 ⋄ (¯3○÷)N
0.2449786631268641 0.1106572211738956 0.06241880999595735

⍝ aplcart/tt.tsv:74 — Arccotangent; Reuse concrete inputs from aplcart/table.tsv:797; execute this alternate recipe independently
N←4 9 16 ⋄ {¯3○÷⍵}N
0.2449786631268641 0.1106572211738956 0.06241880999595735

⍝ aplcart/tt.tsv:75 — Arcsecant
N←4 9 16 ⋄ (¯2○÷)N
1.318116071652818 1.459455312453933 1.508255564998405

⍝ aplcart/tt.tsv:76 — Arcsecant; Reuse concrete inputs from aplcart/table.tsv:796; execute this alternate recipe independently
N←4 9 16 ⋄ {¯2○÷⍵}N
1.318116071652818 1.459455312453933 1.508255564998405

⍝ aplcart/tt.tsv:77 — Arcsine N
N←0.25 0.5 0.75 ⋄ ¯1∘○N
0.2526802551420786 0.5235987755982988 0.848062078981481

⍝ aplcart/tt.tsv:78 — Arcsine N; Reuse concrete inputs from aplcart/table.tsv:571; execute this alternate recipe independently
N←0.25 0.5 0.75 ⋄ {¯1○⍵}N
0.2526802551420786 0.5235987755982988 0.848062078981481

⍝ aplcart/tt.tsv:79 — Arctangent N
N←0.25 0.5 0.75 ⋄ ¯3∘○N
0.2449786631268641 0.4636476090008061 0.6435011087932844

⍝ aplcart/tt.tsv:80 — Arctangent N; Reuse concrete inputs from aplcart/table.tsv:576; execute this alternate recipe independently
N←0.25 0.5 0.75 ⋄ {¯3○⍵}N
0.2449786631268641 0.4636476090008061 0.6435011087932844

⍝ aplcart/tt.tsv:81 — Are I and J co-prime?
I←2 3 4 ⋄ J←4 9 16 ⋄ I(1=∨)J   ⍝ 0 0 0

⍝ aplcart/tt.tsv:82 — Are I and J co-prime?; Reuse concrete inputs from aplcart/table.tsv:780; execute this alternate recipe independently
I←2 3 4 ⋄ J←4 9 16 ⋄ I{1=⍺∨⍵}J   ⍝ 0 0 0

⍝ aplcart/tt.tsv:83 — Are Is and Js amicable numbers?; Reuse concrete inputs from aplcart/table.tsv:1524; execute this alternate recipe independently
Is←220 ⋄ Js←284 ⋄ Is(∧/+=∘(+/∘∪⊢∨⍳)¨,)Js
1

⍝ aplcart/tt.tsv:84 — Are Is and Js amicable numbers?; Reuse concrete inputs from aplcart/table.tsv:1524; execute this alternate recipe independently
Is←220 ⋄ Js←284 ⋄ Is{∧/(⍺+⍵)={+/∪⍵∨⍳⍵}¨⍺,⍵}Js
1

⍝ aplcart/tt.tsv:85 — Are Is and Js betrothed numbers?; Reuse concrete inputs from aplcart/table.tsv:1539; execute this alternate recipe independently
Is←48 ⋄ Js←75 ⋄ Is(∧/+=¯1+∘(+/∘∪⊢∨⍳)¨,)Js
1

⍝ aplcart/tt.tsv:86 — Are Is and Js betrothed numbers?; Reuse concrete inputs from aplcart/table.tsv:1539; execute this alternate recipe independently
Is←48 ⋄ Js←75 ⋄ Is{∧/(⍺+⍵)=¯1+{+/∪⍵∨⍳⍵}¨⍺,⍵}Js
1

⍝ aplcart/tt.tsv:87 — Are X and Y permutations of each other?
X←1 2 3 1 ⋄ Y←3 1 1 2 ⋄ X≡⍥(⊂∘⍋⌷⊢)Y   ⍝ 1

⍝ aplcart/tt.tsv:88 — Are X and Y permutations of each other?; Reuse concrete inputs from aplcart/table.tsv:1262; execute this alternate recipe independently
X←1 2 3 1 ⋄ Y←3 1 1 2 ⋄ X{((⊂⍋⍺)⌷⍺)≡(⊂⍋⍵)⌷⍵}Y
1

⍝ aplcart/tt.tsv:89 — Are all elements of simple Y equal?
((1=≢∘∪∘,) 2 3⍴'a' ⋄ (1=≢∘∪∘,) 2 3⍴'ab')
1 0

⍝ aplcart/tt.tsv:90 — Are all elements of simple Y equal?
Y←2 3⍴7 ⋄ {1=≢∪,⍵}Y   ⍝ 1

⍝ aplcart/tt.tsv:91 — Are all major cells distinct?
((⊢≡∪) 'abcd' ⋄ (⊢≡∪) 'abca')   ⍝ 1 0

⍝ aplcart/tt.tsv:92 — Are all major cells distinct?
Y←3 2⍴1 2 3 4 1 2 ⋄ {⍵≡∪⍵}Y   ⍝ 0

⍝ aplcart/tt.tsv:93 — Are all major cells identical?
Y←3 1 3 2 ⋄ (1≥≢∘∪)Y   ⍝ 0

⍝ aplcart/tt.tsv:94 — Are all major cells identical?; Reuse concrete inputs from aplcart/table.tsv:1057; execute this alternate recipe independently
Y←3 1 3 2 ⋄ {1≥≢∪⍵}Y   ⍝ 0

⍝ aplcart/tt.tsv:95 — Are all major cells of Y identical?
((1=≢∘∪) 'abc' 'abc' 'abc' ⋄ (1=≢∘∪) 'abc' 'abc' 'abd')
1 0

⍝ aplcart/tt.tsv:96 — Are all major cells of Y identical?
Y←3 2⍴1 2 ⋄ {1=≢∪⍵}Y   ⍝ 1

⍝ aplcart/tt.tsv:97 — Are any major cells distinct?
Y←3 1 3 2 ⋄ (1<≢∘∪)Y   ⍝ 1

⍝ aplcart/tt.tsv:98 — Are any major cells distinct?; Reuse concrete inputs from aplcart/table.tsv:1055; execute this alternate recipe independently
Y←3 1 3 2 ⋄ {1<≢∪⍵}Y   ⍝ 1

⍝ aplcart/tt.tsv:99 — Are any major cells identical?
Y←3 1 3 2 ⋄ (0∊≠)Y   ⍝ 1

⍝ aplcart/tt.tsv:100 — Are any major cells identical?; Reuse concrete inputs from aplcart/table.tsv:832; execute this alternate recipe independently
Y←3 1 3 2 ⋄ {0∊≠⍵}Y   ⍝ 1

⍝ aplcart/tt.tsv:101 — Are any true?
B←2 3⍴1 0 0 0 0 0 ⋄ ∨/∘,B   ⍝ 1

⍝ aplcart/tt.tsv:102 — Are any true?; Reuse concrete inputs from aplcart/table.tsv:748; execute this alternate recipe independently
B←2 3⍴1 0 0 0 0 0 ⋄ {∨/,⍵}B   ⍝ 1

⍝ aplcart/tt.tsv:103 — Are characters of D lowercase?
D←'Abc 19 Σς!'  ⋄ (1∘•C≠⊢)D   ⍝ 0 1 1 0 0 0 0 0 1 0

⍝ aplcart/tt.tsv:104 — Are characters of D lowercase?
D←'Abc 19 Σς!'  ⋄ {⍵≠1•C⍵}D   ⍝ 0 1 1 0 0 0 0 0 1 0

⍝ aplcart/tt.tsv:105 — Are characters of D titlecase?
D←'Abc 19 Σς!'  ⋄ ((¯1∘•C≠⊢)∧1∘•C≠⊢)D   ⍝ 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/tt.tsv:106 — Are characters of D titlecase?
D←'Abc 19 Σς!'  ⋄ {(⍵≠¯1•C⍵)∧⍵≠1•C⍵}D   ⍝ 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/tt.tsv:107 — Are characters of D uppercase?
D←'Abc 19 Σς!'  ⋄ (¯1∘•C≠⊢)D   ⍝ 1 0 0 0 0 0 0 1 0 0

⍝ aplcart/tt.tsv:108 — Are characters of D uppercase?
D←'Abc 19 Σς!'  ⋄ {⍵≠¯1•C⍵}D   ⍝ 1 0 0 0 0 0 0 1 0 0

⍝ aplcart/tt.tsv:109 — Are columns of N in ascending order?; Reuse concrete inputs from aplcart/table.tsv:1207; execute this alternate recipe independently
N←3 2⍴1 3 2 2 3 1 ⋄ (∧⌿⊢=⌈⍀)N   ⍝ 1 0

⍝ aplcart/tt.tsv:110 — Are columns of N in ascending order?; Reuse concrete inputs from aplcart/table.tsv:1207; execute this alternate recipe independently
N←3 2⍴1 3 2 2 3 1 ⋄ {∧⌿⍵=⌈⍀⍵}N   ⍝ 1 0

⍝ aplcart/tt.tsv:111 — Are columns of N in descending order?; Reuse concrete inputs from aplcart/table.tsv:1208; execute this alternate recipe independently
N←3 2⍴1 3 2 2 3 1 ⋄ (∧⌿⊢=⌊⍀)N   ⍝ 0 1

⍝ aplcart/tt.tsv:112 — Are columns of N in descending order?; Reuse concrete inputs from aplcart/table.tsv:1208; execute this alternate recipe independently
N←3 2⍴1 3 2 2 3 1 ⋄ {∧⌿⍵=⌊⍀⍵}N   ⍝ 0 1

⍝ aplcart/tt.tsv:113 — Are none true?
B←1 1 0 1 0 1 0 ⋄ (~∨/∘,)B   ⍝ 0

⍝ aplcart/tt.tsv:114 — Are none true?; Reuse concrete inputs from aplcart/table.tsv:1122; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ {~∨/,⍵}B   ⍝ 0

⍝ aplcart/tt.tsv:115 — Area of a polygon given Mv,Nv endpoints; Reuse concrete inputs from aplcart/table.tsv:1506; execute this alternate recipe independently
Mv←0 2 2 0 ⋄ Nv←0 0 3 3 ⋄ Mv(|2÷⍨+.×∘(¯1∘⌽-1∘⌽))Nv
6

⍝ aplcart/tt.tsv:116 — Area of a polygon given Mv,Nv endpoints; Reuse concrete inputs from aplcart/table.tsv:1506; execute this alternate recipe independently
Mv←0 2 2 0 ⋄ Nv←0 0 3 3 ⋄ Mv(|2÷⍨+.×∘(¯1∘⌽-1∘⌽))Nv
6

⍝ aplcart/tt.tsv:117 — Area of box with sides Nv
Nv←1 2 3 ⋄ (2×⊢+.×1∘⌽)Nv   ⍝ 22

⍝ aplcart/tt.tsv:118 — Area of box with sides Nv
Nv←1 2 3 ⋄ (2×⊢+.×1∘⌽)Nv   ⍝ 22

⍝ aplcart/tt.tsv:119 — Area of circle with radius N; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (π*∘2)N
50.26548245743669 254.4690049407732 804.247719318987

⍝ aplcart/tt.tsv:120 — Area of circle with radius N; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (π*∘2)N
50.26548245743669 254.4690049407732 804.247719318987

⍝ aplcart/tt.tsv:121 — Area of cone with height M and radius N (excluding base)
M←2 3 4 ⋄ N←4 9 16 ⋄ M(π×)N
25.13274122871834 84.82300164692441 201.0619298297468

⍝ aplcart/tt.tsv:122 — Area of cone with height M and radius N (excluding base)
M←2 3 4 ⋄ N←4 9 16 ⋄ M(π×)N
25.13274122871834 84.82300164692441 201.0619298297468

⍝ aplcart/tt.tsv:123 — Area of rectangle with sides Nv
×/ 5 10   ⍝ 50

⍝ aplcart/tt.tsv:124 — Area of rectangle with sides Nv
Nv←3 4 ⋄ ×/Nv   ⍝ 12

⍝ aplcart/tt.tsv:125 — Area of sphere with radius N
N←4 9 16 ⋄ (π4×*∘2)N
201.0619298297468 1017.876019763093 3216.990877275948

⍝ aplcart/tt.tsv:126 — Area of sphere with radius N
N←4 9 16 ⋄ (π4×*∘2)N
201.0619298297468 1017.876019763093 3216.990877275948

⍝ aplcart/tt.tsv:127 — Area of triangle with side lengths N
N←3 4 5 ⋄ 0.5∘(⊣*⍨+.××.-0,⊢)N   ⍝ 6

⍝ aplcart/tt.tsv:128 — Area of triangle with side lengths N
N←3 4 5 ⋄ 0.5∘(⊣*⍨+.××.-0,⊢)N   ⍝ 6

⍝ aplcart/tt.tsv:129 — Arithmetic mean of N
N←2 3⍴⍳6 ⋄ (+⌿÷≢)N   ⍝ 2.5 3.5 4.5

⍝ aplcart/tt.tsv:130 — Arithmetic mean of N
N←2 3⍴⍳6 ⋄ (+⌿÷≢)N   ⍝ 2.5 3.5 4.5

⍝ aplcart/tt.tsv:131 — Arithmetic mean value of all elements; Reuse concrete inputs from aplcart/table.tsv:1366; execute this alternate recipe independently
N←2 3⍴⍳6 ⋄ (+/∘,÷(×/⍴))N   ⍝ 3.5

⍝ aplcart/tt.tsv:132 — Arithmetic mean value of all elements; Reuse concrete inputs from aplcart/table.tsv:1366; execute this alternate recipe independently
N←2 3⍴⍳6 ⋄ (+/∘,÷(×/⍴))N   ⍝ 3.5

⍝ aplcart/tt.tsv:133 — Arithmetic progression vector: Js steps of Ms
(⊂3 3⍴⍳9) ×∘⍳ 3
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴2 4 6 8 10 12 14 16 18 ⋄ 3 3⍴3 6 9 12 15 18 21 24 27)

⍝ aplcart/tt.tsv:134 — Arithmetic progression vector: Js steps of Ms
Ms←0.5 ⋄ Js←4 ⋄ Ms×∘⍳Js   ⍝ 0.5 1 1.5 2

⍝ aplcart/tt.tsv:135 — Arithmetic-geometric mean
Nv←1 4 ⋄ (↑((+⌿÷≢),×⌿*∘÷≢)⍣≡)Nv   ⍝ 2.243028580287603

⍝ aplcart/tt.tsv:136 — Arithmetic-geometric mean
Nv←1 4 ⋄ (↑((+⌿÷≢),×⌿*∘÷≢)⍣≡)Nv   ⍝ 2.243028580287603

⍝ aplcart/tt.tsv:137 — Arithmetic-harmonic mean
Nv←1 4 ⋄ (↑((+⌿÷≢),≢÷1⊥÷)⍣≡)Nv   ⍝ 2

⍝ aplcart/tt.tsv:138 — Arithmetic-harmonic mean
Nv←1 4 ⋄ (↑((+⌿÷≢),≢÷1⊥÷)⍣≡)Nv   ⍝ 2

⍝ aplcart/tt.tsv:139 — Array of shape Iv filled with copies of Y
2 3 ⍴∘⊂ 'abc'
2 3⍴('abc' ⋄ 'abc' ⋄ 'abc' ⋄ 'abc' ⋄ 'abc' ⋄ 'abc')

⍝ aplcart/tt.tsv:140 — Array of shape Iv filled with copies of Y
Iv←2 3 ⋄ Y←1 2 ⋄ Iv⍴∘⊂Y   ⍝ 2 3⍴(1 2 ⋄ 1 2 ⋄ 1 2 ⋄ 1 2 ⋄ 1 2 ⋄ 1 2)

⍝ aplcart/tt.tsv:141 — Array with shape of X and content of Y; Reuse concrete inputs from aplcart/table.tsv:666; execute this alternate recipe independently
X←2 3⍴0 ⋄ Y←1 2 ⋄ X⍴⍨∘⍴⍨Y   ⍝ 2 3⍴1 2 1 2 1 2

⍝ aplcart/tt.tsv:142 — Array with shape of X and content of Y; Reuse concrete inputs from aplcart/table.tsv:666; execute this alternate recipe independently
X←2 3⍴0 ⋄ Y←1 2 ⋄ X⍴⍨∘⍴⍨Y   ⍝ 2 3⍴1 2 1 2 1 2

⍝ aplcart/tt.tsv:143 — Ascending cardinal numbers (ranking, all different)
Y←3 1 3 2 ⋄ (⍋⍋)Y   ⍝ 3 1 4 2

⍝ aplcart/tt.tsv:144 — Ascending cardinal numbers (ranking, all different)
Y←3 1 3 2 ⋄ (⍋⍋)Y   ⍝ 3 1 4 2

⍝ aplcart/tt.tsv:145 — Ascending cardinals numbers (ranking, ties equal); Reuse concrete inputs from aplcart/table.tsv:1093; execute this alternate recipe independently
Y←3 1 3 2 ⋄ (⊢⍳⍨⌷⍨∘⊂∘⍋⍨)Y   ⍝ 3 1 3 2

⍝ aplcart/tt.tsv:146 — Ascending cardinals numbers (ranking, ties equal); Reuse concrete inputs from aplcart/table.tsv:1093; execute this alternate recipe independently
Y←3 1 3 2 ⋄ (⊢⍳⍨⌷⍨∘⊂∘⍋⍨)Y   ⍝ 3 1 3 2

⍝ aplcart/tt.tsv:147 — Ascending shortlex grade
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⍋(≢,⊂)¨)Yv   ⍝ 3 1 2 4

⍝ aplcart/tt.tsv:148 — Ascending shortlex grade
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⍋(≢,⊂)¨)Yv   ⍝ 3 1 2 4

⍝ aplcart/tt.tsv:149 — Ascending shortlex sort
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⊂⊃¨⍨∘⍋(≢,⊂)¨)Yv
'a' 'b' ('aa') ('ab')

⍝ aplcart/tt.tsv:150 — Ascending shortlex sort
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⊂⊃¨⍨∘⍋(≢,⊂)¨)Yv
'a' 'b' ('aa') ('ab')

⍝ aplcart/tt.tsv:151 — Ascendingly ordered Nv-coefficient polynomial at point Ms
Ms←2 ⋄ Nv←1 3 4 ⋄ Ms⊥∘⌽Nv   ⍝ 23

⍝ aplcart/tt.tsv:152 — Ascendingly ordered Nv-coefficient polynomial at point Ms
Ms←2 ⋄ Nv←1 3 4 ⋄ Ms⊥∘⌽Nv   ⍝ 23

⍝ aplcart/tt.tsv:153 — Aspect ratio of a triangle given its side lengths
Nv←3 4 5 ⋄ (×/⊢÷+/-2×⊢)Nv   ⍝ 1.25

⍝ aplcart/tt.tsv:154 — Aspect ratio of a triangle given its side lengths
Nv←3 4 5 ⋄ (×/⊢÷+/-2×⊢)Nv   ⍝ 1.25

⍝ aplcart/tt.tsv:155 — Assign ascending ranking based on scores Nv (ties all get average ranking of used slots)
(2÷⍨⍋∘⍋+⍒∘⍋∘⌽) 3 3 3 5 8 8 21   ⍝ 2 2 2 4 5.5 5.5 7

⍝ aplcart/tt.tsv:156 — Assign ascending ranking based on scores Nv (ties all get average ranking of used slots)
Nv←3 1 2 1 ⋄ (2÷⍨⍋∘⍋+⍒∘⍋∘⌽)Nv   ⍝ 4 1.5 3 1.5

⍝ aplcart/tt.tsv:157 — Assign descending ranking based on scores Nv (ties all get average ranking of used slots)
(2÷⍨⍋∘⍒+⍒∘⍒∘⌽) 3 3 3 5 8 8 21   ⍝ 6 6 6 4 2.5 2.5 1

⍝ aplcart/tt.tsv:158 — Assign descending ranking based on scores Nv (ties all get average ranking of used slots)
Nv←3 1 2 1 ⋄ (2÷⍨⍋∘⍒+⍒∘⍒∘⌽)Nv   ⍝ 1 3.5 2 3.5

⍝ aplcart/tt.tsv:159 — Assign ranking based on non-descending scores Nv (ties all get average ranking of used slots)
(2÷⍨⍳+⍸)⍨ 3 3 3 5 8 8 21   ⍝ 2 2 2 4 5.5 5.5 7

⍝ aplcart/tt.tsv:160 — Assign ranking based on non-descending scores Nv (ties all get average ranking of used slots)
Nv←1 1 2 3 3 ⋄ (2÷⍨⍳+⍸)⍨Nv   ⍝ 1.5 1.5 3 4.5 4.5

⍝ aplcart/tt.tsv:161 — Assign ranking based on non-descending scores Nv (ties all get highest ranking of used slots)
⍳⍨ 3 3 3 5 8 8 21   ⍝ 1 1 1 4 5 5 7

⍝ aplcart/tt.tsv:162 — Assign ranking based on non-descending scores Nv (ties all get highest ranking of used slots)
Nv←1 1 2 3 3 ⋄ ⍳⍨Nv   ⍝ 1 1 3 4 4

⍝ aplcart/tt.tsv:163 — Assign ranking based on non-descending scores Nv (ties all get lowest ranking of used slots)
⍸⍨ 3 3 3 5 8 8 21   ⍝ 3 3 3 4 6 6 7

⍝ aplcart/tt.tsv:164 — Assign ranking based on non-descending scores Nv (ties all get lowest ranking of used slots)
Nv←1 1 2 3 3 ⋄ ⍸⍨Nv   ⍝ 2 2 3 5 5

⍝ aplcart/tt.tsv:165 — Attach column numbers to a matrix; Use shape rather than tally to obtain the column count; upstream ≢ fails on rectangular matrices; Concrete 2×3 matrix, independently checked in Dyalog
Ym←2 3⍴⍳6 ⋄ (⍳∘↑∘⌽∘⍴⍪⊢)Ym   ⍝ 3 3⍴1 2 3 1 2 3 4 5 6

⍝ aplcart/tt.tsv:166 — Attach column numbers to a matrix; Use shape rather than tally to obtain the column count; upstream ≢ fails on rectangular matrices; Concrete 2×3 matrix, independently checked in Dyalog
Ym←2 3⍴⍳6 ⋄ (⍳∘↑∘⌽∘⍴⍪⊢)Ym   ⍝ 3 3⍴1 2 3 1 2 3 4 5 6

⍝ aplcart/tt.tsv:167 — Attach row numbers to a matrix; Reuse concrete inputs from aplcart/table.tsv:869; execute this alternate recipe independently
Ym←2 3⍴⍳6 ⋄ (⍳∘≢,⊢)Ym   ⍝ 2 4⍴1 1 2 3 2 4 5 6

⍝ aplcart/tt.tsv:168 — Attach row numbers to a matrix; Reuse concrete inputs from aplcart/table.tsv:869; execute this alternate recipe independently
Ym←2 3⍴⍳6 ⋄ (⍳∘≢,⊢)Ym   ⍝ 2 4⍴1 1 2 3 2 4 5 6

⍝ aplcart/tt.tsv:169 — Bar chart
(⊃⍴¨∘'⎕') 3 1 4 1 5   ⍝ 5 5⍴'⎕⎕⎕  ⎕    ⎕⎕⎕⎕ ⎕    ⎕⎕⎕⎕⎕'

⍝ aplcart/tt.tsv:170 — Bar chart
J←1 3 2 ⋄ (⊃⍴¨∘'⎕')J   ⍝ 3 3⍴'⎕  ⎕⎕⎕⎕⎕ '

⍝ aplcart/tt.tsv:171 — Base-Is digit sum
Is←10 ⋄ J←123 450 7 ⋄ Is(+⌿⊥⍣¯1)J   ⍝ 6 9 7

⍝ aplcart/tt.tsv:172 — Base-Is digit sum
Is←10 ⋄ J←123 450 7 ⋄ Is(+⌿⊥⍣¯1)J   ⍝ 6 9 7

⍝ aplcart/tt.tsv:173 — Base-Is digital root
Is←10 ⋄ J←123 999 ⋄ Is(+⌿⊥⍣¯1)⍣≡J   ⍝ 6 9

⍝ aplcart/tt.tsv:174 — Base-Is digital root
Is←10 ⋄ J←123 999 ⋄ Is(+⌿⊥⍣¯1)⍣≡J   ⍝ 6 9

⍝ aplcart/tt.tsv:175 — Behead: Remove first major cell; dfns display import/wrappers omitted to test underlying arrays
(1∘↓ 1 2 3 4 ⋄ 1∘↓ (50 80 ⋄ 10 20 ⋄ 33 66) ⋄ 1∘↓ 3 3⍴⍳9)
(2 3 4 ⋄ (10 20 ⋄ 33 66) ⋄ 2 3⍴4 5 6 7 8 9)

⍝ aplcart/tt.tsv:176 — Behead: Remove first major cell
Y←3 1 3 2 ⋄ 1∘↓Y   ⍝ 1 3 2

⍝ aplcart/tt.tsv:177 — Beta function
Ms←2 ⋄ Ns←3 ⋄ Ms(+÷××⊣!+)Ns   ⍝ 0.08333333333333333

⍝ aplcart/tt.tsv:178 — Beta function
Ms←2 ⋄ Ns←3 ⋄ Ms(+÷××⊣!+)Ns   ⍝ 0.08333333333333333

⍝ aplcart/tt.tsv:179 — Binary representation of J; optional {X} instantiated as dyadic use
J←3 8 15 ⋄ (2∘⊥⍣¯1)J   ⍝ 4 3⍴0 1 1 0 0 1 1 0 1 1 0 1

⍝ aplcart/tt.tsv:180 — Binary representation of J; optional {X} instantiated as dyadic use
J←3 8 15 ⋄ (2∘⊥⍣¯1)J   ⍝ 4 3⍴0 1 1 0 0 1 1 0 1 1 0 1

⍝ aplcart/tt.tsv:181 — Binomial coefficients until Js
Js←4 ⋄ ( !⌝ ⍨0,⍳)Js
5 5⍴1 1 1 1 1 0 1 2 3 4 0 0 1 3 6 0 0 0 1 4 0 0 0 0 1

⍝ aplcart/tt.tsv:182 — Binomial coefficients until Js
Js←4 ⋄ ( !⌝ ⍨0,⍳)Js
5 5⍴1 1 1 1 1 0 1 2 3 4 0 0 1 3 6 0 0 0 1 4 0 0 0 0 1

⍝ aplcart/tt.tsv:183 — Boolean Parity (even number of 1s in vectors)
(≠/1 ⋄ ≠/1 1 ⋄ ≠/1 1 1 ⋄ ≠/1 0 0 1 0 1)   ⍝ 1 0 1 1

⍝ aplcart/tt.tsv:184 — Boolean Parity (even number of 1s in vectors)
B←1 0 1 1 ⋄ ≠/B   ⍝ 1

⍝ aplcart/tt.tsv:185 — Boolean array of shape Jv with ones in locations Iv (inverse of ⍸Bv)
2 4 7 ∊⍨∘⍳ 10   ⍝ 0 1 0 1 0 0 1 0 0 0

⍝ aplcart/tt.tsv:186 — Boolean array of shape Jv with ones in locations Iv (inverse of ⍸Bv)
I←(1 1⋄ 2 3) ⋄ Jv←2 3 ⋄ I∊⍨∘⍳Jv   ⍝ 2 3⍴1 0 0 0 0 1

⍝ aplcart/tt.tsv:187 — Boolean array of shape Jv with zeros in locations Iv
I←(1 1⋄ 2 3) ⋄ Jv←2 3 ⋄ I(~∊⍨∘⍳)Jv   ⍝ 2 3⍴0 1 1 1 1 0

⍝ aplcart/tt.tsv:188 — Boolean array of shape Jv with zeros in locations Iv
I←(1 1⋄ 2 3) ⋄ Jv←2 3 ⋄ I(~∊⍨∘⍳)Jv   ⍝ 2 3⍴0 1 1 1 1 0

⍝ aplcart/tt.tsv:189 — Boolean gaps of lengths Nv after each one
Nv←2 0 3 ⋄ (1/⍨∘,1,∘⍪-)Nv   ⍝ 1 0 0 1 1 0 0 0

⍝ aplcart/tt.tsv:190 — Boolean gaps of lengths Nv after each one
Nv←2 0 3 ⋄ (1/⍨∘,1,∘⍪-)Nv   ⍝ 1 0 0 1 1 0 0 0

⍝ aplcart/tt.tsv:191 — Boolean items in X that are not in Y
X←3 1 2 ⋄ Y←3 1 3 2 ⋄ X(~∊)Y   ⍝ 0 0 0

⍝ aplcart/tt.tsv:192 — Boolean items in X that are not in Y
X←3 1 2 ⋄ Y←3 1 3 2 ⋄ X(~∊)Y   ⍝ 0 0 0

⍝ aplcart/tt.tsv:193 — Boolean matrix indicating saddle points
Nm←3 3⍴3 1 2 4 2 3 5 3 4 ⋄ (⌊/ =⌝ ⌈⌿)Nm   ⍝ 3 3⍴0 0 0 0 0 0 0 1 0

⍝ aplcart/tt.tsv:194 — Boolean matrix indicating saddle points
Nm←3 3⍴3 1 2 4 2 3 5 3 4 ⋄ (⌊/ =⌝ ⌈⌿)Nm   ⍝ 3 3⍴0 0 0 0 0 0 0 1 0

⍝ aplcart/tt.tsv:195 — Boolean one at first occurrence of X in Y; First-true masks use cumulative counts under basedpl left scan
'fab' ({⍵∧1=+\⍵}@(=⍨)⍷) 3 5⍴'abcdef'   ⍝ 3 5⍴0 0 0 0 0 1 0 0 0 0 0 0 0 0 0

⍝ aplcart/tt.tsv:196 — Boolean one at first occurrence of X in Y; First-true masks use cumulative counts under basedpl left scan
X←1 2 ⋄ Y←0 1 2 1 2 ⋄ X({⍵∧1=+\⍵}@(=⍨)⍷)Y
0 1 0 0 0

⍝ aplcart/tt.tsv:197 — Boolean rows of Xm all equal to scalar Ys
Xm←3 3⍴1 1 1 1 0 1 0 0 0 ⋄ Ys←1 ⋄ Xm∧.=Ys
1 0 0

⍝ aplcart/tt.tsv:198 — Boolean rows of Xm all equal to scalar Ys
Xm←3 3⍴1 1 1 1 0 1 0 0 0 ⋄ Ys←1 ⋄ Xm∧.=Ys
1 0 0

⍝ aplcart/tt.tsv:199 — Boolean rows of Ym starting with X; optional {X} instantiated as dyadic use
Xv←1 2 ⋄ Ym←2 3⍴1 2 3 2 1 3 ⋄ Xv(⊣/⍷)Ym   ⍝ 1 0

⍝ aplcart/tt.tsv:200 — Boolean rows of Ym starting with X; optional {X} instantiated as dyadic use
Xv←1 2 ⋄ Ym←2 3⍴1 2 3 2 1 3 ⋄ Xv(⊣/⍷)Ym   ⍝ 1 0

⍝ aplcart/tt.tsv:201 — Bubble sort; First-true masks use cumulative counts under basedpl left scan
⌽@(({⍵∧1=+\⍵}∨1⌽{⍵∧1=+\⍵})0,(>/∘(2∘↕)))⍣≡ 3 1 4 1 5
1 1 3 4 5

⍝ aplcart/tt.tsv:202 — Bubble sort; First-true masks use cumulative counts under basedpl left scan
Nv←3 1 4 1 2 ⋄ ⌽@(1(⌽∨⊢)0{⍵∧1=+\⍵}⍤,(>/∘(2∘↕)))⍣≡Nv
1 1 2 3 4

⍝ aplcart/tt.tsv:203 — Cartesian product: all pairs of X and Y
X←1 2 ⋄ Y←'ab'  ⋄ X ,⌝ Y   ⍝ 2 2⍴(1 'a' ⋄ 1 'b' ⋄ 2 'a' ⋄ 2 'b')

⍝ aplcart/tt.tsv:204 — Cartesian product: all pairs of X and Y
X←1 2 ⋄ Y←'ab'  ⋄ X ,⌝ Y   ⍝ 2 2⍴(1 'a' ⋄ 1 'b' ⋄ 2 'a' ⋄ 2 'b')

⍝ aplcart/tt.tsv:205 — Caseless operation
C←'ABC'  ⋄ f←≡ ⋄ D←'Abc 19 Σς!'  ⋄ C f⍥•C D
0

⍝ aplcart/tt.tsv:206 — Caseless operation
C←'ABC'  ⋄ f←≡ ⋄ D←'Abc 19 Σς!'  ⋄ C f⍥•C D
0

⍝ aplcart/tt.tsv:207 — Catalogue of all pairs from Xv and Yv
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv {⍺⍵}⌝ Yv
3 3⍴(1 4 ⋄ 1 5 ⋄ 1 6 ⋄ 2 4 ⋄ 2 5 ⋄ 2 6 ⋄ 3 4 ⋄ 3 5 ⋄ 3 6)

⍝ aplcart/tt.tsv:208 — Catalogue of all pairs from Xv and Yv
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv {⍺⍵}⌝ Yv
3 3⍴(1 4 ⋄ 1 5 ⋄ 1 6 ⋄ 2 4 ⋄ 2 5 ⋄ 2 6 ⋄ 3 4 ⋄ 3 5 ⋄ 3 6)

⍝ aplcart/tt.tsv:209 — Catalogue of all pairs from ⍳Ms and ⍳Ns
Ms←2 ⋄ Ns←3 ⋄ Ms(⍳,)Ns   ⍝ 2 3⍴(1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3)

⍝ aplcart/tt.tsv:210 — Catalogue of all pairs from ⍳Ms and ⍳Ns
Ms←2 ⋄ Ns←3 ⋄ Ms(⍳,)Ns   ⍝ 2 3⍴(1 1 ⋄ 1 2 ⋄ 1 3 ⋄ 2 1 ⋄ 2 2 ⋄ 2 3)

⍝ aplcart/tt.tsv:211 — Celsius to Fahrenheit
N←¯40 0 100 ⋄ (32+1.8∘×)N   ⍝ ¯40 32 212

⍝ aplcart/tt.tsv:212 — Celsius to Fahrenheit
N←¯40 0 100 ⋄ (32+1.8∘×)N   ⍝ ¯40 32 212

⍝ aplcart/tt.tsv:213 — Centering text line Dv into a field of width Is; Reuse concrete inputs from aplcart/table.tsv:1468; execute this alternate recipe independently
Is←9 ⋄ Dv← 'abc'  ⋄ Is(⊢↑⍨∘-∘⌊2÷⍨+∘≢)Dv   ⍝ '   abc'

⍝ aplcart/tt.tsv:214 — Centering text line Dv into a field of width Is; Reuse concrete inputs from aplcart/table.tsv:1468; execute this alternate recipe independently
Is←9 ⋄ Dv← 'abc'  ⋄ Is(⊢↑⍨∘-∘⌊2÷⍨+∘≢)Dv   ⍝ '   abc'

⍝ aplcart/tt.tsv:215 — Change all 0s in J into Is
10 (⊢+⊣×0=⊢) 3 1 0 2 7 0 0 1 6   ⍝ 3 1 10 2 7 10 10 1 6

⍝ aplcart/tt.tsv:216 — Change all 0s in J into Is
Is←9 ⋄ J←0 1 0 2 ⋄ Is(⊢+⊣×0=⊢)J   ⍝ 9 1 9 2

⍝ aplcart/tt.tsv:217 — Changing connection matrix Jm (¯1 to 1) to a node matrix
Jm←3 3⍴¯1 1 0 0 ¯1 1 1 0 ¯1 ⋄ (⍳∘≢+.×⍨1 ¯1 =⌝ ⍉)Jm
2 3⍴3 1 2 1 2 3

⍝ aplcart/tt.tsv:218 — Changing connection matrix Jm (¯1 to 1) to a node matrix
Jm←3 3⍴¯1 1 0 0 ¯1 1 1 0 ¯1 ⋄ (⍳∘≢+.×⍨1 ¯1 =⌝ ⍉)Jm
2 3⍴3 1 2 1 2 3

⍝ aplcart/tt.tsv:219 — Changing index of an unfound element to zero (slow)
Xv←1 2 3 ⋄ Yv←2 4 1 ⋄ Xv(∊⍨×⍳)Yv   ⍝ 2 0 1

⍝ aplcart/tt.tsv:220 — Changing index of an unfound element to zero (slow)
Xv←1 2 3 ⋄ Yv←2 4 1 ⋄ Xv(∊⍨×⍳)Yv   ⍝ 2 0 1

⍝ aplcart/tt.tsv:221 — Changing lengths Jv of subvectors to ending indicators
Jv←2 3 1 ⋄ (+\∊⍨∘⍳+/)Jv   ⍝ 0 1 0 0 1 1

⍝ aplcart/tt.tsv:222 — Changing lengths Jv of subvectors to ending indicators
Jv←2 3 1 ⋄ (+\∊⍨∘⍳+/)Jv   ⍝ 0 1 0 0 1 1

⍝ aplcart/tt.tsv:223 — Changing lengths Jv of subvectors to starting indicators
Jv←2 3 1 ⋄ (¯1⌽+\∊⍨∘⍳+/)Jv   ⍝ 1 0 1 0 0 1

⍝ aplcart/tt.tsv:224 — Changing lengths Jv of subvectors to starting indicators
Jv←2 3 1 ⋄ (¯1⌽+\∊⍨∘⍳+/)Jv   ⍝ 1 0 1 0 0 1

⍝ aplcart/tt.tsv:225 — Changing node matrix Im (starts,ends) to a connection matrix
Im←3 2⍴1 2 2 3 3 1 ⋄ (-/(⍳⌈/∘,) =⌝ ⍉)Im   ⍝ 3 2⍴1 1 ¯1 1 1 ¯1

⍝ aplcart/tt.tsv:226 — Changing node matrix Im (starts,ends) to a connection matrix
Im←3 2⍴1 2 2 3 3 1 ⋄ (-/(⍳⌈/∘,) =⌝ ⍉)Im   ⍝ 3 2⍴1 1 ¯1 1 1 ¯1

⍝ aplcart/tt.tsv:227 — Changing starting indicators Bv of subvectors to lengths
Bv←1 0 1 0 0 ⋄ ((-/∘⌽∘(2∘↕))∘⍸,∘1)Bv   ⍝ 2 3

⍝ aplcart/tt.tsv:228 — Changing starting indicators Bv of subvectors to lengths
Bv←1 0 1 0 0 ⋄ ((-/∘⌽∘(2∘↕))∘⍸,∘1)Bv   ⍝ 2 3

⍝ aplcart/tt.tsv:231 — Choose the number closer to zero (the left one if tied)
Is←¯2 ⋄ Js←3 ⋄ Is(↑>⍥|⌽,)Js   ⍝ ¯2

⍝ aplcart/tt.tsv:232 — Choose the number closer to zero (the left one if tied)
Is←¯2 ⋄ Js←3 ⋄ Is(↑>⍥|⌽,)Js   ⍝ ¯2

⍝ aplcart/tt.tsv:233 — Choose the number closer to zero (the positive one if tied)
Is←¯2 ⋄ Js←2 ⋄ Is(⌈(↑>⍥|⌽,)⌊)Js   ⍝ 2

⍝ aplcart/tt.tsv:234 — Choose the number closer to zero (the positive one if tied)
Is←¯2 ⋄ Js←2 ⋄ Is(⌈(↑>⍥|⌽,)⌊)Js   ⍝ 2

⍝ aplcart/tt.tsv:235 — Choosing Is random numbers in the range 1 to Js with replacement; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Is←8 ⋄ Js←3 ⋄ r←Is(?⍴)Js ⋄ (Is=≢r)∧∧/(1≤r)∧r≤Js
1

⍝ aplcart/tt.tsv:236 — Choosing Is random numbers in the range 1 to Js with replacement; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Is←8 ⋄ Js←3 ⋄ r←Is(?⍴)Js ⋄ (Is=≢r)∧∧/(1≤r)∧r≤Js
1

⍝ aplcart/tt.tsv:237 — Choosing grading direction (¯1,0,1) dynamically during execution
Is←¯1 ⋄ Y←3 1 3 2 ⋄ Is(⍋×∘⍋∘⍋)Y   ⍝ 3 1 4 2

⍝ aplcart/tt.tsv:238 — Choosing grading direction (¯1,0,1) dynamically during execution
Is←¯1 ⋄ Y←3 1 3 2 ⋄ Is(⍋×∘⍋∘⍋)Y   ⍝ 3 1 4 2

⍝ aplcart/tt.tsv:239 — Circumference of circle with radius N
N←4 9 16 ⋄ (2×π)N
25.13274122871834 56.54866776461628 100.5309649148734

⍝ aplcart/tt.tsv:240 — Circumference of circle with radius N
N←4 9 16 ⋄ (2×π)N
25.13274122871834 56.54866776461628 100.5309649148734

⍝ aplcart/tt.tsv:241 — Circumference of polygon given as complex points
Nv←0 3 3j4 ⋄ (+/∘|⊢-1∘⌽)Nv   ⍝ 12

⍝ aplcart/tt.tsv:242 — Circumference of polygon given as complex points
Nv←0 3 3j4 ⋄ (+/∘|⊢-1∘⌽)Nv   ⍝ 12

⍝ aplcart/tt.tsv:243 — Coefficients of least squares exponential fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←2 6 18 54 ⋄ Mv(*@1⊢∘⍟⌹1,∘⍪⊣)Nv
2 1.09861228866811

⍝ aplcart/tt.tsv:244 — Coefficients of least squares exponential fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←2 6 18 54 ⋄ Mv(*@1⊢∘⍟⌹1,∘⍪⊣)Nv
2 1.09861228866811

⍝ aplcart/tt.tsv:245 — Coefficients of least squares linear fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←1 3 5 7 ⋄ Mv(⊢⌹1,∘⍪⊣)Nv   ⍝ 1 2

⍝ aplcart/tt.tsv:246 — Coefficients of least squares linear fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←1 3 5 7 ⋄ Mv(⊢⌹1,∘⍪⊣)Nv   ⍝ 1 2

⍝ aplcart/tt.tsv:247 — Coefficients of the binomial (approximated, fastest above 10); Reuse concrete inputs from aplcart/table.tsv:1854; execute this alternate recipe independently
Js←5 ⋄ (1,(×\⌽÷⊢)∘⍳)Js   ⍝ 1 5 10 10 5 1

⍝ aplcart/tt.tsv:248 — Coefficients of the binomial (approximated, fastest above 10); Reuse concrete inputs from aplcart/table.tsv:1854; execute this alternate recipe independently
Js←5 ⋄ (1,(×\⌽÷⊢)∘⍳)Js   ⍝ 1 5 10 10 5 1

⍝ aplcart/tt.tsv:249 — Coefficients of the binomial (exact, fastest below 10)
Js←6 ⋄ (⊢!⍨0,⍳)Js   ⍝ 1 6 15 20 15 6 1

⍝ aplcart/tt.tsv:250 — Coefficients of the binomial (exact, fastest below 10)
Js←6 ⋄ (⊢!⍨0,⍳)Js   ⍝ 1 6 15 20 15 6 1

⍝ aplcart/tt.tsv:251 — Column-wise alternating sum: ((N[1]-N[2])+N[3])-N[4]+…
(5 3⍴3×⍳5 ⋄ '' ⋄ -⌿ 5 3⍴3×⍳5)
(5 3⍴3 6 9 12 15 3 6 9 12 15 3 6 9 12 15 ⋄ '' ⋄ ¯9 9 27)

⍝ aplcart/tt.tsv:252 — Column-wise alternating sum: ((N[1]-N[2])+N[3])-N[4]+…
N←4 2⍴⍳8 ⋄ -⌿N   ⍝ ¯4 ¯4

⍝ aplcart/tt.tsv:253 — Column-wise percentage per column
N←2 3⍴1 2 3 4 5 6 ⋄ (100×⊢÷⍤1 +⌿)N
2 3⍴20 28.57142857142857 33.33333333333333 80 71.42857142857143 66.66666666666666

⍝ aplcart/tt.tsv:254 — Column-wise percentage per column
N←2 3⍴1 2 3 4 5 6 ⋄ (100×⊢÷⍤1 +⌿)N
2 3⍴20 28.57142857142857 33.33333333333333 80 71.42857142857143 66.66666666666666

⍝ aplcart/tt.tsv:255 — Common anti-logarithm
(10∘* 1 2 3 4 ⋄ 10∘* ¯1 ¯2 ¯3 ¯4 ⋄ 10∘* 1.32 1.54 1.75 1.99)
(10 100 1000 10000 ⋄ 0.1 0.01 0.001 0.0001 ⋄ 20.8929613085404 34.67368504525317 56.23413251903491 97.72372209558107)

⍝ aplcart/tt.tsv:256 — Common anti-logarithm
N←¯1 0 1 2 ⋄ 10∘*N   ⍝ 0.1 1 10 100

⍝ aplcart/tt.tsv:257 — Common logarithm
(10∘⍟ 1 10 100 1000 10000 1e5 1e6 ⋄ 10∘⍟ 132 15454 17501 199999)
(0 1 2 3 4 5 5.999999999999999 ⋄ 2.12057393120585 4.18904090790901 4.243062864804807 5.301027824186143)

⍝ aplcart/tt.tsv:258 — Common logarithm
N←0.1 1 10 100 ⋄ 10∘⍟N   ⍝ ¯0.9999999999999998 0 1 2

⍝ aplcart/tt.tsv:259 — Comparing vector Yv with rows of array X
m ← ⊃'Finn' 'Gary' 'Gary' 'Anna' 'Carl' 'Dana' 'Carl' 'Gary' 'Gary' 'Beau' ⋄ (m ⋄ m ∧.= 'Gary')
(10 4⍴'FinnGaryGaryAnnaCarlDanaCarlGaryGaryBeau' ⋄ 0 1 1 0 0 0 0 1 1 0)

⍝ aplcart/tt.tsv:260 — Comparing vector Yv with rows of array X
X←3 2⍴1 2 3 4 1 2 ⋄ Yv←1 2 ⋄ X∧.=Yv   ⍝ 1 0 1

⍝ aplcart/tt.tsv:261 — Comparison of successive rows
Ym←3 2⍴1 2 1 2 3 4 ⋄ (∧/(=/[2]∘(2∘↕)))Ym
1 0

⍝ aplcart/tt.tsv:262 — Comparison of successive rows
Ym←3 2⍴1 2 1 2 3 4 ⋄ (∧/(=/[2]∘(2∘↕)))Ym
1 0

⍝ aplcart/tt.tsv:263 — Complementary Angle
N←0 0.25 0.5 ⋄ (¯2○1○⊢)N
1.570796326794897 1.320796326794897 1.070796326794897

⍝ aplcart/tt.tsv:264 — Complementary Angle
N←0 0.25 0.5 ⋄ (¯2○1○⊢)N
1.570796326794897 1.320796326794897 1.070796326794897

⍝ aplcart/tt.tsv:265 — Component of Mv in direction of Nv
Mv←1 2 3 ⋄ Nv←1 1 0 ⋄ Mv(⊢×+.×)∘(⊢÷2*∘÷⍨+.×⍨)Nv
1.5 1.5 0

⍝ aplcart/tt.tsv:266 — Component of Mv in direction of Nv
Mv←1 2 3 ⋄ Nv←1 1 0 ⋄ Mv(⊢×+.×)∘(⊢÷2*∘÷⍨+.×⍨)Nv
1.5 1.5 0

⍝ aplcart/tt.tsv:267 — Compound interest for principals N[1] at rates N[2] in times N[3]
N←(100 200⋄ 0.05 0.1⋄ 1 2 3) ⋄ (↑ ×⌝ 3∘⊃ *⌝ ⍨1+2∘⊃)N
2 2 3⍴105 110.25 115.7625 110 121 133.1000000000001 210 220.5 231.525 220 242 266.2000000000001

⍝ aplcart/tt.tsv:268 — Compound interest for principals N[1] at rates N[2] in times N[3]
N←(100 200⋄ 0.05 0.1⋄ 1 2 3) ⋄ (↑ ×⌝ 3∘⊃ *⌝ ⍨1+2∘⊃)N
2 2 3⍴105 110.25 115.7625 110 121 133.1000000000001 210 220.5 231.525 220 242 266.2000000000001

⍝ aplcart/tt.tsv:269 — Conditional change of elements of N to one according to A
A←1 0 1 ⋄ N←2 3 4 ⋄ A*∘~⍨N   ⍝ 1 3 1

⍝ aplcart/tt.tsv:270 — Conditional change of elements of N to one according to A
A←1 0 1 ⋄ N←2 3 4 ⋄ A*∘~⍨N   ⍝ 1 3 1

⍝ aplcart/tt.tsv:271 — Conditional drop of last element of Y; Reuse concrete inputs from aplcart/table.tsv:663; execute this alternate recipe independently
As←1 ⋄ Y←2 3⍴⍳6 ⋄ As↓⍨∘-⍨Y   ⍝ 1 3⍴1 2 3

⍝ aplcart/tt.tsv:272 — Conditional drop of last element of Y; Reuse concrete inputs from aplcart/table.tsv:663; execute this alternate recipe independently
As←1 ⋄ Y←2 3⍴⍳6 ⋄ As↓⍨∘-⍨Y   ⍝ 1 3⍴1 2 3

⍝ aplcart/tt.tsv:273 — Conditional elementwise change of sign; optional {X} instantiated as dyadic use
A←1 0 1 ⋄ N←4 9 16 ⋄ A(⊢×¯1*⊣)N   ⍝ ¯4 9 ¯16

⍝ aplcart/tt.tsv:274 — Conditional elementwise change of sign; optional {X} instantiated as dyadic use
A←1 0 1 ⋄ N←4 9 16 ⋄ A(⊢×¯1*⊣)N   ⍝ ¯4 9 ¯16

⍝ aplcart/tt.tsv:275 — Conditional in text
Bs←0 ⋄ ('correct',⍨'in'/⍨~)Bs   ⍝ 'incorrect'

⍝ aplcart/tt.tsv:276 — Conditional in text
Bs←0 ⋄ ('correct',⍨'in'/⍨~)Bs   ⍝ 'incorrect'

⍝ aplcart/tt.tsv:277 — Conjugate Transpose
Nm←2 2⍴1j2 3j4 5j6 7j8 ⋄ ⍉∘+Nm   ⍝ 2 2⍴1j¯2 5j¯6 3j¯4 7j¯8

⍝ aplcart/tt.tsv:278 — Conjugate Transpose
Nm←2 2⍴1j2 3j4 5j6 7j8 ⋄ ⍉∘+Nm   ⍝ 2 2⍴1j¯2 5j¯6 3j¯4 7j¯8

⍝ aplcart/tt.tsv:279 — Consecutive ids (indices with equal major cells mapping to same index)
(∪⍳⊢) 2 7 1 8 2 8 1 8   ⍝ 1 2 3 4 1 4 3 4

⍝ aplcart/tt.tsv:280 — Consecutive ids (indices with equal major cells mapping to same index)
Y←3 1 3 2 1 ⋄ (∪⍳⊢)Y   ⍝ 1 2 1 3 2

⍝ aplcart/tt.tsv:281 — Consecutive integers from Is to Js (Is≤Js)
Is←¯2 ⋄ Js←3 ⋄ Is(⊣,⊣+∘⍳-⍨)Js   ⍝ ¯2 ¯1 0 1 2 3

⍝ aplcart/tt.tsv:282 — Consecutive integers from Is to Js (Is≤Js)
Is←¯2 ⋄ Js←3 ⋄ Is(⊣,⊣+∘⍳-⍨)Js   ⍝ ¯2 ¯1 0 1 2 3

⍝ aplcart/tt.tsv:283 — Consecutive integers from Is to Js (descending if Is>Js)
Is←5 ⋄ Js←2 ⋄ Is(⊣,⊣-∘(⍳∘|××)-)Js   ⍝ 5 4 3 2

⍝ aplcart/tt.tsv:284 — Consecutive integers from Is to Js (descending if Is>Js)
Is←5 ⋄ Js←2 ⋄ Is(⊣,⊣-∘(⍳∘|××)-)Js   ⍝ 5 4 3 2

⍝ aplcart/tt.tsv:285 — Continued fraction convergents with terms N; Reduce each prefix to preserve right association
N←2 3 4 ⋄ (+∘÷/)¨,\N   ⍝ 2 2.333333333333333 2.307692307692307

⍝ aplcart/tt.tsv:286 — Continued fraction convergents with terms N; Reduce each prefix to preserve right association
N←2 3 4 ⋄ (+∘÷/)¨,\N   ⍝ 2 2.333333333333333 2.307692307692307

⍝ aplcart/tt.tsv:287 — Continued fraction with terms N
N←2 3 4 ⋄ +∘÷/N   ⍝ 2.307692307692307

⍝ aplcart/tt.tsv:288 — Continued fraction with terms N
N←2 3 4 ⋄ +∘÷/N   ⍝ 2.307692307692307

⍝ aplcart/tt.tsv:289 — Continued fraction: 1+÷2+÷3+÷4+÷5+÷6+÷…Js; optional {X} instantiated as dyadic use
Js←4 ⋄ +∘÷/∘⍳Js   ⍝ 1.433333333333333

⍝ aplcart/tt.tsv:290 — Continued fraction: 1+÷2+÷3+÷4+÷5+÷6+÷…Js; optional {X} instantiated as dyadic use
Js←4 ⋄ +∘÷/∘⍳Js   ⍝ 1.433333333333333

⍝ aplcart/tt.tsv:295 — Conversion of indices Jm of array to indices of ravelled array
Jm←2 3⍴1 1 2 1 2 3 ⋄ (1+⍴⊥¯1∘+)Jm   ⍝ 1 2 6

⍝ aplcart/tt.tsv:296 — Conversion of indices Jm of array to indices of ravelled array
Jm←2 3⍴1 1 2 1 2 3 ⋄ (1+⍴⊥¯1∘+)Jm   ⍝ 1 2 6

⍝ aplcart/tt.tsv:297 — Conversion of set of indices Jv to a mask; Reuse concrete inputs from aplcart/table.tsv:1372; execute this alternate recipe independently
Jv←2 4 5 ⋄ (⊢∊⍨∘⍳⌈/)Jv   ⍝ 0 1 0 1 1

⍝ aplcart/tt.tsv:298 — Conversion of set of indices Jv to a mask; Reuse concrete inputs from aplcart/table.tsv:1372; execute this alternate recipe independently
Jv←2 4 5 ⋄ (⊢∊⍨∘⍳⌈/)Jv   ⍝ 0 1 0 1 1

⍝ aplcart/tt.tsv:317 — Convert binary to reflected Gray code; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (≠\⍣¯1)B   ⍝ 1 0 1 1 1 1 1

⍝ aplcart/tt.tsv:318 — Convert binary to reflected Gray code; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (≠\⍣¯1)B   ⍝ 1 0 1 1 1 1 1

⍝ aplcart/tt.tsv:319 — Convert bits Bv representing a signed integer of As-endianess (0:big, 1:little) into a number
(1 {((¯1*↑∘,)×2⊥↑∘,≠,)⊖⍣⍺⊢(8÷⍨≢⍵)8⍴⍵} 64↑1 1 ⋄ 1 {((¯1*↑∘,)×2⊥↑∘,≠,)⊖⍣⍺⊢(8÷⍨≢⍵)8⍴⍵} ~64↑1 1)
192 ¯192

⍝ aplcart/tt.tsv:320 — Convert bits Bv representing a signed integer of As-endianess (0:big, 1:little) into a number
As←0 ⋄ Bv←1 1 1 1 1 1 0 1 ⋄ As{((¯1*↑∘,)×2⊥↑∘,≠,)⊖⍣⍺⊢(8÷⍨≢⍵)8⍴⍵}Bv
¯2

⍝ aplcart/tt.tsv:321 — Convert character or numeric data into numeric (unsafe); Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:722
Yv←'1 2 3' ⋄ ⍎∘⍕Yv   ⍝ 1 2 3

⍝ aplcart/tt.tsv:322 — Convert character or numeric data into numeric (unsafe); Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:722
Yv←'1 2 3' ⋄ ⍎∘⍕Yv   ⍝ 1 2 3

⍝ aplcart/tt.tsv:323 — Convert decimal degrees/hours to hours,minutes,seconds
(××0 60 60⊤3600×|) 1.25   ⍝ 1 15 0

⍝ aplcart/tt.tsv:324 — Convert decimal degrees/hours to hours,minutes,seconds
Ns←¯1.5 ⋄ (××0 60 60⊤3600×|)Ns   ⍝ ¯1 ¯30 0

⍝ aplcart/tt.tsv:325 — Convert fraction to (numerator,denominator)
Ns←0.75 ⋄ ((,÷∨)∘1)Ns   ⍝ 3 4

⍝ aplcart/tt.tsv:326 — Convert fraction to (numerator,denominator)
Ns←0.75 ⋄ ((,÷∨)∘1)Ns   ⍝ 3 4

⍝ aplcart/tt.tsv:327 — Convert from signed short integers to unsigned short integers
J←¯128 ¯1 0 127 ⋄ 256∘|J   ⍝ 128 255 0 127

⍝ aplcart/tt.tsv:328 — Convert from signed short integers to unsigned short integers
J←¯128 ¯1 0 127 ⋄ 256∘|J   ⍝ 128 255 0 127

⍝ aplcart/tt.tsv:329 — Convert from unsigned short integers to signed short integers; optional {X} instantiated as dyadic use
J←0 127 128 255 ⋄ (¯128+256|128∘+)J   ⍝ 0 127 ¯128 ¯1

⍝ aplcart/tt.tsv:330 — Convert from unsigned short integers to signed short integers; optional {X} instantiated as dyadic use
J←0 127 128 255 ⋄ (¯128+256|128∘+)J   ⍝ 0 127 ¯128 ¯1

⍝ aplcart/tt.tsv:333 — Convert hours,minutes,seconds to decimal degrees/hours
(3600÷⍨60∘⊥) 1 15 0   ⍝ 1.25

⍝ aplcart/tt.tsv:334 — Convert hours,minutes,seconds to decimal degrees/hours
Ns←1 30 0 ⋄ (3600÷⍨60∘⊥)Ns   ⍝ 1.5

⍝ aplcart/tt.tsv:335 — Convert inverted table to table (character data as matrices; keep trailing spaces); dfns display import/wrappers omitted to test underlying arrays
(⍉∘⊃⊂⍤¯1¨) (2 4⍴'Ab  Cdef' ⋄ 1 2 ⋄ 7 3)   ⍝ 2 3⍴('Ab  ') 1 7 ('Cdef') 2 3

⍝ aplcart/tt.tsv:336 — Convert inverted table to table (character data as matrices; keep trailing spaces); Remove stray trailing minus from upstream recipe
Yv←(1 2 3⋄ 3 2⍴'ab cde') ⋄ (⍉∘⊃⊂⍤¯1¨)Yv   ⍝ 3 2⍴1 ('ab') 2 (' c') 3 ('de')

⍝ aplcart/tt.tsv:337 — Convert inverted table to table (character data as matrices; remove trailing spaces); dfns display import/wrappers omitted to test underlying arrays
(⍉∘⊃{(+/∨\' '≠⌽⍵)↑¨↓⍵}¨@(2=≢∘⍴¨)) (2 4⍴'Ab  Cdef' ⋄ 1 2 ⋄ 7 3)
2 3⍴('Ab') 1 7 ('Cdef') 2 3

⍝ aplcart/tt.tsv:338 — Convert inverted table to table (character data as matrices; remove trailing spaces)
Yv←(1 2 3⋄ 3 2⍴'ab cde') ⋄ (⍉∘⊃{(+/∨\' '≠⌽⍵)↑¨↓⍵}¨@(2=≢∘⍴¨))Yv
3 2⍴1 ('ab') 2 (' c') 3 ('de')

⍝ aplcart/tt.tsv:339 — Convert inverted table to table (character data as vectors of vectors); dfns display import/wrappers omitted to test underlying arrays
(⍉⊃) ('Ab' 'Cdef' ⋄ 1 2 ⋄ 7 3)   ⍝ 2 3⍴('Ab') 1 7 ('Cdef') 2 3

⍝ aplcart/tt.tsv:340 — Convert inverted table to table (character data as vectors of vectors)
Yv←(1 2 3⋄ 'a' 'bc' 'd') ⋄ (⍉⊃)Yv   ⍝ 3 2⍴1 'a' 2 ('bc') 3 'd'

⍝ aplcart/tt.tsv:341 — Convert letters to their positions in the alphabet
D←'Abc 19 Σς!'  ⋄ (•A⍳1∘•C)D   ⍝ 1 2 3 27 27 27 27 27 27 27

⍝ aplcart/tt.tsv:342 — Convert letters to their positions in the alphabet
D←'Abc 19 Σς!'  ⋄ (•A⍳1∘•C)D   ⍝ 1 2 3 27 27 27 27 27 27 27

⍝ aplcart/tt.tsv:343 — Convert permutation matrices in B to permutation vectors
(⍳∘1⍤1) ⊃(1 0 0 0 0⋄ 0 0 0 1 0⋄ 0 1 0 0 0⋄ 0 0 0 0 1⋄ 0 0 1 0 0)
1 4 2 5 3

⍝ aplcart/tt.tsv:344 — Convert permutation matrices in B to permutation vectors
B←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (⍳∘1⍤1)B   ⍝ 2 3 1

⍝ aplcart/tt.tsv:345 — Convert permutation vectors in I to permutation matrices
(1↑⍨⍤0-) 1 4 2 5 3
5 5⍴1 0 0 0 0 0 0 0 1 0 0 1 0 0 0 0 0 0 0 1 0 0 1 0 0

⍝ aplcart/tt.tsv:346 — Convert permutation vectors in I to permutation matrices
I←2 3 1 ⋄ (1↑⍨⍤0-)I   ⍝ 3 3⍴0 1 0 0 0 1 1 0 0

⍝ aplcart/tt.tsv:347 — Convert reflected Gray code to binary
B←1 0 1 1 0 0 1 ⋄ ≠\B   ⍝ 1 1 0 1 1 1 0

⍝ aplcart/tt.tsv:348 — Convert reflected Gray code to binary
B←1 0 1 1 0 0 1 ⋄ ≠\B   ⍝ 1 1 0 1 1 1 0

⍝ aplcart/tt.tsv:351 — Convert table to inverted table (character data as matrices); dfns display import/wrappers omitted to test underlying arrays
⊂[1] 2 3⍴'Ab' 1 7 'Cdef' 2 3   ⍝ (('Ab' ⋄ 'Cdef') ⋄ 1 2 ⋄ 7 3)

⍝ aplcart/tt.tsv:352 — Convert table to inverted table (character data as matrices)
Ym←3 2⍴1 'ab' 2 'cd' 3 'ef'  ⋄ (⊃¨⊂[1])Ym
(1 2 3 ⋄ 3 2⍴'abcdef')

⍝ aplcart/tt.tsv:353 — Convert table to inverted table (character data as vectors of vectors); dfns display import/wrappers omitted to test underlying arrays
(⊃¨⊂[1]) 2 3⍴'Ab' 1 7 'Cdef' 2 3   ⍝ (2 4⍴'Ab  Cdef' ⋄ 1 2 ⋄ 7 3)

⍝ aplcart/tt.tsv:354 — Convert table to inverted table (character data as vectors of vectors)
Ym←3 2⍴1 'ab' 2 'cd' 3 'ef'  ⋄ ⊂[1]Ym   ⍝ (1 2 3 ⋄ ('ab' ⋄ 'cd' ⋄ 'ef'))

⍝ aplcart/tt.tsv:357 — Conway's Game of Life: next generation
Bm←5 5⍴0 0 0 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0 ⋄ ({≢⍸⍵}⌺3 3∊¨3+0,¨⊢)Bm
5 5⍴0 0 0 0 0 0 0 0 0 0 0 1 1 1 0 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/tt.tsv:358 — Conway's Game of Life: next generation
Bm←5 5⍴0 0 0 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0 ⋄ ({≢⍸⍵}⌺3 3∊¨3+0,¨⊢)Bm
5 5⍴0 0 0 0 0 0 0 0 0 0 0 1 1 1 0 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/tt.tsv:359 — Conway's Game of Life: next generation given Bv of 140 surviving 3-by-3 subarrays; the Game of Life recipe constructs its table of surviving neighbourhoods
states←(⊂3 3)⍴¨↓⍉(9⍴2)⊤¯1+⍳512 ⋄ Bv←states/⍨{(3=+/∊⍵)∨(1=2 2⊃⍵)∧4=+/∊⍵}¨states ⋄ Bm←5 5⍴0 0 0 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0 ⋄ (Bv∊⍨⊢∘⊂⌺3 3)Bm
5 5⍴0 0 0 0 0 0 0 0 0 0 0 1 1 1 0 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/tt.tsv:360 — Conway's Game of Life: next generation given Bv of 140 surviving 3-by-3 subarrays; the Game of Life recipe constructs its table of surviving neighbourhoods
states←(⊂3 3)⍴¨↓⍉(9⍴2)⊤¯1+⍳512 ⋄ Bv←states/⍨{(3=+/∊⍵)∨(1=2 2⊃⍵)∧4=+/∊⍵}¨states ⋄ Bm←5 5⍴0 0 0 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0 ⋄ (Bv∊⍨⊢∘⊂⌺3 3)Bm
5 5⍴0 0 0 0 0 0 0 0 0 0 0 1 1 1 0 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/tt.tsv:361 — Corner element of a (non-empty) array Y[1;1;1…]
(3 3⍴ (,/∘(2∘↕)) ⍳9 ⋄ '' ⋄ ↑∘,  3 3⍴ (,/∘(2∘↕)) ⍳9)
(3 3⍴(1 2 ⋄ 2 3 ⋄ 3 4 ⋄ 4 5 ⋄ 5 6 ⋄ 6 7 ⋄ 7 8 ⋄ 8 9 ⋄ 1 2) ⋄ '' ⋄ 1 2)

⍝ aplcart/tt.tsv:362 — Corner element of a (non-empty) array Y[1;1;1…]
Y←2 3⍴⍳6 ⋄ ↑∘,Y   ⍝ 1

⍝ aplcart/tt.tsv:363 — Cosecant
N←4 9 16 ⋄ (÷1∘○)N
¯1.321348708810902 2.426486643551989 ¯3.473388259584929

⍝ aplcart/tt.tsv:364 — Cosecant
N←4 9 16 ⋄ (÷1∘○)N
¯1.321348708810902 2.426486643551989 ¯3.473388259584929

⍝ aplcart/tt.tsv:365 — Cosine N
N←0.25 0.5 0.75 ⋄ 2∘○N
0.9689124217106447 0.8775825618903728 0.7316888688738209

⍝ aplcart/tt.tsv:366 — Cosine N
N←0.25 0.5 0.75 ⋄ 2∘○N
0.9689124217106447 0.8775825618903728 0.7316888688738209

⍝ aplcart/tt.tsv:367 — Cotangent
N←4 9 16 ⋄ (÷3∘○)N
0.8636911544506167 ¯2.210845410999195 3.326323195635449

⍝ aplcart/tt.tsv:368 — Cotangent
N←4 9 16 ⋄ (÷3∘○)N
0.8636911544506167 ¯2.210845410999195 3.326323195635449

⍝ aplcart/tt.tsv:369 — Count of leading ones; optional {X} instantiated as dyadic use
Bv←1 1 1 0 0 ⋄ (⊥⍨⌽)Bv   ⍝ 3

⍝ aplcart/tt.tsv:370 — Count of leading ones; optional {X} instantiated as dyadic use
Bv←1 1 1 0 0 ⋄ (⊥⍨⌽)Bv   ⍝ 3

⍝ aplcart/tt.tsv:371 — Count of leading zeros
Bv←0 0 1 1 0 ⋄ (⊥⍨0=⌽)Bv   ⍝ 2

⍝ aplcart/tt.tsv:372 — Count of leading zeros
Bv←0 0 1 1 0 ⋄ (⊥⍨0=⌽)Bv   ⍝ 2

⍝ aplcart/tt.tsv:373 — Count of occurrences of each unique major cell
Y←3 1 3 2 ⋄ ⊢∘≢⌸Y   ⍝ 2 1 1

⍝ aplcart/tt.tsv:374 — Count of occurrences of each unique major cell
Y←3 1 3 2 ⋄ ⊢∘≢⌸Y   ⍝ 2 1 1

⍝ aplcart/tt.tsv:375 — Count of trailing ones
⊥⍨ 1 0 1 0 1 1 1   ⍝ 3

⍝ aplcart/tt.tsv:376 — Count of trailing ones
Bv←1 0 1 1 ⋄ ⊥⍨Bv   ⍝ 2

⍝ aplcart/tt.tsv:377 — Count of trailing zeros; optional {X} instantiated as dyadic use
Bv←1 1 1 0 0 ⋄ (⊥⍨~)Bv   ⍝ 2

⍝ aplcart/tt.tsv:378 — Count of trailing zeros; optional {X} instantiated as dyadic use
Bv←1 1 1 0 0 ⋄ (⊥⍨~)Bv   ⍝ 2

⍝ aplcart/tt.tsv:379 — Count the number of elements in an array; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (×/⍴)Y   ⍝ 4

⍝ aplcart/tt.tsv:380 — Count the number of elements in an array; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (×/⍴)Y   ⍝ 4

⍝ aplcart/tt.tsv:381 — Counting pairwise matches (equal elements) in two vectors
Xv←1 2 3 4 ⋄ Yv←1 3 3 2 ⋄ Xv+.=Yv   ⍝ 2

⍝ aplcart/tt.tsv:382 — Counting pairwise matches (equal elements) in two vectors
Xv←1 2 3 4 ⋄ Yv←1 3 3 2 ⋄ Xv+.=Yv   ⍝ 2

⍝ aplcart/tt.tsv:383 — Create a “grade decending according to” function with a hashed principal argument (fast X⍒Y for subsequent values of Y)
Y← 'abcac'  ⋄ X← 'cba'  ⋄ name←X∘⍒ ⋄ name Y
1 4 2 3 5

⍝ aplcart/tt.tsv:384 — Create a “grade decending according to” function with a hashed principal argument (fast X⍒Y for subsequent values of Y)
Y← 'abcac'  ⋄ X← 'cba'  ⋄ name←X∘⍒ ⋄ name Y
1 4 2 3 5

⍝ aplcart/tt.tsv:385 — Create a “grade-ascending-according-to” function with a hashed principal argument (fast X⍋Y for subsequent values of Y)
Y← 'abcac'  ⋄ X← 'cba'  ⋄ name←X∘⍋ ⋄ name Y
3 5 2 1 4

⍝ aplcart/tt.tsv:386 — Create a “grade-ascending-according-to” function with a hashed principal argument (fast X⍋Y for subsequent values of Y)
Y← 'abcac'  ⋄ X← 'cba'  ⋄ name←X∘⍋ ⋄ name Y
3 5 2 1 4

⍝ aplcart/tt.tsv:387 — Create a “membership-in” function with a hashed principal argument (fast X∊Y for subsequent values of X)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ name←∊∘Y ⋄ name X
1 1 1 1

⍝ aplcart/tt.tsv:388 — Create a “membership-in” function with a hashed principal argument (fast X∊Y for subsequent values of X)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ name←∊∘Y ⋄ name X
1 1 1 1

⍝ aplcart/tt.tsv:389 — Create a “union-with” function with a hashed principal argument (fast X∪Y for subsequent values of Y)
Y←3 1 3 2 ⋄ X←3 1 2 1 ⋄ name←X∘∪ ⋄ name Y
3 1 2 1

⍝ aplcart/tt.tsv:390 — Create a “union-with” function with a hashed principal argument (fast X∪Y for subsequent values of Y)
Y←3 1 3 2 ⋄ X←3 1 2 1 ⋄ name←X∘∪ ⋄ name Y
3 1 2 1

⍝ aplcart/tt.tsv:391 — Create a “without” function with a hashed principal argument (fast X~Y for subsequent values of X)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ name←~∘Y ⋄ name X
⍬

⍝ aplcart/tt.tsv:392 — Create a “without” function with a hashed principal argument (fast X~Y for subsequent values of X)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ name←~∘Y ⋄ name X
⍬

⍝ aplcart/tt.tsv:393 — Create an “index-in” function with a hashed principal argument (fast X⍳Y for subsequent values of Y)
Y←3 1 3 2 ⋄ X←3 1 2 1 ⋄ name←X∘⍳ ⋄ name Y
1 2 1 3

⍝ aplcart/tt.tsv:394 — Create an “index-in” function with a hashed principal argument (fast X⍳Y for subsequent values of Y)
Y←3 1 3 2 ⋄ X←3 1 2 1 ⋄ name←X∘⍳ ⋄ name Y
1 2 1 3

⍝ aplcart/tt.tsv:395 — Create an “intersection-with” function with a hashed principal argument (fast X∩Y for subsequent values of X)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ name←∩∘Y ⋄ name X
3 1 2 1

⍝ aplcart/tt.tsv:396 — Create an “intersection-with” function with a hashed principal argument (fast X∩Y for subsequent values of X)
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ name←∩∘Y ⋄ name X
3 1 2 1

⍝ aplcart/tt.tsv:397 — Cube
N←4 9 16 ⋄ (*∘3)N   ⍝ 64 729 4096

⍝ aplcart/tt.tsv:398 — Cube
N←4 9 16 ⋄ (*∘3)N   ⍝ 64 729 4096

⍝ aplcart/tt.tsv:399 — Cube Root
N←4 9 16 ⋄ (*∘÷∘3)N
1.587401051968199 2.080083823051904 2.519842099789746

⍝ aplcart/tt.tsv:400 — Cube Root
N←4 9 16 ⋄ (*∘÷∘3)N
1.587401051968199 2.080083823051904 2.519842099789746

⍝ aplcart/tt.tsv:401 — Cumulative sum
(+\ 1 1 1 1 1 ⋄ +\ 2 4 6 8 10)   ⍝ (1 2 3 4 5 ⋄ 2 6 12 20 30)

⍝ aplcart/tt.tsv:402 — Cumulative sum
N←4 9 16 ⋄ +\N   ⍝ 4 13 29

⍝ aplcart/tt.tsv:403 — Cumulative sum (+\) in each group of ones
Bv←0 1 1 0 1 1 1 0 ⋄ 0 {⍵×1+⍺}\Bv   ⍝ 0 1 2 0 1 2 3 0

⍝ aplcart/tt.tsv:404 — Cumulative sum (+\) in each group of ones
Bv←0 1 1 0 1 1 1 0 ⋄ 0 {⍵×1+⍺}\Bv   ⍝ 0 1 2 0 1 2 3 0

⍝ aplcart/tt.tsv:405 — Curtail: Remove last major cell; dfns display import/wrappers omitted to test underlying arrays
(¯1∘↓ 1 2 3 4 ⋄ ¯1∘↓ (50 80 ⋄ 10 20 ⋄ 33 66) ⋄ ¯1∘↓ 3 3⍴⍳9)
(1 2 3 ⋄ (50 80 ⋄ 10 20) ⋄ 2 3⍴1 2 3 4 5 6)

⍝ aplcart/tt.tsv:406 — Curtail: Remove last major cell
Y←3 1 3 2 ⋄ ¯1∘↓Y   ⍝ 3 1 3

⍝ aplcart/tt.tsv:407 — Cut Yv into non-empty partitions of length Iv (+/Iv ↔ ⍴Y); Reuse concrete inputs from aplcart/table.tsv:687; execute this alternate recipe independently
Iv←2 0 3 ⋄ Y←⍳5 ⋄ Iv(⍸⍤⊣⊆⊢)Y   ⍝ (1 2 ⋄ 3 4 5)

⍝ aplcart/tt.tsv:408 — Cut Yv into non-empty partitions of length Iv (+/Iv ↔ ⍴Y); Reuse concrete inputs from aplcart/table.tsv:687; execute this alternate recipe independently
Iv←2 0 3 ⋄ Y←⍳5 ⋄ Iv(⍸⍤⊣⊆⊢)Y   ⍝ (1 2 ⋄ 3 4 5)

⍝ aplcart/tt.tsv:409 — Cyclic compression of successive blanks
Dv← '  ab  c  '  ⋄ (⊢⊢⍤/⍨1(⊢∨⌽)' '∘≠)Dv   ⍝ ' ab c'

⍝ aplcart/tt.tsv:410 — Cyclic compression of successive blanks
Dv← '  ab  c  '  ⋄ (⊢⊢⍤/⍨1(⊢∨⌽)' '∘≠)Dv   ⍝ ' ab c'

⍝ aplcart/tt.tsv:411 — Date (⎕TS format) to D.M.YYYY
Jv←2026 9 18 ⋄ ('.'@(' '∘=)∘⍕∘⌽3∘↑)Jv   ⍝ '18.9.2026'

⍝ aplcart/tt.tsv:412 — Date (⎕TS format) to D.M.YYYY
Jv←2026 9 18 ⋄ ('.'@(' '∘=)∘⍕∘⌽3∘↑)Jv   ⍝ '18.9.2026'

⍝ aplcart/tt.tsv:413 — Date (⎕TS format) to M/D/YY
Jv←2026 9 18 ⋄ ('/'@(' '∘=)∘⍕100|1⌽3∘↑)Jv
'9/18/26'

⍝ aplcart/tt.tsv:414 — Date (⎕TS format) to M/D/YY
Jv←2026 9 18 ⋄ ('/'@(' '∘=)∘⍕100|1⌽3∘↑)Jv
'9/18/26'

⍝ aplcart/tt.tsv:415 — Date (⎕TS format) to YYYY-MM-DD
Jv←2026 9 18 ⋄ ('-'@5 8∘⍕1000⊥3∘↑)Jv   ⍝ '2026-09-18'

⍝ aplcart/tt.tsv:416 — Date (⎕TS format) to YYYY-MM-DD
Jv←2026 9 18 ⋄ ('-'@5 8∘⍕1000⊥3∘↑)Jv   ⍝ '2026-09-18'

⍝ aplcart/tt.tsv:421 — Decoding numeric codes J packed with field widths Iv (ZYYYZZZ:1 3 2); Reuse concrete inputs from aplcart/table.tsv:1081; execute this alternate recipe independently
Iv←1 3 2 ⋄ J←123456 987654 ⋄ Iv(⊢⊤⍨10*⊣)J
3 2⍴1 9 234 876 56 54

⍝ aplcart/tt.tsv:422 — Decoding numeric codes J packed with field widths Iv (ZYYYZZZ:1 3 2); Reuse concrete inputs from aplcart/table.tsv:1081; execute this alternate recipe independently
Iv←1 3 2 ⋄ J←123456 987654 ⋄ Iv(⊢⊤⍨10*⊣)J
3 2⍴1 9 234 876 56 54

⍝ aplcart/tt.tsv:423 — Decrement
N←4 9 16 ⋄ (-∘1)N   ⍝ 3 8 15

⍝ aplcart/tt.tsv:424 — Decrement
N←4 9 16 ⋄ (-∘1)N   ⍝ 3 8 15

⍝ aplcart/tt.tsv:425 — Deltas: (N[2]-N[1])(N[3]-N[2])(N[4]-N[3])…; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ -/⌽2↕N   ⍝ 5 7

⍝ aplcart/tt.tsv:426 — Deltas: (N[2]-N[1])(N[3]-N[2])(N[4]-N[3])…; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ -/⌽2↕N   ⍝ 5 7

⍝ aplcart/tt.tsv:427 — Depth of parentheses
Dv← 'a(b(c)d)e'  ⋄ (+\'('∘=-¯1↓0,')'∘=)Dv
0 1 1 2 2 2 1 1 0

⍝ aplcart/tt.tsv:428 — Depth of parentheses
Dv← 'a(b(c)d)e'  ⋄ (+\'('∘=-¯1↓0,')'∘=)Dv
0 1 1 2 2 2 1 1 0

⍝ aplcart/tt.tsv:429 — Derivative of polynomial with descending coefficients Nv
Nv←2 3 4 5 ⋄ ¯1∘(↓×∘⌽∘⍳+∘≢)Nv   ⍝ 6 6 4

⍝ aplcart/tt.tsv:430 — Derivative of polynomial with descending coefficients Nv
Nv←2 3 4 5 ⋄ ¯1∘(↓×∘⌽∘⍳+∘≢)Nv   ⍝ 6 6 4

⍝ aplcart/tt.tsv:431 — Descending cardinal numbers (ranking, all different)
Y←3 1 3 2 ⋄ (⍋⍒)Y   ⍝ 1 4 2 3

⍝ aplcart/tt.tsv:432 — Descending cardinal numbers (ranking, all different)
Y←3 1 3 2 ⋄ (⍋⍒)Y   ⍝ 1 4 2 3

⍝ aplcart/tt.tsv:433 — Descending cardinals numbers (ranking, all different)
Y←3 1 2 1 ⋄ (⊢⍳⍨⌷⍨∘⊂∘⍒⍨)Y   ⍝ 1 3 2 3

⍝ aplcart/tt.tsv:434 — Descending cardinals numbers (ranking, all different)
Y←3 1 2 1 ⋄ (⊢⍳⍨⌷⍨∘⊂∘⍒⍨)Y   ⍝ 1 3 2 3

⍝ aplcart/tt.tsv:435 — Descending shortlex grade
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⍒(≢,⊂)¨)Yv   ⍝ 4 2 1 3

⍝ aplcart/tt.tsv:436 — Descending shortlex grade
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⍒(≢,⊂)¨)Yv   ⍝ 4 2 1 3

⍝ aplcart/tt.tsv:437 — Descending shortlex sort
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⊂⊃¨⍨∘⍒(≢,⊂)¨)Yv
('ab') ('aa') 'b' 'a'

⍝ aplcart/tt.tsv:438 — Descending shortlex sort
Yv← 'b' 'aa' 'a' 'ab'  ⋄ (⊂⊃¨⍨∘⍒(≢,⊂)¨)Yv
('ab') ('aa') 'b' 'a'

⍝ aplcart/tt.tsv:439 — Detect case of characters (1:uppercase, ¯1:lowercase, 0:neither)
D←'Abc 19 Σς!'  ⋄ ((¯1∘•C≠⊢)-1∘•C≠⊢)D   ⍝ 1 ¯1 ¯1 0 0 0 0 1 ¯1 0

⍝ aplcart/tt.tsv:440 — Detect case of characters (1:uppercase, ¯1:lowercase, 0:neither)
D←'Abc 19 Σς!'  ⋄ ((¯1∘•C≠⊢)-1∘•C≠⊢)D   ⍝ 1 ¯1 ¯1 0 0 0 0 1 ¯1 0

⍝ aplcart/tt.tsv:441 — Diagonal matrix of size Jv (n or m,n)
(2∘⍴⍴1,⍴∘0) 3   ⍝ 3 3⍴1 0 0 0 1 0 0 0 1

⍝ aplcart/tt.tsv:442 — Diagonal matrix of size Jv (n or m,n)
Jv←2 3 ⋄ (2∘⍴⍴1,⍴∘0)Jv   ⍝ 2 3⍴1 0 0 0 1 0

⍝ aplcart/tt.tsv:443 — Diagonal matrix with elements of Yv (filled appropriately); Reuse concrete inputs from aplcart/table.tsv:1497; execute this alternate recipe independently
Yv←1 2 3 ⋄ (⌽∘⍳∘≢⌽⊢,(0 ¯1+≢)⍴0∘⍴)Yv   ⍝ 3 3⍴1 0 0 0 2 0 0 0 3

⍝ aplcart/tt.tsv:444 — Diagonal matrix with elements of Yv (filled appropriately); Reuse concrete inputs from aplcart/table.tsv:1497; execute this alternate recipe independently
Yv←1 2 3 ⋄ (⌽∘⍳∘≢⌽⊢,(0 ¯1+≢)⍴0∘⍴)Yv   ⍝ 3 3⍴1 0 0 0 2 0 0 0 3

⍝ aplcart/tt.tsv:445 — Diagonal ravel
Ym←2 3⍴⍳6 ⋄ (,⌷⍨∘⊂∘⍋1⊥⍴⊤¯1+∘⍳×/∘⍴)Ym   ⍝ 1 2 4 3 5 6

⍝ aplcart/tt.tsv:446 — Diagonal ravel
Ym←2 3⍴⍳6 ⋄ (,⌷⍨∘⊂∘⍋1⊥⍴⊤¯1+∘⍳×/∘⍴)Ym   ⍝ 1 2 4 3 5 6

⍝ aplcart/tt.tsv:447 — Difference of adjacent pairs with seed value; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (+\⍣¯1)N   ⍝ 4 5 7

⍝ aplcart/tt.tsv:448 — Difference of adjacent pairs with seed value; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (+\⍣¯1)N   ⍝ 4 5 7

⍝ aplcart/tt.tsv:449 — Differences of successive elements of N along direction Is
Is←2 ⋄ N←2 3⍴1 3 6 2 5 9 ⋄ (-/∘⌽∘(2∘↕)⍤1)N
2 2⍴2 3 3 4

⍝ aplcart/tt.tsv:450 — Differences of successive elements of N along direction Is
Is←2 ⋄ N←2 3⍴1 3 6 2 5 9 ⋄ (-/∘⌽∘(2∘↕)⍤1)N
2 2⍴2 3 3 4

⍝ aplcart/tt.tsv:451 — Digital sum in base Is; Reuse concrete inputs from aplcart/table.tsv:1281; execute this alternate recipe independently; Rename setup bindings to the alternate recipe’s parameter names
Is←10 ⋄ J←12345 ⋄ Is(⊣⊥⊣|(+/⊥⍣¯1))J   ⍝ 5

⍝ aplcart/tt.tsv:452 — Digital sum in base Is; Reuse concrete inputs from aplcart/table.tsv:1281; execute this alternate recipe independently; Rename setup bindings to the alternate recipe’s parameter names
Is←10 ⋄ J←12345 ⋄ Is(⊣⊥⊣|(+/⊥⍣¯1))J   ⍝ 5

⍝ aplcart/tt.tsv:453 — Digits of N; optional {X} instantiated as dyadic use
N←123 450 7 ⋄ (10∘⊥⍣¯1)N   ⍝ 3 3⍴1 4 0 2 5 0 3 0 7

⍝ aplcart/tt.tsv:454 — Digits of N; optional {X} instantiated as dyadic use
N←123 450 7 ⋄ (10∘⊥⍣¯1)N   ⍝ 3 3⍴1 4 0 2 5 0 3 0 7

⍝ aplcart/tt.tsv:455 — Dimensions of major cells
Y←3 1 3 2 ⋄ (1↓⍴)Y   ⍝ ⍬

⍝ aplcart/tt.tsv:456 — Dimensions of major cells
Y←3 1 3 2 ⋄ (1↓⍴)Y   ⍝ ⍬

⍝ aplcart/tt.tsv:457 — Direct matrix product
Mm←3 2⍴⍳6 ⋄ Nm←2 3⍴⍳6 ⋄ Mm(1 3 2 4⍉ ×⌝ )Nm
3 2 2 3⍴1 2 3 2 4 6 4 5 6 8 10 12 3 6 9 4 8 12 12 15 18 16 20 24 5 10 15 6 12 18 20 25 30 24 30 36

⍝ aplcart/tt.tsv:458 — Direct matrix product
Mm←3 2⍴⍳6 ⋄ Nm←2 3⍴⍳6 ⋄ Mm(1 3 2 4⍉ ×⌝ )Nm
3 2 2 3⍴1 2 3 2 4 6 4 5 6 8 10 12 3 6 9 4 8 12 12 15 18 16 20 24 5 10 15 6 12 18 20 25 30 24 30 36

⍝ aplcart/tt.tsv:459 — Distribute major cells of Y into Is (default Is:≢Y) groups as evenly as possible
UnZip ← |∘⍳∘≢⊢∘⊂⌸⊢ ⋄ var ← 'abcdef' ⋄ (1 UnZip var ⋄ 2 UnZip var ⋄ 3 UnZip var ⋄ 4 UnZip var ⋄ 5 UnZip var ⋄ 6 UnZip var ⋄ 7 UnZip var ⋄ 8 UnZip var ⋄ UnZip var)
(1⍴⊂('abcdef') ⋄ ('ace' ⋄ 'bdf') ⋄ ('ad' ⋄ 'be' ⋄ 'cf') ⋄ ('ae' ⋄ 'bf' ⋄ 1⍴'c' ⋄ 1⍴'d') ⋄ ('af' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e') ⋄ (1⍴'a' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e' ⋄ 1⍴'f') ⋄ (1⍴'a' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e' ⋄ 1⍴'f') ⋄ (1⍴'a' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e' ⋄ 1⍴'f') ⋄ (1⍴'a' ⋄ 1⍴'b' ⋄ 1⍴'c' ⋄ 1⍴'d' ⋄ 1⍴'e' ⋄ 1⍴'f'))

⍝ aplcart/tt.tsv:460 — Distribute major cells of Y into Is (default Is:≢Y) groups as evenly as possible
Is←3 ⋄ Y←⍳8 ⋄ Is(|∘⍳∘≢⊢∘⊂⌸⊢)Y   ⍝ (1 4 7 ⋄ 2 5 8 ⋄ 3 6)

⍝ aplcart/tt.tsv:461 — Distribution of Y into intervals with cut-offs X
X←2 4 ⋄ Y←0 1 2 3 4 5 ⋄ X({¯1+≢⍵}⌸⍸,⍨0,∘⍳∘≢⊣)Y
2 2 2

⍝ aplcart/tt.tsv:462 — Distribution of Y into intervals with cut-offs X
X←2 4 ⋄ Y←0 1 2 3 4 5 ⋄ X({¯1+≢⍵}⌸⍸,⍨0,∘⍳∘≢⊣)Y
2 2 2

⍝ aplcart/tt.tsv:463 — Divisibility table
Jv←2 3 6 ⋄ (0=⊢ |⌝ ⍨∘⍳⌈/)Jv   ⍝ 6 3⍴1 1 1 1 0 1 0 1 1 0 0 0 0 0 0 0 0 1

⍝ aplcart/tt.tsv:464 — Divisibility table
Jv←2 3 6 ⋄ (0=⊢ |⌝ ⍨∘⍳⌈/)Jv   ⍝ 6 3⍴1 1 1 1 0 1 0 1 1 0 0 0 0 0 0 0 0 1

⍝ aplcart/tt.tsv:465 — Division: force DOMAIN ERROR for division by 0
M←2 3 ⋄ N←4 6 ⋄ M×∘÷N   ⍝ 0.5 0.5

⍝ aplcart/tt.tsv:466 — Division: force DOMAIN ERROR for division by 0
M←2 3 ⋄ N←4 6 ⋄ M×∘÷N   ⍝ 0.5 0.5

⍝ aplcart/tt.tsv:467 — Do Xv and Yv have any elements in common?; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←3 4 ⋄ Xv(∨/∊)Yv   ⍝ 1

⍝ aplcart/tt.tsv:468 — Do Xv and Yv have any elements in common?; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←3 4 ⋄ Xv(∨/∊)Yv   ⍝ 1

⍝ aplcart/tt.tsv:469 — Do characters in D have no case?
D←'Abc 19 Σς!'  ⋄ (¯1∘•C=1∘•C)D   ⍝ 0 0 0 1 1 1 1 0 0 1

⍝ aplcart/tt.tsv:470 — Do characters in D have no case?
D←'Abc 19 Σς!'  ⋄ (¯1∘•C=1∘•C)D   ⍝ 0 0 0 1 1 1 1 0 0 1

⍝ aplcart/tt.tsv:471 — Do rows of Y contain elements differing from Xs?
Xs←9 ⋄ Y←2 3⍴9 9 9 1 9 1 ⋄ Xs∨.≠⍨Y   ⍝ 0 1

⍝ aplcart/tt.tsv:472 — Do rows of Y contain elements differing from Xs?
Xs←9 ⋄ Y←2 3⍴9 9 9 1 9 1 ⋄ Xs∨.≠⍨Y   ⍝ 0 1

⍝ aplcart/tt.tsv:473 — Does D have any duplicated spaces? (per row)
D←2 4⍴'a  bc d '  ⋄ (∨/'  '∘⍷)D   ⍝ 1 0

⍝ aplcart/tt.tsv:474 — Does D have any duplicated spaces? (per row)
D←2 4⍴'a  bc d '  ⋄ (∨/'  '∘⍷)D   ⍝ 1 0

⍝ aplcart/tt.tsv:475 — Does I divide J?
I←2 3 4 ⋄ J←4 9 16 ⋄ I(0=|)J   ⍝ 1 1 1

⍝ aplcart/tt.tsv:476 — Does I divide J?
I←2 3 4 ⋄ J←4 9 16 ⋄ I(0=|)J   ⍝ 1 1 1

⍝ aplcart/tt.tsv:477 — Does I not divide J?
I←2 3 4 ⋄ J←4 9 16 ⋄ I(0≠|)J   ⍝ 0 0 0

⍝ aplcart/tt.tsv:478 — Does I not divide J?
I←2 3 4 ⋄ J←4 9 16 ⋄ I(0≠|)J   ⍝ 0 0 0

⍝ aplcart/tt.tsv:479 — Does Y have Non-Uniform Depth?
Y←(1 2⋄ 3(4 5)) ⋄ (0>≡)Y   ⍝ 1

⍝ aplcart/tt.tsv:480 — Does Y have Non-Uniform Depth?
Y←(1 2⋄ 3(4 5)) ⋄ (0>≡)Y   ⍝ 1

⍝ aplcart/tt.tsv:481 — Does Y have Uniform Depth?
Y←(1 2⋄ 3(4 5)) ⋄ (0≤≡)Y   ⍝ 0

⍝ aplcart/tt.tsv:482 — Does Y have Uniform Depth?
Y←(1 2⋄ 3(4 5)) ⋄ (0≤≡)Y   ⍝ 0

⍝ aplcart/tt.tsv:483 — Dot/Vector/Cross/Matrix Product of M and N (¯1↑⍴M ↔ 1↑⍴N)
M←2 3⍴⍳6 ⋄ N←3 2⍴⍳6 ⋄ M+.×N   ⍝ 2 2⍴22 28 49 64

⍝ aplcart/tt.tsv:484 — Dot/Vector/Cross/Matrix Product of M and N (¯1↑⍴M ↔ 1↑⍴N)
M←2 3⍴⍳6 ⋄ N←3 2⍴⍳6 ⋄ M+.×N   ⍝ 2 2⍴22 28 49 64

⍝ aplcart/tt.tsv:485 — Double: 2×N
(+⍨ 5 ⋄ +⍨ ⍳10 ⋄ +⍨ ¯1 ¯2.5 0 2.5 4.3j1.1 ⋄ +⍨ (3 3⍴⍳9))
10 (2 4 6 8 10 12 14 16 18 20) (¯2 ¯5 0 5 8.6j2.2) (3 3⍴2 4 6 8 10 12 14 16 18)

⍝ aplcart/tt.tsv:486 — Double: 2×N
N←4 9 16 ⋄ +⍨N   ⍝ 8 18 32

⍝ aplcart/tt.tsv:487 — Doubling quotes for execution
Dv← 'don''t'  ⋄ ''''∘(⊣,⊣,⍨⊢⊢⍤/⍨1+=)Dv   ⍝ '''don''''t'''

⍝ aplcart/tt.tsv:488 — Doubling quotes for execution
Dv← 'don''t'  ⋄ ''''∘(⊣,⊣,⍨⊢⊢⍤/⍨1+=)Dv   ⍝ '''don''''t'''

⍝ aplcart/tt.tsv:489 — Drop Is columns from matrix Ym; Reuse concrete inputs from aplcart/table.tsv:809; execute this alternate recipe independently
Is←2 ⋄ Ym←2 3⍴⍳6 ⋄ Is(⊢↓⍨0,⊣)Ym   ⍝ 2 1⍴3 6

⍝ aplcart/tt.tsv:490 — Drop Is columns from matrix Ym; Reuse concrete inputs from aplcart/table.tsv:809; execute this alternate recipe independently
Is←2 ⋄ Ym←2 3⍴⍳6 ⋄ Is(⊢↓⍨0,⊣)Ym   ⍝ 2 1⍴3 6

⍝ aplcart/tt.tsv:491 — Drop first and last Iv items along leading axes of Y
Iv←1 2 ⋄ Y←4 6⍴⍳24 ⋄ Iv(-⍤⊣↓↓)Y   ⍝ 2 2⍴9 10 15 16

⍝ aplcart/tt.tsv:492 — Drop first and last Iv items along leading axes of Y
Iv←1 2 ⋄ Y←4 6⍴⍳24 ⋄ Iv(-⍤⊣↓↓)Y   ⍝ 2 2⍴9 10 15 16

⍝ aplcart/tt.tsv:493 — Duplicating vector Yv Is times
Is←2 ⋄ Yv←4 5 6 ⋄ Is(×∘⍴⍴⊢)Yv   ⍝ 4 5 6 4 5 6

⍝ aplcart/tt.tsv:494 — Duplicating vector Yv Is times
Is←2 ⋄ Yv←4 5 6 ⋄ Is(×∘⍴⍴⊢)Yv   ⍝ 4 5 6 4 5 6

⍝ aplcart/tt.tsv:495 — Effective rate of interest with nominal rate N for I periods
I←12 ⋄ N←0.05 ⋄ I(⊣*⍨1+÷⍨)N   ⍝ 1.051161897881733

⍝ aplcart/tt.tsv:496 — Effective rate of interest with nominal rate N for I periods
I←12 ⋄ N←0.05 ⋄ I(⊣*⍨1+÷⍨)N   ⍝ 1.051161897881733

⍝ aplcart/tt.tsv:497 — Elements of Jv divisible by Is; optional {X} instantiated as dyadic use
Is←3 ⋄ Jv←1 2 3 4 5 6 ⋄ Is(⊢∩∧)Jv   ⍝ 3 6

⍝ aplcart/tt.tsv:498 — Elements of Jv divisible by Is; optional {X} instantiated as dyadic use
Is←3 ⋄ Jv←1 2 3 4 5 6 ⋄ Is(⊢∩∧)Jv   ⍝ 3 6

⍝ aplcart/tt.tsv:499 — Empty array along first axis
(⍴     4 3⍴⍳12 ⋄ '' ⋄ ⍴ 0∘⌿ 4 3⍴⍳12)   ⍝ (4 3 ⋄ '' ⋄ 0 3)

⍝ aplcart/tt.tsv:500 — Empty array along first axis
Y←2 3⍴⍳6 ⋄ 0∘⌿Y   ⍝ 0 3⍴0

⍝ aplcart/tt.tsv:503 — Enclose columns of a matrix; dfns display import/wrappers omitted to test underlying arrays
(3 3⍴⍳9 ⋄ ⊂[1] 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ (1 4 7 ⋄ 2 5 8 ⋄ 3 6 9))

⍝ aplcart/tt.tsv:504 — Enclose columns of a matrix
Ym←2 3⍴⍳6 ⋄ ⊂[1]Ym   ⍝ (1 4 ⋄ 2 5 ⋄ 3 6)

⍝ aplcart/tt.tsv:505 — Enclose each major cell for any rank Y
Y←3 1 3 2 ⋄ (⊂⍤¯1)Y   ⍝ 3 1 3 2

⍝ aplcart/tt.tsv:506 — Enclose each major cell for any rank Y
Y←3 1 3 2 ⋄ (⊂⍤¯1)Y   ⍝ 3 1 3 2

⍝ aplcart/tt.tsv:507 — Encode a vector of positive integers as equal-width fields of digits in an integer
(10⊥∘,∘⍉10∘⊥⍣¯1) 31 1 27 0   ⍝ 31012700

⍝ aplcart/tt.tsv:508 — Encode a vector of positive integers as equal-width fields of digits in an integer
Jv←12 3 456 ⋄ (10⊥∘,∘⍉10∘⊥⍣¯1)Jv   ⍝ 12003456

⍝ aplcart/tt.tsv:509 — Ending points for Is fields of width Js
Is←2 ⋄ Js←4 ⋄ Is(×⍴1↑⍨∘-⊢)Js   ⍝ 0 0 0 1 0 0 0 1

⍝ aplcart/tt.tsv:510 — Ending points for Is fields of width Js
Is←2 ⋄ Js←4 ⋄ Is(×⍴1↑⍨∘-⊢)Js   ⍝ 0 0 0 1 0 0 0 1

⍝ aplcart/tt.tsv:511 — Ending points of groups of equal elements
Yv←1 1 2 3 3 ⋄ (≠/2↕Yv),1   ⍝ 0 1 1 0 1

⍝ aplcart/tt.tsv:512 — Ending points of groups of equal elements
Yv←1 1 2 3 3 ⋄ (≠/2↕Yv),1   ⍝ 0 1 1 0 1

⍝ aplcart/tt.tsv:513 — Ending points of groups of equal elements (non-empty Yv)
Yv←1 1 2 2 1 ⋄ (1,⍨(≠/∘(2∘↕)))Yv   ⍝ 0 1 0 1 1

⍝ aplcart/tt.tsv:514 — Ending points of groups of equal elements (non-empty Yv)
Yv←1 1 2 2 1 ⋄ (1,⍨(≠/∘(2∘↕)))Yv   ⍝ 0 1 0 1 1

⍝ aplcart/tt.tsv:515 — Enlist (∊Y) but keep leaf simple arrays intact
{,/,¨⊆¨⍵}⍣≡ ('aaa' 'bbb'⋄ ('ccc' 'ccc' 'ccc') 'ddd'⋄ ⊂'eee')
('aaa' ⋄ 'bbb' ⋄ 'ccc' ⋄ 'ccc' ⋄ 'ccc' ⋄ 'ddd' ⋄ 'eee')

⍝ aplcart/tt.tsv:516 — Enlist (∊Y) but keep leaf simple arrays intact
Y←(1 2⋄ (3 4⋄ 5 6)) ⋄ {,/,¨⊆¨⍵}⍣≡Y   ⍝ (1 2 ⋄ 3 4 ⋄ 5 6)

⍝ aplcart/tt.tsv:517 — Ensure minimum rank 1 (reshaping scalar into one-element vector)
(⍴     27 ⋄ ⍴ 1∘/ 27 ⋄ ⍴     27 30 35 ⋄ ⍴ 1∘/ 27 30 35)
(⍬ ⋄ 1⍴1 ⋄ 1⍴3 ⋄ 1⍴3)

⍝ aplcart/tt.tsv:518 — Ensure minimum rank 1 (reshaping scalar into one-element vector)
Y←7 ⋄ 1∘/Y   ⍝ 1⍴7

⍝ aplcart/tt.tsv:519 — Ensure that N is non-negative (negatives become zero); dfns display import/wrappers omitted to test underlying arrays
(0∘⌈ ¯3 ¯2 ¯1 0 1 2 3 ⋄ 0∘⌈ (30 ¯30 ⋄ 100 200 ⋄ ¯5.3 ¯6) ⋄ 0∘⌈ ¯5+3 3⍴⍳9)
(0 0 0 0 1 2 3 ⋄ (30 0 ⋄ 100 200 ⋄ 0 0) ⋄ 3 3⍴0 0 0 0 0 1 2 3 4)

⍝ aplcart/tt.tsv:520 — Ensure that N is non-negative (negatives become zero)
N←¯2 0 3 ⋄ 0∘⌈N   ⍝ 0 0 3

⍝ aplcart/tt.tsv:521 — Ensure that N is non-positive (positives become zero); dfns display import/wrappers omitted to test underlying arrays
(0∘⌊ ¯3 ¯2 ¯1 0 1 2 3 ⋄ 0∘⌊ (30 ¯30 ⋄ 100 200 ⋄ ¯5.3 ¯6) ⋄ 0∘⌊ ¯5+3 3⍴⍳9)
(¯3 ¯2 ¯1 0 0 0 0 ⋄ (0 ¯30 ⋄ 0 0 ⋄ ¯5.3 ¯6) ⋄ 3 3⍴¯4 ¯3 ¯2 ¯1 0 0 0 0 0)

⍝ aplcart/tt.tsv:522 — Ensure that N is non-positive (positives become zero)
N←¯2 0 3 ⋄ 0∘⌊N   ⍝ ¯2 0 0

⍝ aplcart/tt.tsv:523 — Ensure that all elements are vectors; dfns display import/wrappers omitted to test underlying arrays
(⍳5 ⋄ ,¨ ⍳5)
(1 2 3 4 5 ⋄ (1⍴1 ⋄ 1⍴2 ⋄ 1⍴3 ⋄ 1⍴4 ⋄ 1⍴5))

⍝ aplcart/tt.tsv:524 — Ensure that all elements are vectors
Y←1 (2 3) 4 ⋄ ,¨Y   ⍝ (1⍴1 ⋄ 2 3 ⋄ 1⍴4)

⍝ aplcart/tt.tsv:529 — Euclidean distance between two points in N-space
0 0 0 0 (2*∘÷⍨1⊥2*⍨-) 0 3 4 0   ⍝ 5

⍝ aplcart/tt.tsv:530 — Euclidean distance between two points in N-space
Mv←1 2 3 ⋄ Nv←4 6 3 ⋄ Mv(2*∘÷⍨1⊥2*⍨-)Nv   ⍝ 5

⍝ aplcart/tt.tsv:531 — Euclidean distance table for points in N-space (one point per row); nested displayed values saved once in evaluation order and returned together
(r1←4 2⍴0 0 0 1 1 0 1 1 ⋄ (2*∘÷⍨1⊥2*⍨-)⍤1⍤1 99⍨ r1)
(4 2⍴0 0 0 1 1 0 1 1 ⋄ 4 4⍴0 1 1 1.414213562373095 1 0 1.414213562373095 1 1 1.414213562373095 0 1 1.414213562373095 1 1 0)

⍝ aplcart/tt.tsv:532 — Euclidean distance table for points in N-space (one point per row)
Nm←3 2⍴0 0 3 0 3 4 ⋄ (2*∘÷⍨1⊥2*⍨-)⍤1⍤1 99⍨Nm
3 3⍴0 3 5 3 0 4 5 4 0

⍝ aplcart/tt.tsv:535 — Euler's totient function (fastest up to about 1000)
(+/1=⊢∨⍳) 1000   ⍝ 400

⍝ aplcart/tt.tsv:536 — Euler's totient function (fastest up to about 1000)
Js←4 ⋄ (+/1=⊢∨⍳)Js   ⍝ 2

⍝ aplcart/tt.tsv:545 — Evaluate polynomial with descending coefficients Mv for point(s) Nv
3 ¯1 4 ⊥⍨∘⍪ 0 1 2 3   ⍝ 4 6 14 28

⍝ aplcart/tt.tsv:546 — Evaluate polynomial with descending coefficients Mv for point(s) Nv
Mv←2 3 4 ⋄ Nv←1 2 3 ⋄ Mv⊥⍨∘⍪Nv   ⍝ 9 18 31

⍝ aplcart/tt.tsv:547 — Even integers from 0 to 2×Js
((0,2×⍳) 5 ⋄ (0,2×⍳) 6)   ⍝ (0 2 4 6 8 10 ⋄ 0 2 4 6 8 10 12)

⍝ aplcart/tt.tsv:548 — Even integers from 0 to 2×Js
Js←4 ⋄ (0,2×⍳)Js   ⍝ 0 2 4 6 8

⍝ aplcart/tt.tsv:549 — Expand last axis of Y (forces \ to be a function even with a function on its left)
(3 ¯2 4 \ 7 8 ⋄ 1 0 1 0 1 \ 'Hat')   ⍝ (7 7 7 0 0 8 8 8 8 ⋄ 'H a t')

⍝ aplcart/tt.tsv:550 — Expand last axis of Y (forces \ to be a function even with a function on its left)
Iv←1 0 1 0 1 1 ⋄ Y←2 4⍴⍳8 ⋄ Iv⊢⍤\Y   ⍝ 2 6⍴1 0 2 0 3 4 5 0 6 0 7 8

⍝ aplcart/tt.tsv:551 — Expand leading axis of Y (forces ⍀ to be a function even with a function on its left)
mat←3 4⍴⍳12 ⋄ 1 0 2 1 ⍀ mat
5 4⍴1 2 3 4 0 0 0 0 5 6 7 8 5 6 7 8 9 10 11 12

⍝ aplcart/tt.tsv:552 — Expand leading axis of Y (forces ⍀ to be a function even with a function on its left)
Iv←1 0 1 ⋄ Y←2 3⍴⍳6 ⋄ Iv⊢⍤⍀Y   ⍝ 3 3⍴1 2 3 0 0 0 4 5 6

⍝ aplcart/tt.tsv:553 — Expansion mask (left argument for ⍀) for fields of length Jv to uniform field of length |Is
Is←4 ⋄ Jv←2 3 1 ⋄ Is(,(⊣↑1⍴⍨⊢)⍤0)Jv   ⍝ 1 1 0 0 1 1 1 0 1 0 0 0

⍝ aplcart/tt.tsv:554 — Expansion mask (left argument for ⍀) for fields of length Jv to uniform field of length |Is
Is←4 ⋄ Jv←2 3 1 ⋄ Is(,(⊣↑1⍴⍨⊢)⍤0)Jv   ⍝ 1 1 0 0 1 1 1 0 1 0 0 0

⍝ aplcart/tt.tsv:555 — Expansion vector (left argument for \ or ⍀) to insert Jv[i] elements after i'th element
Jv←2 0 1 ⋄ ((¯1⌽+\∊⍨∘⍳+/)1∘+)Jv   ⍝ 1 0 0 1 1 0

⍝ aplcart/tt.tsv:556 — Expansion vector (left argument for \ or ⍀) to insert Jv[i] elements after i'th element
Jv←2 0 1 ⋄ ((¯1⌽+\∊⍨∘⍳+/)1∘+)Jv   ⍝ 1 0 0 1 1 0

⍝ aplcart/tt.tsv:557 — Expansion vector (left argument for \ or ⍀) to insert Jv[i] elements before i'th element
Jv←2 0 1 ⋄ ((+\∊⍨∘⍳+/)1∘+)Jv   ⍝ 0 0 1 1 0 1

⍝ aplcart/tt.tsv:558 — Expansion vector (left argument for \ or ⍀) to insert Jv[i] elements before i'th element
Jv←2 0 1 ⋄ ((+\∊⍨∘⍳+/)1∘+)Jv   ⍝ 0 0 1 1 0 1

⍝ aplcart/tt.tsv:559 — Expansion vector (left argument for \ or ⍀) to insert a new element after each one in Bv; Reuse concrete inputs from aplcart/table.tsv:1469; execute this alternate recipe independently
Bv←1 0 1 0 ⋄ ((1,⍨⍪)⊢⍤/⍥,⍪,~)Bv   ⍝ 1 0 1 1 0 1

⍝ aplcart/tt.tsv:560 — Expansion vector (left argument for \ or ⍀) to insert a new element after each one in Bv; Reuse concrete inputs from aplcart/table.tsv:1469; execute this alternate recipe independently
Bv←1 0 1 0 ⋄ ((1,⍨⍪)⊢⍤/⍥,⍪,~)Bv   ⍝ 1 0 1 1 0 1

⍝ aplcart/tt.tsv:561 — Expansion vector for Y with zeros after indices Iv; Reuse concrete inputs from aplcart/table.tsv:1470; execute this alternate recipe independently
Iv←1 3 ⋄ Y←1 2 3 4 ⋄ Iv(⊢∘≢≥∘⍋⊢∘⍳∘≢,⊣)Y   ⍝ 1 0 1 1 0 1

⍝ aplcart/tt.tsv:562 — Expansion vector for Y with zeros after indices Iv; Reuse concrete inputs from aplcart/table.tsv:1470; execute this alternate recipe independently
Iv←1 3 ⋄ Y←1 2 3 4 ⋄ Iv(⊢∘≢≥∘⍋⊢∘⍳∘≢,⊣)Y   ⍝ 1 0 1 1 0 1

⍝ aplcart/tt.tsv:563 — Extending a distance table to next leg
Mm←3 3⍴0 2 5 2 0 1 5 1 0 ⋄ Nm←Mm ⋄ Mm⌊.+Nm
3 3⍴0 2 3 2 0 1 3 1 0

⍝ aplcart/tt.tsv:564 — Extending a distance table to next leg
Mm←3 3⍴0 2 5 2 0 1 5 1 0 ⋄ Nm←Mm ⋄ Mm⌊.+Nm
3 3⍴0 2 3 2 0 1 3 1 0

⍝ aplcart/tt.tsv:565 — Extending a transitive binary relation
Am←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ Bm←Am ⋄ Am∨.∧Bm
3 3⍴0 0 1 1 0 0 0 1 0

⍝ aplcart/tt.tsv:566 — Extending a transitive binary relation
Am←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ Bm←Am ⋄ Am∨.∧Bm
3 3⍴0 0 1 1 0 0 0 1 0

⍝ aplcart/tt.tsv:567 — Extract text (without quotes) in expression; Reuse concrete inputs from aplcart/table.tsv:1495; execute this alternate recipe independently
Dv← 'a+''hello''+''world''+b'  ⋄ (⊢⊆⍨∘(~∧≠\)=∘'''')Dv
('hello' ⋄ 'world')

⍝ aplcart/tt.tsv:568 — Extract text (without quotes) in expression; Reuse concrete inputs from aplcart/table.tsv:1495; execute this alternate recipe independently
Dv← 'a+''hello''+''world''+b'  ⋄ (⊢⊆⍨∘(~∧≠\)=∘'''')Dv
('hello' ⋄ 'world')

⍝ aplcart/tt.tsv:569 — Extract the lower triangular part of the matrix Mm with main diagonal
(⊢× ≥⌝ ⍨∘⍳∘≢) 5 5⍴⍳25
5 5⍴1 0 0 0 0 6 7 0 0 0 11 12 13 0 0 16 17 18 19 0 21 22 23 24 25

⍝ aplcart/tt.tsv:570 — Extract the lower triangular part of the matrix Mm with main diagonal; Reuse concrete inputs from aplcart/table.tsv:1296; execute this alternate recipe independently
Mm←3 3⍴⍳9 ⋄ (⊢× ≥⌝ ⍨∘⍳∘≢)Mm   ⍝ 3 3⍴1 0 0 4 5 0 7 8 9

⍝ aplcart/tt.tsv:571 — Extract the lower triangular part of the matrix Mm without main diagonal
(⊢× >⌝ ⍨∘⍳∘≢) 5 5⍴⍳25
5 5⍴0 0 0 0 0 6 0 0 0 0 11 12 0 0 0 16 17 18 0 0 21 22 23 24 0

⍝ aplcart/tt.tsv:572 — Extract the lower triangular part of the matrix Mm without main diagonal; Reuse concrete inputs from aplcart/table.tsv:1297; execute this alternate recipe independently
Mm←3 3⍴⍳9 ⋄ (⊢× >⌝ ⍨∘⍳∘≢)Mm   ⍝ 3 3⍴0 0 0 4 0 0 7 8 0

⍝ aplcart/tt.tsv:573 — Extract the upper triangular part of the matrix Mm with main diagonal
(⊢× ≤⌝ ⍨∘⍳∘≢) 5 5⍴⍳25
5 5⍴1 2 3 4 5 0 7 8 9 10 0 0 13 14 15 0 0 0 19 20 0 0 0 0 25

⍝ aplcart/tt.tsv:574 — Extract the upper triangular part of the matrix Mm with main diagonal; Reuse concrete inputs from aplcart/table.tsv:1295; execute this alternate recipe independently
Mm←3 3⍴⍳9 ⋄ (⊢× ≤⌝ ⍨∘⍳∘≢)Mm   ⍝ 3 3⍴1 2 3 0 5 6 0 0 9

⍝ aplcart/tt.tsv:575 — Extract the upper triangular part of the matrix Mm without main diagonal
(⊢× <⌝ ⍨∘⍳∘≢) 5 5⍴⍳25
5 5⍴0 2 3 4 5 0 0 8 9 10 0 0 0 14 15 0 0 0 0 20 0 0 0 0 0

⍝ aplcart/tt.tsv:576 — Extract the upper triangular part of the matrix Mm without main diagonal; Reuse concrete inputs from aplcart/table.tsv:1294; execute this alternate recipe independently
Mm←3 3⍴⍳9 ⋄ (⊢× <⌝ ⍨∘⍳∘≢)Mm   ⍝ 3 3⍴0 2 3 0 0 6 0 0 0

⍝ aplcart/tt.tsv:577 — Fahrenheit to Celsius
N←¯40 32 212 ⋄ (1.8÷⍨¯32∘+)N   ⍝ ¯40 0 100

⍝ aplcart/tt.tsv:578 — Fahrenheit to Celsius
N←¯40 32 212 ⋄ (1.8÷⍨¯32∘+)N   ⍝ ¯40 0 100

⍝ aplcart/tt.tsv:579 — Fast: 0 corresponding to each item of Y
Y←(1 2⋄ 3 4 5) ⋄ {0}¨Y   ⍝ 0 0

⍝ aplcart/tt.tsv:580 — Fast: 0 corresponding to each item of Y
Y←(1 2⋄ 3 4 5) ⋄ {0}¨Y   ⍝ 0 0

⍝ aplcart/tt.tsv:581 — Fast: 0 irrespective of Y
{0} (4 5 6) 'ABC'   ⍝ 0

⍝ aplcart/tt.tsv:582 — Fast: 0 irrespective of Y
Y←3 1 3 2 ⋄ {0}Y   ⍝ 0

⍝ aplcart/tt.tsv:583 — Catenate items of Yv along their first axes; dfns display import/wrappers omitted to test underlying arrays
((9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ ⍪/ (9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ (3 3⍴⍳9 ⋄ 3 3⍴⍳9) ⋄ ⍪/ (3 3⍴⍳9 ⋄ 3 3⍴⍳9))
((9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ 9 10 20 40 60 4 5 ⋄ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴1 2 3 4 5 6 7 8 9) ⋄ 6 3⍴1 2 3 4 5 6 7 8 9 1 2 3 4 5 6 7 8 9)

⍝ aplcart/tt.tsv:584 — Catenate items of Yv along their first axes
Yv←(2 2⍴⍳4⋄ 1 2⍴5 6) ⋄ ⍪/Yv   ⍝ (3 2⍴1 2 3 4 5 6)

⍝ aplcart/tt.tsv:585 — Catenate items of Yv along their last axes; dfns display import/wrappers omitted to test underlying arrays
((9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ ,/ (9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ (3 3⍴⍳9 ⋄ 3 3⍴⍳9) ⋄ ,/ (3 3⍴⍳9 ⋄ 3 3⍴⍳9))
((9 10 ⋄ 20 40 60 ⋄ 4 5) ⋄ 9 10 20 40 60 4 5 ⋄ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴1 2 3 4 5 6 7 8 9) ⋄ 3 6⍴1 2 3 1 2 3 4 5 6 4 5 6 7 8 9 7 8 9)

⍝ aplcart/tt.tsv:586 — Catenate items of Yv along their last axes
Yv←(2 2⍴⍳4⋄ 2 1⍴5 6) ⋄ ,/Yv   ⍝ (2 3⍴1 2 5 3 4 6)

⍝ aplcart/tt.tsv:587 — Fast: The first sub-array along the first axis of Y
(3 3⍴⍳9 ⋄ '' ⋄ ⊣⌿ 3 3⍴⍳9)   ⍝ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 1 2 3)

⍝ aplcart/tt.tsv:588 — Fast: The first sub-array along the first axis of Y
Y←2 3⍴⍳6 ⋄ ⊣⌿Y   ⍝ 1 2 3

⍝ aplcart/tt.tsv:589 — Fast: The first sub-array along the last axis of Y
(⊣/ 1 2 3 4 5 ⋄ '' ⋄ 3 3⍴⍳9 ⋄ '' ⋄ ⊣/ 3 3⍴⍳9)
1 ('') (3 3⍴1 2 3 4 5 6 7 8 9) ('') (1 4 7)

⍝ aplcart/tt.tsv:590 — Fast: The first sub-array along the last axis of Y
Y←2 3⍴⍳6 ⋄ ⊣/Y   ⍝ 1 4

⍝ aplcart/tt.tsv:591 — Fast: The last sub-array along the first axis of Y
(3 3⍴⍳9 ⋄ '' ⋄ ⊢⌿ 3 3⍴⍳9)   ⍝ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 7 8 9)

⍝ aplcart/tt.tsv:592 — Fast: The last sub-array along the first axis of Y
Y←2 3⍴⍳6 ⋄ ⊢⌿Y   ⍝ 4 5 6

⍝ aplcart/tt.tsv:593 — Fast: The last sub-array along the last axis of Y
(⊢/ 1 2 3 4 5 ⋄ '' ⋄ 3 3⍴⍳9 ⋄ '' ⋄ ⊢/ 3 3⍴⍳9)
5 ('') (3 3⍴1 2 3 4 5 6 7 8 9) ('') (3 6 9)

⍝ aplcart/tt.tsv:594 — Fast: The last sub-array along the last axis of Y
Y←2 3⍴⍳6 ⋄ ⊢/Y   ⍝ 3 6

⍝ aplcart/tt.tsv:595 — Fast: The length of the first axis of each item in X
Y←(1 2⋄ 3 4 5⋄ 2 2⍴⍳4) ⋄ ↑∘⍴¨Y   ⍝ 2 3 2

⍝ aplcart/tt.tsv:596 — Fast: The length of the first axis of each item in X
Y←(1 2⋄ 3 4 5⋄ 2 2⍴⍳4) ⋄ ↑∘⍴¨Y   ⍝ 2 3 2

⍝ aplcart/tt.tsv:597 — Fill array Y with Xs; Reuse concrete inputs from aplcart/table.tsv:929; execute this alternate recipe independently
Xs←9 ⋄ Y←2 3⍴⍳6 ⋄ Xs@(1⍴⍨⍴)Y   ⍝ 2 3⍴9 9 9 9 9 9

⍝ aplcart/tt.tsv:598 — Fill array Y with Xs; Reuse concrete inputs from aplcart/table.tsv:929; execute this alternate recipe independently
Xs←9 ⋄ Y←2 3⍴⍳6 ⋄ Xs@(1⍴⍨⍴)Y   ⍝ 2 3⍴9 9 9 9 9 9

⍝ aplcart/tt.tsv:599 — Fill element (converts characters to spaces, numbers to zeros); optional {X} instantiated as dyadic use
Y←('ab'⋄ 1 2)  ⋄ (↑0∘⍴)Y   ⍝ '  '

⍝ aplcart/tt.tsv:600 — Fill element (converts characters to spaces, numbers to zeros); optional {X} instantiated as dyadic use
Y←('ab'⋄ 1 2)  ⋄ (↑0∘⍴)Y   ⍝ '  '

⍝ aplcart/tt.tsv:601 — Filtering columns of Y according to mask Av (forces / to be a function even with a function on its left)
1 0 1 0 1 / 'Heart'   ⍝ 'Hat'

⍝ aplcart/tt.tsv:602 — Filtering columns of Y according to mask Av (forces / to be a function even with a function on its left)
Av←1 0 1 ⋄ Y←2 3⍴⍳6 ⋄ Av⊢⍤/Y   ⍝ 2 2⍴1 3 4 6

⍝ aplcart/tt.tsv:603 — Filtering major cells of Y according to mask Av (forces ⌿ to be a function even with a function on its left)
mat ← 3 4⍴⍳12 ⋄ 1 0 1 ⌿ mat   ⍝ 2 4⍴1 2 3 4 9 10 11 12

⍝ aplcart/tt.tsv:604 — Filtering major cells of Y according to mask Av (forces ⌿ to be a function even with a function on its left)
Av←1 0 ⋄ Y←2 3⍴⍳6 ⋄ Av⊢⍤⌿Y   ⍝ 1 3⍴1 2 3

⍝ aplcart/tt.tsv:607 — First Js figurate numbers
Js←4 ⋄ (+\+\∘⍳)Js   ⍝ 1 4 10 20

⍝ aplcart/tt.tsv:608 — First Js figurate numbers
Js←4 ⋄ (+\+\∘⍳)Js   ⍝ 1 4 10 20

⍝ aplcart/tt.tsv:609 — First Js triangular numbers; optional {X} instantiated as dyadic use
Js←4 ⋄ (+\⍳)Js   ⍝ 1 3 6 10

⍝ aplcart/tt.tsv:610 — First Js triangular numbers; optional {X} instantiated as dyadic use
Js←4 ⋄ (+\⍳)Js   ⍝ 1 3 6 10

⍝ aplcart/tt.tsv:611 — First Js triangular pyramidal numbers
Js←4 ⋄ +\⍣2∘⍳Js   ⍝ 1 4 10 20

⍝ aplcart/tt.tsv:612 — First Js triangular pyramidal numbers
Js←4 ⋄ +\⍣2∘⍳Js   ⍝ 1 4 10 20

⍝ aplcart/tt.tsv:613 — First column as a column matrix (column vector); optional {X} instantiated as dyadic use
Ym←2 3⍴⍳6 ⋄ (1∘↑⍤1)Ym   ⍝ 2 1⍴1 4

⍝ aplcart/tt.tsv:614 — First column as a column matrix (column vector); optional {X} instantiated as dyadic use
Ym←2 3⍴⍳6 ⋄ (1∘↑⍤1)Ym   ⍝ 2 1⍴1 4

⍝ aplcart/tt.tsv:615 — First element (as vector) and remaining elements
Y←1 2 3 4 5 ⋄ 1 1∘⊂Y   ⍝ (1⍴1 ⋄ 2 3 4 5)

⍝ aplcart/tt.tsv:616 — First element (as vector) and remaining elements
Y←1 2 3 4 5 ⋄ 1 1∘⊂Y   ⍝ (1⍴1 ⋄ 2 3 4 5)

⍝ aplcart/tt.tsv:617 — First element of Y as a scalar; dfns display import/wrappers omitted to test underlying arrays
(⍬∘⍴ 1 2 3 4 ⋄ ⍬∘⍴ (50 80 ⋄ 10 20 ⋄ 33 66) ⋄ ⍬∘⍴ 3 3⍴⍳9)
⊂¨1 (50 80) 1

⍝ aplcart/tt.tsv:618 — First element of Y as a scalar
Y←3 1 3 2 ⋄ ⍬∘⍴Y   ⍝ ⊂3

⍝ aplcart/tt.tsv:619 — First group of ones; Reuse concrete inputs from aplcart/table.tsv:1326; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ (⊢∧(∧⍀∨⍀=⊢))B   ⍝ 1 1 0 0 0 0 0

⍝ aplcart/tt.tsv:620 — First group of ones; Reuse concrete inputs from aplcart/table.tsv:1326; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ (⊢∧(∧⍀∨⍀=⊢))B   ⍝ 1 1 0 0 0 0 0

⍝ aplcart/tt.tsv:621 — First indices in X of major cells Y, 0 if not found
X←1 2 3 ⋄ Y←2 4 ⋄ X(⍳|⍨1+∘≢⊣)Y   ⍝ 2 0

⍝ aplcart/tt.tsv:622 — First indices in X of major cells Y, 0 if not found
X←1 2 3 ⋄ Y←2 4 ⋄ X(⍳|⍨1+∘≢⊣)Y   ⍝ 2 0

⍝ aplcart/tt.tsv:623 — First number with largest magnitude
(↑∘⍒∘|⊃⊢) 3 ¯4 2   ⍝ ¯4

⍝ aplcart/tt.tsv:624 — First number with largest magnitude; Reuse concrete inputs from aplcart/table.tsv:1095; execute this alternate recipe independently
Nv←¯5 2 ¯2 4 ⋄ (↑∘⍒∘|⊃⊢)Nv   ⍝ ¯5

⍝ aplcart/tt.tsv:625 — First number with smallest magnitude
(↑∘⍋∘|⊃⊢) 3 ¯4 2   ⍝ 2

⍝ aplcart/tt.tsv:626 — First number with smallest magnitude; Reuse concrete inputs from aplcart/table.tsv:1092; execute this alternate recipe independently
Nv←¯5 2 ¯2 4 ⋄ (↑∘⍋∘|⊃⊢)Nv   ⍝ 2

⍝ aplcart/tt.tsv:627 — First occurrence of string Cv in string Dv; optional {X} instantiated as dyadic use
Cv← 'ab'  ⋄ Dv← 'xxabyyab'  ⋄ Cv(1⍳⍨⍷)Dv
3

⍝ aplcart/tt.tsv:628 — First occurrence of string Cv in string Dv; optional {X} instantiated as dyadic use
Cv← 'ab'  ⋄ Dv← 'xxabyyab'  ⋄ Cv(1⍳⍨⍷)Dv
3

⍝ aplcart/tt.tsv:629 — First one (<\) in each subvector of Bv indicated by Av (fast ∊<\¨Av⊂Bv)
Av←1 0 0 1 0 0 ⋄ Bv←0 1 1 1 0 1 ⋄ Av(∧∨⊢{⍵\(</∘(2∘↕))0,⍵/⍺}∨)Bv
0 1 0 1 0 0

⍝ aplcart/tt.tsv:630 — First one (<\) in each subvector of Bv indicated by Av (fast ∊<\¨Av⊂Bv)
Av←1 0 0 1 0 0 ⋄ Bv←0 1 1 1 0 1 ⋄ Av(∧∨⊢{⍵\(</∘(2∘↕))0,⍵/⍺}∨)Bv
0 1 0 1 0 0

⍝ aplcart/tt.tsv:631 — First ones in each group of ones
B←1 1 0 1 0 1 0 ⋄ ((</[2]∘(2∘↕))0∘⍪)B   ⍝ 1 0 0 1 0 1 0

⍝ aplcart/tt.tsv:632 — First ones in each group of ones
B←1 1 0 1 0 1 0 ⋄ ((</[2]∘(2∘↕))0∘⍪)B   ⍝ 1 0 0 1 0 1 0

⍝ aplcart/tt.tsv:633 — First row as a row matrix (row vector)
Ym←2 3⍴⍳6 ⋄ (1∘↑)Ym   ⍝ 1 3⍴1 2 3

⍝ aplcart/tt.tsv:634 — First row as a row matrix (row vector)
Ym←2 3⍴⍳6 ⋄ (1∘↑)Ym   ⍝ 1 3⍴1 2 3

⍝ aplcart/tt.tsv:635 — First word in Dv
Dv← 'hello world'  ⋄ (⊢↑⍨¯1+⍳∘' ')Dv   ⍝ 'hello'

⍝ aplcart/tt.tsv:636 — First word in Dv
Dv← 'hello world'  ⋄ (⊢↑⍨¯1+⍳∘' ')Dv   ⍝ 'hello'

⍝ aplcart/tt.tsv:637 — Floored division
¯10 10 ¯10 10 (⌊÷) ¯3 ¯3 3 3   ⍝ 3 ¯4 ¯4 3

⍝ aplcart/tt.tsv:638 — Floored division
M←¯7 7 8 ⋄ N←3 3 3 ⋄ M(⌊÷)N   ⍝ ¯3 2 2

⍝ aplcart/tt.tsv:639 — Force numbers N to range (-M)≤N≤M
3 (⌊∘-⍣2) 0 1 ¯2 4 ¯5   ⍝ 0 1 ¯2 3 ¯3

⍝ aplcart/tt.tsv:640 — Force numbers N to range (-M)≤N≤M
M←2 ⋄ N←¯4 ¯1 0 1 4 ⋄ M(⌊∘-⍣2)N   ⍝ ¯2 ¯1 0 1 2

⍝ aplcart/tt.tsv:641 — Force numbers N to range 0≤N≤M
M←2 3 4 ⋄ N←4 9 16 ⋄ M(0⌈⌊)N   ⍝ 2 3 4

⍝ aplcart/tt.tsv:642 — Force numbers N to range 0≤N≤M
M←2 3 4 ⋄ N←4 9 16 ⋄ M(0⌈⌊)N   ⍝ 2 3 4

⍝ aplcart/tt.tsv:643 — Formatting N with Jv decimals in fields of width Iv; Swap outer arguments so the width/precision specification, not the data matrix, goes through (∊,⍤0/); original recipe errors in Dyalog
Iv←7 8 ⋄ Jv←1 2 ⋄ N←2 2⍴1.25 2.375 3.25 4.625 ⋄ N⍕⍨∘(∊,⍤0/)Iv Jv
2 15⍴'    1.3    2.38    3.3    4.63'

⍝ aplcart/tt.tsv:644 — Formatting N with Jv decimals in fields of width Iv; Swap outer arguments so the width/precision specification, not the data matrix, goes through (∊,⍤0/); original recipe errors in Dyalog
Iv←7 8 ⋄ Jv←1 2 ⋄ N←2 2⍴1.25 2.375 3.25 4.625 ⋄ N⍕⍨∘(∊,⍤0/)Iv Jv
2 15⍴'    1.3    2.38    3.3    4.63'

⍝ aplcart/tt.tsv:645 — Formatting with zero values replaced with blanks
N←2 3⍴0 1 2 3 0 4 ⋄ (⍕' '@(0∘=))N   ⍝ 2 5⍴'  1 23   4'

⍝ aplcart/tt.tsv:646 — Formatting with zero values replaced with blanks
N←2 3⍴0 1 2 3 0 4 ⋄ (⍕' '@(0∘=))N   ⍝ 2 5⍴'  1 23   4'

⍝ aplcart/tt.tsv:647 — Forming an Is-row matrix with all rows being Yv
Is←2 ⋄ Yv←4 5 6 ⋄ Is(,∘≢⍴⊢)Yv   ⍝ 2 3⍴4 5 6 4 5 6

⍝ aplcart/tt.tsv:648 — Forming an Is-row matrix with all rows being Yv
Is←2 ⋄ Yv←4 5 6 ⋄ Is(,∘≢⍴⊢)Yv   ⍝ 2 3⍴4 5 6 4 5 6

⍝ aplcart/tt.tsv:649 — Forming first row of a matrix for later expansion
(⍴     1 2 3 ⋄ ⍴ ⍉∘⍪ 1 2 3)   ⍝ (1⍴3 ⋄ 1 3)

⍝ aplcart/tt.tsv:650 — Forming first row of a matrix for later expansion
Yv←1 2 3 4 ⋄ ⍉∘⍪Yv   ⍝ 1 4⍴1 2 3 4

⍝ aplcart/tt.tsv:651 — Fractional part of number; dfns display import/wrappers omitted to test underlying arrays
(1∘| 0.55 1.23 8.76 0 ⋄ 1∘| (3.3 ¯3.3 ⋄ 100.2 200.1 ⋄ ¯5.3 ¯6) ⋄ 1∘| 1.1×3 3⍴⍳9)
(0.55 0.23 0.7599999999999998 0 ⋄ (0.2999999999999998 0.7000000000000002 ⋄ 0.2000000000000028 0.09999999999999432 ⋄ 0.7000000000000002 0) ⋄ 3 3⍴0.1000000000000001 0.2000000000000002 0.3000000000000003 0.4000000000000004 0.5 0.6000000000000005 0.7000000000000011 0.8000000000000007 0.9000000000000004)

⍝ aplcart/tt.tsv:652 — Fractional part of number
N←¯1.5 0 2.75 ⋄ 1∘|N   ⍝ 0.5 0 0.75

⍝ aplcart/tt.tsv:653 — Fractional part with sign
N←¯2.5 0 3.75 ⋄ (×|⊢)N   ⍝ ¯0.5 0 0.75

⍝ aplcart/tt.tsv:654 — Fractional part with sign
N←¯2.5 0 3.75 ⋄ (×|⊢)N   ⍝ ¯0.5 0 0.75

⍝ aplcart/tt.tsv:655 — Frequencies of major cells; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ {≢⍵}⌸Y   ⍝ 2 1 1

⍝ aplcart/tt.tsv:656 — Frequencies of major cells; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ {≢⍵}⌸Y   ⍝ 2 1 1

⍝ aplcart/tt.tsv:657 — From complex to magnitude and radians (increase rank with leading length-two axis)
N←1j2 ¯3j4 ⋄ (10 12 ○⌝ ⊢)N
2 2⍴2.23606797749979 5 1.10714871779409 2.214297435588181

⍝ aplcart/tt.tsv:658 — From complex to magnitude and radians (increase rank with leading length-two axis)
N←1j2 ¯3j4 ⋄ (10 12 ○⌝ ⊢)N
2 2⍴2.23606797749979 5 1.10714871779409 2.214297435588181

⍝ aplcart/tt.tsv:659 — From complex to real and imaginary (increase rank with leading length-two axis)
N←1j2 ¯3j4 ⋄ (9 11 ○⌝ ⊢)N   ⍝ 2 2⍴1 ¯3 2 4

⍝ aplcart/tt.tsv:660 — From complex to real and imaginary (increase rank with leading length-two axis)
N←1j2 ¯3j4 ⋄ (9 11 ○⌝ ⊢)N   ⍝ 2 2⍴1 ¯3 2 4

⍝ aplcart/tt.tsv:661 — Future value of cash flows N at interest Ms; Reuse concrete inputs from aplcart/table.tsv:1078; execute this alternate recipe independently
Ms←0.1 ⋄ N←100 200 300 ⋄ Ms(⊢⊥⍨1+⊣)N   ⍝ 641

⍝ aplcart/tt.tsv:662 — Future value of cash flows N at interest Ms; Reuse concrete inputs from aplcart/table.tsv:1078; execute this alternate recipe independently
Ms←0.1 ⋄ N←100 200 300 ⋄ Ms(⊢⊥⍨1+⊣)N   ⍝ 641

⍝ aplcart/tt.tsv:663 — Gamma function of N
N←1 2 3 4 ⋄ (!-∘1)N   ⍝ 1 1 2 6

⍝ aplcart/tt.tsv:664 — Gamma function of N
N←1 2 3 4 ⋄ (!-∘1)N   ⍝ 1 1 2 6

⍝ aplcart/tt.tsv:665 — General comparison according to Total Array Order (¯1:X precedes Y, 0:X≡Y, 1:X succeeds Y)
X←1 2 ⋄ Y←1 3 ⋄ X{↑(⍋-⍒)⍺⍵}Y   ⍝ ¯1

⍝ aplcart/tt.tsv:666 — General comparison according to Total Array Order (¯1:X precedes Y, 0:X≡Y, 1:X succeeds Y)
X←1 2 ⋄ Y←1 3 ⋄ X{↑(⍋-⍒)⍺⍵}Y   ⍝ ¯1

⍝ aplcart/tt.tsv:667 — Generate Roman numeral (purely additive, no refinements)
Js←1984 ⋄ ('MDCLXVI'/⍨(0,6⍴2 5)⊤⊢)Js   ⍝ 'MDCCCCLXXXIIII'

⍝ aplcart/tt.tsv:668 — Generate Roman numeral (purely additive, no refinements)
Js←1984 ⋄ ('MDCLXVI'/⍨(0,6⍴2 5)⊤⊢)Js   ⍝ 'MDCCCCLXXXIIII'

⍝ aplcart/tt.tsv:669 — Generate consolidated left argument for successive transposes Iv⍉Jv⍉Y
Iv←2 3 1 ⋄ Jv←3 1 2 ⋄ Iv⌷⍨∘⊂Jv   ⍝ 1 2 3

⍝ aplcart/tt.tsv:670 — Generate consolidated left argument for successive transposes Iv⍉Jv⍉Y
Iv←2 3 1 ⋄ Jv←3 1 2 ⋄ Iv⌷⍨∘⊂Jv   ⍝ 1 2 3

⍝ aplcart/tt.tsv:671 — Geometric mean
N←1 4 16 ⋄ (×⌿*∘÷≢)N   ⍝ 4

⍝ aplcart/tt.tsv:672 — Geometric mean
N←1 4 16 ⋄ (×⌿*∘÷≢)N   ⍝ 4

⍝ aplcart/tt.tsv:673 — Geometric-harmonic mean
Nv←1 4 ⋄ (↑((×⌿*∘÷≢),≢÷1⊥÷)⍣≡)Nv   ⍝ 1.783303179974246

⍝ aplcart/tt.tsv:674 — Geometric-harmonic mean
Nv←1 4 ⋄ (↑((×⌿*∘÷≢),≢÷1⊥÷)⍣≡)Nv   ⍝ 1.783303179974246

⍝ aplcart/tt.tsv:683 — Grade down of Y according to key X
X←4 3 2 1 ⋄ Y←1 3 2 ⋄ X(⍒⍳)Y   ⍝ 1 3 2

⍝ aplcart/tt.tsv:684 — Grade down of Y according to key X
X←4 3 2 1 ⋄ Y←1 3 2 ⋄ X(⍒⍳)Y   ⍝ 1 3 2

⍝ aplcart/tt.tsv:685 — Grade up of Y according to key X
X←4 3 2 1 ⋄ Y←1 3 2 ⋄ X(⍋⍳)Y   ⍝ 2 3 1

⍝ aplcart/tt.tsv:686 — Grade up of Y according to key X
X←4 3 2 1 ⋄ Y←1 3 2 ⋄ X(⍋⍳)Y   ⍝ 2 3 1

⍝ aplcart/tt.tsv:687 — Groups of ones in Bv pointed to by at least one 1 in Av
Av←0 1 0 0 0 0 0 1 ⋄ Bv←0 1 1 0 1 1 0 1 ⋄ Av(∧{⍵∧s∊⍺/s←+\(</∘(2∘↕))0,⍵}⊢)Bv
0 1 1 0 0 0 0 1

⍝ aplcart/tt.tsv:688 — Groups of ones in Bv pointed to by at least one 1 in Av
Av←0 1 0 0 0 0 0 1 ⋄ Bv←0 1 1 0 1 1 0 1 ⋄ Av(∧{⍵∧s∊⍺/s←+\(</∘(2∘↕))0,⍵}⊢)Bv
0 1 1 0 0 0 0 1

⍝ aplcart/tt.tsv:689 — Halve: N÷2
N←4 9 16 ⋄ (÷∘2)N   ⍝ 2 4.5 8

⍝ aplcart/tt.tsv:690 — Halve: N÷2
N←4 9 16 ⋄ (÷∘2)N   ⍝ 2 4.5 8

⍝ aplcart/tt.tsv:691 — Hamming distance; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(+/≠)Yv   ⍝ 3

⍝ aplcart/tt.tsv:692 — Hamming distance; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(+/≠)Yv   ⍝ 3

⍝ aplcart/tt.tsv:693 — Hamming weight
J←0 3 15 ⋄ 2∘(+⌿⊥⍣¯1)J   ⍝ 0 2 4

⍝ aplcart/tt.tsv:694 — Hamming weight
J←0 3 15 ⋄ 2∘(+⌿⊥⍣¯1)J   ⍝ 0 2 4

⍝ aplcart/tt.tsv:695 — Handling array Y temporarily as a vector (optionally with left argument X); Reuse concrete inputs from aplcart/table.tsv:928; execute this alternate recipe independently
X←10 ⋄ f←⌽ ⋄ Y←2 3⍴⍳6 ⋄ X f@(1⍴⍨⍴)Y   ⍝ 2 3⍴5 6 1 2 3 4

⍝ aplcart/tt.tsv:696 — Handling array Y temporarily as a vector (optionally with left argument X); Reuse concrete inputs from aplcart/table.tsv:928; execute this alternate recipe independently
X←10 ⋄ f←⌽ ⋄ Y←2 3⍴⍳6 ⋄ X f@(1⍴⍨⍴)Y   ⍝ 2 3⍴5 6 1 2 3 4

⍝ aplcart/tt.tsv:697 — Harmonic mean
N←4 9 16 ⋄ (≢÷1⊥÷)N   ⍝ 7.081967213114754

⍝ aplcart/tt.tsv:698 — Harmonic mean
N←4 9 16 ⋄ (≢÷1⊥÷)N   ⍝ 7.081967213114754

⍝ aplcart/tt.tsv:699 — Hartley kernel
N←4 9 16 ⋄ (1∘○+2∘○)N
¯1.41044611617154 ¯0.4990117766429203 ¯1.24556279698845

⍝ aplcart/tt.tsv:700 — Hartley kernel
N←4 9 16 ⋄ (1∘○+2∘○)N
¯1.41044611617154 ¯0.4990117766429203 ¯1.24556279698845

⍝ aplcart/tt.tsv:701 — Head: First major cell of Y
Y←2 3⍴⍳6 ⋄ 1∘⌷Y   ⍝ 1 2 3

⍝ aplcart/tt.tsv:702 — Head: First major cell of Y
Y←2 3⍴⍳6 ⋄ 1∘⌷Y   ⍝ 1 2 3

⍝ aplcart/tt.tsv:703 — Hilbert matrix of order Js
Js←4 ⋄ (÷⍳ +⌝ ¯1+⍳)Js
4 4⍴1 0.5 0.3333333333333333 0.25 0.5 0.3333333333333333 0.25 0.2 0.3333333333333333 0.25 0.2 0.1666666666666667 0.25 0.2 0.1666666666666667 0.1428571428571428

⍝ aplcart/tt.tsv:704 — Hilbert matrix of order Js
Js←4 ⋄ (÷⍳ +⌝ ¯1+⍳)Js
4 4⍴1 0.5 0.3333333333333333 0.25 0.5 0.3333333333333333 0.25 0.2 0.3333333333333333 0.25 0.2 0.1666666666666667 0.25 0.2 0.1666666666666667 0.1428571428571428

⍝ aplcart/tt.tsv:705 — Histogram
N←0 2 4 ⋄ (⊃'⎕'⍴¨⍨⌊)N   ⍝ 3 4⍴'    ⎕⎕  ⎕⎕⎕⎕'

⍝ aplcart/tt.tsv:706 — Histogram
N←0 2 4 ⋄ (⊃'⎕'⍴¨⍨⌊)N   ⍝ 3 4⍴'    ⎕⎕  ⎕⎕⎕⎕'

⍝ aplcart/tt.tsv:707 — Histogram (distribution barchart, down the page)
Jv←1 3 3 4 1 ⋄ ({'⎕'/⍨¯1+≢⍵}⌸⊢,⍨∘⍳⌈/)Jv   ⍝ 4 2⍴'⎕⎕  ⎕⎕⎕ '

⍝ aplcart/tt.tsv:708 — Histogram (distribution barchart, down the page)
Jv←1 3 3 4 1 ⋄ ({'⎕'/⍨¯1+≢⍵}⌸⊢,⍨∘⍳⌈/)Jv   ⍝ 4 2⍴'⎕⎕  ⎕⎕⎕ '

⍝ aplcart/tt.tsv:709 — Hook (S-combinator): apply f between Y and (g Y), that is Y f g Y
f←- ⋄ g←⌽ ⋄ Y←1 2 3 ⋄ f∘g⍨Y   ⍝ ¯2 0 2

⍝ aplcart/tt.tsv:710 — Hook (S-combinator): apply f between Y and (g Y), that is Y f g Y
f←- ⋄ g←⌽ ⋄ Y←1 2 3 ⋄ f∘g⍨Y   ⍝ ¯2 0 2

⍝ aplcart/tt.tsv:711 — Hyperbolic arcos N
N←1 2 3 ⋄ ¯6∘○N   ⍝ 0 1.316957896924817 1.762747174039086

⍝ aplcart/tt.tsv:712 — Hyperbolic arcos N
N←1 2 3 ⋄ ¯6∘○N   ⍝ 0 1.316957896924817 1.762747174039086

⍝ aplcart/tt.tsv:713 — Hyperbolic area cosecant
N←4 9 16 ⋄ (¯5○÷)N
0.2474664615472635 0.1108837483012855 0.06245938125554031

⍝ aplcart/tt.tsv:714 — Hyperbolic area cosecant
N←4 9 16 ⋄ (¯5○÷)N
0.2474664615472635 0.1108837483012855 0.06245938125554031

⍝ aplcart/tt.tsv:715 — Hyperbolic area cotangent
N←4 9 16 ⋄ (¯7○÷)N
0.2554128118829953 0.1115717756571049 0.06258157147700301

⍝ aplcart/tt.tsv:716 — Hyperbolic area cotangent
N←4 9 16 ⋄ (¯7○÷)N
0.2554128118829953 0.1115717756571049 0.06258157147700301

⍝ aplcart/tt.tsv:717 — Hyperbolic area secant
N←0.25 0.5 1 ⋄ (¯6○÷)N   ⍝ 2.063437068895561 1.316957896924817 0

⍝ aplcart/tt.tsv:718 — Hyperbolic area secant
N←0.25 0.5 1 ⋄ (¯6○÷)N   ⍝ 2.063437068895561 1.316957896924817 0

⍝ aplcart/tt.tsv:719 — Hyperbolic arsin N
N←0.25 0.5 0.75 ⋄ ¯5∘○N
0.2474664615472635 0.4812118250596035 0.6931471805599453

⍝ aplcart/tt.tsv:720 — Hyperbolic arsin N
N←0.25 0.5 0.75 ⋄ ¯5∘○N
0.2474664615472635 0.4812118250596035 0.6931471805599453

⍝ aplcart/tt.tsv:721 — Hyperbolic artan N
N←0.25 0.5 0.75 ⋄ ¯7∘○N
0.2554128118829953 0.5493061443340549 0.9729550745276566

⍝ aplcart/tt.tsv:722 — Hyperbolic artan N
N←0.25 0.5 0.75 ⋄ ¯7∘○N
0.2554128118829953 0.5493061443340549 0.9729550745276566

⍝ aplcart/tt.tsv:723 — Hyperbolic cosecant; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (÷5∘○)N
0.03664357032586561 0.0002468196119324168 2.250703494385211e¯07

⍝ aplcart/tt.tsv:724 — Hyperbolic cosecant; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (÷5∘○)N
0.03664357032586561 0.0002468196119324168 2.250703494385211e¯07

⍝ aplcart/tt.tsv:725 — Hyperbolic cosine N
N←0.25 0.5 0.75 ⋄ 6∘○N
1.031413099879573 1.127625965206381 1.294683284676845

⍝ aplcart/tt.tsv:726 — Hyperbolic cosine N
N←0.25 0.5 0.75 ⋄ 6∘○N
1.031413099879573 1.127625965206381 1.294683284676845

⍝ aplcart/tt.tsv:727 — Hyperbolic cotangent; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (÷7∘○)N
1.000671150401683 1.00000003045996 1.000000000000025

⍝ aplcart/tt.tsv:728 — Hyperbolic cotangent; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (÷7∘○)N
1.000671150401683 1.00000003045996 1.000000000000025

⍝ aplcart/tt.tsv:729 — Hyperbolic secant; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (÷6∘○)N
0.03661899347368653 0.0002468196044143015 2.250703494385154e¯07

⍝ aplcart/tt.tsv:730 — Hyperbolic secant; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (÷6∘○)N
0.03661899347368653 0.0002468196044143015 2.250703494385154e¯07

⍝ aplcart/tt.tsv:731 — Hyperbolic sine N
N←0.25 0.5 0.75 ⋄ 5∘○N
0.2526123168081683 0.5210953054937474 0.82231673193583

⍝ aplcart/tt.tsv:732 — Hyperbolic sine N
N←0.25 0.5 0.75 ⋄ 5∘○N
0.2526123168081683 0.5210953054937474 0.82231673193583

⍝ aplcart/tt.tsv:733 — Hyperbolic tangent N
N←0.25 0.5 0.75 ⋄ 7∘○N
0.2449186624037091 0.4621171572600097 0.6351489523872873

⍝ aplcart/tt.tsv:734 — Hyperbolic tangent N
N←0.25 0.5 0.75 ⋄ 7∘○N
0.2449186624037091 0.4621171572600097 0.6351489523872873

⍝ aplcart/tt.tsv:735 — ISBN check digit generator from ten first digits Jv
Jv←0 3 0 6 4 0 6 1 5 ⋄ (|¯11|1⊥+\)Jv   ⍝ 5

⍝ aplcart/tt.tsv:736 — ISBN check digit generator from ten first digits Jv
Jv←0 3 0 6 4 0 6 1 5 ⋄ (|¯11|1⊥+\)Jv   ⍝ 5

⍝ aplcart/tt.tsv:737 — Identity matrix of order Js; Reuse concrete inputs from aplcart/table.tsv:1252; execute this alternate recipe independently
Js←4 ⋄ (,⍨⍴1↑⍨1∘+)Js   ⍝ 4 4⍴1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1

⍝ aplcart/tt.tsv:738 — Identity matrix of order Js; Reuse concrete inputs from aplcart/table.tsv:1252; execute this alternate recipe independently
Js←4 ⋄ (,⍨⍴1↑⍨1∘+)Js   ⍝ 4 4⍴1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1

⍝ aplcart/tt.tsv:739 — Identity matrix of shape of matrix Nm
Nm←2 3⍴⍳6 ⋄ (⌽⍤⍴⍴1↑⍨1+≢)Nm   ⍝ 3 2⍴1 0 0 1 0 0

⍝ aplcart/tt.tsv:740 — Identity matrix of shape of matrix Nm
Nm←2 3⍴⍳6 ⋄ (⌽⍤⍴⍴1↑⍨1+≢)Nm   ⍝ 3 2⍴1 0 0 1 0 0

⍝ aplcart/tt.tsv:741 — Identity of two sets
Xv←1 2 3 ⋄ Yv←3 2 1 1 ⋄ Xv(∧/∊⍨,∊)Yv   ⍝ 1

⍝ aplcart/tt.tsv:742 — Identity of two sets
Xv←1 2 3 ⋄ Yv←3 2 1 1 ⋄ Xv(∧/∊⍨,∊)Yv   ⍝ 1

⍝ aplcart/tt.tsv:743 — If Y begins with X
X←1 2 ⋄ Y←1 2 3 1 2 ⋄ X(↑⍷)Y   ⍝ 1

⍝ aplcart/tt.tsv:744 — If Y begins with X
X←1 2 ⋄ Y←1 2 3 1 2 ⋄ X(↑⍷)Y   ⍝ 1

⍝ aplcart/tt.tsv:745 — Ignore left argument (call f monadically on Y)
f←+/ ⋄ X←1 2 ⋄ Y←3 4 ⋄ X f⍤⊢Y   ⍝ 7

⍝ aplcart/tt.tsv:746 — Ignore left argument (call f monadically on Y)
f←+/ ⋄ X←1 2 ⋄ Y←3 4 ⋄ X f⍤⊢Y   ⍝ 7

⍝ aplcart/tt.tsv:747 — Ignore right argument (call f monadically on X)
f←+/ ⋄ X←1 2 ⋄ Y←3 4 ⋄ X f⍤⊣Y   ⍝ 3

⍝ aplcart/tt.tsv:748 — Ignore right argument (call f monadically on X)
f←+/ ⋄ X←1 2 ⋄ Y←3 4 ⋄ X f⍤⊣Y   ⍝ 3

⍝ aplcart/tt.tsv:749 — Imaginary part of N
N←3j4 0j2 ⋄ 11∘○N   ⍝ 4 2

⍝ aplcart/tt.tsv:750 — Imaginary part of N
N←3j4 0j2 ⋄ 11∘○N   ⍝ 4 2

⍝ aplcart/tt.tsv:751 — Inclusive integer difference
I←2 3 4 ⋄ J←4 9 16 ⋄ I(1+-)J   ⍝ ¯1 ¯5 ¯11

⍝ aplcart/tt.tsv:752 — Inclusive integer difference
I←2 3 4 ⋄ J←4 9 16 ⋄ I(1+-)J   ⍝ ¯1 ¯5 ¯11

⍝ aplcart/tt.tsv:753 — Increase rank scalar/vector/matrix Ym to 2 (matrix: 1-row if vector, 1-column if scalar); nested displayed values saved once in evaluation order and returned together
(r1←(⊢⍴⍨¯2↑1 1,⍴) 'a' ⋄ ⍴ r1 ⋄ r3←(⊢⍴⍨¯2↑1 1,⍴) 'abc' ⋄ ⍴ r3 ⋄ r5←(⊢⍴⍨¯2↑1 1,⍴) 2 3⍴'abcdef' ⋄ ⍴ r5)
(1 1⍴'a' ⋄ 1 1 ⋄ 1 3⍴'abc' ⋄ 1 3 ⋄ 2 3⍴'abcdef' ⋄ 2 3)

⍝ aplcart/tt.tsv:754 — Increase rank scalar/vector/matrix Ym to 2 (matrix: 1-row if vector, 1-column if scalar)
Ym←2 3⍴⍳6 ⋄ (⊢⍴⍨¯2↑1 1,⍴)Ym   ⍝ 2 3⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:755 — Increasing absolute value without change of sign
M←1 ⋄ N←¯3 0 2 ⋄ M(⊢∘××+∘|)N   ⍝ ¯4 0 3

⍝ aplcart/tt.tsv:756 — Increasing absolute value without change of sign
M←1 ⋄ N←¯3 0 2 ⋄ M(⊢∘××+∘|)N   ⍝ ¯4 0 3

⍝ aplcart/tt.tsv:757 — Increasing the dimensions of Y to multiples of Iv
Iv←3 4 ⋄ Y←2 3⍴⍳6 ⋄ Iv(⊢↑⍨⊢∘⍴+|∘-∘⍴)Y   ⍝ 3 4⍴1 2 3 0 4 5 6 0 0 0 0 0

⍝ aplcart/tt.tsv:758 — Increasing the dimensions of Y to multiples of Iv
Iv←3 4 ⋄ Y←2 3⍴⍳6 ⋄ Iv(⊢↑⍨⊢∘⍴+|∘-∘⍴)Y   ⍝ 3 4⍴1 2 3 0 4 5 6 0 0 0 0 0

⍝ aplcart/tt.tsv:759 — Increasing the leading dimension of Y to multiple of Is
Is←4 ⋄ Y←1 2 3 4 5 ⋄ Is(⊢↑⍨⊢∘≢+|∘-∘≢)Y   ⍝ 1 2 3 4 5 0 0 0

⍝ aplcart/tt.tsv:760 — Increasing the leading dimension of Y to multiple of Is
Is←4 ⋄ Y←1 2 3 4 5 ⋄ Is(⊢↑⍨⊢∘≢+|∘-∘≢)Y   ⍝ 1 2 3 4 5 0 0 0

⍝ aplcart/tt.tsv:761 — Increment on change: Array of same shape as Y beginning with 1 and increasing for each change in adjacent values
(+⍀1⍪(≢/[2]∘(2∘↕))) 'Mississippi'   ⍝ 1 2 3 3 4 5 5 6 7 7 8

⍝ aplcart/tt.tsv:762 — Increment on change: Array of same shape as Y beginning with 1 and increasing for each change in adjacent values
Y←5 2⍴1 2 1 2 3 4 3 4 1 2 ⋄ (+⍀1⍪(≢/[2]∘(2∘↕)))Y
5 2⍴1 1 1 1 2 2 2 2 3 3

⍝ aplcart/tt.tsv:763 — Increment rank by inserting a new dimension after the leading one
⍴ (⊃∘,∘⊂⍤¯1) 2 3 4⍴•A   ⍝ 2 1 3 4

⍝ aplcart/tt.tsv:764 — Increment rank by inserting a new dimension after the leading one
Y←2 3⍴⍳6 ⋄ (⊃∘,∘⊂⍤¯1)Y   ⍝ 2 1 3⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:765 — Increment rank by inserting a new dimension after the trailing one
(⍴ (,⍤0) 'abc' ⋄ ⍴ (,⍤0) 3 4⍴•A)   ⍝ (3 1 ⋄ 3 4 1)

⍝ aplcart/tt.tsv:766 — Increment rank by inserting a new dimension after the trailing one
Y←2 3⍴⍳6 ⋄ (,⍤0)Y   ⍝ 2 3 1⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:767 — Increment rank by inserting a new dimension before the leading one
(⍴ (⊃,∘⊂) 'abc' ⋄ ⍴ (⊃,∘⊂) 3 4⍴•A)   ⍝ (1 3 ⋄ 1 3 4)

⍝ aplcart/tt.tsv:768 — Increment rank by inserting a new dimension before the leading one
Y←2 3⍴⍳6 ⋄ (⊃,∘⊂)Y   ⍝ 1 2 3⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:769 — Increment rank by inserting a new dimension before the trailing one
⍴ (⊃,∘⊂⍤1) 2 3 4⍴•A   ⍝ 2 3 1 4

⍝ aplcart/tt.tsv:770 — Increment rank by inserting a new dimension before the trailing one
Y←2 3⍴⍳6 ⋄ (⊃,∘⊂⍤1)Y   ⍝ 2 1 3⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:771 — Increment: N+1
(1∘+ 10 20 30 ⋄ 1∘+ ¯10 ¯20 ¯30 ⋄ (3 3⍴4) ⋄ 1∘+ (3 3⍴4))
(11 21 31 ⋄ ¯9 ¯19 ¯29 ⋄ 3 3⍴4 4 4 4 4 4 4 4 4 ⋄ 3 3⍴5 5 5 5 5 5 5 5 5)

⍝ aplcart/tt.tsv:772 — Increment: N+1
N←4 9 16 ⋄ 1∘+N   ⍝ 5 10 17

⍝ aplcart/tt.tsv:773 — Incrementing cyclic counter J with upper limit I
I←2 3 4 ⋄ J←4 9 16 ⋄ I(1+|)J   ⍝ 1 1 1

⍝ aplcart/tt.tsv:774 — Incrementing cyclic counter J with upper limit I
I←2 3 4 ⋄ J←4 9 16 ⋄ I(1+|)J   ⍝ 1 1 1

⍝ aplcart/tt.tsv:775 — Index in X of the first element which is a member of Y
X←1 2 3 ⋄ Y←3 4 ⋄ X(↑∘⍸∊)Y   ⍝ 3

⍝ aplcart/tt.tsv:776 — Index in X of the first element which is a member of Y
X←1 2 3 ⋄ Y←3 4 ⋄ X(↑∘⍸∊)Y   ⍝ 3

⍝ aplcart/tt.tsv:777 — Index of Largest
Y←3 1 3 2 ⋄ (↑⍒)Y   ⍝ 1

⍝ aplcart/tt.tsv:778 — Index of Largest
Y←3 1 3 2 ⋄ (↑⍒)Y   ⍝ 1

⍝ aplcart/tt.tsv:779 — Index of Smallest
Y←3 1 3 2 ⋄ (↑⍋)Y   ⍝ 2

⍝ aplcart/tt.tsv:780 — Index of Smallest
Y←3 1 3 2 ⋄ (↑⍋)Y   ⍝ 2

⍝ aplcart/tt.tsv:781 — Index of first 1 in N
N←0 0 1 0 ⋄ (⍳∘1)N   ⍝ 3

⍝ aplcart/tt.tsv:782 — Index of first 1 in N
N←0 0 1 0 ⋄ (⍳∘1)N   ⍝ 3

⍝ aplcart/tt.tsv:783 — Index of first consecutive occurrence of major cells of X in Y; Reuse concrete inputs from aplcart/table.tsv:1213; execute this alternate recipe independently
X←2 2⍴3 4 5 6 ⋄ Y←4 2⍴1 2 3 4 5 6 7 8 ⋄ X(1⍳⍨↑⍤¯1⍤⍷)Y
2

⍝ aplcart/tt.tsv:784 — Index of first consecutive occurrence of major cells of X in Y; Reuse concrete inputs from aplcart/table.tsv:1213; execute this alternate recipe independently
X←2 2⍴3 4 5 6 ⋄ Y←4 2⍴1 2 3 4 5 6 7 8 ⋄ X(1⍳⍨↑⍤¯1⍤⍷)Y
2

⍝ aplcart/tt.tsv:785 — Index of first differing element in X and Y
X←1 2 3 ⋄ Y←1 4 3 ⋄ X(↑∘⍸≠)Y   ⍝ 2

⍝ aplcart/tt.tsv:786 — Index of first differing element in X and Y
X←1 2 3 ⋄ Y←1 4 3 ⋄ X(↑∘⍸≠)Y   ⍝ 2

⍝ aplcart/tt.tsv:787 — Index of first instance of each major cell; Reuse concrete inputs from aplcart/table.tsv:1136; execute this alternate recipe independently
Y←3 1 3 2 ⋄ (⍳⍨∪⍳⊢)Y   ⍝ 1 2 1 4

⍝ aplcart/tt.tsv:788 — Index of first instance of each major cell; Reuse concrete inputs from aplcart/table.tsv:1136; execute this alternate recipe independently
Y←3 1 3 2 ⋄ (⍳⍨∪⍳⊢)Y   ⍝ 1 2 1 4

⍝ aplcart/tt.tsv:789 — Index of first occurrence in X of any item of Y; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X(1⍳⍨∊)Y   ⍝ 1

⍝ aplcart/tt.tsv:790 — Index of first occurrence in X of any item of Y; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X(1⍳⍨∊)Y   ⍝ 1

⍝ aplcart/tt.tsv:791 — Index of first occurrence in X of any major cell of Y; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←1 2 ⋄ X(⌊/⍳)Y   ⍝ 2

⍝ aplcart/tt.tsv:792 — Index of first occurrence in X of any major cell of Y; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←1 2 ⋄ X(⌊/⍳)Y   ⍝ 2

⍝ aplcart/tt.tsv:793 — Index of first one after index Is in Bv; Reuse concrete inputs from aplcart/table.tsv:1130; execute this alternate recipe independently
I←2 ⋄ B←0 1 0 0 1 0 ⋄ I(⊣+1⍳⍨↓)B   ⍝ 5

⍝ aplcart/tt.tsv:794 — Index of first one after index Is in Bv; Reuse concrete inputs from aplcart/table.tsv:1130; execute this alternate recipe independently
I←2 ⋄ B←0 1 0 0 1 0 ⋄ I(⊣+1⍳⍨↓)B   ⍝ 5

⍝ aplcart/tt.tsv:795 — Index of first satisfied condition in B
B←1 1 0 1 0 1 0 ⋄ (↑⍸)B   ⍝ 1

⍝ aplcart/tt.tsv:796 — Index of first satisfied condition in B
B←1 1 0 1 0 1 0 ⋄ (↑⍸)B   ⍝ 1

⍝ aplcart/tt.tsv:797 — Index of last maximum element of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⊢/⍋)Y   ⍝ 3

⍝ aplcart/tt.tsv:798 — Index of last maximum element of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⊢/⍋)Y   ⍝ 3

⍝ aplcart/tt.tsv:799 — Index of last minimum element of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⊢/⍒)Y   ⍝ 2

⍝ aplcart/tt.tsv:800 — Index of last minimum element of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⊢/⍒)Y   ⍝ 2

⍝ aplcart/tt.tsv:801 — Index of last occurrence in X of any major cell of Y; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←1 2 ⋄ X(⌈/⍳)Y   ⍝ 3

⍝ aplcart/tt.tsv:802 — Index of last occurrence in X of any major cell of Y; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←1 2 ⋄ X(⌈/⍳)Y   ⍝ 3

⍝ aplcart/tt.tsv:803 — Index of last occurrence of major cells Y in X, counted from the rear
'abracadabra' ⍳⍨∘⊖⍨ 'ab'   ⍝ 1 3

⍝ aplcart/tt.tsv:804 — Index of last occurrence of major cells Y in X, counted from the rear; Reuse concrete inputs from aplcart/table.tsv:680; execute this alternate recipe independently
X←4 2⍴1 2 3 4 1 2 5 6 ⋄ Y←3 2⍴1 2 5 6 7 8 ⋄ X⍳⍨∘⊖⍨Y
2 1 5

⍝ aplcart/tt.tsv:805 — Index random item from array; Reuse concrete inputs from aplcart/table.tsv:867; execute this alternate recipe independently
Y←2 3⍴10×⍳6 ⋄ r←(?∘⍴⌷⊢)Y ⋄ (0=≢⍴r)∧r∊Y   ⍝ 1

⍝ aplcart/tt.tsv:806 — Index random item from array; Reuse concrete inputs from aplcart/table.tsv:867; execute this alternate recipe independently
Y←2 3⍴10×⍳6 ⋄ r←(?∘⍴⌷⊢)Y ⋄ (0=≢⍴r)∧r∊Y   ⍝ 1

⍝ aplcart/tt.tsv:807 — Indicate increases in N
N←1 3 3 2 5 ⋄ (0⍪(>/[2]∘(2∘↕)))N   ⍝ 0 0 0 1 0

⍝ aplcart/tt.tsv:808 — Indicate increases in N
N←1 3 3 2 5 ⋄ (0⍪(>/[2]∘(2∘↕)))N   ⍝ 0 0 0 1 0

⍝ aplcart/tt.tsv:809 — Indicate leading elements that are equal; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X(∧\=)Y   ⍝ 1 1 0 0

⍝ aplcart/tt.tsv:810 — Indicate leading elements that are equal; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X(∧\=)Y   ⍝ 1 1 0 0

⍝ aplcart/tt.tsv:811 — Indicate leading elements that are unequal; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X(∧\≠)Y   ⍝ 0 0 0 0

⍝ aplcart/tt.tsv:812 — Indicate leading elements that are unequal; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X(∧\≠)Y   ⍝ 0 0 0 0

⍝ aplcart/tt.tsv:813 — Indicate starting points of groups of equal elements (non-empty Yv)
Y←1 1 2 2 1 ⋄ (1⍪(≢/[2]∘(2∘↕)))Y   ⍝ 1 0 1 0 1

⍝ aplcart/tt.tsv:814 — Indicate starting points of groups of equal elements (non-empty Yv)
Y←1 1 2 2 1 ⋄ (1⍪(≢/[2]∘(2∘↕)))Y   ⍝ 1 0 1 0 1

⍝ aplcart/tt.tsv:815 — Indicate trailing elements that are equal
X←1 2 3 4 ⋄ Y←0 2 3 4 ⋄ X(∧\∘⌽=)Y   ⍝ 1 1 1 0

⍝ aplcart/tt.tsv:816 — Indicate trailing elements that are equal
X←1 2 3 4 ⋄ Y←0 2 3 4 ⋄ X(∧\∘⌽=)Y   ⍝ 1 1 1 0

⍝ aplcart/tt.tsv:817 — Indicate trailing elements that are unequal
X←1 2 3 4 ⋄ Y←0 2 0 0 ⋄ X(∧\∘⌽≠)Y   ⍝ 1 1 0 0

⍝ aplcart/tt.tsv:818 — Indicate trailing elements that are unequal
X←1 2 3 4 ⋄ Y←0 2 0 0 ⋄ X(∧\∘⌽≠)Y   ⍝ 1 1 0 0

⍝ aplcart/tt.tsv:819 — Indicate which elements differ from previous ones (non-empty Yv)
Y←1 1 2 2 1 ⋄ (1⍪(≢/[2]∘(2∘↕)))Y   ⍝ 1 0 1 0 1

⍝ aplcart/tt.tsv:820 — Indicate which elements differ from previous ones (non-empty Yv)
Y←1 1 2 2 1 ⋄ (1⍪(≢/[2]∘(2∘↕)))Y   ⍝ 1 0 1 0 1

⍝ aplcart/tt.tsv:821 — Indicate which numbers in N are perfect squares
(=∘⌊⍨*∘0.5) 1 2 3 4 5 6 7 8 9 10   ⍝ 1 0 0 1 0 0 0 0 1 0

⍝ aplcart/tt.tsv:822 — Indicate which numbers in N are perfect squares
N←0 1 2 4 9 10 ⋄ (=∘⌊⍨*∘0.5)N   ⍝ 1 1 0 1 1 0

⍝ aplcart/tt.tsv:823 — Indicator of first occurrence of each unique major cell of Y
Y←3 1 3 2 ⋄ (⍳⍨=⍳∘≢)Y   ⍝ 1 1 0 1

⍝ aplcart/tt.tsv:824 — Indicator of first occurrence of each unique major cell of Y
Y←3 1 3 2 ⋄ (⍳⍨=⍳∘≢)Y   ⍝ 1 1 0 1

⍝ aplcart/tt.tsv:825 — Indices (⍳) in X of items of Y; Upstream compose calls dyadic-only find monadically (Dyalog SYNTAX ERROR); use atop ⍤⍷ to preserve both arguments and compute the described indices
X←1 2 3 4 2 ⋄ Y←2 4 ⋄ X(↑∘⍸⍤⍷)¨∘⊂⍨Y   ⍝ 2 4

⍝ aplcart/tt.tsv:826 — Indices (⍳) in X of items of Y; Upstream compose calls dyadic-only find monadically (Dyalog SYNTAX ERROR); use atop ⍤⍷ to preserve both arguments and compute the described indices
X←1 2 3 4 2 ⋄ Y←2 4 ⋄ X(↑∘⍸⍤⍷)¨∘⊂⍨Y   ⍝ 2 4

⍝ aplcart/tt.tsv:827 — Indices of Iv'th elements in ravel order of an array of dimensions Jv
Iv←1 4 6 ⋄ Jv←2 3 ⋄ Iv(,⌿1+⊢⊤¯1+⊣)Jv   ⍝ (1 1 ⋄ 2 1 ⋄ 2 3)

⍝ aplcart/tt.tsv:828 — Indices of Iv'th elements in ravel order of an array of dimensions Jv
Iv←1 4 6 ⋄ Jv←2 3 ⋄ Iv(,⌿1+⊢⊤¯1+⊣)Jv   ⍝ (1 1 ⋄ 2 1 ⋄ 2 3)

⍝ aplcart/tt.tsv:829 — Indices of Major Cells of Y
Y←3 1 3 2 ⋄ (⍳≢)Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:830 — Indices of Major Cells of Y
Y←3 1 3 2 ⋄ (⍳≢)Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:831 — Indices of all occurrences of elements of X in Y
X←1 2 3 4 2 ⋄ Y←2 4 ⋄ X(⍸∊)Y   ⍝ 2 4 5

⍝ aplcart/tt.tsv:832 — Indices of all occurrences of elements of X in Y
X←1 2 3 4 2 ⋄ Y←2 4 ⋄ X(⍸∊)Y   ⍝ 2 4 5

⍝ aplcart/tt.tsv:833 — Indices of dimensions of Y
Y←3 1 3 2 ⋄ (⍳∘≢⍴)Y   ⍝ 1⍴1

⍝ aplcart/tt.tsv:834 — Indices of dimensions of Y
Y←3 1 3 2 ⋄ (⍳∘≢⍴)Y   ⍝ 1⍴1

⍝ aplcart/tt.tsv:835 — Indices of elements of X in corresponding rows of X (X[i;]⍳Y[i;])
X←2 3⍴1 2 3 4 5 6 ⋄ Y←2 2⍴2 7 5 4 ⋄ X(⍳⍤1)Y
2 2⍴2 4 2 1

⍝ aplcart/tt.tsv:836 — Indices of elements of X in corresponding rows of X (X[i;]⍳Y[i;])
X←2 3⍴1 2 3 4 5 6 ⋄ Y←2 2⍴2 7 5 4 ⋄ X(⍳⍤1)Y
2 2⍴2 4 2 1

⍝ aplcart/tt.tsv:837 — Indices of first blanks in rows of array D
D← 3 4⍴'abc a  b xyz'  ⋄ (⍳∘' '⍤1)D   ⍝ 4 2 1

⍝ aplcart/tt.tsv:838 — Indices of first blanks in rows of array D
D← 3 4⍴'abc a  b xyz'  ⋄ (⍳∘' '⍤1)D   ⍝ 4 2 1

⍝ aplcart/tt.tsv:839 — Indices of items of Yv in right-inclusive intervals with cut-offs Xv
('AEIOU' (⍸-∊⍨) 'DYALOG' ⋄ 2 4 6 (⍸-∊⍨) 1 2 3 4 5 6 7)
(1 5 0 3 3 2 ⋄ 0 0 1 1 2 2 3)

⍝ aplcart/tt.tsv:840 — Indices of items of Yv in right-inclusive intervals with cut-offs Xv
Xv←2 4 ⋄ Yv←1 2 3 4 5 ⋄ Xv(⍸-∊⍨)Yv   ⍝ 0 0 1 1 2

⍝ aplcart/tt.tsv:841 — Indices of last non-blanks in rows
D← 3 4⍴'ab    c d   '  ⋄ (↑⍤⌽⍤⍸⍤1≠∘' ')D
2 3 1

⍝ aplcart/tt.tsv:842 — Indices of last non-blanks in rows
D← 3 4⍴'ab    c d   '  ⋄ (↑⍤⌽⍤⍸⍤1≠∘' ')D
2 3 1

⍝ aplcart/tt.tsv:843 — Indices of major cells of Y in right-inclusive intervals with cut-offs X
mat←3 2⍴⍳6 ⋄ (mat (⍸-⍳≤∘≢⊣) 3 3 ⋄ mat (⍸-⍳≤∘≢⊣) 3 4)
1 1

⍝ aplcart/tt.tsv:844 — Indices of major cells of Y in right-inclusive intervals with cut-offs X
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X(⍸-⍳≤∘≢⊣)Y   ⍝ 0 0 1 1 2

⍝ aplcart/tt.tsv:845 — Indices of trailing axis of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⍳¯1↑⍴)Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:846 — Indices of trailing axis of Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (⍳¯1↑⍴)Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:847 — Infinity-norm
(⌈⌿|) 3 4   ⍝ 4

⍝ aplcart/tt.tsv:848 — Infinity-norm; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌈⌿|)N   ⍝ 4 5 6

⍝ aplcart/tt.tsv:849 — Initialise a matrix with Js columns and no rows; Reuse concrete inputs from aplcart/table.tsv:747; execute this alternate recipe independently
Js←4 ⋄ (⍴⍨0∘,)Js   ⍝ 0 4⍴0

⍝ aplcart/tt.tsv:850 — Initialise a matrix with Js columns and no rows; Reuse concrete inputs from aplcart/table.tsv:747; execute this alternate recipe independently
Js←4 ⋄ (⍴⍨0∘,)Js   ⍝ 0 4⍴0

⍝ aplcart/tt.tsv:853 — Inserting Xs before each element of Yv; optional {X} instantiated as dyadic use
Xs←9 ⋄ Yv←4 5 6 ⋄ Xs(,,⍤0)Yv   ⍝ 9 4 9 5 9 6

⍝ aplcart/tt.tsv:854 — Inserting Xs before each element of Yv; optional {X} instantiated as dyadic use
Xs←9 ⋄ Yv←4 5 6 ⋄ Xs(,,⍤0)Yv   ⍝ 9 4 9 5 9 6

⍝ aplcart/tt.tsv:855 — Inserting Xv after every element of Yv
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,,⍤0 1⍨)Yv   ⍝ 4 1 2 3 5 1 2 3 6 1 2 3

⍝ aplcart/tt.tsv:856 — Inserting Xv after every element of Yv
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,,⍤0 1⍨)Yv   ⍝ 4 1 2 3 5 1 2 3 6 1 2 3

⍝ aplcart/tt.tsv:857 — Inserting Xv before every element of Yv; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,,⍤1 0)Yv   ⍝ 1 2 3 4 1 2 3 5 1 2 3 6

⍝ aplcart/tt.tsv:858 — Inserting Xv before every element of Yv; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,,⍤1 0)Yv   ⍝ 1 2 3 4 1 2 3 5 1 2 3 6

⍝ aplcart/tt.tsv:859 — Integer representation of logical vector Bv
(2∘⊥ 0 0 1 ⋄ 2∘⊥ 0 1 0 ⋄ 2∘⊥ 0 1 1 ⋄ 2∘⊥ 1 0 0)
1 2 3 4

⍝ aplcart/tt.tsv:860 — Integer representation of logical vector Bv
Bv←1 1 1 0 0 ⋄ 2∘⊥Bv   ⍝ 28

⍝ aplcart/tt.tsv:861 — Integers from -Js to Js
Js←4 ⋄ (⌽⌽,0,-)∘⍳Js   ⍝ ¯4 ¯3 ¯2 ¯1 0 1 2 3 4

⍝ aplcart/tt.tsv:862 — Integers from -Js to Js
Js←4 ⋄ (⌽⌽,0,-)∘⍳Js   ⍝ ¯4 ¯3 ¯2 ¯1 0 1 2 3 4

⍝ aplcart/tt.tsv:863 — Integers from 0 to Js-1
Js←4 ⋄ (¯1+⍳)Js   ⍝ 0 1 2 3

⍝ aplcart/tt.tsv:864 — Integers from 0 to Js-1
Js←4 ⋄ (¯1+⍳)Js   ⍝ 0 1 2 3

⍝ aplcart/tt.tsv:865 — Integral and fractional part of positive number
0 1∘⊤ 8.75   ⍝ 8 0.75

⍝ aplcart/tt.tsv:866 — Integral and fractional part of positive number
N←0 1.5 2.75 ⋄ 0 1∘⊤N   ⍝ 2 3⍴0 1 2 0 0.5 0.75

⍝ aplcart/tt.tsv:867 — Inverse: find Z such that Y ≡ X f Z
X←3 ⋄ f←+ ⋄ Y←1 2 3 ⋄ X(f⍣¯1)Y   ⍝ ¯2 ¯1 0

⍝ aplcart/tt.tsv:868 — Inverse: find Z such that Y ≡ X f Z
X←3 ⋄ f←+ ⋄ Y←1 2 3 ⋄ X(f⍣¯1)Y   ⍝ ¯2 ¯1 0

⍝ aplcart/tt.tsv:869 — Inverse: find Z such that Y ≡ f Z
f←- ⋄ Y←1 2 3 ⋄ (f⍣¯1)Y   ⍝ ¯1 ¯2 ¯3

⍝ aplcart/tt.tsv:870 — Inverse: find Z such that Y ≡ f Z
f←- ⋄ Y←1 2 3 ⋄ (f⍣¯1)Y   ⍝ ¯1 ¯2 ¯3

⍝ aplcart/tt.tsv:871 — Is Bm a full lower triangular matrix with diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (⊢≡ ≥⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 0 0 1 1 0 1 1 1 ⋄ (⊢≡ ≥⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 0 0 1 0 0 1 1 0 ⋄ (⊢≡ ≥⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 0 0 0 0 1 0 0 ⋄ (⊢≡ ≥⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 0 0 1 1 0 1 1 1) 1 (3 3⍴0 0 0 1 0 0 1 1 0) 0 (3 3⍴0 0 0 0 0 0 1 0 0) 0

⍝ aplcart/tt.tsv:872 — Is Bm a full lower triangular matrix with diagonal?; Reuse concrete inputs from aplcart/table.tsv:1300; execute this alternate recipe independently
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (⊢≡ ≥⌝ ⍨∘⍳∘≢)Bm
0

⍝ aplcart/tt.tsv:873 — Is Bm a full lower triangular matrix without diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (⊢≡ >⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 0 0 1 1 0 1 1 1 ⋄ (⊢≡ >⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 0 0 1 0 0 1 1 0 ⋄ (⊢≡ >⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 0 0 0 0 1 0 0 ⋄ (⊢≡ >⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 0 0 1 1 0 1 1 1) 0 (3 3⍴0 0 0 1 0 0 1 1 0) 1 (3 3⍴0 0 0 0 0 0 1 0 0) 0

⍝ aplcart/tt.tsv:874 — Is Bm a full lower triangular matrix without diagonal?; Reuse concrete inputs from aplcart/table.tsv:1301; execute this alternate recipe independently
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (⊢≡ >⌝ ⍨∘⍳∘≢)Bm
0

⍝ aplcart/tt.tsv:875 — Is Bm a full upper triangular matrix with diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (⊢≡ ≤⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 1 1 0 1 1 0 0 1 ⋄ (⊢≡ ≤⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 1 1 0 0 1 0 0 0 ⋄ (⊢≡ ≤⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 1 0 0 0 0 0 0 ⋄ (⊢≡ ≤⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 1 1 0 1 1 0 0 1) 1 (3 3⍴0 1 1 0 0 1 0 0 0) 0 (3 3⍴0 0 1 0 0 0 0 0 0) 0

⍝ aplcart/tt.tsv:876 — Is Bm a full upper triangular matrix with diagonal?; Reuse concrete inputs from aplcart/table.tsv:1299; execute this alternate recipe independently
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (⊢≡ ≤⌝ ⍨∘⍳∘≢)Bm
0

⍝ aplcart/tt.tsv:877 — Is Bm a full upper triangular matrix without diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (⊢≡ <⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 1 1 0 1 1 0 0 1 ⋄ (⊢≡ <⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 1 1 0 0 1 0 0 0 ⋄ (⊢≡ <⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 1 0 0 0 0 0 0 ⋄ (⊢≡ <⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 1 1 0 1 1 0 0 1) 0 (3 3⍴0 1 1 0 0 1 0 0 0) 1 (3 3⍴0 0 1 0 0 0 0 0 0) 0

⍝ aplcart/tt.tsv:878 — Is Bm a full upper triangular matrix without diagonal?; Reuse concrete inputs from aplcart/table.tsv:1298; execute this alternate recipe independently
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (⊢≡ <⌝ ⍨∘⍳∘≢)Bm
0

⍝ aplcart/tt.tsv:879 — Is Bm a lower triangular matrix with diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (~0∊×∘|≤ ≥⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 0 0 1 1 0 1 1 1 ⋄ (~0∊×∘|≤ ≥⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 0 0 1 0 0 1 1 0 ⋄ (~0∊×∘|≤ ≥⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 0 0 0 0 1 0 0 ⋄ (~0∊×∘|≤ ≥⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 0 0 1 1 0 1 1 1) 1 (3 3⍴0 0 0 1 0 0 1 1 0) 1 (3 3⍴0 0 0 0 0 0 1 0 0) 1

⍝ aplcart/tt.tsv:880 — Is Bm a lower triangular matrix with diagonal?
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (~0∊×∘|≤ ≥⌝ ⍨∘⍳∘≢)Bm
0

⍝ aplcart/tt.tsv:881 — Is Bm a lower triangular matrix without diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (~0∊×∘|≤ >⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 0 0 1 1 0 1 1 1 ⋄ (~0∊×∘|≤ >⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 0 0 1 0 0 1 1 0 ⋄ (~0∊×∘|≤ >⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 0 0 0 0 1 0 0 ⋄ (~0∊×∘|≤ >⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 0 0 1 1 0 1 1 1) 0 (3 3⍴0 0 0 1 0 0 1 1 0) 1 (3 3⍴0 0 0 0 0 0 1 0 0) 1

⍝ aplcart/tt.tsv:882 — Is Bm a lower triangular matrix without diagonal?
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (~0∊×∘|≤ >⌝ ⍨∘⍳∘≢)Bm
0

⍝ aplcart/tt.tsv:883 — Is Bm an upper triangular matrix with diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (~0∊×∘|≤ ≤⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 1 1 0 1 1 0 0 1 ⋄ (~0∊×∘|≤ ≤⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 1 1 0 0 1 0 0 0 ⋄ (~0∊×∘|≤ ≤⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 1 0 0 0 0 0 0 ⋄ (~0∊×∘|≤ ≤⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 1 1 0 1 1 0 0 1) 1 (3 3⍴0 1 1 0 0 1 0 0 0) 1 (3 3⍴0 0 1 0 0 0 0 0 0) 1

⍝ aplcart/tt.tsv:884 — Is Bm an upper triangular matrix with diagonal?
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (~0∊×∘|≤ ≤⌝ ⍨∘⍳∘≢)Bm
0

⍝ aplcart/tt.tsv:885 — Is Bm an upper triangular matrix without diagonal?; nested displayed values saved once in evaluation order and returned together
(r1←3 3⍴1 1 1 1 1 1 1 1 1 ⋄ (~0∊×∘|≤ <⌝ ⍨∘⍳∘≢) r1 ⋄ r3←3 3⍴1 1 1 0 1 1 0 0 1 ⋄ (~0∊×∘|≤ <⌝ ⍨∘⍳∘≢) r3 ⋄ r5←3 3⍴0 1 1 0 0 1 0 0 0 ⋄ (~0∊×∘|≤ <⌝ ⍨∘⍳∘≢) r5 ⋄ r7←3 3⍴0 0 1 0 0 0 0 0 0 ⋄ (~0∊×∘|≤ <⌝ ⍨∘⍳∘≢) r7)
(3 3⍴1 1 1 1 1 1 1 1 1) 0 (3 3⍴1 1 1 0 1 1 0 0 1) 0 (3 3⍴0 1 1 0 0 1 0 0 0) 1 (3 3⍴0 0 1 0 0 0 0 0 0) 1

⍝ aplcart/tt.tsv:886 — Is Bm an upper triangular matrix without diagonal?
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (~0∊×∘|≤ <⌝ ⍨∘⍳∘≢)Bm
0

⍝ aplcart/tt.tsv:887 — Is D entirely ASCII-only
D←'Abc 19 Σς!'  ⋄ (∧/127≥•UCS∘∊)D   ⍝ 0

⍝ aplcart/tt.tsv:888 — Is D entirely ASCII-only
D←'Abc 19 Σς!'  ⋄ (∧/127≥•UCS∘∊)D   ⍝ 0

⍝ aplcart/tt.tsv:897 — Is Dv a palindrome?
((⌽≡⊢) 'racecar' ⋄ (⌽≡⊢) 'carrace')   ⍝ 1 0

⍝ aplcart/tt.tsv:898 — Is Dv a palindrome?
Dv←'racecar' ⋄ (⌽≡⊢)Dv   ⍝ 1

⍝ aplcart/tt.tsv:903 — Is Dv a valid Finnish social security number? (10=≢Dv); Concrete checksum recipe with matching and mismatching check characters; independent modulo-31 checksum T and Dyalog result 1 0. No external identity lookup
valid←(⊢/=(•D,•A~'GIOQ')⊃⍨1+31|∘⍎9∘↑) ⋄ valid¨'131052308T' '131052308A'
1 0

⍝ aplcart/tt.tsv:904 — Is Dv a valid Finnish social security number? (10=≢Dv); Concrete checksum recipe with matching and mismatching check characters; independent modulo-31 checksum T and Dyalog result 1 0. No external identity lookup
valid←(⊢/=(•D,•A~'GIOQ')⊃⍨1+31|∘⍎9∘↑) ⋄ valid¨'131052308T' '131052308A'
1 0

⍝ aplcart/tt.tsv:919 — Is J (YYYY) a leap year?
J←1900 2000 2024 2025 ⋄ (0≠.=400 100 4 |⌝ ⊢)J
0 1 1 0

⍝ aplcart/tt.tsv:920 — Is J (YYYY) a leap year?
J←1900 2000 2024 2025 ⋄ (0≠.=400 100 4 |⌝ ⊢)J
0 1 1 0

⍝ aplcart/tt.tsv:921 — Is Js a deficient number?; Reuse concrete inputs from aplcart/table.tsv:1384; execute this alternate recipe independently
Js←8 ⋄ (+⍨>1⊥∘∪⊢∨⍳)Js   ⍝ 1

⍝ aplcart/tt.tsv:922 — Is Js a deficient number?; Reuse concrete inputs from aplcart/table.tsv:1384; execute this alternate recipe independently
Js←8 ⋄ (+⍨>1⊥∘∪⊢∨⍳)Js   ⍝ 1

⍝ aplcart/tt.tsv:923 — Is Js a perfect number?; Reuse concrete inputs from aplcart/table.tsv:1383; execute this alternate recipe independently
Js←6 ⋄ (+⍨=1⊥∘∪⊢∨⍳)Js   ⍝ 1

⍝ aplcart/tt.tsv:924 — Is Js a perfect number?; Reuse concrete inputs from aplcart/table.tsv:1383; execute this alternate recipe independently
Js←6 ⋄ (+⍨=1⊥∘∪⊢∨⍳)Js   ⍝ 1

⍝ aplcart/tt.tsv:925 — Is Js a quasiperfect number? (none are known); Reuse concrete inputs from aplcart/table.tsv:1430; execute this alternate recipe independently
Js←6 ⋄ (+⍨=¯1+1⊥∘∪⊢∨⍳)Js   ⍝ 0

⍝ aplcart/tt.tsv:926 — Is Js a quasiperfect number? (none are known); Reuse concrete inputs from aplcart/table.tsv:1430; execute this alternate recipe independently
Js←6 ⋄ (+⍨=¯1+1⊥∘∪⊢∨⍳)Js   ⍝ 0

⍝ aplcart/tt.tsv:927 — Is Js an abundant number?; Reuse concrete inputs from aplcart/table.tsv:1382; execute this alternate recipe independently
Js←12 ⋄ (+⍨<1⊥∘∪⊢∨⍳)Js   ⍝ 1

⍝ aplcart/tt.tsv:928 — Is Js an abundant number?; Reuse concrete inputs from aplcart/table.tsv:1382; execute this alternate recipe independently
Js←12 ⋄ (+⍨<1⊥∘∪⊢∨⍳)Js   ⍝ 1

⍝ aplcart/tt.tsv:929 — Is Js an almost perfect number?; Reuse concrete inputs from aplcart/table.tsv:1429; execute this alternate recipe independently
Js←8 ⋄ (+⍨=1+1⊥∘∪⊢∨⍳)Js   ⍝ 1

⍝ aplcart/tt.tsv:930 — Is Js an almost perfect number?; Reuse concrete inputs from aplcart/table.tsv:1429; execute this alternate recipe independently
Js←8 ⋄ (+⍨=1+1⊥∘∪⊢∨⍳)Js   ⍝ 1

⍝ aplcart/tt.tsv:931 — Is Ms in range 1…Ns?
27 ∊∘⍳ 50   ⍝ 1

⍝ aplcart/tt.tsv:932 — Is Ms in range 1…Ns?
Ms←2 ⋄ Ns←3 ⋄ Ms∊∘⍳Ns   ⍝ 1

⍝ aplcart/tt.tsv:933 — Is N Non-decreasing?
((⍳∘≢≡⍋) 31 41 59 26 ⋄ (⍳∘≢≡⍋) 31 41 59 265 ⋄ (⍳∘≢≡⍋) 27 1828 1828 4590)
0 1 1

⍝ aplcart/tt.tsv:934 — Is N Non-decreasing?
N←1 1 2 ⋄ (⍳∘≢≡⍋)N   ⍝ 1

⍝ aplcart/tt.tsv:935 — Is N Non-increasing?
((⍳∘≢≡⍒) 314 159 265 ⋄ (⍳∘≢≡⍒) 314 159 26 5 ⋄ (⍳∘≢≡⍒) 2700 1828 1828 459)
0 1 1

⍝ aplcart/tt.tsv:936 — Is N Non-increasing?
N←3 2 2 ⋄ (⍳∘≢≡⍒)N   ⍝ 1

⍝ aplcart/tt.tsv:937 — Is N Strictly Decreasing?
((⍳∘≢≡⌽∘⍋) 314 159 265 ⋄ (⍳∘≢≡⌽∘⍋) 314 159 26 5 ⋄ (⍳∘≢≡⌽∘⍋) 2700 1828 1828 459)
0 1 0

⍝ aplcart/tt.tsv:938 — Is N Strictly Decreasing?
N←3 2 1 ⋄ (⍳∘≢≡⌽∘⍋)N   ⍝ 1

⍝ aplcart/tt.tsv:939 — Is N Strictly Increasing?
((⍳∘≢≡⌽∘⍒) 31 41 59 26 ⋄ (⍳∘≢≡⌽∘⍒) 31 41 59 265 ⋄ (⍳∘≢≡⌽∘⍒) 27 1828 1828 4590)
0 1 0

⍝ aplcart/tt.tsv:940 — Is N Strictly Increasing?
N←1 2 3 ⋄ (⍳∘≢≡⌽∘⍒)N   ⍝ 1

⍝ aplcart/tt.tsv:941 — Is N complex?
N←1 2j3 0j1 ⋄ (⊢≠+)N   ⍝ 0 1 1

⍝ aplcart/tt.tsv:942 — Is N complex?
N←1 2j3 0j1 ⋄ (⊢≠+)N   ⍝ 0 1 1

⍝ aplcart/tt.tsv:943 — Is N integer?
N←1 1.5 2j3 ⋄ (⌊=⊢)N   ⍝ 1 0 1

⍝ aplcart/tt.tsv:944 — Is N integer?
N←1 1.5 2j3 ⋄ (⌊=⊢)N   ⍝ 1 0 1

⍝ aplcart/tt.tsv:945 — Is N real?
N←1 2j3 0j1 ⋄ (⊢=+)N   ⍝ 1 0 0

⍝ aplcart/tt.tsv:946 — Is N real?
N←1 2j3 0j1 ⋄ (⊢=+)N   ⍝ 1 0 0

⍝ aplcart/tt.tsv:947 — Is Nm a Hermitian matrix?; optional {X} instantiated as dyadic use
Nm←2 2⍴1 2j3 2j¯3 4 ⋄ (⍉≡+)Nm   ⍝ 1

⍝ aplcart/tt.tsv:948 — Is Nm a Hermitian matrix?; optional {X} instantiated as dyadic use
Nm←2 2⍴1 2j3 2j¯3 4 ⋄ (⍉≡+)Nm   ⍝ 1

⍝ aplcart/tt.tsv:949 — Is Nm a Unitary matrix?
Nm←2 2⍴0 0j1 0j1 0 ⋄ (⌹≡⍉∘+)Nm   ⍝ 1

⍝ aplcart/tt.tsv:950 — Is Nm a Unitary matrix?
Nm←2 2⍴0 0j1 0j1 0 ⋄ (⌹≡⍉∘+)Nm   ⍝ 1

⍝ aplcart/tt.tsv:951 — Is Nm an Orthogonal matrix?; optional {X} instantiated as dyadic use
Nm←2 2⍴0 1 ¯1 0 ⋄ (⍉≡⌹)Nm   ⍝ 1

⍝ aplcart/tt.tsv:952 — Is Nm an Orthogonal matrix?; optional {X} instantiated as dyadic use
Nm←2 2⍴0 1 ¯1 0 ⋄ (⍉≡⌹)Nm   ⍝ 1

⍝ aplcart/tt.tsv:953 — Is Ns a prime?
((2=0+.=⍳|⊢) 6 ⋄ (2=0+.=⍳|⊢) 7 ⋄ (2=0+.=⍳|⊢) 8 ⋄ (2=0+.=⍳|⊢) 9 ⋄ (2=0+.=⍳|⊢) 10 ⋄ (2=0+.=⍳|⊢) 11)
0 1 0 0 0 1

⍝ aplcart/tt.tsv:954 — Is Ns a prime?; Reuse concrete inputs from aplcart/table.tsv:1275; execute this alternate recipe independently
Ns←7 ⋄ (2=0+.=⍳|⊢)Ns   ⍝ 1

⍝ aplcart/tt.tsv:955 — Is Nv a permutation vector?; Reuse concrete inputs from aplcart/table.tsv:875; execute this alternate recipe independently
Nv←3 1 2 ⋄ (⊢≡∘⍋⍋)Nv   ⍝ 1

⍝ aplcart/tt.tsv:956 — Is Nv a permutation vector?; Reuse concrete inputs from aplcart/table.tsv:875; execute this alternate recipe independently
Nv←3 1 2 ⋄ (⊢≡∘⍋⍋)Nv   ⍝ 1

⍝ aplcart/tt.tsv:957 — Is X a Subarray of Y?
X←1 2 ⋄ Y←3 1 2 4 ⋄ X(≡∨1∊⍷)Y   ⍝ 1

⍝ aplcart/tt.tsv:958 — Is X a Subarray of Y?
X←1 2 ⋄ Y←3 1 2 4 ⋄ X(≡∨1∊⍷)Y   ⍝ 1

⍝ aplcart/tt.tsv:959 — Is Xv a Subset of Yv?; optional {X} instantiated as dyadic use
Xv←1 2 ⋄ Yv←3 2 1 ⋄ Xv(∧/∊)Yv   ⍝ 1

⍝ aplcart/tt.tsv:960 — Is Xv a Subset of Yv?; optional {X} instantiated as dyadic use
Xv←1 2 ⋄ Yv←3 2 1 ⋄ Xv(∧/∊)Yv   ⍝ 1

⍝ aplcart/tt.tsv:961 — Is Xv a Superset of Yv?
Xv←1 2 3 4 ⋄ Yv←2 3 ⋄ Xv(∧/∊⍨)Yv   ⍝ 1

⍝ aplcart/tt.tsv:962 — Is Xv a Superset of Yv?
Xv←1 2 3 4 ⋄ Yv←2 3 ⋄ Xv(∧/∊⍨)Yv   ⍝ 1

⍝ aplcart/tt.tsv:967 — Is Y a Nested Array?; optional {X} instantiated as dyadic use
Y←(1 2⋄ 3 4) ⋄ (⊂≢⊆)Y   ⍝ 1

⍝ aplcart/tt.tsv:968 — Is Y a Nested Array?; optional {X} instantiated as dyadic use
Y←(1 2⋄ 3 4) ⋄ (⊂≢⊆)Y   ⍝ 1

⍝ aplcart/tt.tsv:969 — Is Y a Scalar?
Y←3 1 3 2 ⋄ (⍬≡⍴)Y   ⍝ 0

⍝ aplcart/tt.tsv:970 — Is Y a Scalar?
Y←3 1 3 2 ⋄ (⍬≡⍴)Y   ⍝ 0

⍝ aplcart/tt.tsv:971 — Is Y a Simple Array?; optional {X} instantiated as dyadic use
Y←2 3⍴⍳6 ⋄ (⊂≡⊆)Y   ⍝ 1

⍝ aplcart/tt.tsv:972 — Is Y a Simple Array?; optional {X} instantiated as dyadic use
Y←2 3⍴⍳6 ⋄ (⊂≡⊆)Y   ⍝ 1

⍝ aplcart/tt.tsv:973 — Is Y a Simple Scalar?
Y←42 ⋄ (0=≡)Y   ⍝ 1

⍝ aplcart/tt.tsv:974 — Is Y a Simple Scalar?
Y←42 ⋄ (0=≡)Y   ⍝ 1

⍝ aplcart/tt.tsv:975 — Is Y a Singleton?
Y←1 1 1⍴42 ⋄ (1=×/∘⍴)Y   ⍝ 1

⍝ aplcart/tt.tsv:976 — Is Y a Singleton?
Y←1 1 1⍴42 ⋄ (1=×/∘⍴)Y   ⍝ 1

⍝ aplcart/tt.tsv:977 — Is Y a simple character array?; Reuse concrete inputs from aplcart/table.tsv:721; execute this alternate recipe independently
Y←2 3⍴'abcdef'  ⋄ (⍕≡⊢)Y   ⍝ 1

⍝ aplcart/tt.tsv:978 — Is Y a simple character array?; Reuse concrete inputs from aplcart/table.tsv:721; execute this alternate recipe independently
Y←2 3⍴'abcdef'  ⋄ (⍕≡⊢)Y   ⍝ 1

⍝ aplcart/tt.tsv:979 — Is Y a vector?
((1=≢∘⍴) 'Hello' ⋄ (1=≢∘⍴) 'H' ⋄ (1=≢∘⍴) '')
1 0 1

⍝ aplcart/tt.tsv:980 — Is Y a vector?
Y←3 1 3 2 ⋄ (1=≢∘⍴)Y   ⍝ 1

⍝ aplcart/tt.tsv:981 — Is Y an Empty Array?
Y←2 0 3⍴0 ⋄ (0∊⍴)Y   ⍝ 1

⍝ aplcart/tt.tsv:982 — Is Y an Empty Array?
Y←2 0 3⍴0 ⋄ (0∊⍴)Y   ⍝ 1

⍝ aplcart/tt.tsv:989 — Is Y outside the range ( 1⌷X , 2⌷X )
(1 3 (6≠⍳+3×⍸) 0 ⋄ 1 3 (6≠⍳+3×⍸) 1 ⋄ 1 3 (6≠⍳+3×⍸) 2 ⋄ 1 3 (6≠⍳+3×⍸) 3 ⋄ 1 3 (6≠⍳+3×⍸) 4)
1 1 0 1 1

⍝ aplcart/tt.tsv:990 — Is Y outside the range ( 1⌷X , 2⌷X )
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X(6≠⍳+3×⍸)Y   ⍝ 1 1 0 1 1

⍝ aplcart/tt.tsv:991 — Is Y outside the range ( 1⌷X , 2⌷X ]
(1 3 (1≠6 9⍸⍳+3×⍸) 0 ⋄ 1 3 (1≠6 9⍸⍳+3×⍸) 1 ⋄ 1 3 (1≠6 9⍸⍳+3×⍸) 2 ⋄ 1 3 (1≠6 9⍸⍳+3×⍸) 3 ⋄ 1 3 (1≠6 9⍸⍳+3×⍸) 4)
1 1 0 0 1

⍝ aplcart/tt.tsv:992 — Is Y outside the range ( 1⌷X , 2⌷X ]
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X(1≠6 9⍸⍳+3×⍸)Y   ⍝ 1 1 0 0 1

⍝ aplcart/tt.tsv:993 — Is Y outside the range [ 1⌷X , 2⌷X )
(1 3 (1≠⍸) 0 ⋄ 1 3 (1≠⍸) 1 ⋄ 1 3 (1≠⍸) 2 ⋄ 1 3 (1≠⍸) 3 ⋄ 1 3 (1≠⍸) 4)
1 0 0 1 1

⍝ aplcart/tt.tsv:994 — Is Y outside the range [ 1⌷X , 2⌷X )
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X(1≠⍸)Y   ⍝ 1 0 0 1 1

⍝ aplcart/tt.tsv:995 — Is Y outside the range [ 1⌷X , 2⌷X ]
(1 3 (1≠4 9⍸⍳+3×⍸) 0 ⋄ 1 3 (1≠4 9⍸⍳+3×⍸) 1 ⋄ 1 3 (1≠4 9⍸⍳+3×⍸) 2 ⋄ 1 3 (1≠4 9⍸⍳+3×⍸) 3 ⋄ 1 3 (1≠4 9⍸⍳+3×⍸) 4)
1 0 0 0 1

⍝ aplcart/tt.tsv:996 — Is Y outside the range [ 1⌷X , 2⌷X ]
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X(1≠4 9⍸⍳+3×⍸)Y   ⍝ 1 0 0 0 1

⍝ aplcart/tt.tsv:997 — Is Y within the range ( 1⌷X , 2⌷X )
(1 3 (6=⍳+3×⍸) 0 ⋄ 1 3 (6=⍳+3×⍸) 1 ⋄ 1 3 (6=⍳+3×⍸) 2 ⋄ 1 3 (6=⍳+3×⍸) 3 ⋄ 1 3 (6=⍳+3×⍸) 4)
0 0 1 0 0

⍝ aplcart/tt.tsv:998 — Is Y within the range ( 1⌷X , 2⌷X )
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X(6=⍳+3×⍸)Y   ⍝ 0 0 1 0 0

⍝ aplcart/tt.tsv:999 — Is Y within the range ( 1⌷X , 2⌷X ]
(1 3 (1=6 9⍸⍳+3×⍸) 0 ⋄ 1 3 (1=6 9⍸⍳+3×⍸) 1 ⋄ 1 3 (1=6 9⍸⍳+3×⍸) 2 ⋄ 1 3 (1=6 9⍸⍳+3×⍸) 3 ⋄ 1 3 (1=6 9⍸⍳+3×⍸) 4)
0 0 1 1 0

⍝ aplcart/tt.tsv:1000 — Is Y within the range ( 1⌷X , 2⌷X ]
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X(1=6 9⍸⍳+3×⍸)Y   ⍝ 0 0 1 1 0

⍝ aplcart/tt.tsv:1001 — Is Y within the range [ 1⌷X , 2⌷X )
(1 3 (1=⍸) 0 ⋄ 1 3 (1=⍸) 1 ⋄ 1 3 (1=⍸) 2 ⋄ 1 3 (1=⍸) 3 ⋄ 1 3 (1=⍸) 4)
0 1 1 0 0

⍝ aplcart/tt.tsv:1002 — Is Y within the range [ 1⌷X , 2⌷X )
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X(1=⍸)Y   ⍝ 0 1 1 0 0

⍝ aplcart/tt.tsv:1003 — Is Y within the range [ 1⌷X , 2⌷X ]
(1 3 (1=4 9⍸⍳+3×⍸) 0 ⋄ 1 3 (1=4 9⍸⍳+3×⍸) 1 ⋄ 1 3 (1=4 9⍸⍳+3×⍸) 2 ⋄ 1 3 (1=4 9⍸⍳+3×⍸) 3 ⋄ 1 3 (1=4 9⍸⍳+3×⍸) 4)
0 1 1 1 0

⍝ aplcart/tt.tsv:1004 — Is Y within the range [ 1⌷X , 2⌷X ]
X←2 4 ⋄ Y←1 2 3 4 5 ⋄ X(1=4 9⍸⍳+3×⍸)Y   ⍝ 0 1 1 1 0

⍝ aplcart/tt.tsv:1005 — Is Ym a square matrix?
((=/⍴) ⊃'abc' 'def' ⋄ (=/⍴) ⊃'ab' 'cd')   ⍝ 0 1

⍝ aplcart/tt.tsv:1006 — Is Ym a square matrix?
Ym←2 3⍴⍳6 ⋄ (=/⍴)Ym   ⍝ 0

⍝ aplcart/tt.tsv:1007 — Is Ym anti-symmetric?; optional {X} instantiated as dyadic use
Ym←2 2⍴0 ¯2 2 0 ⋄ (-≡⍉)Ym   ⍝ 1

⍝ aplcart/tt.tsv:1008 — Is Ym anti-symmetric?; optional {X} instantiated as dyadic use
Ym←2 2⍴0 ¯2 2 0 ⋄ (-≡⍉)Ym   ⍝ 1

⍝ aplcart/tt.tsv:1009 — Is Ym symmetric?; Reuse concrete inputs from aplcart/table.tsv:699; execute this alternate recipe independently
Ym←3 3⍴1 2 3 2 4 5 3 5 6 ⋄ (⍉≡⊢)Ym   ⍝ 1

⍝ aplcart/tt.tsv:1010 — Is Ym symmetric?; Reuse concrete inputs from aplcart/table.tsv:699; execute this alternate recipe independently
Ym←3 3⍴1 2 3 2 4 5 3 5 6 ⋄ (⍉≡⊢)Ym   ⍝ 1

⍝ aplcart/tt.tsv:1011 — Is string Dv a member of list of strings C
C←'cat' 'dog' 'bird' ⋄ Dv←'dog' ⋄ C∊⍨∘⊂Dv
1

⍝ aplcart/tt.tsv:1012 — Is string Dv a member of list of strings C
C←'cat' 'dog' 'bird' ⋄ Dv←'dog' ⋄ C∊⍨∘⊂Dv
1

⍝ aplcart/tt.tsv:1013 — Is'th moment of Nv
Is←2 ⋄ Nv←1 2 4 ⋄ Is(⊢∘≢÷⍨1⊥⊣*⍨⊢-⊢∘≢÷⍨1⊥⊢)Nv
1.555555555555556

⍝ aplcart/tt.tsv:1014 — Is'th moment of Nv
Is←2 ⋄ Nv←1 2 4 ⋄ Is(⊢∘≢÷⍨1⊥⊣*⍨⊢-⊢∘≢÷⍨1⊥⊢)Nv
1.555555555555556

⍝ aplcart/tt.tsv:1015 — Is'th number in the Aliqout sequence for Js; Reuse concrete inputs from aplcart/table.tsv:1494; execute this alternate recipe independently
Is←2 ⋄ Js←6 ⋄ Is{(+/∘∪⊢∨⍳)⍣⍺⊢⍵}Js   ⍝ 28

⍝ aplcart/tt.tsv:1016 — Is'th number in the Aliqout sequence for Js; Reuse concrete inputs from aplcart/table.tsv:1494; execute this alternate recipe independently
Is←2 ⋄ Js←6 ⋄ Is{(+/∘∪⊢∨⍳)⍣⍺⊢⍵}Js   ⍝ 28

⍝ aplcart/tt.tsv:1017 — Is-largest (default: the largest) major cell of Y
names ← 'Bob' 'Dan' 'Cal' 'Abe' ⋄ (2 {⍺←1 ⋄ (⍺⊃⍒⍵)⌷⍵} names ⋄ {⍺←1 ⋄ (⍺⊃⍒⍵)⌷⍵} names)
('Cal' ⋄ 'Dan')

⍝ aplcart/tt.tsv:1018 — Is-largest (default: the largest) major cell of Y
Is←2 ⋄ Y←3 2⍴1 2 5 6 3 4 ⋄ Is{⍺←1 ⋄ (⍺⊃⍒⍵)⌷⍵}Y
3 4

⍝ aplcart/tt.tsv:1019 — Is-norm
2 (⊣*∘÷⍨1⊥*⍨∘|) 3 4   ⍝ 5

⍝ aplcart/tt.tsv:1020 — Is-norm
Is←2 ⋄ N←3 4 ⋄ Is(⊣*∘÷⍨1⊥*⍨∘|)N   ⍝ 5

⍝ aplcart/tt.tsv:1021 — Is-smallest (default: the smallest) major cell of Y
names ← 'Bob' 'Dan' 'Cal' 'Abe' ⋄ (2 {⍺←1 ⋄ (⍺⊃⍋⍵)⌷⍵} names ⋄ {⍺←1 ⋄ (⍺⊃⍋⍵)⌷⍵} names)
('Bob' ⋄ 'Abe')

⍝ aplcart/tt.tsv:1022 — Is-smallest (default: the smallest) major cell of Y
Is←2 ⋄ Y←3 2⍴1 2 5 6 3 4 ⋄ Is{⍺←1 ⋄ (⍺⊃⍋⍵)⌷⍵}Y
3 4

⍝ aplcart/tt.tsv:1023 — Is-wise rolling average
Is←2 ⋄ N←2 4 6 8 10 ⋄ (+/Is↕N)÷Is   ⍝ 3 5 7 9

⍝ aplcart/tt.tsv:1024 — Is-wise rolling average
Is←2 ⋄ N←2 4 6 8 10 ⋄ (+/Is↕N)÷Is   ⍝ 3 5 7 9

⍝ aplcart/tt.tsv:1025 — Iv copies of Y
3 (⊃⍴∘⊂) 'abc'   ⍝ 3 3⍴'abcabcabc'

⍝ aplcart/tt.tsv:1026 — Iv copies of Y
Iv←2 1 ⋄ Y←3 1 3 2 ⋄ Iv(⊃⍴∘⊂)Y   ⍝ 2 1 4⍴3 1 3 2 3 1 3 2

⍝ aplcart/tt.tsv:1027 — J Hook: Y f g Y when monadic and X f g Y when dyadic; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ f←+ ⋄ g←⌽ ⋄ Y←3 1 3 2 ⋄ X f∘g⍨⍨Y
5 4 3 4

⍝ aplcart/tt.tsv:1028 — J Hook: Y f g Y when monadic and X f g Y when dyadic; optional {X} instantiated as dyadic use
X←3 1 2 1 ⋄ f←+ ⋄ g←⌽ ⋄ Y←3 1 3 2 ⋄ X f∘g⍨⍨Y
5 4 3 4

⍝ aplcart/tt.tsv:1029 — J is Even; optional {X} instantiated as dyadic use
J←4 9 16 ⋄ (~2∘|)J   ⍝ 1 0 1

⍝ aplcart/tt.tsv:1030 — J is Even; optional {X} instantiated as dyadic use
J←4 9 16 ⋄ (~2∘|)J   ⍝ 1 0 1

⍝ aplcart/tt.tsv:1031 — Jacobsthal number
Js←4 ⋄ (3÷⍨2∘*-¯1∘*)Js   ⍝ 5

⍝ aplcart/tt.tsv:1032 — Jacobsthal number
Js←4 ⋄ (3÷⍨2∘*-¯1∘*)Js   ⍝ 5

⍝ aplcart/tt.tsv:1033 — Jacobsthal-Lucas number
Js←4 ⋄ (2∘*+¯1∘*)Js   ⍝ 17

⍝ aplcart/tt.tsv:1034 — Jacobsthal-Lucas number
Js←4 ⋄ (2∘*+¯1∘*)Js   ⍝ 17

⍝ aplcart/tt.tsv:1035 — Join (⍪) planes of rank 3 array Y to form a single matrix
Y←2 3 4⍴⍳24 ⋄ (⍉⍪∘⍉)Y
6 4⍴1 2 3 4 13 14 15 16 5 6 7 8 17 18 19 20 9 10 11 12 21 22 23 24

⍝ aplcart/tt.tsv:1036 — Join (⍪) planes of rank 3 array Y to form a single matrix
Y←2 3 4⍴⍳24 ⋄ (⍉⍪∘⍉)Y
6 4⍴1 2 3 4 13 14 15 16 5 6 7 8 17 18 19 20 9 10 11 12 21 22 23 24

⍝ aplcart/tt.tsv:1037 — Join array of arrays horizontally; optional {X} instantiated as dyadic use
Yv←(2 2⍴⍳4⋄ 2 3⍴⍳6) ⋄ (,/)Yv   ⍝ 2 5⍴1 2 1 2 3 3 4 4 5 6

⍝ aplcart/tt.tsv:1038 — Join array of arrays horizontally; optional {X} instantiated as dyadic use
Yv←(2 2⍴⍳4⋄ 2 3⍴⍳6) ⋄ (,/)Yv   ⍝ 2 5⍴1 2 1 2 3 3 4 4 5 6

⍝ aplcart/tt.tsv:1039 — Join array of arrays vertically; optional {X} instantiated as dyadic use
Y←(2 3⍴⍳6⋄ 1 3⍴7 8 9) ⋄ (⍪⌿)Y   ⍝ 3 3⍴1 2 3 4 5 6 7 8 9

⍝ aplcart/tt.tsv:1040 — Join array of arrays vertically; optional {X} instantiated as dyadic use
Y←(2 3⍴⍳6⋄ 1 3⍴7 8 9) ⋄ (⍪⌿)Y   ⍝ 3 3⍴1 2 3 4 5 6 7 8 9

⍝ aplcart/tt.tsv:1041 — Join corresponding items; dfns display import/wrappers omitted to test underlying arrays
(5 6 7 ,¨ 10 11 12 ⋄ 5 6 7 ,¨ 15)
((5 10 ⋄ 6 11 ⋄ 7 12) ⋄ (5 15 ⋄ 6 15 ⋄ 7 15))

⍝ aplcart/tt.tsv:1042 — Join corresponding items
X←3 1 2 1 ⋄ Y←3 1 3 2 ⋄ X,¨Y   ⍝ (3 3 ⋄ 1 1 ⋄ 2 3 ⋄ 1 2)

⍝ aplcart/tt.tsv:1043 — Join digits of strictly positive integers into a single integer
(10⊥∘∊10∘⊥⍣¯1¨) 31 1 27   ⍝ 31127

⍝ aplcart/tt.tsv:1044 — Join digits of strictly positive integers into a single integer
Jv←12 3 456 ⋄ (⊢⊥⍨10*∘⌊1+10∘⍟)Jv   ⍝ 123456

⍝ aplcart/tt.tsv:1045 — Join lines with line feed (LF)
Cv←'first'  ⋄ Dv←'Hello, world! 123'  ⋄ Cv(⊣,(•UCS 10),⊢)Dv
•UCS 102 105 114 115 116 10 72 101 108 108 111 44 32 119 111 114 108 100 33 32 49 50 51

⍝ aplcart/tt.tsv:1046 — Join lines with line feed (LF)
Cv←'first'  ⋄ Dv←'Hello, world! 123'  ⋄ Cv(⊣,(•UCS 10),⊢)Dv
•UCS 102 105 114 115 116 10 72 101 108 108 111 44 32 119 111 114 108 100 33 32 49 50 51

⍝ aplcart/tt.tsv:1047 — Join magnitude M and radians N to form complex; optional {X} instantiated as dyadic use
M←1 2 3 ⋄ N←0 0.5 1 ⋄ M(⊣×¯12○⊢)N
1 1.755165123780746j0.958851077208406 1.620906917604419j2.524412954423689

⍝ aplcart/tt.tsv:1048 — Join magnitude M and radians N to form complex; optional {X} instantiated as dyadic use
M←1 2 3 ⋄ N←0 0.5 1 ⋄ M(⊣×¯12○⊢)N
1 1.755165123780746j0.958851077208406 1.620906917604419j2.524412954423689

⍝ aplcart/tt.tsv:1049 — Join magnitude and radians major cells by removing leading axis; Reuse concrete inputs from aplcart/table.tsv:864; execute this alternate recipe independently
N←2 3⍴2 3 4 0 0.5 1 ⋄ (⊣×¯12○⊢)⌿N
2 2.632747685671118j1.438276615812609 2.161209223472559j3.365883939231586

⍝ aplcart/tt.tsv:1050 — Join magnitude and radians major cells by removing leading axis; Reuse concrete inputs from aplcart/table.tsv:864; execute this alternate recipe independently
N←2 3⍴2 3 4 0 0.5 1 ⋄ (⊣×¯12○⊢)⌿N
2 2.632747685671118j1.438276615812609 2.161209223472559j3.365883939231586

⍝ aplcart/tt.tsv:1051 — Join real and imaginary major cells by removing leading axis; Reuse concrete inputs from aplcart/table.tsv:863; execute this alternate recipe independently
N←2 3⍴1 2 3 4 5 6 ⋄ (⊣+¯11○⊢)⌿N   ⍝ 1j4 2j5 3j6

⍝ aplcart/tt.tsv:1052 — Join real and imaginary major cells by removing leading axis; Reuse concrete inputs from aplcart/table.tsv:863; execute this alternate recipe independently
N←2 3⍴1 2 3 4 5 6 ⋄ (⊣+¯11○⊢)⌿N   ⍝ 1j4 2j5 3j6

⍝ aplcart/tt.tsv:1053 — Join real part M and imaginary part N to form complex; optional {X} instantiated as dyadic use
M←1 2 3 ⋄ N←4 5 6 ⋄ M(⊣+¯11○⊢)N   ⍝ 1j4 2j5 3j6

⍝ aplcart/tt.tsv:1054 — Join real part M and imaginary part N to form complex; optional {X} instantiated as dyadic use
M←1 2 3 ⋄ N←4 5 6 ⋄ M(⊣+¯11○⊢)N   ⍝ 1j4 2j5 3j6

⍝ aplcart/tt.tsv:1055 — Join scalar elements of vector Yv with separator Xs
Xs←9 ⋄ Yv←4 5 6 ⋄ Xs(1↓∘,,⍤0)Yv   ⍝ 4 9 5 9 6

⍝ aplcart/tt.tsv:1056 — Join scalar elements of vector Yv with separator Xs
Xs←9 ⋄ Yv←4 5 6 ⋄ Xs(1↓∘,,⍤0)Yv   ⍝ 4 9 5 9 6

⍝ aplcart/tt.tsv:1057 — Joining date YYYY M D to packed YYYYMMDD integer
100∘⊥ 1969 7 21   ⍝ 19690721

⍝ aplcart/tt.tsv:1058 — Joining date YYYY M D to packed YYYYMMDD integer
Jv←2026 9 18 ⋄ 100∘⊥Jv   ⍝ 20260918

⍝ aplcart/tt.tsv:1059 — Js spokes of unit wheel
Js←4 ⋄ (*∘π0J2×⊢÷⍨1+⍳)Js
¯1 0j¯1 1 0j1

⍝ aplcart/tt.tsv:1060 — Js spokes of unit wheel
Js←4 ⋄ (*∘π0J2×⊢÷⍨1+⍳)Js
¯1 0j¯1 1 0j1

⍝ aplcart/tt.tsv:1061 — Js-bit reflected Gray code
Js←3 ⋄ (2∘*↑(⌽2*⍳)⊖∘⍉⍴∘2⊤2/∘⍳2∘*)Js
8 3⍴1 1 0 1 1 0 1 0 1 1 0 1 1 0 0 1 0 0 0 1 1 0 1 1

⍝ aplcart/tt.tsv:1062 — Js-bit reflected Gray code
Js←3 ⋄ (2∘*↑(⌽2*⍳)⊖∘⍉⍴∘2⊤2/∘⍳2∘*)Js
8 3⍴1 1 0 1 1 0 1 0 1 1 0 1 1 0 0 1 0 0 0 1 1 0 1 1

⍝ aplcart/tt.tsv:1063 — Juxtapositioning planes of rank 3 array Y
Y←2 3 4⍴⍳24 ⋄ ((×⌿2 2⍴1,⍴)⍴2 1 3∘⍉)Y
3 8⍴1 2 3 4 13 14 15 16 5 6 7 8 17 18 19 20 9 10 11 12 21 22 23 24

⍝ aplcart/tt.tsv:1064 — Juxtapositioning planes of rank 3 array Y
Y←2 3 4⍴⍳24 ⋄ ((×⌿2 2⍴1,⍴)⍴2 1 3∘⍉)Y
3 8⍴1 2 3 4 13 14 15 16 5 6 7 8 17 18 19 20 9 10 11 12 21 22 23 24

⍝ aplcart/tt.tsv:1065 — Kronecker product
A ← 2 3⍴1 ¯4 7,¯2 3 3 ⋄ B ← 4 4⍴8 ¯9 ¯6 5,1 ¯3 ¯4 7,2 8 ¯8 ¯3,1 2 ¯5 ¯1 ⋄ A {,⍤2,[⍳2]1 3 2 4⍉⍺ ×⌝ ⍵} B
8 12⍴8 ¯9 ¯6 5 ¯32 36 24 ¯20 56 ¯63 ¯42 35 1 ¯3 ¯4 7 ¯4 12 16 ¯28 7 ¯21 ¯28 49 2 8 ¯8 ¯3 ¯8 ¯32 32 12 14 56 ¯56 ¯21 1 2 ¯5 ¯1 ¯4 ¯8 20 4 7 14 ¯35 ¯7 ¯16 18 12 ¯10 24 ¯27 ¯18 15 24 ¯27 ¯18 15 ¯2 6 8 ¯14 3 ¯9 ¯12 21 3 ¯9 ¯12 21 ¯4 ¯16 16 6 6 24 ¯24 ¯9 6 24 ¯24 ¯9 ¯2 ¯4 10 2 3 6 ¯15 ¯3 3 6 ¯15 ¯3

⍝ aplcart/tt.tsv:1066 — Kronecker product
Mm←2 2⍴1 2 3 4 ⋄ Nm←2 3⍴⍳6 ⋄ Mm{,⍤2,[⍳2]1 3 2 4⍉⍺ ×⌝ ⍵}Nm
4 6⍴1 2 3 2 4 6 4 5 6 8 10 12 3 6 9 4 8 12 12 15 18 16 20 24

⍝ aplcart/tt.tsv:1067 — Largest column-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌈⌿|)N   ⍝ 4 5 6

⍝ aplcart/tt.tsv:1068 — Largest column-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌈⌿|)N   ⍝ 4 5 6

⍝ aplcart/tt.tsv:1069 — Largest row-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌈/|)N   ⍝ 3 6

⍝ aplcart/tt.tsv:1070 — Largest row-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌈/|)N   ⍝ 3 6

⍝ aplcart/tt.tsv:1071 — Largest whole multiple of M less than or equal to N (N rounded down to closest smaller multiple of M)
10 (⊢-|) 9 10 11 19 20 21   ⍝ 0 10 10 10 20 20

⍝ aplcart/tt.tsv:1072 — Largest whole multiple of M less than or equal to N (N rounded down to closest smaller multiple of M)
M←3 ⋄ N←¯4 0 4 8 ⋄ M(⊢-|)N   ⍝ ¯6 0 3 6

⍝ aplcart/tt.tsv:1073 — Last column as a column matrix (column vector); optional {X} instantiated as dyadic use
Ym←2 3⍴⍳6 ⋄ (¯1∘↑⍤1)Ym   ⍝ 2 1⍴3 6

⍝ aplcart/tt.tsv:1074 — Last column as a column matrix (column vector); optional {X} instantiated as dyadic use
Ym←2 3⍴⍳6 ⋄ (¯1∘↑⍤1)Ym   ⍝ 2 1⍴3 6

⍝ aplcart/tt.tsv:1075 — Last indices in X of major cells Y, 0 if not found; Reuse concrete inputs from aplcart/table.tsv:1369; execute this alternate recipe independently
X←1 2 3 2 ⋄ Y←2 4 ⋄ X(⍳⍨∘⊖⍨-⍨1+∘≢⊣)Y   ⍝ 4 0

⍝ aplcart/tt.tsv:1076 — Last indices in X of major cells Y, 0 if not found; Reuse concrete inputs from aplcart/table.tsv:1369; execute this alternate recipe independently
X←1 2 3 2 ⋄ Y←2 4 ⋄ X(⍳⍨∘⊖⍨-⍨1+∘≢⊣)Y   ⍝ 4 0

⍝ aplcart/tt.tsv:1077 — Last major cell of numeric array
(0∘⊥ ⍳9 ⋄ 0∘⊥ 3 3⍴⍳9)   ⍝ 9 (7 8 9)

⍝ aplcart/tt.tsv:1078 — Last major cell of numeric array
N←2 3⍴⍳6 ⋄ 0∘⊥N   ⍝ 4 5 6

⍝ aplcart/tt.tsv:1079 — Last ones in each group of ones
B←1 1 0 1 0 1 0 ⋄ ((>/[2]∘(2∘↕))⍪∘0)B   ⍝ 0 1 0 1 0 1 0

⍝ aplcart/tt.tsv:1080 — Last ones in each group of ones
B←1 1 0 1 0 1 0 ⋄ ((>/[2]∘(2∘↕))⍪∘0)B   ⍝ 0 1 0 1 0 1 0

⍝ aplcart/tt.tsv:1081 — Last part of packed numeric code ABBB
IA←1234 5678 ⋄ 1000∘|IA   ⍝ 234 678

⍝ aplcart/tt.tsv:1082 — Last part of packed numeric code ABBB
IA←1234 5678 ⋄ 1000∘|IA   ⍝ 234 678

⍝ aplcart/tt.tsv:1083 — Last row as a row matrix (row vector)
Ym←2 3⍴⍳6 ⋄ (¯1∘↑)Ym   ⍝ 1 3⍴4 5 6

⍝ aplcart/tt.tsv:1084 — Last row as a row matrix (row vector)
Ym←2 3⍴⍳6 ⋄ (¯1∘↑)Ym   ⍝ 1 3⍴4 5 6

⍝ aplcart/tt.tsv:1085 — Leading ones (∧⍀) in each subvector of Bv indicated by Av
Av←1 0 0 1 0 0 ⋄ Bv←1 1 0 1 0 1 ⋄ Av(≥{~≠\⍺\(≠/∘(2∘↕))1,⍺⌿⍵}⊢)Bv
1 1 0 1 0 0

⍝ aplcart/tt.tsv:1086 — Leading ones (∧⍀) in each subvector of Bv indicated by Av
Av←1 0 0 1 0 0 ⋄ Bv←1 1 0 1 0 1 ⋄ Av(≥{~≠\⍺\(≠/∘(2∘↕))1,⍺⌿⍵}⊢)Bv
1 1 0 1 0 0

⍝ aplcart/tt.tsv:1087 — Leftmost neighbouring elements (cyclically)
Y←3 1 3 2 ⋄ ¯1∘⌽Y   ⍝ 2 3 1 3

⍝ aplcart/tt.tsv:1088 — Leftmost neighbouring elements (cyclically)
Y←3 1 3 2 ⋄ ¯1∘⌽Y   ⍝ 2 3 1 3

⍝ aplcart/tt.tsv:1089 — Leftmost neighbouring elements (padding at edge)
Y←2 3⍴⍳6 ⋄ ((¯1↓⊢,⍨1↑0∘⍴)⍤1)Y   ⍝ 2 3⍴0 1 2 0 4 5

⍝ aplcart/tt.tsv:1090 — Leftmost neighbouring elements (padding at edge)
Y←2 3⍴⍳6 ⋄ ((¯1↓⊢,⍨1↑0∘⍴)⍤1)Y   ⍝ 2 3⍴0 1 2 0 4 5

⍝ aplcart/tt.tsv:1091 — Length of edges of pyramid with height and width Mv and length Ns
Mv←3 4 ⋄ Ns←6 ⋄ Mv(⊣,⍥(2*∘÷⍨+.×⍨)⊢÷2⍨)Ns
5 3

⍝ aplcart/tt.tsv:1092 — Length of edges of pyramid with height and width Mv and length Ns
Mv←3 4 ⋄ Ns←6 ⋄ Mv(⊣,⍥(2*∘÷⍨+.×⍨)⊢÷2⍨)Ns
5 3

⍝ aplcart/tt.tsv:1093 — Length of path given as complex points
Nv←0 3 3j4 ⋄ (+/∘|(-/∘(2∘↕)))Nv   ⍝ 7

⍝ aplcart/tt.tsv:1094 — Length of path given as complex points
Nv←0 3 3j4 ⋄ (+/∘|(-/∘(2∘↕)))Nv   ⍝ 7

⍝ aplcart/tt.tsv:1095 — Length of subvectors indicated by Bv (Fast ≢¨⊆⍨Bv)
(≢(⊢-⍨1↓⊢,1+⊣)⍸) 1 0 0 0 1 1 0 0   ⍝ 4 1 3

⍝ aplcart/tt.tsv:1096 — Length of subvectors indicated by Bv (Fast ≢¨⊆⍨Bv)
Bv←1 0 0 1 0 1 ⋄ (≢(⊢-⍨1↓⊢,1+⊣)⍸)Bv   ⍝ 3 2 1

⍝ aplcart/tt.tsv:1097 — Length of vector Nv
Nv←3 4 ⋄ (2*∘÷⍨+.×⍨)Nv   ⍝ 5

⍝ aplcart/tt.tsv:1098 — Length of vector Nv
Nv←3 4 ⋄ (2*∘÷⍨+.×⍨)Nv   ⍝ 5

⍝ aplcart/tt.tsv:1099 — Length to represent J in base I
I←2 3 4 ⋄ J←4 9 16 ⋄ I(⌊1+⍟)J   ⍝ 3 3 3

⍝ aplcart/tt.tsv:1100 — Length to represent J in base I
I←2 3 4 ⋄ J←4 9 16 ⋄ I(⌊1+⍟)J   ⍝ 3 3 3

⍝ aplcart/tt.tsv:1101 — Lengths of groups of ones in Bv; Reuse concrete inputs from aplcart/table.tsv:1451; execute this alternate recipe independently
Bv←0 1 1 0 1 0 1 1 1 0 ⋄ 0~⍨¯1+-/⌽2↕⍸1,(~Bv),1
2 1 3

⍝ aplcart/tt.tsv:1102 — Lengths of groups of ones in Bv; Reuse concrete inputs from aplcart/table.tsv:1451; execute this alternate recipe independently
Bv←0 1 1 0 1 0 1 1 1 0 ⋄ 0~⍨¯1+-/⌽2↕⍸1,(~Bv),1
2 1 3

⍝ aplcart/tt.tsv:1103 — Lengths of leading axes
Y←3 1 3 2 ⋄ (¯1↓⍴)Y   ⍝ ⍬

⍝ aplcart/tt.tsv:1104 — Lengths of leading axes
Y←3 1 3 2 ⋄ (¯1↓⍴)Y   ⍝ ⍬

⍝ aplcart/tt.tsv:1105 — Lengths of subvectors of Yv having equal elements
Yv←1 1 2 3 3 ⋄ ((-/∘⌽∘(2∘↕))∘⍸1,1,⍨(≠/∘(2∘↕)))Yv
2 1 2

⍝ aplcart/tt.tsv:1106 — Lengths of subvectors of Yv having equal elements
Yv←1 1 2 3 3 ⋄ ((-/∘⌽∘(2∘↕))∘⍸1,1,⍨(≠/∘(2∘↕)))Yv
2 1 2

⍝ aplcart/tt.tsv:1107 — Limit of nominal rate N when continuously compounded
N←0.01 0.05 0.1 ⋄ (*×⍨)N
1.000100005000167 1.002503127605795 1.010050167084168

⍝ aplcart/tt.tsv:1108 — Limit of nominal rate N when continuously compounded
N←0.01 0.05 0.1 ⋄ (*×⍨)N
1.000100005000167 1.002503127605795 1.010050167084168

⍝ aplcart/tt.tsv:1109 — Limit: apply X∘f until stable
X←2 ⋄ f←⌈ ⋄ Y←¯1 0 3 ⋄ X f⍣≡Y   ⍝ 2 2 3

⍝ aplcart/tt.tsv:1110 — Limit: apply X∘f until stable
X←2 ⋄ f←⌈ ⋄ Y←¯1 0 3 ⋄ X f⍣≡Y   ⍝ 2 2 3

⍝ aplcart/tt.tsv:1111 — Limit: apply f until stable
f←⌊ ⋄ Y←¯1.2 0 3.9 ⋄ f⍣≡Y   ⍝ ¯2 0 3

⍝ aplcart/tt.tsv:1112 — Limit: apply f until stable
f←⌊ ⋄ Y←¯1.2 0 3.9 ⋄ f⍣≡Y   ⍝ ¯2 0 3

⍝ aplcart/tt.tsv:1113 — Limit: apply inverse of X∘f until stable; output expression returned directly
f←+∘÷ ⋄ 1 f⍣¯1⍣≡0   ⍝ ¯0.618033988749894

⍝ aplcart/tt.tsv:1114 — Limit: apply inverse of X∘f until stable
X←10 ⋄ f←- ⋄ Y←5 ⋄ X f⍣¯1⍣≡Y   ⍝ 5

⍝ aplcart/tt.tsv:1115 — Limit: apply inverse of f until stable; output expression returned directly
f←1∘+∘÷ ⋄ f⍣¯1⍣≡0   ⍝ ¯0.618033988749894

⍝ aplcart/tt.tsv:1116 — Limit: apply inverse of f until stable
f←⌽ ⋄ Y←1 2 1 ⋄ f⍣¯1⍣≡Y   ⍝ 1 2 1

⍝ aplcart/tt.tsv:1119 — Locate all instances of maximum of N; Reuse concrete inputs from aplcart/table.tsv:744; execute this alternate recipe independently
N←3 1 3 2 3 ⋄ (⊢=⌈⌿)N   ⍝ 1 0 1 0 1

⍝ aplcart/tt.tsv:1120 — Locate all instances of maximum of N; Reuse concrete inputs from aplcart/table.tsv:744; execute this alternate recipe independently
N←3 1 3 2 3 ⋄ (⊢=⌈⌿)N   ⍝ 1 0 1 0 1

⍝ aplcart/tt.tsv:1121 — Locate leading blank columns
Dm←2 4⍴'  ab  cd'  ⋄ (∧\' '∧.=⊢)Dm   ⍝ 1 1 0 0

⍝ aplcart/tt.tsv:1122 — Locate leading blank columns
Dm←2 4⍴'  ab  cd'  ⋄ (∧\' '∧.=⊢)Dm   ⍝ 1 1 0 0

⍝ aplcart/tt.tsv:1123 — Locate leading blank rows
Dm←3 3⍴'   abc   '  ⋄ (∧\∧.=∘' ')Dm   ⍝ 1 0 0

⍝ aplcart/tt.tsv:1124 — Locate leading blank rows
Dm←3 3⍴'   abc   '  ⋄ (∧\∧.=∘' ')Dm   ⍝ 1 0 0

⍝ aplcart/tt.tsv:1125 — Locate leading blanks
D← 2 4⍴'  ab c d'  ⋄ (∨\' '∘≠)D   ⍝ 2 4⍴0 0 1 1 0 1 1 1

⍝ aplcart/tt.tsv:1126 — Locate leading blanks
D← 2 4⍴'  ab c d'  ⋄ (∨\' '∘≠)D   ⍝ 2 4⍴0 0 1 1 0 1 1 1

⍝ aplcart/tt.tsv:1127 — Lower triangular matrix with diagonal: Js by Js
 ≥⌝ ⍨∘⍳ 5
5 5⍴1 0 0 0 0 1 1 0 0 0 1 1 1 0 0 1 1 1 1 0 1 1 1 1 1

⍝ aplcart/tt.tsv:1128 — Lower triangular matrix with diagonal: Js by Js
Js←4 ⋄  ≥⌝ ⍨∘⍳Js   ⍝ 4 4⍴1 0 0 0 1 1 0 0 1 1 1 0 1 1 1 1

⍝ aplcart/tt.tsv:1129 — Lower triangular matrix without diagonal: Js by Js
 >⌝ ⍨∘⍳ 5
5 5⍴0 0 0 0 0 1 0 0 0 0 1 1 0 0 0 1 1 1 0 0 1 1 1 1 0

⍝ aplcart/tt.tsv:1130 — Lower triangular matrix without diagonal: Js by Js
Js←4 ⋄  >⌝ ⍨∘⍳Js   ⍝ 4 4⍴0 0 0 0 1 0 0 0 1 1 0 0 1 1 1 0

⍝ aplcart/tt.tsv:1131 — M'th Root of N
M←2 3 ⋄ N←4 27 ⋄ M*∘÷⍨N   ⍝ 2 3

⍝ aplcart/tt.tsv:1132 — M'th Root of N
M←2 3 ⋄ N←4 27 ⋄ M*∘÷⍨N   ⍝ 2 3

⍝ aplcart/tt.tsv:1133 — Magnitude of N
N←3j4 0j2 ⋄ 10∘○N   ⍝ 5 2

⍝ aplcart/tt.tsv:1134 — Magnitude of N
N←3j4 0j2 ⋄ 10∘○N   ⍝ 5 2

⍝ aplcart/tt.tsv:1135 — Magnitude of fractional part of N
N←4 9 16 ⋄ (1||)N   ⍝ 0 0 0

⍝ aplcart/tt.tsv:1136 — Magnitude of fractional part of N
N←4 9 16 ⋄ (1||)N   ⍝ 0 0 0

⍝ aplcart/tt.tsv:1137 — Main diagonal of any rank array
Y←2 3 4⍴⍳24 ⋄ (⊢⍉⍨1*⍴)Y   ⍝ 1 18

⍝ aplcart/tt.tsv:1138 — Main diagonal of any rank array
Y←2 3 4⍴⍳24 ⋄ (⊢⍉⍨1*⍴)Y   ⍝ 1 18

⍝ aplcart/tt.tsv:1139 — Main diagonal of matrix
(3 3⍴⍳9 ⋄ '' ⋄ 1 1∘⍉ 3 3⍴⍳9)   ⍝ (3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 1 5 9)

⍝ aplcart/tt.tsv:1140 — Main diagonal of matrix
Ym←2 3⍴⍳6 ⋄ 1 1∘⍉Ym   ⍝ 1 5

⍝ aplcart/tt.tsv:1147 — Manhattan distance between two points in N-space
0 0 ¯1 1 (1⊥∘|-) 0 3 0 0   ⍝ 5

⍝ aplcart/tt.tsv:1148 — Manhattan distance between two points in N-space
Mv←2 3 4 ⋄ Nv←1 2 3 ⋄ Mv(1⊥∘|-)Nv   ⍝ 3

⍝ aplcart/tt.tsv:1149 — Manhattan distance table for points in N-space (one point per row); nested displayed values saved once in evaluation order and returned together
(r1←4 2⍴0 0 0 1 1 0 1 1 ⋄ (1⊥∘|-)⍤1⍤1 99⍨ r1)
(4 2⍴0 0 0 1 1 0 1 1 ⋄ 4 4⍴0 1 1 2 1 0 2 1 1 2 0 1 2 1 1 0)

⍝ aplcart/tt.tsv:1150 — Manhattan distance table for points in N-space (one point per row)
Nm←3 2⍴0 0 3 0 3 4 ⋄ (1⊥∘|-)⍤1⍤1 99⍨Nm   ⍝ 3 3⍴0 3 7 3 0 4 7 4 0

⍝ aplcart/tt.tsv:1151 — Map 0/1 to ¯1/1
(¯1*~) 1 0 1 1 0   ⍝ 1 ¯1 1 1 ¯1

⍝ aplcart/tt.tsv:1152 — Map 0/1 to ¯1/1
B←1 1 0 1 0 1 0 ⋄ (¯1*~)B   ⍝ 1 1 ¯1 1 ¯1 1 ¯1

⍝ aplcart/tt.tsv:1153 — Mask Operator: Merge X and Y using Bv (1 for X's item, 0 for Yv's item)
X←10 20 30 ⋄ Bv←1 0 1 ⋄ Y←1 2 3 ⋄ X(Bv{(⍶⌿⍺)@(⍸⍶)⊢⍵})Y
10 2 30

⍝ aplcart/tt.tsv:1154 — Mask Operator: Merge X and Y using Bv (1 for X's item, 0 for Yv's item)
X←10 20 30 ⋄ Bv←1 0 1 ⋄ Y←1 2 3 ⋄ X(Bv{(⍶⌿⍺)@(⍸⍶)⊢⍵})Y
10 2 30

⍝ aplcart/tt.tsv:1155 — Mask for blank columns; Reuse concrete inputs from aplcart/table.tsv:1287; execute this alternate recipe independently
D←3 3⍴'  a  b  c'  ⋄ (∧⌿' '∘=)D   ⍝ 1 1 0

⍝ aplcart/tt.tsv:1156 — Mask for blank columns; Reuse concrete inputs from aplcart/table.tsv:1287; execute this alternate recipe independently
D←3 3⍴'  a  b  c'  ⋄ (∧⌿' '∘=)D   ⍝ 1 1 0

⍝ aplcart/tt.tsv:1157 — Mask for blank rows; Reuse concrete inputs from aplcart/table.tsv:1237; execute this alternate recipe independently
D←3 3⍴'abc   de '  ⋄ (∧/' '∘=)D   ⍝ 0 1 0

⍝ aplcart/tt.tsv:1158 — Mask for blank rows; Reuse concrete inputs from aplcart/table.tsv:1237; execute this alternate recipe independently
D←3 3⍴'abc   de '  ⋄ (∧/' '∘=)D   ⍝ 0 1 0

⍝ aplcart/tt.tsv:1159 — Mask for selecting between first and last 1 on each row
B←2 5⍴0 1 0 1 0 0 0 1 0 0 ⋄ (∨\∧∘⌽∨\∘⌽)B
2 5⍴0 1 1 1 0 0 0 1 0 0

⍝ aplcart/tt.tsv:1160 — Mask for selecting between first and last 1 on each row
B←2 5⍴0 1 0 1 0 0 0 1 0 0 ⋄ (∨\∧∘⌽∨\∘⌽)B
2 5⍴0 1 1 1 0 0 0 1 0 0

⍝ aplcart/tt.tsv:1161 — Mask to get subvectors with indices Iv as indicated by Bv
Iv←1 3 ⋄ Bv←1 0 1 0 1 0 ⋄ Iv(+\⍤⊢∊⊣)Bv   ⍝ 1 1 0 0 1 1

⍝ aplcart/tt.tsv:1162 — Mask to get subvectors with indices Iv as indicated by Bv
Iv←1 3 ⋄ Bv←1 0 1 0 1 0 ⋄ Iv(+\⍤⊢∊⊣)Bv   ⍝ 1 1 0 0 1 1

⍝ aplcart/tt.tsv:1163 — Matrix of all indices of Y (↑,⍳⍴Y for large Y)
Y←2 3⍴0 ⋄ (⍉1+⍴⊤¯1+∘⍳×/∘⍴)Y   ⍝ 6 2⍴1 1 1 2 1 3 2 1 2 2 2 3

⍝ aplcart/tt.tsv:1164 — Matrix of all indices of Y (↑,⍳⍴Y for large Y)
Y←2 3⍴0 ⋄ (⍉1+⍴⊤¯1+∘⍳×/∘⍴)Y   ⍝ 6 2⍴1 1 1 2 1 3 2 1 2 2 2 3

⍝ aplcart/tt.tsv:1165 — Matrix to segmented string using Cs or linefeed as delimiter (includes initial delimiter)
Cs←'|'  ⋄ Dm←2 4⍴'ABCDabcd'  ⋄ Cs{⍺←•UCS 10 ⋄ ,⍺,⍵}Dm
'|ABCD|abcd'

⍝ aplcart/tt.tsv:1166 — Matrix to segmented string using Cs or linefeed as delimiter (includes initial delimiter)
Cs←'|'  ⋄ Dm←2 4⍴'ABCDabcd'  ⋄ Cs{⍺←•UCS 10 ⋄ ,⍺,⍵}Dm
'|ABCD|abcd'

⍝ aplcart/tt.tsv:1167 — Matrix to vector of column vectors
(↓⍉) 3 4⍴⍳12   ⍝ (1 5 9 ⋄ 2 6 10 ⋄ 3 7 11 ⋄ 4 8 12)

⍝ aplcart/tt.tsv:1168 — Matrix to vector of column vectors
Ym←2 3⍴⍳6 ⋄ (↓⍉)Ym   ⍝ (1 4 ⋄ 2 5 ⋄ 3 6)

⍝ aplcart/tt.tsv:1169 — Matrix to vector using Xs as separator (excludes initial separator)
Xs←9 ⋄ Ym←2 3⍴⍳6 ⋄ Xs(1↓∘,,)Ym   ⍝ 1 2 3 9 4 5 6

⍝ aplcart/tt.tsv:1170 — Matrix to vector using Xs as separator (excludes initial separator)
Xs←9 ⋄ Ym←2 3⍴⍳6 ⋄ Xs(1↓∘,,)Ym   ⍝ 1 2 3 9 4 5 6

⍝ aplcart/tt.tsv:1171 — Matrix with Is columns, each consisting of Yv
4 /∘⍪ 1 2 3   ⍝ 3 4⍴1 1 1 1 2 2 2 2 3 3 3 3

⍝ aplcart/tt.tsv:1172 — Matrix with Is columns, each consisting of Yv
Is←2 ⋄ Yv←1 2 3 4 ⋄ Is/∘⍪Yv   ⍝ 4 2⍴1 1 2 2 3 3 4 4

⍝ aplcart/tt.tsv:1173 — Matrix with Is rows, each consisting of Yv
Is←2 ⋄ Yv←4 5 6 ⋄ Is⌿∘⍉∘⍪Yv   ⍝ 2 3⍴4 5 6 4 5 6

⍝ aplcart/tt.tsv:1174 — Matrix with Is rows, each consisting of Yv
Is←2 ⋄ Yv←4 5 6 ⋄ Is⌿∘⍉∘⍪Yv   ⍝ 2 3⍴4 5 6 4 5 6

⍝ aplcart/tt.tsv:1175 — Matrix with Iv[i] leading ones on row i
Iv←0 2 4 ⋄ (⊢ ≥⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴0 0 0 0 1 1 0 0 1 1 1 1

⍝ aplcart/tt.tsv:1176 — Matrix with Iv[i] leading ones on row i
Iv←0 2 4 ⋄ (⊢ ≥⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴0 0 0 0 1 1 0 0 1 1 1 1

⍝ aplcart/tt.tsv:1177 — Matrix with Iv[i] leading zeroes on row i
Iv←0 2 4 ⋄ (⊢ <⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴1 1 1 1 0 0 1 1 0 0 0 0

⍝ aplcart/tt.tsv:1178 — Matrix with Iv[i] leading zeroes on row i
Iv←0 2 4 ⋄ (⊢ <⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴1 1 1 1 0 0 1 1 0 0 0 0

⍝ aplcart/tt.tsv:1179 — Matrix with Iv[i] trailing ones on row i
Iv←0 2 4 ⋄ (⌽⊢ ≥⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴0 0 0 0 0 0 1 1 1 1 1 1

⍝ aplcart/tt.tsv:1180 — Matrix with Iv[i] trailing ones on row i
Iv←0 2 4 ⋄ (⌽⊢ ≥⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴0 0 0 0 0 0 1 1 1 1 1 1

⍝ aplcart/tt.tsv:1181 — Matrix with Iv[i] trailing zeroes on row i
Iv←0 2 4 ⋄ (⌽⊢ <⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴1 1 1 1 1 1 0 0 0 0 0 0

⍝ aplcart/tt.tsv:1182 — Matrix with Iv[i] trailing zeroes on row i
Iv←0 2 4 ⋄ (⌽⊢ <⌝ ∘⍳⌈/)Iv   ⍝ 3 4⍴1 1 1 1 1 1 0 0 0 0 0 0

⍝ aplcart/tt.tsv:1183 — Matrix with shape of Xm and Yv as its columns; Reuse concrete inputs from aplcart/table.tsv:1210; execute this alternate recipe independently
Xm←2 3⍴⍳6 ⋄ Yv←4 5 6 ⋄ Xm(⍉⍴⍨∘⌽∘⍴⍨)Yv   ⍝ 2 3⍴4 6 5 5 4 6

⍝ aplcart/tt.tsv:1184 — Matrix with shape of Xm and Yv as its columns; Reuse concrete inputs from aplcart/table.tsv:1210; execute this alternate recipe independently
Xm←2 3⍴⍳6 ⋄ Yv←4 5 6 ⋄ Xm(⍉⍴⍨∘⌽∘⍴⍨)Yv   ⍝ 2 3⍴4 6 5 5 4 6

⍝ aplcart/tt.tsv:1185 — Maximum of N
(⌈/ 58 15 22 80 26 11 ⋄ ⌈/ 0 0 0 1 0 0 0 ⋄ 3 3⍴ 2 1 3 6 5 4 7 9 8 ⋄ '' ⋄ ⌈/ 3 3⍴ 2 1 3 6 5 4 7 9 8)
80 1 (3 3⍴2 1 3 6 5 4 7 9 8) ('') (3 6 9)

⍝ aplcart/tt.tsv:1186 — Maximum of N
N←4 9 16 ⋄ ⌈/N   ⍝ 16

⍝ aplcart/tt.tsv:1187 — Maximum of Nv with weights Mv
Mv←1 2 3 ⋄ Nv←3 2 1 ⋄ Mv⌈.×Nv   ⍝ 4

⍝ aplcart/tt.tsv:1188 — Maximum of Nv with weights Mv
Mv←1 2 3 ⋄ Nv←3 2 1 ⋄ Mv⌈.×Nv   ⍝ 4

⍝ aplcart/tt.tsv:1189 — Maximum table of 1…Js; optional {X} instantiated as dyadic use
Js←4 ⋄  ⌈⌝ ⍨∘⍳Js   ⍝ 4 4⍴1 2 3 4 2 2 3 4 3 3 3 4 4 4 4 4

⍝ aplcart/tt.tsv:1190 — Maximum table of 1…Js; optional {X} instantiated as dyadic use
Js←4 ⋄  ⌈⌝ ⍨∘⍳Js   ⍝ 4 4⍴1 2 3 4 2 2 3 4 3 3 3 4 4 4 4 4

⍝ aplcart/tt.tsv:1191 — Mean squared error
1 2 3 ((+⌿÷≢)2*⍨-) 0.9 2.1 3.1   ⍝ 0.01000000000000001

⍝ aplcart/tt.tsv:1192 — Mean squared error
M←2 3 4 ⋄ N←4 9 16 ⋄ M((+⌿÷≢)2*⍨-)N   ⍝ 61.33333333333334

⍝ aplcart/tt.tsv:1193 — Median of non-empty Nv
Nv←7 1 4 2 ⋄ (2÷⍨1⊥⊢⌷⍨∘⊂⍋⌷⍨∘⊂∘⌈2÷⍨0 1+≢)Nv
3

⍝ aplcart/tt.tsv:1194 — Median of non-empty Nv
Nv←7 1 4 2 ⋄ (2÷⍨1⊥⊢⌷⍨∘⊂⍋⌷⍨∘⊂∘⌈2÷⍨0 1+≢)Nv
3

⍝ aplcart/tt.tsv:1195 — Membership (∊) on major cells for any rank
X←3 2⍴1 2 3 4 1 2 ⋄ Y←2 2⍴3 4 5 6 ⋄ X(⊢∘≢≥⍳⍨)Y
0 1 0

⍝ aplcart/tt.tsv:1196 — Membership (∊) on major cells for any rank
X←3 2⍴1 2 3 4 1 2 ⋄ Y←2 2⍴3 4 5 6 ⋄ X(⊢∘≢≥⍳⍨)Y
0 1 0

⍝ aplcart/tt.tsv:1197 — Merge the leading Is axes of Y
(a ← 2 3 4⍴⍳24 ⋄ ⍴ a ⋄ ⍴ 1 {,[⍳⍺]⍵} a ⋄ ⍴ 2 {,[⍳⍺]⍵} a ⋄ ⍴ 3 {,[⍳⍺]⍵} a)
(2 3 4⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 ⋄ 2 3 4 ⋄ 2 3 4 ⋄ 6 4 ⋄ 1⍴24)

⍝ aplcart/tt.tsv:1198 — Merge the leading Is axes of Y
Is←2 ⋄ Y←2 3 2⍴⍳12 ⋄ Is{,[⍳⍺]⍵}Y   ⍝ 6 2⍴1 2 3 4 5 6 7 8 9 10 11 12

⍝ aplcart/tt.tsv:1199 — Merge the leading two axes of Y
(a ← 2 3 4⍴⍳24 ⋄ ⍴ a ⋄ ,[⍳2] a ⋄ ⍴ ,[⍳2] a)
(2 3 4⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 ⋄ 2 3 4 ⋄ 6 4⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 ⋄ 6 4)

⍝ aplcart/tt.tsv:1200 — Merge the leading two axes of Y
Y←2 3 2⍴⍳12 ⋄ ,[⍳2]Y   ⍝ 6 2⍴1 2 3 4 5 6 7 8 9 10 11 12

⍝ aplcart/tt.tsv:1201 — Merge vectors in Yv alternately (↓⍉↑ that removes trailing elements from longer vectors)
(↓(⌊/≢¨)↑⍉∘⊃) 'Now' 'is' 'the' 'time'   ⍝ ('Nitt' ⋄ 'oshi')

⍝ aplcart/tt.tsv:1202 — Merge vectors in Yv alternately (↓⍉↑ that removes trailing elements from longer vectors)
Yv←(1 2 3⋄ 4 5) ⋄ (↓(⌊/≢¨)↑⍉∘⊃)Yv   ⍝ (1 4 ⋄ 2 5)

⍝ aplcart/tt.tsv:1203 — Merging Y[1] Y[2] Y[3], … under control of Iv (1:cell from Y[1], 2:cell from Y[2], …)
Iv←1 2 1 2 ⋄ Y←(10 11 12 13⋄ 20 21 22 23) ⋄ Iv(⍳⍤≢⍤⊣⊃¨⌷¨∘⊂)Y
10 21 12 23

⍝ aplcart/tt.tsv:1204 — Merging Y[1] Y[2] Y[3], … under control of Iv (1:cell from Y[1], 2:cell from Y[2], …)
Iv←1 2 1 2 ⋄ Y←(10 11 12 13⋄ 20 21 22 23) ⋄ Iv(⍳⍤≢⍤⊣⊃¨⌷¨∘⊂)Y
10 21 12 23

⍝ aplcart/tt.tsv:1205 — Merging equal-length vectors Xv and Yv alternately; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,,⍤0)Yv   ⍝ 1 4 2 5 3 6

⍝ aplcart/tt.tsv:1206 — Merging equal-length vectors Xv and Yv alternately; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,,⍤0)Yv   ⍝ 1 4 2 5 3 6

⍝ aplcart/tt.tsv:1207 — Mid product of M and N
M←2 3⍴⍳6 ⋄ N←3 2⍴⍳6 ⋄ M,.×N
2 2⍴(1 6 15 ⋄ 2 8 18 ⋄ 4 15 30 ⋄ 8 20 36)

⍝ aplcart/tt.tsv:1208 — Mid product of M and N
M←2 3⍴⍳6 ⋄ N←3 2⍴⍳6 ⋄ M,.×N
2 2⍴(1 6 15 ⋄ 2 8 18 ⋄ 4 15 30 ⋄ 8 20 36)

⍝ aplcart/tt.tsv:1209 — Minimum of N
(⌊/ 58 15 22 80 26 11 ⋄ ⌊/ 0 0 0 1 0 0 0 ⋄ 3 3⍴ 2 1 3 6 5 4 7 9 8 ⋄ '' ⋄ ⌊/ 3 3⍴ 2 1 3 6 5 4 7 9 8)
11 0 (3 3⍴2 1 3 6 5 4 7 9 8) ('') (1 4 7)

⍝ aplcart/tt.tsv:1210 — Minimum of N
N←4 9 16 ⋄ ⌊/N   ⍝ 4

⍝ aplcart/tt.tsv:1211 — Minimum of Nv with weights Mv
Mv←1 2 3 ⋄ Nv←3 2 1 ⋄ Mv⌊.×Nv   ⍝ 3

⍝ aplcart/tt.tsv:1212 — Minimum of Nv with weights Mv
Mv←1 2 3 ⋄ Nv←3 2 1 ⋄ Mv⌊.×Nv   ⍝ 3

⍝ aplcart/tt.tsv:1213 — Mirror complex N across x-axis if As; Reuse concrete inputs from aplcart/table.tsv:1024; execute this alternate recipe independently
As←1 ⋄ N←1j2 3j¯4 ⋄ As○⍨∘(¯10+~)⍨N   ⍝ 1j¯2 3j4

⍝ aplcart/tt.tsv:1214 — Mirror complex N across x-axis if As; Reuse concrete inputs from aplcart/table.tsv:1024; execute this alternate recipe independently
As←1 ⋄ N←1j2 3j¯4 ⋄ As○⍨∘(¯10+~)⍨N   ⍝ 1j¯2 3j4

⍝ aplcart/tt.tsv:1215 — Mirror complex N across y-axis
-∘+ ¯7J3   ⍝ 7j3

⍝ aplcart/tt.tsv:1216 — Mirror complex N across y-axis
N←1J2 ¯3J4 ⋄ -∘+N   ⍝ ¯1j2 3j4

⍝ aplcart/tt.tsv:1217 — More accurately sum a vector of floating point numbers
Nv←1 2 3 ⋄ (+/⍒∘|⊃¨⊂)Nv   ⍝ 6

⍝ aplcart/tt.tsv:1218 — More accurately sum a vector of floating point numbers
Nv←1 2 3 ⋄ (+/⍒∘|⊃¨⊂)Nv   ⍝ 6

⍝ aplcart/tt.tsv:1219 — Move elements Yv (which are members of Xv) to the front of Xv; optional {X} instantiated as dyadic use
Xv←1 2 3 4 5 ⋄ Yv←2 4 ⋄ Xv(∩,~)Yv   ⍝ 2 4 1 3 5

⍝ aplcart/tt.tsv:1220 — Move elements Yv (which are members of Xv) to the front of Xv; optional {X} instantiated as dyadic use
Xv←1 2 3 4 5 ⋄ Yv←2 4 ⋄ Xv(∩,~)Yv   ⍝ 2 4 1 3 5

⍝ aplcart/tt.tsv:1221 — Move elements Yv (which are members of Xv) to the rear of Xv; optional {X} instantiated as dyadic use
Xv←1 2 3 4 5 ⋄ Yv←2 4 ⋄ Xv(~,∩)Yv   ⍝ 1 3 5 2 4

⍝ aplcart/tt.tsv:1222 — Move elements Yv (which are members of Xv) to the rear of Xv; optional {X} instantiated as dyadic use
Xv←1 2 3 4 5 ⋄ Yv←2 4 ⋄ Xv(~,∩)Yv   ⍝ 1 3 5 2 4

⍝ aplcart/tt.tsv:1223 — Move items X to end of Y
X←2 4 ⋄ Y←1 2 3 4 2 ⋄ X(⊂⍤⍋⍤∊⍨⍥(⊂⍤¯1)⌷⊢)Y
1 3 2 4 2

⍝ aplcart/tt.tsv:1224 — Move items X to end of Y
X←2 4 ⋄ Y←1 2 3 4 2 ⋄ X(⊂⍤⍋⍤∊⍨⍥(⊂⍤¯1)⌷⊢)Y
1 3 2 4 2

⍝ aplcart/tt.tsv:1225 — Move items Xv to end of Yv
Xv←2 4 ⋄ Yv←1 2 3 4 2 ⋄ Xv(⊢⌷⍨∘⊂∘⍋∊⍨)Yv   ⍝ 1 3 2 4 2

⍝ aplcart/tt.tsv:1226 — Move items Xv to end of Yv
Xv←2 4 ⋄ Yv←1 2 3 4 2 ⋄ Xv(⊢⌷⍨∘⊂∘⍋∊⍨)Yv   ⍝ 1 3 2 4 2

⍝ aplcart/tt.tsv:1227 — Move set of points Nm into first quadrant
Nm←3 3⍴¯2 1 4 0 ¯3 2 1 5 ¯1 ⋄ (1 2 1⍉⊢ -⌝ ⌊/)Nm
3 3⍴0 3 6 3 0 5 2 6 0

⍝ aplcart/tt.tsv:1228 — Move set of points Nm into first quadrant
Nm←3 3⍴¯2 1 4 0 ¯3 2 1 5 ¯1 ⋄ (1 2 1⍉⊢ -⌝ ⌊/)Nm
3 3⍴0 3 6 3 0 5 2 6 0

⍝ aplcart/tt.tsv:1229 — Moving all blanks to end of text; dfns display import/wrappers omitted to test underlying arrays
((~,∩)∘' ') 'Here be spaces'   ⍝ 'Herebespaces  '

⍝ aplcart/tt.tsv:1230 — Moving all blanks to end of text
Dv←' a b  c ' ⋄ ((~,∩)∘' ')Dv   ⍝ 'abc     '

⍝ aplcart/tt.tsv:1231 — Moving width-Is window of indices for array Y; optional {X} instantiated as dyadic use
Is←2 ⋄ Y←3 1 3 2 ⋄ ,/Is↕⍳≢Y   ⍝ (1 2 ⋄ 2 3 ⋄ 3 4)

⍝ aplcart/tt.tsv:1232 — Moving width-Is window of indices for array Y; optional {X} instantiated as dyadic use
Is←2 ⋄ Y←3 1 3 2 ⋄ ,/Is↕⍳≢Y   ⍝ (1 2 ⋄ 2 3 ⋄ 3 4)

⍝ aplcart/tt.tsv:1233 — Ms±Ns; optional {X} instantiated as dyadic use
Ms←2 ⋄ Ns←3 ⋄ Ms(+,-)Ns   ⍝ 5 ¯1

⍝ aplcart/tt.tsv:1234 — Ms±Ns; optional {X} instantiated as dyadic use
Ms←2 ⋄ Ns←3 ⋄ Ms(+,-)Ns   ⍝ 5 ¯1

⍝ aplcart/tt.tsv:1235 — Ms∓Ns; optional {X} instantiated as dyadic use
Ms←2 ⋄ Ns←3 ⋄ Ms(-,+)Ns   ⍝ ¯1 5

⍝ aplcart/tt.tsv:1236 — Ms∓Ns; optional {X} instantiated as dyadic use
Ms←2 ⋄ Ns←3 ⋄ Ms(-,+)Ns   ⍝ ¯1 5

⍝ aplcart/tt.tsv:1237 — Multiplication Table for Iv down and Jv across
Iv←¯1 0 1 ⋄ Jv←2 3 ⋄ Iv ×⌝ Jv   ⍝ 3 2⍴¯2 ¯3 0 0 2 3

⍝ aplcart/tt.tsv:1238 — Multiplication Table for Iv down and Jv across
Iv←¯1 0 1 ⋄ Jv←2 3 ⋄ Iv ×⌝ Jv   ⍝ 3 2⍴¯2 ¯3 0 0 2 3

⍝ aplcart/tt.tsv:1239 — Multiplication Table for Numbers up to Js; optional {X} instantiated as dyadic use
Js←4 ⋄  ×⌝ ⍨∘⍳Js   ⍝ 4 4⍴1 2 3 4 2 4 6 8 3 6 9 12 4 8 12 16

⍝ aplcart/tt.tsv:1240 — Multiplication Table for Numbers up to Js; optional {X} instantiated as dyadic use
Js←4 ⋄  ×⌝ ⍨∘⍳Js   ⍝ 4 4⍴1 2 3 4 2 4 6 8 3 6 9 12 4 8 12 16

⍝ aplcart/tt.tsv:1241 — Multivariate Beta Function
N←2 3 4 ⋄ ((×⌿∘!-∘1)÷∘!¯1++⌿)N   ⍝ 0.0002976190476190476

⍝ aplcart/tt.tsv:1242 — Multivariate Beta Function
N←2 3 4 ⋄ ((×⌿∘!-∘1)÷∘!¯1++⌿)N   ⍝ 0.0002976190476190476

⍝ aplcart/tt.tsv:1243 — N (identity)
N←0.25 0.5 0.75 ⋄ ¯9∘○N   ⍝ 0.25 0.5 0.75

⍝ aplcart/tt.tsv:1244 — N (identity)
N←0.25 0.5 0.75 ⋄ ¯9∘○N   ⍝ 0.25 0.5 0.75

⍝ aplcart/tt.tsv:1245 — N Degrees in Radians; Reuse concrete inputs from aplcart/table.tsv:1020; execute this alternate recipe independently
N←0 30 90 180 ⋄ (180÷⍨π)N
0 0.5235987755982988 1.570796326794897 3.141592653589793

⍝ aplcart/tt.tsv:1246 — N Degrees in Radians; Reuse concrete inputs from aplcart/table.tsv:1020; execute this alternate recipe independently
N←0 30 90 180 ⋄ (180÷⍨π)N
0 0.5235987755982988 1.570796326794897 3.141592653589793

⍝ aplcart/tt.tsv:1247 — N Radians in Degrees; Reuse concrete inputs from aplcart/table.tsv:1107; execute this alternate recipe independently; Use nonzero angles: this reciprocal-based formula raises DOMAIN ERROR for zero in Dyalog
N←0.5 1 2 ⋄ (180÷π∘÷)N
28.64788975654116 57.29577951308232 114.5915590261646

⍝ aplcart/tt.tsv:1248 — N Radians in Degrees; Reuse concrete inputs from aplcart/table.tsv:1107; execute this alternate recipe independently; Use nonzero angles: this reciprocal-based formula raises DOMAIN ERROR for zero in Dyalog
N←0.5 1 2 ⋄ (180÷π∘÷)N
28.64788975654116 57.29577951308232 114.5915590261646

⍝ aplcart/tt.tsv:1249 — N in Base M
M←2 ⋄ N←5 8 15 ⋄ M(⊥⍣¯1)N   ⍝ 4 3⍴0 1 1 1 0 1 0 0 1 1 0 1

⍝ aplcart/tt.tsv:1250 — N in Base M
M←2 ⋄ N←5 8 15 ⋄ M(⊥⍣¯1)N   ⍝ 4 3⍴0 1 1 1 0 1 0 0 1 1 0 1

⍝ aplcart/tt.tsv:1251 — N-column matrix from N vectors; Instantiate the ellipsis with two vectors
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ ⍉∘⊃Xv Yv   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/tt.tsv:1252 — N-column matrix from N vectors; Instantiate the ellipsis with two vectors
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ ⍉∘⊃Xv Yv   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/tt.tsv:1255 — Negate real part (“real conjugate”)
+∘- ¯7J3   ⍝ 7j3

⍝ aplcart/tt.tsv:1256 — Negate real part (“real conjugate”)
N←1J2 ¯3J4 ⋄ +∘-N   ⍝ ¯1j2 3j4

⍝ aplcart/tt.tsv:1257 — Non-diagonal matrix of order Js
Js←4 ⋄ (,⍨⍴0,⍴∘1)Js   ⍝ 4 4⍴0 1 1 1 1 0 1 1 1 1 0 1 1 1 1 0

⍝ aplcart/tt.tsv:1258 — Non-diagonal matrix of order Js
Js←4 ⋄ (,⍨⍴0,⍴∘1)Js   ⍝ 4 4⍴0 1 1 1 1 0 1 1 1 1 0 1 1 1 1 0

⍝ aplcart/tt.tsv:1259 — Non-diagonal matrix of shape of matrix Nm
Nm←3 3⍴0 ⋄ (⍴⍴0,1⍨¨)Nm   ⍝ 3 3⍴0 1 1 1 0 1 1 1 0

⍝ aplcart/tt.tsv:1260 — Non-diagonal matrix of shape of matrix Nm
Nm←3 3⍴0 ⋄ (⍴⍴0,1⍨¨)Nm   ⍝ 3 3⍴0 1 1 1 0 1 1 1 0

⍝ aplcart/tt.tsv:1261 — Non-negative?
(0∘≤ 1 2 3 4 ⋄ 0∘≤ ¯2 ¯1 0 1 2)   ⍝ (1 1 1 1 ⋄ 0 0 1 1 1)

⍝ aplcart/tt.tsv:1262 — Non-negative?
N←¯2 0 3 ⋄ 0∘≤N   ⍝ 0 1 1

⍝ aplcart/tt.tsv:1263 — Non-positive?
(0∘≥ 1 2 3 4 ⋄ 0∘≥ ¯2 ¯1 0 1 2)   ⍝ (0 0 0 0 ⋄ 1 1 1 0 0)

⍝ aplcart/tt.tsv:1264 — Non-positive?
N←¯2 0 3 ⋄ 0∘≥N   ⍝ 1 1 0

⍝ aplcart/tt.tsv:1265 — Non-zero?
(0∘≠ 1 2 3 4 ⋄ 0∘≠ ¯2 ¯1 0 1 2)   ⍝ (1 1 1 1 ⋄ 1 1 0 1 1)

⍝ aplcart/tt.tsv:1266 — Non-zero?
N←¯2 0 3 ⋄ 0∘≠N   ⍝ 1 0 1

⍝ aplcart/tt.tsv:1267 — Normalisation by Infinity-norm
(⊢÷⌈⌿∘|) 3 4   ⍝ 0.75 1

⍝ aplcart/tt.tsv:1268 — Normalisation by Infinity-norm
N←¯3 4 ⋄ (⊢÷⌈⌿∘|)N   ⍝ ¯0.75 1

⍝ aplcart/tt.tsv:1269 — Normalisation by Is-norm
2 (⊢÷⊣*∘÷⍨1⊥*⍨∘|) 3 4   ⍝ 0.6 0.8

⍝ aplcart/tt.tsv:1270 — Normalisation by Is-norm
Is←2 ⋄ N←3 4 ⋄ Is(⊢÷⊣*∘÷⍨1⊥*⍨∘|)N   ⍝ 0.6 0.8

⍝ aplcart/tt.tsv:1271 — Normalise N so that minimum item is 0
N←¯2 4 1 ⋄ (⊢-⌊⌿)N   ⍝ 0 6 3

⍝ aplcart/tt.tsv:1272 — Normalise N so that minimum item is 0
N←¯2 4 1 ⋄ (⊢-⌊⌿)N   ⍝ 0 6 3

⍝ aplcart/tt.tsv:1273 — Normalise N so that sum is 1
(⊢÷+⌿) 3 1 4 1
0.3333333333333333 0.1111111111111111 0.4444444444444444 0.1111111111111111

⍝ aplcart/tt.tsv:1274 — Normalise N so that sum is 1
N←4 9 16 ⋄ (⊢÷+⌿)N
0.1379310344827586 0.3103448275862069 0.5517241379310345

⍝ aplcart/tt.tsv:1275 — Normalise scalar/vector/vector of scalars/vectors to vector of vectors
Y←(1 2)3(4 5) ⋄ ,∘⊆∘,Y   ⍝ (1 2) 3 (4 5)

⍝ aplcart/tt.tsv:1276 — Normalise scalar/vector/vector of scalars/vectors to vector of vectors
Y←(1 2)3(4 5) ⋄ ,∘⊆∘,Y   ⍝ (1 2) 3 (4 5)

⍝ aplcart/tt.tsv:1277 — Not Any; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (~1∘∊)B   ⍝ 0

⍝ aplcart/tt.tsv:1278 — Not Any; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (~1∘∊)B   ⍝ 0

⍝ aplcart/tt.tsv:1279 — Not all true?
(0∘∊ 1 1 1 1 ⋄ 0∘∊ 1 1 0 1)   ⍝ 0 1

⍝ aplcart/tt.tsv:1280 — Not all true?
B←1 1 0 1 0 1 0 ⋄ 0∘∊B   ⍝ 1

⍝ aplcart/tt.tsv:1281 — Null near-zero (within absolute distance Ms) real and imaginary parts in N
Ms←0.01 ⋄ N←0.001j2 3j0.002 0.001j0.002 ⋄ Ms(0j1⊥⊣(⊢×<∘|)11 9 ○⌝ ⊢)N
0j2 3 0

⍝ aplcart/tt.tsv:1282 — Null near-zero (within absolute distance Ms) real and imaginary parts in N
Ms←0.01 ⋄ N←0.001j2 3j0.002 0.001j0.002 ⋄ Ms(0j1⊥⊣(⊢×<∘|)11 9 ○⌝ ⊢)N
0j2 3 0

⍝ aplcart/tt.tsv:1283 — Null near-zero (within absolute distance Ms) values in N
Ms←0.1 ⋄ N←0.05 ¯0.2 1 ⋄ Ms(⊢×<∘|)N   ⍝ 0 ¯0.2 1

⍝ aplcart/tt.tsv:1284 — Null near-zero (within absolute distance Ms) values in N
Ms←0.1 ⋄ N←0.05 ¯0.2 1 ⋄ Ms(⊢×<∘|)N   ⍝ 0 ¯0.2 1

⍝ aplcart/tt.tsv:1285 — Number of columns in Y
Y←3 1 3 2 ⋄ (0⊥⍴)Y   ⍝ 4

⍝ aplcart/tt.tsv:1286 — Number of columns in Y
Y←3 1 3 2 ⋄ (0⊥⍴)Y   ⍝ 4

⍝ aplcart/tt.tsv:1289 — Number of days in February of year J (YYYY)
J←1900 2000 2024 2025 ⋄ (28+0≠.=400 100 4 |⌝ ⊢)J
28 29 29 28

⍝ aplcart/tt.tsv:1290 — Number of days in February of year J (YYYY)
J←1900 2000 2024 2025 ⋄ (28+0≠.=400 100 4 |⌝ ⊢)J
28 29 29 28

⍝ aplcart/tt.tsv:1291 — Number of days in months I of years J
I←2 2 4 ⋄ J←2000 1900 2026 ⋄ I((31-2|7|¯1+⊣)-2∘=⍤⊣×2-0≠.=400 100 4 |⌝ ⊢)J
29 28 30

⍝ aplcart/tt.tsv:1292 — Number of days in months I of years J
I←2 2 4 ⋄ J←2000 1900 2026 ⋄ I((31-2|7|¯1+⊣)-2∘=⍤⊣×2-0≠.=400 100 4 |⌝ ⊢)J
29 28 30

⍝ aplcart/tt.tsv:1293 — Number of decimals of elements of Nv; Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:1493
Nv←1.2 1.23 1.234 ⋄ (⌊10⍟⊢÷⍨∘⍎'.'~⍨⍕)Nv   ⍝ 1 2 3

⍝ aplcart/tt.tsv:1294 — Number of decimals of elements of Nv; Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:1493
Nv←1.2 1.23 1.234 ⋄ (⌊10⍟⊢÷⍨∘⍎'.'~⍨⍕)Nv   ⍝ 1 2 3

⍝ aplcart/tt.tsv:1295 — Number of digit positions in Js (depends on ⎕PP)
Js←4 ⋄ (≢⍕)Js   ⍝ 1

⍝ aplcart/tt.tsv:1296 — Number of digit positions in Js (depends on ⎕PP)
Js←4 ⋄ (≢⍕)Js   ⍝ 1

⍝ aplcart/tt.tsv:1297 — Number of digit positions in integers in J
J←0 1 9 10 ¯99 100 ⋄ 0∘(1+<+∘⌊10⍟∘|⊢+=)J
1 2 2 3 2 4

⍝ aplcart/tt.tsv:1298 — Number of digit positions in integers in J
J←0 1 9 10 ¯99 100 ⋄ 0∘(1+<+∘⌊10⍟∘|⊢+=)J
1 2 2 3 2 4

⍝ aplcart/tt.tsv:1299 — Number of digits in integers in J
(⌊1+10⍟|+0∘=) 1618 0 ¯271828   ⍝ 4 1 6

⍝ aplcart/tt.tsv:1300 — Number of digits in integers in J
J←¯123 0 9 10 999 ⋄ (⌊1+10⍟|+0∘=)J   ⍝ 3 1 1 2 3

⍝ aplcart/tt.tsv:1301 — Number of digits in strictly positive integers in J
(⌊1+10∘⍟) 1618 1 271828   ⍝ 4 1 6

⍝ aplcart/tt.tsv:1302 — Number of digits in strictly positive integers in J
J←1 9 10 999 ⋄ (⌊1+10∘⍟)J   ⍝ 1 1 2 3

⍝ aplcart/tt.tsv:1303 — Number of elements in major cells
Y←2 3 4⍴⍳24 ⋄ (×/1↓⍴)Y   ⍝ 12

⍝ aplcart/tt.tsv:1304 — Number of elements in major cells
Y←2 3 4⍴⍳24 ⋄ (×/1↓⍴)Y   ⍝ 12

⍝ aplcart/tt.tsv:1305 — Number of items in trailing axis
Y←3 1 3 2 ⋄ (↑⌽∘⍴)Y   ⍝ 4

⍝ aplcart/tt.tsv:1306 — Number of items in trailing axis
Y←3 1 3 2 ⋄ (↑⌽∘⍴)Y   ⍝ 4

⍝ aplcart/tt.tsv:1307 — Number of leading blanks
Dv← '  abc '  ⋄ (⊥⍨' '=⌽)Dv   ⍝ 2

⍝ aplcart/tt.tsv:1308 — Number of leading blanks
Dv← '  abc '  ⋄ (⊥⍨' '=⌽)Dv   ⍝ 2

⍝ aplcart/tt.tsv:1309 — Number of occurrences of scalar Xs in array Y
Xs←9 ⋄ Y←3 1 3 2 ⋄ Xs+.=∘,Y   ⍝ 0

⍝ aplcart/tt.tsv:1310 — Number of occurrences of scalar Xs in array Y
Xs←9 ⋄ Y←3 1 3 2 ⋄ Xs+.=∘,Y   ⍝ 0

⍝ aplcart/tt.tsv:1311 — Number of ordered of combinations of I out of J
I←2 3 4 ⋄ J←4 9 16 ⋄ I(!×∘!⊣)J   ⍝ 12 504 43680

⍝ aplcart/tt.tsv:1312 — Number of ordered of combinations of I out of J
I←2 3 4 ⋄ J←4 9 16 ⋄ I(!×∘!⊣)J   ⍝ 12 504 43680

⍝ aplcart/tt.tsv:1313 — Number of rows in array Y (also of vector)
Y←2 3 4⍴⍳24 ⋄ (×/¯1↓⍴)Y   ⍝ 6

⍝ aplcart/tt.tsv:1314 — Number of rows in array Y (also of vector)
Y←2 3 4⍴⍳24 ⋄ (×/¯1↓⍴)Y   ⍝ 6

⍝ aplcart/tt.tsv:1315 — Number of segments in delimited string Dv where the first character is the delimiter ≢ ⍴
Dv← '/ab/cd/ef'  ⋄ (↑+.=⊢)Dv   ⍝ 3

⍝ aplcart/tt.tsv:1316 — Number of segments in delimited string Dv where the first character is the delimiter ≢ ⍴
Dv← '/ab/cd/ef'  ⋄ (↑+.=⊢)Dv   ⍝ 3

⍝ aplcart/tt.tsv:1317 — Number of trailing blanks
Dv← '  abc '  ⋄ (⊥⍨' '∘=)Dv   ⍝ 1

⍝ aplcart/tt.tsv:1318 — Number of trailing blanks
Dv← '  abc '  ⋄ (⊥⍨' '∘=)Dv   ⍝ 1

⍝ aplcart/tt.tsv:1319 — Number of trues; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (+/,)B   ⍝ 4

⍝ aplcart/tt.tsv:1320 — Number of trues; optional {X} instantiated as dyadic use
B←1 1 0 1 0 1 0 ⋄ (+/,)B   ⍝ 4

⍝ aplcart/tt.tsv:1321 — Number-of-divisors of Js; Reuse concrete inputs from aplcart/table.tsv:1201; execute this alternate recipe independently
Js←4 ⋄ (≢∘∪⊢∨⍳)Js   ⍝ 3

⍝ aplcart/tt.tsv:1322 — Number-of-divisors of Js; Reuse concrete inputs from aplcart/table.tsv:1201; execute this alternate recipe independently
Js←4 ⋄ (≢∘∪⊢∨⍳)Js   ⍝ 3

⍝ aplcart/tt.tsv:1323 — Numeric matrix of all unordered combinations of Is out of Js without replacement; Reuse concrete inputs from aplcart/table.tsv:1571; execute this alternate recipe independently
Is←2 ⋄ Js←4 ⋄ Is({⍵/⍨∧⌿(</[2]∘(2∘↕))⍵}1+{(-⍺)↑⍳⍵}⊤∘⍳!×∘!⊣)Js
2 6⍴1 1 1 2 2 3 2 3 4 3 4 4

⍝ aplcart/tt.tsv:1324 — Numeric matrix of all unordered combinations of Is out of Js without replacement; Reuse concrete inputs from aplcart/table.tsv:1571; execute this alternate recipe independently
Is←2 ⋄ Js←4 ⋄ Is({⍵/⍨∧⌿(</[2]∘(2∘↕))⍵}1+{(-⍺)↑⍳⍵}⊤∘⍳!×∘!⊣)Js
2 6⍴1 1 1 2 2 3 2 3 4 3 4 4

⍝ aplcart/tt.tsv:1325 — N×0J1
N←3j4 0j2 ⋄ ¯11∘○N   ⍝ ¯4j3 ¯2

⍝ aplcart/tt.tsv:1326 — N×0J1
N←3j4 0j2 ⋄ ¯11∘○N   ⍝ ¯4j3 ¯2

⍝ aplcart/tt.tsv:1327 — Odd integers from 1 to 2×Js
((¯1+2×⍳) 5 ⋄ (¯1+2×⍳) 6)   ⍝ (1 3 5 7 9 ⋄ 1 3 5 7 9 11)

⍝ aplcart/tt.tsv:1328 — Odd integers from 1 to 2×Js
Js←4 ⋄ (¯1+2×⍳)Js   ⍝ 1 3 5 7

⍝ aplcart/tt.tsv:1329 — Ohm's Law: resistance of parallel resistors/capacitance of serial capacitors; optional {X} instantiated as dyadic use
Nv←1 2 3 ⋄ (÷1⊥÷)Nv   ⍝ 0.5454545454545455

⍝ aplcart/tt.tsv:1330 — Ohm's Law: resistance of parallel resistors/capacitance of serial capacitors; optional {X} instantiated as dyadic use
Nv←1 2 3 ⋄ (÷1⊥÷)Nv   ⍝ 0.5454545454545455

⍝ aplcart/tt.tsv:1331 — Ones, same shape and structure
(3 3⍴⍳9 ⋄ '' ⋄ =⍨ 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 3 3⍴1 1 1 1 1 1 1 1 1)

⍝ aplcart/tt.tsv:1332 — Ones, same shape and structure
Y←(1 2⋄ 3 4 5) ⋄ =⍨Y   ⍝ (1 1 ⋄ 1 1 1)

⍝ aplcart/tt.tsv:1333 — Ones, same shape plus one; optional {X} instantiated as dyadic use
Yv←4 5 6 ⋄ =/0↕Yv   ⍝ 1 1 1 1

⍝ aplcart/tt.tsv:1334 — Ones, same shape plus one; optional {X} instantiated as dyadic use
Yv←4 5 6 ⋄ =/0↕Yv   ⍝ 1 1 1 1

⍝ aplcart/tt.tsv:1335 — Ordinal suffix for positive integer Js
Ord ← ⍕,{2↑'thstndrd'↓⍨2×↑⍵⌽∊1 0 8\⊂10↑0,⍳3} ⋄ (Ord 0 ⋄ Ord¨⍳121 ⋄ Ord 1000000)
('0th' ⋄ ('1st' ⋄ '2nd' ⋄ '3rd' ⋄ '4th' ⋄ '5th' ⋄ '6th' ⋄ '7th' ⋄ '8th' ⋄ '9th' ⋄ '10th' ⋄ '11th' ⋄ '12th' ⋄ '13th' ⋄ '14th' ⋄ '15th' ⋄ '16th' ⋄ '17th' ⋄ '18th' ⋄ '19th' ⋄ '20th' ⋄ '21st' ⋄ '22nd' ⋄ '23rd' ⋄ '24th' ⋄ '25th' ⋄ '26th' ⋄ '27th' ⋄ '28th' ⋄ '29th' ⋄ '30th' ⋄ '31st' ⋄ '32nd' ⋄ '33rd' ⋄ '34th' ⋄ '35th' ⋄ '36th' ⋄ '37th' ⋄ '38th' ⋄ '39th' ⋄ '40th' ⋄ '41st' ⋄ '42nd' ⋄ '43rd' ⋄ '44th' ⋄ '45th' ⋄ '46th' ⋄ '47th' ⋄ '48th' ⋄ '49th' ⋄ '50th' ⋄ '51st' ⋄ '52nd' ⋄ '53rd' ⋄ '54th' ⋄ '55th' ⋄ '56th' ⋄ '57th' ⋄ '58th' ⋄ '59th' ⋄ '60th' ⋄ '61st' ⋄ '62nd' ⋄ '63rd' ⋄ '64th' ⋄ '65th' ⋄ '66th' ⋄ '67th' ⋄ '68th' ⋄ '69th' ⋄ '70th' ⋄ '71st' ⋄ '72nd' ⋄ '73rd' ⋄ '74th' ⋄ '75th' ⋄ '76th' ⋄ '77th' ⋄ '78th' ⋄ '79th' ⋄ '80th' ⋄ '81st' ⋄ '82nd' ⋄ '83rd' ⋄ '84th' ⋄ '85th' ⋄ '86th' ⋄ '87th' ⋄ '88th' ⋄ '89th' ⋄ '90th' ⋄ '91st' ⋄ '92nd' ⋄ '93rd' ⋄ '94th' ⋄ '95th' ⋄ '96th' ⋄ '97th' ⋄ '98th' ⋄ '99th' ⋄ '100th' ⋄ '101st' ⋄ '102nd' ⋄ '103rd' ⋄ '104th' ⋄ '105th' ⋄ '106th' ⋄ '107th' ⋄ '108th' ⋄ '109th' ⋄ '110th' ⋄ '111th' ⋄ '112th' ⋄ '113th' ⋄ '114th' ⋄ '115th' ⋄ '116th' ⋄ '117th' ⋄ '118th' ⋄ '119th' ⋄ '120th' ⋄ '121st') ⋄ '1000000th')

⍝ aplcart/tt.tsv:1336 — Ordinal suffix for positive integer Js
Js←23 ⋄ (⍕,{2↑'thstndrd'↓⍨2×↑⍵⌽∊1 0 8\⊂10↑0,⍳3})Js
'23rd'

⍝ aplcart/tt.tsv:1337 — Pad elements of vector of arrays Yv to equal shape
Yv←(1 2⋄ 3 4 5) ⋄ ⊂⍤¯1∘⊃Yv   ⍝ (1 2 0 ⋄ 3 4 5)

⍝ aplcart/tt.tsv:1338 — Pad elements of vector of arrays Yv to equal shape
Yv←(1 2⋄ 3 4 5) ⋄ ⊂⍤¯1∘⊃Yv   ⍝ (1 2 0 ⋄ 3 4 5)

⍝ aplcart/tt.tsv:1339 — Padding Yv on the left to width Is; Reuse concrete inputs from aplcart/table.tsv:664; execute this alternate recipe independently
Is←7 ⋄ Yv←1 2 3 ⋄ Is↑⍨∘-⍨Yv   ⍝ 0 0 0 0 1 2 3

⍝ aplcart/tt.tsv:1340 — Padding Yv on the left to width Is; Reuse concrete inputs from aplcart/table.tsv:664; execute this alternate recipe independently
Is←7 ⋄ Yv←1 2 3 ⋄ Is↑⍨∘-⍨Yv   ⍝ 0 0 0 0 1 2 3

⍝ aplcart/tt.tsv:1341 — Parallel projection of 3D object in Nm
Nm←2 3⍴⍳6 ⋄ (0J1⊥⊖)Nm   ⍝ 1j4 2j5 3j6

⍝ aplcart/tt.tsv:1342 — Parallel projection of 3D object in Nm
Nm←2 3⍴⍳6 ⋄ (0J1⊥⊖)Nm   ⍝ 1j4 2j5 3j6

⍝ aplcart/tt.tsv:1343 — Parity of J (is J odd?); dfns display import/wrappers omitted to test underlying arrays
(2∘| 1 2 3 4 5 6 ⋄ 2∘| (51 ¯25 ⋄ 103 3 ⋄ 4 5) ⋄ 2∘| 3 3⍴⍳9)
(1 0 1 0 1 0 ⋄ (1 1 ⋄ 1 1 ⋄ 0 1) ⋄ 3 3⍴1 0 1 0 1 0 1 0 1)

⍝ aplcart/tt.tsv:1344 — Parity of J (is J odd?)
J←4 9 16 ⋄ 2∘|J   ⍝ 0 1 0

⍝ aplcart/tt.tsv:1345 — Parity with connectors: Joining pairs of odd and even ones (fill gaps with ones); Reuse concrete inputs from aplcart/table.tsv:756; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ (≠⍀∨⊢)B   ⍝ 1 1 0 1 1 1 0

⍝ aplcart/tt.tsv:1346 — Parity with connectors: Joining pairs of odd and even ones (fill gaps with ones); Reuse concrete inputs from aplcart/table.tsv:756; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ (≠⍀∨⊢)B   ⍝ 1 1 0 1 1 1 0

⍝ aplcart/tt.tsv:1347 — Parity: Connect odd and even ones
B←1 0 1 1 0 0 1 ⋄ ≠\B   ⍝ 1 1 0 1 1 1 0

⍝ aplcart/tt.tsv:1348 — Parity: Connect odd and even ones
B←1 0 1 1 0 0 1 ⋄ ≠\B   ⍝ 1 1 0 1 1 1 0

⍝ aplcart/tt.tsv:1349 — Percentage corresponding to rate N
N←0 0.5 1 ⋄ 100∘×N   ⍝ 0 50 100

⍝ aplcart/tt.tsv:1350 — Percentage corresponding to rate N
N←0 0.5 1 ⋄ 100∘×N   ⍝ 0 50 100

⍝ aplcart/tt.tsv:1351 — Permutation vector that sorts like Y
Y←3 1 2 1 ⋄ ⍋∘⍋Y   ⍝ 4 1 3 2

⍝ aplcart/tt.tsv:1352 — Permutation vector that sorts like Y
Y←3 1 2 1 ⋄ ⍋∘⍋Y   ⍝ 4 1 3 2

⍝ aplcart/tt.tsv:1353 — Permute: Reorder major cells of Y according tot permutation vector Iv
Iv←3 1 2 ⋄ Y←3 2⍴⍳6 ⋄ Iv⌷⍨∘⊂⍨Y   ⍝ 3 2⍴5 6 1 2 3 4

⍝ aplcart/tt.tsv:1354 — Permute: Reorder major cells of Y according tot permutation vector Iv
Iv←3 1 2 ⋄ Y←3 2⍴⍳6 ⋄ Iv⌷⍨∘⊂⍨Y   ⍝ 3 2⍴5 6 1 2 3 4

⍝ aplcart/tt.tsv:1355 — Perspective projection of Nm from distance Ms; Correct perspective projection to multiply x+iy by distance/(distance-z) per column; Upstream matrix division gives LENGTH ERROR; independently checked two points with different depths
Ms←10 ⋄ Nm←3 2⍴1 2 3 4 5 6 ⋄ Ms((0J1⊥1↓⊢∘⊖)×⊣÷⊣-⊢⌿⍤⊢)Nm
2j6 5j10

⍝ aplcart/tt.tsv:1356 — Perspective projection of Nm from distance Ms; Correct perspective projection to multiply x+iy by distance/(distance-z) per column; Upstream matrix division gives LENGTH ERROR; independently checked two points with different depths
Ms←10 ⋄ Nm←3 2⍴1 2 3 4 5 6 ⋄ Ms((0J1⊥1↓⊢∘⊖)×⊣÷⊣-⊢⌿⍤⊢)Nm
2j6 5j10

⍝ aplcart/tt.tsv:1357 — Phase of N
N←3j4 0j2 ⋄ 12∘○N   ⍝ 0.9272952180016122 1.570796326794897

⍝ aplcart/tt.tsv:1358 — Phase of N
N←3j4 0j2 ⋄ 12∘○N   ⍝ 0.9272952180016122 1.570796326794897

⍝ aplcart/tt.tsv:1359 — Pick item of vector Yv at cyclic offset Is (like ⎕IO←0, default Is:¯1)
7 (↑⌽) 'abcdef'   ⍝ 'b'

⍝ aplcart/tt.tsv:1360 — Pick item of vector Yv at cyclic offset Is (like ⎕IO←0, default Is:¯1)
Is←2 ⋄ Yv←1 2 3 4 ⋄ Is(↑⌽)Yv   ⍝ 3

⍝ aplcart/tt.tsv:1361 — Pick last item of vector Yv
Yv←(1 2⋄ 3 4 5) ⋄ (↑⌽)Yv   ⍝ 3 4 5

⍝ aplcart/tt.tsv:1362 — Pick last item of vector Yv
Yv←(1 2⋄ 3 4 5) ⋄ (↑⌽)Yv   ⍝ 3 4 5

⍝ aplcart/tt.tsv:1363 — Pick random item from vector; Reuse concrete inputs from aplcart/table.tsv:865; execute this alternate recipe independently
Yv←10 20 30 ⋄ r←(?∘≢⊃⊢)Yv ⋄ (0=≢⍴r)∧r∊Yv
1

⍝ aplcart/tt.tsv:1364 — Pick random item from vector; Reuse concrete inputs from aplcart/table.tsv:865; execute this alternate recipe independently
Yv←10 20 30 ⋄ r←(?∘≢⊃⊢)Yv ⋄ (0=≢⍴r)∧r∊Yv
1

⍝ aplcart/tt.tsv:1365 — Picking one of three values according to sign of Ms
Ms←¯1 ⋄ Yv←10 20 30 ⋄ Ms(⊢⊃⍨2+∘×⊣)Yv   ⍝ 10

⍝ aplcart/tt.tsv:1366 — Picking one of three values according to sign of Ms
Ms←¯1 ⋄ Yv←10 20 30 ⋄ Ms(⊢⊃⍨2+∘×⊣)Yv   ⍝ 10

⍝ aplcart/tt.tsv:1367 — Picking one of two values according to Bs
Bs←1 ⋄ Yv←(1 2⋄ 3 4 5) ⋄ Bs(⊢⊃⍨1+⊣)Yv   ⍝ 3 4 5

⍝ aplcart/tt.tsv:1368 — Picking one of two values according to Bs
Bs←1 ⋄ Yv←(1 2⋄ 3 4 5) ⋄ Bs(⊢⊃⍨1+⊣)Yv   ⍝ 3 4 5

⍝ aplcart/tt.tsv:1369 — Places between pairs of ones; Reuse concrete inputs from aplcart/table.tsv:757; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ (~∧≠⍀)B   ⍝ 0 0 0 0 1 0 0

⍝ aplcart/tt.tsv:1370 — Places between pairs of ones; Reuse concrete inputs from aplcart/table.tsv:757; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ (~∧≠⍀)B   ⍝ 0 0 0 0 1 0 0

⍝ aplcart/tt.tsv:1371 — Poisson distribution of states N with average M
M←2 ⋄ N←0 1 2 3 ⋄ M(*⍤-⍤⊣×*÷!⍤⊢)N
0.1353352832366127 0.2706705664732254 0.2706705664732254 0.1804470443154836

⍝ aplcart/tt.tsv:1372 — Poisson distribution of states N with average M
M←2 ⋄ N←0 1 2 3 ⋄ M(*⍤-⍤⊣×*÷!⍤⊢)N
0.1353352832366127 0.2706705664732254 0.2706705664732254 0.1804470443154836

⍝ aplcart/tt.tsv:1373 — Position of /*comments*/
D← 'a/*bc*/d/*e*/f'  ⋄ '/*'∘(≠\⍷∨¯1⌽∘⌽⍷∘⌽)D
0 1 1 1 1 1 1 0 1 1 1 1 1 0

⍝ aplcart/tt.tsv:1374 — Position of /*comments*/
D← 'a/*bc*/d/*e*/f'  ⋄ '/*'∘(≠\⍷∨¯1⌽∘⌽⍷∘⌽)D
0 1 1 1 1 1 1 0 1 1 1 1 1 0

⍝ aplcart/tt.tsv:1375 — Position of first item Y in X
X←(1 2⋄ 3 4⋄ 1 2) ⋄ Y←1 2 ⋄ X(↑∘⍸⍷⍨∘⊂)Y   ⍝ 1

⍝ aplcart/tt.tsv:1376 — Position of first item Y in X
X←(1 2⋄ 3 4⋄ 1 2) ⋄ Y←1 2 ⋄ X(↑∘⍸⍷⍨∘⊂)Y   ⍝ 1

⍝ aplcart/tt.tsv:1377 — Position of first item in X not in Y
X←1 2 3 ⋄ Y←1 3 ⋄ X(↑∘⍸∘~∊)Y   ⍝ 2

⍝ aplcart/tt.tsv:1378 — Position of first item in X not in Y
X←1 2 3 ⋄ Y←1 3 ⋄ X(↑∘⍸∘~∊)Y   ⍝ 2

⍝ aplcart/tt.tsv:1379 — Position of first occurrence of string Dv in list of strings C
'lorem' 'ipsum' 'dolor' 'sit' 'amet' ⍳∘⊂  'sit'
4

⍝ aplcart/tt.tsv:1380 — Position of first occurrence of string Dv in list of strings C
C←'cat' 'dog' 'bird' ⋄ Dv←'dog' ⋄ C⍳∘⊂Dv
2

⍝ aplcart/tt.tsv:1381 — Position of first subarray X in Y
X←1 2 ⋄ Y←3 1 2 4 ⋄ X(↑∘⍸⍷)Y   ⍝ 2

⍝ aplcart/tt.tsv:1382 — Position of first subarray X in Y
X←1 2 ⋄ Y←3 1 2 4 ⋄ X(↑∘⍸⍷)Y   ⍝ 2

⍝ aplcart/tt.tsv:1383 — Positions of item Y in X
X←1 2 1 3 1 ⋄ Y←1 ⋄ X(⍸⍷⍨∘⊂)Y   ⍝ 1 3 5

⍝ aplcart/tt.tsv:1384 — Positions of item Y in X
X←1 2 1 3 1 ⋄ Y←1 ⋄ X(⍸⍷⍨∘⊂)Y   ⍝ 1 3 5

⍝ aplcart/tt.tsv:1385 — Positions of starts of subarrays X in Y
X←1 2 ⋄ Y←1 2 3 1 2 ⋄ X(⍸⍷)Y   ⍝ 1 4

⍝ aplcart/tt.tsv:1386 — Positions of starts of subarrays X in Y
X←1 2 ⋄ Y←1 2 3 1 2 ⋄ X(⍸⍷)Y   ⍝ 1 4

⍝ aplcart/tt.tsv:1387 — Positive maximum, at least zero (also for empty N)
N←⍬ ⋄ (⌈/,∘0)N   ⍝ 0

⍝ aplcart/tt.tsv:1388 — Positive maximum, at least zero (also for empty N)
N←⍬ ⋄ (⌈/,∘0)N   ⍝ 0

⍝ aplcart/tt.tsv:1389 — Postfix vector to each row of matrix
Xm←2 3⍴⍳6 ⋄ Yv←4 5 6 ⋄ Xm(,⍤1)Yv   ⍝ 2 6⍴1 2 3 4 5 6 4 5 6 4 5 6

⍝ aplcart/tt.tsv:1390 — Postfix vector to each row of matrix
Xm←2 3⍴⍳6 ⋄ Yv←4 5 6 ⋄ Xm(,⍤1)Yv   ⍝ 2 6⍴1 2 3 4 5 6 4 5 6 4 5 6

⍝ aplcart/tt.tsv:1391 — Predicted values of least squares exponential fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←2 6 18 54 ⋄ Mv(*⊢∘⍟(⊢+.×⌹)1,∘⍪⊣)Nv
2 6.000000000000001 18 54.00000000000001

⍝ aplcart/tt.tsv:1392 — Predicted values of least squares exponential fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←2 6 18 54 ⋄ Mv(*⊢∘⍟(⊢+.×⌹)1,∘⍪⊣)Nv
2 6.000000000000001 18 54.00000000000001

⍝ aplcart/tt.tsv:1393 — Predicted values of least squares linear fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←1 3 5 7 ⋄ Mv(⊢(⊢+.×⌹)1,∘⍪⊣)Nv
1 3 5 7

⍝ aplcart/tt.tsv:1394 — Predicted values of least squares linear fit given X values Mv and Y values Nv
Mv←0 1 2 3 ⋄ Nv←1 3 5 7 ⋄ Mv(⊢(⊢+.×⌹)1,∘⍪⊣)Nv
1 3 5 7

⍝ aplcart/tt.tsv:1395 — Preface a column of 1s
(1∘, 5 6 7 8 ⋄ '' ⋄ 1∘, 3 3⍴⍳9)
(1 5 6 7 8 ⋄ '' ⋄ 3 4⍴1 1 2 3 1 4 5 6 1 7 8 9)

⍝ aplcart/tt.tsv:1396 — Preface a column of 1s
Y←2 3⍴⍳6 ⋄ 1∘,Y   ⍝ 2 4⍴1 1 2 3 1 4 5 6

⍝ aplcart/tt.tsv:1397 — Preface a row of 1s
1∘⍪ 2 3⍴8   ⍝ 3 3⍴1 1 1 8 8 8 8 8 8

⍝ aplcart/tt.tsv:1398 — Preface a row of 1s
Y←2 3⍴⍳6 ⋄ 1∘⍪Y   ⍝ 3 3⍴1 1 1 1 2 3 4 5 6

⍝ aplcart/tt.tsv:1399 — Prefix Vector: length Is with Js ones on the left, the rest zeroes
Is←6 ⋄ Js←3 ⋄ Is(⊣↑1⍴⍨⊢)Js   ⍝ 1 1 1 0 0 0

⍝ aplcart/tt.tsv:1400 — Prefix Vector: length Is with Js ones on the left, the rest zeroes
Is←6 ⋄ Js←3 ⋄ Is(⊣↑1⍴⍨⊢)Js   ⍝ 1 1 1 0 0 0

⍝ aplcart/tt.tsv:1401 — Prefix vector to each row of matrix
Xv←1 2 3 ⋄ Ym←2 3⍴⍳6 ⋄ Xv(,⍤1)Ym   ⍝ 2 6⍴1 2 3 1 2 3 1 2 3 4 5 6

⍝ aplcart/tt.tsv:1402 — Prefix vector to each row of matrix
Xv←1 2 3 ⋄ Ym←2 3⍴⍳6 ⋄ Xv(,⍤1)Ym   ⍝ 2 6⍴1 2 3 1 2 3 1 2 3 4 5 6

⍝ aplcart/tt.tsv:1403 — Prefixes
((⍳∘≢↑¨⊂) 'ABCD' ⋄ (⍳∘≢↑¨⊂) ⍪'ABCD')
((1⍴'A' ⋄ 'AB' ⋄ 'ABC' ⋄ 'ABCD') ⋄ (1 1⍴'A' ⋄ 2 1⍴'AB' ⋄ 3 1⍴'ABC' ⋄ 4 1⍴'ABCD'))

⍝ aplcart/tt.tsv:1404 — Prefixes
Y←3 1 3 2 ⋄ (⍳∘≢↑¨⊂)Y   ⍝ (1⍴3 ⋄ 3 1 ⋄ 3 1 3 ⋄ 3 1 3 2)

⍝ aplcart/tt.tsv:1405 — Prefixes of a vector
Yv←4 5 6 ⋄ (,¨,\)Yv   ⍝ (1⍴4 ⋄ 4 5 ⋄ 4 5 6)

⍝ aplcart/tt.tsv:1406 — Prefixes of a vector
Yv←4 5 6 ⋄ (,¨,\)Yv   ⍝ (1⍴4 ⋄ 4 5 ⋄ 4 5 6)

⍝ aplcart/tt.tsv:1407 — Present value of cash flows Nv at interval Ms
Ms←0.1 ⋄ Nv←100 200 300 ⋄ Ms(⊢∘⌽⊥⍨∘÷1+⊣)Nv
529.7520661157025

⍝ aplcart/tt.tsv:1408 — Present value of cash flows Nv at interval Ms
Ms←0.1 ⋄ Nv←100 200 300 ⋄ Ms(⊢∘⌽⊥⍨∘÷1+⊣)Nv
529.7520661157025

⍝ aplcart/tt.tsv:1409 — Primes until Js
Js←20 ⋄ ((⊢~ ×⌝ ⍨)1↓⍳)Js   ⍝ 2 3 5 7 11 13 17 19

⍝ aplcart/tt.tsv:1410 — Primes until Js
Js←20 ⋄ ((⊢~ ×⌝ ⍨)1↓⍳)Js   ⍝ 2 3 5 7 11 13 17 19

⍝ aplcart/tt.tsv:1411 — Probabilistic NAND
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(1-×)N   ⍝ 1 0.875 1

⍝ aplcart/tt.tsv:1412 — Probabilistic NAND
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(1-×)N   ⍝ 1 0.875 1

⍝ aplcart/tt.tsv:1413 — Probabilistic NOR
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M×⍥(1∘-)N   ⍝ 0 0.375 0

⍝ aplcart/tt.tsv:1414 — Probabilistic NOR
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M×⍥(1∘-)N   ⍝ 0 0.375 0

⍝ aplcart/tt.tsv:1415 — Probabilistic OR; Reuse concrete inputs from aplcart/table.tsv:885; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(1-×⍥(1∘-))N   ⍝ 1 0.625 1

⍝ aplcart/tt.tsv:1416 — Probabilistic OR; Reuse concrete inputs from aplcart/table.tsv:885; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(1-×⍥(1∘-))N   ⍝ 1 0.625 1

⍝ aplcart/tt.tsv:1417 — Probabilistic XNOR; Reuse concrete inputs from aplcart/table.tsv:1460; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M((1-⊣×1-⊢)×1-⊢×1-⊣)N
0 0.546875 0

⍝ aplcart/tt.tsv:1418 — Probabilistic XNOR; Reuse concrete inputs from aplcart/table.tsv:1460; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M((1-⊣×1-⊢)×1-⊢×1-⊣)N
0 0.546875 0

⍝ aplcart/tt.tsv:1419 — Probabilistic XOR; Reuse concrete inputs from aplcart/table.tsv:1522; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M((1-×)×1-(1-⊣)×1-⊢)N
1 0.546875 1

⍝ aplcart/tt.tsv:1420 — Probabilistic XOR; Reuse concrete inputs from aplcart/table.tsv:1522; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M((1-×)×1-(1-⊣)×1-⊢)N
1 0.546875 1

⍝ aplcart/tt.tsv:1421 — Probabilistic converse implication; Reuse concrete inputs from aplcart/table.tsv:1011; execute this alternate recipe independently; Rename setup bindings to the alternate recipe’s parameter names
A←0 0.5 1 ⋄ B←1 0.25 0 ⋄ A(1-⊢×1-⊣)B   ⍝ 0 0.875 1

⍝ aplcart/tt.tsv:1422 — Probabilistic converse implication; Reuse concrete inputs from aplcart/table.tsv:1011; execute this alternate recipe independently; Rename setup bindings to the alternate recipe’s parameter names
A←0 0.5 1 ⋄ B←1 0.25 0 ⋄ A(1-⊢×1-⊣)B   ⍝ 0 0.875 1

⍝ aplcart/tt.tsv:1423 — Probabilistic converse nonimplication; Reuse concrete inputs from aplcart/table.tsv:883; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(⊢×1-⊣)N   ⍝ 1 0.125 0

⍝ aplcart/tt.tsv:1424 — Probabilistic converse nonimplication; Reuse concrete inputs from aplcart/table.tsv:883; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(⊢×1-⊣)N   ⍝ 1 0.125 0

⍝ aplcart/tt.tsv:1425 — Probabilistic implication; Reuse concrete inputs from aplcart/table.tsv:1012; execute this alternate recipe independently; Rename setup bindings to the alternate recipe’s parameter names
A←0 0.5 1 ⋄ B←1 0.25 0 ⋄ A(1-⊣×1-⊢)B   ⍝ 1 0.625 0

⍝ aplcart/tt.tsv:1426 — Probabilistic implication; Reuse concrete inputs from aplcart/table.tsv:1012; execute this alternate recipe independently; Rename setup bindings to the alternate recipe’s parameter names
A←0 0.5 1 ⋄ B←1 0.25 0 ⋄ A(1-⊣×1-⊢)B   ⍝ 1 0.625 0

⍝ aplcart/tt.tsv:1427 — Probabilistic inverse (NOT)
N←4 9 16 ⋄ (1∘-)N   ⍝ ¯3 ¯8 ¯15

⍝ aplcart/tt.tsv:1428 — Probabilistic inverse (NOT)
N←4 9 16 ⋄ (1∘-)N   ⍝ ¯3 ¯8 ¯15

⍝ aplcart/tt.tsv:1429 — Probabilistic nonimplication; Reuse concrete inputs from aplcart/table.tsv:884; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(⊣×1-⊢)N   ⍝ 0 0.375 1

⍝ aplcart/tt.tsv:1430 — Probabilistic nonimplication; Reuse concrete inputs from aplcart/table.tsv:884; execute this alternate recipe independently
M←0 0.5 1 ⋄ N←1 0.25 0 ⋄ M(⊣×1-⊢)N   ⍝ 0 0.375 1

⍝ aplcart/tt.tsv:1431 — Product of N (column-wise)
(4 3⍴⍳12 ⋄ '' ⋄ ×⌿ 4 3⍴⍳12)
(4 3⍴1 2 3 4 5 6 7 8 9 10 11 12 ⋄ '' ⋄ 280 880 1944)

⍝ aplcart/tt.tsv:1432 — Product of N (column-wise)
N←2 3⍴3 1 4 1 5 2 ⋄ ×⌿N   ⍝ 3 5 8

⍝ aplcart/tt.tsv:1433 — Product of N (row-wise)
(×/ 1 2 3 4 5 ⋄ '' ⋄ 5 5⍴⍳25 ⋄ '' ⋄ ×/ 5 5⍴⍳25)
120 ('') (5 5⍴1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25) ('') (120 30240 360360 1860480 6375600)

⍝ aplcart/tt.tsv:1434 — Product of N (row-wise)
N←2 3⍴3 1 4 1 5 2 ⋄ ×/N   ⍝ 12 10

⍝ aplcart/tt.tsv:1435 — Product of common parts of matrices (matrix sum)
Mm←3 2⍴⍳6 ⋄ Nm←2 3⍴⍳6 ⋄ Mm(1 2 1 2⍉ ×⌝ )Nm
2 2⍴1 4 12 20

⍝ aplcart/tt.tsv:1436 — Product of common parts of matrices (matrix sum)
Mm←3 2⍴⍳6 ⋄ Nm←2 3⍴⍳6 ⋄ Mm(1 2 1 2⍉ ×⌝ )Nm
2 2⍴1 4 12 20

⍝ aplcart/tt.tsv:1437 — Product of polynomials with descending coefficients
Mv←1 2 ⋄ Nv←1 3 2 ⋄ Mv(+⌿∘⊃(,\0×⊣)(1↓,)¨×∘⊂)Nv
1 5 8 4

⍝ aplcart/tt.tsv:1438 — Product of polynomials with descending coefficients
Mv←1 2 ⋄ Nv←1 3 2 ⋄ Mv(+⌿∘⊃(,\0×⊣)(1↓,)¨×∘⊂)Nv
1 5 8 4

⍝ aplcart/tt.tsv:1439 — Products over subsets of Nv specified by B
Nv←2 3 5 ⋄ B←3 4⍴1 0 1 0 0 1 1 0 0 0 1 0 ⋄ Nv×.*B
2 3 30 1

⍝ aplcart/tt.tsv:1440 — Products over subsets of Nv specified by B
Nv←2 3 5 ⋄ B←3 4⍴1 0 1 0 0 1 1 0 0 0 1 0 ⋄ Nv×.*B
2 3 30 1

⍝ aplcart/tt.tsv:1441 — Progressive index of (⍳) cyclic uniques (∪) without replacement
(i ← (0~⍨∘,∘⍉⊢⌸) 'abracadabra' ⋄ 'abracadabra'[i])
(1 2 3 5 7 4 9 10 6 8 11 ⋄ 'abrcdabraaa')

⍝ aplcart/tt.tsv:1442 — Progressive index of (⍳) cyclic uniques (∪) without replacement
Y←1 2 1 3 2 1 ⋄ (0~⍨∘,∘⍉⊢⌸)Y   ⍝ 1 2 4 3 5 6

⍝ aplcart/tt.tsv:1443 — Progressive maxima (column-wise)
col←⍪20 11 47 2 5 300 99 ⋄ col, ⌈⍀ col
7 2⍴20 20 11 20 47 47 2 47 5 47 300 300 99 300

⍝ aplcart/tt.tsv:1444 — Progressive maxima (column-wise)
N←2 3⍴3 1 4 1 5 2 ⋄ ⌈⍀N   ⍝ 2 3⍴3 1 4 3 5 4

⍝ aplcart/tt.tsv:1445 — Progressive maxima (row-wise)
⌈\ 20 11 47 2 5 300 99   ⍝ 20 20 47 47 47 300 300

⍝ aplcart/tt.tsv:1446 — Progressive maxima (row-wise)
N←2 3⍴3 1 4 1 5 2 ⋄ ⌈\N   ⍝ 2 3⍴3 3 4 1 5 5

⍝ aplcart/tt.tsv:1447 — Progressive minima (column-wise)
col←⍪20 11 47 2 5 300 99 ⋄ col, ⌊⍀ col
7 2⍴20 20 11 11 47 11 2 2 5 2 300 2 99 2

⍝ aplcart/tt.tsv:1448 — Progressive minima (column-wise)
N←2 3⍴3 1 4 1 5 2 ⋄ ⌊⍀N   ⍝ 2 3⍴3 1 4 1 1 2

⍝ aplcart/tt.tsv:1449 — Progressive minima (row-wise)
⌊\ 20 11 47 2 5 300 99   ⍝ 20 11 11 2 2 2 2

⍝ aplcart/tt.tsv:1450 — Progressive minima (row-wise)
N←2 3⍴3 1 4 1 5 2 ⋄ ⌊\N   ⍝ 2 3⍴3 1 1 1 1 1

⍝ aplcart/tt.tsv:1451 — Proper divisors of Js
Js←12 ⋄ (∪⊢∨¯1↓⍳)Js   ⍝ 1 2 3 4 6

⍝ aplcart/tt.tsv:1452 — Proper divisors of Js
Js←12 ⋄ (∪⊢∨¯1↓⍳)Js   ⍝ 1 2 3 4 6

⍝ aplcart/tt.tsv:1453 — Prototype (converts characters to spaces, numbers to zeros)
Y← ('ab'⋄ 1 2)  ⋄ ↑0⍴,Y   ⍝ '  '

⍝ aplcart/tt.tsv:1454 — Prototype (converts characters to spaces, numbers to zeros)
Y← ('ab'⋄ 1 2)  ⋄ ↑0⍴,Y   ⍝ '  '

⍝ aplcart/tt.tsv:1455 — Quadratic mean
N←4 9 16 ⋄ (2*∘÷⍨1⊥×⍨÷≢)N   ⍝ 10.84742673018199

⍝ aplcart/tt.tsv:1456 — Quadratic mean
N←4 9 16 ⋄ (2*∘÷⍨1⊥×⍨÷≢)N   ⍝ 10.84742673018199

⍝ aplcart/tt.tsv:1457 — Random Boolean array of shape Jv; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Jv←2 3 ⋄ r←(1=∘?⍴∘2)Jv ⋄ (Jv≡⍴r)∧∧/,r∊0 1
1

⍝ aplcart/tt.tsv:1458 — Random Boolean array of shape Jv; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Jv←2 3 ⋄ r←(1=∘?⍴∘2)Jv ⋄ (Jv≡⍴r)∧∧/,r∊0 1
1

⍝ aplcart/tt.tsv:1459 — Random Permutation of length Js; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Js←7 ⋄ r←?⍨Js ⋄ (⍳Js)≡r[⍋r]   ⍝ 1

⍝ aplcart/tt.tsv:1460 — Random Permutation of length Js; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Js←7 ⋄ r←?⍨Js ⋄ (⍳Js)≡r[⍋r]   ⍝ 1

⍝ aplcart/tt.tsv:1461 — Random Permutation vector for Y; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Y←3 2⍴⍳6 ⋄ r←?⍨∘≢Y ⋄ (⍳≢Y)≡r[⍋r]   ⍝ 1

⍝ aplcart/tt.tsv:1462 — Random Permutation vector for Y; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
Y←3 2⍴⍳6 ⋄ r←?⍨∘≢Y ⋄ (⍳≢Y)≡r[⍋r]   ⍝ 1

⍝ aplcart/tt.tsv:1463 — Range (difference between largest and smallest element) in N
N←4 9 16 ⋄ (⌈/-⌊/)∘,N   ⍝ 12

⍝ aplcart/tt.tsv:1464 — Range (difference between largest and smallest element) in N
N←4 9 16 ⋄ (⌈/-⌊/)∘,N   ⍝ 12

⍝ aplcart/tt.tsv:1465 — Rank (number of dimensions) of Y
Y←3 1 3 2 ⋄ (≢⍴)Y   ⍝ 1

⍝ aplcart/tt.tsv:1466 — Rank (number of dimensions) of Y
Y←3 1 3 2 ⋄ (≢⍴)Y   ⍝ 1

⍝ aplcart/tt.tsv:1467 — Rate corresponding to percentage N
N←4 9 16 ⋄ (÷∘100)N   ⍝ 0.04 0.09 0.16

⍝ aplcart/tt.tsv:1468 — Rate corresponding to percentage N
N←4 9 16 ⋄ (÷∘100)N   ⍝ 0.04 0.09 0.16

⍝ aplcart/tt.tsv:1469 — Ratio of each number in a list to its predecessor: (N[2]÷N[1])(N[3]÷N[2])(N[4]÷N[3])…; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ ÷/⌽2↕N   ⍝ 2.25 1.777777777777778

⍝ aplcart/tt.tsv:1470 — Ratio of each number in a list to its predecessor: (N[2]÷N[1])(N[3]÷N[2])(N[4]÷N[3])…; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ ÷/⌽2↕N   ⍝ 2.25 1.777777777777778

⍝ aplcart/tt.tsv:1471 — Ravel order indices of elements at indices Jv in an array of dimensions Jv
Iv←2 3 ⋄ Jv←(1 1⋄ 2 1⋄ 2 3) ⋄ Iv{1+⍺∘⊥¨⍵-1}Jv
1 4 6

⍝ aplcart/tt.tsv:1472 — Ravel order indices of elements at indices Jv in an array of dimensions Jv
Iv←2 3 ⋄ Jv←(1 1⋄ 2 1⋄ 2 3) ⋄ Iv{1+⍺∘⊥¨⍵-1}Jv
1 4 6

⍝ aplcart/tt.tsv:1483 — Real part of N
N←3j4 0j2 ⋄ 9∘○N   ⍝ 3 0

⍝ aplcart/tt.tsv:1484 — Real part of N
N←3j4 0j2 ⋄ 9∘○N   ⍝ 3 0

⍝ aplcart/tt.tsv:1485 — Rectangular scale of complex N by complex factor Ms
Ms←2j3 ⋄ N←1j2 ¯3j4 ⋄ Ms((9 11○⊣)+.×9 11 ○⌝ ⊢)N
8 6

⍝ aplcart/tt.tsv:1486 — Rectangular scale of complex N by complex factor Ms
Ms←2j3 ⋄ N←1j2 ¯3j4 ⋄ Ms((9 11○⊣)+.×9 11 ○⌝ ⊢)N
8 6

⍝ aplcart/tt.tsv:1489 — Reflect counter-diagonally
Ym←2 3⍴⍳6 ⋄ ⌽∘⍉∘⌽Ym   ⍝ 3 2⍴6 3 5 2 4 1

⍝ aplcart/tt.tsv:1490 — Reflect counter-diagonally
Ym←2 3⍴⍳6 ⋄ ⌽∘⍉∘⌽Ym   ⍝ 3 2⍴6 3 5 2 4 1

⍝ aplcart/tt.tsv:1491 — Regular unit polygon of Js edges
Js←4 ⋄ (*∘π0J2×⊢÷⍨0,⍳)Js
1 0j1 ¯1 0j¯1 1

⍝ aplcart/tt.tsv:1492 — Regular unit polygon of Js edges
Js←4 ⋄ (*∘π0J2×⊢÷⍨0,⍳)Js
1 0j1 ¯1 0j¯1 1

⍝ aplcart/tt.tsv:1493 — Remove blanks in each string; optional {X} instantiated as dyadic use
D← ' a b' 'c  d ' ''  ⋄ ~∘' '¨D   ⍝ ('ab' ⋄ 'cd' ⋄ '')

⍝ aplcart/tt.tsv:1494 — Remove blanks in each string; optional {X} instantiated as dyadic use
D← ' a b' 'c  d ' ''  ⋄ ~∘' '¨D   ⍝ ('ab' ⋄ 'cd' ⋄ '')

⍝ aplcart/tt.tsv:1495 — Remove blanks in string
Dv← ' a b  c '  ⋄ (~∘' ')Dv   ⍝ 'abc'

⍝ aplcart/tt.tsv:1496 — Remove blanks in string
Dv← ' a b  c '  ⋄ (~∘' ')Dv   ⍝ 'abc'

⍝ aplcart/tt.tsv:1497 — Remove consecutive duplicate Xs's from vector Yv
Xs←0 ⋄ Yv←0 0 1 1 0 0 2 0 ⋄ Xs{⍵/⍨∨/2↕1,⍺≠⍵}Yv
0 1 1 0 2 0

⍝ aplcart/tt.tsv:1498 — Remove consecutive duplicate Xs's from vector Yv
Xs←0 ⋄ Yv←0 0 1 1 0 0 2 0 ⋄ Xs{⍵/⍨∨/2↕1,⍺≠⍵}Yv
0 1 1 0 2 0

⍝ aplcart/tt.tsv:1499 — Remove leading, multiple and trailing Xs's
Xs←0 ⋄ Yv←0 0 1 0 0 2 0 0 ⋄ Xs(1↓,⊢⍤/⍨1(⊢∨⌽)0,≠)Yv
1 0 2

⍝ aplcart/tt.tsv:1500 — Remove leading, multiple and trailing Xs's
Xs←0 ⋄ Yv←0 0 1 0 0 2 0 0 ⋄ Xs(1↓,⊢⍤/⍨1(⊢∨⌽)0,≠)Yv
1 0 2

⍝ aplcart/tt.tsv:1501 — Remove leading, trailing and duplicate blanks
Dv← '  ab  c  '  ⋄ ' '∘(1↓,⊢⍤/⍨1(⊢∨⌽)0,≠)Dv
'ab c'

⍝ aplcart/tt.tsv:1502 — Remove leading, trailing and duplicate blanks
Dv← '  ab  c  '  ⋄ ' '∘(1↓,⊢⍤/⍨1(⊢∨⌽)0,≠)Dv
'ab c'

⍝ aplcart/tt.tsv:1503 — Remove non-alphanumeric ASCII characters
Dv←'Hello, world! 123'  ⋄ (∩∘(•D,•A,•C•A))Dv
'Helloworld123'

⍝ aplcart/tt.tsv:1504 — Remove non-alphanumeric ASCII characters
Dv←'Hello, world! 123'  ⋄ (∩∘(•D,•A,•C•A))Dv
'Helloworld123'

⍝ aplcart/tt.tsv:1505 — Remove punctuation
Dv← 'Hello, world! Why?'  ⋄ (~∘'.,:;?!')Dv
'Hello world Why'

⍝ aplcart/tt.tsv:1506 — Remove punctuation
Dv← 'Hello, world! Why?'  ⋄ (~∘'.,:;?!')Dv
'Hello world Why'

⍝ aplcart/tt.tsv:1507 — Reorder X according to the order of Y
X←10 20 30 ⋄ Y←3 1 2 ⋄ X⌷⍨∘⊂∘⍋∘⍋Y   ⍝ 30 10 20

⍝ aplcart/tt.tsv:1508 — Reorder X according to the order of Y
X←10 20 30 ⋄ Y←3 1 2 ⋄ X⌷⍨∘⊂∘⍋∘⍋Y   ⍝ 30 10 20

⍝ aplcart/tt.tsv:1509 — Replace 1s in Boolean array B with their enumeration
⍸@⊢ 1 0 1 1 0 0 1 1   ⍝ 1 0 2 3 0 0 4 5

⍝ aplcart/tt.tsv:1510 — Replace 1s in Boolean array B with their enumeration
B←2 3⍴1 0 1 0 1 1 ⋄ ⍸@⊢B   ⍝ 2 3⍴1 0 2 0 3 4

⍝ aplcart/tt.tsv:1511 — Replace all blanks with dashes; Reuse concrete inputs from aplcart/table.tsv:1352; execute this alternate recipe independently; Rename setup bindings to the alternate recipe’s parameter names
Y←2 4⍴'  ab c d'  ⋄ '-'@(=∘' ')Y   ⍝ 2 4⍴'--ab-c-d'

⍝ aplcart/tt.tsv:1512 — Replace all blanks with dashes; Reuse concrete inputs from aplcart/table.tsv:1352; execute this alternate recipe independently; Rename setup bindings to the alternate recipe’s parameter names
Y←2 4⍴'  ab c d'  ⋄ '-'@(=∘' ')Y   ⍝ 2 4⍴'--ab-c-d'

⍝ aplcart/tt.tsv:1513 — Replace all occurrences of elements from Y in array Z with X
'x'@(∊∘'AEIOU') 'HELLO AND GOODBYE'   ⍝ 'HxLLx xND GxxDBYx'

⍝ aplcart/tt.tsv:1514 — Replace all occurrences of elements from Y in array Z with X
X←9 ⋄ Y←1 3 ⋄ Z←1 2 3 4 1 ⋄ X@(∊∘Y)Z   ⍝ 9 2 9 4 9

⍝ aplcart/tt.tsv:1515 — Replace backslashes with slashes
('/'@('\'∘=)) 'path\to\file'   ⍝ 'path/to/file'

⍝ aplcart/tt.tsv:1516 — Replace backslashes with slashes
Dv←'a\b\c' ⋄ ('/'@('\'∘=))Dv   ⍝ 'a/b/c'

⍝ aplcart/tt.tsv:1517 — Replace leading zeros with blanks
D← ' 00120'  ⋄ ' '@{2=⌈\' 0'⍳⍵}D   ⍝ '   120'

⍝ aplcart/tt.tsv:1518 — Replace leading zeros with blanks
D← ' 00120'  ⋄ ' '@{2=⌈\' 0'⍳⍵}D   ⍝ '   120'

⍝ aplcart/tt.tsv:1519 — Replacing all values Ys in Y with Xs; optional {X} instantiated as dyadic use
Xs←9 ⋄ Ys←1 ⋄ Y←3 1 3 2 ⋄ Xs@(Ys∘=)Y   ⍝ 3 9 3 2

⍝ aplcart/tt.tsv:1520 — Replacing all values Ys in Y with Xs; optional {X} instantiated as dyadic use
Xs←9 ⋄ Ys←1 ⋄ Y←3 1 3 2 ⋄ Xs@(Ys∘=)Y   ⍝ 3 9 3 2

⍝ aplcart/tt.tsv:1521 — Replacing first major cell of Y with Xs
Xs←99 ⋄ Y←2 3⍴⍳6 ⋄ Xs(@1)Y   ⍝ 2 3⍴99 99 99 4 5 6

⍝ aplcart/tt.tsv:1522 — Replacing first major cell of Y with Xs
Xs←99 ⋄ Y←2 3⍴⍳6 ⋄ Xs(@1)Y   ⍝ 2 3⍴99 99 99 4 5 6

⍝ aplcart/tt.tsv:1523 — Replacing last major cell of Y with Xs
Xs←9 ⋄ Y←3 2⍴⍳6 ⋄ Xs(⊖⊣@1∘⊖)Y   ⍝ 3 2⍴1 2 3 4 9 9

⍝ aplcart/tt.tsv:1524 — Replacing last major cell of Y with Xs
Xs←9 ⋄ Y←3 2⍴⍳6 ⋄ Xs(⊖⊣@1∘⊖)Y   ⍝ 3 2⍴1 2 3 4 9 9

⍝ aplcart/tt.tsv:1525 — Replacing zeroes in N with corresponding elements of M
M←10 20 30 ⋄ N←0 1 0 ⋄ M(⊢+⊣×0=⊢)N   ⍝ 10 1 30

⍝ aplcart/tt.tsv:1526 — Replacing zeroes in N with corresponding elements of M
M←10 20 30 ⋄ N←0 1 0 ⋄ M(⊢+⊣×0=⊢)N   ⍝ 10 1 30

⍝ aplcart/tt.tsv:1527 — Replicate along last axis of Y (forces / to be a function even with a function on its left)
3 1 ¯2 2 / 6 7 8 9   ⍝ 6 6 6 7 0 0 9 9

⍝ aplcart/tt.tsv:1528 — Replicate along last axis of Y (forces / to be a function even with a function on its left)
Iv←1 0 2 ⋄ Y←2 3⍴⍳6 ⋄ Iv⊢⍤/Y   ⍝ 2 3⍴1 3 3 4 6 6

⍝ aplcart/tt.tsv:1529 — Replicate along leading axis of Y (forces ⌿ to be a function even with a function on its left)
mat ← 3 4⍴⍳12 ⋄ 1 0 2 ⌿ mat   ⍝ 3 4⍴1 2 3 4 9 10 11 12 9 10 11 12

⍝ aplcart/tt.tsv:1530 — Replicate along leading axis of Y (forces ⌿ to be a function even with a function on its left)
Iv←1 2 ⋄ Y←2 3⍴⍳6 ⋄ Iv⊢⍤⌿Y   ⍝ 3 3⍴1 2 3 4 5 6 4 5 6

⍝ aplcart/tt.tsv:1531 — Reshape Yv to Is-column matrix (filled column-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⍉⊢⍴⍨⊣,÷⍨∘≢)Yv   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/tt.tsv:1532 — Reshape Yv to Is-column matrix (filled column-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⍉⊢⍴⍨⊣,÷⍨∘≢)Yv   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/tt.tsv:1533 — Reshape Yv to Is-column matrix (filled row-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⊢⍴⍨÷⍨∘≢,⊣)Yv   ⍝ 3 2⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:1534 — Reshape Yv to Is-column matrix (filled row-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⊢⍴⍨÷⍨∘≢,⊣)Yv   ⍝ 3 2⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:1535 — Reshape Yv to Is-row matrix (filled column-wise); Reuse concrete inputs from aplcart/table.tsv:1423; execute this alternate recipe independently
Is←2 ⋄ Yv←⍳6 ⋄ Is(⍉⊢⍴⍨⊣,⍨⊢∘≢÷⊣)Yv   ⍝ 2 3⍴1 3 5 2 4 6

⍝ aplcart/tt.tsv:1536 — Reshape Yv to Is-row matrix (filled column-wise); Reuse concrete inputs from aplcart/table.tsv:1423; execute this alternate recipe independently
Is←2 ⋄ Yv←⍳6 ⋄ Is(⍉⊢⍴⍨⊣,⍨⊢∘≢÷⊣)Yv   ⍝ 2 3⍴1 3 5 2 4 6

⍝ aplcart/tt.tsv:1537 — Reshape Yv to Is-row matrix (filled row-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⊢⍴⍨⊣,÷⍨∘≢)Yv   ⍝ 2 3⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:1538 — Reshape Yv to Is-row matrix (filled row-wise)
Is←2 ⋄ Yv←⍳6 ⋄ Is(⊢⍴⍨⊣,÷⍨∘≢)Yv   ⍝ 2 3⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:1539 — Reshape as in J (outer shape Iv with inner shape of major cells of Y)
Iv←2 2 ⋄ Y←2 3⍴⍳6 ⋄ Iv(⊢⍴⍨⊣,1↓⊢∘⍴)Y   ⍝ 2 2 3⍴1 2 3 4 5 6 1 2 3 4 5 6

⍝ aplcart/tt.tsv:1540 — Reshape as in J (outer shape Iv with inner shape of major cells of Y)
Iv←2 2 ⋄ Y←2 3⍴⍳6 ⋄ Iv(⊢⍴⍨⊣,1↓⊢∘⍴)Y   ⍝ 2 2 3⍴1 2 3 4 5 6 1 2 3 4 5 6

⍝ aplcart/tt.tsv:1541 — Reshaping non-empty lower-rank array Yv into a matrix
Yv←4 5 6 ⋄ (⊢⍴⍨1⌈¯2↑⍴)Yv   ⍝ 1 3⍴4 5 6

⍝ aplcart/tt.tsv:1542 — Reshaping non-empty lower-rank array Yv into a matrix
Yv←4 5 6 ⋄ (⊢⍴⍨1⌈¯2↑⍴)Yv   ⍝ 1 3⍴4 5 6

⍝ aplcart/tt.tsv:1543 — Reshaping only one-element numeric vector Nv into a scalar (leave longer vectors as-is); Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:857
Nv←,7 ⋄ (⍎⍕)Nv   ⍝ 7

⍝ aplcart/tt.tsv:1544 — Reshaping only one-element numeric vector Nv into a scalar (leave longer vectors as-is); Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:857
Nv←,7 ⋄ (⍎⍕)Nv   ⍝ 7

⍝ aplcart/tt.tsv:1545 — Reshaping vector Yv into a one-row matrix
(⍴     1 2 3 ⋄ ⍴ ⍉∘⍪ 1 2 3)   ⍝ (1⍴3 ⋄ 1 3)

⍝ aplcart/tt.tsv:1546 — Reshaping vector Yv into a one-row matrix
Yv←1 2 3 4 ⋄ ⍉∘⍪Yv   ⍝ 1 4⍴1 2 3 4

⍝ aplcart/tt.tsv:1547 — Reshaping vector Yv into a two-column matrix
Yv←1 2 3 4 5 6 ⋄ (⊢⍴⍨2,⍨2÷⍨≢)Yv   ⍝ 3 2⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:1548 — Reshaping vector Yv into a two-column matrix
Yv←1 2 3 4 5 6 ⋄ (⊢⍴⍨2,⍨2÷⍨≢)Yv   ⍝ 3 2⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:1549 — Residue after dividing N by M but replacing 0 with M; Reuse concrete inputs from aplcart/table.tsv:1112; execute this alternate recipe independently
M←3 ⋄ N←0 3 4 6 ⋄ M(⊣+|⍨∘-⍨)N   ⍝ 3 3 1 3

⍝ aplcart/tt.tsv:1550 — Residue after dividing N by M but replacing 0 with M; Reuse concrete inputs from aplcart/table.tsv:1112; execute this alternate recipe independently
M←3 ⋄ N←0 3 4 6 ⋄ M(⊣+|⍨∘-⍨)N   ⍝ 3 3 1 3

⍝ aplcart/tt.tsv:1551 — Resign: Transfer of sign from M to N
M←¯1 0 1 ⋄ N←2 ¯3 4 ⋄ M(×∘×⍨∘|)N   ⍝ ¯2 0 4

⍝ aplcart/tt.tsv:1552 — Resign: Transfer of sign from M to N
M←¯1 0 1 ⋄ N←2 ¯3 4 ⋄ M(×∘×⍨∘|)N   ⍝ ¯2 0 4

⍝ aplcart/tt.tsv:1553 — Reverse hook (S-combinator): apply f between (g Y) and Y, that is (g Y) f Y
f←+ ⋄ g←⌽ ⋄ Y←3 1 3 2 ⋄ f⍨∘g⍨Y   ⍝ 5 4 4 5

⍝ aplcart/tt.tsv:1554 — Reverse hook (S-combinator): apply f between (g Y) and Y, that is (g Y) f Y
f←+ ⋄ g←⌽ ⋄ Y←3 1 3 2 ⋄ f⍨∘g⍨Y   ⍝ 5 4 4 5

⍝ aplcart/tt.tsv:1555 — Right justify matrix Dm
Dm← 2 4⍴'ab  c   '  ⋄ (⊢⌽⍨1-1⊥⍨=∘' ')Dm   ⍝ 2 4⍴'  ab   c'

⍝ aplcart/tt.tsv:1556 — Right justify matrix Dm
Dm← 2 4⍴'ab  c   '  ⋄ (⊢⌽⍨1-1⊥⍨=∘' ')Dm   ⍝ 2 4⍴'  ab   c'

⍝ aplcart/tt.tsv:1557 — Rightmost neighbouring elements (cyclically)
Y←3 1 3 2 ⋄ 1∘⌽Y   ⍝ 1 3 2 3

⍝ aplcart/tt.tsv:1558 — Rightmost neighbouring elements (cyclically)
Y←3 1 3 2 ⋄ 1∘⌽Y   ⍝ 1 3 2 3

⍝ aplcart/tt.tsv:1559 — Rightmost neighbouring elements (padding at edge)
Y←2 3⍴⍳6 ⋄ ((1↓⊢,1↑0∘⍴)⍤1)Y   ⍝ 2 3⍴2 3 0 5 6 0

⍝ aplcart/tt.tsv:1560 — Rightmost neighbouring elements (padding at edge)
Y←2 3⍴⍳6 ⋄ ((1↓⊢,1↑0∘⍴)⍤1)Y   ⍝ 2 3⍴2 3 0 5 6 0

⍝ aplcart/tt.tsv:1561 — Rot-13; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1. Corrected ROT13 to 1+26|12+index so M does not select index zero; sample covers M/N and nonletters
{•A[1+26|12+•A⍳⍵]}@(∊∘•A)'AMNZ 123 abc!'
'NZAM 123 abc!'

⍝ aplcart/tt.tsv:1562 — Rot-13; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1. Corrected ROT13 to 1+26|12+index so M does not select index zero; sample covers M/N and nonletters
{•A[1+26|12+•A⍳⍵]}@(∊∘•A)'AMNZ 123 abc!'
'NZAM 123 abc!'

⍝ aplcart/tt.tsv:1563 — Rotate 180°
(3 3⍴⍳9 ⋄ '' ⋄ ⌽∘⊖  3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 3 3⍴9 8 7 6 5 4 3 2 1)

⍝ aplcart/tt.tsv:1564 — Rotate 180°
Ym←2 3⍴⍳6 ⋄ ⌽∘⊖Ym   ⍝ 2 3⍴6 5 4 3 2 1

⍝ aplcart/tt.tsv:1565 — Rotate 90° clockwise
(3 3⍴⍳9 ⋄ ⌽∘⍉ 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴7 4 1 8 5 2 9 6 3)

⍝ aplcart/tt.tsv:1566 — Rotate 90° clockwise
Ym←2 3⍴⍳6 ⋄ ⌽∘⍉Ym   ⍝ 3 2⍴4 1 5 2 6 3

⍝ aplcart/tt.tsv:1567 — Rotate 90° counter-clockwise
(3 3⍴⍳9 ⋄ ⍉∘⌽ 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ 3 3⍴3 6 9 2 5 8 1 4 7)

⍝ aplcart/tt.tsv:1568 — Rotate 90° counter-clockwise
Ym←2 3⍴⍳6 ⋄ ⍉∘⌽Ym   ⍝ 3 2⍴3 6 2 5 1 4

⍝ aplcart/tt.tsv:1569 — Rotate figure Nv in direction of point Ms
Ms←1j1 ⋄ Nv←1 1j1 0j1 ⋄ Ms×∘×⍨Nv
0.7071067811865475j0.7071067811865475 0j1.414213562373095 ¯0.7071067811865475j0.7071067811865475

⍝ aplcart/tt.tsv:1570 — Rotate figure Nv in direction of point Ms
Ms←1j1 ⋄ Nv←1 1j1 0j1 ⋄ Ms×∘×⍨Nv
0.7071067811865475j0.7071067811865475 0j1.414213562373095 ¯0.7071067811865475j0.7071067811865475

⍝ aplcart/tt.tsv:1571 — Rotation matrix for angle Ns (in radians) counter-clockwise
Ns←0.5 ⋄ (2 2⍴2 1 1 2-@2⍤○⊢)Ns
2 2⍴0.8775825618903728 ¯0.479425538604203 0.479425538604203 0.8775825618903728

⍝ aplcart/tt.tsv:1572 — Rotation matrix for angle Ns (in radians) counter-clockwise
Ns←0.5 ⋄ (2 2⍴2 1 1 2-@2⍤○⊢)Ns
2 2⍴0.8775825618903728 ¯0.479425538604203 0.479425538604203 0.8775825618903728

⍝ aplcart/tt.tsv:1573 — Rounding N to Is decimal places; Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:856
Is←2 ⋄ N←¯1.234 1.236 20.125 ⋄ Is(⍎⍕)N   ⍝ ¯1.23 1.24 20.13

⍝ aplcart/tt.tsv:1574 — Rounding N to Is decimal places; Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:856
Is←2 ⋄ N←¯1.234 1.236 20.125 ⋄ Is(⍎⍕)N   ⍝ ¯1.23 1.24 20.13

⍝ aplcart/tt.tsv:1575 — Rounding N to nearest M (favouring away from 0)
M←0.5 ⋄ N←¯1.25 ¯0.75 0.75 1.25 ⋄ M(⊢∘××⊣×∘⌊0.5+∘|÷⍨)N
¯1.5 ¯1 1 1.5

⍝ aplcart/tt.tsv:1576 — Rounding N to nearest M (favouring away from 0)
M←0.5 ⋄ N←¯1.25 ¯0.75 0.75 1.25 ⋄ M(⊢∘××⊣×∘⌊0.5+∘|÷⍨)N
¯1.5 ¯1 1 1.5

⍝ aplcart/tt.tsv:1577 — Rounding N to nearest M (favouring towards 0)
M←0.5 ⋄ N←¯1.25 ¯0.75 0.75 1.25 ⋄ M(⊢∘××⊣×∘⌈¯0.5+∘|÷⍨)N
¯1 ¯0.5 0.5 1

⍝ aplcart/tt.tsv:1578 — Rounding N to nearest M (favouring towards 0)
M←0.5 ⋄ N←¯1.25 ¯0.75 0.75 1.25 ⋄ M(⊢∘××⊣×∘⌈¯0.5+∘|÷⍨)N
¯1 ¯0.5 0.5 1

⍝ aplcart/tt.tsv:1579 — Rounding currencies to nearest 5 subunits
N←1.23 2.47 3.51 ⋄ 0.05∘(⊣×∘⌊0.5+÷⍨)N   ⍝ 1.25 2.45 3.5

⍝ aplcart/tt.tsv:1580 — Rounding currencies to nearest 5 subunits
N←1.23 2.47 3.51 ⋄ 0.05∘(⊣×∘⌊0.5+÷⍨)N   ⍝ 1.25 2.45 3.5

⍝ aplcart/tt.tsv:1581 — Rounding to nearest even integer (favouring up)
N←¯3 ¯2.5 0 1 2.5 3 ⋄ (⌊⊢+1≤2|⊢)N   ⍝ ¯2 ¯2 0 2 2 4

⍝ aplcart/tt.tsv:1582 — Rounding to nearest even integer (favouring up)
N←¯3 ¯2.5 0 1 2.5 3 ⋄ (⌊⊢+1≤2|⊢)N   ⍝ ¯2 ¯2 0 2 2 4

⍝ aplcart/tt.tsv:1583 — Rounding to nearest even number (favouring away from 0)
N←¯3 ¯2.5 2.5 3 ⋄ (××∘⌊|+1≤2||)N   ⍝ ¯4 ¯2 2 4

⍝ aplcart/tt.tsv:1584 — Rounding to nearest even number (favouring away from 0)
N←¯3 ¯2.5 2.5 3 ⋄ (××∘⌊|+1≤2||)N   ⍝ ¯4 ¯2 2 4

⍝ aplcart/tt.tsv:1585 — Rounding to nearest even number (favouring towards 0)
N←¯3 ¯2.5 2.5 3 ⋄ (××2×∘⌈2÷⍨1-⍨|)N   ⍝ ¯2 ¯2 2 2

⍝ aplcart/tt.tsv:1586 — Rounding to nearest even number (favouring towards 0)
N←¯3 ¯2.5 2.5 3 ⋄ (××2×∘⌈2÷⍨1-⍨|)N   ⍝ ¯2 ¯2 2 2

⍝ aplcart/tt.tsv:1587 — Rounding to nearest hundredth (favouring up)
N←¯1.234 0.125 2.678 ⋄ 0.01∘(⊣×∘⌊0.5+÷⍨)N
¯1.23 0.13 2.68

⍝ aplcart/tt.tsv:1588 — Rounding to nearest hundredth (favouring up)
N←¯1.234 0.125 2.678 ⋄ 0.01∘(⊣×∘⌊0.5+÷⍨)N
¯1.23 0.13 2.68

⍝ aplcart/tt.tsv:1589 — Rounding to nearest integer (favouring up)
(⌊0.5+⊢) 31.4 1.5 92.6   ⍝ 31 2 93

⍝ aplcart/tt.tsv:1590 — Rounding to nearest integer (favouring up)
N←¯2.5 ¯1.2 0.5 2.5 ⋄ (⌊0.5+⊢)N   ⍝ ¯2 ¯1 1 3

⍝ aplcart/tt.tsv:1591 — Rounding to nearest odd number (favouring away from 0)
N←¯4 ¯2 0 2 4 ⋄ (××∘⌊|+1>2||)N   ⍝ ¯5 ¯3 0 3 5

⍝ aplcart/tt.tsv:1592 — Rounding to nearest odd number (favouring away from 0)
N←¯4 ¯2 0 2 4 ⋄ (××∘⌊|+1>2||)N   ⍝ ¯5 ¯3 0 3 5

⍝ aplcart/tt.tsv:1593 — Rounding to nearest odd number (favouring towards 0)
N←¯4 ¯2 0 2 4 ⋄ (××¯1+2×∘⌈2÷⍨|)N   ⍝ ¯3 ¯1 0 1 3

⍝ aplcart/tt.tsv:1594 — Rounding to nearest odd number (favouring towards 0)
N←¯4 ¯2 0 2 4 ⋄ (××¯1+2×∘⌈2÷⍨|)N   ⍝ ¯3 ¯1 0 1 3

⍝ aplcart/tt.tsv:1595 — Rounding to nearest whole number (favouring away from 0)
N←¯2.5 ¯1.5 1.5 2.5 ⋄ (××∘⌊0.5+|)N   ⍝ ¯3 ¯2 2 3

⍝ aplcart/tt.tsv:1596 — Rounding to nearest whole number (favouring away from 0)
N←¯2.5 ¯1.5 1.5 2.5 ⋄ (××∘⌊0.5+|)N   ⍝ ¯3 ¯2 2 3

⍝ aplcart/tt.tsv:1597 — Rounding to nearest whole number (favouring towards 0)
N←¯2.5 ¯1.5 1.5 2.5 ⋄ (××∘⌈¯0.5+|)N   ⍝ ¯2 ¯1 1 2

⍝ aplcart/tt.tsv:1598 — Rounding to nearest whole number (favouring towards 0)
N←¯2.5 ¯1.5 1.5 2.5 ⋄ (××∘⌈¯0.5+|)N   ⍝ ¯2 ¯1 1 2

⍝ aplcart/tt.tsv:1599 — Rounding to zero values of N within M of zero
M←0.1 ⋄ N←0.05 ¯0.2 1 ⋄ M(≤∘|×⊢)N   ⍝ 0 ¯0.2 1

⍝ aplcart/tt.tsv:1600 — Rounding to zero values of N within M of zero
M←0.1 ⋄ N←0.05 ¯0.2 1 ⋄ M(≤∘|×⊢)N   ⍝ 0 ¯0.2 1

⍝ aplcart/tt.tsv:1601 — Rounding to ⎕PP precision; Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:857
Nv←,7 ⋄ (⍎⍕)Nv   ⍝ 7

⍝ aplcart/tt.tsv:1602 — Rounding to ⎕PP precision; Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:857
Nv←,7 ⋄ (⍎⍕)Nv   ⍝ 7

⍝ aplcart/tt.tsv:1603 — Rounding towards zero
N←¯2.5 0 3.75 ⋄ (××∘⌊|)N   ⍝ ¯2 0 3

⍝ aplcart/tt.tsv:1604 — Rounding towards zero
N←¯2.5 0 3.75 ⋄ (××∘⌊|)N   ⍝ ¯2 0 3

⍝ aplcart/tt.tsv:1605 — Rounding, to nearest even integer for 0.5 = 1||N
N←¯2.5 ¯1.5 0.5 1.5 2.5 ⋄ (⌊⊢+2÷⍨0.5≠2|⊢)N
¯2 ¯2 0 2 2

⍝ aplcart/tt.tsv:1606 — Rounding, to nearest even integer for 0.5 = 1||N
N←¯2.5 ¯1.5 0.5 1.5 2.5 ⋄ (⌊⊢+2÷⍨0.5≠2|⊢)N
¯2 ¯2 0 2 2

⍝ aplcart/tt.tsv:1607 — Row averages (0 if none)
N←2 0⍴0 ⋄ (+/÷1⌈0⊥⍴)N   ⍝ 0 0

⍝ aplcart/tt.tsv:1608 — Row averages (0 if none)
N←2 0⍴0 ⋄ (+/÷1⌈0⊥⍴)N   ⍝ 0 0

⍝ aplcart/tt.tsv:1609 — Row averages of non-zero elements (0 if none)
N←2 3⍴0 2 4 0 0 0 ⋄ (+/÷1⌈+.≠∘0)N   ⍝ 3 0

⍝ aplcart/tt.tsv:1610 — Row averages of non-zero elements (0 if none)
N←2 3⍴0 2 4 0 0 0 ⋄ (+/÷1⌈+.≠∘0)N   ⍝ 3 0

⍝ aplcart/tt.tsv:1611 — Row-wise alternating sum: ((N[1]-N[2])+N[3])-N[4]+…
-/ 5 10 15 20 25 30   ⍝ ¯15

⍝ aplcart/tt.tsv:1612 — Row-wise alternating sum: ((N[1]-N[2])+N[3])-N[4]+…
N←2 4⍴⍳8 ⋄ -/N   ⍝ ¯2 ¯2

⍝ aplcart/tt.tsv:1613 — Row-wise percentage per row
N←2 3⍴1 2 3 4 5 6 ⋄ (100×⊢÷⍤1 0+/)N
2 3⍴16.66666666666666 33.33333333333333 50 26.66666666666667 33.33333333333333 40

⍝ aplcart/tt.tsv:1614 — Row-wise percentage per row
N←2 3⍴1 2 3 4 5 6 ⋄ (100×⊢÷⍤1 0+/)N
2 3⍴16.66666666666666 33.33333333333333 50 26.66666666666667 33.33333333333333 40

⍝ aplcart/tt.tsv:1615 — Run f on scalars
X←1 2 ⋄ f←, ⋄ Y←3 4 ⋄ X(f⍤0)Y   ⍝ 2 2⍴1 3 2 4

⍝ aplcart/tt.tsv:1616 — Run f on scalars
X←1 2 ⋄ f←, ⋄ Y←3 4 ⋄ X(f⍤0)Y   ⍝ 2 2⍴1 3 2 4

⍝ aplcart/tt.tsv:1617 — Running sum of Is consecutive elements of N
((+/∘(2∘↕)) 2 2 3 2 2 ⋄ '' ⋄ 5 5⍴⍳9 ⋄ '' ⋄ (+/∘(3∘↕)⍤1) 5 5⍴⍳9)
(4 5 5 4 ⋄ '' ⋄ 5 5⍴1 2 3 4 5 6 7 8 9 1 2 3 4 5 6 7 8 9 1 2 3 4 5 6 7 ⋄ '' ⋄ 5 3⍴6 9 12 21 24 18 9 12 15 24 18 12 12 15 18)

⍝ aplcart/tt.tsv:1618 — Running sum of Is consecutive elements of N
Is←3 ⋄ N←1 2 3 4 5 ⋄ +/Is↕N   ⍝ 6 9 12

⍝ aplcart/tt.tsv:1619 — SWIFT check digit from Is bank number
Is←123456 ⋄ (¯97(|-⊣)⊢)Is   ⍝ 72

⍝ aplcart/tt.tsv:1620 — SWIFT check digit from Is bank number
Is←123456 ⋄ (¯97(|-⊣)⊢)Is   ⍝ 72

⍝ aplcart/tt.tsv:1621 — Safe conversion of string into integer; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
(10⊥¯1+•D∘⍳)'2718'   ⍝ 2718

⍝ aplcart/tt.tsv:1622 — Safe conversion of string into integer; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
(10⊥¯1+•D∘⍳)'2718'   ⍝ 2718

⍝ aplcart/tt.tsv:1623 — Sample Pearson correlation coefficient
Mv←1 2 3 4 ⋄ Nv←2 4 6 8 ⋄ Mv+.×⍥((⊢÷2*∘÷⍨+.×⍨)⊢-+⌿÷≢)Nv
1

⍝ aplcart/tt.tsv:1624 — Sample Pearson correlation coefficient
Mv←1 2 3 4 ⋄ Nv←2 4 6 8 ⋄ Mv+.×⍥((⊢÷2*∘÷⍨+.×⍨)⊢-+⌿÷≢)Nv
1

⍝ aplcart/tt.tsv:1625 — Sample standard deviation
Nv←1 2 4 ⋄ ((2*∘÷⍨+⌿÷¯1+≢)2*⍨⊢-+⌿÷≢)Nv   ⍝ 1.527525231651947

⍝ aplcart/tt.tsv:1626 — Sample standard deviation
Nv←1 2 4 ⋄ ((2*∘÷⍨+⌿÷¯1+≢)2*⍨⊢-+⌿÷≢)Nv   ⍝ 1.527525231651947

⍝ aplcart/tt.tsv:1627 — Sample variance
Nv←1 2 4 ⋄ (((≢×+.*∘2)-2*⍨+⌿)÷≢×1⌈¯1+≢)Nv
2.333333333333333

⍝ aplcart/tt.tsv:1628 — Sample variance
Nv←1 2 4 ⋄ (((≢×+.*∘2)-2*⍨+⌿)÷≢×1⌈¯1+≢)Nv
2.333333333333333

⍝ aplcart/tt.tsv:1633 — Scatter plot of two series (one per row of Jm)
Jm←2 3⍴1 2 3 3 1 2 ⋄ {⍉' +○⍟'[1+2⊥⍵ =⌝ ⌽⍳⌈/,⍵]}Jm
3 3⍴'+ ○ ○+○+ '

⍝ aplcart/tt.tsv:1634 — Scatter plot of two series (one per row of Jm)
Jm←2 3⍴1 2 3 3 1 2 ⋄ {⍉' +○⍟'[1+2⊥⍵ =⌝ ⌽⍳⌈/,⍵]}Jm
3 3⍴'+ ○ ○+○+ '

⍝ aplcart/tt.tsv:1637 — Secant
N←4 9 16 ⋄ (÷2∘○)N
¯1.529885656466397 ¯1.097537906304962 ¯1.044212499898521

⍝ aplcart/tt.tsv:1638 — Secant
N←4 9 16 ⋄ (÷2∘○)N
¯1.529885656466397 ¯1.097537906304962 ¯1.044212499898521

⍝ aplcart/tt.tsv:1639 — Segment lengths (excluding delimiters) in delimited string Dv where the first character is the delimiter ≢¨ ⍴¨
Dv←',ab,c,,def' ⋄ ¯1+-/⌽2↕⍸(Dv=↑Dv),1   ⍝ 2 1 0 3

⍝ aplcart/tt.tsv:1640 — Segment lengths (excluding delimiters) in delimited string Dv where the first character is the delimiter ≢¨ ⍴¨
Dv←',ab,c,,def' ⋄ ¯1+-/⌽2↕⍸(Dv=↑Dv),1   ⍝ 2 1 0 3

⍝ aplcart/tt.tsv:1641 — Segment lengths from beginning indices
Iv←2 5 9 ⋄ ((-/∘⌽∘(2∘↕))0∘,)Iv   ⍝ 2 3 4

⍝ aplcart/tt.tsv:1642 — Segment lengths from beginning indices
Iv←2 5 9 ⋄ ((-/∘⌽∘(2∘↕))0∘,)Iv   ⍝ 2 3 4

⍝ aplcart/tt.tsv:1643 — Segment lengths from ending indices
Iv←0 1 0 0 1 ⋄ ((-/∘⌽∘(2∘↕))∘⍸1∘,)Iv   ⍝ 2 3

⍝ aplcart/tt.tsv:1644 — Segment lengths from ending indices
Iv←0 1 0 0 1 ⋄ ((-/∘⌽∘(2∘↕))∘⍸1∘,)Iv   ⍝ 2 3

⍝ aplcart/tt.tsv:1645 — Select major cell of Y at cyclic offset Is (like ⎕IO←0, default Is:¯1)
Is←2 ⋄ Y←3 1 3 2 ⋄ Is(1⌷⊖)Y   ⍝ 3

⍝ aplcart/tt.tsv:1646 — Select major cell of Y at cyclic offset Is (like ⎕IO←0, default Is:¯1)
Is←2 ⋄ Y←3 1 3 2 ⋄ Is(1⌷⊖)Y   ⍝ 3

⍝ aplcart/tt.tsv:1647 — Select major cells Iv from Y; Reuse concrete inputs from aplcart/table.tsv:675; execute this alternate recipe independently
Iv←3 1 2 ⋄ Y←3 2⍴⍳6 ⋄ Iv⌷⍨∘⊂⍨Y   ⍝ 3 2⍴5 6 1 2 3 4

⍝ aplcart/tt.tsv:1648 — Select major cells Iv from Y; Reuse concrete inputs from aplcart/table.tsv:675; execute this alternate recipe independently
Iv←3 1 2 ⋄ Y←3 2⍴⍳6 ⋄ Iv⌷⍨∘⊂⍨Y   ⍝ 3 2⍴5 6 1 2 3 4

⍝ aplcart/tt.tsv:1649 — Select: each element of Iv selects a cell from Y; Reuse concrete inputs from aplcart/table.tsv:1046; execute this alternate recipe independently
Iv←(1 2⋄ 2 3) ⋄ Y←2 3⍴⍳6 ⋄ Iv(⌷⍨∘⊃⍨⍤0 99)Y
2 6

⍝ aplcart/tt.tsv:1650 — Select: each element of Iv selects a cell from Y; Reuse concrete inputs from aplcart/table.tsv:1046; execute this alternate recipe independently
Iv←(1 2⋄ 2 3) ⋄ Y←2 3⍴⍳6 ⋄ Iv(⌷⍨∘⊃⍨⍤0 99)Y
2 6

⍝ aplcart/tt.tsv:1651 — Select: each major cell of Im selects a cell from Y
Im←2 2⍴1 2 2 3 ⋄ Y←3 4⍴⍳12 ⋄ Im(⌷⍤¯1 99)Y
2 7

⍝ aplcart/tt.tsv:1652 — Select: each major cell of Im selects a cell from Y
Im←2 2⍴1 2 2 3 ⋄ Y←3 4⍴⍳12 ⋄ Im(⌷⍤¯1 99)Y
2 7

⍝ aplcart/tt.tsv:1653 — Selective picking from array
X←(1 2⋄ 2 1) ⋄ Y←(1 2⋄ 3 4) ⋄ X⊃¨⊂⊃Y   ⍝ 2 3

⍝ aplcart/tt.tsv:1654 — Selective picking from array
X←(1 2⋄ 2 1) ⋄ Y←(1 2⋄ 3 4) ⋄ X⊃¨⊂⊃Y   ⍝ 2 3

⍝ aplcart/tt.tsv:1655 — Self-classify: table of unique vs all major cells of Y
Y←1 2 1 3 ⋄ ((∪ =⌝ ⊢)⍳⍨)Y   ⍝ 3 4⍴1 0 1 0 0 1 0 0 0 0 0 1

⍝ aplcart/tt.tsv:1656 — Self-classify: table of unique vs all major cells of Y
Y←1 2 1 3 ⋄ ((∪ =⌝ ⊢)⍳⍨)Y   ⍝ 3 4⍴1 0 1 0 0 1 0 0 0 0 0 1

⍝ aplcart/tt.tsv:1657 — Separating packed YYYYMMDD date integer date
0 100 100∘⊤ 19690721   ⍝ 1969 7 21

⍝ aplcart/tt.tsv:1658 — Separating packed YYYYMMDD date integer date
Js←20260918 ⋄ 0 100 100∘⊤Js   ⍝ 2026 9 18

⍝ aplcart/tt.tsv:1659 — Set Identity (are the sets identical?)
Xv←1 2 3 ⋄ Yv←3 2 1 ⋄ Xv(∊∧.∧∊⍨)Yv   ⍝ 1

⍝ aplcart/tt.tsv:1660 — Set Identity (are the sets identical?)
Xv←1 2 3 ⋄ Yv←3 2 1 ⋄ Xv(∊∧.∧∊⍨)Yv   ⍝ 1

⍝ aplcart/tt.tsv:1661 — Shannon entropy of array ⍵
Y← 'aaabbc'  ⋄ (-1⊥2(⍟×⊢)⊢∘≢⌸÷≢)Y   ⍝ 1.459147917027245

⍝ aplcart/tt.tsv:1662 — Shannon entropy of array ⍵
Y← 'aaabbc'  ⋄ (-1⊥2(⍟×⊢)⊢∘≢⌸÷≢)Y   ⍝ 1.459147917027245

⍝ aplcart/tt.tsv:1663 — Shift after: Appending X at back/right/bottom of Y, pushing corresponding cells off the front/left/top edge
X←8 9 ⋄ Y←1 2 3 4 ⋄ X(⊢∘-∘≢↑⍪⍨)Y   ⍝ 3 4 8 9

⍝ aplcart/tt.tsv:1664 — Shift after: Appending X at back/right/bottom of Y, pushing corresponding cells off the front/left/top edge
X←8 9 ⋄ Y←1 2 3 4 ⋄ X(⊢∘-∘≢↑⍪⍨)Y   ⍝ 3 4 8 9

⍝ aplcart/tt.tsv:1665 — Shift before: Inserting X at front/left/top of Y, pushing corresponding cells off the back/right/bottom edge
X←1 2 ⋄ Y←3 4 5 6 ⋄ X(⊢∘≢↑⍪)Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:1666 — Shift before: Inserting X at front/left/top of Y, pushing corresponding cells off the back/right/bottom edge
X←1 2 ⋄ Y←3 4 5 6 ⋄ X(⊢∘≢↑⍪)Y   ⍝ 1 2 3 4

⍝ aplcart/tt.tsv:1667 — Shift each dimension of Y by corresponding amount in Iv
Iv←1 ¯1 ⋄ Y←2 3⍴⍳6 ⋄ Iv(↓↑⍨⍴⍤⊢××⍤⊣+0=⊣)Y
2 3⍴0 4 5 0 0 0

⍝ aplcart/tt.tsv:1668 — Shift each dimension of Y by corresponding amount in Iv
Iv←1 ¯1 ⋄ Y←2 3⍴⍳6 ⋄ Iv(↓↑⍨⍴⍤⊢××⍤⊣+0=⊣)Y
2 3⍴0 4 5 0 0 0

⍝ aplcart/tt.tsv:1669 — Shifting Y Is positions forward/left/up (if Is is positive, padding on right/bottom) or backward/right/down (if Is is negative, padding on left/top)
Is←¯2 ⋄ Y←1 2 3 4 5 ⋄ Is(×∘×⍨∘≢↑↓)Y   ⍝ 0 0 1 2 3

⍝ aplcart/tt.tsv:1670 — Shifting Y Is positions forward/left/up (if Is is positive, padding on right/bottom) or backward/right/down (if Is is negative, padding on left/top)
Is←¯2 ⋄ Y←1 2 3 4 5 ⋄ Is(×∘×⍨∘≢↑↓)Y   ⍝ 0 0 1 2 3

⍝ aplcart/tt.tsv:1671 — Shifting Y left/up Is positions (padding on right/bottom)
Is←2 ⋄ Y←3 1 3 2 ⋄ Is(⊢∘≢↑↓)Y   ⍝ 3 2 0 0

⍝ aplcart/tt.tsv:1672 — Shifting Y left/up Is positions (padding on right/bottom)
Is←2 ⋄ Y←3 1 3 2 ⋄ Is(⊢∘≢↑↓)Y   ⍝ 3 2 0 0

⍝ aplcart/tt.tsv:1673 — Shifting Y left/up one position (padding on right/bottom); optional {X} instantiated as dyadic use
Y←2 3⍴⍳6 ⋄ (≢↑1∘↓)Y   ⍝ 2 3⍴4 5 6 0 0 0

⍝ aplcart/tt.tsv:1674 — Shifting Y left/up one position (padding on right/bottom); optional {X} instantiated as dyadic use
Y←2 3⍴⍳6 ⋄ (≢↑1∘↓)Y   ⍝ 2 3⍴4 5 6 0 0 0

⍝ aplcart/tt.tsv:1675 — Shifting Y right/down one position (padding on left/top)
Y←2 3⍴⍳6 ⋄ (-∘≢↑¯1∘↓)Y   ⍝ 2 3⍴0 0 0 1 2 3

⍝ aplcart/tt.tsv:1676 — Shifting Y right/down one position (padding on left/top)
Y←2 3⍴⍳6 ⋄ (-∘≢↑¯1∘↓)Y   ⍝ 2 3⍴0 0 0 1 2 3

⍝ aplcart/tt.tsv:1677 — Shortest path length matrix from weighted adjacency matrix n2, using Floyd-Warshall
Nm←3 3⍴0 2 9 2 0 3 9 3 0 ⋄ (⍳⍤≢(⊢⌊⌷⍤1 +⌝ ⌷)/⍤,⊂)Nm
3 3⍴0 2 5 2 0 3 5 3 0

⍝ aplcart/tt.tsv:1678 — Shortest path length matrix from weighted adjacency matrix n2, using Floyd-Warshall
Nm←3 3⍴0 2 9 2 0 3 9 3 0 ⋄ (⍳⍤≢(⊢⌊⌷⍤1 +⌝ ⌷)/⍤,⊂)Nm
3 3⍴0 2 5 2 0 3 5 3 0

⍝ aplcart/tt.tsv:1679 — Show all digits of integer Js (unknown digits as “_”)
(3.41252E10 ⋄ (1↓0∘⍕) 3.41252E10)   ⍝ 34125200000 ('34125200000')

⍝ aplcart/tt.tsv:1680 — Show all digits of integer Js (unknown digits as “_”)
Js←123456789 ⋄ (1↓0∘⍕)Js   ⍝ '123456789'

⍝ aplcart/tt.tsv:1681 — Shuffle major cells; Reuse concrete inputs from aplcart/table.tsv:1164; execute this alternate recipe independently
Y←4 2⍴⍳8 ⋄ r←(⊂⍤?⍨∘≢⌷⊢)Y ⋄ Y≡r[⍋r;]   ⍝ 1

⍝ aplcart/tt.tsv:1682 — Shuffle major cells; Reuse concrete inputs from aplcart/table.tsv:1164; execute this alternate recipe independently
Y←4 2⍴⍳8 ⋄ r←(⊂⍤?⍨∘≢⌷⊢)Y ⋄ Y≡r[⍋r;]   ⍝ 1

⍝ aplcart/tt.tsv:1683 — Sign of difference (¯1:M is smaller, 0:M=N, 1:M is bigger)
M←2 3 4 ⋄ N←4 9 16 ⋄ M(×-)N   ⍝ ¯1 ¯1 ¯1

⍝ aplcart/tt.tsv:1684 — Sign of difference (¯1:M is smaller, 0:M=N, 1:M is bigger)
M←2 3 4 ⋄ N←4 9 16 ⋄ M(×-)N   ⍝ ¯1 ¯1 ¯1

⍝ aplcart/tt.tsv:1687 — Sine N
N←0.25 0.5 0.75 ⋄ 1∘○N
0.2474039592545229 0.479425538604203 0.6816387600233341

⍝ aplcart/tt.tsv:1688 — Sine N
N←0.25 0.5 0.75 ⋄ 1∘○N
0.2474039592545229 0.479425538604203 0.6816387600233341

⍝ aplcart/tt.tsv:1691 — Skew N in y-axis by fraction Ms
Ms←0.5 ⋄ N←1j2 3j4 ⋄ Ms(⊢+¯11○⊣×9○⊢)N   ⍝ 1j2.5 3j5.5

⍝ aplcart/tt.tsv:1692 — Skew N in y-axis by fraction Ms
Ms←0.5 ⋄ N←1j2 3j4 ⋄ Ms(⊢+¯11○⊣×9○⊢)N   ⍝ 1j2.5 3j5.5

⍝ aplcart/tt.tsv:1693 — Smallest column-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌊⌿|)N   ⍝ 1 2 3

⍝ aplcart/tt.tsv:1694 — Smallest column-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌊⌿|)N   ⍝ 1 2 3

⍝ aplcart/tt.tsv:1695 — Smallest row-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌊/|)N   ⍝ 1 4

⍝ aplcart/tt.tsv:1696 — Smallest row-wise magnitude found in N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (⌊/|)N   ⍝ 1 4

⍝ aplcart/tt.tsv:1697 — Solutions of quadratic equation Nv₁x²+Nv₂x+Nv₃=0
Nv←1 ¯5 6 ⋄ (↑÷¯2÷2∘⊃-¯1 1×2*∘÷⍨(×⍨2∘⊃)-(×/4@2))Nv
2 3

⍝ aplcart/tt.tsv:1698 — Solutions of quadratic equation Nv₁x²+Nv₂x+Nv₃=0
Nv←1 ¯5 6 ⋄ (↑÷¯2÷2∘⊃-¯1 1×2*∘÷⍨(×⍨2∘⊃)-(×/4@2))Nv
2 3

⍝ aplcart/tt.tsv:1699 — Sort A according to Ms (1: ascending, 0: unordered, ¯1: descending)
Ms←¯1 ⋄ A←3 1 2 ⋄ Ms{⍵⌷⍨⊂⍋⍋⍺×⍋⍵}A   ⍝ 1 3 2

⍝ aplcart/tt.tsv:1700 — Sort A according to Ms (1: ascending, 0: unordered, ¯1: descending)
Ms←¯1 ⋄ A←3 1 2 ⋄ Ms{⍵⌷⍨⊂⍋⍋⍺×⍋⍵}A   ⍝ 1 3 2

⍝ aplcart/tt.tsv:1701 — Sort Ascending; Reuse concrete inputs from aplcart/table.tsv:871; execute this alternate recipe independently
Y←3 1 3 2 ⋄ (⊂∘⍋⌷⊢)Y   ⍝ 1 2 3 3

⍝ aplcart/tt.tsv:1702 — Sort Ascending; Reuse concrete inputs from aplcart/table.tsv:871; execute this alternate recipe independently
Y←3 1 3 2 ⋄ (⊂∘⍋⌷⊢)Y   ⍝ 1 2 3 3

⍝ aplcart/tt.tsv:1703 — Sort Descending; Reuse concrete inputs from aplcart/table.tsv:876; execute this alternate recipe independently
Y←3 1 3 2 ⋄ (⊂∘⍒⌷⊢)Y   ⍝ 3 3 2 1

⍝ aplcart/tt.tsv:1704 — Sort Descending; Reuse concrete inputs from aplcart/table.tsv:876; execute this alternate recipe independently
Y←3 1 3 2 ⋄ (⊂∘⍒⌷⊢)Y   ⍝ 3 3 2 1

⍝ aplcart/tt.tsv:1705 — Sort Y ascending according to column Is; Reuse concrete inputs from aplcart/table.tsv:1321; execute this alternate recipe independently
Is←2 ⋄ Y←3 2⍴10 3 20 1 30 2 ⋄ Is(⊢⌷⍨∘⊂∘⍋⌷⍤1)Y
3 2⍴20 1 30 2 10 3

⍝ aplcart/tt.tsv:1706 — Sort Y ascending according to column Is; Reuse concrete inputs from aplcart/table.tsv:1321; execute this alternate recipe independently
Is←2 ⋄ Y←3 2⍴10 3 20 1 30 2 ⋄ Is(⊢⌷⍨∘⊂∘⍋⌷⍤1)Y
3 2⍴20 1 30 2 10 3

⍝ aplcart/tt.tsv:1707 — Sort Y descending according to column Is; Reuse concrete inputs from aplcart/table.tsv:1323; execute this alternate recipe independently
Is←2 ⋄ Y←3 2⍴10 3 20 1 30 2 ⋄ Is(⊢⌷⍨∘⊂∘⍒⌷⍤1)Y
3 2⍴10 3 30 2 20 1

⍝ aplcart/tt.tsv:1708 — Sort Y descending according to column Is; Reuse concrete inputs from aplcart/table.tsv:1323; execute this alternate recipe independently
Is←2 ⋄ Y←3 2⍴10 3 20 1 30 2 ⋄ Is(⊢⌷⍨∘⊂∘⍒⌷⍤1)Y
3 2⍴10 3 30 2 20 1

⍝ aplcart/tt.tsv:1709 — Sort each column in ascending order; Reuse concrete inputs from aplcart/table.tsv:1477; execute this alternate recipe independently
Y←2 3 2⍴3 2 1 4 2 1 5 6 4 5 6 4 ⋄ ({⍉(⊂∘⍋⌷⊢)⍤1⍉⍵}⍤2)Y
2 3 2⍴1 1 2 2 3 4 4 4 5 5 6 6

⍝ aplcart/tt.tsv:1710 — Sort each column in ascending order; Reuse concrete inputs from aplcart/table.tsv:1477; execute this alternate recipe independently
Y←2 3 2⍴3 2 1 4 2 1 5 6 4 5 6 4 ⋄ ({⍉(⊂∘⍋⌷⊢)⍤1⍉⍵}⍤2)Y
2 3 2⍴1 1 2 2 3 4 4 4 5 5 6 6

⍝ aplcart/tt.tsv:1711 — Sort each row in ascending order; Reuse concrete inputs from aplcart/table.tsv:1202; execute this alternate recipe independently
Y←2 3⍴3 1 2 2 3 1 ⋄ ((⊂∘⍋⌷⊢)⍤1)Y   ⍝ 2 3⍴1 2 3 1 2 3

⍝ aplcart/tt.tsv:1712 — Sort each row in ascending order; Reuse concrete inputs from aplcart/table.tsv:1202; execute this alternate recipe independently
Y←2 3⍴3 1 2 2 3 1 ⋄ ((⊂∘⍋⌷⊢)⍤1)Y   ⍝ 2 3⍴1 2 3 1 2 3

⍝ aplcart/tt.tsv:1713 — Sort major cells ascending; Reuse concrete inputs from aplcart/table.tsv:1203; execute this alternate recipe independently
Y←2 3⍴3 1 2 2 3 1 ⋄ ((⊂∘⍋⌷⊢)⍤¯1)Y   ⍝ 2 3⍴1 2 3 1 2 3

⍝ aplcart/tt.tsv:1714 — Sort major cells ascending; Reuse concrete inputs from aplcart/table.tsv:1203; execute this alternate recipe independently
Y←2 3⍴3 1 2 2 3 1 ⋄ ((⊂∘⍋⌷⊢)⍤¯1)Y   ⍝ 2 3⍴1 2 3 1 2 3

⍝ aplcart/tt.tsv:1715 — Sort major cells descending; Reuse concrete inputs from aplcart/table.tsv:1205; execute this alternate recipe independently
Y←2 3⍴3 1 2 2 3 1 ⋄ ((⊂∘⍒⌷⊢)⍤¯1)Y   ⍝ 2 3⍴3 2 1 3 2 1

⍝ aplcart/tt.tsv:1716 — Sort major cells descending; Reuse concrete inputs from aplcart/table.tsv:1205; execute this alternate recipe independently
Y←2 3⍴3 1 2 2 3 1 ⋄ ((⊂∘⍒⌷⊢)⍤¯1)Y   ⍝ 2 3⍴3 2 1 3 2 1

⍝ aplcart/tt.tsv:1717 — Sorted frequency table; Reuse concrete inputs from aplcart/table.tsv:1510; execute this alternate recipe independently
Y←1 2 1 3 1 2 ⋄ ({⍵⌷⍨⊂⍒⊢/⍵}{⍺,≢⍵}⌸)Y   ⍝ 3 2⍴1 3 2 2 3 1

⍝ aplcart/tt.tsv:1718 — Sorted frequency table; Reuse concrete inputs from aplcart/table.tsv:1510; execute this alternate recipe independently
Y←1 2 1 3 1 2 ⋄ ({⍵⌷⍨⊂⍒⊢/⍵}{⍺,≢⍵}⌸)Y   ⍝ 3 2⍴1 3 2 2 3 1

⍝ aplcart/tt.tsv:1719 — Sorting Y according to X; Reuse concrete inputs from aplcart/table.tsv:874; execute this alternate recipe independently
X← 'cba'  ⋄ Y← 'abca'  ⋄ X(⌷⍨∘⊂∘⍋⍨)Y   ⍝ 'cba'

⍝ aplcart/tt.tsv:1720 — Sorting Y according to X; Reuse concrete inputs from aplcart/table.tsv:874; execute this alternate recipe independently
X← 'cba'  ⋄ Y← 'abca'  ⋄ X(⌷⍨∘⊂∘⍋⍨)Y   ⍝ 'cba'

⍝ aplcart/tt.tsv:1721 — Sorting indices Jv according to data X
X←30 10 20 40 ⋄ Jv←1 3 2 ⋄ X(⊂⍤⍋⍤⌷⍨∘⊂⌷⊢)Jv
2 3 1

⍝ aplcart/tt.tsv:1722 — Sorting indices Jv according to data X
X←30 10 20 40 ⋄ Jv←1 3 2 ⋄ X(⊂⍤⍋⍤⌷⍨∘⊂⌷⊢)Jv
2 3 1

⍝ aplcart/tt.tsv:1723 — Split Yv (which has to be simple) at occurrences of Xs (removes separators and keeps empty segments)
Xs←0 ⋄ Yv←0 1 2 0 0 3 0 ⋄ Xs(1↓¨,⊂⍨1,=)Yv
(⍬ ⋄ 1 2 ⋄ ⍬ ⋄ 1⍴3 ⋄ ⍬)

⍝ aplcart/tt.tsv:1724 — Split Yv (which has to be simple) at occurrences of Xs (removes separators and keeps empty segments)
Xs←0 ⋄ Yv←0 1 2 0 0 3 0 ⋄ Xs(1↓¨,⊂⍨1,=)Yv
(⍬ ⋄ 1 2 ⋄ ⍬ ⋄ 1⍴3 ⋄ ⍬)

⍝ aplcart/tt.tsv:1725 — Split Yv at occurrences of Xs (removes separators and empty segments)
'/' (≠⊆⊢) 'hello/there/world'   ⍝ ('hello' ⋄ 'there' ⋄ 'world')

⍝ aplcart/tt.tsv:1726 — Split Yv at occurrences of Xs (removes separators and empty segments)
Xs←0 ⋄ Yv←0 1 2 0 0 3 0 ⋄ Xs(≠⊆⊢)Yv   ⍝ (1 2 ⋄ 1⍴3)

⍝ aplcart/tt.tsv:1727 — Split Yv at occurrences of Xv (removes separators and keeps empty segments)
Xv←0 9 ⋄ Yv←0 9 1 2 0 9 0 9 3 0 9 ⋄ Xv(≢⍤⊣↓¨,⊂⍨⊣⍷,)Yv
(⍬ ⋄ 1 2 ⋄ ⍬ ⋄ 1⍴3 ⋄ ⍬)

⍝ aplcart/tt.tsv:1728 — Split Yv at occurrences of Xv (removes separators and keeps empty segments)
Xv←0 9 ⋄ Yv←0 9 1 2 0 9 0 9 3 0 9 ⋄ Xv(≢⍤⊣↓¨,⊂⍨⊣⍷,)Yv
(⍬ ⋄ 1 2 ⋄ ⍬ ⋄ 1⍴3 ⋄ ⍬)

⍝ aplcart/tt.tsv:1729 — Split Yv at occurrences of sequences of elements in Xv (removes separators and empty segments)
Xv←0 9 ⋄ Yv←0 1 2 9 0 3 0 ⋄ Xv(~⍤∊⍨⊆⊢)Yv
(1 2 ⋄ 1⍴3)

⍝ aplcart/tt.tsv:1730 — Split Yv at occurrences of sequences of elements in Xv (removes separators and empty segments)
Xv←0 9 ⋄ Yv←0 1 2 9 0 3 0 ⋄ Xv(~⍤∊⍨⊆⊢)Yv
(1 2 ⋄ 1⍴3)

⍝ aplcart/tt.tsv:1731 — Split complex array into two-element vector of magnitude array and angle array
N←1j2 ¯3j4 ⋄ (10 12○⊂)N
(2.23606797749979 5 ⋄ 1.10714871779409 2.214297435588181)

⍝ aplcart/tt.tsv:1732 — Split complex array into two-element vector of magnitude array and angle array
N←1j2 ¯3j4 ⋄ (10 12○⊂)N
(2.23606797749979 5 ⋄ 1.10714871779409 2.214297435588181)

⍝ aplcart/tt.tsv:1733 — Split complex array into two-element vector of real array and imaginary array
N←1j2 ¯3j4 ⋄ (9 11○⊂)N   ⍝ (1 ¯3 ⋄ 2 4)

⍝ aplcart/tt.tsv:1734 — Split complex array into two-element vector of real array and imaginary array
N←1j2 ¯3j4 ⋄ (9 11○⊂)N   ⍝ (1 ¯3 ⋄ 2 4)

⍝ aplcart/tt.tsv:1735 — Split-Compose (D₂-combinator): apply g between (f X) and (h Y), that is (f X) g (h Y); Reuse concrete inputs from aplcart/table.tsv:585; execute this alternate recipe independently
f←+/ ⋄ g←- ⋄ h←×/ ⋄ X←1 2 3 ⋄ Y←2 3 4 ⋄ X g⍨∘f⍨∘h Y
¯18

⍝ aplcart/tt.tsv:1736 — Split-Compose (D₂-combinator): apply g between (f X) and (h Y), that is (f X) g (h Y); Reuse concrete inputs from aplcart/table.tsv:585; execute this alternate recipe independently
f←+/ ⋄ g←- ⋄ h←×/ ⋄ X←1 2 3 ⋄ Y←2 3 4 ⋄ X g⍨∘f⍨∘h Y
¯18

⍝ aplcart/tt.tsv:1737 — Square Root; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (*∘0.5)N   ⍝ 2 3 4

⍝ aplcart/tt.tsv:1738 — Square Root; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (*∘0.5)N   ⍝ 2 3 4

⍝ aplcart/tt.tsv:1739 — Square matrix with Yv as columns
 ⊣⌝ ⍨ 1 2 3 4   ⍝ 4 4⍴1 1 1 1 2 2 2 2 3 3 3 3 4 4 4 4

⍝ aplcart/tt.tsv:1740 — Square matrix with Yv as columns
Yv←1 2 3 4 ⋄  ⊣⌝ ⍨Yv   ⍝ 4 4⍴1 1 1 1 2 2 2 2 3 3 3 3 4 4 4 4

⍝ aplcart/tt.tsv:1741 — Square matrix with Yv as rows
 ⊢⌝ ⍨ 1 2 3 4   ⍝ 4 4⍴1 2 3 4 1 2 3 4 1 2 3 4 1 2 3 4

⍝ aplcart/tt.tsv:1742 — Square matrix with Yv as rows
Yv←1 2 3 4 ⋄  ⊢⌝ ⍨Yv   ⍝ 4 4⍴1 2 3 4 1 2 3 4 1 2 3 4 1 2 3 4

⍝ aplcart/tt.tsv:1743 — Square without changing sign
N←¯3 ¯2 0 2 3 ⋄ ×∘|⍨N   ⍝ ¯9 ¯4 0 4 9

⍝ aplcart/tt.tsv:1744 — Square without changing sign
N←¯3 ¯2 0 2 3 ⋄ ×∘|⍨N   ⍝ ¯9 ¯4 0 4 9

⍝ aplcart/tt.tsv:1745 — Square: N*2
(×⍨ 5 ⋄ ×⍨ ⍳10 ⋄ ×⍨ ¯1 ¯2.5 0 2.5 4.3j1.1 ⋄ ×⍨ (3 3⍴⍳9))
25 (1 4 9 16 25 36 49 64 81 100) (1 6.25 0 6.25 17.28j9.46) (3 3⍴1 4 9 16 25 36 49 64 81)

⍝ aplcart/tt.tsv:1746 — Square: N*2
N←4 9 16 ⋄ ×⍨N   ⍝ 16 81 256

⍝ aplcart/tt.tsv:1747 — Start and length of groups of 1s in Bv
Bv←0 1 1 0 1 1 1 0 ⋄ p←⍸≠/2↕0,Bv,0 ⋄ -⍨\((≢p)÷2) 2⍴p
2 2⍴2 2 5 3

⍝ aplcart/tt.tsv:1748 — Start and length of groups of 1s in Bv
Bv←0 1 1 0 1 1 1 0 ⋄ p←⍸≠/2↕0,Bv,0 ⋄ -⍨\((≢p)÷2) 2⍴p
2 2⍴2 2 5 3

⍝ aplcart/tt.tsv:1749 — Starting points for Is fields of width Js
Is←2 ⋄ Js←4 ⋄ Is(×⍴1↑⍨⊢)Js   ⍝ 1 0 0 0 1 0 0 0

⍝ aplcart/tt.tsv:1750 — Starting points for Is fields of width Js
Is←2 ⋄ Js←4 ⋄ Is(×⍴1↑⍨⊢)Js   ⍝ 1 0 0 0 1 0 0 0

⍝ aplcart/tt.tsv:1751 — Starting points for Y in indices pointed by Iv
Iv←2 1 ⋄ Y←3 1 3 2 ⋄ Iv∊∘⍳∘≢Y   ⍝ 1 1

⍝ aplcart/tt.tsv:1752 — Starting points for Y in indices pointed by Iv
Iv←2 1 ⋄ Y←3 1 3 2 ⋄ Iv∊∘⍳∘≢Y   ⍝ 1 1

⍝ aplcart/tt.tsv:1753 — Starting positions of subvectors having lengths Jv
Jv←2 0 3 ⋄ (+\¯1↓1∘,)Jv   ⍝ 1 3 3

⍝ aplcart/tt.tsv:1754 — Starting positions of subvectors having lengths Jv
Jv←2 0 3 ⋄ (+\¯1↓1∘,)Jv   ⍝ 1 3 3

⍝ aplcart/tt.tsv:1755 — State of switch given Bv on and Av off spikes
Av←0 0 1 0 0 1 ⋄ Bv←1 0 0 0 1 0 ⋄ Av(≠\∨{⍺\(≠/∘(2∘↕))0,⍺/⍵}⊢)Bv
1 1 0 0 1 0

⍝ aplcart/tt.tsv:1756 — State of switch given Bv on and Av off spikes
Av←0 0 1 0 0 1 ⋄ Bv←1 0 0 0 1 0 ⋄ Av(≠\∨{⍺\(≠/∘(2∘↕))0,⍺/⍵}⊢)Bv
1 1 0 0 1 0

⍝ aplcart/tt.tsv:1757 — Stereo pair (Eye separation Ms)
Ms←0.5 ⋄ N←1j2 3j4 ⋄ Ms(⊢∘⊂+¯0.5 0.5×⊣)N
(0.75j2 2.75j4 ⋄ 1.25j2 3.25j4)

⍝ aplcart/tt.tsv:1758 — Stereo pair (Eye separation Ms)
Ms←0.5 ⋄ N←1j2 3j4 ⋄ Ms(⊢∘⊂+¯0.5 0.5×⊣)N
(0.75j2 2.75j4 ⋄ 1.25j2 3.25j4)

⍝ aplcart/tt.tsv:1759 — Stochastic rounding to integer; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
N←¯2 ¯1.25 0 0.75 3 ⋄ r←(⌊+1∘|>∘?0∘×)N ⋄ ((⍴N)≡⍴r)∧∧/(r=⌊N)∨r=⌈N
1

⍝ aplcart/tt.tsv:1760 — Stochastic rounding to integer; Reviewed deterministic random-operation invariant; independently checked in Dyalog and through the Rust reference worker; Assert shape, range, uniqueness, permutation preservation, rounding bounds or deterministic sorting; no sampled values or seed are compared
N←¯2 ¯1.25 0 0.75 3 ⋄ r←(⌊+1∘|>∘?0∘×)N ⋄ ((⍴N)≡⍴r)∧∧/(r=⌊N)∨r=⌈N
1

⍝ aplcart/tt.tsv:1763 — Strictly negative?
(0∘> 1 2 3 4 ⋄ 0∘> ¯2 ¯1 0 1 2)   ⍝ (0 0 0 0 ⋄ 1 1 0 0 0)

⍝ aplcart/tt.tsv:1764 — Strictly negative?
N←¯2 0 3 ⋄ 0∘>N   ⍝ 1 0 0

⍝ aplcart/tt.tsv:1765 — Strictly positive?
(0∘< 1 2 3 4 ⋄ 0∘< ¯2 ¯1 0 1 2)   ⍝ (1 1 1 1 ⋄ 0 0 0 1 1)

⍝ aplcart/tt.tsv:1766 — Strictly positive?
N←¯2 0 3 ⋄ 0∘<N   ⍝ 0 0 1

⍝ aplcart/tt.tsv:1767 — Students grades given score; Reuse concrete inputs from aplcart/table.tsv:1342; execute this alternate recipe independently
J←55 65 75 85 95 ⋄ ('FDCBA'⌷⍨∘⊂0 60 70 80 90∘⍸)J
'FDCBA'

⍝ aplcart/tt.tsv:1768 — Students grades given score; Reuse concrete inputs from aplcart/table.tsv:1342; execute this alternate recipe independently
J←55 65 75 85 95 ⋄ ('FDCBA'⌷⍨∘⊂0 60 70 80 90∘⍸)J
'FDCBA'

⍝ aplcart/tt.tsv:1769 — Suffix Vector: length Is with Js ones on the right, the rest zeroes
Is←6 ⋄ Js←3 ⋄ Is(-⍤⊣↑1⍴⍨⊢)Js   ⍝ 0 0 0 1 1 1

⍝ aplcart/tt.tsv:1770 — Suffix Vector: length Is with Js ones on the right, the rest zeroes
Is←6 ⋄ Js←3 ⋄ Is(-⍤⊣↑1⍴⍨⊢)Js   ⍝ 0 0 0 1 1 1

⍝ aplcart/tt.tsv:1771 — Suffixes of a vector
Yv←1 2 3 4 ⋄ (⌽∘,¨,\∘⌽)Yv   ⍝ (1⍴4 ⋄ 3 4 ⋄ 2 3 4 ⋄ 1 2 3 4)

⍝ aplcart/tt.tsv:1772 — Suffixes of a vector
Yv←1 2 3 4 ⋄ (⌽∘,¨,\∘⌽)Yv   ⍝ (1⍴4 ⋄ 3 4 ⋄ 2 3 4 ⋄ 1 2 3 4)

⍝ aplcart/tt.tsv:1773 — Sum Nv by buckets Xv (⍴Nv ↔ ⍴Xv)
Xv←1 2 1 2 ⋄ Nv←10 20 30 40 ⋄ Xv{+/⍵}⌸Nv
40 60

⍝ aplcart/tt.tsv:1774 — Sum Nv by buckets Xv (⍴Nv ↔ ⍴Xv)
Xv←1 2 1 2 ⋄ Nv←10 20 30 40 ⋄ Xv{+/⍵}⌸Nv
40 60

⍝ aplcart/tt.tsv:1775 — Sum all elements in an array; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (+/,)N   ⍝ 29

⍝ aplcart/tt.tsv:1776 — Sum all elements in an array; optional {X} instantiated as dyadic use
N←4 9 16 ⋄ (+/,)N   ⍝ 29

⍝ aplcart/tt.tsv:1777 — Sum of M'th powers of positive divisors of Js
M←1 2 ⋄ Js←6 ⋄ M(+⌿⊣ *⌝ ⍨∘∪⊢∨⊢∘⍳)Js   ⍝ 12 50

⍝ aplcart/tt.tsv:1778 — Sum of M'th powers of positive divisors of Js
M←1 2 ⋄ Js←6 ⋄ M(+⌿⊣ *⌝ ⍨∘∪⊢∨⊢∘⍳)Js   ⍝ 12 50

⍝ aplcart/tt.tsv:1779 — Sum of N (column-wise)
(3 3⍴1 2 3 ⋄ '' ⋄ +⌿ 3 3⍴1 2 3)   ⍝ (3 3⍴1 2 3 1 2 3 1 2 3 ⋄ '' ⋄ 3 6 9)

⍝ aplcart/tt.tsv:1780 — Sum of N (column-wise)
N←2 3⍴⍳6 ⋄ +⌿N   ⍝ 5 7 9

⍝ aplcart/tt.tsv:1781 — Sum of N (row-wise)
(+/ 1 2 3 4 5 ⋄ '' ⋄ 3 3⍴⍳9 ⋄ '' ⋄ +/ 3 3⍴⍳9)
15 ('') (3 3⍴1 2 3 4 5 6 7 8 9) ('') (6 15 24)

⍝ aplcart/tt.tsv:1782 — Sum of N (row-wise)
N←2 3⍴⍳6 ⋄ +/N   ⍝ 6 15

⍝ aplcart/tt.tsv:1783 — Sum of all atoms in N; optional {X} instantiated as dyadic use
N←(1 2⋄ 3(4 5)) ⋄ (+/∊)N   ⍝ 15

⍝ aplcart/tt.tsv:1784 — Sum of all atoms in N; optional {X} instantiated as dyadic use
N←(1 2⋄ 3(4 5)) ⋄ (+/∊)N   ⍝ 15

⍝ aplcart/tt.tsv:1785 — Sum of alternating reciprocal series Mv÷Nv
Mv←1 2 3 ⋄ Nv←1 2 3 ⋄ Mv-.÷Nv   ⍝ 1

⍝ aplcart/tt.tsv:1786 — Sum of alternating reciprocal series Mv÷Nv
Mv←1 2 3 ⋄ Nv←1 2 3 ⋄ Mv-.÷Nv   ⍝ 1

⍝ aplcart/tt.tsv:1787 — Sum of common parts of matrices (matrix sum)
Mm←3 2⍴⍳6 ⋄ Nm←2 3⍴⍳6 ⋄ Mm(1 2 1 2⍉ +⌝ )Nm
2 2⍴2 4 7 9

⍝ aplcart/tt.tsv:1788 — Sum of common parts of matrices (matrix sum)
Mm←3 2⍴⍳6 ⋄ Nm←2 3⍴⍳6 ⋄ Mm(1 2 1 2⍉ +⌝ )Nm
2 2⍴2 4 7 9

⍝ aplcart/tt.tsv:1789 — Sum of magnitude of N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (+⌿|)N   ⍝ 5 7 9

⍝ aplcart/tt.tsv:1790 — Sum of magnitude of N; optional {X} instantiated as dyadic use
N←2 3⍴¯1 2 ¯3 4 ¯5 6 ⋄ (+⌿|)N   ⍝ 5 7 9

⍝ aplcart/tt.tsv:1793 — Sum of positive divisors of Js
(+/∘∪⊢∨⍳) 12   ⍝ 28

⍝ aplcart/tt.tsv:1794 — Sum of positive divisors of Js; Reuse concrete inputs from aplcart/table.tsv:1261; execute this alternate recipe independently
Js←12 ⋄ (+/∘∪⊢∨⍳)Js   ⍝ 28

⍝ aplcart/tt.tsv:1797 — Sum of reciprocal series Mv÷Nv
Mv←1 2 3 ⋄ Nv←1 2 3 ⋄ Mv+.÷Nv   ⍝ 3

⍝ aplcart/tt.tsv:1798 — Sum of reciprocal series Mv÷Nv
Mv←1 2 3 ⋄ Nv←1 2 3 ⋄ Mv+.÷Nv   ⍝ 3

⍝ aplcart/tt.tsv:1799 — Sum of squares of Nv
Nv←1 2 3 ⋄ +.×⍨Nv   ⍝ 14

⍝ aplcart/tt.tsv:1800 — Sum of squares of Nv
Nv←1 2 3 ⋄ +.×⍨Nv   ⍝ 14

⍝ aplcart/tt.tsv:1801 — Summation over subsets of Nv specified by Av
Av←1 0 1 ⋄ Nv←10 20 30 ⋄ Av+.×⍨Nv   ⍝ 40

⍝ aplcart/tt.tsv:1802 — Summation over subsets of Nv specified by Av
Av←1 0 1 ⋄ Nv←10 20 30 ⋄ Av+.×⍨Nv   ⍝ 40

⍝ aplcart/tt.tsv:1803 — Surround matrix Ym with scalar Xs
Xs←9 ⋄ Ym←2 3⍴⍳6 ⋄ Xs(,∘⌽∘⍉⍣4)Ym
4 5⍴9 9 9 9 9 9 1 2 3 9 9 4 5 6 9 9 9 9 9 9

⍝ aplcart/tt.tsv:1804 — Surround matrix Ym with scalar Xs
Xs←9 ⋄ Ym←2 3⍴⍳6 ⋄ Xs(,∘⌽∘⍉⍣4)Ym
4 5⍴9 9 9 9 9 9 1 2 3 9 9 4 5 6 9 9 9 9 9 9

⍝ aplcart/tt.tsv:1805 — Survivor number in the Josephus problem of order Js
Js←10 ⋄ (2⊥1⊖2⊥⍣¯1⊢)Js   ⍝ 5

⍝ aplcart/tt.tsv:1806 — Survivor number in the Josephus problem of order Js
Js←10 ⋄ (2⊥1⊖2⊥⍣¯1⊢)Js   ⍝ 5

⍝ aplcart/tt.tsv:1807 — Swap real and imaginary
N←1j2 ¯3j4 ⋄ (¯11○+)N   ⍝ 2j1 4j¯3

⍝ aplcart/tt.tsv:1808 — Swap real and imaginary
N←1j2 ¯3j4 ⋄ (¯11○+)N   ⍝ 2j1 4j¯3

⍝ aplcart/tt.tsv:1809 — Syllabisation of a Finnish word Dv
Dv←'Hello, world! 123'  ⋄ Dv⊂⍨1,1↓(≢Dv)↑</2↕(•C Dv)∊'aeiouyäö'
('Hel' ⋄ 'lo, ' ⋄ 'world! 123')

⍝ aplcart/tt.tsv:1810 — Syllabisation of a Finnish word Dv
Dv←'Hello, world! 123'  ⋄ Dv⊂⍨1,1↓(≢Dv)↑</2↕(•C Dv)∊'aeiouyäö'
('Hel' ⋄ 'lo, ' ⋄ 'world! 123')

⍝ aplcart/tt.tsv:1811 — Symmetric Set Difference; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←2 3 4 ⋄ Xv(∪~∩)Yv   ⍝ 1 4

⍝ aplcart/tt.tsv:1812 — Symmetric Set Difference; optional {X} instantiated as dyadic use
Xv←1 2 3 ⋄ Yv←2 3 4 ⋄ Xv(∪~∩)Yv   ⍝ 1 4

⍝ aplcart/tt.tsv:1813 — Tail: Last major cell
Y←3 1 3 2 ⋄ (1⌷⊖)Y   ⍝ 2

⍝ aplcart/tt.tsv:1814 — Tail: Last major cell
Y←3 1 3 2 ⋄ (1⌷⊖)Y   ⍝ 2

⍝ aplcart/tt.tsv:1815 — Take of at most Iv elements from Y
Iv←5 1 ⋄ Y←2 3⍴⍳6 ⋄ Iv(⊢↑⍨≢⍤⊣↑⌊∘⍴)Y   ⍝ 2 1⍴1 4

⍝ aplcart/tt.tsv:1816 — Take of at most Iv elements from Y
Iv←5 1 ⋄ Y←2 3⍴⍳6 ⋄ Iv(⊢↑⍨≢⍤⊣↑⌊∘⍴)Y   ⍝ 2 1⍴1 4

⍝ aplcart/tt.tsv:1817 — Tangent N
N←0.25 0.5 0.75 ⋄ 3∘○N
0.2553419212210363 0.5463024898437905 0.9315964599440724

⍝ aplcart/tt.tsv:1818 — Tangent N
N←0.25 0.5 0.75 ⋄ 3∘○N
0.2553419212210363 0.5463024898437905 0.9315964599440724

⍝ aplcart/tt.tsv:1819 — Test relations (¯2…2) of elements of N to range 1⌷Mv , 2⌷Mv
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(¯2+1⊥ <⌝ ⍪ ≤⌝ )N
¯2 ¯1 0 1 2

⍝ aplcart/tt.tsv:1820 — Test relations (¯2…2) of elements of N to range 1⌷Mv , 2⌷Mv
Mv←1 3 ⋄ N←0 1 2 3 4 ⋄ Mv(¯2+1⊥ <⌝ ⍪ ≤⌝ )N
¯2 ¯1 0 1 2

⍝ aplcart/tt.tsv:1821 — Test relations (¯2…2) of elements of N to ranges M (2=¯1↑⍴M)
M←2 2⍴1 3 2 4 ⋄ N←0 1 2 3 4 5 ⋄ M(+/∘× -⌝ ⍨)N
6 2⍴¯2 ¯2 ¯1 ¯2 0 ¯1 1 0 2 1 2 2

⍝ aplcart/tt.tsv:1822 — Test relations (¯2…2) of elements of N to ranges M (2=¯1↑⍴M)
M←2 2⍴1 3 2 4 ⋄ N←0 1 2 3 4 5 ⋄ M(+/∘× -⌝ ⍨)N
6 2⍴¯2 ¯2 ¯1 ¯2 0 ¯1 1 0 2 1 2 2

⍝ aplcart/tt.tsv:1823 — Test relations (¯2…2) of major cells Y to range 1⌷X , 2⌷X
X←1 3 ⋄ Y←0 1 2 3 4 ⋄ X(⌊¯3+0.6×⍳+3×⍸)Y   ⍝ ¯2 ¯1 0 1 2

⍝ aplcart/tt.tsv:1824 — Test relations (¯2…2) of major cells Y to range 1⌷X , 2⌷X
X←1 3 ⋄ Y←0 1 2 3 4 ⋄ X(⌊¯3+0.6×⍳+3×⍸)Y   ⍝ ¯2 ¯1 0 1 2

⍝ aplcart/tt.tsv:1825 — Tetration: ᴺˢIs
(3 (*/⍴) 2 ⋄ 2 (*/⍴) 4)   ⍝ 16 256

⍝ aplcart/tt.tsv:1826 — Tetration: ᴺˢIs
Is←3 ⋄ Ns←2 ⋄ Is(*/⍴)Ns   ⍝ 16

⍝ aplcart/tt.tsv:1827 — The last item of Y
Y←3 1 3 2 ⋄ (↑⌽∘,)Y   ⍝ 2

⍝ aplcart/tt.tsv:1828 — The last item of Y
Y←3 1 3 2 ⋄ (↑⌽∘,)Y   ⍝ 2

⍝ aplcart/tt.tsv:1829 — Theoretical standard deviation
Nv←1 2 4 ⋄ ((2*∘÷⍨+⌿÷≢)2*⍨⊢-+⌿÷≢)Nv   ⍝ 1.247219128924647

⍝ aplcart/tt.tsv:1830 — Theoretical standard deviation
Nv←1 2 4 ⋄ ((2*∘÷⍨+⌿÷≢)2*⍨⊢-+⌿÷≢)Nv   ⍝ 1.247219128924647

⍝ aplcart/tt.tsv:1831 — Theoretical variance
Nv←1 2 4 ⋄ (≢÷⍨≢÷⍨(≢×+.*∘2)-2*⍨+⌿)Nv   ⍝ 1.555555555555556

⍝ aplcart/tt.tsv:1832 — Theoretical variance
Nv←1 2 4 ⋄ (≢÷⍨≢÷⍨(≢×+.*∘2)-2*⍨+⌿)Nv   ⍝ 1.555555555555556

⍝ aplcart/tt.tsv:1833 — Tiling a matrix Ym over a matrix of shape Iv
Iv←4 6 ⋄ Ym←2 3⍴⍳6 ⋄ Iv(⊣⍴⊢⌿⍤⊣⍴⍤1⊢)Ym
4 6⍴1 2 3 1 2 3 4 5 6 4 5 6 1 2 3 1 2 3 4 5 6 4 5 6

⍝ aplcart/tt.tsv:1834 — Tiling a matrix Ym over a matrix of shape Iv
Iv←4 6 ⋄ Ym←2 3⍴⍳6 ⋄ Iv(⊣⍴⊢⌿⍤⊣⍴⍤1⊢)Ym
4 6⍴1 2 3 1 2 3 4 5 6 4 5 6 1 2 3 1 2 3 4 5 6 4 5 6

⍝ aplcart/tt.tsv:1835 — Totatives of Js; Reuse concrete inputs from aplcart/table.tsv:1134; execute this alternate recipe independently
Js←12 ⋄ (⍸1=⊢∨⍳)Js   ⍝ 1 5 7 11

⍝ aplcart/tt.tsv:1836 — Totatives of Js; Reuse concrete inputs from aplcart/table.tsv:1134; execute this alternate recipe independently
Js←12 ⋄ (⍸1=⊢∨⍳)Js   ⍝ 1 5 7 11

⍝ aplcart/tt.tsv:1837 — Transitive closure; Reuse concrete inputs from aplcart/table.tsv:1165; execute this alternate recipe independently
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (∨.∧⍨∨⊢)⍣≡Bm   ⍝ 3 3⍴1 1 1 1 1 1 1 1 1

⍝ aplcart/tt.tsv:1838 — Transitive closure; Reuse concrete inputs from aplcart/table.tsv:1165; execute this alternate recipe independently
Bm←3 3⍴0 1 0 0 0 1 1 0 0 ⋄ (∨.∧⍨∨⊢)⍣≡Bm   ⍝ 3 3⍴1 1 1 1 1 1 1 1 1

⍝ aplcart/tt.tsv:1839 — Translate characters to digits (bases 2 through 36); Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
(¯1+(•D,•A)⍳⊢)'09AZ'   ⍝ 0 9 10 35

⍝ aplcart/tt.tsv:1840 — Translate characters to digits (bases 2 through 36); Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
(¯1+(•D,•A)⍳⊢)'09AZ'   ⍝ 0 9 10 35

⍝ aplcart/tt.tsv:1841 — Translate digits to characters (bases 2 through 36); Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
((•D,•A)⌷⍨1+⊂)0 9 10 35   ⍝ '09AZ'

⍝ aplcart/tt.tsv:1842 — Translate digits to characters (bases 2 through 36); Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
((•D,•A)⌷⍨1+⊂)0 9 10 35   ⍝ '09AZ'

⍝ aplcart/tt.tsv:1843 — Transpose every submatrix of Y
Y←2 3 4⍴⍳24 ⋄ (⍉⍤2)Y
2 4 3⍴1 5 9 2 6 10 3 7 11 4 8 12 13 17 21 14 18 22 15 19 23 16 20 24

⍝ aplcart/tt.tsv:1844 — Transpose every submatrix of Y
Y←2 3 4⍴⍳24 ⋄ (⍉⍤2)Y
2 4 3⍴1 5 9 2 6 10 3 7 11 4 8 12 13 17 21 14 18 22 15 19 23 16 20 24

⍝ aplcart/tt.tsv:1845 — Transpose matrix Ym on condition Bs; Reuse concrete inputs from aplcart/table.tsv:759; execute this alternate recipe independently
Bs←1 ⋄ Ym←2 3⍴⍳6 ⋄ Bs(⊢⍉⍨1 2⌽⍨⊣)Ym   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/tt.tsv:1846 — Transpose matrix Ym on condition Bs; Reuse concrete inputs from aplcart/table.tsv:759; execute this alternate recipe independently
Bs←1 ⋄ Ym←2 3⍴⍳6 ⋄ Bs(⊢⍉⍨1 2⌽⍨⊣)Ym   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/tt.tsv:1847 — Triangle hypotenuse as function of side ratio
N←0.25 0.5 0.75 ⋄ 4∘○N
1.030776406404415 1.118033988749895 1.25

⍝ aplcart/tt.tsv:1848 — Triangle hypotenuse as function of side ratio
N←0.25 0.5 0.75 ⋄ 4∘○N
1.030776406404415 1.118033988749895 1.25

⍝ aplcart/tt.tsv:1849 — Triangle side (≥1) as function of hypotenuse
N←1 2 3 ⋄ ¯4∘○N   ⍝ 0 1.732050807568877 2.82842712474619

⍝ aplcart/tt.tsv:1850 — Triangle side (≥1) as function of hypotenuse
N←1 2 3 ⋄ ¯4∘○N   ⍝ 0 1.732050807568877 2.82842712474619

⍝ aplcart/tt.tsv:1851 — Triangle side as function of side (hypotenuse≤1)
N←0.25 0.5 0.75 ⋄ 0∘○N
0.9682458365518543 0.8660254037844386 0.6614378277661477

⍝ aplcart/tt.tsv:1852 — Triangle side as function of side (hypotenuse≤1)
N←0.25 0.5 0.75 ⋄ 0∘○N
0.9682458365518543 0.8660254037844386 0.6614378277661477

⍝ aplcart/tt.tsv:1853 — Triple: 3×N; dfns display import/wrappers omitted to test underlying arrays
(3∘× 1 2 3 4 5 ⋄ 3∘× (8 8 ⋄ 100 200 ⋄ 5.3 ¯6) ⋄ 3∘× 3 3⍴⍳9)
(3 6 9 12 15 ⋄ (24 24 ⋄ 300 600 ⋄ 15.9 ¯18) ⋄ 3 3⍴3 6 9 12 15 18 21 24 27)

⍝ aplcart/tt.tsv:1854 — Triple: 3×N
N←4 9 16 ⋄ 3∘×N   ⍝ 12 27 48

⍝ aplcart/tt.tsv:1855 — Truncated division
¯10 10 ¯10 10 ((××∘⌊|)÷) ¯3 ¯3 3 3   ⍝ 3 ¯3 ¯3 3

⍝ aplcart/tt.tsv:1856 — Truncated division
M←¯7 7 8 ⋄ N←3 3 3 ⋄ M((××∘⌊|)÷)N   ⍝ ¯2 2 2

⍝ aplcart/tt.tsv:1857 — Truth table: All possibilities of Boolean primitive Ds; Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:1544
Ds←'∧' ⋄ 0 1∘{(⍵,⍺)⍪⍺, (⍎⍵)⌝ ⍨⍺}Ds   ⍝ 3 3⍴'∧' 0 1 0 0 0 1 0 1

⍝ aplcart/tt.tsv:1858 — Truth table: All possibilities of Boolean primitive Ds; Reviewed Execute example checked through the Rust reference worker; Same concrete inputs and independent Dyalog expectation as aplcart/table.tsv:1544
Ds←'∧' ⋄ 0 1∘{(⍵,⍺)⍪⍺, (⍎⍵)⌝ ⍨⍺}Ds   ⍝ 3 3⍴'∧' 0 1 0 0 0 1 0 1

⍝ aplcart/tt.tsv:1859 — Turn off all 1s after first 0 (indicate all elements until the first 0)
∧\ 1 1 0 0 1 1 1   ⍝ 1 1 0 0 0 0 0

⍝ aplcart/tt.tsv:1860 — Turn off all 1s after first 0 (indicate all elements until the first 0)
B←1 1 0 1 0 1 0 ⋄ ∧\B   ⍝ 1 1 0 0 0 0 0

⍝ aplcart/tt.tsv:1861 — Turn off all 1s after first 1 (indicate only the first 1); First-true masks use cumulative counts under basedpl left scan
{⍵∧1=+\⍵} 0 1 0 1 0 1   ⍝ 0 1 0 0 0 0

⍝ aplcart/tt.tsv:1862 — Turn off all 1s after first 1 (indicate only the first 1); First-true masks use cumulative counts under basedpl left scan
B←1 1 0 1 0 1 0 ⋄ {⍵∧1=+\⍵}B   ⍝ 1 0 0 0 0 0 0

⍝ aplcart/tt.tsv:1863 — Turn off all 1s before first 0 (remove leading 1s)
(∧\<⊢) 1 1 0 0 1 1 1   ⍝ 0 0 0 0 1 1 1

⍝ aplcart/tt.tsv:1864 — Turn off all 1s before first 0 (remove leading 1s); Reuse concrete inputs from aplcart/table.tsv:754; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ (∧\<⊢)B   ⍝ 0 0 0 1 0 1 0

⍝ aplcart/tt.tsv:1865 — Turn on all 0s after first 0 (indicate all elements except the first 0); First-zero mask uses cumulative counts under basedpl left scan
{⍵∨1≠+\~⍵} 1 0 1 0 1 0   ⍝ 1 0 1 1 1 1

⍝ aplcart/tt.tsv:1866 — Turn on all 0s after first 0 (indicate all elements except the first 0); First-zero mask uses cumulative counts under basedpl left scan
B←1 1 0 1 0 1 0 ⋄ {⍵∨1≠+\~⍵}B   ⍝ 1 1 0 1 1 1 1

⍝ aplcart/tt.tsv:1867 — Turn on all 0s after first 1 (indicate all elements except leading 0s)
∨\ 0 0 1 0 0 1 1 1   ⍝ 0 0 1 1 1 1 1 1

⍝ aplcart/tt.tsv:1868 — Turn on all 0s after first 1 (indicate all elements except leading 0s)
B←1 1 0 1 0 1 0 ⋄ ∨\B   ⍝ 1 1 1 1 1 1 1

⍝ aplcart/tt.tsv:1869 — Turn on all 0s before first 1 (add leading 0s)
(∨\≤⊢) 0 0 1 0 0 1 1 1   ⍝ 1 1 1 0 0 1 1 1

⍝ aplcart/tt.tsv:1870 — Turn on all 0s before first 1 (add leading 0s); Reuse concrete inputs from aplcart/table.tsv:755; execute this alternate recipe independently
B←1 1 0 1 0 1 0 ⋄ (∨\≤⊢)B   ⍝ 1 1 0 1 0 1 0

⍝ aplcart/tt.tsv:1871 — Two-column matrix from two vectors (pad shorter vector)
Xv←1 2 ⋄ Yv←3 4 5 ⋄ Xv{⍉⊃⍺⍵}Yv   ⍝ 3 2⍴1 3 2 4 0 5

⍝ aplcart/tt.tsv:1872 — Two-column matrix from two vectors (pad shorter vector)
Xv←1 2 ⋄ Yv←3 4 5 ⋄ Xv{⍉⊃⍺⍵}Yv   ⍝ 3 2⍴1 3 2 4 0 5

⍝ aplcart/tt.tsv:1873 — Two-column matrix from two vectors (repeat scalars)
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,⍤0)Yv   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/tt.tsv:1874 — Two-column matrix from two vectors (repeat scalars)
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv(,⍤0)Yv   ⍝ 3 2⍴1 4 2 5 3 6

⍝ aplcart/tt.tsv:1875 — Two-row matrix from two vectors (repeat scalars)
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv,[0.5]Yv   ⍝ 2 3⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:1876 — Two-row matrix from two vectors (repeat scalars)
Xv←1 2 3 ⋄ Yv←4 5 6 ⋄ Xv,[0.5]Yv   ⍝ 2 3⍴1 2 3 4 5 6

⍝ aplcart/tt.tsv:1879 — Type: 'a' 1 ⎕NULL → ' ' 0 ⎕NULL (∊ with ⎕ML←0); optional {X} instantiated as dyadic use
Y← 'a' 1  ⋄ (↑0⍴⊂)Y   ⍝ ' ' 0

⍝ aplcart/tt.tsv:1880 — Type: 'a' 1 ⎕NULL → ' ' 0 ⎕NULL (∊ with ⎕ML←0); optional {X} instantiated as dyadic use
Y← 'a' 1  ⋄ (↑0⍴⊂)Y   ⍝ ' ' 0

⍝ aplcart/tt.tsv:1881 — Underlines a string (1=⎕IO)
Dv← 'xxabyyab'  ⋄ (,[0.5]∘'¯')Dv   ⍝ 2 8⍴'xxabyyab¯¯¯¯¯¯¯¯'

⍝ aplcart/tt.tsv:1882 — Underlines a string (1=⎕IO)
Dv← 'xxabyyab'  ⋄ (,[0.5]∘'¯')Dv   ⍝ 2 8⍴'xxabyyab¯¯¯¯¯¯¯¯'

⍝ aplcart/tt.tsv:1883 — Underlines non-blanks in a string
Dv← 'ab cd'  ⋄ (⊢,[0.5]'¯'\⍨≠∘' ')Dv   ⍝ 2 5⍴'ab cd¯¯ ¯¯'

⍝ aplcart/tt.tsv:1884 — Underlines non-blanks in a string
Dv← 'ab cd'  ⋄ (⊢,[0.5]'¯'\⍨≠∘' ')Dv   ⍝ 2 5⍴'ab cd¯¯ ¯¯'

⍝ aplcart/tt.tsv:1887 — Upper triangular matrix with diagonal: Js by Js
 ≤⌝ ⍨∘⍳ 5
5 5⍴1 1 1 1 1 0 1 1 1 1 0 0 1 1 1 0 0 0 1 1 0 0 0 0 1

⍝ aplcart/tt.tsv:1888 — Upper triangular matrix with diagonal: Js by Js
Js←4 ⋄  ≤⌝ ⍨∘⍳Js   ⍝ 4 4⍴1 1 1 1 0 1 1 1 0 0 1 1 0 0 0 1

⍝ aplcart/tt.tsv:1889 — Upper triangular matrix without diagonal: Js by Js
 <⌝ ⍨∘⍳ 5
5 5⍴0 1 1 1 1 0 0 1 1 1 0 0 0 1 1 0 0 0 0 1 0 0 0 0 0

⍝ aplcart/tt.tsv:1890 — Upper triangular matrix without diagonal: Js by Js
Js←4 ⋄  <⌝ ⍨∘⍳Js   ⍝ 4 4⍴0 1 1 1 0 0 1 1 0 0 0 1 0 0 0 0

⍝ aplcart/tt.tsv:1891 — Using Boolean array A for expanding Yv (Yv's elements at 1s in A)
A←1 0 1 0 1 ⋄ Yv←2 3 4 ⋄ A⊣@⊢⍨Yv   ⍝ 2 0 3 0 4

⍝ aplcart/tt.tsv:1892 — Using Boolean array A for expanding Yv (Yv's elements at 1s in A)
A←1 0 1 0 1 ⋄ Yv←2 3 4 ⋄ A⊣@⊢⍨Yv   ⍝ 2 0 3 0 4

⍝ aplcart/tt.tsv:1893 — Using sample row Yv to form an initially empty matrix for later expansion; optional {X} instantiated as dyadic use
Yv←4 5 6 ⋄ (⍉0/⍪)Yv   ⍝ 0 3⍴0

⍝ aplcart/tt.tsv:1894 — Using sample row Yv to form an initially empty matrix for later expansion; optional {X} instantiated as dyadic use
Yv←4 5 6 ⋄ (⍉0/⍪)Yv   ⍝ 0 3⍴0

⍝ aplcart/tt.tsv:1895 — Valid credit card?
Jv←4 5 3 9 1 4 8 8 0 3 4 3 6 4 6 7 ⋄ (0=10|1⊥∘,0 10⊤⊢×∘⌽1 2⍴⍨≢)Jv
1

⍝ aplcart/tt.tsv:1896 — Valid credit card?
Jv←4 5 3 9 1 4 8 8 0 3 4 3 6 4 6 7 ⋄ (0=10|1⊥∘,0 10⊤⊢×∘⌽1 2⍴⍨≢)Jv
1

⍝ aplcart/tt.tsv:1897 — Value of saddle point; First-true masks use cumulative counts under basedpl left scan
Nm←2 2⍴3 4 1 2 ⋄ (,⊢⍤/⍨(⊢=⍴⍴⌈⌿){⍵∧1=+\⍵}⍤,⍤∧⊢=∘⍉⌽∘⍴⍴⌊/)Nm
1⍴3

⍝ aplcart/tt.tsv:1898 — Value of saddle point; First-true masks use cumulative counts under basedpl left scan
Nm←2 2⍴3 4 1 2 ⋄ (,⊢⍤/⍨(⊢=⍴⍴⌈⌿){⍵∧1=+\⍵}⍤,⍤∧⊢=∘⍉⌽∘⍴⍴⌊/)Nm
1⍴3

⍝ aplcart/tt.tsv:1899 — Vector (Jv[1]⍴1),(Jv[2]⍴0),(Jv[3]⍴1),…; Reuse concrete inputs from aplcart/table.tsv:1231; execute this alternate recipe independently
Jv←1 2 3 ⋄ (≠\(⍳+/)∊(+\1∘,))Jv   ⍝ 1 0 0 1 1 1

⍝ aplcart/tt.tsv:1900 — Vector (Jv[1]⍴1),(Jv[2]⍴0),(Jv[3]⍴1),…; Reuse concrete inputs from aplcart/table.tsv:1231; execute this alternate recipe independently
Jv←1 2 3 ⋄ (≠\(⍳+/)∊(+\1∘,))Jv   ⍝ 1 0 0 1 1 1

⍝ aplcart/tt.tsv:1901 — Vector (cross) product of vectors
Mv←1 2 3 ⋄ Nv←4 5 6 ⋄ Mv((1∘⌽⍤⊣×¯1⌽⊢)-¯1∘⌽⍤⊣×1⌽⊢)Nv
¯3 6 ¯3

⍝ aplcart/tt.tsv:1902 — Vector (cross) product of vectors
Mv←1 2 3 ⋄ Nv←4 5 6 ⋄ Mv((1∘⌽⍤⊣×¯1⌽⊢)-¯1∘⌽⍤⊣×1⌽⊢)Nv
¯3 6 ¯3

⍝ aplcart/tt.tsv:1903 — Vector having as many ones as Ym has rows
Ym←2 3⍴⍳6 ⋄ (1⍴⍨≢)Ym   ⍝ 1 1

⍝ aplcart/tt.tsv:1904 — Vector having as many ones as Ym has rows
Ym←2 3⍴⍳6 ⋄ (1⍴⍨≢)Ym   ⍝ 1 1

⍝ aplcart/tt.tsv:1905 — Vector of major cells for any rank Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (,⊂⍤¯1)Y   ⍝ 3 1 3 2

⍝ aplcart/tt.tsv:1906 — Vector of major cells for any rank Y; optional {X} instantiated as dyadic use
Y←3 1 3 2 ⋄ (,⊂⍤¯1)Y   ⍝ 3 1 3 2

⍝ aplcart/tt.tsv:1907 — Vertical column headings for character matrix of width Js; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
(•D⌷⍨∘⊂1+10⊥⍣¯1⍳)12   ⍝ 2 12⍴'000000000111123456789012'

⍝ aplcart/tt.tsv:1908 — Vertical column headings for character matrix of width Js; Concrete APLcart recipe using existing read-only text constants; independently captured in Dyalog 20.0.53963.0, IO=1 CT=1E¯14 DIV=0 ML=1
(•D⌷⍨∘⊂1+10⊥⍣¯1⍳)12   ⍝ 2 12⍴'000000000111123456789012'

⍝ aplcart/tt.tsv:1909 — Vertically lengthening matrix Xm to be compatible (for ,) with Ym
Xm←2 2⍴⍳4 ⋄ Ym←3 3⍴⍳9 ⋄ Xm↑⍨∘≢Ym   ⍝ 3 2⍴1 2 3 4 0 0

⍝ aplcart/tt.tsv:1910 — Vertically lengthening matrix Xm to be compatible (for ,) with Ym
Xm←2 2⍴⍳4 ⋄ Ym←3 3⍴⍳9 ⋄ Xm↑⍨∘≢Ym   ⍝ 3 2⍴1 2 3 4 0 0

⍝ aplcart/tt.tsv:1911 — Vertically stack digits of ⍳Is (helps locating column positions); optional {X} instantiated as dyadic use
Js←4 ⋄ (1 0⍕10 10⊤⍳)Js   ⍝ 2 4⍴'00001234'

⍝ aplcart/tt.tsv:1912 — Vertically stack digits of ⍳Is (helps locating column positions); optional {X} instantiated as dyadic use
Js←4 ⋄ (1 0⍕10 10⊤⍳)Js   ⍝ 2 4⍴'00001234'

⍝ aplcart/tt.tsv:1913 — Volume of box with sides Nv
×/ 10 10 2   ⍝ 200

⍝ aplcart/tt.tsv:1914 — Volume of box with sides Nv
Nv←1 2 3 ⋄ ×/Nv   ⍝ 6

⍝ aplcart/tt.tsv:1915 — Volume of cone with height M and radius N
M←2 3 4 ⋄ N←4 9 16 ⋄ M(π××3÷⍨⊢)N
33.51032163829112 254.4690049407732 1072.330292425316

⍝ aplcart/tt.tsv:1916 — Volume of cone with height M and radius N
M←2 3 4 ⋄ N←4 9 16 ⋄ M(π××3÷⍨⊢)N
33.51032163829112 254.4690049407732 1072.330292425316

⍝ aplcart/tt.tsv:1917 — Volume of pyramid with height, width, length Nv
Nv←1 2 3 ⋄ (3÷⍨×/)Nv   ⍝ 2

⍝ aplcart/tt.tsv:1918 — Volume of pyramid with height, width, length Nv
Nv←1 2 3 ⋄ (3÷⍨×/)Nv   ⍝ 2

⍝ aplcart/tt.tsv:1919 — Volume of sphere with radius N
N←4 9 16 ⋄ (π4÷3÷*∘3)N
268.082573106329 3053.628059289279 17157.28467880506

⍝ aplcart/tt.tsv:1920 — Volume of sphere with radius N
N←4 9 16 ⋄ (π4÷3÷*∘3)N
268.082573106329 3053.628059289279 17157.28467880506

⍝ aplcart/tt.tsv:1921 — Weighted average of columns of Nm with weights Mv
Mv←1 2 ⋄ Nm←2 3⍴1 2 3 4 5 6 ⋄ Mv(+.×÷1⊥⊣)Nm
3 4 5

⍝ aplcart/tt.tsv:1922 — Weighted average of columns of Nm with weights Mv
Mv←1 2 ⋄ Nm←2 3⍴1 2 3 4 5 6 ⋄ Mv(+.×÷1⊥⊣)Nm
3 4 5

⍝ aplcart/tt.tsv:1923 — Weighted average of rows of Nm with weights Mv
Mv←1 2 3 ⋄ Nm←2 3⍴1 2 3 4 5 6 ⋄ Mv(+.×⍨÷1⊥⊣)Nm
2.333333333333333 5.333333333333333

⍝ aplcart/tt.tsv:1924 — Weighted average of rows of Nm with weights Mv
Mv←1 2 3 ⋄ Nm←2 3⍴1 2 3 4 5 6 ⋄ Mv(+.×⍨÷1⊥⊣)Nm
2.333333333333333 5.333333333333333

⍝ aplcart/tt.tsv:1925 — Which elements differ from next ones (non-empty Yv)
Yv←1 1 2 2 1 ⋄ (1,⍨(≠/∘(2∘↕)))Yv   ⍝ 0 1 0 1 1

⍝ aplcart/tt.tsv:1926 — Which elements differ from next ones (non-empty Yv)
Yv←1 1 2 2 1 ⋄ (1,⍨(≠/∘(2∘↕)))Yv   ⍝ 0 1 0 1 1

⍝ aplcart/tt.tsv:1927 — Which elements of X belong to corresponding row of Y (≢X ↔ ≢Y)
X←2 3⍴1 2 3 4 5 6 ⋄ Y←2 2⍴2 7 5 4 ⋄ X(∊⍤1)Y
2 3⍴0 1 0 1 1 0

⍝ aplcart/tt.tsv:1928 — Which elements of X belong to corresponding row of Y (≢X ↔ ≢Y)
X←2 3⍴1 2 3 4 5 6 ⋄ Y←2 2⍴2 7 5 4 ⋄ X(∊⍤1)Y
2 3⍴0 1 0 1 1 0

⍝ aplcart/tt.tsv:1931 — Widening matrix Ym to be compatible with Xm; Reuse concrete inputs from aplcart/table.tsv:1317; execute this alternate recipe independently
Xm←2 4⍴0 ⋄ Ym←2 2⍴⍳4 ⋄ Xm↑⍤1⍨∘↑∘⌽∘⍴⍨Ym   ⍝ 2 4⍴1 2 0 0 3 4 0 0

⍝ aplcart/tt.tsv:1932 — Widening matrix Ym to be compatible with Xm; Reuse concrete inputs from aplcart/table.tsv:1317; execute this alternate recipe independently
Xm←2 4⍴0 ⋄ Ym←2 2⍴⍳4 ⋄ Xm↑⍤1⍨∘↑∘⌽∘⍴⍨Ym   ⍝ 2 4⍴1 2 0 0 3 4 0 0

⍝ aplcart/tt.tsv:1933 — Word lengths of words in list D
D←3 3⍴'abc   de '  ⋄ (+.≠∘' ')D   ⍝ 3 0 2

⍝ aplcart/tt.tsv:1934 — Word lengths of words in list D
D←3 3⍴'abc   de '  ⋄ (+.≠∘' ')D   ⍝ 3 0 2

⍝ aplcart/tt.tsv:1937 — Y's Head of Length Is and its Tail
Is←2 ⋄ Y←3 1 3 2 ⋄ Is(↑,⍥⊂↓)Y   ⍝ (3 1 ⋄ 3 2)

⍝ aplcart/tt.tsv:1938 — Y's Head of Length Is and its Tail
Is←2 ⋄ Y←3 1 3 2 ⋄ Is(↑,⍥⊂↓)Y   ⍝ (3 1 ⋄ 3 2)

⍝ aplcart/tt.tsv:1939 — Zero array of shape, size, and structure of N; dfns display import/wrappers omitted to test underlying arrays
(0∘× 1 2 3 4 5 ⋄ 0∘× (8 8 ⋄ 100 200 ⋄ 5.3 ¯6) ⋄ 0∘× 3 3⍴⍳9)
(0 0 0 0 0 ⋄ (0 0 ⋄ 0 0 ⋄ 0 0) ⋄ 3 3⍴0 0 0 0 0 0 0 0 0)

⍝ aplcart/tt.tsv:1940 — Zero array of shape, size, and structure of N
N←(1 2⋄ 3 4 5) ⋄ 0∘×N   ⍝ (0 0 ⋄ 0 0 0)

⍝ aplcart/tt.tsv:1941 — Zero?
(0∘= 1 2 3 4 ⋄ 0∘= ¯2 ¯1 0 1 2)   ⍝ (0 0 0 0 ⋄ 0 0 1 0 0)

⍝ aplcart/tt.tsv:1942 — Zero?
N←¯2 0 3 ⋄ 0∘=N   ⍝ 0 1 0

⍝ aplcart/tt.tsv:1943 — Zeroing elements of N that are found in M
M←2 4 ⋄ N←1 2 3 4 ⋄ M(⊢×∘~∊⍨)N   ⍝ 1 0 3 0

⍝ aplcart/tt.tsv:1944 — Zeroing elements of N that are found in M
M←2 4 ⋄ N←1 2 3 4 ⋄ M(⊢×∘~∊⍨)N   ⍝ 1 0 3 0

⍝ aplcart/tt.tsv:1945 — Zeros, same shape and structure
(3 3⍴⍳9 ⋄ '' ⋄ ≠⍨ 3 3⍴⍳9)
(3 3⍴1 2 3 4 5 6 7 8 9 ⋄ '' ⋄ 3 3⍴0 0 0 0 0 0 0 0 0)

⍝ aplcart/tt.tsv:1946 — Zeros, same shape and structure
Y←(1 2⋄ 3 4 5) ⋄ ≠⍨Y   ⍝ (0 0 ⋄ 0 0 0)

⍝ aplcart/tt.tsv:1947 — Zeros, same shape plus one; optional {X} instantiated as dyadic use
Yv←4 5 6 ⋄ ≠/0↕Yv   ⍝ 0 0 0 0

⍝ aplcart/tt.tsv:1948 — Zeros, same shape plus one; optional {X} instantiated as dyadic use
Yv←4 5 6 ⋄ ≠/0↕Yv   ⍝ 0 0 0 0

⍝ aplcart/tt.tsv:1949 — Zeros, simple with same shape
Y←3 1 3 2 ⋄ (∊∘⍬)Y   ⍝ 0 0 0 0

⍝ aplcart/tt.tsv:1950 — Zeros, simple with same shape
Y←3 1 3 2 ⋄ (∊∘⍬)Y   ⍝ 0 0 0 0

⍝ aplcart/tt.tsv:1951 — cos ↔ sin: (1-N*2)*.5 (more precise cos arcsin N or sin arccos N)
N←0.25 0.5 0.75 ⋄ 0∘○N
0.9682458365518543 0.8660254037844386 0.6614378277661477

⍝ aplcart/tt.tsv:1952 — cos ↔ sin: (1-N*2)*.5 (more precise cos arcsin N or sin arccos N)
N←0.25 0.5 0.75 ⋄ 0∘○N
0.9682458365518543 0.8660254037844386 0.6614378277661477

⍝ aplcart/tt.tsv:1953 — cosh → sinh: (N+1)×((N-1)÷N+1)*.5 (more precise sinh arcosh N or tan arcsec N)
N←1 2 3 ⋄ ¯4∘○N   ⍝ 0 1.732050807568877 2.82842712474619

⍝ aplcart/tt.tsv:1954 — cosh → sinh: (N+1)×((N-1)÷N+1)*.5 (more precise sinh arcosh N or tan arcsec N)
N←1 2 3 ⋄ ¯4∘○N   ⍝ 0 1.732050807568877 2.82842712474619

⍝ aplcart/tt.tsv:1955 — icos ↔ -isin: -(-1+N*2)*.5
N←0.25 0.5 0.75 ⋄ ¯8∘○N
0j¯1.030776406404415 0j¯1.118033988749895 0j¯1.25

⍝ aplcart/tt.tsv:1956 — icos ↔ -isin: -(-1+N*2)*.5
N←0.25 0.5 0.75 ⋄ ¯8∘○N
0j¯1.030776406404415 0j¯1.118033988749895 0j¯1.25

⍝ aplcart/tt.tsv:1957 — icos ↔ isin: (-1+N*2)*.5
N←0.25 0.5 0.75 ⋄ 8∘○N
0j1.030776406404415 0j1.118033988749895 0j1.25

⍝ aplcart/tt.tsv:1958 — icos ↔ isin: (-1+N*2)*.5
N←0.25 0.5 0.75 ⋄ 8∘○N
0j1.030776406404415 0j1.118033988749895 0j1.25

⍝ aplcart/tt.tsv:1959 — last index of ((⍳): Last indices in X of major cells Y
mat ← 3 2⍴⍳6 ⋄ ('ABCDABCDEF' (1-⍳⍨∘⊖⍨-∘≢⊣) 'ACF' ⋄ mat (1-⍳⍨∘⊖⍨-∘≢⊣) 5 6)
(5 7 10) 3

⍝ aplcart/tt.tsv:1960 — last index of ((⍳): Last indices in X of major cells Y; Reuse concrete inputs from aplcart/table.tsv:1318; execute this alternate recipe independently
X←1 2 3 2 ⋄ Y←2 4 ⋄ X(1-⍳⍨∘⊖⍨-∘≢⊣)Y   ⍝ 4 0

⍝ aplcart/tt.tsv:1961 — sinh → cosh: (1+N*2)*.5 (more precise cosh arsinh N or sec arctan N)
N←0.25 0.5 0.75 ⋄ 4∘○N
1.030776406404415 1.118033988749895 1.25

⍝ aplcart/tt.tsv:1962 — sinh → cosh: (1+N*2)*.5 (more precise cosh arsinh N or sec arctan N)
N←0.25 0.5 0.75 ⋄ 4∘○N
1.030776406404415 1.118033988749895 1.25

⍝ aplcart/tt.tsv:1963 — ±N by juxtaposition
N←4 9 16 ⋄ (1 ¯1×⊂)N   ⍝ (4 9 16 ⋄ ¯4 ¯9 ¯16)

⍝ aplcart/tt.tsv:1964 — ±N by juxtaposition
N←4 9 16 ⋄ (1 ¯1×⊂)N   ⍝ (4 9 16 ⋄ ¯4 ¯9 ¯16)

⍝ aplcart/tt.tsv:1965 — ±N increasing rank
N←4 9 16 ⋄ (1 ¯1 ×⌝ ⊢)N   ⍝ 2 3⍴4 9 16 ¯4 ¯9 ¯16

⍝ aplcart/tt.tsv:1966 — ±N increasing rank
N←4 9 16 ⋄ (1 ¯1 ×⌝ ⊢)N   ⍝ 2 3⍴4 9 16 ¯4 ¯9 ¯16

⍝ aplcart/tt.tsv:1967 — ∓N by juxtaposition
N←4 9 16 ⋄ (¯1 1×⊂)N   ⍝ (¯4 ¯9 ¯16 ⋄ 4 9 16)

⍝ aplcart/tt.tsv:1968 — ∓N by juxtaposition
N←4 9 16 ⋄ (¯1 1×⊂)N   ⍝ (¯4 ¯9 ¯16 ⋄ 4 9 16)

⍝ aplcart/tt.tsv:1969 — ∓N increasing rank
N←4 9 16 ⋄ (¯1 1 ×⌝ ⊢)N   ⍝ 2 3⍴¯4 ¯9 ¯16 4 9 16

⍝ aplcart/tt.tsv:1970 — ∓N increasing rank
N←4 9 16 ⋄ (¯1 1 ×⌝ ⊢)N   ⍝ 2 3⍴¯4 ¯9 ¯16 4 9 16

⍝ aplcart/table.tsv:1394 — Euler's totient function (fastest above about 1000); dfns.pco uses ℙ/⨸
((×/⊢-≠)3∘ℙ)60   ⍝ 16

⍝ aplcart/table.tsv:1559 — Sum of positive divisors of Js (fast +/∘∪⊢∨⍳); dfns.pco uses ℙ/⨸
(×/({(¯1+⍺*⍵+1)÷⍺-1}⌿2∘ℙ))60   ⍝ 168

⍝ aplcart/table.tsv:1682 — prime factors and exponents; dfns.pco uses ℙ/⨸
2ℙ360   ⍝ 2 3⍴2 3 5 3 2 1

⍝ aplcart/table.tsv:1683 — prime factorization of ⍵; dfns.pco uses ℙ/⨸
⨸360   ⍝ 2 2 2 3 3 5

⍝ aplcart/table.tsv:1684 — Is ⍵ a non-prime?; dfns.pco uses ℙ/⨸
0ℙ1 2 9 17   ⍝ 1 0 1 0

⍝ aplcart/table.tsv:1685 — Is ⍵ a prime?; dfns.pco uses ℙ/⨸
1ℙ1 2 9 17   ⍝ 0 1 0 1

⍝ aplcart/table.tsv:1686 — m+b/⍳⍴b are all the primes between m and n, where ⍵≡m,n; prime mask over [10,20)
1ℙ9+⍳10   ⍝ 0 1 0 1 0 0 0 1 0 1

⍝ aplcart/table.tsv:1687 — number of primes less than ⍵; dfns.pco uses ℙ/⨸
¯1ℙ2 3 10 20   ⍝ 0 1 4 8

⍝ aplcart/table.tsv:1688 — next prime larger than ⍵; dfns.pco uses ℙ/⨸
4ℙ2 3 10 20   ⍝ 3 5 11 23

⍝ aplcart/table.tsv:1689 — ⍵-th prime; dfns.pco uses ℙ/⨸
ℙ⍳8   ⍝ 2 3 5 7 11 13 17 19

⍝ aplcart/table.tsv:1690 — next prime smaller than ⍵; dfns.pco uses ℙ/⨸
¯4ℙ3 5 10 20   ⍝ 2 3 7 19

⍝ aplcart/table.tsv:1779 — Random numbers with normal distribution
•LOAD 'lib/numeric.apl'
⍴100+15×NormRand 2 3
⍝ =>
2 3

⍝ aplcart/table.tsv:1810 — Phinary representation of numbers ⍵
•LOAD 'lib/numeric.apl'
phinary 2 3 4 5
⍝ =>
('10.01' ⋄ '100.01' ⋄ '101.01' ⋄ '1000.1001')

⍝ aplcart/tt.tsv:533 — Euler's totient function (fastest above about 1000); dfns.pco uses ℙ/⨸
((×/⊢-≠)3∘ℙ)60   ⍝ 16

⍝ aplcart/tt.tsv:534 — Euler's totient function (fastest above about 1000); dfns.pco uses ℙ/⨸
((×/⊢-≠)3∘ℙ)60   ⍝ 16

⍝ aplcart/tt.tsv:1795 — Sum of positive divisors of Js (fast +/∘∪⊢∨⍳); dfns.pco uses ℙ/⨸
(×/({(¯1+⍺*⍵+1)÷⍺-1}⌿2∘ℙ))60   ⍝ 168

⍝ aplcart/tt.tsv:1796 — Sum of positive divisors of Js (fast +/∘∪⊢∨⍳); dfns.pco uses ℙ/⨸
(×/({(¯1+⍺*⍵+1)÷⍺-1}⌿2∘ℙ))60   ⍝ 168

⍝ aplcart/table.tsv:1913 — Inverted Table Nubsieve (≠Y where Y is unverted Yv); convert columns to rows for ordinary search and grouping
Xv←(1 2 1 3)'abab' ⋄ ≠↓⍉⊃Xv   ⍝ 1 1 0 1

⍝ aplcart/table.tsv:1948 — Inverted Table Member of (X∊⍥↓Y where X and Y are unverted Xv and Yv); convert columns to rows for ordinary search and grouping
Xv←(1 2 1 3)'abab' ⋄ Yv←(1 3 4)'abc' ⋄ (↓⍉⊃Xv)∊↓⍉⊃Yv
1 0 1 1

⍝ aplcart/table.tsv:2015 — Inverted Table Unique (∪Y where Y is unverted Yv); convert columns to rows for ordinary search and grouping
Xv←(1 2 1 3)'abab' ⋄ {⍵⌿¨⍨⊂≠↓⍉⊃⍵}Xv   ⍝ (1 2 3) ('abb')

⍝ aplcart/table.tsv:2033 — Inverted Table Without (↑X~⍥↓Y where X and Y are unverted Xv and Yv); convert columns to rows for ordinary search and grouping
Xv←(1 2 1 3)'abab' ⋄ Yv←(1 3 4)'abc' ⋄ Xv{⍺⌿¨⍨⊂~(↓⍉⊃⍺)∊↓⍉⊃⍵}Yv
(1⍴2 ⋄ 1⍴'b')

⍝ aplcart/table.tsv:2267 — Inverted Table Dyadic Key (Xf⌸Y where X and Y are unverted Xv and Yv); convert columns to rows for ordinary search and grouping
Xv←(1 2 1 3)'abab' ⋄ Zv←(10 20 30 40⋄ 1 2 3 4) ⋄ {(↓⍉⊃Xv){+/⍵}⌸⍵}¨Zv
(40 20 40 ⋄ 4 2 4)

⍝ aplcart/table.tsv:2713 — Inverted Table Index-of (X⍳Y where X and Y are unverted Xv and Yv); convert columns to rows for ordinary search and grouping
Xv←(1 2 1 3)'abab' ⋄ Yv←(1 3 4)'abc' ⋄ (↓⍉⊃Xv)⍳↓⍉⊃Yv
1 4 5

⍝ aplcart/table.tsv:2150 — Pick'th fn applied to arg; Native agenda
(2◶(-˘÷˘×))4   ⍝ 0.25

⍝ aplcart/table.tsv:2165 — Sequential test; Short-circuit with a guard
{⍵≤0:0 ⋄ 2>⍟⍵}¨¯1 0 5 12   ⍝ 0 0 1 0

⍝ aplcart/table.tsv:2166 — Sequential test; Short-circuit with a guard
{⍵≤0:1 ⋄ 2>⍟⍵}¨¯1 0 5 12   ⍝ 1 1 1 0

⍝ aplcart/table.tsv:2174 — Proposition:consequence:alternative; Conditional with a guard
{⍵<0:-⍵ ⋄ ⍵}¨¯3 0 4   ⍝ 3 0 4

⍝ aplcart/table.tsv:2177 — Simulation of “fork” syntax; Native fork
(+/÷≢)1 2 3 4   ⍝ 2.5

⍝ aplcart/table.tsv:2178 — Slower but elegant simulation of “fork” syntax; Native fork
(+/÷≢)1 2 3 4   ⍝ 2.5

⍝ aplcart/table.tsv:2196 — Fast each for pure operand function; Native Each
+/¨(1 2⋄ 3 4 5)   ⍝ 3 12

⍝ aplcart/table.tsv:2652 — Dfn/dop Error Guard (result upon listed error); Concrete error guard
{11::42 ⋄ ÷⍵}0   ⍝ 42

⍝ aplcart/table.tsv:3732 — Load other source files prior to this one; Load shared library
•LOAD 'lib/numeric.apl' ⋄ phinary 2   ⍝ '10.01'

⍝ aplcart/table.tsv:1786 — Continued fraction approximation of real ⍵; Concrete recipe with independent Dyalog expectation; Use fixed comparison tolerance and shared library
•LOAD 'lib/numeric.apl'
cfract 1.5
⍝ =>
1 2

⍝ aplcart/table.tsv:1787 — Rational approximation to real ⍵; Concrete recipe with independent Dyalog expectation; Use fixed comparison tolerance and shared library
•LOAD 'lib/numeric.apl'
rational 1.5
⍝ =>
3 2

⍝ aplcart/table.tsv:2175 — Array of functions; Concrete native equivalent; independent Dyalog dfns expectation; Store and retrieve native function values
v←+˘× ⋄ (2(1⊃v)3),(2(2⊃v)3)   ⍝ 5 6

⍝ aplcart/table.tsv:2208 — Apply arrayified function Xs on Y (optionally with left argument Y); Concrete native equivalent; independent Dyalog dfns expectation; Apply a native stored function
f←1⊃(+˘×) ⋄ 2 f 3   ⍝ 5

⍝ aplcart/table.tsv:2217 — Condition f else g …; Concrete native equivalent; independent Dyalog dfns expectation; Choose the branch with Agenda
{((2-⍵)◶(+˘-))5}¨0 1   ⍝ ¯5 5

⍝ aplcart/table.tsv:2223 — Prefix-friendly @; Concrete native equivalent; independent Dyalog dfns expectation; Use native At with vector indices
10(+@2 4)1 2 3 4   ⍝ 1 12 3 14

⍝ aplcart/table.tsv:2229 — List of functions; Concrete native equivalent; independent Dyalog dfns expectation; Apply a native function vector to one argument
(+˘-){(↑⍺)⍵}¨3   ⍝ 3 ¯3

⍝ aplcart/table.tsv:2239 — Vector of functions; Concrete native equivalent; independent Dyalog dfns expectation; Pair each native function with its argument
(+˘-){(↑⍺)⍵}¨3 4   ⍝ 3 ¯4

⍝ aplcart/table.tsv:2173 — Apply no-result function “en passant”; Concrete no-result side effect and pass-through result; independently checked in Dyalog; Retain Execute-based sequencing; name the operand in the operator body
seen←0 ⋄ touch←{seen+←⍵ ⋄ 0:} ⋄ r←(touch {f←⍶ ⋄ ⍎'f ⍵ ⋄ ⍵'})3 ⋄ r,seen
3 3

⍝ aplcart/table.tsv:1591 — A hard, simple problem; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl; One-move sliding-tile puzzle; decimal move counts preserve the source text
•LOAD 'lib/dyalog.apl'
1 quzzle 2 2⍴'A   '
⍝ =>
3 1⍴('Top-right' ⋄ 'in 1 moves' ⋄ 2 2⍴'  A→')

⍝ aplcart/table.tsv:1598 — ⍺-separated segments of vector ⍵; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
',' segs 'a,b,,c'
⍝ =>
(1⍴'a' ⋄ 1⍴'b' ⋄ 1⍴'c')

⍝ aplcart/table.tsv:1599 — McCarthy's M91 function; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
m91 95
⍝ =>
91

⍝ aplcart/table.tsv:1600 — ⍺-combination matrix of ⍳⍵; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
2 cmat 4
⍝ =>
6 2⍴1 2 1 3 1 4 2 3 2 4 3 4

⍝ aplcart/table.tsv:1604 — John Conway's “Game of Life”; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
life 5 5⍴0 0 0 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0
⍝ =>
5 5⍴0 0 0 0 0 0 0 0 0 0 0 1 1 1 0 0 0 0 0 0 0 0 0 0 0

⍝ aplcart/table.tsv:1624 — Conversion to/from Morse code; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
morse 'SOS'
⍝ =>
('...' ⋄ '---' ⋄ '...')

⍝ aplcart/table.tsv:1625 — Base64 encoding and decoding as used in MIME; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
base64 72 101 108 108 111
⍝ =>
'SGVsbG8='

⍝ aplcart/table.tsv:1639 — APL source with comments removed; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
rmcm 'a←1 ⍝ note'
⍝ =>
'a←1       '

⍝ aplcart/table.tsv:1646 — Perfect Ripple Shuffle; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
ripple ⍳8
⍝ =>
5 1 6 2 7 3 8 4

⍝ aplcart/table.tsv:1654 — Probability of same birthday; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
365 birthday 23
⍝ =>
0.5072972343239857

⍝ aplcart/table.tsv:1662 — Lindenmayer L-system expansion; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
('A' 'AB'⋄ 'B' 'A') lsys 'AB'
⍝ =>
'ABA'

⍝ aplcart/table.tsv:1667 — Round-robin tournament; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
rr 4
⍝ =>
3 2 2⍴1 2 4 3 1 3 2 4 1 4 3 2

⍝ aplcart/table.tsv:1675 — Easter Sunday in year ⍵; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
easter 2025
⍝ =>
20250420

⍝ aplcart/table.tsv:1681 — Interpret a throw of dice; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
dice 6 6
⍝ =>
'Box Cars'

⍝ aplcart/table.tsv:1699 — Simple macro processor for bf; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl; Concrete macro definition and expansion
•LOAD 'lib/dyalog.apl'
mac 'A=++ AA'
⍝ =>
'++++'

⍝ aplcart/table.tsv:1700 — Manchester Small Scale Experimental Machine; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl; Machine load/store/halt program; final memory word captured in a Dyalog dfn to avoid collecting setup results
•LOAD 'lib/dyalog.apl'
m←⌽⍉(32⍴2)⊤32↑0 16389 24582 57344 0 13 0 ⋄ 2⊥⌽(baby m)[7;]
⍝ =>
4294967283

⍝ aplcart/table.tsv:1706 — ⍺-ary representation of rational ⍵; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl; The source reads the comparison tolerance; this port uses the fixed 1e-14
•LOAD 'lib/dyalog.apl'
2 ary÷3
⍝ =>
'0.0101...'

⍝ aplcart/table.tsv:1740 — Box the simple text array ⍵; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
box 2 3⍴'abcdef'
⍝ =>
4 5⍴'┌───┐│abc││def│└───┘'

⍝ aplcart/table.tsv:1743 — TeXt Packer; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
packX 'hello hello'
⍝ =>
('helo ' ⋄ 1 11 ⋄ 0 1 0 0 0 0 0 0 0 0 0 0 1 0 0 1 0 0 0 1 0 0 0 0 0 0 0 1 0 0 1 0 1 1 0 1 0)

⍝ aplcart/table.tsv:1744 — Null packing; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
packN 0 0 3 0 0 4
⍝ =>
(1⍴6 ⋄ 0 0 1 0 0 1 ⋄ 3 4)

⍝ aplcart/table.tsv:1745 — Pack a simple array; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
packB 1 0 1 1 0
⍝ =>
(1 5 1 0 ⋄ 1 1 1 0 1 ⋄ 0 1 0 1)

⍝ aplcart/table.tsv:1747 — Huffman packing; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
packH 'abbccc'
⍝ =>
(1⍴6 ⋄ 'cab' ⋄ 1 2 2 ⋄ 1 0 1 1 1 1 0 0 0)

⍝ aplcart/table.tsv:1748 — Assorted uniQues packer; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
packQ 2 1 2 1 2 1
⍝ =>
(2 1 ⋄ 1 6 ⋄ 1 0 1 0 0 1 0 1 0 0 1 0 1 0 0)

⍝ aplcart/table.tsv:1749 — Run-Length Encoding (RLE packing); Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
packR 1 1 1 2 2 3
⍝ =>
(1⍴6 ⋄ 3 2 1 ⋄ 1 2 3)

⍝ aplcart/table.tsv:1750 — Shannon-Fano packing; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
packS 'abbccc'
⍝ =>
(1⍴6 ⋄ 'cba' ⋄ 1 2 2 ⋄ 0 1 0 1 1 1 1 1 0 1 0 0 0 0)

⍝ aplcart/table.tsv:1752 — Unique packing; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
packU 'abbccc'
⍝ =>
(1⍴6 ⋄ 'abc' ⋄ 0 1 1 2 2 2)

⍝ aplcart/table.tsv:1753 — Mayan numbers; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
mayan 42
⍝ =>
2 1⍴(2 5⍴'      ⍟ ⍟ ' ⋄ 2 5⍴'      ⍟ ⍟ ')

⍝ aplcart/table.tsv:1754 — Reformat dfn/op representation; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
refmt 2 7⍴'{  ⍵+1}'
⍝ =>
2 7⍴'{  ⍵+1}{  ⍵+1}'

⍝ aplcart/table.tsv:1755 — ⎕TS format from day number; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
date 0
⍝ =>
1899 12 31 0 0 0 0

⍝ aplcart/table.tsv:1756 — Day number from ⎕TS format; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
days 1899 12 31
⍝ =>
0

⍝ aplcart/table.tsv:1762 — Quad-tree packing; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
pack4 2 2⍴1 0 0 1
⍝ =>
(1 0 ⋄ 2 2 ⋄ ¯2 0 1 ¯1 1 0)

⍝ aplcart/table.tsv:1809 — Knight's Tour Chess Problem; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
kt 3 4
⍝ =>
1⍴⊂(3 4⍴1 4 7 10 12 9 2 5 3 6 11 8)

⍝ aplcart/table.tsv:1824 — Draw over '*'s; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
draw 3 3⍴'**** ****'
⍝ =>
3 3⍴'┌─┐│ │└─┘'

⍝ aplcart/table.tsv:1828 — Show dfn with “white dots”; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
dots 1 5⍴'{⍵+1}'
⍝ =>
1 5⍴'{⍵+1}'

⍝ aplcart/table.tsv:1846 — Capitalise first letters of names; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl; Unicode lower/upper casing uses •C
•LOAD 'lib/dyalog.apl'
von 'ludwig van beethoven'
⍝ =>
'Ludwig van Beethoven'

⍝ aplcart/table.tsv:1907 — Solution vector for Sudoku problem ⍵; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
sudoku 4 4⍴1 0 0 4 0 4 1 0 0 1 4 0 4 0 0 1
⍝ =>
(4 4⍴1 2 3 4 3 4 1 2 2 1 4 3 4 3 2 1 ⋄ 4 4⍴1 3 2 4 2 4 1 3 3 1 4 2 4 2 3 1)

⍝ aplcart/table.tsv:2162 — Logical function array; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
0 0 1 1 ({7}logic)0 1 0 1
⍝ =>
0 1 1 1

⍝ aplcart/table.tsv:2179 — Apply fn at depths k; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
1 2(+Depth 0)3 4
⍝ =>
4 6

⍝ aplcart/table.tsv:2218 — Select statement; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
0 1(+case -)5
⍝ =>
¯5

⍝ aplcart/table.tsv:1622 — The N-Queens Problem; Ported in lib/dyalog.apl; checked against the independent Dyalog expectation; Concrete arguments; shared origin-one based-array port loaded from lib/dyalog.apl
•LOAD 'lib/dyalog.apl'
≢queens 4
⍝ =>
1

⍝ aplcart/table.tsv:1702 — Source vector split into words; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Seeded Boolean scan tracks word state; use the original Dyalog name-start alphabet as fixed data
•LOAD 'lib/dyalog.apl'
words 'x←1+2'
⍝ =>
(1⍴'x' ⋄ '←1+2')

⍝ aplcart/table.tsv:1714 — Enclose-if-simple / link; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Lazy default-left assignment distinguishes monadic enclosure from dyadic link; no name introspection
•LOAD 'lib/dyalog.apl'
eis 1 2 3
⍝ =>
⊂(1 2 3)

⍝ aplcart/table.tsv:1717 — Generalized iota; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Origin-one interval/search results; type detection uses fill, and Dyalog character order is fixed library data
•LOAD 'lib/dyalog.apl'
2 iotag 6
⍝ =>
2 3 4 5 6

⍝ aplcart/table.tsv:1739 — Lex of APL src line; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Dyalog APL token grammar and name-start alphabet retained; token accumulation uses direct reductions
•LOAD 'lib/dyalog.apl'
tokens 'x←1+2'
⍝ =>
(1⍴'x' ⋄ 1⍴'←' ⋄ 1⍴'1' ⋄ 1⍴'+' ⋄ 1⍴'2')

⍝ aplcart/table.tsv:1746 — Pack character array to Boolean vector; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Dyalog alphabet retained as fixed data for the original wire format; bAsedPL partition and direct fold
•LOAD 'lib/dyalog.apl'
packD 'abc'
⍝ =>
0 0 0 1 0 0 0 1 0 1 1 0 0 0 0 0 0 1 0 0 0 0 1 0 0 0 1 0 0 0 1 0 0 1 0 0 0 0 1 0 0 1 1 1 0 1 0 0 1 1 0

⍝ aplcart/table.tsv:1751 — Simple text vector packager; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Dyalog alphabet retained as fixed data for the original wire format; windows express adjacent differences
•LOAD 'lib/dyalog.apl'
packT 'hello'
⍝ =>
•UCS 0 104 101 108 108 111

⍝ aplcart/table.tsv:1764 — Evaluator for a subset of Scheme; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Origin-one environment and closure indexing; parser offsets remain zero-based; First is ↑ and explicit output is preserved
•LOAD 'lib/dyalog.apl'
lisp '(+ 2 3)'
⍝ =>
5

⍝ aplcart/table.tsv:1768 — Bunda-Gerth parse of expression ⍵; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Concrete arithmetic grammar; origin-one binding/category indices, direct folds, First/Mix, and existing array-library display/substitution helpers
•LOAD 'lib/dyalog.apl'
g←'A 1 2 3' 'F + ×' 'B' '' 'F:A→B' 'A:B→A' ⋄ g parse '1+2×3'
⍝ =>
5 9⍴'    A     ┌──┴──┐ ┌┴─┐  ┌┴┐1 ┌┴┐ × 3  + 2    '

⍝ aplcart/table.tsv:1794 — Approximate alternative to xutils ss; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Use the word-state splitter and original identifier alphabet for whole-word substitution
•LOAD 'lib/dyalog.apl'
ssword 'alpha + beta' 'alpha' 'gamma'
⍝ =>
'gamma + beta'

⍝ aplcart/table.tsv:2195 — Cut operator; Ported in lib/dyalog.apl; checked against independent Dyalog expectation; Origin-one axes and bAsedPL operands; direct reductions and First/Mix replace nested-array idioms
•LOAD 'lib/dyalog.apl'
1 0 1 0(+/Cut 1)1 2 3 4
⍝ =>
3 7

⍝ aplcart/table.tsv:2332 — Delta-underscore; Concrete identifier binding/readback; existing lexer already accepts delta-underscore
⍙←42 ⋄ ⍙   ⍝ 42

⍝ aplcart/table.tsv:2696 — Assign (in dfns/dops) to closest scope where localised or in global scope if not localised anyhere (proper replacement for name∘←Y); Existing nearest-owner assignment expresses the source scope-update operation without a namespace object; Use right-tack modified assignment to resolve the nearest existing binding
name←1 ⋄ f←{name⊢←⍵} ⋄ f 42 ⋄ name   ⍝ 42

⍝ aplcart/table.tsv:1827 — Avoiding division by zero error (gives zero); Pure guarded arithmetic/comparison expresses this recipe under completed BasedPL decisions; Use scalar guards with Each for zero denominators
1 0 6{⍵=0:0 ⋄ ⍺÷⍵}¨0 0 2   ⍝ 0 0 3

⍝ aplcart/table.tsv:2285 — Tolerant comparison: Do M and N match with an absolute tolerance of Ms?; Pure guarded arithmetic/comparison expresses this recipe under completed BasedPL decisions; Drop mutable tolerance settings; concrete comparisons use the fixed BasedPL comparison policy and retain the independent Dyalog result
f←1e¯3{~0∊⍶≥|⍺-⍵} ⋄ (1 2 f 1.0005 2.0005⋄ 1 2 f 1.002 2)
1 0

⍝ aplcart/table.tsv:2305 — Tolerant comparison: Do M and N match with a relative tolerance of Ms?; Pure guarded arithmetic/comparison expresses this recipe under completed BasedPL decisions; Drop mutable tolerance settings; concrete comparisons use the fixed BasedPL comparison policy and retain the independent Dyalog result
f←1e¯3{~0∊⍶≥|⍺(-÷⌈⍥|)⍵} ⋄ (100 200 f 100.05 200.05⋄ 100 200 f 100.2 200)
1 0

⍝ aplcart/table.tsv:2798 — Called Monadically? (dfns/dops only); Lazy default-left assignment detects call valence without introspection; Test both monadic and dyadic calls using a local default-assignment flag
f←{m←0 ⋄ ⍺←m←1 ⋄ m} ⋄ (f 0⋄ 0 f 0)   ⍝ 1 0

⍝ aplcart/table.tsv:2799 — Called Dyadically? (dfns/dops only); Lazy default-left assignment detects call valence without introspection; Test both monadic and dyadic calls using a local default-assignment flag
f←{d←1 ⋄ ⍺←d←0 ⋄ d} ⋄ (f 0⋄ 0 f 0)   ⍝ 0 1

⍝ aplcart/table.tsv:2902 — Number of arguments used in call (dfns/dops only); Lazy default-left assignment detects call valence without introspection; Test both monadic and dyadic calls using a local default-assignment flag
f←{n←2 ⋄ ⍺←n←1 ⋄ n} ⋄ (f 0⋄ 0 f 0)   ⍝ 1 2

⍝ aplcart/table.tsv:2117 — Turn Y into a character vector that looks visually like ⎕FMT Y; Existing Format and Unicode functions express this text-rendering recipe; Use ⍕ for plain matrix formatting and •UCS for the line separator
{¯1↓∊({(+/∨\' '≠⌽⍵)↑¨↓⍵}⍕⍵),¨•UCS 10}2 2⍴1 20 300 4
•UCS 32 32 49 32 50 48 10 51 48 48 32 32 52

⍝ aplcart/table.tsv:2228 — Arrayify: Convert f into an array for use in an array of functions; BasedPL function atoms already belong in ordinary arrays; no object-representation API is needed; Store functions with explicit stranding and retrieve one with First before application
fs←+˘× ⋄ f←↑fs ⋄ 2 f 3   ⍝ 5

⍝ aplcart/table.tsv:239 — Convert (⎕TS-style) date-times to Dyalog date numbers; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ days 2024 2 29   ⍝ 45350

⍝ aplcart/table.tsv:250 — Convert (⎕TS-style or Dyalog date number) date-times to UNIX time numbers; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 86400×(days 2024 2 29)-days 1970 1 1
1709164800

⍝ aplcart/table.tsv:645 — Convert UNIX times to ⎕TS-style timestamps; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ date(1709164800÷86400)+days 1970 1 1
2024 2 29 0 0 0 0

⍝ aplcart/table.tsv:646 — Convert (⎕TS-style or Dyalog date number) date-times to UNIX times; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 86400×(days 2024 2 29)-days 1970 1 1
1709164800

⍝ aplcart/table.tsv:650 — Convert (Dyalog date number) date-times to ⎕TS-style time-stamps; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ date 45350   ⍝ 2024 2 29 0 0 0 0

⍝ aplcart/table.tsv:994 — Day of week (Sunday:0); Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 7|days 2024 2 29
4

⍝ aplcart/table.tsv:1082 — Number of days between (⎕TS-style or Dyalog date number) date-times I and J; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 2024 3 1-⍥days 2024 2 28
2

⍝ aplcart/table.tsv:1083 — Day of week (Sunday:0) of first of January J; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 7|days¨2023 2024,¨⊂1 1
0 1

⍝ aplcart/table.tsv:3771 — Number of days in month Js of year Is; Prepared independent calendar example using existing days/date library functions; Correct upstream next-month calculation: subtract one from the year before mixed-radix encoding; Original formula gives 29/28 for February 2023/2024; corrected Dyalog and BasedPL give 28/29
•LOAD 'lib/dyalog.apl' ⋄ f←{3⊃date¯1+days(1+0 12⊤12⊥(⍺-1),⍵),1} ⋄ 2023 2024 f¨2 2
28 29

⍝ aplcart/tt.tsv:299 — Convert (Dyalog date number) date-times to ⎕TS-style time stamps; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ date 45350   ⍝ 2024 2 29 0 0 0 0

⍝ aplcart/tt.tsv:300 — Convert (Dyalog date number) date-times to ⎕TS-style time stamps; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ date 45350   ⍝ 2024 2 29 0 0 0 0

⍝ aplcart/tt.tsv:313 — Convert UNIX times to ⎕TS-style timestamps; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ date(1709164800÷86400)+days 1970 1 1
2024 2 29 0 0 0 0

⍝ aplcart/tt.tsv:314 — Convert UNIX times to ⎕TS-style timestamps; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ date(1709164800÷86400)+days 1970 1 1
2024 2 29 0 0 0 0

⍝ aplcart/tt.tsv:355 — Convert ⎕TS-style timestamps to UNIX times; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 86400×(days 2024 2 29)-days 1970 1 1
1709164800

⍝ aplcart/tt.tsv:356 — Convert ⎕TS-style timestamps to UNIX times; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 86400×(days 2024 2 29)-days 1970 1 1
1709164800

⍝ aplcart/tt.tsv:417 — Day of week (Sunday:0); Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 7|days 2024 2 29
4

⍝ aplcart/tt.tsv:418 — Day of week (Sunday:0); Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 7|days 2024 2 29
4

⍝ aplcart/tt.tsv:419 — Day of week (Sunday:0) of first of January J; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 7|days¨2023 2024,¨⊂1 1
0 1

⍝ aplcart/tt.tsv:420 — Day of week (Sunday:0) of first of January J; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 7|days¨2023 2024,¨⊂1 1
0 1

⍝ aplcart/tt.tsv:1287 — Number of days between ⎕TS-style timestamps I and J; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 2024 3 1-⍥days 2024 2 28
2

⍝ aplcart/tt.tsv:1288 — Number of days between ⎕TS-style timestamps I and J; Prepared independent calendar example using existing days/date library functions; Concrete modern dates through dfns days/date and epoch arithmetic; calendar conversion policy outside these examples is unchanged
•LOAD 'lib/dyalog.apl' ⋄ 2024 3 1-⍥days 2024 2 28
2

⍝ aplcart/table.tsv:241 — Convert (⎕TS-style or Dyalog date number) date-times to J nanosecond time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400000000000x×(days 2024 2 29)-0 days 2000 1 1
7.6248e17

⍝ aplcart/table.tsv:243 — Convert (⎕TS-style or Dyalog date number) date-times to Shakti K7 time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400000×(days 2024 2 29)-0 days 2024 1 1
5097600000

⍝ aplcart/table.tsv:245 — Convert (⎕TS-style or Dyalog date number) date-times to JavaScript/D/Q time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400000×(days 2024 2 29)-0 days 1970 1 1
1709164800000

⍝ aplcart/table.tsv:246 — Convert (⎕TS-style or Dyalog date number) date-times to R chron format time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 1×(days 2024 2 29)-0 days 1970 1 1
19782

⍝ aplcart/table.tsv:247 — Convert (⎕TS-style or Dyalog date number) date-times to Shakti K9 time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400000×(days 2024 2 29)-0 days 2001 1 1
730857600000

⍝ aplcart/table.tsv:248 — Convert (⎕TS-style or Dyalog date number) date-times to Dyalog component file time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 5184000×(days 2024 2 29)-0 days 1970 1 1
102549888000

⍝ aplcart/table.tsv:257 — Convert (⎕TS-style or Dyalog date number) date-times to Microsoft Win32 FILETIME numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 864000000000x×(days 2024 2 29)-0 days 1601 1 1
1.33536384e17

⍝ aplcart/table.tsv:259 — Convert (⎕TS-style or Dyalog date number) date-times to Microsoft CLR DateTime (.NET) ticks property numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 864000000000x×(days 2024 2 29)-0 days 1 1 1
6.38447616e17

⍝ aplcart/table.tsv:260 — Convert (⎕TS-style or Dyalog date number) date-times to Microsoft OLE Automation Date numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 1×(days 2024 2 29)-0 days 1899 12 30
45351

⍝ aplcart/table.tsv:264 — Convert (⎕TS-style or Dyalog date number) date-times to Excel (1904 Date System) time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 1×(days 2024 2 29)-0 days 1904 1 1
43889

⍝ aplcart/table.tsv:266 — Convert (⎕TS-style or Dyalog date number) date-times to Stata statistics package time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400000×(days 2024 2 29)-0 days 1960 1 1
2024784000000

⍝ aplcart/table.tsv:267 — Convert (⎕TS-style or Dyalog date number) date-times to SPSS statistics package time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400×(days 2024 2 29)-0 days 1582 10 14
13928544000

⍝ aplcart/table.tsv:268 — Convert (⎕TS-style or Dyalog date number) date-times to SAS time numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400×(days 2024 2 29)-0 days 1960 1 1
2024784000

⍝ aplcart/table.tsv:272 — Convert (⎕TS-style or Dyalog date number) date-times to J daynos; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 1×(days 2024 2 29)-0 days 1800 1 1
81873

⍝ aplcart/table.tsv:274 — Convert (⎕TS-style or Dyalog date number) date-times to Reduced Julian Date numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 1×(days 2024 2 29)-0 days 1858 11 16 12
60369.5

⍝ aplcart/table.tsv:276 — Convert (⎕TS-style or Dyalog date number) date-times to Modified Julian Date numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 1×(days 2024 2 29)-0 days 1858 11 17
60369

⍝ aplcart/table.tsv:278 — Convert (⎕TS-style or Dyalog date number) date-times to Dublin Julian Date numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 1×(days 2024 2 29)-0 days 1899 12 31 12
45349.5

⍝ aplcart/table.tsv:280 — Convert (⎕TS-style or Dyalog date number) date-times to CNES Julian Date numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 1×(days 2024 2 29)-0 days 1950 1 1
27087

⍝ aplcart/table.tsv:281 — Convert (⎕TS-style or Dyalog date number) date-times to CCSDS Julian Date numbers; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 1×(days 2024 2 29)-0 days 1958 1 1
24165

⍝ aplcart/table.tsv:642 — Convert (⎕TS-style or Dyalog date number) date-times to Go UnixMicro times; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400000000x×(days 2024 2 29)-0 days 1970 1 1
1709164800000000

⍝ aplcart/table.tsv:643 — Convert (⎕TS-style or Dyalog date number) date-times to Go UnixNano times; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400000000000x×(days 2024 2 29)-0 days 1970 1 1
1.7091648e18

⍝ aplcart/table.tsv:644 — Convert (⎕TS-style or Dyalog date number) date-times to APL+Win/APL64 workspace times; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400000000x×(days 2024 2 29)-0 days 1900 1 1
3918153600000000

⍝ aplcart/table.tsv:647 — Convert (⎕TS-style or Dyalog date number) date-times to Apollo NCS UUID times; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 21600000000x×(days 2024 2 29)-0 days 1980 1 1
348408000000000

⍝ aplcart/table.tsv:648 — Convert (⎕TS-style or Dyalog date number) date-times to OSF DCE UUID times; Prepared independent epoch-arithmetic example using existing days; Concrete modern timestamp using documented epoch/tick scale and dfns days; The epoch uses proleptic Gregorian days (left argument 0); no system-format selector API is introduced
•LOAD 'lib/dyalog.apl' ⋄ 86400000000000x×(days 2024 2 29)-0 days 1582 10 15
1.39284576e19

⍝ aplcart/table.tsv:2491 — Determine the prime factors of the argument; Prepared command-catalogue algorithm with concrete input and independent result; Use existing pure mathematical primitive/library function instead of the Dyalog user-command wrapper
⨸360   ⍝ 2 2 2 3 3 5

⍝ aplcart/table.tsv:2492 — Convert a list of hexadecimal representations of integers to a numeric vector; Prepared command-catalogue algorithm with concrete input and independent result; Use existing pure mathematical primitive/library function instead of the Dyalog user-command wrapper
•LOAD 'lib/numeric.apl' ⋄ dec 'ff' '100' 'abc'
255 256 2748

⍝ aplcart/table.tsv:2494 — Convert integer(s) to a vector of text vectors containing the hexadecimal representation of each number; Prepared command-catalogue algorithm with concrete input and independent result; Use existing pure mathematical primitive/library function instead of the Dyalog user-command wrapper
•LOAD 'lib/numeric.apl' ⋄ hex 255 256 2748
('00ff' ⋄ '0100' ⋄ '0abc')

⍝ aplcart/table.tsv:255 — Convert (⎕TS-style or Dyalog date number) date-times to Microsoft DOS date/time numbers; Prepared independent date-component arithmetic example; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ {year month day hour minute second←⍵ ⋄ 128 16 32 32 64 32⊥(year-1980),month,day,hour,minute,⌊second÷2}2024 2 29 12 34 56
1482515548

⍝ aplcart/table.tsv:262 — Convert (⎕TS-style or Dyalog date number) date-times to Excel (1900 Date System)/Lotus 1-2-3 time numbers; Prepared independent date-component arithmetic example; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ 1+days 2024 2 29
45351

⍝ aplcart/table.tsv:270 — Convert (⎕TS-style or Dyalog date number) date-times to Julian date numbers; Prepared independent date-component arithmetic example; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ 2415019.5+days 2024 2 29
2460369.5

⍝ aplcart/table.tsv:283 — Convert (⎕TS-style or Dyalog date number) date-times to YYYYMMDD.hhmmss floating-point decimal encoded datetime format; Prepared independent date-component arithmetic example; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
{(100⊥3↑⍵)+(100⊥3↓⍵)÷1000000}2024 2 29 12 34 56
20240229.123456

⍝ aplcart/table.tsv:285 — Convert (⎕TS-style or Dyalog date number) date-times to YYYYMMMDDhhmmss integer encoded datetime formatsJ digit time; Prepared independent date-component arithmetic example; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ 100⊥2024 2 29 12 34 56
20240229123456

⍝ aplcart/table.tsv:651 — Convert (⎕TS-style or Dyalog date number) date-times to ISO day-of-year components time-stamps; Prepared independent date-component arithmetic example; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ 2024,(1+(days 2024 2 29)-days 2024 1 1),12 34 56 123000
2024 60 12 34 56 123000

⍝ aplcart/table.tsv:654 — Convert (⎕TS-style or Dyalog date number) date-times to decimal encoded date and time-stamps; Prepared independent date-component arithmetic example; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ 100⊥¨(2024 2 29⋄ 12 34 56)
20240229 123456

⍝ aplcart/table.tsv:656 — Convert (⎕TS-style or Dyalog date number) date-times to DateTimePicker format time-stamps; Prepared independent date-component arithmetic example; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ (days 2024 2 29),12 34 56
45350 12 34 56

⍝ aplcart/table.tsv:1783 — Time-stamped message; Prepared text helper with independent explicit-argument expectation; Supply the documented explicit time/width argument and concrete text; use the existing library port
•LOAD 'lib/string.apl' ⋄ 2024 2 29 12 34 56 0 timestamp 'Ready'
'2024-02-29 12:34:56 Ready'

⍝ aplcart/table.tsv:1785 — Wrap word vector at ⍺ columns; Prepared text helper with independent explicit-argument expectation; Pass explicit width 10 to existing wrap; Normalize Dyalog CR line separators to the library port’s LF; preserve original capture
•LOAD 'lib/string.apl' ⋄ 10 wrap 'one two three four five'
•UCS 111 110 101 32 116 119 111 10 116 104 114 101 101 32 102 111 117 114 10 102 105 118 101

⍝ aplcart/table.tsv:1868 — Wrap text paras in note vect; Prepared text helper with independent explicit-argument expectation; Supply the documented explicit time/width argument and concrete text; use the existing library port
•LOAD 'lib/string.apl' ⋄ 12 18 wrapnote 'Title',(•UCS 13 13),'one two three four',(•UCS 13),'five six seven',(•UCS 13)
•UCS 84 105 116 108 101 13 13 111 110 101 32 32 32 32 32 32 116 119 111 13 116 104 114 101 101 32 32 32 102 111 117 114 13 102 105 118 101 32 32 32 32 32 115 105 120 13 115 101 118 101 110 13

⍝ aplcart/table.tsv:2070 — 2's-complement bit-wise OR; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥∨/(w⍴2)⊤⍉I,[0.5]J
¯1 ¯5 ¯1 ¯1

⍝ aplcart/table.tsv:2071 — 2's-complement bit-wise AND; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥∧/(w⍴2)⊤⍉I,[0.5]J
0 2 ¯2 0

⍝ aplcart/table.tsv:2072 — 2's-complement bit-wise converse nonimplication; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥</(w⍴2)⊤⍉I,[0.5]J
2 ¯7 0 ¯1

⍝ aplcart/table.tsv:2073 — 2's-complement bit-wise implication; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥≤/(w⍴2)⊤⍉I,[0.5]J
2 ¯1 ¯2 ¯1

⍝ aplcart/table.tsv:2074 — 2's-complement bit-wise XNOR; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥=/(w⍴2)⊤⍉I,[0.5]J
0 6 ¯2 0

⍝ aplcart/table.tsv:2075 — 2's-complement bit-wise converse implication; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥≥/(w⍴2)⊤⍉I,[0.5]J
¯3 6 ¯1 0

⍝ aplcart/table.tsv:2076 — 2's-complement bit-wise nonimplication; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥>/(w⍴2)⊤⍉I,[0.5]J
¯3 0 1 0

⍝ aplcart/table.tsv:2077 — 2's-complement bit-wise XOR; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥≠/(w⍴2)⊤⍉I,[0.5]J
¯1 ¯7 1 ¯1

⍝ aplcart/table.tsv:2078 — 2's-complement bit-wise NOR; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥⍱/(w⍴2)⊤⍉I,[0.5]J
0 4 0 0

⍝ aplcart/table.tsv:2079 — 2's-complement bit-wise NAND; Prepared signed-bitwise algorithm using ordinary fixed-width encoding; Python integer truth-table expectations retained; Use an explicit sufficient two’s-complement width, ordinary Encode, and the existing int signed-decoding helper; This avoids undefined inverse Decode on negatives without changing language semantics or reducing the original mixed-sign inputs
•LOAD 'lib/numeric.apl' ⋄ I←¯3 2 ¯1 0 ⋄ J←2 ¯5 ¯2 ¯1 ⋄ w←1+⌈2⍟1+⌈/|I,J ⋄ w int 2⊥⍲/(w⍴2)⊤⍉I,[0.5]J
¯1 ¯3 1 ¯1

⍝ aplcart/tt.tsv:301 — Convert (⎕TS-style or Dyalog date number) date-times to DateTimePicker format time stamps; Prepared duplicate calendar recipe with the same independently captured timestamp; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ (days 2024 2 29),12 34 56
45350 12 34 56

⍝ aplcart/tt.tsv:302 — Convert (⎕TS-style or Dyalog date number) date-times to DateTimePicker format time stamps; Prepared duplicate calendar recipe with the same independently captured timestamp; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ (days 2024 2 29),12 34 56
45350 12 34 56

⍝ aplcart/tt.tsv:305 — Convert (⎕TS-style or Dyalog date number) date-times to ISO day-of-year components time stamps; Prepared duplicate calendar recipe with the same independently captured timestamp; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ 2024,(1+(days 2024 2 29)-days 2024 1 1),12 34 56 123000
2024 60 12 34 56 123000

⍝ aplcart/tt.tsv:306 — Convert (⎕TS-style or Dyalog date number) date-times to ISO day-of-year components time stamps; Prepared duplicate calendar recipe with the same independently captured timestamp; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ 2024,(1+(days 2024 2 29)-days 2024 1 1),12 34 56 123000
2024 60 12 34 56 123000

⍝ aplcart/tt.tsv:307 — Convert (⎕TS-style or Dyalog date number) date-times to decimal encoded date and time stamps; Prepared duplicate calendar recipe with the same independently captured timestamp; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ 100⊥¨(2024 2 29⋄ 12 34 56)
20240229 123456

⍝ aplcart/tt.tsv:308 — Convert (⎕TS-style or Dyalog date number) date-times to decimal encoded date and time stamps; Prepared duplicate calendar recipe with the same independently captured timestamp; Concrete timestamp using existing calendar library and numeric encoding; Excel example is after its fictitious 1900 leap day
•LOAD 'lib/dyalog.apl' ⋄ 100⊥¨(2024 2 29⋄ 12 34 56)
20240229 123456

⍝ aplcart/table.tsv:652 — Convert (⎕TS-style or Dyalog date number) date-times to ISO day-of-week components time-stamps; Prepared existing-array arithmetic with independently captured date components; Explicit timestamp: ISO week uses the year of its Thursday; subsecond components are rescaled arithmetically; No clock or general date-conversion API needed
•LOAD 'lib/dyalog.apl' ⋄ f←{d←days 3↑⍵ ⋄ w←1+7|d-1 ⋄ th←d+4-w ⋄ y←↑date th ⋄ y,(1+⌊(th-days y 1 1)÷7),w,(3↑3↓⍵),(7⊃⍵)×1000} ⋄ f 2021 1 1 12 34 56 123
2020 53 5 12 34 56 123000

⍝ aplcart/table.tsv:653 — Convert (⎕TS-style or Dyalog date number) date-times to microsecond precision ⎕TS-style time-stamps; Prepared existing-array arithmetic with independently captured date components; Explicit timestamp: ISO week uses the year of its Thursday; subsecond components are rescaled arithmetically; No clock or general date-conversion API needed
•LOAD 'lib/dyalog.apl' ⋄ (1000∘×)@7⊢2024 2 29 12 34 56 123
2024 2 29 12 34 56 123000

⍝ aplcart/table.tsv:655 — Convert (⎕TS-style or Dyalog date number) date-times to nanosecond precision ⎕TS-style time-stamps; Prepared existing-array arithmetic with independently captured date components; Explicit timestamp: ISO week uses the year of its Thursday; subsecond components are rescaled arithmetically; No clock or general date-conversion API needed
•LOAD 'lib/dyalog.apl' ⋄ (1000000∘×)@7⊢2024 2 29 12 34 56 123
2024 2 29 12 34 56 123000000

⍝ aplcart/tt.tsv:303 — Convert (⎕TS-style or Dyalog date number) date-times to ISO day-of-week components time stamps; Prepared existing-array arithmetic with independently captured date components; Explicit timestamp: ISO week uses the year of its Thursday; subsecond components are rescaled arithmetically; No clock or general date-conversion API needed
•LOAD 'lib/dyalog.apl' ⋄ f←{d←days 3↑⍵ ⋄ w←1+7|d-1 ⋄ th←d+4-w ⋄ y←↑date th ⋄ y,(1+⌊(th-days y 1 1)÷7),w,(3↑3↓⍵),(7⊃⍵)×1000} ⋄ f 2021 1 1 12 34 56 123
2020 53 5 12 34 56 123000

⍝ aplcart/tt.tsv:304 — Convert (⎕TS-style or Dyalog date number) date-times to ISO day-of-week components time stamps; Prepared existing-array arithmetic with independently captured date components; Explicit timestamp: ISO week uses the year of its Thursday; subsecond components are rescaled arithmetically; No clock or general date-conversion API needed
•LOAD 'lib/dyalog.apl' ⋄ f←{d←days 3↑⍵ ⋄ w←1+7|d-1 ⋄ th←d+4-w ⋄ y←↑date th ⋄ y,(1+⌊(th-days y 1 1)÷7),w,(3↑3↓⍵),(7⊃⍵)×1000} ⋄ f 2021 1 1 12 34 56 123
2020 53 5 12 34 56 123000

⍝ aplcart/tt.tsv:309 — Convert (⎕TS-style or Dyalog date number) date-times to microsecond precision ⎕TS-style time stamps; Prepared existing-array arithmetic with independently captured date components; Explicit timestamp: ISO week uses the year of its Thursday; subsecond components are rescaled arithmetically; No clock or general date-conversion API needed
•LOAD 'lib/dyalog.apl' ⋄ (1000∘×)@7⊢2024 2 29 12 34 56 123
2024 2 29 12 34 56 123000

⍝ aplcart/tt.tsv:310 — Convert (⎕TS-style or Dyalog date number) date-times to microsecond precision ⎕TS-style time stamps; Prepared existing-array arithmetic with independently captured date components; Explicit timestamp: ISO week uses the year of its Thursday; subsecond components are rescaled arithmetically; No clock or general date-conversion API needed
•LOAD 'lib/dyalog.apl' ⋄ (1000∘×)@7⊢2024 2 29 12 34 56 123
2024 2 29 12 34 56 123000

⍝ aplcart/tt.tsv:311 — Convert (⎕TS-style or Dyalog date number) date-times to nanosecond precision ⎕TS-style time stamps; Prepared existing-array arithmetic with independently captured date components; Explicit timestamp: ISO week uses the year of its Thursday; subsecond components are rescaled arithmetically; No clock or general date-conversion API needed
•LOAD 'lib/dyalog.apl' ⋄ (1000000∘×)@7⊢2024 2 29 12 34 56 123
2024 2 29 12 34 56 123000000

⍝ aplcart/tt.tsv:312 — Convert (⎕TS-style or Dyalog date number) date-times to nanosecond precision ⎕TS-style time stamps; Prepared existing-array arithmetic with independently captured date components; Explicit timestamp: ISO week uses the year of its Thursday; subsecond components are rescaled arithmetically; No clock or general date-conversion API needed
•LOAD 'lib/dyalog.apl' ⋄ (1000000∘×)@7⊢2024 2 29 12 34 56 123
2024 2 29 12 34 56 123000000

⍝ aplcart/table.tsv:2188 — Trace of function application; Prepared tracing dop using first-class function values and explicit output; no function-source introspection required; Trace ordinary function values rather than reconstructing a function name through ⎕OR. Preserve monadic/dyadic calls and return the operand result
•LOAD 'lib/dyalog.apl' ⋄ a←(-tc)3 ⋄ b←2(+tc)3 ⋄ a b
¯3 5

⍝ aplcart/table.tsv:1843 — Multi-column display; Prepared pure column layout using explicit width; no terminal state required; Use based Mix/direct reduction and the existing library width default 102; retain explicit gap/width argument
•LOAD 'lib/dyalog.apl' ⋄ (1 14 cols 'one' 'two' 'three' 'four' 'five' ⋄ 1 4 cols 'abcdefgh' 'ijk' ⋄ 1 14 cols ⊃'one' 'two' 'three')
(2 14⍴'one three fivetwo four      ' ⋄ 2 4⍴'abcdijk ' ⋄ 1 13⍴'one two three')

⍝ aplcart/table.tsv:2493 — Create a pivot table from an appropriate matrix; Prepared independent concrete example of the command’s mathematical operation; Express the pure calculation with existing array operations/library functions, without the command wrapper or file/workspace facilities
t←5 3⍴1 1 10 1 2 20 2 1 30 1 1 5 2 2 40 ⋄ r c v←↓⍉t ⋄ (∪r){+/v/⍨(r=⍺)∧c=⍵}⌝∪c
2 2⍴15 20 30 40

⍝ aplcart/table.tsv:2509 — Convert a component file timestamp (single float number) to ⎕TS format (vector of 7 numbers); Prepared independent concrete example of the command’s mathematical operation; Express the pure calculation with existing array operations/library functions, without the command wrapper or file/workspace facilities
•LOAD 'lib/dyalog.apl' ⋄ date(days 1970 1 1)+102549888000÷5184000
2024 2 29 0 0 0 0

⍝ aplcart/table.tsv:1895 — Convert namespace of column vectors into table (matrix with names in header row); Namespace used as a record of column vectors: adapted to a keyed vector built from the example's JSON object. `⎕VGET ¯2` name-value pairs become the key array `⍳⍵` and its values `(⍳⍵)⊃⍵`; Mix is `⊃` in bAsedPL
T←('Age':'21' '32' ⋄ 'Name':'Bob' 'Sally' ⋄ 'Zipcode':'30102' '43001') ⋄ {(⍳⍵)⍪⍉⊃(⍳⍵)⊃⍵}T
3 3⍴('Age') ('Name') ('Zipcode') ('21') ('Bob') ('30102') ('32') ('Sally') ('43001')

⍝ aplcart/table.tsv:2340 — Namespace Member; Dyalog 20 namespace syntax used as a record; Adapted to keyed arrays: keys are quoted strings, and a keyed result is shown through ordinary arrays because the structured `expected` format cannot express keys
ns←('name':42) ⋄ ns.name   ⍝ 42

⍝ aplcart/table.tsv:2678 — New empty namespace; Dyalog 20 namespace syntax used as a record; Adapted to keyed arrays: keys are quoted strings, and a keyed result is shown through ordinary arrays because the structured `expected` format cannot express keys
≢()   ⍝ 0

⍝ aplcart/table.tsv:2948 — New namespace with members name1, name2, name3 and values X, Y, Z; Dyalog 20 namespace syntax used as a record; Adapted to keyed arrays: keys are quoted strings, and a keyed result is shown through ordinary arrays because the structured `expected` format cannot express keys
R←('name1':1 ⋄ 'name2':'two' ⋄ 'name3':3 4) ⋄ K←⍳R ⋄ (K ⋄ K⊃R)
(('name1') ('name2') ('name3')) (1 ('two') (3 4))

