⍝⍝ setup
⍝ Each case's counts are for each of `n` steps. In `calls`, a case makes its own `n` steps, each reading the step before. In `steps`, the count runs a case `n` times, forcing its value each time, because a value that nothing reads never runs. These cases have counts and no time budgets.
n←1024 size 16
k←0 ⋄ t←64⍴"ab" "cde" ⋄ m←[0 1⋄¯1 0] ⋄ s←[64 2 2]⍴$t $f $f $t $t $t $f $t
v_4←1.5 2.5 3.5 4.5 ⋄ v_16←1.5+⍳16 ⋄ a←v_4
loss←{+/⊼1-2×⍵} ⋄ rosen←{[x y]←⍵ ⋄ ⊼1-x + 100×⊼y-⊼x}
⍝⍝ calls
⍝ A primitive call.
+⍣n 0
⍝⍝: -	-	-	instructions macos 3442.289 linux 3082.852	allocated macos 24.688 linux 24.688

⍝ A dfn call that returns its argument.
{⍵}⍣n 0
⍝⍝: -	-	-	instructions macos 2218.688 linux 2182.375	allocated macos 9.812 linux 9.812

⍝ A dfn that reads a global.
{⍵+k}⍣n 0
⍝⍝: -	-	-	instructions macos 6979.812 linux 7092.195	allocated macos 41.812 linux 41.812

⍝ A local assignment and read.
{x←⍵+1 ⋄ x}⍣n 0
⍝⍝: -	-	-	instructions macos 8994.930 linux 9084.070	allocated macos 41.812 linux 41.812

⍝ A spaced run that constructs and applies a train at once.
{(⍵)+⊢ ⍵}⍣n 0
⍝⍝: -	-	-	instructions macos 14665.000 linux 13708.461	allocated macos 403.312 linux 403.312

⍝ A bracket literal. Joining its items' kinds, fixedness and weakness in `Promoted::join` added about 125 instructions on 24 October 2026. The memory guard's check on each allocation and the later promotion refactor added about 95 more. Moving `other_step` out of line and the `Value::Pending` arms in the resolving functions added about 220 more.
{↑[⍵ 1]}⍣n 0
⍝⍝: -	-	-	instructions macos 10616.148 linux 8929.242	allocated macos 297.812 linux 297.812

⍝ Each reverses the 64 ragged strings in `t` cell by cell. Their two lengths prevent rectangular stacking, so batching declines and the ordinary structural-copy path reads each string's plan.
⌽¨⍣n t
⍝⍝: -	-	-	instructions macos 520127.914 linux 440998.188	allocated macos 37761.812 linux 37761.812

⍝ Rank's general path calls a dfn once for each of the same 64 cells.
{⌽⍵}⍤0⍣n t
⍝⍝: -	-	-	instructions macos 828788.719 linux 816740.578	allocated macos 43355.375 linux 43355.375

⍝ A system function that converts each item on its own runs once on the leaf of the same 64 strings (`batch::leaf_system`).
•ucs⍣n t
⍝⍝: -	-	-	instructions macos 240322.578 linux 241054.875	allocated macos 11984.688 linux 11984.688

⍝ `⇄` by L-BFGS on Rosenbrock's function: 53 evaluations, about 153,500 instructions each. Each evaluation calls `linearized` directly, outside the program cache. The case runs once, so its counts are for a 1,024th of the run.
rosen⇄ ¯1.2 1
⍝⍝: -	-	-	instructions macos 192045.312 linux 167131.688	allocated macos 27658.445 linux 27655.133

⍝⍝ steps
⍝ A fixed mark on a 4-item float vector, which floors each item to `i32` through `derivative::converted`, the entry point that marks on values and on functions share. Stage 3's step 6 sent value marks there, which cost about 215 instructions. The inline size test in `fusion::convert` and the inline tests of `Value::carried` and `Value::holds_values` recover about 100 of them. On 26 October 2026, single-stepping `binder::derived` under lldb counted every instruction that it and its calls retire. Outside `malloc`, the other 115 lay in four functions: 130 in `derivative::converted`, 17 in an out-of-line `product` for its size test's length, 15 in `Value::converted`'s dispatch, and 47 fewer in `binder::derived`, which no longer reads the settings itself. Of the 130, about 45 read the settings, 25 test for carriers and 12 make the size test. The inline `rustymath::layout::count` and an always-inlined `derivative::plain` then saved 19.
v_4ₓ₄
⍝⍝: -	-	-	instructions macos 8263.07 linux 6222.758	allocated macos 536 linux 536


⍝ A 2×2 float matrix product.
m+.×m
⍝⍝: -	-	-	instructions macos 112926.297 linux 86488.672	allocated macos 14808 linux 15032

⍝ Rank on a stack of 64 Boolean 2×2 matrices, which runs one or-and product for the whole stack.
∨.∧⍤2⍨s
⍝⍝: -	-	-	instructions macos 137271.828 linux 117393.383	allocated macos 18354 linux 18354

⍝ `∂` of a dfn on 4 floats, whose later calls replay the program that its first call traced.
loss∂v_4
⍝⍝: -	-	-	instructions macos 39399.234 linux 33128.766	allocated macos 34888 linux 34888

⍝ The same on 16 floats. At this size a replay costs the same per call.
loss∂v_16
⍝⍝: -	-	-	instructions macos 39468.695 linux 33152.766	allocated macos 35080 linux 35080


⍝ Selective assignment through a selecting function on 4 floats, then a read of the result. The target `1↑a` runs under a `Label` level on the simple root `a`, whose labels hold a mapping and no ids. `Selection::write_into` writes the value at the position that the labels give.
(1↑a)←0 ⋄ a
⍝⍝: -	-	-	instructions macos 33168.719 linux 25247.758	allocated macos 4832 linux 4832

⍝ Under with a selecting function on the same kind of root. `1↑` runs under a `Label` level and `2×` on its result. `Selection::write_into` writes that result at the position that the labels give.
(2×)@(1↑)v_4
⍝⍝: -	-	-	instructions macos 61749.781 linux 47231.773	allocated macos 7332 linux 7332
