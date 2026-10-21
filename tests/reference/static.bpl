•prefs ["kinds":"static"]

⍝⍝ Static kinds

⍝ — Under static kinds, a result's kind follows from its arguments' kinds: real arguments stay real, complex results stay complex, ⌊ and ÷ give floats, and integers compute at the int width
{(•storage ⍵).kind}¨[√¯4 9;1j0;0j1×0j1;⌊2.5 3.5;7ₓ÷2ₓ;[1 2]ₓ]
"float64" "complex128" "complex128" "float64" "float64" "int64"

⍝ — Static kinds give NaN outside the real domain and wrap integers that don't fit. Exact fractions stay exact
[√¯4 9;⌊2.5 ∞;7ₓ÷2ₓ;9223372036854775807ₓ+1ₓ;2ₓ*100ₓ;1r3+1r6]
[[$n 3] [2 ∞] 3.5 ¯9223372036854775808ₓ 0ₓ 1ᵣ2]

⍝ — Under static kinds, a weak atom takes the width of the floats beside it, as under dynamic kinds. An integer beside a float takes the float's width. Booleans add as integers
{(•storage ⍵).kind}¨[(32•float 1 2)×2;(32•float 1 2)+0j1;[1 2]ₓ+0.5;[1 2]ₓ+32•float 0.5;$t $f+$t $t]
"float32" "complex64" "float64" "float32" "int64"

⍝ — Under static kinds, folds, scans and integer products wrap too, where dynamic kinds widen to exact integers of any size
[+/[9223372036854775807 1]ₓ +\[9223372036854775807 1]ₓ ×/[4294967296 4294967296]ₓ [9223372036854775807 1]ₓ+.×[1 1]ₓ]
[¯9223372036854775808ₓ [9223372036854775807 ¯9223372036854775808]ₓ 0ₓ ¯9223372036854775808ₓ]

⍝ — Under static kinds, ÷ divides integers as floats on every route: on arrays, in folds and in scans, lanes of one item included
{(•storage ⍵).kind}¨[[6 8]ₓ÷[3 2]ₓ;÷/[6 3]ₓ;÷\[6 3]ₓ;÷/,5ₓ;÷\,5ₓ;÷/[2 1]⍴5ₓ 7ₓ]
"float64" "float64" "float64" "float64" "float64" "float64"
