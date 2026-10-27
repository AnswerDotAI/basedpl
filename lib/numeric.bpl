⍝ Numeric dfns — adapted for BPL from April
⍝ Source: https://dfns.dyalog.com/n_contents.htm (individual sources below)
⍝ April: libraries/dfns/numeric/numeric.apl; Apache-2.0, see LICENSE-april
_graph←•load "./graph.bpl"

⍝⍝ Ported from http://dfns.dyalog.com/n_contents.htm into April APL

⍝⍝ Whole number processing

⍝ From http://dfns.dyalog.com/c_adic.htm

adic ← {  ⍝ Bijective base-⍺ numeration.
  b←≢a←,⍺  ⍝ base and alphabet
  1ₓ=⍴⍴⍵?b⊥≥a⍳⍵;
  1ₓ=b?⍵#⍺;  ⍝ unary: special case
  n←⌊b⍟≥⍵×≤b  ⍝ number of digits
  z←≤b*n ÷ ≤b  ⍝ smallest integer with length n
  [(n#b ⊤ ⍵-z)]⌷a
}

⍝ From http://dfns.dyalog.com/c_apportion.htm

apportion ← {  ⍝ Huntington-Hill apportionment.
  ⍺←435ₓ  ⍝ default number of seats
  ⍵{  ⍝ population per state
    d←√⍵×≥⍵  ⍝ divisor
    cs←⍺÷d  ⍝ priority value
    ⍵+cs=⌈/cs  ⍝ next seat allocation
  }⍣ ⍺-≢⍵ ,1ₓ  ⍝ iterated per remaining seat.
}

⍝ From http://dfns.dyalog.com/c_bsearch.htm

bsearch ← {  ⍝ Binary search: least n in range ⍵ with ⍶ n.
  ¯1ₓ≤-/⍵?1↑⍵+2ₓ-+/⍶¨⍵;
  mid←⌈0.5×+/⍵  ⍝ Mid point:
  ⍶ mid?∇ 1↑⍵ , mid;
  ∇ mid,1↓⍵  ⍝ 0: search upper half.
}

⍝ From http://dfns.dyalog.com/c_cfract.htm

cfract ← {  ⍝ Continued fraction approximation of real ⍵.
  ,{
    ⍵=0ₓ?⍬;
    [n r]←[0ₓ ⍵]⊤⍺  ⍝ next term and remainder.
    n,⍵∇r  ⍝ next term and cf of remainder.
  }/⌊1r10¹⁴+[⍵ 1ₓ]÷1ₓ∨⍵
}

⍝ From http://dfns.dyalog.com/c_colsum.htm

colsum ← {  ⍝ Sum of (default decimal) columns.
  ⍺←10ₓ ⋄ ⍺{{0ₓ=↑⍵ ↓ ⍵}+⌿1 0⌽0ₓ,⍉[0ₓ ⍺]⊤⍵}⍣≡+⌿⍵  ⍝ repeat while overflow.
}

⍝ From http://dfns.dyalog.com/c_efract.htm

efract ← {  ⍝ Egyptian fractions: Fibonacci-Sylvester algorithm
  ⍬{
    [p q]←⍵÷∨/⍵
    p=1ₓ?⍺,q;
    r←p|q ⋄ s←q-r ÷ p
    ⍺,≥s ∇ [p-r q×≥s]
  }[⍺ ⍵]
}

⍝ From http://dfns.dyalog.com/c_factorial.htm

factorial←{⍺←1ₓ ⋄ ⍵=0ₓ?⍺;⍺×⍵ ∇ ≤⍵}  ⍝ Tail recursive factorial.

⍝ From http://dfns.dyalog.com/c_fibonacci.htm

fibonacci ← { ⍺←[0 1]ₓ  ⍝ Tail-recursive Fibonacci.
  ⍵=0ₓ?↑⍺;1↓⍺,+/⍺ ∇ ≤⍵
}

sulFib ← {  ⍝ Sullivan Fibonacci
  z←0.5×≥s←√5
  (z*⍵ - 2○π⍵ × z*-⍵)÷s
}

⍝ From http://dfns.dyalog.com/c_factors.htm

factors ← { ⍵{  ⍝ Prime factors of ⍵.
    ⍵ , ⍺÷×/⍵ ~ [1ₓ]  ⍝ append factor > sqrt(⍵).
  }∊⍵{  ⍝ concatenated,
    (0ₓ = ⍵*≥⍳⌊⍵⍟⍺ | ⍺)#⍵  ⍝ powers of each prime factor.
  }¨⍬{  ⍝ remove multiples:
    nxt←↑⍵
    msk←0ₓ≠nxt|⍵  ⍝ ... mask of non-multiples.
    ∧/1↓msk?⍺,⍵;  ⍝ all non multiples - finished.
    ⍺,nxt ∇ msk#⍵  ⍝ sieve remainder.
  }⍵{  ⍝ from,
    0ₓ=⍵|⍺ # ⍵  ⍝ divisors of ⍵ in:
  }2ₓ,(3ₓ+⊽⍳⌊0.5×√⍵),⍵  ⍝ 2,3 5 .. sqrt(⍵),⍵
}

⍝ From http://dfns.dyalog.com/c_k6174.htm

k_6174 ← {  ⍝ Kaprekar's operation.
  enco←4#10ₓ ⊤  ⍝ 4-digit encode.
  deco←enco⁻¹  ⍝ and decode.
  1ₓ=⍴∪enco⍵?"error";  ⍝ all digits the same: no go.
  ⍬{  ⍝ starting with null sequence.
    ⍵=↑⌽⍺?⍺;
    v←>enco ⍵  ⍝ digits in descending order.
    ⍺,⍵ ∇ deco⁝v - deco⌽v  ⍝ smaller to larger difference.
  }⍵  ⍝ :: [#] ∇ # → [#]
}

⍝ From http://dfns.dyalog.com/c_hex.htm

hex ← {  ⍝ Hexadecimal from decimal.
  1ₓ≠≡,⍵?⍺∇¨⍵;  ⍝ simple-array-wise:
  0∊,⍵-≥⍵?"Too big";
  n←↑⍺,2*⌈2⍟2⌈16⍟≥⌈/|⍵  ⍝ default width.
  ↓[(n#16ₓ ⊤ ⍵)]⌷"0123456789abcdef"
}

⍝ From http://dfns.dyalog.com/c_dec.htm

dec ← {  ⍝ Decimal from hexadecimal
  ⍺←$f  ⍝ unsigned by default.
  1ₓ<⍴⍴⍵?⍺↣∇⍤1 ⍵;  ⍝ vector-wise:
  0ₓ≡≢⍵?0ₓ;  ⍝ dec"" → 0.
  1ₓ≠≡,⍵?⍺∇¨⍵;  ⍝ simple-array-wise:
  ws←∊↢(•ucs 9 10 13 32 133 160)
  ws↑⍵?⍺ ∇ 1↓⍵;
  ws↑⌽⍵?⍺ ∇ ¯1↓⍵;
  ∨/ws ⍵?⍺∇¨ ≥ws⍵ ⊆ ⍵;  ⍝ white-space-separated:
  v←16ₓ|"0123456789abcdef0123456789ABCDEF"⍳⍵
  16ₓ⊥v - ⍺ × 8ₓ≤↑v × 16ₓ*≢v
}

⍝ From http://dfns.dyalog.com/c_int.htm

int ← {m←2ₓ*≤⍺ ⋄ (⊽m | m+⍵)-m}  ⍝ Signed from unsigned integer.

⍝ From http://dfns.dyalog.com/c_uns.htm

uns ← {2ₓ*⍺ | ⍵}  ⍝ Unsigned from signed integer.

⍝ From http://dfns.dyalog.com/c_nicediv.htm

nicediv ← {  ⍝ ⍵ similar integers with sum ⍺.
  q←⍵⍴⌊⍺÷⍵  ⍝ quotient.
  d←+\ ⍵|⍺ ÷ ⍵⍴⍵  ⍝ residue spread in ⍵ decimal steps.
  i←</↕0ₓ,⌊0.5+d
  q+i
}

⍝ From http://dfns.dyalog.com/s_nicediv.htm

stack ← {⍉⊃ ⍺nicediv⍵ #¨ '⎕'}

osc ← {1ₓ=⍵?1ₓ;2ₓ|⍵?∇ ≥3ₓ×⍵;∇ ⍵÷2ₓ}  ⍝ Oscillate - probably returns 1.

⍝ From http://dfns.dyalog.com/c_rational.htm

rational ← {  ⍝ Rational approximation to real ⍵.
  ⊃[⍵ 1ₓ]÷⊂1ₓ∨⍵
}

⍝ From http://dfns.dyalog.com/c_roman.htm

roman ← {  ⍝ Roman numeral arithmetic.
  num←{{⍵+.××0.5+×-/↕⍵,0ₓ}[7ₓ|"IVXLCDMivxlcdm"⍳⍵]⌷,⍉[1 5]ₓ×⊗10ₓ*⍳4ₓ}
  fmt←{~↢" ",0 1 0⍉(⊂[⍵⊤⍨4⍴10]⌷0 4 2 2⊤0 16 20 22 24 32 36 38 39 28)⌷' '⍪3 4⍴"MCXI DLV "}
  depth←{⍹≥≡⍵?⍶ ⍵;∇¨⍵}
  nums←num depth 1ₓ  ⍝ arabic from roman.
  fmts←fmt depth 0ₓ  ⍝ roman from arabic.
  fmts ⌊nums⍺ ⍶ ⌊nums⍵
}

⍝ From http://dfns.dyalog.com/c_stamps.htm

stamps ← {  ⍝ Postage stamps to the value of ⍵.
  ⍺←[1 5 6 10 26 39 43]ₓ  ⍝ Default UK stamp denominations.
  graph←⍺{⍵↣∩¨⍵+⊂⍺}⍳⍵+|⌊/⍺  ⍝ values: 0 ·· ⍵.
  spath←graph _graph.path [0ₓ ⍵]
  -/⌽↕spath
}

⍝ From http://dfns.dyalog.com/c_sieve.htm

sieve ← {  ⍝ Sieve of Eratosthenes.
  ⍺←⍬  ⍝ Default no primes yet.
  nxt←1↑⍵  ⍝ Next prime, and
  msk←0ₓ≠nxt|⍵  ⍝ ... mask of non-multiples.
  ∧/1↓msk?⍺,⍵;  ⍝ All non multiples - finished.
  ⍺,nxt ∇ msk#⍵  ⍝ Sieve remainder.
}

⍝ From http://dfns.dyalog.com/c_to.htm

to ← {  ⍝ Sequence ⍺ .. ⍵
  [from step]←[1 ¯1]ₓ×-\2↑⍺,⍺+×⍵-⍺  ⍝ step default is +/- 1.
  from+step×⍳0ₓ⌈≥⌊(⍵-from)÷step+step=0ₓ
}

⍝ From http://dfns.dyalog.com/s_to.htm

xTo ← {  ⍝ Sequence ⍺ .. ⍵
  [from step]←⊂¨[1 ¯1]ₓ×-\2↑⍺,⍺+×⍵-⍺  ⍝ step default is +/- 1.
  size←0ₓ⌈≥⌊⊃(⍵-from)÷step+step=0ₓ  ⍝ shape of result
  from+step×⍳size
}


⍝⍝ Real number processing

⍝ From http://dfns.dyalog.com/n_abc.htm

rg ← {⍺₀⍶⍵ ∧ ⍺₁⍹⍵}  ⍝ Range operator

xr ← {d←⍉⊃+/×⍵,.-⍺ᵀ ⋄ (2ₓ∨.=|d #⍠¯1 d)←2ₓ ⋄ 3ₓ⊥dᵀ}  ⍝ Outside location

⍝ From http://dfns.dyalog.com/c_alt.htm

alt ← {  ⍝ Alternant.
  [r c]←⍴⍵  ⍝ matrix ⍵
  0ₓ=r?⍹⌿,⍵;
  1ₓ≥c?⍶⌿,⍵;
  M←(⍳r)~⍤1⍪⍳r  ⍝ minors
  ⍵.[: 0]⍶.⍹∇¨⊂⍠[1 2][M 1↓⍳c]⌷⍵
}

bayes ← {⍺ ×÷+.× ⍵}  ⍝ Bayes' formula. (implemented as a fork)

⍝ From http://dfns.dyalog.com/c_Cholesky.htm

Cholesky ← {  ⍝ decomposition of a Hermitian positive-definite matrix.
  1ₓ≥n←≢⍵?√⍵;
  p←⌈n÷2ₓ
  q←⌊n÷2ₓ
  X←[p p]↑⍵ ⋄ Y←[p -q]↑⍵ ⋄ Z←(-q,q)↑⍵
  L_0←∇ X
  L_1←∇ Z-(TT←+Yᵀ +.× ⌹X)+.×Y
  [p n]↑L_0 ⍪ TT+.×L_0 , L_1
}

⍝ From http://dfns.dyalog.com/c_det.htm

det ← {  ⍝ Determinant of square matrix.
  ⍺←1ₓ  ⍝ product of co-factor coefficients so far
  0ₓ=n←≢⍵?⍺;  ⍝ result for 0-by-0
  [i j]←⍴⍵ ⊤ ↑⍒|,⍵
  k←⍳n
  ⍺×⍵.[i j]×¯1ₓ*i+j ∇ [k~[i] k~[j]]⌷⍵ - [k~[i] j]⌷⍵ ×⊗ [i k~[j]]⌷⍵ ÷ [i j]⌷⍵
}

⍝ From http://dfns.dyalog.com/c_gauss_jordan.htm

gauss_jordan ← {  ⍝ Gauss-Jordan elimination.
  elim←{  ⍝ elimination of row/col ⍺
    p←⍺+↑⍒|⍺↓[: ⍺]⌷⍵
    swap←⊖@[⍺ p] ⍵  ⍝ ⍺th and pth rows exchanged
    mat←swap.[⍺ ⍺]÷⍨@⍺ swap  ⍝ col diagonal reduced to 1
    mat - mat.[: ⍺]×⍺≠⍳≢⍵ ×⊗ ⍺⌷mat
  }
  ⍺←=/⊃⍳⍴⍵
  ⍴⍺ ⍴ 0 1×⍴⍵ ↓ (⍵,⍺)elim/⌽⍳⌊/⍴⍵
}

⍝ From http://dfns.dyalog.com/s_gauss_jordan.htm

tryGJ←{[⍺⌹⍵ ⍺gauss_jordan⍵]}  ⍝ gauss_jordan vs primitive ⌹.

⍝ From http://dfns.dyalog.com/s_gauss_jordan.htm

hil ← {÷1ₓ+ +⊗⍨⍳⍵}  ⍝ order ⍵ Hilbert matrix.

⍝ From http://dfns.dyalog.com/c_kcell.htm

kcell ← {  ⍝ Relationship between point and k-cell.
  ⍺←≢⍵ #⍠¯1 [2 1]⍴0 1  ⍝ Default is unit k-cell.
  b←,⍠(2ₓ=⍴⍴⍺ # 0)⍺
  p←(2⌊⍴⍴⍺ ↓ 1 1,⍴⍵)⍴⍵  ⍝ Points to evaluate.
  d←,⍶⌿ ⍤1 ⍉×-b,.-p
  ⍶/⍬?⌈/d;5ₓ⊥d
}

⍝ From http://dfns.dyalog.com/c_kball.htm

kball ← { ⍺←1  ⍝ Relationship between point and k-ball.
  r←↑⍺ ⋄ p←1#⍵
  c←≢p ↑ 1↓⍺  ⍝ Remaining coordinates are center.
  ×-/[⍉p-⍠0 c;r]+.*¨2ₓ
}

⍝ ⍝ From http://dfns.dyalog.com/c_ksphere.htm

ksphere ← {  ⍝ Surface area of k-sphere.
  n←≥⍺  ⍝ dimension of enclosed k-ball.
  pi←π1 * n÷2
  n × ⍵*⍺ × pi÷!n÷2  ⍝ k-sphere surface area.
}

⍝ From http://dfns.dyalog.com/s_ksphere.htm

kvol ← {⍵×(≤⍺ ksphere ⍵)÷⍺}

⍝ From http://dfns.dyalog.com/s_mean.htm

stdev ← {√(+/÷≢ ⍵²)-(+/÷≢ ⍵)²}  ⍝ Standard deviation.

⍝ From http://dfns.dyalog.com/c_NormRand.htm

NormRand ← {                                 ⍝ Random numbers with a normal distribution
  depth←10*9                                 ⍝ randomness depth
  [x y]←⊂⍤¯1 ≥¿depth⍴⍨2,⍵ ÷ depth          ⍝ two random variables within ]0;1]
  (¯2×⍟x * 0.5)×1○π2×y                       ⍝ Box-Muller distribution
}

⍝ From http://dfns.dyalog.com/c_phinary.htm
phinary ← {  ⍝ Phinary representation; left argument 0 returns exponents.
  ⍺←1
  Ø←≥√5 ÷ 2
  ""≡0#∊⍵?{
    1ₓ<≡⍵?∇¨⍵;
    '¯'=↑⍵?-∇ 1↓⍵;
    a←Ø⊥$d⍳⍵~"."
    a÷Ø*(≢⍵∪'.')-≥(,⍵)⍳'.'
  }⍵;
  0ₓ≠≡⍵?⍺∇¨⍵;
  ⍵<0?'¯',⍺∇-⍵;
  num←⍵
  ⍺{
    ⍺=0?⍵;
    ⍵≡⍬?"0";
    fmt←{[⍵]⌷"01"}
    lft←⌽⍳0ₓ⌈≥⌈/⍵ ∊ ⍵
    rgt←-≥⍳0ₓ⌈|⌊/⍵ ∊ ⍵
    rgt∧.=$f?fmt lft;
    lft∧.=$f?"0.",fmt rgt;
    fmt⁝lft , '.' , fmt⁝rgt
  }⍬ₓ{
    num=Ø+.*⍺?⍺;
    ∆←-⍴⍺ ↑ 1ₓ
    num=Ø+.*⍺+∆?⍺+∆;
    k←⌊Ø⍟⍵
    ⍺,k ∇ ⍵-Ø*k
  }⍵
}

⍝ From http://dfns.dyalog.com/s_phinary.htm

align←{p←{⍵⍳'.'}¨⍵ ⋄ ⊃(⌈/p - p){⍺⍴' ' , ⍵}¨⍵}

⍝ From http://dfns.dyalog.com/c_roots.htm

realroots ← {  ⍝ Real roots of quadratic.
  [a b c]←⍵  ⍝ Coefficients.
  d←b²-4ₓ×a×c  ⍝ Discriminant.
  d<0ₓ?⍬;  ⍝ No roots
  d=0ₓ?-b÷2ₓ×a;  ⍝ One root
  -b+[¯1 1]ₓ×√d ÷ 2ₓ×a  ⍝ Two roots
}

⍝ From http://dfns.dyalog.com/s_roots.htm

roots ← {  ⍝ Roots of quadratic.
  [a b c]←⍵  ⍝ coefficients.
  d←b²-4ₓ×a×c  ⍝ discriminant.
  -b+[¯1 1]ₓ×√d ÷ 2ₓ×a  ⍝ both roots.
}


⍝⍝ Complex number processing

⍝ From http://dfns.dyalog.com/c_polar.htm

polar ← {  ⍝ Polar from/to cartesian coordinates.
  pol_car←{  ⍝ polar from cartesian (default).
    radius←{√+⌿⍵²}  ⍝ radius (pythagorus).
    angle←{  ⍝ phase angle.
      [x y]←⊂⍤¯1 ⍵  ⍝ x and y coordinates.
      [on off]←$t $f=⊂0=x  ⍝ points on/off y axis.
      atan←3○⁻¹y÷x+on  ⍝ arctan y÷x (avoiding y÷0).
      qne←off×atan + on×π0.5×2-×y
      nsw←πx<0
      qse←π2× x>0 ∧ y<0
      nsw+qse+qne  ⍝ all quadrants.
    }
    radius⍵ lam angle⍵  ⍝ (2,···) array of polar coordinates.
  }
  car_pol←{  ⍝ cartesian from polar.
    [r o]←⊂⍤¯1 ⍵  ⍝ radius and phase angle.
    r×2○o lam r×1○o  ⍝ r×cos(ø), r×sin(ø).
  }
  lam←{⊃[⍺ ⍵]}
  ⍺←1ₓ  ⍝ default polar from cartesian.
  ⍺=+1ₓ?pol_car ⍵;  ⍝ polar from cartesian.
  ⍺=-1ₓ?car_pol ⍵  ⍝ cartesian from polar.
}

⍝ From http://dfns.dyalog.com/s_polar.htm

rnd ← {10*-⍺ × ⌊0.5+⍵×10*⍺}

poly ← {2 1 ○⊗ π2÷⍵ × ⍳⍵}

⍝ From http://dfns.dyalog.com/c_xtimes.htm

xtimes ← { m←0  ⍝ Fast multi-digit product using FFT.
  xroots    ← {×\1,1↓(⍵÷2)⍴¯1*2÷⍵}
  cube      ← {⍵⍴⍨2⍴⍨⌊2⍟⍴⍵}
  extend    ← {2*⌈2⍟≤⍺+⍥⍴⍵ ↑¨ [⍺ ⍵]}
  floop     ← {(⊣/⍺)∇⍣(×m) (+⌿⍵){⊃[⍺ ⍵]}⍤(≢⍴⍵ - m+1) ⍺×⍠(⍳m←≢⍴⍺)-⌿⍵}
  FFT       ← {, cube⁝xroots⍴⍵ floop cube⍵}
  iFFT      ← {⍴⍵ ÷⍨ , cube+xroots⍴⍵ floop cube⍵}
  rconvolve ← {≤⍺+⍥⍴⍵ ↑ iFFT×/FFT¨⍺extend⍵}
  carry     ← {1↓+⌿1 0⌽0ₓ,⍉[0 10]ₓ⊤⍵}
  (+/∧\0ₓ=t)↓t←carry⍣≡0ₓ,⌊0.5+⊣/∨⍺rconvolve⍵
}

convolve ← {+⌿ -⍳⍴⍺ ⌽ ⍺×⊗⍵,0ₓ×1↓⍺}

⍝ From http://dfns.dyalog.com/c_xpower.htm

xpower ← {  ⍝ Fast multi-digit power using FFT.
  xt←{0ₓ,⍺ xtimes 0ₓ,⍵} ⋄ b←⌽⊤10ₓ⊥⍵  ⍝ boolean showing which powers needed
  ⊃,/xt/b#{xt⍨⍺}\(⊂,10ₓ⊥⁻¹⍺)⍴⍨⍴b
}
