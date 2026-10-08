⍝⍝ Choose

⍝ bqn:choose:7 — Choose subtracts 1 from negatives and adds 1 otherwise
(0≤)⍚[-↢1 +↢1]¨ 3 ¯1 5   ⍝ 4 ¯2 6

⍝ bqn:choose:11 — Choose with two arguments as a minimum
2 >⍚[⊣ ⊢] 6   ⍝ 2

⍝ bqn:choose:15 — Choose dispatches on the first character, with a default for no match
fn←("rtd"⍳↑)⍚[⌽ 1↑ 1↓ ⊢] ⋄ [fn "r123";fn "d123";fn "123"]
"321r" "123" "123"

⍝⍝ Valences

⍝ bqn:valences:11 — Valences calls the left function with one argument and the right with two
[-⊘+ 6;3 -⊘+ 2]   ⍝ ¯6 5

⍝ bqn:valences:17 — Valences with array operands tests for a left argument
[{⍺ 0⊘1 ⍵}'x';'w'{⍺ 0⊘1 ⍵}'x']   ⍝ 0 1

⍝⍝ Under

⍝ bqn:under:43 — Square root of the sum of squares
3 +⌾(×⍨) 4   ⍝ 5

⍝ bqn:under:47 — Prefix sum through Join keeps the nesting
+\⌾∊ [3 1 0] [2 5] [0 0 6]   ⍝ (3 4 4⋄6 11⋄11 11 17)

⍝ bqn:under:61 — Rotate the first column
a←[0 1 2⋄3 4 5⋄6 7 8⋄9 10 11] ⋄ (1⌽)⌾(↑⍤1) a
[3 1 2⋄6 4 5⋄9 7 8⋄0 10 11]

⍝ bqn:under:69 — Reverse the first half of a list
⌽⌾((2÷⍨≢)↑⊢) "abcdef"   ⍝ "cbadef"

⍝ bqn:under:73 — Add 10 to the elements below 5, with the mask computed from the argument
{10+⌾((⍵<5)#)⍵} 3 8 2 2 6   ⍝ 13 8 12 12 6

⍝ bqn:under:81 — Scan from the end
∧\⌾⌽ 1 0 1 0 1 1 1   ⍝ 0 0 0 0 1 1 1

⍝ bqn:under:89 — Root-mean-square
(+/÷≢)⌾(×⍨) 2 3 4 5   ⍝ 3.6742346141747673

⍝ bqn:under:99 — An arithmetic progression up to a limit, through an inverted transformation
⍳∘⌈⌾((4+3×⊢)⁻¹) 20   ⍝ 4 7 10 13 16 19

⍝ bqn:under:109 — Replace selected positions with the same positions of the left argument
"abcd" ⊣⌾([[1 2]]⌷) "0123"   ⍝ "0bc3"

⍝ bqn:under:113 — Replace the last element of the first two lists
["ab" "cde" "fg"] ⊣⌾((¯1⌷)¨2↑⊢) ⍳¨3 2 1 1
(0 1 'b'⋄0 'e'⋄0⋄0)

⍝ bqn:under:117 — Regroup characters into the structure of the right argument
["ab" "cde" "fg"] ⊣⌾∊ ["---" "----"]   ⍝ "abc" "defg"

⍝ bqn:under:123 — Add to the selected elements, keeping the others
(1 2 3+)⌾(1 1 0 1#) 10 20 30 40   ⍝ 11 22 30 43

⍝⍝ Undo

⍝ bqn:undo:9 — Undo a rotation
2 ⌽⁻¹ 2⌽"abcde"   ⍝ "abcde"

⍝ bqn:undo:24 — The inverse of squaring returns the principal root
×⍨⁻¹ ×⍨ ¯3   ⍝ 3

⍝ bqn:undo:36 — Undo multiplication by a bound left argument
(3×)⁻¹ 12   ⍝ 4

⍝ bqn:undo:51 — Undo Left accepts arguments that already match
3 ⊣⁻¹ 3   ⍝ 3

⍝⍝ Repeat

⍝ bqn:repeat:24 — Repeat a dyadic function, keeping the left argument
3 (+⍣2) 7   ⍝ 13

⍝ bqn:repeat:39 — Halve the numbers above 6, using a computed count of 0 or 1
{(÷↢2)⍣(6<⍵)⍵}¨ 3 7 2 1 8   ⍝ 3 3.5 2 1 4

⍝ bqn:repeat:49 — A negative count applies the inverse
1 (⌽⍣¯1) "abcde"   ⍝ "eabcd"

⍝ bqn:repeat:57 — Nested counts give results with the same structure
2×⍣[2 [4 ¯2 1]] 1   ⍝ [4 [16 0.25 2]]

⍝⍝ Windows

⍝ bqn:windows:49 — Windows of length 5
5↕"abcdefg"   ⍝ ["abcde"⋄"bcdef"⋄"cdefg"]

⍝ bqn:windows:55 — A window is a take after a drop
(2⌷5↕"abcdefg")≡5↑2↓"abcdefg"   ⍝ $t

⍝ bqn:windows:65 — Sums of windows of length 3
+/3↕2 6 0 1 4 3   ⍝ 8 7 5 8

⍝ bqn:windows:71 — Differences of a prefix sum recover the original list
((-⍨⌿)≢↕0,⊢)+\3 2 1 1   ⍝ 3 2 1 1

⍝ bqn:windows:75 — Window sums with zero padding keep the length
(+⌿≢↕0 0,⊢) 2 6 0 1 4 3   ⍝ 2 8 8 7 5 8

⍝ bqn:windows:89 — Windows of two lengths are transposes of each other
{(5↕⍵)≡⍉3↕⍵}"abcdefg"   ⍝ $t

⍝ bqn:windows:99 — Windows along the first axis of a matrix
(⊂⍤2) 2↕["0123"⋄"abcd"⋄"ABCD"]
[["0123"⋄"abcd"] ["abcd"⋄"ABCD"]]

⍝ bqn:windows:103 — Windows along two axes of a matrix
(⊂⍤2) [2 2]↕["0123"⋄"abcd"⋄"ABCD"]
[["01"⋄"ab"] ["12"⋄"bc"] ["23"⋄"cd"]⋄["ab"⋄"AB"] ["bc"⋄"BC"] ["cd"⋄"CD"]]

⍝⍝ Swap

⍝ bqn:swap:22 — Select characters with a matrix of indices
[[0 1 1 0 0⋄1 0 1 0 1]]⌷" +"   ⍝ [" ++  "⋄"+ + +"]

⍝ bqn:swap:28 — An identity matrix from an outer product with the argument on both sides
=⊗⍨⍳3   ⍝ [$t $f $f⋄$f $t $f⋄$f $f $t]

⍝⍝ Constant

⍝ bqn:constant:17 — A constant in a fork
3 (+÷2⍨) 7   ⍝ 5

⍝ bqn:constant:21 — Under with an array operand replaces the selected element
∞ {⍺⌾(2⌷) ⍵} 1 2 3 4   ⍝ 1 2 ∞ 4

⍝⍝ Based arrays

⍝ bqn:based:23 — Numbers and characters are atoms with depth 0, and enclosing adds a level
[≡3;≡⊂3;≡'3';≡"abc";≡≡"abc"]   ⍝ [0 1 0 1 0]ₓ

⍝⍝ Scan

⍝ bqn:scan:69 — A seeded maximum scan
0⌈\¯1 ¯2 0 4 2 1 5 ¯2   ⍝ 0 0 0 4 4 4 5 5

⍝ bqn:scan:73 — Scan is a left fold over each prefix
{"(",⍺,")F",⍵}\"a" "b" "c" "d"
"a" "(a)Fb" "((a)Fb)Fc" "(((a)Fb)Fc)Fd"

⍝ bqn:scan:75 — A seed starts the fold
"w" {"(",⍺,")F",⍵}\ ["a" "b" "c" "d"]
"(w)Fa" "((w)Fa)Fb" "(((w)Fa)Fb)Fc" "((((w)Fa)Fb)Fc)Fd"

⍝ bqn:scan:80 — Scan calls its operand once for each item after the first
c←0
{c+←1 ⋄ ⍺+⍵}\⍳10
c   ⍝ 9

⍝ bqn:scan:91 — A Less Than scan
<\0 0 1 1 1 0 0 1 1 1 1   ⍝ 0 0 1 0 1 0 0 1 0 1 0

⍝ bqn:scan:105 — A scan from the end, through Reverse
{"(",⍺,")F",⍵}\⌾⌽"a" "b" "c" "d"
"(((d)Fc)Fb)Fa" "((d)Fc)Fb" "(d)Fc" "d"

⍝ bqn:scan:109 — A right fold over each suffix, swapping the arguments
{"(",⍺,")F",⍵}⍨\⌾⌽"a" "b" "c" "d"
"(a)F(b)F(c)Fd" "(b)F(c)Fd" "(c)Fd" "d"

⍝⍝ Rank

⍝ bqn:rank:92 — Join the strings in each row
s←["words" "go" "here"⋄"some" "other" "words"] ⋄ ⊂∘∊⍤1 s
"wordsgohere" "someotherwords"

⍝ bqn:rank:146 — Rank 2 on an array with empty cells keeps the frame and the cell shape
x←⊂⍤2⍳4 3 2 1 0 ⋄ [⍴x;⍴↑,x]   ⍝ (4ₓ 3ₓ 2ₓ⋄1ₓ 0ₓ)

⍝ bqn:rank:183 — A matrix product through Rank with an infinite rank for the right argument
m←[0 1 0⋄¯1 0 0⋄0 0 1] ⋄ m (+⌿∘×)⍤[1 ∞] 1 2 3×⊗1 10
[2 20⋄¯1 ¯10⋄3 30]

⍝ bqn:rank:193 — Rank 1 joins cells, with leading-axis agreement between the frames
⍴(⍳3 2 5),⍤1 ⍳3 4   ⍝ [3 2 9]ₓ

⍝ bqn:rank:195 — Frames that disagree on the leading axis
⍴(⍳2 3 5),⍤1 ⍳3 4
⍝ error: LENGTH ERROR

⍝⍝ Fold

⍝ bqn:fold:115 — A continued fraction for e
(+↢÷)/2 1 2 1 1 4 1 1   ⍝ 2.7183098591549295

⍝ bqn:fold:139 — A seeded fold starts from the right end
"STOP" (⌽↣,)/ ["ABCDE" "012" "abcd"]   ⍝ "EDCBA210dcbaSTOP"

⍝ bqn:fold:158 — Reducing an empty first axis gives one identity per cell
+⌿0 4⍴0   ⍝ 0 0 0 0

⍝⍝ Leading axis

⍝ bqn:leading:19 — A Left scan along the first axis copies the first major cell
a←["ab"⋄"cd"⋄"ef"] ⋄ ⊣⍀a   ⍝ ["ab"⋄"ab"⋄"ab"]

⍝ bqn:leading:90 — Rotate the first axis by 2 and the second by 1
2⊖1⌽⍳3 5
[[2 1] [2 2] [2 3] [2 4] [2 0]⋄[0 1] [0 2] [0 3] [0 4] [0 0]⋄[1 1] [1 2] [1 3] [1 4] [1 0]]

⍝ bqn:leading:94 — Drop applies to the leading axes
⍴3 2↓7 7 7 7⍴"abc"   ⍝ [4 5 7 7]ₓ

⍝ bqn:leading:110 — Leading-axis agreement pairs a matrix with a rank-3 array
x←3 2 4⍴⍳60
c←100×(⍳3)=⊗⍳2
c+x
⍝ =>
[[100 101 102 103⋄4 5 6 7]⋄[8 9 10 11⋄112 113 114 115]⋄[16 17 18 19⋄20 21 22 23]]

⍝⍝ Blocks

⍝ bqn:block:49 — A block can modify its argument
{⍵+←2 ⋄ [0 ⍵]}3   ⍝ 0 5

⍝ bqn:block:55 — An absent left argument makes the function that receives it monadic
[3{(2×⍺)-⍵}1;{(2×⍺)-⍵}1]   ⍝ 5 ¯1

⍝ bqn:block:60 — An absent left argument in a train leaves an atop
[3{(⍺-⌽)⍵}1 2;{(⍺-⌽)⍵}1 2]   ⍝ (1 2⋄¯2 ¯1)

⍝ bqn:block:66 — An absent left argument with Before
{⍺ *↣- ⍵}5   ⍝ 143.4131591025766

⍝ bqn:block:99 — Recursion through Choose
{⍵×(0<)⍚[1 ∇]⍵-1}7   ⍝ 5040

⍝ bqn:block:177 — Different functions for one and two arguments
ambiv←{[1 ⍵]}⊘{[2 ⍺ ⍵]} ⋄ [ambiv 'a';'a' ambiv 'b']
(1 'a'⋄2 'a' 'b')

⍝ bqn:block:231 — A predicate after a setup statement
{r←⌽⍵ ⋄ 't'=↑r? r;⍵}¨"test" "this"   ⍝ "tset" "this"

⍝ bqn:block:237 — Two predicates in one body
thing←{⍵≥3? ⍵≤8? 2|⍵; ⍵=0? '@'; ∞} ⋄ thing¨⍳10
'@' ∞ ∞ 1 0 1 0 1 0 ∞

⍝⍝ BQNcrate

⍝ bqncrate/table.tsv:695 — Ohm's law: resistance of parallel resistors
+/⌾÷ 2 3 6   ⍝ 1

⍝ bqncrate/table.tsv:866 — Length of a vector
+/⌾(×⍨) 3 4   ⍝ 5

⍝ bqncrate/table.tsv:922 — Harmonic mean
(≢×+/⌾÷) 1 2 4   ⍝ 1.7142857142857142

⍝ bqncrate/table.tsv:1050 — Geometric mean, using logarithm
(+/÷≢)⌾⍟ 1 2 4   ⍝ 2

⍝ bqncrate/table.tsv:1063 — Round to the nearest hundredth, ties towards ∞
{⌊0.5+⍵}⌾(100×) 3.14159 2.71828 ¯0.125   ⍝ 3.14 2.72 ¯0.12

⍝ bqncrate/table.tsv:930 — Number of derangements of length 5
1-/×\⌾⌽1+⍳5   ⍝ 44

⍝ bqncrate/table.tsv:976 — Mask that selects values between the first and last 1
(∨\∧∨\⌾⌽) 0 0 1 0 1 1 0 0   ⍝ 0 0 1 1 1 1 0 0

⍝ bqncrate/table.tsv:1098 — Remove trailing spaces
(∨\⌾⌽' '≠⊢)↣# "ab c  "   ⍝ "ab c"

⍝ bqncrate/table.tsv:1174 — Remove leading and trailing spaces
{(∨\∧∨\⌾⌽)' '≠⍵}↣# "  ab c  "   ⍝ "ab c"

⍝ bqncrate/table.tsv:713 — Ternary: apply F if a0, otherwise G
a←1 ⋄ (~a)⍚[- ⊢] 5   ⍝ ¯5

⍝ bqncrate/table.tsv:478 — Sequential AND, which skips G when F fails
{⍵>0}⍚[0 {⍵<10}]¨ 5 ¯1 20   ⍝ $t 0 $f

⍝ bqncrate/table.tsv:479 — Sequential OR, which skips G when F succeeds
{⍵<0}⍚[{⍵>10} 1]¨ ¯1 5 20   ⍝ 1 $f $t

⍝ bqncrate/table.tsv:415 — Replace the first major cell
[0 0 0]⌾↑ [1 2 3⋄4 5 6]   ⍝ [0 0 0⋄4 5 6]

⍝ bqncrate/table.tsv:572 — Reverse along every axis
⌽⌾, [1 2 3⋄4 5 6]   ⍝ [6 5 4⋄3 2 1]

⍝ bqncrate/table.tsv:738 — Apply a function to the last major cell
-⌾(¯1⌷) [1 2⋄3 4]   ⍝ [1 2⋄¯3 ¯4]

⍝ bqncrate/table.tsv:792 — Blend two lists by a mask
"abcd" ⊣⌾(1 0 1 0#) "wxyz"   ⍝ "axcz"

⍝ bqncrate/table.tsv:859 — Swap the first and last major cells
⌽⌾([[0 ¯1]]⌷) 1 2 3 4   ⍝ 4 2 3 1

⍝ bqncrate/table.tsv:1191 — Replace all spaces with dashes
{('-'⍨)¨⌾((' '=⍵)#)⍵} "a b c"   ⍝ "a-b-c"

