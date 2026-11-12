⍝ Numeric dfns — adapted for BPL from April
⍝ Source: https://dfns.dyalog.com/n_contents.htm (individual sources below)
⍝ April: libraries/dfns/numeric/numeric.apl; Apache-2.0, see LICENSE-april
_graph←•load "./graph.bpl"

⍝⍝ Ported from http://dfns.dyalog.com/n_contents.htm into April APL

⍝⍝ Whole number processing

⍝ From http://dfns.dyalog.com/c_adic.htm

adic ← {  ⍝ Bijective base-α numeration.
  b←≢a←,α  ⍝ base and alphabet
  1ₓ=ρρω?b⊥≥aιω;
  1ₓ=b?ω#α;  ⍝ unary: special case
  n←⌊b⊛≥ω×≤b  ⍝ number of digits
  z←≤b*n ÷ ≤b  ⍝ smallest integer with length n
  [(n#b ⊤ ω-z)]⌷a
}

⍝ From http://dfns.dyalog.com/c_apportion.htm

apportion ← {  ⍝ Huntington-Hill apportionment.
  α←435ₓ  ⍝ default number of seats
  ω{  ⍝ population per state
    d←√ω×≥ω  ⍝ divisor
    cs←α÷d  ⍝ priority value
    ω+cs=⌈/cs  ⍝ next seat allocation
  }⍣ α-≢ω ,1ₓ  ⍝ iterated per remaining seat.
}

⍝ From http://dfns.dyalog.com/c_bsearch.htm

bsearch ← {  ⍝ Binary search: least n in range ω with ⍶ n.
  ¯1ₓ≤-/ω?1↑ω+2ₓ-+/⍶¨ω;
  mid←⌈0.5×+/ω  ⍝ Mid point:
  ⍶ mid?∇ 1↑ω , mid;
  ∇ mid,1↓ω  ⍝ 0: search upper half.
}

⍝ From http://dfns.dyalog.com/c_cfract.htm

cfract ← {  ⍝ Continued fraction approximation of real ω.
  ,{
    ω=0ₓ?⍬;
    [n r]←[0ₓ ω]⊤α  ⍝ next term and remainder.
    n,ω∇r  ⍝ next term and cf of remainder.
  }/⌊1r10¹⁴+[ω 1ₓ]÷1ₓ∨ω
}

⍝ From http://dfns.dyalog.com/c_colsum.htm

colsum ← {  ⍝ Sum of (default decimal) columns.
  α←10ₓ ◇ α{{0ₓ=↑ω ↓ ω}+⌿1 0⌽0ₓ,⍉[0ₓ α]⊤ω}⍣≡+⌿ω  ⍝ repeat while overflow.
}

⍝ From http://dfns.dyalog.com/c_efract.htm

efract ← {  ⍝ Egyptian fractions: Fibonacci-Sylvester algorithm
  ⍬{
    [p q]←ω÷∨/ω
    p=1ₓ?α,q;
    r←p|q ◇ s←q-r ÷ p
    α,≥s ∇ [p-r q×≥s]
  }[α ω]
}

⍝ From http://dfns.dyalog.com/c_factorial.htm

factorial←{α←1ₓ ◇ ω=0ₓ?α;α×ω ∇ ≤ω}  ⍝ Tail recursive factorial.

⍝ From http://dfns.dyalog.com/c_fibonacci.htm

fibonacci ← { α←[0 1]ₓ  ⍝ Tail-recursive Fibonacci.
  ω=0ₓ?↑α;1↓α,+/α ∇ ≤ω
}

sulFib ← {  ⍝ Sullivan Fibonacci
  z←0.5×≥s←√5
  (z*ω - 2○πω × z*-ω)÷s
}

⍝ From http://dfns.dyalog.com/c_factors.htm

factors ← { ω{  ⍝ Prime factors of ω.
    ω , α÷×/ω ~ [1ₓ]  ⍝ append factor > sqrt(ω).
  }∊ω{  ⍝ concatenated,
    (0ₓ = ω*≥ι⌊ω⊛α | α)#ω  ⍝ powers of each prime factor.
  }¨⍬{  ⍝ remove multiples:
    nxt←↑ω
    msk←0ₓ≠nxt|ω  ⍝ ... mask of non-multiples.
    ∧/1↓msk?α,ω;  ⍝ all non multiples - finished.
    α,nxt ∇ msk#ω  ⍝ sieve remainder.
  }ω{  ⍝ from,
    0ₓ=ω|α # ω  ⍝ divisors of ω in:
  }2ₓ,(3ₓ+⊽ι⌊0.5×√ω),ω  ⍝ 2,3 5 .. sqrt(ω),ω
}

⍝ From http://dfns.dyalog.com/c_k6174.htm

k_6174 ← {  ⍝ Kaprekar's operation.
  enco←4#10ₓ ⊤  ⍝ 4-digit encode.
  deco←enco⁻  ⍝ and decode.
  1ₓ=ρ∪encoω?"error";  ⍝ all digits the same: no go.
  ⍬{  ⍝ starting with null sequence.
    ω=↑⌽α?α;
    v←>enco ω  ⍝ digits in descending order.
    α,ω ∇ deco⁝v - deco⌽v  ⍝ smaller to larger difference.
  }ω  ⍝ :: [#] ∇ # → [#]
}

⍝ From http://dfns.dyalog.com/c_hex.htm

hex ← {  ⍝ Hexadecimal from decimal.
  1ₓ≠≡,ω?α∇¨ω;  ⍝ simple-array-wise:
  0∊,ω-≥ω?"Too big";
  n←↑α,2*⌈2⊛2⌈16⊛≥⌈/|ω  ⍝ default width.
  ↓[(n#16ₓ ⊤ ω)]⌷"0123456789abcdef"
}

⍝ From http://dfns.dyalog.com/c_dec.htm

dec ← {  ⍝ Decimal from hexadecimal
  α←$f  ⍝ unsigned by default.
  1ₓ<ρρω?α↣∇⍤1 ω;  ⍝ vector-wise:
  0ₓ≡≢ω?0ₓ;  ⍝ dec"" → 0.
  1ₓ≠≡,ω?α∇¨ω;  ⍝ simple-array-wise:
  ws←∊↢(•ucs 9 10 13 32 133 160)
  ws↑ω?α ∇ 1↓ω;
  ws↑⌽ω?α ∇ ¯1↓ω;
  ∨/ws ω?α∇¨ ≥wsω ⊆ ω;  ⍝ white-space-separated:
  v←16ₓ|"0123456789abcdef0123456789ABCDEF"ιω
  16ₓ⊥v - α × 8ₓ≤↑v × 16ₓ*≢v
}

⍝ From http://dfns.dyalog.com/c_int.htm

int ← {m←2ₓ*≤α ◇ (⊽m | m+ω)-m}  ⍝ Signed from unsigned integer.

⍝ From http://dfns.dyalog.com/c_uns.htm

uns ← {2ₓ*α | ω}  ⍝ Unsigned from signed integer.

⍝ From http://dfns.dyalog.com/c_nicediv.htm

nicediv ← {  ⍝ ω similar integers with sum α.
  q←ωρ⌊α÷ω  ⍝ quotient.
  d←+\ ω|α ÷ₐ ωρω  ⍝ residue spread in ω decimal steps.
  i←</↕0ₓ,⌊0.5+d
  q+i
}

⍝ From http://dfns.dyalog.com/s_nicediv.htm

stack ← {⍉⊃ αnicedivω #¨ '⎕'}

osc ← {1ₓ=ω?1ₓ;2ₓ|ω?∇ ≥3ₓ×ω;∇ ω÷2ₓ}  ⍝ Oscillate - probably returns 1.

⍝ From http://dfns.dyalog.com/c_rational.htm

rational ← {  ⍝ Rational approximation to real ω.
  ⊃[ω 1ₓ]÷⊂1ₓ∨ω
}

⍝ From http://dfns.dyalog.com/c_roman.htm

roman ← {  ⍝ Roman numeral arithmetic.
  num←{{ω+.××0.5+×-/↕ω,0ₓ}[7ₓ|"IVXLCDMivxlcdm"ιω]⌷,⍉[1 5]ₓ×⊗10ₓ*ι4ₓ}
  fmt←{~↢" ",0 1 0⍉(⊂[ω⊤⍨4ρ10]⌷0 4 2 2⊤0 16 20 22 24 32 36 38 39 28)⌷' '⍪3 4ρ"MCXI DLV "}
  depth←{⍹≥≡ω?⍶ ω;∇¨ω}
  nums←num depth 1ₓ  ⍝ arabic from roman.
  fmts←fmt depth 0ₓ  ⍝ roman from arabic.
  fmts ⌊numsα ⍶ ⌊numsω
}

⍝ From http://dfns.dyalog.com/c_stamps.htm

stamps ← {  ⍝ Postage stamps to the value of ω.
  α←[1 5 6 10 26 39 43]ₓ  ⍝ Default UK stamp denominations.
  graph←α{ω↣∩¨ω+⊂α}ιω+|⌊/α  ⍝ values: 0 ·· ω.
  spath←graph _graph.path [0ₓ ω]
  -/⌽↕spath
}

⍝ From http://dfns.dyalog.com/c_sieve.htm

sieve ← {  ⍝ Sieve of Eratosthenes.
  α←⍬  ⍝ Default no primes yet.
  nxt←1↑ω  ⍝ Next prime, and
  msk←0ₓ≠nxt|ω  ⍝ ... mask of non-multiples.
  ∧/1↓msk?α,ω;  ⍝ All non multiples - finished.
  α,nxt ∇ msk#ω  ⍝ Sieve remainder.
}

⍝ From http://dfns.dyalog.com/c_to.htm

to ← {  ⍝ Sequence α .. ω
  [from step]←[1 ¯1]ₓ×-\2↑α,α+×ω-α  ⍝ step default is +/- 1.
  from+step×ι0ₓ⌈≥⌊(ω-from)÷step+step=0ₓ
}

⍝ From http://dfns.dyalog.com/s_to.htm

xTo ← {  ⍝ Sequence α .. ω
  [from step]←⊂¨[1 ¯1]ₓ×-\2↑α,α+×ω-α  ⍝ step default is +/- 1.
  size←0ₓ⌈≥⌊⊃(ω-from)÷step+step=0ₓ  ⍝ shape of result
  from+step×ιsize
}


⍝⍝ Real number processing

⍝ From http://dfns.dyalog.com/n_abc.htm

rg ← {α₀⍶ω ∧ α₁⍹ω}  ⍝ Range operator

xr ← {d←⍉⊃+/×ω,.-αᵀ ◇ (2ₓ∨.=|d #₋₁ d)←2ₓ ◇ 3ₓ⊥dᵀ}  ⍝ Outside location

⍝ From http://dfns.dyalog.com/c_alt.htm

alt ← {  ⍝ Alternant.
  [r c]←ρω  ⍝ matrix ω
  0ₓ=r?⍹⌿,ω;
  1ₓ≥c?⍶⌿,ω;
  M←(ιr)~¹⍪ιr  ⍝ minors
  ω.[: 0]⍶.⍹∇¨⊂₁ ₂[M 1↓ιc]⌷ω
}

bayes ← {α ×÷+.× ω}  ⍝ Bayes' formula. (implemented as a fork)

⍝ From http://dfns.dyalog.com/c_Cholesky.htm

Cholesky ← {  ⍝ decomposition of a Hermitian positive-definite matrix.
  1ₓ≥n←≢ω?√ω;
  q←n÷2ₓ
  p←n-q
  X←[p p]↑ω ◇ Y←[p -q]↑ω ◇ Z←(-q,q)↑ω
  L_0←∇ X
  L_1←∇ Z-(TT←+Yᵀ +.× ⌹X)+.×Y
  [p n]↑L_0 ⍪ TT+.×L_0 , L_1
}

⍝ From http://dfns.dyalog.com/c_det.htm

det ← {  ⍝ Determinant of square matrix.
  α←1ₓ  ⍝ product of co-factor coefficients so far
  0ₓ=n←≢ω?α;  ⍝ result for 0-by-0
  [i j]←ρω ⊤ ↑⍒|,ω
  k←ιn
  α×ω.[i j]×¯1ₓ*i+j ∇ [k~[i] k~[j]]⌷ω - [k~[i] j]⌷ω ×⊗ [i k~[j]]⌷ω ÷ [i j]⌷ω
}

⍝ From http://dfns.dyalog.com/c_gauss_jordan.htm

gauss_jordan ← {  ⍝ Gauss-Jordan elimination.
  elim←{  ⍝ elimination of row/col α
    p←α+↑⍒|α↓[: α]⌷ω
    swap←⊖@[α p] ω  ⍝ αth and pth rows exchanged
    mat←swap.[α α]÷⍨@α swap  ⍝ col diagonal reduced to 1
    mat - mat.[: α]×α≠ι≢ω ×⊗ α⌷mat
  }
  α←=/⊃ιρω
  ρα ρ 0 1×ρω ↓ (ω,α)elim/⌽ι⌊/ρω
}

⍝ From http://dfns.dyalog.com/s_gauss_jordan.htm

tryGJ←{[α⌹ω αgauss_jordanω]}  ⍝ gauss_jordan vs primitive ⌹.

⍝ From http://dfns.dyalog.com/s_gauss_jordan.htm

hil ← {÷1ₓ+ +⊗⍨ιω}  ⍝ order ω Hilbert matrix.

⍝ From http://dfns.dyalog.com/c_kcell.htm

kcell ← {  ⍝ Relationship between point and k-cell.
  α←≢ω #₋₁ [2 1]ρ0 1  ⍝ Default is unit k-cell.
  b←,⍠(2ₓ=ρρα # 0)α
  p←(2⌊ρρα ↓ 1 1,ρω)ρω  ⍝ Points to evaluate.
  d←,⍶⌿ ¹ ⍉×-b,.-p
  ⍶/⍬?⌈/d;5ₓ⊥d
}

⍝ From http://dfns.dyalog.com/c_kball.htm

kball ← { α←1  ⍝ Relationship between point and k-ball.
  r←↑α ◇ p←1#ω
  c←≢p ↑ 1↓α  ⍝ Remaining coordinates are center.
  ×-/[⍉p-₀c;r]+.*¨2ₓ
}

⍝ ⍝ From http://dfns.dyalog.com/c_ksphere.htm

ksphere ← {  ⍝ Surface area of k-sphere.
  n←≥α  ⍝ dimension of enclosed k-ball.
  pi←π1 * n÷2
  n × ω*α × pi÷!n÷2  ⍝ k-sphere surface area.
}

⍝ From http://dfns.dyalog.com/s_ksphere.htm

kvol ← {ω×(≤α ksphere ω)÷α}

⍝ From http://dfns.dyalog.com/s_mean.htm

stdev ← {√(+/÷≢ ω²)-(+/÷≢ ω)²}  ⍝ Standard deviation.

⍝ From http://dfns.dyalog.com/c_NormRand.htm

NormRand ← {                                 ⍝ Random numbers with a normal distribution
  depth←10*9                                 ⍝ randomness depth
  [x y]←⊂⁻¹ ≥¿depthρ⍨2,ω ÷ depth          ⍝ two random variables within ]0;1]
  (¯2×⊛x * 0.5)×1○π2×y                       ⍝ Box-Muller distribution
}

⍝ From http://dfns.dyalog.com/c_phinary.htm
phinary ← {  ⍝ Phinary representation; left argument 0 returns exponents.
  α←1
  Ø←≥√5 ÷ 2
  ""≡0#∊ω?{
    1ₓ<≡ω?∇¨ω;
    '¯'=↑ω?-∇ 1↓ω;
    a←Ø⊥$dιω~"."
    a÷Ø*(≢ω∪'.')-≥(,ω)ι'.'
  }ω;
  0ₓ≠≡ω?α∇¨ω;
  ω<0?'¯',α∇-ω;
  num←ω
  α{
    α=0?ω;
    ω≡⍬?"0";
    fmt←{[ω]⌷"01"}
    lft←⌽ι0ₓ⌈≥⌈/ω ∊ ω
    rgt←-≥ι0ₓ⌈|⌊/ω ∊ ω
    rgt∧.=$f?fmt lft;
    lft∧.=$f?"0.",fmt rgt;
    fmt⁝lft , '.' , fmt⁝rgt
  }⍬ₓ{
    num=Ø+.*α?α;
    ∆←-ρα ↑ 1ₓ
    num=Ø+.*α+∆?α+∆;
    k←⌊Ø⊛ω
    α,k ∇ ω-Ø*k
  }ω
}

⍝ From http://dfns.dyalog.com/s_phinary.htm

align←{p←{ωι'.'}¨ω ◇ ⊃(⌈/p - p){αρ' ' , ω}¨ω}

⍝ From http://dfns.dyalog.com/c_roots.htm

realroots ← {  ⍝ Real roots of quadratic.
  [a b c]←ω  ⍝ Coefficients.
  d←b²-4ₓ×a×c  ⍝ Discriminant.
  d<0ₓ?⍬;  ⍝ No roots
  d=0ₓ?-b÷2ₓ×a;  ⍝ One root
  -b+[¯1 1]ₓ×√d ÷ 2ₓ×a  ⍝ Two roots
}

⍝ From http://dfns.dyalog.com/s_roots.htm

roots ← {  ⍝ Roots of quadratic.
  [a b c]←ω  ⍝ coefficients.
  d←b²-4ₓ×a×c  ⍝ discriminant.
  -b+[¯1 1]ₓ×√d ÷ 2ₓ×a  ⍝ both roots.
}


⍝⍝ Complex number processing

⍝ From http://dfns.dyalog.com/c_polar.htm

polar ← {  ⍝ Polar from/to cartesian coordinates.
  pol_car←{  ⍝ polar from cartesian (default).
    radius←{√+⌿ω²}  ⍝ radius (pythagorus).
    angle←{  ⍝ phase angle.
      [x y]←⊂⁻¹ ω  ⍝ x and y coordinates.
      [on off]←$t $f=⊂0=x  ⍝ points on/off y axis.
      atan←3○⁻y÷x+on  ⍝ arctan y÷x (avoiding y÷0).
      qne←off×atan + on×π0.5×2-×y
      nsw←πx<0
      qse←π2× x>0 ∧ y<0
      nsw+qse+qne  ⍝ all quadrants.
    }
    radiusω lam angleω  ⍝ (2,···) array of polar coordinates.
  }
  car_pol←{  ⍝ cartesian from polar.
    [r o]←⊂⁻¹ ω  ⍝ radius and phase angle.
    r×2○o lam r×1○o  ⍝ r×cos(ø), r×sin(ø).
  }
  lam←{⊃[α ω]}
  α←1ₓ  ⍝ default polar from cartesian.
  α=+1ₓ?pol_car ω;  ⍝ polar from cartesian.
  α=-1ₓ?car_pol ω  ⍝ cartesian from polar.
}

⍝ From http://dfns.dyalog.com/s_polar.htm

rnd ← {10*-α × ⌊0.5+ω×10*α}

poly ← {2 1 ○⊗ π2÷ω × ιω}

⍝ From http://dfns.dyalog.com/c_xtimes.htm

xtimes ← { m←0  ⍝ Fast multi-digit product using FFT.
  xroots    ← {×\1,1↓(ω÷2)ρ¯1*2÷ω}
  cube      ← {ωρ⍨2ρ⍨⌊2⊛ρω}
  extend    ← {2*⌈2⊛≤α+⍥ρω ↑¨ [α ω]}
  floop     ← {(⊣/α)∇⍣(×m) (+⌿ω){⊃[α ω]}⍤(≢ρω - m+1) α×⍠(ιm←≢ρα)-⌿ω}
  FFT       ← {, cube⁝xrootsρω floop cubeω}
  iFFT      ← {ρω ÷⍨ , cube+xrootsρω floop cubeω}
  rconvolve ← {≤α+⍥ρω ↑ iFFT×/FFT¨αextendω}
  carry     ← {1↓+⌿1 0⌽0ₓ,⍉[0 10]ₓ⊤ω}
  (+/∧\0ₓ=t)↓t←carry⍣≡0ₓ,⌊0.5+⊣/∨αrconvolveω
}

convolve ← {+⌿ -ιρα ⌽ α×⊗ω,0ₓ×1↓α}

⍝ From http://dfns.dyalog.com/c_xpower.htm

xpower ← {  ⍝ Fast multi-digit power using FFT.
  xt←{0ₓ,α xtimes 0ₓ,ω} ◇ b←⌽⊤10ₓ⊥ω  ⍝ boolean showing which powers needed
  ⊃,/xt/b#{xt⍨α}\(⊂,10ₓ⊥⁻α)ρ⍨ρb
}
