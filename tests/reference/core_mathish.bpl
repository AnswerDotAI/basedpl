⍝⍝ Mathish semantics

⍝ — A prime index accepts a float within comparison tolerance of an integer
⍭1.000000000000001   ⍝ 3ₓ

⍝ logical-numeric-truth — a float stands for a Boolean within tolerance only under Mathish
n←1-1e¯15 ◇ on←[~[0 n];(1+1e¯15)⊼n;0⊽n]
•prefs ["mathish":$f] ◇ off←{"DOMAIN"::1 ◇ ~n ◇ 0}0 ◇ •prefs ["mathish":$t] ◇ [on;off]
⍝ =>
[[[$t $f] $f $f] 1]

⍝ — Mathish division gives zero for zero divided by zero in arrays
0 1÷0 2   ⍝ 0 0.5

⍝ — A character offset accepts a float within comparison tolerance of an integer
'a'+1.000000000000001   ⍝ 'b'

⍝ — Complex gcd uses Gaussian-integer arithmetic and tolerant comparison
0.2=3.8j7.6∨5.2j6.8   ⍝ $t

⍝ — Scale and ceil to expose the complex lcm despite floating-point roundoff
⌈1000×3.8j7.6∧5.2j6.8   ⍝ ¯159600j326800

⍝ — Floor applies comparison tolerance near integers, including across a storage-width boundary
[⌊1.000000000000001 ¯1.000000000000001;⌊255.9999999999999 0]   ⍝ [[1 ¯1]ₓ [256 0]ₓ]

⍝ — An integer argument accepts a float within comparison tolerance of an integer
(0.1×3 × 10)⌷10 20 30 40   ⍝ 40

⍝ — A count accepts a float within comparison tolerance of an integer
ι(0.1×3)×10   ⍝ 0 1 2

⍝ — The gcd of two reals recovers their rational ratio, in either order [rtol=1e-14 atol=0]
x←[1 1ₓ ¯1 0 0 1 1E¯200 1E200 1E¯200]
y←[103993÷33102 π1 ¯103993÷33102 ¯0.75 0 1.000000000000005 3E¯200 3E200 1E200]
[x∨y y∨x]
⍝ =>
2ρ⊂[÷33102 ÷1725033 ÷33102 0.75 0 1 1E¯200 1E200 1E¯200]

⍝ — Reals outside comparison tolerance of a rational ratio have a tiny gcd
1E¯12>1∨1.00000000000005   ⍝ $t

⍝ — Index-of compares each candidate directly; tolerance is not transitive
x←1
y←1+8E¯15
z←1+16E¯15
[x y]ιz   ⍝ 1ₓ

⍝ — Unique-mask puts a chain of tolerant matches in one class
≠[1 1+8E¯15 1+16E¯15]   ⍝ $t $f $f

⍝ — A row is a copy of the first earlier distinct row that it matches, so rows don't chain
≠[1 2◇1+8E¯15 2◇1+16E¯15 2]   ⍝ $t $f $t

⍝ — A large index-of takes the first position within tolerance, not the nearest value
(300ρ5 , [1+16E¯15 1])ι1+8E¯15   ⍝ 300ₓ

⍝ — A large index-of finds nothing outside tolerance
(300ρ5 , 1)ι1+2E¯14   ⍝ 301ₓ

⍝ — Large membership compares exact integers with floats within tolerance
[1+8E¯15 1+2E¯14]∊ι300ₓ   ⍝ $t $f

⍝ — Without uses tolerance when approximate values participate
1 2~[1+8E¯15]   ⍝ [2]

⍝ — Membership uses tolerance when approximate values participate
1 2∊[1+8E¯15]   ⍝ $t $f

⍝ native-search-shapes — Compact and mixed patterns share shape traversal; fractional needles keep tolerant matching
p←[1 [2 3]] ◇ v←[1 [2 3] 0 1 [2 3]]
[p⍷v [1 2]⍷[1 2 1◇1 2 3] [1 2 3]ₓι[2 2.000000000000001]]
⍝ =>
[$t $f $f $t $f;[$t $f $f◇$t $f $f];[1 1]ₓ]

⍝ native-find-widths — Compact float patterns use the wider operand's tolerance and match NaNs
[(32•float[1])⍷[1+8E¯15] (32•float[1])⍷32•float[1+8E¯7] [$n]⍷[$n 0]]
⍝ =>
[[$t] [$f] [$t $f]]

⍝ exact-key-search — A search gives the same positions whether every cell is an exact key or one float cell sends them all through `Value::matches`, and for strings held as items or as a matrix's rows
h←["ab";"cd";"";7ₓ;'x';[3 4]ₓ;"abc";1ₓ] ◇ ns←32ρ["cd";"zz";"";7ₓ;'x';[3 4]ₓ;"ab";'y';"abc";$t]
s←["ab";"cd";"ef";"gh";"ij";"kl";"mn";"op"] ◇ t←32ρs,⊂"zz"
[(ns∊h)≡ns∊h,⊂0.5 ; (≠h,ns)≡¯1↓≠h,ns,⊂0.5 ; (hιns)≡8⌊(h,⊂0.5)ιns ; (sιt)≡(⊃s)ι⊃t ; 10↑hιns ; hι1 1+1E¯15]
⍝ =>
[$t;$t;$t;$t;[1 8 2 3 4 5 0 8 6 7]ₓ;[7 7]ₓ]

⍝ — Interval index in each cell's own boundaries, and index of within the tolerance
[({ω⍸2.5}⍤1)[2 3]ρ1 2 3 0 5 10 ; ({ωι0.3}⍤1)[2 3]ρ0.1 0.2 0.30000000000000004 0.3 0.1 0.2]   ⍝ [[2 1]ₓ ; [2 0]ₓ]

⍝ — `≡` compares a batch's exact-integer or Boolean cells with a float constant on the batch's whole, within the tolerance [batched]
m←3 2ρι6ₓ ◇ b←2 2ρ$t $f $f $t ◇ c←2 3.0000000000000004
[{ω≡c}⍤1 m ; {ω≡1 0.0}⍤1 b ; (•explain "{ω≡c}⍤1 m").fallbacks.reason]   ⍝ [[$f $t $f] [$t $f] 0ρ⊂""]

⍝ — Tolerant Key puts a chain of matches in one group
{≢ω}⌸[1 1+8E¯15 1+16E¯15]   ⍝ [3]ₓ

⍝ — Complex equality compares the magnitude of the difference within tolerance, in either order
x←1j1 1j1 1ₓ 1 0 1.7E308j1.7E308 1.7E308j1.7E308
y←1j1.000000000000012 1j1.00000000000002 1j5E¯15 1j5E¯14 0j1E¯100 1.7E308j1.7E308 ¯1.7E308j¯1.7E308
[x=y y=x x≠y y≠x]
⍝ =>
[[$t $f $t $f $f $t $f] [$t $f $t $f $f $t $f] [$f $t $f $t $t $f $t] [$f $t $f $t $t $f $t]]

⍝ — Approximate equality uses relative tolerance 1E¯14. https://docs.dyalog.com/20.0/language-reference-guide/system-functions/ct/
0.3=[0.3 0.1+0.2 0.4]   ⍝ $t $t $f

⍝ — Tolerant equality is not transitive: x=y and y=z need not imply x=z
[1=1+8E¯15 (1+8E¯15)=1+16E¯15 1=1+16E¯15]   ⍝ $t $t $f

⍝ — Comparisons apply tolerance only when an approximate number takes part
x←[0.3 0.1+0.2 ¯0.3 1 1 1E8 1E8 0 1E¯100 1E308 1r3 1r3 9007199254740993ₓ 1ₓ 1 1r3]
y←[0.1+0.2 0.3 ¯0.1-0.2 1+5E¯15 1+5E¯14 1E8+5E¯7 1E8+5E¯5 1E¯100 0 ¯1E308 2r6 1r2 9007199254740992ₓ 1000000000000001r1000000000000000 1000000000000001r1000000000000000 1÷3]
[x=y◇x≠y◇x<y◇x≤y◇x>y◇x≥y]
⍝ =>
[$t $t $t $t $f $t $f $f $f $f $t $f $f $f $t $t
 $f $f $f $f $t $f $t $t $t $t $f $t $t $t $f $f
 $f $f $f $f $t $f $t $t $f $f $f $t $f $t $f $f
 $t $t $t $t $t $t $t $t $f $f $t $t $f $t $t $t
 $f $f $f $f $f $f $f $f $t $t $f $f $t $f $f $f
 $t $t $t $t $f $t $f $f $t $t $t $f $t $f $t $t]

⍝ — Tolerance makes the whole float 2E14 equal the integers beside it, as a float comparison does
(200000000000000+ι3ₓ)=200000000000000   ⍝ $t $t $t

⍝ — Under Mathish mode, a residue is 0 where the divisor times the floor of the quotient equals the item within tolerance, as 0.1 times 3 and 10 do here
0.1|0.3 1   ⍝ 0 0

⍝ — Mathish division leaves other indeterminate arithmetic unchanged
[0÷0 ∞-∞ 0×∞ ∞×0ₓ ∞÷∞]   ⍝ [0 $n $n $n $n]

⍝ —
0÷0   ⍝ 0

⍝ — Exact numbers that `÷ₐ` divides as 64-bit floats take Mathish's zero for 0÷0. Integer floor division gives 0 whether Mathish is on or off
[0ₓ÷ₐ0ₓ ; 0ₓ÷0ₓ]
[0 0ₓ]

⍝ — Mathish division follows the array and fused routes
(0÷ι8)+0   ⍝ 8ρ0

⍝ —
0÷¯0   ⍝ 0

⍝ subscript-fixed — After a function, ₓ₁, ₓ₂, ₓ₄ or ₓ₈ converts its results to fixed integers of that width. A float takes its floor and then clamps to the type's range: NaN gives 0, and an infinity gives the nearer end of the range. Under Mathish mode the floor is tolerant: 2.9999999999999996 gives 3, where it gives 2 with Mathish mode off. An integer wraps to the width, and a Boolean gives 0 or 1
v←0.5 1.5 ¯0.1 2.9999999999999996 ¯3.5 300.5 $n ∞ ¯∞ ◇ a←[⊢ₓ₁ v ; ⊢ₓ₂ v ; ⊢ₓ₁ [255 256 ¯1 ¯257]ₓ ; ⊢ₓ₂ 0≠[1 0 1]ₓ] ◇ •prefs ["mathish":$f] ◇ a,⊂(⊢ₓ₂ v)
[[0 1 0 3 0 255 0 255 0]ₓ₁ ; [0 1 ¯1 3 ¯4 300 0 32767 ¯32768]ₓ₂ ; [255 0 255 255]ₓ₁ ; [1 0 1]ₓ₂ ; [0 1 ¯1 2 ¯4 300 0 32767 ¯32768]ₓ₂]

⍝ — The number of steps is whole within tolerance
0↦0.3÷0.1   ⍝ 0 1 2 3

