⍝⍝ Mathish semantics

⍝ aplcart/table.tsv:1903 — Rounding N to I decimal places (Fast ⍎Is⍕N)
I←2
N←1.234 ¯2.345 0.005
I{p÷⍨⌊0.5+ω×p←10*α}N   ⍝ 1.23 ¯2.34 0.01

⍝ aplcart/table.tsv:1706 — α-ary representation of rational ω; Ported in lib/dyalog.bpl; checked against the independent Dyalog expectation; Concrete arguments; shared 0-origin based-array port loaded from lib/dyalog.bpl; The source reads the comparison tolerance; this port uses the fixed 1e-14
[ary:]←•load "lib/dyalog.bpl"
2 ary ÷3   ⍝ "0.0101..."

