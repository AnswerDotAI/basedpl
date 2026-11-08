⍝⍝ Mathish semantics

⍝ — ary writes zero, terminating fractions and exact powers of the base, and repeats a recurring unit twice
[ary:]←•load "lib/dyalog.bpl"
[2 ary 0;2 ary 0.75;2 ary ¯0.75;2 ary 8;16 ary 255;10 ary ÷7]
⍝ =>
"0" "0.11" "¯0.11" "1000" "FF" "0.142857142857..."

⍝ — ary rounds a fraction whose digits run past its limit, and marks the result with ?
[ary:]←•load "lib/dyalog.bpl"
[10 ary π 1;10 ary 2*÷2;16 ary π 1]
⍝ =>
"3.1415926535898?" "1.4142135623731?" "3.243F6A8885A?"

