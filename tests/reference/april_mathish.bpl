⍝⍝ Mathish complex arithmetic and numerical convergence
⍝ april:589 —
⌈1000×1.3j2.6∧4.5j8.9   ⍝ ¯172900j232700

⍝ april:590 —
⌈1000×3.8j7.6∧5.2j6.8   ⍝ ¯159600j326800

⍝ april:602 —
1.3j2.6∨4.5j8.9   ⍝ 0.1

⍝ april:603 —
3.8j7.6∨5.2j6.8   ⍝ 0.2

⍝ april:1754 — remove April numeric separators
⌊1000000000×2○⍣=1   ⍝ 739085133ₓ

⍝ april/libraries/dfns/power/demo.lisp:21 — Upstream numerical-inverse library setup; its CT=1E-14 is the fixed language tolerance
[invr:]←•load "lib/power.bpl"
⌊10000×(1↣○)invr(1↣○)10   ⍝ 685398ₓ

⍝ april/libraries/dfns/power/demo.lisp:22 — Upstream numerical-inverse library setup; its CT=1E-14 is the fixed language tolerance
[invr:]←•load "lib/power.bpl"
⌊1000× **invr 1.23   ⍝ 1230ₓ

⍝ april/libraries/dfns/power/demo.lisp:42 — Dyalog independently confirms five Newton iterates at fixed CT, rather than April fixture six
[nr: traj:]←•load "lib/power.bpl"
⌊1000000000×1e¯12↣(2↣○ nr)traj 1.5
⍝ =>
[1500000000 1570909012 1570796336 1570796326 1570796326]ₓ

⍝ april/libraries/dfns/numeric/demo.lisp:271 — Phinary; original independent upstream expectation
[phinary:]←•load "lib/numeric.bpl"
phinary42   ⍝ "10100010.00100001"

⍝ april/libraries/dfns/numeric/demo.lisp:279 — Phinary; original independent upstream expectation
[align: phinary: to:]←•load "lib/numeric.bpl"
↓align phinary (1+√5 ÷ 2)* 0 to 10
⍝ =>
"          1" "         10" "        100" "       1000" "      10000" "     100000" "    1000000" "   10000000" "  100000000" " 1000000000" "10000000000"

⍝ april/libraries/dfns/numeric/demo.lisp:283 — Phinary; original independent upstream expectation
[align: phinary: to:]←•load "lib/numeric.bpl"
↓align phinary (1+√5 ÷ 2)* ¯1 to ¯10
⍝ =>
"0.1         " "0.01        " "0.001       " "0.0001      " "0.00001     " "0.000001    " "0.0000001   " "0.00000001  " "0.000000001 " "0.0000000001"

⍝ april/libraries/dfns/numeric/demo.lisp:289 — Phinary; original independent upstream expectation
[align: phinary: to:]←•load "lib/numeric.bpl"
↓align phinary (√5)* 0 to 6
⍝ =>
"          1           " "         10.1         " "       1000.1001      " "     100000.00001     " "    1010000.000101    " "  100010010.010001001 " "10000000010.0100000001"

⍝ april:410 — Commuted division follows the Mathish zero rule
÷⍨0   ⍝ 0

⍝ april/libraries/dfns/numeric/demo.lisp:211 — Real rational recovery uses the first continued-fraction convergent within relative error 1e-14. Pi recovers 5419351/1725033, whose canonical continued fraction ends in 4. April's 14-column float-round-trip expectation is retained separately; 0+ makes coefficients approximate
[cfract: rational:]←•load "lib/numeric.bpl"
rational (+↢÷)/¨,\ 0+(0 cfract π1)
⍝ =>
[3 22 333 355 103993 104348 208341 312689 833719 1146408 5419351⋄1 7 106 113 33102 33215 66317 99532 265381 364913 1725033]

