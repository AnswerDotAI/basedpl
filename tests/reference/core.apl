⍝⍝ Operand glyphs

⍝ — Pipeline stages use ordinary APL binding, then apply left to right
1+2×3 → 2∘× → -∘1   ⍝ 13

⍝ — Assignment encloses the whole pipeline
r←s←⍳4 → +/ → √ ⋄ r s   ⍝ (√10)(√10)

⍝ — Parentheses select pipeline boundaries
1+(⍳4 → +/)   ⍝ 11

⍝ — Pipe bodies still classify defined operators
op←{⍵→⍶→⍹} ⋄ (-op|)3   ⍝ 3

⍝ — A pipeline may supply a guard condition or result
{⍵→0∘<:⍵→⍲ ⋄ 0}3   ⍝ 9

⍝ — Stages see earlier effects and run exactly once
v←0 ⋄ 3 → {v+←1 ⋄ ⍵+v} → {v+←10 ⋄ ⍵+v}
15

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
∨3j4 5 0j¯2   ⍝ 3 2⍴3 4 5 0 0 ¯2

⍝ — Exact real parts stay exact
∨1r3   ⍝ 1r3 0x

⍝ — Magnitude and phase, in radians [rtol=1e-14]
∧3j4 0j1   ⍝ 2 2⍴5 0.9272952180016122 1 1.5707963267948966

⍝ — Empty decomposition retains frame and pair axis
⍴∨2 0⍴0x   ⍝ 2x 0x 2x

⍝ — Self-classification keeps classes in first-occurrence order
='aba'   ⍝ 2 3⍴1x 0x 1x 0x 1x 0x

⍝ — Self-classify major cells
=3 2⍴1 2 3 4 1 2   ⍝ 2 3⍴1x 0x 1x 0x 1x 0x

⍝ — A scalar has one class and one item
=7   ⍝ 1 1⍴1x

⍝ — No items, no classes
=⍬   ⍝ 0 0⍴0x

⍝ — Zero-width rows are equal
=3 0⍴0   ⍝ 1 3⍴1x

⍝ — Binary encode chooses enough first-axis digits
⊤2x 5x   ⍝ 3 2⍴0x 1x 1x 0x 0x 1x

⍝ — Binary decode uses the existing first digit axis
⊥3 2⍴0x 1x 1x 0x 0x 1x   ⍝ 2x 5x

⍝ — Binary zero needs no digits
⊤0x   ⍝ 0⍴0x

⍝ — Binary encoding works above machine-integer range
⊥⊤18446744073709551615x   ⍝ 18446744073709551615x

⍝ — Float binary encoding must not use tolerant floor
⊤9007199254740991   ⍝ 53⍴1

⍝ — Binary digits require integral values
⊤1.5
⍝ error: DOMAIN ERROR

⍝ — Binary encoding is unsigned
⊤¯1
⍝ error: DOMAIN ERROR

⍝ — Increment/decrement pervade and preserve exactness
≥≤1x (2x 3x)   ⍝ 1x (2x 3x)

⍝ — Increment promotes on overflow
≥9223372036854775807x   ⍝ 9223372036854775808x

⍝ — Square promotes on overflow
⍲3037000500x   ⍝ 9223372037000250000x

⍝ — Doubling preserves rational values
⍱1r3   ⍝ 2r3

⍝ — Square and double preserve empty exact prototypes
(⍲0⍴0x) (⍱0⍴0x)
(0⍴0x)(0⍴0x)

⍝ — Square root extends into complex numbers
√0 9 ¯4 3j4   ⍝ 0 3 0j2 2j1

⍝ — Exact perfect rational roots
√1r9 18446744073709551616x   ⍝ 1r3 4294967296x

⍝ — Irrational roots become approximate [rtol=1e-14]
√2x   ⍝ 1.4142135623730951

⍝ — Odd integral roots of negative reals use the real branch
3 ¯3√¯8   ⍝ ¯2 ¯0.5

⍝ — Negative exact degrees reciprocate
¯3x√¯8x   ⍝ ¯1r2

⍝ — Non-integral degree uses principal power
0.5√¯2   ⍝ 4

⍝ — Complex square root uses the principal branch
1E¯14>|(√¯3j¯4)-1j¯2   ⍝ 1x

⍝ — Zero degree is undefined
0√2
⍝ error: DOMAIN ERROR

⍝ — Zero cannot have a negative-degree root
¯3√0
⍝ error: DOMAIN ERROR

⍝ — Pi fractions and multiples [rtol=1e-14]
(π2)(1π2)(2π3)   ⍝ 6.283185307179586 1.5707963267948966 2.0943951023931953

⍝ — Unit-circle points [atol=1e-14]
○0 (1π2) (π1)   ⍝ 1 0j1 ¯1

⍝ — Complex angles use exp(i z) [rtol=1e-14]
○0j1   ⍝ 0.36787944117144233

⍝ — Principal inverses of root and circle [atol=1e-14]
((√⍣¯1)3)((○⍣¯1)○0.5)   ⍝ 9 0.5

⍝ — Bound pi fractions invert either argument [rtol=1e-14]
((1∘π)⍣¯1)1π4   ⍝ 4

⍝⍝ Function arrays

⍝ — Pick a function from a vector, then call it
fs←+˘×˘÷ ⋄ mul←2⊃fs ⋄ 2 mul 3   ⍝ 6

⍝ — Primitives, dfns and derived functions share one function vector
fs←(+/)˘{⍵×⍵}˘(3∘+) ⋄ square←2⊃fs ⋄ square 4
16

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
fs←+˘× ⋄ result←(⊂1 2 3),fs[2] ⋄ back←2⊃result ⋄ 2 back 3
6

⍝ — A pick path descends into a nested function vector
x←(+˘×)(-˘÷) ⋄ f←2⊃1⊃x ⋄ 2 f 3   ⍝ 6

⍝ — A dfn can return a selected function
fs←+˘× ⋄ f←{2⊃⍵}fs ⋄ 2 f 3   ⍝ 6

⍝ — Inverse enclose discloses a callable function
fs←+˘× ⋄ f←⊂⍣¯1⊢fs[⊂2] ⋄ 2 f 3   ⍝ 6

⍝ — Each assembles function results back into an array
fs←+˘× ⋄ f←↑↑¨fs ⋄ 2 f 3   ⍝ 5

⍝ — Rank assembles function results back into an array
fs←+˘× ⋄ f←↑↑⍤0⊢fs ⋄ 2 f 3   ⍝ 5

⍝ — An empty pick path preserves the scalar array, without disclosing its function
fs←+˘× ⋄ fs[⊂2]≡⍬⊃fs[⊂2]   ⍝ 1x

⍝ — A function vector matches itself by function identity
fs←+˘× ⋄ fs≡fs   ⍝ 1x

⍝ — A selected function can use its still-active lexical binding
{a←⍵ ⋄ fs←{a+⍵}˘× ⋄ f←↑fs ⋄ f 3}4   ⍝ 7

⍝ — Passing a function vector to another dfn retains lexical lookup
use←{f←↑⍵ ⋄ f 3} ⋄ {a←⍵ ⋄ use {a+⍵}˘+}4   ⍝ 7

⍝ — A helper may return a function while its defining frame remains active
id←{↑⍵} ⋄ {a←⍵ ⋄ f←id {a+⍵}˘+ ⋄ f 3}4   ⍝ 7

⍝ — Returning a top-level function needs no escaping local frame
fs←{⍵×⍵}˘+ ⋄ f←{↑⍵}fs ⋄ f 3   ⍝ 9

⍝ — Formatting a function vector produces character text
fs←+˘× ⋄ ⍴⍕fs   ⍝ ,6x

⍝ — One strand may contain numbers, callable functions and text
x←1˘+˘'abc' ⋄ f←2⊃x ⋄ (1⊃x)f≢3⊃x   ⍝ 4

⍝ — Functions have no grade ordering
⍋+˘×
⍝ error: DOMAIN ERROR

⍝ — Interval index requires ordered data, not functions
(+˘×)⍸+˘×
⍝ error: DOMAIN ERROR

⍝ — A returned function vector cannot retain an expired local frame
{x←⍵ ⋄ {x+⍵}˘+}2
⍝ error: DOMAIN ERROR

⍝ — Indexed assignment cannot export a local function into an outer array
fs←+˘× ⋄ {fs[1]←({⍵}˘+)[1] ⋄ 0}2
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
a←1 2 ⋄ b←3 4 ⋄ a˘b   ⍝ (1 2)(3 4)

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

⍝ — The selector and selected function both receive the original arguments
choose←{1+⍺>⍵}◶(-˘÷) ⋄ 12 choose 3   ⍝ 4

⍝ — An agenda branch may itself return a function
choose←1◶({↑⍵}˘⊢) ⋄ f←choose (+˘×) ⋄ 2 f 3
5

⍝ — A singleton vector is not a scalar selector
(,1)◶(+˘×)
⍝ error: RANK ERROR

⍝ — The cases must be a vector, not a function-containing scalar
1◶((+˘×)[⊂1])
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
(2 1⍴1 2)<1 3⍴1 2 3   ⍝ 2 3⍴0x 1x 1x 0x 0x 1x

⍝ —
(2 1⍴1x 2x)<1 3⍴1x 2x 3x   ⍝ 2 3⍴0x 1x 1x 0x 0x 1x

⍝ — Rank pairs vector cells using broadcast frames
(2 1 2⍴1 2 3 4)(+⍤1)1 3 2⍴10 20 30 40 50 60
2 3 2⍴11 22 31 42 51 62 13 24 33 44 53 64

⍝ — Unit axes extend independently across three dimensions
(2 1 2⍴1 2 3 4)+1 3 1⍴10 20 30
2 3 2⍴11 12 21 22 31 32 13 14 23 24 33 34

⍝ — An explicit axis aligns the vector with columns rather than rows
(2 3⍴⍳6)+[2]10 20 30   ⍝ 2 3⍴11 22 33 14 25 36

⍝ — Axis permutation and unit-axis extension work together
(2 1⍴10 20)+[2 1]3 1⍴1 2 3   ⍝ 2 3⍴11 12 13 21 22 23

⍝ — Broadcasting recurs inside nested arrays
(⊂2 1⍴1 2)+⊂1 3⍴10 20 30   ⍝ ⊂2 3⍴11 21 31 12 22 32

⍝ — Broadcast integer overflow promotes to exact big numbers
(1 2⍴9223372036854775807x 1x)+1x 2x
2 2⍴9223372036854775808x 2x 9223372036854775809x 3x

⍝ — Rank-zero application extends a singleton frame
(⍳1)(+⍤0)⍳3   ⍝ 2 3 4

⍝ — A vector frame aligns with the leading matrix axis
1 2 (+⍤0)2 3⍴0   ⍝ 2 3⍴1 1 1 2 2 2

⍝⍝ Selective assignment

⍝ — Selection through identity and disclose replaces the whole nested item
a←(1 2)(3 4) ⋄ (⊢↑a)←7 8 9 ⋄ a   ⍝ (7 8 9)(3 4)

⍝ — Identity selection may replace the whole array with a different shape
a←1 2 ⋄ (⊣a)←3 4 5 ⋄ a   ⍝ 3 4 5

⍝ — An empty pick path selects the whole scalar for replacement
a←1 ⋄ (⍬⊃a)←3 4 ⋄ a   ⍝ 3 4

⍝ — Whole-array replacement also works when the old array is empty
a←⍬ ⋄ (⍬⊃a)←3 4 ⋄ a   ⍝ 3 4

⍝ — Modified assignment through an empty pick path can resize the array
a←1 2 ⋄ (⍬⊃a),←3 4 ⋄ a   ⍝ 1 2 3 4

⍝ — Whole-item replacement composes with a nested pick
a←(1 2)(3 4) ⋄ (⍬⊃1⊃a)←5 6 7 ⋄ a   ⍝ (5 6 7)(3 4)

⍝ — Assignment through reverse maps replacements back to original positions
a←1 2 ⋄ (⍬⊃⌽a)←3 4 ⋄ a   ⍝ 4 3

⍝ — Ravel selection cannot resize its source through an empty pick path
a←1 2 ⋄ (⍬⊃,a)←3 4 5
⍝ error: LENGTH ERROR

⍝ — Squad selection still requires replacement-shape agreement
a←1 2 ⋄ (⍬⊃⍬⌷a)←2 2⍴3 4
⍝ error: LENGTH ERROR

⍝ — Replace vowels selected by a Boolean mask
a←'HELLO' ⋄ ((a∊'AEIOU')/a)←'*' ⋄ a   ⍝ 'H*LL*'

⍝ — Assign through a ravel prefix without changing matrix shape
z←3 4⍴⍳12 ⋄ (5↑,z)←0 ⋄ ,z   ⍝ 0 0 0 0 0 6 7 8 9 10 11 12

⍝ — Repeated transpose axes select the diagonal
m←3 3⍴⍳9 ⋄ (1 1⍉m)←0 ⋄ ,m   ⍝ 0 2 3 4 0 6 7 8 0

⍝ — Selection through enlist updates characters inside nested vectors
a←'Andy' 'Karen' 'Liam' ⋄ (('a'=∊a)/∊a)←'*' ⋄ a
'Andy' 'K*ren' 'Li*m'

⍝ — Each selects a prefix in every nested vector
a←'HELLO' 'WORLD' ⋄ (2↑¨a)←'*' ⋄ a   ⍝ '**LLO' '**RLD'

⍝ — Each uses a separate mask for each nested vector
a←'HELLO' 'WORLD' ⋄ ((a='O')/¨a)←'*' ⋄ a
'HELL*' 'W*RLD'

⍝ — Replacing one nested item may change its length
a←(1 2)(3 4) ⋄ (1↑a)←⊂8 9 10 ⋄ a   ⍝ (8 9 10)(3 4)

⍝ — Repeated selection updates the same source element more than once
a←3⍴0x ⋄ (5⍴a)+←1x ⋄ a   ⍝ 2x 2x 1x

⍝ — Reverse composes with indexed selection
a←1 2 3 ⋄ (⌽a[1 3])←8 9 ⋄ a   ⍝ 9 2 8

⍝ — An empty selection leaves the source unchanged
a←1 2 3 ⋄ (0↑a)←9 ⋄ a   ⍝ 1 2 3

⍝ — Empty each-selection preserves the nested prototype shape
a←0⍴⊂2 3⍴⍳6 ⋄ (⌽[2]¨a)←9 ⋄ ⍴↑a   ⍝ 2x 3x

⍝ — Overtake's fill positions do not create new source elements
a←1 2 ⋄ (3↑a)←4 ⋄ a   ⍝ 4 4

⍝ —
a←(1 2)(3 4) ⋄ (↑a)←7 8 9 ⋄ 1⊃a   ⍝ 7 8 9

⍝ — First selection replaces an atom with a vector
a←1 ⋄ (↑a)←3 4 ⋄ a   ⍝ 3 4

⍝ — Disclose-each selects the first element of each nested vector
a←(1 2)(3 4) ⋄ (↑¨a)←(5 6)(7 8) ⋄ a   ⍝ ((5 6)2)((7 8)4)

⍝ — A bound take function remains assignment-selective
a←1 2 3 ⋄ ((1∘↑)a)←9 ⋄ a   ⍝ 9 2 3

⍝ — A bound pick function can replace an item with a nested vector
a←1 2 ⋄ ((1∘⊃)a)←3 4 ⋄ a   ⍝ (3 4)2

⍝ — Binding an empty pick path retains whole-array replacement
a←1 2 ⋄ ((⍬∘⊃)a)←3 4 5 ⋄ a   ⍝ 3 4 5

⍝ — Bound pick under each updates the selected nested elements
a←(1 2)(3 4) ⋄ ((1∘⊃)¨a)←(5 6)(7 8) ⋄ a   ⍝ ((5 6)2)((7 8)4)

⍝ — An empty array has no first item to replace
a←⍬ ⋄ (↑a)←3 4
⍝ error: INDEX ERROR

⍝ — The selection expression executes only once
a←1 2 ⋄ ((⎕←1)/a)←3 ⋄ a
3 3
⍝ ⎕: 1

⍝⍝ Indexed modified and strand assignment

⍝ — Modified assignment updates an outer lexical binding
a←10 ⋄ f←{a+←⍵ ⋄ a} ⋄ z←f 3 ⋄ z,a   ⍝ 13 13

⍝ — A local array can shadow an outer operator
o←¨ ⋄ {o←3 ⋄ o}0   ⍝ 3

⍝ — Strand assignment creates local names even when an outer name is an operator
o←¨ ⋄ {a o←3 4 ⋄ a o}0   ⍝ 3 4

⍝ — A local function can shadow an outer operator
o←¨ ⋄ {o←{⍵} ⋄ o 3}0   ⍝ 3

⍝ — Modified assignment finds the nearest lexical binding
a←10 ⋄ f←{a←2 ⋄ g←{a+←⍵ ⋄ a} ⋄ z←g ⍵ ⋄ z,a} ⋄ z←f 3 ⋄ z,a
5 5 10

⍝ — The caller's local name does not redirect a callee's lexical assignment
a←10 ⋄ g←{a+←⍵ ⋄ a} ⋄ f←{a←2 ⋄ z←g ⍵ ⋄ z,a} ⋄ z←f 3 ⋄ z,a
13 2 13

⍝ — Updating an outer array leaves a previously assigned copy unchanged
a←1 2 ⋄ b←a ⋄ f←{a[1]←⍵ ⋄ a} ⋄ z←f 3 ⋄ z,a,b
3 2 3 2 1 2

⍝ — Selective modified assignment can update an outer array
a←1 2 ⋄ f←{(⌽a)+←⍵ ⋄ a} ⋄ z←f 3 ⋄ z,a   ⍝ 4 5 4 5

⍝ — A local error guard does not roll back writes to an outer binding
a←10 ⋄ f←{0::a ⋄ a+←⍵ ⋄ 1÷0} ⋄ z←f 3 ⋄ z,a
13 13

⍝ — A guard restores local bindings to their installation state
{a←2 ⋄ 0::a ⋄ a+←3 ⋄ 1÷0}0   ⍝ 2

⍝ — Brackets bind to the complete strand on their left
a←9 10 11 ⋄ 1 2 a[2] 3 4 5 6[3]   ⍝ 4

⍝ — Each successive bracket selects from the strand built so far
'a' 2[1] 2[1] 2[1]   ⍝ 'a'

⍝ —
1+a←3   ⍝ 4

⍝ — Applying a function to an assignment result makes it non-shy
{1+a←3 ⋄ 9}0   ⍝ 4

⍝ — Modified assignment yields its right argument, not the updated binding
{a←1 ⋄ +a+←3 ⋄ 9}0   ⍝ 3

⍝ —
a←1+b←2 ⋄ a b   ⍝ 3 2

⍝ — Indexed modified assignment yields the supplied increment
a←1 2 3 ⋄ 2×a[2]+←10   ⍝ 20

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
a←1 2 3 ⋄ a[2]←9 ⋄ a   ⍝ 1 9 3

⍝ — Repeated indices accumulate rather than overwriting from the original value
a←1 2 3 ⋄ r←a[2 2]+←10 20 ⋄ a   ⍝ 1 32 3

⍝ — The result of modified assignment is the original right argument
a←1 2 3 ⋄ r←a[2 2]+←10 20 ⋄ r   ⍝ 10 20

⍝ — Repeated row and column indices multiply the number of updates
a←3 5⍴0 ⋄ a[1 1 3;1 3 3 5]+←1 ⋄ ,a   ⍝ 2 0 4 0 2 0 0 0 0 0 1 0 2 0 1

⍝ — Indexed assignment preserves value semantics for an earlier copy
a←1 2 3 ⋄ b←a ⋄ a[1 3]←8 9 ⋄ b   ⍝ 1 2 3

⍝ —
a←1x 2x ⋄ a×←2x ⋄ a   ⍝ 2x 4x

⍝ — Modified integer assignment promotes on overflow
a←1x ⋄ a+←9223372036854775807x ⋄ a   ⍝ 9223372036854775808x

⍝ — Right-to-left evaluation reads the strand's a before the function updates it
a←1 ⋄ f←{a+←1 ⋄ a} ⋄ (f 0) a   ⍝ 2 1

⍝ —
(a (b c))←1 (2 3) ⋄ a b c   ⍝ 1 2 3

⍝ —
a b←1 ⋄ a b   ⍝ 1 1

⍝ — Successive bracket selections map back to the original array
a←1 2 3 ⋄ a[3 1][2]←9 ⋄ a   ⍝ 9 2 3

⍝ — A nested bracket path updates an element inside the second item
a←(1 2)(3 4) ⋄ a[⊂(,2)(,1)]←9 ⋄ ↑2⊃a   ⍝ 9

⍝ — Strand modification calls the operand from left to right
a←1 ⋄ b←2 ⋄ a b{⎕←⍺ ⋄ ⍺+⍵}←3 4 ⋄ a b
4 6
⍝ ⎕: 1\n2

⍝ — The right-hand assignment runs before the parenthesized read
a←0 ⋄ (⎕←a)+a←⎕←3
6
⍝ ⎕: 3\n3

⍝ — A modifying dfn may read the array it is updating
a←1 2 ⋄ f←{⎕←a ⋄ ⍺+⍵} ⋄ a[1 2]f←10 20 ⋄ a
11 22

⍝⍝ General axis forms

⍝ — Ravel merges adjacent selected axes. Dyalog 20.0.53963.0, IO=1, CT=1e-14, ML=1.
⍴,[2 3]2 3 4⍴⍳24   ⍝ 2x 12x

⍝ — Fractional-axis ravel inserts a unit axis between existing axes
⍴,[1.5]2 3⍴⍳6   ⍝ 2x 1x 3x

⍝ — Empty-axis ravel appends a unit axis
⍴,[⍬]2 3⍴⍳6   ⍝ 2x 3x 1x

⍝ — Mix places item axes before the outer vector axis
⊃[0.5](1 2)(3 4)   ⍝ 2 2⍴1 3 2 4

⍝ — Mix places the matrix-cell axes at positions one and three
⊃[1 3](2 3⍴⍳6)(2 3⍴6+⍳6)   ⍝ 2 2 3⍴1 2 3 7 8 9 4 5 6 10 11 12

⍝ — Laminate inserts a leading axis
1 2,[0.5]3 4   ⍝ 2 2⍴1 2 3 4

⍝ — Laminate extends a scalar along the vector's existing axis
1,[1.5]2 3   ⍝ 2 2⍴1 2 1 3

⍝ — Scalar-function axis one aligns the vector with rows
1 2+[1]2 3⍴⍳6   ⍝ 2 3⍴2 3 4 6 7 8

⍝ — Take counts follow the specified axis order
2 1↑[2 1]3 4⍴⍳12   ⍝ 1 2⍴1 2

⍝ — Drop counts follow the specified axis order
1 1↓[2 1]3 4⍴⍳12   ⍝ 2 3⍴6 7 8 10 11 12

⍝ — Squad's index vectors correspond to the listed axes
(2 1)(1 2)⌷[2 1]2 3⍴⍳6   ⍝ 2 2⍴2 1 5 4

⍝ — Enclosing all axes in reverse order transposes the enclosed cell
↑⊂[2 1]2 3⍴⍳6   ⍝ 3 2⍴1 4 2 5 3 6

⍝ — Axis enclosure leaves the unselected frame, including its zero dimension
⍴⊂[3]2 0 4⍴0x   ⍝ 2x 0x

⍝ — Empty axis-enclosure retains the length-four cell prototype
⍴↑↑⊂[3]2 0 4⍴0x   ⍝ ,4x

⍝⍝ Format and execute

⍝ — Decimal formatting rounds halfway cases away from zero
2⍕3.125 ¯3.125 2.675 ¯2.675   ⍝ ' 3.13 ¯3.13 2.68 ¯2.68'

⍝ — Integer formatting also rounds halves away from zero
0⍕2.5 ¯2.5 1.5 ¯1.5   ⍝ ' 3 ¯3 2 ¯2'

⍝ — Rounded zero loses its minus sign but retains alignment space
2⍕¯0.001 0.001   ⍝ '  0.00 0.00'

⍝ — A negative precision selects significant digits in exponential notation
¯2⍕3.25 ¯3.25 325 0.0325   ⍝ ' 3.3E0 ¯3.3E0 3.3E2 3.3E¯2'

⍝ — Matrix formatting aligns exponential fields by column
¯2⍕2 2⍴3.125 0.002 1000 20   ⍝ ⊃' 3.1E0 2.0E¯3' ' 1.0E3 2.0E1 '

⍝ —
⍕¯1E¯100j¯2E¯99   ⍝ '¯1E¯100j¯2E¯99'

⍝ — Default formatting switches to exponents beyond these magnitude thresholds
⍕1E¯6 1E¯7 1E16 1E17   ⍝ '0.000001 1E¯7 10000000000000000 1E17'

⍝ — Default format and execute round-trip subnormal, large and complex values
⍎⍕5E¯324 ¯1.2345678901234567E200 1E¯100j2E100
5E¯324 ¯1.2345678901234567E200 1E¯100j2E100

⍝ — Formatting a scalar produces a character vector
⍴⍕1   ⍝ ,1x

⍝ — A zero-row matrix retains column widths and separators
⍴⍕0 3⍴0   ⍝ 0x 5x

⍝ — A matrix with no columns formats to zero-width rows
⍴⍕3 0⍴0   ⍝ 3x 0x

⍝ — Fixed-width formatting expands only the trailing dimension
⍴5 2⍕2 3 4⍴⍳24   ⍝ 2x 3x 20x

⍝ — Per-column formats replace overflowing fields with stars
,3 0 6 2⍕3 2⍴10.1 15 1001 22.357 101 1110.1
' 10 15.00*** 22.36101******'

⍝ — Default matrix formatting aligns decimal points within each column
,⍕2 2⍴1 12.3 123 4   ⍝ '  1 12.3123  4  '

⍝ — Format and execute preserve exact integer and rational domains
⍎⍕1x 1r3   ⍝ 1x 1r3

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
⍕12.34   ⍝ '12.34'

⍝ —
⍕1x 1r3   ⍝ '1x 1r3'

⍝ —
⍕⍬   ⍝ ''

⍝ —
4 1⍕1.1 2 ¯4 2.547   ⍝ ' 1.1 2.0¯4.0 2.5'

⍝ — Fixed-width fields retain a trailing space when the exponent is short
7 ¯3⍕5 15 155 1555   ⍝ '5.00E0 1.50E1 1.55E2 1.56E3 '

⍝ —
0 2⍕1 2   ⍝ ' 1.00 2.00'

⍝ —
0 2⍕1r3 2r3   ⍝ ' 0.33 0.67'

⍝ — Exact big integers format without conversion through float
0 0⍕9223372036854775808x   ⍝ ' 9223372036854775808'

⍝ — Requested digits beyond float precision are marked with underscores
0 20⍕÷3   ⍝ ' 0.3333333333333333____'

⍝ — Execute sees the active local binding without changing the global
a←4 ⋄ f←{a←10 ⋄ ⍎'a+⍵'} ⋄ b←f 3 ⋄ b a   ⍝ 13 4

⍝ — Dyadic execute with an empty left argument uses the current scope
a←4 ⋄ ''⍎'a+1'   ⍝ 5

⍝ — Execute can return a primitive function
g←⍎'+' ⋄ 2 g 3   ⍝ 5

⍝ — A final assignment in execute can also return a function
g←⍎'h←+' ⋄ 2 g 3   ⍝ 5

⍝ — A dfn can return the function produced by execute
g←{⍎'+'}0 ⋄ 2 g 3   ⍝ 5

⍝ — Execute preserves the hybrid category
r←⍎'/' ⋄ +r 1 2 3   ⍝ 6

⍝ — Execute can return a defined operator
op←⍎'{⍶ ⍵}' ⋄ -op 3   ⍝ ¯3

⍝ — Empty execute produces no value
⍎''   ⍝ {}0

⍝ — A missing execute result cannot be assigned
a←⍎''
⍝ error: VALUE ERROR

⍝⍝ Polynomial representations and derivatives

⍝ — Polynomial coefficients are constant-first
1x 2x 3xⓅ0x 1x 2x   ⍝ 1x 6x 17x

⍝ — Evaluating at an array of points preserves its shape
1x 2x 3xⓅ2 2⍴0x 1x 2x 3x   ⍝ 2 2⍴1x 6x 17x 34x

⍝ — Evaluate the factored form 2(x-1)(x-3)
(2x (1x 3x))Ⓟ0x 1x 2x 3x   ⍝ 6x 0x ¯2x 0x

⍝ — Convert multiplier and roots to constant-first coefficients
Ⓟ2x (1x 3x)   ⍝ 6x ¯8x 2x

⍝ — Enclosed roots imply a leading coefficient of one
Ⓟ⊂1x 3x   ⍝ 3x ¯4x 1x

⍝ — Convert an exponent table for x⁵-1, filling missing degrees with zero
Ⓟ⊂2 2⍴1x 5x ¯1x 0x   ⍝ ¯1x 0x 0x 0x 0x 1x

⍝ — Fractional exponents evaluate numerically even with exact input
(⊂2 2⍴2x 1r2 3x 1r4)Ⓟ16x   ⍝ 14

⍝ — Evaluate a two-variable exponent table at an enclosed coordinate vector
(⊂4 3⍴¯1 2 1 1 1 1 2 1 2 3 0 2)Ⓟ⊂2.5 ¯1   ⍝ 11.75

⍝ — Empty exact evaluation points retain an exact prototype
1x 2xⓅ0⍴0x   ⍝ 0⍴0x

⍝ — Polynomial rows pair with scalar evaluation points by frame
(2 3⍴1x 2x 3x 4x 5x 6x)Ⓟ1x 2x   ⍝ 6x 38x

⍝ —
5xⓅ2x   ⍝ 5x

⍝ —
0x 0xⓅ2x   ⍝ 0x

⍝ — A multiplier with no roots is a constant polynomial
Ⓟ2x (0⍴0x)   ⍝ ,2x

⍝ — Differentiate the bound polynomial evaluator
f←1x 2x 3x∘Ⓟ ⋄ f∂2x   ⍝ 14x

⍝ — Repeated differentiation gives the second derivative
f←1x 2x 3x∘Ⓟ ⋄ f∂∂2x   ⍝ 6x

⍝ — Differentiating beyond the polynomial's degree gives exact zero
f←1x 2x 3x∘Ⓟ ⋄ f∂∂∂2x   ⍝ 0x

⍝ — Dyadic derivative weights each output derivative by its cotangent
f←1x 2x 3x∘Ⓟ ⋄ 10x 20x(f∂)1x 2x   ⍝ 80x 280x

⍝ — Differentiate directly from the factored representation
f←(2x (1x 3x))∘Ⓟ ⋄ f∂2x   ⍝ 0x

⍝ — A multivariate gradient retains the coordinate enclosure
f←(⊂2 3⍴1x 2x 0x 1x 0x 2x)∘Ⓟ ⋄ f∂⊂3x 4x   ⍝ ⊂6x 8x

⍝ — A scalar cotangent scales the multivariate gradient
f←(⊂2 3⍴1x 2x 0x 1x 0x 2x)∘Ⓟ ⋄ 2x(f∂)⊂3x 4x
⊂12x 16x

⍝ — A shared scalar coordinate sums the partial derivatives
f←(⊂2 3⍴1x 2x 0x 1x 0x 2x)∘Ⓟ ⋄ f∂3x   ⍝ 12x

⍝ — Multiple polynomial outputs contribute to one scalar-input VJP
f←(2 2⍴1x 2x 3x 4x)∘Ⓟ ⋄ 10x 20x(f∂)3x   ⍝ 100x

⍝ — Negative exponents cannot be converted to a coefficient vector
Ⓟ⊂1 2⍴1 ¯1
⍝ error: DOMAIN ERROR

⍝ — The cotangent must match the scalar output's structure
(⊂1 2)(1 2∘Ⓟ∂)3
⍝ error: DOMAIN ERROR

⍝ —
Ⓟ1 ∞
⍝ error: DOMAIN ERROR

⍝ — Monadic derivative requires scalar output, not a vector of evaluations
1 2∘Ⓟ∂1 2
⍝ error: RANK ERROR

⍝ — VJP requires one cotangent for each output
1(1 2∘Ⓟ∂)1 2
⍝ error: LENGTH ERROR

⍝ — The coordinate count must match the exponent table's variables
(⊂1 3⍴1 2 3)Ⓟ⊂1 2 3
⍝ error: LENGTH ERROR

⍝⍝ Prime and factor families

⍝ — Prime indexing is one-based and returns exact integers
ℙ⍳8   ⍝ 2x 3x 5x 7x 11x 13x 17x 19x

⍝ — Prime lookup preserves shape and accepts repeated, unordered indices
ℙ2 2⍴10000 1 10000 2   ⍝ 2 2⍴104729x 2x 104729x 3x

⍝ —
n←5 ⋄ ℙn   ⍝ 11x

⍝ — Previous prime is strictly below the argument
¯4ℙ3 4 5 6   ⍝ 2x 3x 3x 5x

⍝ — Count primes strictly below each argument
¯1ℙ1 2 3 4 5 6   ⍝ 0x 0x 1x 2x 2x 3x

⍝ — Non-prime includes negative integers, zero and one
0ℙ¯1 0 1 2 3 4   ⍝ 1x 1x 1x 0x 0x 1x

⍝ — Test primality rather than indexing the prime sequence
1ℙ¯1 0 1 2 3 4   ⍝ 0x 0x 0x 1x 1x 0x

⍝ — Distinct factors occupy the first row, their exponents the second
2ℙ700   ⍝ 2 3⍴2x 5x 7x 2x 2x 1x

⍝ — Factor-list mode includes repeated prime factors
3ℙ700   ⍝ 2x 2x 5x 5x 7x

⍝ — Next prime is strictly above the argument
4ℙ1 2 3 4 5   ⍝ 2x 3x 5x 5x 7x

⍝ — Euler's totient counts positive integers up to n coprime to n
5ℙ1 2 3 4 5 6 10   ⍝ 1x 1x 2x 2x 4x 2x 4x

⍝ —
Ⓠ700   ⍝ 2x 2x 5x 5x 7x

⍝ — One has an empty exact factorization
Ⓠ1   ⍝ 0⍴0x

⍝ — Return exponents of only the first two primes
2Ⓠ700   ⍝ 2x 0x

⍝ — Requested primes beyond the largest factor receive zero exponents
10Ⓠ700   ⍝ 2x 0x 2x 1x 0x 0x 0x 0x 0x 0x

⍝ — Infinite count includes every prime through the largest factor
∞Ⓠ700   ⍝ 2x 0x 2x 1x

⍝ — Negative count selects the last distinct factors and their exponents
¯2Ⓠ700   ⍝ 2 2⍴5x 7x 2x 1x

⍝ — Negative infinity requests the complete factor/exponent table
¯∞Ⓠ700   ⍝ 2 3⍴2x 5x 7x 2x 2x 1x

⍝ —
0Ⓠ700   ⍝ 0⍴0x

⍝ — Factoring one retains the table's two-row shape
¯∞Ⓠ1   ⍝ 2 0⍴0x

⍝ — Inverse prime lookup recovers one-based indices
ℙ⍣¯1⊢ℙ⍳10   ⍝ ⍳10x

⍝ — Inverse factorization multiplies the factors
Ⓠ⍣¯1⊢Ⓠ700   ⍝ 700x

⍝ — Primality accepts exact integers at and beyond the unsigned 64-bit boundary
1ℙ18446744073709551557x 18446744073709551615x 170141183460469231731687303715884105727x
1x 0x 1x

⍝ — Strong pseudoprimes must not pass as primes
1ℙ341550071728321x 3825123056546413051x   ⍝ 0x 0x

⍝ — Factor a semiprime whose two factors are both large
Ⓠ1000000016000000063x   ⍝ 1000000007x 1000000009x

⍝ — Empty prime lookup preserves shape with an exact prototype
ℙ0 2⍴0   ⍝ 0 2⍴0x

⍝ — Factor lists of unequal lengths assemble with zero fill
Ⓠ2 12   ⍝ 2 3⍴2x 0x 0x 2x 2x 3x

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
Ⓠ0
⍝ error: DOMAIN ERROR

⍝ —
Ⓠ¯1
⍝ error: DOMAIN ERROR

⍝ —
Ⓠ2j1
⍝ error: DOMAIN ERROR

⍝ —
6ℙ7
⍝ error: DOMAIN ERROR

⍝ — There is no prime strictly below two
¯4ℙ2
⍝ error: DOMAIN ERROR

⍝⍝ Full windows

⍝ — Full vector windows overlap without padding
3↕⍳5   ⍝ 3 3⍴1 2 3 2 3 4 3 4 5

⍝ — Window-position axes precede the two-dimensional window axes
2 2↕2 3⍴⍳6   ⍝ 1 2 2 2⍴1 2 4 5 2 3 5 6

⍝ — One window length slides along the leading axis, retaining whole rows
2↕3 2⍴⍳6   ⍝ 2 2 2⍴1 2 3 4 3 4 5 6

⍝ — An oversized window produces no windows but retains window shape and character fill
4↕'ab'   ⍝ 0 4⍴' '

⍝ — Zero-length windows occur at every boundary, including both ends
0↕'ab'   ⍝ 3 0⍴' '

⍝ — No window axes leaves the argument unchanged
⍬↕2 3⍴⍳6   ⍝ 2 3⍴⍳6

⍝ — Zero-length and nonempty window axes combine without losing the frame
0 2↕2 3⍴⍳6   ⍝ 3 2 0 2⍴0

⍝ —
¯1↕⍳3
⍝ error: DOMAIN ERROR

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
(1 1⍴2)↕⍳3
⍝ error: RANK ERROR

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
(1∘+)⍣(2 2⍴3 ¯2 0 3)⊢10   ⍝ 2 2⍴13 8 10 13

⍝ — A singleton count vector adds a singleton result frame
(+⍣(,1))2   ⍝ ,2

⍝ — Different-sized iterates assemble into padded rows
{⍵,1}⍣0 1 2⊢,2   ⍝ 3 3⍴2 0 0 2 1 0 2 1 1

⍝ — Empty counts never invoke the operand and retain count and argument shapes
{1÷0}⍣(0 2⍴0)⊢'ab'   ⍝ 0 2 2⍴' '

⍝ — Iteration history includes the starting value
(1∘+)⍣\3⊢5   ⍝ 5 6 7 8

⍝ — A negative history count follows the inverse
(1∘+)⍣ \ ¯2⊢5   ⍝ 5 4 3

⍝ — History pads growing iterates into rows
{⍵,1}⍣\2⊢,2   ⍝ 3 3⍴2 0 0 2 1 0 2 1 1

⍝ — Predicate history includes the iterate that satisfies the stopping test
1 +⍣\{⍺=3}0   ⍝ 0 1 2 3

⍝ — Fixed-point history includes both the initial and unchanged next value
⊢⍣\≡⊢4   ⍝ 4 4

⍝ — Zero-step history retains the initial value without calling the operand
{1÷0}⍣\0⊢'ab'   ⍝ 1 2⍴'ab'

⍝ — Parenthesizing power leaves the following backslash as ordinary scan
(+⍣1)\1 2 3   ⍝ 1 3 6

⍝ — A declared right-argument inverse does not provide a left-argument inverse
((+⇄-)∘3)⍣¯1⊢8
⍝ error: DOMAIN ERROR

⍝ —
+⍣0 0.5⊢1
⍝ error: DOMAIN ERROR

⍝ —
+⍣\1 2⊢3
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
W←×∘*⍨⍣¯1 ⋄ W (0x 0x)(0⍴0x)   ⍝ (0 0)⍬

⍝ — Check W(x)exp(W(x))=x across tiny and large real inputs
W←×∘*⍨⍣¯1 ⋄ x←1E¯100 ¯1E¯100 1E¯12 ¯1E¯12 0.099 ¯0.099 1E300 ⋄ ∧/1E¯12>|1-(W x)×(*W x)÷x
1x

⍝ — Check the Lambert W inverse identity on complex inputs
W←×∘*⍨⍣¯1 ⋄ x←¯1j0.1 1j¯1 ⋄ ∧/1E¯12>|1-(W x)×(*W x)÷x
1x

⍝ — A real Lambert W argument below -1/e is rejected
(×∘*⍨⍣¯1)¯1
⍝ error: DOMAIN ERROR

⍝ —
(×∘*⍨⍣¯1)'a'
⍝ error: DOMAIN ERROR

⍝ — Invert an outer subtraction with its right vector fixed
( -⌝ ∘4 5)⍣¯1⊢2 2⍴¯3 ¯4 ¯2 ¯3   ⍝ 1 2

⍝ — Invert an outer subtraction with its left vector fixed
(4 5∘( -⌝ ))⍣¯1⊢2 2⍴3 2 4 3   ⍝ 1 2

⍝ — Outer-product inversion preserves exact rational results
( ×⌝ ∘4x 5x)⍣¯1⊢2 2⍴2x 5r2 1x 5r4   ⍝ 1r2 1r4

⍝ — A fixed matrix occupies the trailing axes of the outer-product result
( ×⌝ ∘(2 2⍴1 2 3 4))⍣¯1⊢3 2 2⍴1 2 3 4 2 4 6 8 3 6 9 12
1 2 3

⍝ — Outer inversion with a fixed scalar recovers a vector
(4∘( ×⌝ ))⍣¯1⊢4 8   ⍝ 1 2

⍝ — Removing the fixed vector's axes can leave a scalar
( ×⌝ ∘4 5)⍣¯1⊢4 5   ⍝ ⊂1

⍝ — Outer inversion recovers an empty argument from an empty result frame
( +⌝ ∘4 5)⍣¯1⊢0 2⍴0   ⍝ ⍬

⍝ — With the left vector fixed, the remaining axes describe the recovered matrix
(4 5∘( -⌝ ))⍣¯1⊢2 2 2⍴3 2 1 0 4 3 2 1   ⍝ 2 2⍴1 2 3 4

⍝ — Outer inversion preserves zero dimensions in the recovered frame
( +⌝ ∘4 5)⍣¯1⊢3 0 2⍴0   ⍝ 3 0⍴0

⍝ — Every outer-product cell must imply the same recovered value
( ×⌝ ∘4 5)⍣¯1⊢2 2⍴4 5 8 11
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
⍳⍣¯1⊢1 2 3   ⍝ ,3x

⍝ — Inverse iota also recognizes multidimensional index arrays
⍳⍣¯1⊢⍳2 3   ⍝ 2x 3x

⍝ — Each positive circle code's inverse agrees with its negative-code counterpart here
{(5○⍨-⍵)=⍵∘○⍣¯1⊢5}⍳12   ⍝ 12⍴1x

⍝ —
(1∘+⍣¯3)10   ⍝ 7

⍝ — Invert a Celsius-to-Fahrenheit composition in reverse function order
((32∘+)∘(×∘1.8)⍣¯1)32 212   ⍝ 0 100

⍝ — Inverse sum scan recovers successive differences
(+\⍣¯1)1 3 6 10   ⍝ 1 2 3 4

⍝ — Inverse scan retains the exact numeric domain
(+\⍣¯1)1x 3x 6x   ⍝ 1x 2x 3x

⍝ — Inverse Boolean xor scan recovers changes between adjacent values
(≠\⍣¯1)1x 1x 0x 0x   ⍝ 1x 0x 1x 0x

⍝ — Inverse base-two decode chooses the required number of digits
(2∘⊥⍣¯1)9   ⍝ 1 0 0 1

⍝ — Exact base and value produce exact inverse-decode digits
(2x∘⊥⍣¯1)9x   ⍝ 1x 0x 0x 1x

⍝ — Invert mixed-radix time decoding into hours, minutes and seconds
(0x 60x 60x∘⊥⍣¯1)3661x   ⍝ 1x 1x 1x

⍝ — Inverse encode decodes the supplied digits
(2x∘⊤⍣¯1)1x 0x 1x   ⍝ 5x

⍝ — Inverse decode permits a fractional least-significant digit
(2∘⊥⍣¯1)2.5   ⍝ 1 0.5

⍝ — Inverse transpose applies the inverse axis permutation
(2 1∘⍉⍣¯1)2 3⍴⍳6   ⍝ 3 2⍴1 4 2 5 3 6

⍝ — Inverting self-addition halves the argument exactly
(+⍨⍣¯1)3x   ⍝ 3r2

⍝ — Inverse sine selects a branch which maps back to the input
0.5=1∘○(1∘○⍣¯1)0.5   ⍝ 1x

⍝ — Inverse sine within the real domain has no imaginary component
11○(1∘○⍣¯1)0.5   ⍝ 0

⍝ — sqrt(1+x²) avoids overflowing its intermediate square
4○1E300   ⍝ 1E300

⍝ — Signed sqrt(x²-1) avoids intermediate overflow for large negative x
¯4○¯1E300   ⍝ ¯1E300

⍝ — Inversion preserves rank-zero application
((2x∘+)⍤0⍣¯1)3x 4x   ⍝ 1x 2x

⍝ — Inverse where counts repeated indices
(⍸⍣¯1)1 3 3   ⍝ 1x 0x 2x

⍝ — Inverse where infers multidimensional shape from coordinate vectors
(⍸⍣¯1)(1 2)(2 1)   ⍝ 2 2⍴0x 1x 1x 0x

⍝ — Inverse where of no indices returns an empty exact count vector
(⍸⍣¯1)⍬   ⍝ 0⍴0x

⍝ — Zero needs no digits in minimal-width inverse decode
(2∘⊥⍣¯1)0   ⍝ ⍬

⍝ —
3x(+⍣¯1)5x   ⍝ 2x

⍝ — Inversion distributes through each
((2x∘+)¨⍣¯1)3x 4x   ⍝ 1x 2x

⍝ — Inverting self-multiplication selects the positive square root
(×⍨⍣¯1)4   ⍝ 2

⍝ —
(⌽⍣¯1)⍳3x   ⍝ 3x 2x 1x

⍝ —
(⊂⍣¯1)⊂1x 2x   ⍝ 1x 2x

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
2x (÷\⍣¯1)1x 1r3 1r12   ⍝ 2x 3x 4x

⍝ — Axis-one scan inversion uses a separate seed for each column
⍉10 20 ((+\⍣¯1)⍤0 1)⍉2 2⍴11 22 14 26   ⍝ 2 2⍴1 2 3 4

⍝ —
(+\⍣¯1)⍬   ⍝ ⍬

⍝ — A seeded scalar scan still needs its first step undone
10 (+\⍣¯1)13   ⍝ 3

⍝ — Inverse axis-enclosure restores both shape and axis order
(⊂[2 1]⍣¯1)⊂[2 1]2 3 4⍴⍳24   ⍝ 2 3 4⍴⍳24

⍝ — Inverse split restores the original matrix axis
(↓[1]⍣¯1)↓[1]2 3⍴⍳6   ⍝ 2 3⍴⍳6

⍝ — Inverse fractional-axis mix recovers the original nested vectors
(⊃[0.5]⍣¯1)3 2⍴1 4 2 5 3 6   ⍝ (1 2 3)(4 5 6)

⍝ — Axis-qualified scalar inversion aligns the fixed left vector with rows
10 20 (+[1]⍣¯1)2 3⍴11 12 13 24 25 26   ⍝ 2 3⍴⍳6

⍝ — The last axis qualifier determines the inverse's axis too
(⌽[1][2]⍣¯1)2 3⍴3 2 1 6 5 4   ⍝ 2 3⍴⍳6

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
avg←+/÷≢ ⋄ avg 1x 2x 4x   ⍝ 7r3

⍝ —
+/⍳3x   ⍝ 6x

⍝ —
×/⍳4x   ⍝ 24x

⍝ —
1x 2x+.×3x 4x   ⍝ 11x

⍝ —
⌊3r2   ⍝ 1x

⍝ —
⌈3r2   ⍝ 2x

⍝ —
|¯3x   ⍝ 3x

⍝ —
3x⌊4x   ⍝ 3x

⍝ —
3x⌈4x   ⍝ 4x

⍝ —
¯3x|5x   ⍝ ¯1x

⍝ —
6x∨4x   ⍝ 2x

⍝ —
6x∧4x   ⍝ 12x

⍝ —
!20x   ⍝ 2432902008176640000x

⍝ —
5x!10x   ⍝ 252x

⍝ —
2x*10x   ⍝ 1024x

⍝ —
2x*¯1x   ⍝ 1r2

⍝ —
2x⊥1x 0x 1x   ⍝ 5x

⍝ —
≢(1x 2x)(1r3 'a')   ⍝ 2x

⍝ —
≢0⍴1x   ⍝ 0x

⍝ — Subtraction below i64 minimum promotes without losing precision
¯9223372036854775808x-1x   ⍝ ¯9223372036854775809x

⍝ — Multiplication above i64 maximum promotes without losing precision
9223372036854775807x×2x   ⍝ 18446744073709551614x

⍝ — An exact sum remains exact at the i64 boundary
+/9223372036854775807x 1x ¯1x   ⍝ 9223372036854775807x

⍝ — Ordinary division remains approximate, unlike explicit exact arithmetic
(1÷3)+(1÷6)   ⍝ 0.5

⍝ —
1x+0.5   ⍝ 1.5

⍝ — Tally counts outer items regardless of nested numeric domains
≢(1x 2x)(3 4)   ⍝ 2x

⍝ —
≢'abc'   ⍝ 3x

⍝ —
≢0⍴⊂1x 2   ⍝ 0x

⍝ —
=/⍬   ⍝ 1x

⍝ —
≠/⍬   ⍝ 0x

⍝ — Empty nested roll retains an exact vector prototype
↑?0⍴⊂1x 2x   ⍝ 0x 0x

⍝⍝ Matrix division

⍝ —
⌹2x   ⍝ 1r2

⍝ — Exact overdetermined solve recovers the line y=1+2x
3x 5x 7x⌹3 2⍴1x 1x 1x 2x 1x 3x   ⍝ 1x 2x

⍝ — A vector pseudoinverse divides by the squared norm
⌹1x 2x   ⍝ 1r5 2r5

⍝ — Exact matrix inversion handles a zero leading pivot
⌹2 2⍴0x 2x 1x 0x   ⍝ 2 2⍴0x 1x 1r2 0x

⍝ — Inverting a zero-column matrix exchanges its dimensions
⌹3 0⍴0x   ⍝ 0 3⍴0x

⍝ — Solving for no unknowns preserves the number of right-hand sides
(3 2⍴1)⌹3 0⍴0   ⍝ 0 2⍴0

⍝ —
⌹0
⍝ error: DOMAIN ERROR

⍝ — Singular matrices are rejected rather than assigned an arbitrary solution
⌹2 2⍴1
⍝ error: DOMAIN ERROR

⍝ — The exact solver also rejects singular matrices
⌹2 2⍴1x
⍝ error: DOMAIN ERROR

⍝ —
⌹'ab'
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

⍝ — Functional update leaves the original array unchanged
a←⍳3 ⋄ b←0@2⊢a ⋄ a   ⍝ 1 2 3

⍝ — Stencil reports leading-axis edge padding even when rows are empty
{⍺}⌺3⊢2 0⍴0   ⍝ 2 1⍴1 ¯1

⍝ — A two-row stencil specification gives window size then movement
{+/,⍵}⌺(2 1⍴3 2)⍳8   ⍝ 3 9 15 21

⍝ — A single stencil size varies the leading axis and retains whole rows
{⍴⍵}⌺3⊢2 3⍴⍳6   ⍝ 2 2⍴3x

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
0@{2 0 1}⍳3
⍝ error: DOMAIN ERROR

⍝ — Nested index paths select fields inside matrix items
G←2 3⍴('ABC' 1)('DEF' 2)('GHI' 3)('JKL' 4)('MNO' 5)('PQR' 6) ⋄ G[((1 2)1)((2 3)2)]
'DEF' 6

⍝ — At replaces nested fields without replacing their containing items
G←2 3⍴('ABC' 1)('DEF' 2)('GHI' 3)('JKL' 4)('MNO' 5)('PQR' 6) ⋄ H←('' '*' @((1 2)1)((2 3)2))G ⋄ (1⊃1 2⊃H)(2⊃2 3⊃H)
'' '*'

⍝ — At calls its operand once even for an empty selection
{⎕←99 ⋄ ⍵}@⍬⍳3
⍳3
⍝ ⎕: 99

⍝⍝ Encode decode

⍝ —
60⊥3 13   ⍝ 193

⍝ —
2x⊥1x 0x 1x 0x   ⍝ 10x

⍝ — A leading zero radix retains the remaining quotient
0x 10x⊤125x   ⍝ 12x 5x

⍝ — Empty digits decode to zero
2⊥⍬   ⍝ 0

⍝ — An empty radix vector leaves no terms to decode
⍬⊥1   ⍝ 0

⍝ — A zero radix consumes the remainder, leaving zero for earlier digits
0 0 2⊤3   ⍝ 0 1 1

⍝ — Encode appends the value frame to the radix shape, including zero dimensions
⍴(0 3⍴0)⊤2 2⍴1   ⍝ 0x 3x 2x 2x

⍝ — Decode the same digits in binary and decimal
(2 1⍴2 10)⊥1 0 1   ⍝ 5 101

⍝ —
2 3⊥1 2 3
⍝ error: LENGTH ERROR

⍝⍝ Pick and partition

⍝ — Pick descends through a matrix coordinate, a nested field, then a character index
2⊃1⊃2 1⊃2 3⍴('ABC' 1)('DEF' 2)('GHI' 3)('JKL' 4)('MNO' 5)('PQR' 6)
'K'

⍝ — Empty coordinates can repeatedly pick an atom without changing it
(⍬∘⊃)⍣5⊢10   ⍝ 10

⍝ — An empty pick path returns the whole argument
⍬⊃1 2   ⍝ 1 2

⍝ —
2⌷3 4⍴⍳12   ⍝ 5 6 7 8

⍝ — Axis-qualified squad selects a column rather than a row
2⌷[2]3 4⍴⍳12   ⍝ 2 6 10

⍝ — Empty indices on both axes retain a rank-two empty result
⍴⍬ ⍬⌷3 4⍴0   ⍝ 0x 0x

⍝ —
⍬⌷1   ⍝ 1

⍝ —
⌷1 2   ⍝ 1 2

⍝ —
⊆1 2   ⍝ ⊂1 2

⍝ —
⊆1   ⍝ ⊂1

⍝ — Nest leaves an already nested vector unchanged
⊆(1 2)(3 4)   ⍝ (1 2)(3 4)

⍝ — Partition starts on a positive rise, not every label change; zero omits an item
3 2 2 1 0 1⊆'abcdef'   ⍝ 'abcd'(,'f')

⍝ — Axis-one partition groups column segments and omits the zero-marked row
1 1 0 1⊆[1]4 2⍴⍳8   ⍝ 2 2⍴(1 3)(2 4)(,7)(,8)

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
(,1)⊂'abcd'   ⍝ ,⊂'abcd'

⍝ — A scalar partition label groups the whole vector
1⊆'abcd'   ⍝ ,⊂'abcd'

⍝ — A singleton label vector also extends over the whole argument
(,1)⊆'abcd'   ⍝ ,⊂'abcd'

⍝⍝ Grading

⍝ — Descending grade preserves the original order of ties
⍒2 1 2 1   ⍝ 1x 3x 2x 4x

⍝ — Grade ignores comparison tolerance
⍋1 (1+8E¯15) 1   ⍝ 1x 3x 2x

⍝ — Numbers precede characters; complex numbers sort by real then imaginary part
⍋1j2 1 1j¯2 'a' 0   ⍝ 5x 3x 2x 1x 4x

⍝ — Mixed-domain grade does not round large exact integers through float
⍋9007199254740993x 9007199254740992 9007199254740992x
2x 3x 1x

⍝ — Equal-rank nested arrays compare ravelled contents before shape
⍋(2 2⍴1 2 3 4)(1 4⍴1 2 0 0)   ⍝ 2x 1x

⍝ — Nested rank takes precedence over contents
⍋(1 2⍴1 2)(1 2)   ⍝ 2x 1x

⍝ — Empty nested arrays sort by rank, then shape
⍋(0 5 2⍴0)(0 3 4⍴0)(0 1⍴'')⍬   ⍝ 4x 3x 2x 1x

⍝ — Empty-array prototypes do not break sorting ties
⍋(0⍴⊂1 2)(0⍴0)(0⍴'')   ⍝ 1x 2x 3x

⍝ — Structural order puts numbers before characters before nested arrays
⍋'z' (0 0) 100 'a'   ⍝ 3x 4x 1x 2x

⍝ — Character vectors sort lexicographically, not by length first
⍋'ba' 'aaa' 'ab'   ⍝ 2x 3x 1x

⍝ — Equal numbers retain their order across numeric representations
⍋1x 1 1j0   ⍝ 1x 2x 3x

⍝ — Interval index shares grade's ordering across numbers, characters and nested arrays
1 'a' (1 2)⍸0 'b' (2 3)   ⍝ 0x 2x 3x

⍝ — Interval lookup preserves exact distinctions beyond float's integer range
9007199254740992x 9007199254740993x⍸9007199254740992 9007199254740994
1x 2x

⍝ — Complex interval lookup uses real-then-imaginary ordering
1j¯1 1 1j1⍸1j¯2 1j0 1j2   ⍝ 0x 2x 3x

⍝ — Zero-width rows are stable ties, not an empty collection of rows
⍋3 0⍴0   ⍝ 1x 2x 3x

⍝ —
⍋⍬   ⍝ 0⍴0x

⍝ — Custom collation puts unlisted characters last, retaining their order
'cba'⍋'azb?c'   ⍝ 5x 3x 1x 2x 4x

⍝ — A multidimensional collation array supplies successive collation keys
(2 2⍴'ABBA')⍋3 2⍴'BAABBA'   ⍝ 1x 2x 3x

⍝ —
⍋1
⍝ error: RANK ERROR

⍝ —
'a'⍋'ab'
⍝ error: RANK ERROR

⍝ —
'ab'⍒'a'
⍝ error: RANK ERROR

⍝ —
1 2⍋'ab'
⍝ error: DOMAIN ERROR

⍝ —
'ab'⍋1 2
⍝ error: DOMAIN ERROR

⍝ —
''⍋⍬
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
+/1x 2x 3x   ⍝ 6x

⍝ — Zero divided by zero is one, applied elementwise
0 1÷0 2   ⍝ 1 0.5

⍝⍝ Unicode conversion

⍝ — Case folding pervades nested text and leaves numbers unchanged
•C 42 'Pete' 'Πέτρος'   ⍝ 42 'pete' 'πέτροσ'

⍝ — Simple uppercase preserves shape and never expands one character to several
1•C 2 3⍴'aBcΣςß'   ⍝ 2 3⍴'ABCΣΣß'

⍝ — Simple lowercase maps Unicode characters without expanding them
¯1•C 'İẞᾈΣ'   ⍝ 'ißᾀσ'

⍝ — Simple case folding differs from full folding for dotted I and ligatures
•C 'ẞİﬀᾀ'   ⍝ 'ßİﬀᾀ'

⍝ — A singleton selector of any rank is accepted for case folding
(1 1⍴¯3)•C 'ίσως'   ⍝ 'ίσωσ'

⍝ — Case conversion preserves an empty nested character prototype
•C 2 0⍴⊂'Ab'   ⍝ 2 0⍴⊂'  '

⍝ — System names are case-insensitive and may be bound to ordinary names
u←•ucs ⋄ u 2 2⍴'A⍳λ😀'   ⍝ 2 2⍴65x 9075x 955x 128512x

⍝ —
•UCS 2 2⍴65x 9075x 955x 128512x   ⍝ 2 2⍴'A⍳λ😀'

⍝ — Code-point conversion preserves an empty character array's shape
•UCS 2 0⍴''   ⍝ 2 0⍴0x

⍝ — Empty numeric input converts to a character prototype
•UCS 0 3⍴0   ⍝ 0 3⍴''

⍝ —
'UTF-8'•UCS 'Æ😀'   ⍝ 195x 134x 240x 159x 152x 128x

⍝ —
('UTF-8' 0)•UCS 195 134 240 159 152 128   ⍝ 'Æ😀'

⍝ — UTF-16 encodes an astral character as a surrogate pair
'UTF-16'•UCS 'A😀'   ⍝ 65x 55357x 56832x

⍝ — UTF-16 decodes a surrogate pair into one character
'UTF-16'•UCS 65 55357 56832   ⍝ 'A😀'

⍝ — An enclosed encoding name is accepted; UTF-32 retains one unit per character
(⊂'UTF-32')•UCS 'A😀'   ⍝ 65x 128512x

⍝ —
'UTF-32'•UCS 65 128512   ⍝ 'A😀'

⍝ — Encoding a scalar character returns a byte vector
'UTF-8'•UCS 'A'   ⍝ ,65x

⍝ — Decoding a scalar byte returns a character vector
'UTF-8'•UCS 65   ⍝ ,'A'

⍝ —
'UTF-16'•UCS ⍬   ⍝ ''

⍝ —
0•C 'a'
⍝ error: DOMAIN ERROR

⍝ —
1 2•C 'a'
⍝ error: DOMAIN ERROR

⍝ —
•UCS ¯1
⍝ error: DOMAIN ERROR

⍝ — Surrogates are not Unicode scalar values
•UCS 55296
⍝ error: DOMAIN ERROR

⍝ — Unicode code points stop at 10FFFF
•UCS 1114112
⍝ error: DOMAIN ERROR

⍝ —
•UCS 1.5
⍝ error: DOMAIN ERROR

⍝ — Mixed numeric and character input has no single conversion direction
•UCS 65 'B'
⍝ error: DOMAIN ERROR

⍝ — A nested prototype does not make empty input a simple character array
•UCS 0⍴⊂'ab'
⍝ error: DOMAIN ERROR

⍝ — Encoding labels are case-sensitive
'utf-8'•UCS 'a'
⍝ error: DOMAIN ERROR

⍝ — Reject an overlong UTF-8 encoding of zero
'UTF-8'•UCS 192 128
⍝ error: DOMAIN ERROR

⍝ —
'UTF-8'•UCS 256
⍝ error: DOMAIN ERROR

⍝ — A lone high surrogate is invalid UTF-16
'UTF-16'•UCS 55357
⍝ error: DOMAIN ERROR

⍝ — UTF-16 code units must fit in 16 bits
'UTF-16'•UCS 65536
⍝ error: DOMAIN ERROR

⍝ —
'UTF-8'•UCS 2 2⍴'a'
⍝ error: RANK ERROR

⍝ — Signed-byte mode is explicitly outside miniapl's Unicode interface
('UTF-8' 83)•UCS 'abc'
⍝ error: UNSUPPORTED

⍝⍝ Character arithmetic

⍝ —
'a'+3   ⍝ 'd'

⍝ —
3x+'a'   ⍝ 'd'

⍝ —
'd'-3r1   ⍝ 'a'

⍝ —
'd'-'a'   ⍝ 3x

⍝ —
'012'-'0'   ⍝ 0x 1x 2x

⍝ —
('ab' 'CD')+1   ⍝ 'bc' 'DE'

⍝ — Character offsets follow leading-axis agreement
(2 3⍴'abcdef')+1 2   ⍝ 2 3⍴'bcdfgh'

⍝ —
''+3   ⍝ ''

⍝ — Empty character subtraction has an exact numeric prototype
''-'a'   ⍝ 0⍴0x

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
(•UCS 55295)+1
⍝ error: DOMAIN ERROR

⍝ — Character arithmetic cannot advance beyond the Unicode range
(•UCS 1114111)+1
⍝ error: DOMAIN ERROR

⍝ — Character arithmetic cannot move below code point zero
(•UCS 0)-1
⍝ error: DOMAIN ERROR

⍝⍝ Characters nesting and empty fill

⍝ — System functions use ordinary binding and composition
upper←1∘•c ⋄ (•UCS∘upper)'aZ'   ⍝ 65x 90x

⍝ — System names resolve when executed
f←{•missing ⍵} ⋄ 1   ⍝ 1

⍝ — A system name includes its entire word
•C_unknown2 'abc'
⍝ error: UNSUPPORTED

⍝ — System functions are read-only
•C←+
⍝ error: SYNTAX ERROR

⍝ — Explicit strands can hold system functions
fs←•UCS˘•C ⋄ (↑fs)'A'   ⍝ 65x

⍝ —
•A   ⍝ 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'

⍝ —
•d   ⍝ '0123456789'

⍝ —
3↑•a   ⍝ 'ABC'

⍝ — Alphabet constants are read-only
•A←'abc'
⍝ error: SYNTAX ERROR

⍝ — miniapl has fixed origin one, not a mutable index-origin variable
•IO←0
⍝ error: UNSUPPORTED

⍝ —
'abc
⍝ error: SYNTAX ERROR

⍝ — Character literals cannot span source lines
'a
b'
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
'界λ'   ⍝ '界λ'

⍝ —
'can''t'   ⍝ 'can''t'

⍝ —
''   ⍝ ''

⍝ —
⍬   ⍝ ⍬

⍝ — Disclosing an empty numeric vector returns its zero prototype
↑⍬   ⍝ 0.0

⍝ —
↑⊂1 2   ⍝ 1.0 2.0

⍝ — An empty nested vector retains its first item's shape as prototype
↑0⍴(1 2)(3 4 5)   ⍝ 0.0 0.0

⍝ — Taking from empty text fills with spaces
3↑''   ⍝ '   '

⍝ — Negative overtake pads on the left
¯4↑1 2   ⍝ 0.0 0.0 1.0 2.0

⍝ —
2↓'abcd'   ⍝ 'cd'

⍝ —
¯2↓'abcd'   ⍝ 'ab'

⍝ —
5↓'ab'   ⍝ ''

⍝ —
0⍴'abc'   ⍝ ''

⍝ — Reshaping empty text uses its character prototype
3⍴''   ⍝ '   '

⍝ — An empty shape produces a scalar from the first element
⍬⍴1 2   ⍝ ⊂1.0

⍝ — Numeric negation of empty text yields an empty numeric result
-''   ⍝ ⍬

⍝ — Direct scalar division on empty text makes no element calls
1÷''   ⍝ ⍬

⍝ —
'ab'=1 2   ⍝ 0x 0x

⍝ —
'abc'='axc'   ⍝ 1x 0x 1x

⍝ —
+'abc'   ⍝ 'abc'

⍝ —
+''   ⍝ ''

⍝ — An atomic index retrieves a character vector
'abc' 'def' 'ghi'[2]   ⍝ 'def'

⍝ —
gg←2 3 4 5 ⋄ 9,gg[2],3 4   ⍝ 9 3 3 4

⍝ — Split into rows; each further split encloses the resulting vector or scalar
↓↓↓2 2⍴⍳4   ⍝ ⊂⊂(1 2)(3 4)

⍝ — Reduction returns the accumulated vector
+/(1 2)(3 4)   ⍝ 4 6

⍝ —
(1 2)(3 4)+10   ⍝ (11 12)(13 14)

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
10x -\1x 2x 3x   ⍝ 9x 7x 4x

⍝ — Seeded scalar scan still applies the operand once
2 +\5   ⍝ 7

⍝ — Trailing-axis scan takes one seed per row
10 20 (+\⍤0 1)2 3⍴⍳6   ⍝ 2 3⍴11 13 16 24 29 35

⍝ — Leading-axis scan takes one seed per column
⍉10 20 30 (+\⍤0 1)⍉2 3⍴⍳6   ⍝ 2 3⍴11 22 33 15 27 39

⍝ — Rank pairs row seeds for axis-qualified scan
10 20 ({⍺+⍵}\[1]⍤0 1)2 3⍴⍳6   ⍝ 2 3⍴11 13 16 24 29 35

⍝ — Growing scan accumulators remain nested, without mix-style padding
{⍺,⍵}\1 2 3   ⍝ 1 (1 2) (1 2 3)

⍝ — A seed participates in every growing accumulator
0 {⍺,⍵}\1 2 3   ⍝ (0 1)(0 1 2)(0 1 2 3)

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
+\2 3⍴⍳6   ⍝ 2 3⍴1 3 6 4 9 15

⍝ —
+⍀2 3⍴⍳6   ⍝ 2 3⍴1 2 3 5 7 9

⍝ —
+\0 3⍴0   ⍝ 0 3⍴0

⍝ — Mix pads shorter cells to the longest cell
⊃(1 2)(3 4 5)   ⍝ 2 3⍴1 2 0 3 4 5

⍝ — Mix pads an atomic cell rather than extending its value
⊃1 (2 3)   ⍝ 2 2⍴1 0 2 3

⍝ — Empty mix uses the retained cell prototype to determine its trailing shape
⊃0⍴(1 2)(3 4 5)   ⍝ 0 2⍴0

⍝ —
⊃⊂1 2   ⍝ 1 2

⍝ — Empty nested reduction uses a conforming identity
+/0⍴(1 2)(3 4)   ⍝ 0 0

⍝ — Empty character sum uses the numeric addition identity
+/''   ⍝ 0

⍝ — Leading-axis replicate keeps the selected matrix row
1 0⌿2 3⍴⍳6   ⍝ 1 3⍴1 2 3

⍝ — Each accumulator retains the whole vector seed
(,10)+\1 2 3   ⍝ (,11)(,13)(,16)

⍝ — Each lane starts with the same whole seed
10 20 30+\2 3⍴⍳6
2 3⍴(11 21 31)(13 23 33)(16 26 36)(14 24 34)(19 29 39)(25 35 45)

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

⍝ —
⍉[1 2 3 ⋄ 4 5 6]   ⍝ 3 2⍴1 4 2 5 3 6

⍝ —
⌽[1 2 3 ⋄ 4 5 6]   ⍝ 2 3⍴3 2 1 6 5 4

⍝ —
⊖[1 2 3 ⋄ 4 5 6]   ⍝ 2 3⍴4 5 6 1 2 3

⍝ — Rotate each row by its own count
1 2⌽[1 2 3 ⋄ 4 5 6]   ⍝ 2 3⍴2 3 1 6 4 5

⍝ — Catenating a vector to a matrix appends one element per row
[1 2 3 ⋄ 4 5 6],8 9   ⍝ 2 4⍴1 2 3 8 4 5 6 9

⍝ —
1 2↑[1 2 3 ⋄ 4 5 6]   ⍝ 1 2⍴1 2

⍝ — Negative multidimensional overtake pads before the retained bottom-right cells
¯3 ¯2↑[1 2 3 ⋄ 4 5 6]   ⍝ 3 2⍴0 0 2 3 5 6

⍝ —
1 ¯1↓[1 2 3 ⋄ 4 5 6]   ⍝ 1 2⍴4 5

⍝ — Taking a matrix from a scalar pads rather than repeating it
2 3↑7   ⍝ 2 3⍴7 0 0 0 0 0

⍝ —
[1 2 3 ⋄ 4 5 6][2;1]   ⍝ 4

⍝ —
[1 2 3 ⋄ 4 5 6][;2]   ⍝ 2 5

⍝ —
[1 2 3 ⋄ 4 5 6][2;]   ⍝ 4 5 6

⍝ —
(10 20 30)[3 1]   ⍝ 30 10

⍝ — The first brackets qualify the reduction axis; the second form an array literal
+/[1][1 2 3 ⋄ 4 5 6]   ⍝ 5 7 9

⍝ — A named reduction accepts the same axis qualifier
s←+/ ⋄ s[1][1 2 3 ⋄ 4 5 6]   ⍝ 5 7 9

⍝ — Repeated transpose axes select the diagonal of a rectangular matrix
1 1⍉[1 2 3 ⋄ 4 5 6]   ⍝ 1 5

⍝ — Negative replicate counts replace that item with fill
2 ¯1 1/10 20 30   ⍝ 10 10 0 30

⍝ — Negative expand counts insert fill without consuming an input item
2 ¯1 1\10 20   ⍝ 10 10 0 20

⍝ — Expansion uses character fill for inserted positions
1 0 1\'ab'   ⍝ 'a b'

⍝ —
(1 2)[0]
⍝ error: INDEX ERROR

⍝ —
(1 2)[¯1]
⍝ error: INDEX ERROR

⍝ —
(1 2)[3]
⍝ error: INDEX ERROR

⍝ —
(1 2)[1.5]
⍝ error: DOMAIN ERROR

⍝ — Bracket indexing a matrix requires an index slot for each axis
[1 2 ⋄ 3 4][1]
⍝ error: RANK ERROR

⍝ —
+/[0][1 2 ⋄ 3 4]
⍝ error: DOMAIN ERROR

⍝ — Nested literals and dfn guards do not split the surrounding matrix incorrectly
[({⍵=0:1 ⋄ ⍵+2}0 ⋄ 3) ⋄ (4 ⋄ 5)][;1]   ⍝ 1 4

⍝⍝ Scalar math

⍝ — Complex floor uses Gaussian-integer cells, not independent component floors
⌊1.5j0.5   ⍝ 1j1

⍝ — Complex gcd uses Gaussian-integer arithmetic and tolerant comparison
0.2=3.8j7.6∨5.2j6.8   ⍝ 1x

⍝ — Scale and ceil to expose the complex lcm despite floating-point roundoff
⌈1000×3.8j7.6∧5.2j6.8   ⍝ ¯159600j326800

⍝ —
|3 ¯3 3J4   ⍝ 3 3 5

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
9 11○3J4   ⍝ 3 4

⍝ —
2x*¯3x   ⍝ 1r8

⍝ —
2r3|7r3   ⍝ 1r3

⍝ —
⌊¯4r3   ⍝ ¯2x

⍝ —
⌈¯4r3   ⍝ ¯1x

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
0J1⌊2
⍝ error: DOMAIN ERROR

⍝⍝ Search depth and random

⍝ — Index-of compares each candidate directly; tolerance is not transitive
x←1 ⋄ y←1+8E¯15 ⋄ z←1+16E¯15 ⋄ x y⍳z   ⍝ 2x

⍝ — Unique-mask compares against retained representatives, not every earlier item
≠1 (1+8E¯15) (1+16E¯15)   ⍝ 1x 0x 1x

⍝ — Without uses tolerance when approximate values participate
1 2~1+8E¯15   ⍝ ,2

⍝ — Without does not apply tolerance to all-exact values
1x 2x~1000000000000001r1000000000000000   ⍝ 1x 2x

⍝ — Each empty row matches the first empty row
(3 0⍴0)⍳2 0⍴0   ⍝ 1x 1x

⍝ — A matrix of empty rows still has one unique major cell
≠3 0⍴0   ⍝ 1x 0x 0x

⍝ — Where repeats a position according to its count
⍸0 1 0 2   ⍝ 2x 4x 4x

⍝ — Interval index ignores comparison tolerance at interval boundaries
1 2⍸1-1E¯15   ⍝ 0x

⍝ — Depth of an empty nested array comes from its prototype
≡0⍴(1 2)3   ⍝ 2x

⍝ — Not-match distinguishes an atom from a singleton vector
1≢,1   ⍝ 1x

⍝ — Match compares numeric value across exact and approximate domains
1x≡1   ⍝ 1x

⍝ — Empty numeric prototypes match across exact and approximate domains
(0⍴1x)≡⍬   ⍝ 1x

⍝ —
⍳,3   ⍝ 1 2 3

⍝ — Union retains duplicates within either argument
1 1∪2 2   ⍝ 1 1 2 2

⍝ —
1 2∊1+8E¯15   ⍝ 1x 0x

⍝ — A vector pattern does not fit a scalar search target
(,1)⍷1   ⍝ 0x

⍝ — An empty pattern still cannot exceed the target on another axis
(2 0⍴0)⍷1 3⍴1   ⍝ 1 3⍴0x

⍝ — Rank-zero iota contains one enclosed empty coordinate
⍳⍬   ⍝ ⊂⍬

⍝ — Where on scalar zero retains an empty-coordinate prototype
⍸0   ⍝ 0⍴⊂0⍴0x

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
f←{100⊃'abc'} ⋄ f¨⍬   ⍝ ''

⍝ — Prototype mode survives nested helper calls
pick←{⍺⊃⍵} ⋄ wrap←{⍺ pick ⍵} ⋄ ⍬ wrap¨⊂'abc'
''

⍝ — Prototype mode also passes through a defined operator
op←{⍶ ⍵} ⋄ ({100⊃'abc'}op)¨⍬   ⍝ ''

⍝ — A composed operand derives character fill in empty each
⍬(⊃∘⊢)¨⊂'abc'   ⍝ ''

⍝ — Commute preserves prototype-mode Pick
(⊂1 2 3)⊃⍨¨⍬   ⍝ ⍬

⍝ — Empty Pick-each retains the selected nested vector's shape
⍬⊃¨⊂(1 2)(3 4 5)   ⍝ 0⍴⊂0 0

⍝ — An empty-path prototype selects the whole right-hand vector
(0⍴⊂⍬)⊃¨⊂1 2 3   ⍝ 0⍴⊂0 0 0

⍝ — The requested nested item, not always the first, determines result fill
2⊃¨0⍴⊂(1 2)(3 4 5)   ⍝ 0⍴⊂0 0 0

⍝ — Named functions and operators retain their grammatical roles
e←¨ ⋄ sum←+/ ⋄ sum e (1 2)(3 4 5)   ⍝ 3 12

⍝ — Nested each applies singleton reduction to each numeric leaf
+/¨¨(1 2)(3 4)   ⍝ (1 2)(3 4)

⍝ — Selfie with an array operand creates a constant function
2⍨3   ⍝ 2

⍝ —
2-⍨5   ⍝ 3

⍝ — Oversized windows have an empty frame
+/3↕1 2   ⍝ ⍬

⍝ — Reduce adjacent-row windows along their window axis
+/[2]2↕2 3⍴⍳6   ⍝ 1 3⍴5 7 9

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

⍝ — Prototype mode does not make negative Pick indices valid
¯1⊃¨0⍴⊂1 2 3
⍝ error: INDEX ERROR

⍝ — Invalid negative Pick indices still error through a dfn in prototype mode
¯1{⍺⊃⍵}¨0⍴⊂1 2 3
⍝ error: INDEX ERROR

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
'abc'⍴⍛⍴'z'   ⍝ 'zzz'

⍝ — Behind combines matrix rows as imaginary and real components
¯11∘○⍛+⌿2 3⍴1 2 3 4 5 6   ⍝ 4j1 5j2 6j3

⍝ — Naming compose, Behind and reduction preserves their binding
c←∘ ⋄ b←⍛ ⋄ r←⌿ ⋄ ¯11 c ○ b + r 2 3⍴1 2 3 4 5 6
4j1 5j2 6j3

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
f←{k←3 ⋄ g←{k+⍵} ⋄ op←{⍶ ⍹ ⍵} ⋄ (+op g)⍵} ⋄ f 4
7

⍝ —
+⍤⍬⊢3
⍝ error: LENGTH ERROR

⍝ —
+⍤0.5⊢3
⍝ error: DOMAIN ERROR

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
{≢⍵}⌸1 (1+8E¯15)(1+16E¯15)   ⍝ 2x 1x

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
⍝ ⎕: 0x 2x

⍝ — Zero iterations never call the operand
({⎕←7 ⋄ ⍵}⍣0)3
3
⍝ ⎕:

⍝ — Power propagates an operand's no-result return
({}⍣1)3   ⍝ {}0

⍝⍝ Products

⍝ — Inner product may use catenate rather than a scalar reduction
1 2 3,.-3 3⍴4 5 6   ⍝ (¯3 ¯2 ¯1)(¯4 ¯3 ¯2)(¯5 ¯4 ¯3)

⍝ — This fork enumerates leading-axis indices and selects each major cell
(⍳∘≢( ⌷⌝ )⊂)2 3 3⍴⍳18
(3 3⍴1 2 3 4 5 6 7 8 9)(3 3⍴10 11 12 13 14 15 16 17 18)

⍝ —
(2 3⍴⍳6)+.×3 2⍴⍳6   ⍝ 2 2⍴22 28 49 64

⍝ — An empty contraction fills every result cell with the reduction identity
(2 0⍴0)+.×0 3⍴0   ⍝ 2 3⍴0

⍝ — Singleton extension also applies along the contracted axis
(,2)+.×1 2 3   ⍝ 12

⍝ — Named outer-product operator
outer←⌝ ⋄ times←× ⋄ 1 2 times outer 3 4   ⍝ 2 2⍴3 4 6 8

⍝ — Parenthesized outer-product operator
1 2 +(⌝) 10 20   ⍝ 2 2⍴11 21 12 22

⍝ — Outer product accepts composed operands
1 2 (-⍤+)⌝ 10 20   ⍝ 2 2⍴¯11 ¯21 ¯12 ¯22

⍝ — Commute follows outer-product derivation
-⌝⍨1 2   ⍝ 2 2⍴0 ¯1 1 0

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
×/0/1J2   ⍝ 1

⍝ — Iota accepts a complex representation only when its imaginary part is zero
⍳3J0   ⍝ 1 2 3

⍝⍝ Complex comparison errors and recovery

⍝ — A complex literal requires an imaginary component
1J
⍝ error: SYNTAX ERROR

⍝ —
1J¯
⍝ error: SYNTAX ERROR

⍝ —
1J2E¯
⍝ error: SYNTAX ERROR

⍝ — Exact suffixes cannot be attached to a complex component
1J2x
⍝ error: SYNTAX ERROR

⍝ — Complex components use approximate literals, not exact integers
1xJ2
⍝ error: SYNTAX ERROR

⍝ — Complex components cannot use rational-literal syntax
1r2J3
⍝ error: SYNTAX ERROR

⍝ —
1J2J3
⍝ error: SYNTAX ERROR

⍝ — Complex components must remain finite
1J1E309
⍝ error: DOMAIN ERROR

⍝ —
1E309J1
⍝ error: DOMAIN ERROR

⍝ — Overflow to complex infinity is rejected
1E308J1E308+1E308J1E308
⍝ error: DOMAIN ERROR

⍝ —
1J2÷0
⍝ error: DOMAIN ERROR

⍝ —
÷0J0
⍝ error: DOMAIN ERROR

⍝ — Counts require exactly zero imaginary part, without tolerance
⍳1J1E¯15
⍝ error: DOMAIN ERROR

⍝ —
1J2/3
⍝ error: DOMAIN ERROR

⍝ — Complex numbers have no scalar ordering, even against themselves
1J2<1J2
⍝ error: DOMAIN ERROR

⍝ —
1J2≤2
⍝ error: DOMAIN ERROR

⍝ —
2>1J2
⍝ error: DOMAIN ERROR

⍝ —
2≥1J2
⍝ error: DOMAIN ERROR

⍝⍝ Exact literals arithmetic and roundtrips

⍝ — One approximate operand makes division approximate, even when the other is exact
1x÷2   ⍝ 0.5

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
1x÷0x
⍝ error: DOMAIN ERROR

⍝ —
÷0x
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
1r2r3
⍝ error: SYNTAX ERROR

⍝ —
1x2
⍝ error: SYNTAX ERROR

⍝⍝ Exact arrays prototypes and counts

⍝ — Empty exact sum retains an exact additive identity
+/0/1r3   ⍝ 0x

⍝ — Empty exact product retains an exact multiplicative identity
×/0/1r3   ⍝ 1x

⍝ — Shape returns exact dimensions even for approximate data
⍴2 3⍴0.5   ⍝ 2x 3x

⍝ — Scalar shape is an empty exact vector
⍴42   ⍝ 0⍴0x

⍝ —
⍴''   ⍝ ,0x

⍝ —
≢'abc'   ⍝ 3x

⍝ — Tally counts rows, so a zero-row matrix has tally zero
≢0 3⍴0.5   ⍝ 0x

⍝ — Nested element shapes do not affect the outer shape
⍴(1r3)(1.5 2)   ⍝ ,2x

⍝ — Iota follows an exact argument's numeric domain
⍳3x   ⍝ 1x 2x 3x

⍝ —
⍳3r2
⍝ error: DOMAIN ERROR

⍝ —
⍳¯1x
⍝ error: DOMAIN ERROR

⍝ —
⍳1000001x
⍝ error: LIMIT ERROR

⍝ —
⍳999999999999999999999x
⍝ error: LIMIT ERROR

⍝ —
1r2/3
⍝ error: DOMAIN ERROR

⍝⍝ Exact and tolerant comparisons

⍝ — Approximate equality uses relative tolerance 1E¯14. https://docs.dyalog.com/20.0/language-reference-guide/system-functions/ct/
0.3=0.3 (0.1+0.2) 0.4   ⍝ 1x 1x 0x

⍝ —
1 2≤2 1   ⍝ 1x 0x

⍝ — Tolerant equality is not transitive: x=y and y=z need not imply x=z
(1=1+8E¯15)((1+8E¯15)=1+16E¯15)(1=1+16E¯15)
1x 1x 0x

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
⍴⍨¨/3/⊂⍳4   ⍝ (,1)(2 2⍴2)(3 3 3⍴3)(4 4 4 4⍴4)

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
99999999999999999999x/1
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
⌊/0⍴0x   ⍝ ∞

⍝ — Empty exact maximum uses negative infinity
⌈/0⍴0x   ⍝ ¯∞

⍝ — Minimum retains a finite exact value even beyond float range
∞⌊10x*1000x   ⍝ 10x*1000x

⍝ — Maximum against negative infinity preserves an exact fraction
¯∞⌈1r3   ⍝ 1r3

⍝ —
∞=∞ ¯∞ 1   ⍝ 1x 0x 0x

⍝ — Infinity exceeds any finite exact integer without converting that integer to float
∞>10x*1000x   ⍝ 1x

⍝ — Reversing comparison arguments retains the finite-versus-infinite distinction
(10x*1000x)<∞   ⍝ 1x

⍝ —
∞=1j2   ⍝ 0x

⍝ —
∞∊1 2 ∞   ⍝ 1x

⍝ —
∞ ¯∞⍳¯∞ ∞ 0   ⍝ 2x 1x 3x

⍝ — Grade orders a huge finite integer between the two infinities
⍋∞ (10x*1000x) ¯∞   ⍝ 3x 2x 1x

⍝ —
∪∞ ∞ ¯∞   ⍝ ∞ ¯∞

⍝ —
⍕∞ ¯∞   ⍝ '∞ ¯∞'

⍝ — Format and execute round-trip both real infinities
⍎⍕∞ ¯∞   ⍝ ∞ ¯∞

⍝ — Exponential formatting keeps the infinity symbol within the field
3 ¯2⍕∞   ⍝ '∞  '

⍝ —
3 2⍕¯∞   ⍝ ' ¯∞'

⍝ —
(10x*1000x)+∞   ⍝ ∞

⍝ — A huge exact numerator divided by infinity is still zero
(10x*1000x)÷∞   ⍝ 0

⍝ — Subtracting a huge finite integer from infinity must not become infinity minus infinity
∞-(10x*1000x)   ⍝ ∞

⍝ — Division by a huge negative finite integer retains negative infinity
∞÷(¯10x*1001x)   ⍝ ¯∞

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
∞×0x
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
!¯2x
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
outer←{offset←{⍶+⍵} ⋄ (2 offset)3} ⋄ outer 0
5.0

⍝ — An array operand captures its value at derivation, not its name
offset←{⍶+⍵} ⋄ a←2 ⋄ kept←a offset ⋄ a←9 ⋄ kept 3
5.0

⍝ — Recursion through del uses the current dfn
fact←{⍵=0:1 ⋄ ⍵×∇⍵-1} ⋄ fact 6   ⍝ 720.0

⍝ — Mutually recursive local functions resolve definitions introduced later
outer←{even←{⍵=0:1 ⋄ odd ⍵-1} ⋄ odd←{⍵=0:0 ⋄ even ⍵-1} ⋄ even ⍵} ⋄ outer 8
1.0

⍝ — A callee's error reaches the caller's guard, which restores the caller's local binding
bad←{1÷⍵} ⋄ guarded←{x←10 ⋄ 0::x ⋄ x←20 ⋄ bad ⍵} ⋄ guarded 0
10.0

⍝ — Guard rollback removes a newly introduced local, revealing the outer name
temp←9 ⋄ guarded←{0::temp ⋄ temp←20 ⋄ 1÷⍵} ⋄ guarded 0
9.0

⍝ — The checkpoint is taken after evaluating the guard's error-code expression
guarded←{x←10 ⋄ (0×(x←20))::x ⋄ x←30 ⋄ 1÷⍵} ⋄ guarded 0
20.0

⍝ — A failing handler is inactive while it runs, allowing the earlier guard to catch it
guarded←{0::7 ⋄ 0::1÷0 ⋄ 1÷⍵} ⋄ guarded 0
7.0

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
≢⍴⊂1 2 3   ⍝ 0x

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
≢,7   ⍝ 1x

⍝ — A scalar has an empty shape vector
⍴7   ⍝ 0⍴0x

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
2.   ⍝ 2.0

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
x←10 ⋄ read←{x} ⋄ caller←{x←99 ⋄ read ⍵} ⋄ caller 0
10

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
(⊂3)+4   ⍝ ⊂7

⍝ —
≢1 2 3   ⍝ 3x

⍝ —
≢¨(1 2 3)(4 5)   ⍝ 3x 2x

⍝ —
+/1 2 3   ⍝ 6

⍝ —
{⍺+⍵}/1 2 3   ⍝ 6

⍝ — A one-item reduction returns the item without calling its operand
f←{1÷0} ⋄ (f/3)(f/⊂3)(f/,3)(f⌿⊂⊂3)(f/⊂2 3)
3 3 3 (⊂3) (2 3)

⍝ —
1 2 3+.×4 5 6   ⍝ 32

⍝ —
{⍵}⍤0⊢1 2 3   ⍝ 1 2 3

⍝ —
+⌿2 3⍴⍳6   ⍝ 5 7 9

⍝ —
+/0⍴(1 2)(3 4)   ⍝ 0 0

⍝ —
↑2 3⍴⍳6   ⍝ 1 2 3

⍝ —
2⊃2 3⍴⍳6   ⍝ 4 5 6

⍝ —
2 3⊃2 3⍴⍳6   ⍝ 6

⍝ —
3⊃2⊃(10 20)(30 40 50)   ⍝ 50

⍝ —
(1 3)˘(2 4)⊃3 4⍴⍳12   ⍝ 2 12

⍝ —
⊃(1 2)(3 4 5)   ⍝ 2 3⍴1 2 0 3 4 5

⍝ —
↑⍬   ⍝ 0

⍝ —
↑0 3⍴0   ⍝ 0 0 0

⍝ —
a←1 2 ⋄ (↑a)←3 4 ⋄ a   ⍝ (3 4)2

⍝ —
a←2 3⍴⍳6 ⋄ (2⊃a)←7 8 9 ⋄ a   ⍝ 2 3⍴1 2 3 7 8 9

⍝ —
a←2 3⍴⍳6 ⋄ (2 3⊃a)←9 ⋄ a   ⍝ 2 3⍴1 2 3 4 5 9

⍝ —
a←(1 2)(3 4) ⋄ (2⊃1⊃a)←9 ⋄ a   ⍝ (1 9)(3 4)

⍝ —
a←3 ⋄ a[]←4 ⋄ a   ⍝ 4

⍝ —
v←3 4 ⋄ (v[1]*2)+v[2]*2   ⍝ 25

⍝ —
(1 3)(2 4)⍳⊂1 3   ⍝ 1x

⍝ —
v←10 20 30 ⋄ v[2] (v[⊂2]) (v[,2])   ⍝ 20 (⊂20) (,20)

⍝ —
a←(1 2)(3 4) ⋄ a[2][1]←9 ⋄ a   ⍝ (1 2)(9 4)

⍝ —
a←(1 2)(3 4) ⋄ a[2],←5 ⋄ a   ⍝ (1 2)(3 4 5)

⍝ —
a←(1 2)(3 4) ⋄ (2⌷a)←5 6 7 ⋄ a   ⍝ (1 2)(5 6 7)

⍝ —
a←(1 2)(3 4) ⋄ (a[2])←5 6 7 ⋄ a   ⍝ (1 2)(5 6 7)

⍝ —
fs←+˘× ⋄ fs[2]←- ⋄ f←fs[2] ⋄ 3 f 2   ⍝ 1

⍝ —
1 3⍳2   ⍝ 3x

⍝ —
1 3⍳⊂2   ⍝ 3x

⍝ —
1 3⍸2   ⍝ 1x

⍝ —
1 3⍸⊂2   ⍝ 1x

⍝ —
2∊1 2 3   ⍝ 1x

⍝ —
(⊂2)∊1 2 3   ⍝ 1x

⍝ —
(1 3 (1=⍸) 0 ⋄ 1 3 (1=⍸) 1 ⋄ 1 3 (1=⍸) 2 ⋄ 1 3 (1=⍸) 3 ⋄ 1 3 (1=⍸) 4)
0x 1x 1x 0x 0x

⍝ —
{⍵∊1 3}¨⍳4   ⍝ 1x 0x 1x 0x

⍝ —
(2 2⍴⍳4)⍳3 4   ⍝ 2x

⍝ —
1 3⍳⍬   ⍝ 0⍴0x

⍝ —
(⌽2)(⍉⊂2)(⍬↑2)(⍬↓⊂2)   ⍝ 2 (⊂2) 2 (⊂2)

⍝ —
2⍷2   ⍝ 1x

⍝⍝ Seeded reduction

⍝ — A reduction seed is the final right argument
10 -/1 2 3   ⍝ ¯8

⍝ — Each matrix lane starts with the same reduction seed
10 +/2 3⍴⍳6   ⍝ 16 25

⍝ — Rank pairs each row with its own seed
10 20 (+/⍤0 1)2 3⍴⍳6   ⍝ 16 35

⍝ — Empty reduction returns the whole seed without calling the operand
(1 2)(3 4) {1÷0}/⍬   ⍝ (1 2)(3 4)

⍝ — Seeded reduction follows a nested path
tree←(10 20)(30 (40 50)) ⋄ tree⊃/⌽2 2 1   ⍝ 40

⍝ — A scalar seed remains distinct from an atom
(⊂10)+/1 2 3   ⍝ ⊂16

⍝ — Whole-seed scan follows a path and retains its intermediate arrays
((10 20)(30 40)) ⊃⍨\1 2   ⍝ (10 20) 20

⍝⍝ Libraries

⍝ — Signed 8-bit integers
•LOAD 'lib/numeric.apl'
8 int 0 127 128 255
⍝ =>
0 127 ¯128 ¯1

⍝ — Normal random array shape
•LOAD 'lib/numeric.apl'
⍴NormRand 2 3
⍝ =>
2x 3x

⍝ — Phinary decoding
•LOAD 'lib/numeric.apl'
1e¯12>|42-phinary '10100010.00100001'
⍝ =>
1x

⍝ — Associative scan along the last axis
•LOAD 'lib/array.apl'
+ascana 2 3⍴1 2 3 4 5 6
⍝ =>
2 3⍴1 3 6 4 9 15

⍝ — Unwrap line breaks
•LOAD 'lib/string.apl'
unwrap 'abc',(•UCS 10),'def'
⍝ =>
'abc def'

⍝ — Parent-first traversal
•LOAD 'lib/tree.apl'
t←1(2(,4)(,5))(,3)
⍬{⍺,↑⍵}trav{1↓⍵}t
⍝ =>
1 2 4 5 3

⍝ — Children-first traversal
•LOAD 'lib/tree.apl'
t←1(2(,4)(,5))(,3)
⍬{⍺,↑⍵}ravt{1↓⍵}t
⍝ =>
4 5 2 3 1
