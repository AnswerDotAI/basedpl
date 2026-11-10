⍝⍝ setup
v←¿(65536 size 256)⍴0
n←≢v ⋄ outer_side←256 size 16 ⋄ map_steps←1000 size 8 ⋄ descent_steps←100 size 2
sg←¿[(128 size 8) (128 size 8)]⍴0
g←0.6<¿[(128 size 8) (128 size 8)]⍴0
X←¿[(4096 size 8) 8]⍴0
thz←¿8⍴0
vv←¿8⍴0
cs←¿(400 size 32)⍴0
big←¿[4 (256 size 8) (256 size 8)]⍴0
st←¿[(4096 size 16) 4 4]⍴0
irow←(¿[(10000 size 16) 8]⍴100)ₓ
nest←⊂⍤1⁝irow
rag←{¿[2 ⍵]⍴0}¨1+15|⍳(4096 size 32)
⍝ Sum of squared sines.
f←{+/⊼1○⍵}
⍝ Sum of squared sines of linear projections over rows of X.
loss←{
  th←⍵
  +/{⊼1○+/th×⍵}⍤1 X
}
⍝ The same projection loss using a matrix-vector product.
lossw←{+/⊼1○X+.×⍵}
⍝ Directional derivative of loss along vv.
hv←{+/vv×loss∂⍵}
⍝ Directional derivative of lossw along vv.
hvw←{+/vv×lossw∂⍵}
⍝ One step of gradient descent on lossw.
descend←{⍵-0.01×lossw∂⍵}
⍝ One Game of Life update from a 3 by 3 neighbourhood.
life←{
  s←+/,⍵
  s=3ₓ ∨ 1 1⌷⍵ ∧ s=4ₓ
}
⍝ One Game of Life update over the whole grid.
lifew←{
  s←+/+/¯3 ¯3↕⍵
  s=3ₓ ∨ ⍵∧s=4ₓ
}
sq←{⍵×⍵}
sq∂←{⍺×3×⍵}
imgs←¿[(64 size 2) (32 size 8) (32 size 8) 3]⍴0
ck←0.1×¯0.5+¿[27 32]⍴0
cb←32⍴0
⍝ Valid 3 by 3 convolution over all input channels.
conv←{
  w←3 3↕⍵
  s←2↑⍴w
  (s,¯1↑⍴⍺)⍴([×/s ×/2↓⍴w]⍴w)+.×⍺
}
⍝ Non-overlapping 2 by 2 max pooling, as one fold along both axes of each window.
pool←{⌈/₂ ₃[[2 2]⋄[2 2]]↕⍵}
⍝ Normalize each channel across spatial positions.
norm←{
  x←(×/¯1↓⍴⍵ , ¯1↑⍴⍵)⍴⍵
  c←x-⍤1 +⌿÷≢ x
  ⍴⍵ ⍴ c÷⍤1(√1E¯5+(+⌿÷≢)⊼c)
}
⍝ Convolution, ReLU, max pooling and channel normalization.
cnn←{norm pool 0⌈cb+⍤1(ck conv ⍵)}
Y←¿[(4096 size 8) 8]⍴0
⍝⍝ maps
⍝ Each, Rank and `⌺` first try one call of their operand on batches of all their cells, and call it once for each cell where that call stops. Maps inside `∂` batch too. Each mapped case sits beside the same computation written with whole arrays, whose cost a batched map should approach.
⍝ Expected: `{⍵×⍵+1}¨v` 2.5 copies: as `v×v+1`; `v×v+1` 2.5 copies: two passes below the fusion threshold, one copy then two reads and a write; `{+/,⍵}⌺[3 3] sg` 2 copies: nine reads for each of 16,384 windows, and their sums; `+/+/¯3 ¯3↕sg` 0.75 copies: a padded copy and two sliding passes over 16,384 items.
⍝⍝⍝ route batching
{⍵×⍵+1}¨v
⍝⍝: 5	6.9	6.8	instructions macos 21.74 linux 15.191	allocated macos 16.826 linux 16.826

v×v+1
⍝⍝: 5	6.3	6.2	instructions macos 16.566 linux 10.7	allocated macos 16.621 linux 16.621

⍝ Each over a nested array whose 10,000 items are vectors of 8 small integers. Each passes its operand each item, as Rank passes each row of the matrix that holds them. Where every item is a compact array of one layout and dtype, a batched map stacks the items into one array and runs as Rank does.
⍝ Expected: `{⍵+1ₓ}¨nest` and `{+/⍵}¨nest` 2.5 copies: as `{⍵+1ₓ}⍤1⁝irow` and `+/⍤1 irow`, which cost 1.88 and 2.02 on 30 October 2026, and a copy of the 80,000 one-byte items into one array.
⍝ On 30 October 2026 `{⍵+1ₓ}¨nest` cost 33 times its Rank form, and `{+/⍵}¨nest` 12 times. Stacking costs about 18 ns an item, and assembling `{⍵+1ₓ}¨nest`'s 10,000 result arrays about 30 more. Before then each called its dfn once for each item, at 416 and 357.
⍝⍝⍝ held macos 62.9: stacking and assembling the items; Linux and wasm hold estimates of 1.7 and 1.4 times the macOS cost
⍝⍝⍝ route batching
{⍵+1ₓ}¨nest
⍝⍝: 5	760	720	instructions macos 260.262 linux 261.597	allocated macos 31.784 linux 31.784

⍝⍝⍝ held macos 24.7: stacking and assembling the items; Linux and wasm hold estimates of 1.7 and 1.4 times the macOS cost
⍝⍝⍝ route batching
{+/⍵}¨nest
⍝⍝: 5	570	570	instructions macos 95.507 linux 97.45	allocated macos 3.459 linux 3.459

⍝ Each over a nested array whose 4,096 items are matrices of 2 rows and 1 to 15 columns, ragged cells in mixed storage, with a constant vector added along each item's rows. The batched try declines on ragged cells, which aren't compact (`batch::mapped`: "an argument that isn't compact"). The dfn then runs once for each of the 4,096 cells, at about 18,600 instructions a call.
⍝ Expected: 10 copies, from a batched map that gathers the cells into one leaf and runs the pervasive call once on it, with each item of the constant repeated over its cell's row through one `#`. That route is Later work.
⍝⍝⍝ held linux 627: the dfn runs once for each ragged cell until batching takes ragged cells, which is later work
⍝⍝⍝ route batching
{⍵+10 20}¨rag
⍝⍝: 20	420	400	instructions macos 3674.203 linux 2593.752	allocated macos 191.807 linux 191.807

⍝⍝⍝ held macos 5.37: open.md lists the stencils among the speed audit's outliers that compiling didn't fix
⍝⍝⍝ gap forced "{+/,⍵}⌺[3 3]": a dfn operand's fold over windows should slide over the source
{+/,⍵}⌺[3 3] sg
⍝⍝: 4	12	15	instructions macos 62.599 linux 50.681	allocated macos 25.295 linux 25.295

⍝⍝⍝ held macos 4.19: open.md lists wide and 2-D window folds among the speed audit's outliers
+/+/¯3 ¯3↕sg
⍝⍝: 1.5	7.3	7.8	instructions macos 62.781 linux 51.714	allocated macos 31.885 linux 31.885

⍝ The Game of Life on a 128 by 128 Boolean grid, as a stencil and with padded windows.
⍝ Expected: `life⌺[3 3] g` 1.5 copies: a few passes over nine 1-byte reads for each of 16,384 windows; `life⌺["size":[3 3] "edge":"wrap"] g` 1.5 copies: as the stencil with fill; `lifew g` 0.75 copies: sliding sums and comparisons over 16,384 items.
⍝⍝⍝ held macos 4.54: open.md lists the stencils among the speed audit's outliers that compiling didn't fix
life⌺[3 3] g
⍝⍝: 3	15	13	instructions macos 105.821 linux 82.039	allocated macos 11.215 linux 11.215

⍝⍝⍝ held macos 4.8: open.md lists the stencils among the speed audit's outliers that compiling didn't fix
life⌺["size":[3 3] "edge":"wrap"] g
⍝⍝: 3	15	15	instructions macos 114.052 linux 87.687	allocated macos 11.842 linux 11.85

⍝⍝⍝ held macos 4.15: open.md lists wide and 2-D window folds among the speed audit's outliers
lifew g
⍝⍝: 1.5	9.7	8.2	instructions macos 85.208 linux 69.371	allocated macos 17.089 linux 17.089


⍝ Agenda compares each item, computes square roots and blends the selected result. Expected: 4 copies.
⍝⍝⍝ held macos 7.66: open.md lists the masked selects among the speed audit's outliers
(0.5≤)⍚[0 √] v
⍝⍝: 8	18	21	instructions macos 50.11 linux 39.678	allocated macos 39.126 linux 39.126

⍝ The gradient of a loss mapped over the rows of `X`, whose parameter every row shares, then per-example gradients, then a Hessian–vector product through the mapped loss. Each sits beside its form with `+.×`, or its closed form.
⍝ Expected: `loss∂ thz` 4 copies: the primal over 32,768 items, about 1 copy, with sines of 4,096 items at 7 copies for each 65,536, about 0.45, and a transpose of about 3; `lossw∂ thz` 3 copies: a product over 32,768 items, `1○` over 4,096 items, about 0.45, and its transpose; `{f∂ ⍵}⍤1 X` 9 copies: sines and cosines of 32,768 items, half of `n`, at 7 copies each as `1○v` counts them, then the squares, the row sums and each item's product of sine and cosine, about 2; `2×(1○X)×2○X` 7.5 copies: sines and cosines of 32,768 items, half of `n`, at 7 copies each as `1○v` counts them, and two products; `hv∂ thz` 8 copies: about twice the gradient; `hvw∂ thz` 6 copies: about twice the gradient.
⍝ On 27 October 2026 `loss∂ thz` measured 6.02 on macOS. Its wasm budget is twice its native budget, as `1○v`'s are. On 22 October 2026 CI measured 8.09 in wasm.
loss∂ thz
⍝⍝: 8	15	18	instructions macos 72.231 linux 46.722	allocated macos 19.423 linux 19.423

⍝ On 27 October 2026 `lossw∂ thz` measured 5.22 on macOS, close to twice its expected cost. Each of its two products of the 4096×8 `X` and a vector costs about 1.6 copies, where reading `X` costs 0.25: `Xb+.×thz` measured 1.59. On 22 October 2026 CI measured 8.39 on Linux and 7.21 in wasm.
lossw∂ thz
⍝⍝: 6	7.5	11	instructions macos 71.702 linux 46.542	allocated macos 19.423 linux 19.423

⍝ On 27 October 2026 `{f∂ ⍵}⍤1 X` measured 12.38 on macOS. Its budgets are twice the expected cost: 18 on macOS and Linux, and 36 in wasm, where `1○v`'s budgets in arrays.bpl give sine and cosine twice their native cost. On 22 October 2026 CI measured 11.6 on Linux and 20.6 in wasm.
{f∂ ⍵}⍤1 X
⍝⍝: 18	23	53	instructions macos 124.486 linux 86.376	allocated macos 31.765 linux 31.755

⍝ `•explain` shows no fused stage: `1○X` and `2○X` each run over all of `X`, one after the other. On 27 October 2026 it measured 8.24 on macOS. The budgets are twice the expected cost: 15 on macOS and Linux, and 30 in wasm, where `1○v`'s budgets in arrays.bpl give sine and cosine twice their native cost. On 22 October 2026 CI measured 16.1 in wasm.
2× 1○X × 2○X
⍝⍝: 15	13	44	instructions macos 49.062 linux 31.612	allocated macos 17.071 linux 17.071

hv∂ thz
⍝⍝: 16	29	36	instructions macos 123.006 linux 81.529	allocated macos 21.407 linux 21.407

hvw∂ thz
⍝⍝: 12	11	16	instructions macos 123.006 linux 81.311	allocated macos 21.407 linux 21.407

⍝ An attached rule inside Each, beside the same rule on the whole array.
⍝ Expected: `{+/sq¨⍵}∂ v` 3 copies: as `{+/sq ⍵}∂ v`; `{+/sq ⍵}∂ v` 3 copies: the primal and the attached rule over every item.
⍝⍝⍝ gap stopped "a read of a traced value": Each should trace the attached derivative rule over the whole array
{+/sq¨⍵}∂ v
⍝⍝: 6	22	19	instructions macos 13.867 linux 8.991	allocated macos 24.572 linux 24.572

{+/sq ⍵}∂ v
⍝⍝: 6	5.4	5.7	instructions macos 13.865 linux 8.991	allocated macos 24.572 linux 24.572

⍝ Each on one atomic cell, repeated inside Power. Count-only: preserve the original allocation/instruction regression check. Counted setup runs 64 iterations and divides by n=4096; these legacy scaled counts are not literal bytes per operator call.
{⍵+1}¨⍣map_steps,0
⍝⍝: -	-	-	instructions macos 166.653 linux 142.885	allocated macos 3.191 linux 3.191

⍝ 16 steps of an elementwise body. After its first step, a loop runs such a body as ordinary code, so each step's add waits and all 16 run as one block stage, as the chain written out does (`v+v+v` and so on).
⍝ Expected: 8.4 copies: one interpreted block stage of 16 adds over `v`, about half a copy for each add. On this Mac on 1 November 2026 the chain written out measured 8.39.
{⍵+v}⍣16 v
⍝⍝: 17	66	59	instructions macos 107.032 linux 75.474	allocated macos 131.822 linux 131.822

⍝ 100 steps of gradient descent on `lossw`. Each step after the first replays the recording of `descend`'s body, the gradient and the update, as one program.
⍝ Expected: 300 copies: 100 gradients at `lossw∂ thz`'s 3 copies each. On this Mac on 1 November 2026 it measured 217, 2.17 a step, against 2.40 for one call of `lossw∂ thz`.
(descend⍣descent_steps)thz
⍝⍝: 450	600	710	instructions macos 1206.832 linux 831.121	allocated macos 316.357 linux 316.357

⍝ The gradient of a sum through a loop with a predicate, which halves `v` until every item is under 1e¯3: 10 steps, a number the data decide. `∂` gives two loop nodes: the forward loop, and one that reruns it from its first carry, keeping the carries, then folds the body's VJP back over them.
⍝ Expected: 26 copies: the rerun's 10 steps at about 1.5 copies each (each halving writes `v` once, and the predicate's `⌈/|⍺` reads it once), then the backward pass's 10 halvings of the cotangent at about 1 copy each. A replay runs no forward loop of its own: only the loss's value reads it, and the gradient doesn't. On this Mac on 1 November 2026 it measured 28.5, against 30.4 for the gradient through 10 counted steps, `{+/{⍵×0.5}⍣10⁝⍵}∂ v`, whose forward loop keeps a stack.
{+/{⍵×0.5}⍣{1e¯3>⌈/|⍺}⁝⍵}∂ v
⍝⍝: 52	120	230	instructions macos 373.167 linux 280.654	allocated macos 270.116 linux 270.11

⍝ A map of 400 scalars against `v`. Repeated for each cell, `v` would hold 26 million items, past the memory limit of a batched run. The map then calls its operand once for each cell, holding one copy of `v` at a time.
⍝ Expected: 1000 copies: 400 cells, each about 2.5 copies in chunks.
⍝ CI measured 812 to 852 on Linux in most runs from 56 to 62, and up to 1,232 in runs whose hosts were busy.
{+/ v-⍵ × v-⍵}¨cs
⍝⍝: 1300	900	1700	instructions macos 1368.041 linux 807.877	allocated macos 26.217 linux 26.198

⍝ Four cells of 256 by 256 floats. The batched run takes the sines of all four cells as one call, and `⍳` searches each cell's own table on slices of the batch (`searched_cells`).
⍝ Expected: 30 copies: sines of four cells of `n` items at `1○v`'s 7 copies each, 28, and one read of each cell's table for the search, 2. In wasm 58: sines at twice their native cost, as `1○v` expects. The expected cost was 18 until 31 October 2026, when cf2_ measured the sines at 73% of the case's instructions; 18 had counted about 4 copies a cell for them.
⍝⍝⍝ allows forced "⍳": a search reads its table's items
{
  r←1○⍵
  ,r ⍳ 0.5
}⍤2 big
⍝⍝: 60	130	190	instructions macos 121.528 linux 72.719	allocated macos 32.932 linux 32.932

⍝ Rank of a dfn on each row of `X` makes one batched call. The subtraction pads each row's mean, `+/÷≢ ⍵`, over the row.
⍝ Expected: 1 copy each: row sums of 32,768 items and one padded subtraction.
⍝⍝⍝ held macos 1.85: as `X - +/X ÷ 8`, the subtraction broadcasts each row's mean over its 8 items, one `pairs` call per row (open.md, row means, centring)
{⍵ - +/÷≢ ⍵}⍤1 X
⍝⍝: 2	5	5.5	instructions macos 36.401 linux 27.953	allocated macos 11.053 linux 11.053

X - +/X ÷ 8
⍝⍝: 2	4.6	4.9	instructions macos 28.558 linux 21.649	allocated macos 10.635 linux 10.635

⍝ An outer product of a dfn makes one batched call, with both arguments laid over the result's frame.
⍝ Expected: 1 copy: the operand on views laid over the frame, and one write of `n` items.
⍝⍝⍝ held macos 4.54: both arguments are laid over the 65,536-item frame, so `⍵+1` runs at every frame position (open.md, dfn operands of scans, folds, outer products); stage 6 keeps each framed value to its own frame axes (main's decision 23)
⍳outer_side {⍺×⍵+1}⊗ ⍳outer_side
⍝⍝: 2	11	9.1	instructions macos 38.896 linux 30.648	allocated macos 35.486 linux 35.486

⍝ Expected: 0.73 copies: 0.5 for writing 8 bytes an item, and one node, about 0.23 copies of 65,536 floats (`fusion::MAC_NODE_NS`, 1,800 ns), because the lone outer product waits under the cost rule and nothing fuses with it (decision 13), as for its twin `⍳√n ×⊗ ⍳√n` in arrays.bpl. In the browser the call runs at once, because the cost rule's limit there is about 830 KB, above the call's 512 KB. The expected cost was 0.5 until 31 October 2026, when cf2_ found that the two cases time alike (1.31 and 1.27 copies under load).
⍳outer_side ×⊗ 1+⍳outer_side
⍝⍝: 1.5	1.5	1.9	instructions macos 27.093 linux 18.965	allocated macos 26.328 linux 26.328

⍝ Each executes this body and its draw once per item. Draws are effects and do not batch; the bulk spelling below draws one whole array.
⍝⍝⍝ allows itemwise "batch cells": random draws run separately for each item
{⍵+¿10}¨v
⍝⍝: 56	2300	2000	instructions macos 2331.689 linux 2226.287	allocated macos 72.614 linux 72.614

⍝ Expected: 28 copies for the bulk form: about 13 for ChaCha8's words, about 11 for `rand`'s mapping of each word into the range, one draw at a time, and about 4 for the addition and the result. The bulk form measured about 31 on macOS. `rustymath::random::roll` draws every word first, then maps the words into their ranges by Lemire's method in a separate pass, and the bounds convert to integers in one fold. Drawing each word through `next_u64` still costs about 15 instructions. `rand`'s `fill` draws the same words, but copies each 4-byte word through a call to `memmove`, which costs several times as much.
⍝ Older batched-form measurements were 23.5–23.6 copies on CI's Linux on 22 October 2026; the bulk form's Linux budget follows its expected cost.
v+¿n⍴10
⍝⍝: 56	48	90	instructions macos 69.361 linux 62.433	allocated macos 40.54 linux 40.54

⍝ The join rule runs a join of each row with itself as one call on `X`.
⍝ Expected: 1 copy each: reads 32,768 items twice and writes 65,536.
⍝ `{⍵,⍵}⍤1 X` measures 0.9 on macOS since a join writes each row whole (`gather::interleave`, 22 October 2026), down from 2.4 to 2.7. In wasm it measured 0.91 to 1.02 in seven CI runs from 22 October 2026. CI measured 0.94 on Linux on 22 October 2026.
{⍵,⍵}⍤1 X
⍝⍝: 2	2.6	2.7	instructions macos 15.139 linux 10.203	allocated macos 8.893 linux 8.893

⍝ `X,X` measures 0.85 on macOS since a join writes each row whole, a lane from each argument in turn, straight into the result (`gather::interleave`, 22 October 2026), down from 2.3 to 2.6. In wasm it measured 0.67 to 0.74 in seven CI runs from 22 October 2026. CI measured 0.71 on Linux on 22 October 2026.
X,X
⍝⍝: 2	2.1	2	instructions macos 9.393 linux 5.541	allocated macos 8.482 linux 8.482

⍝ A matrix product of each 4 by 4 matrix in a stack with itself runs as one call of the stacked kernel.
⍝ Expected: 3 copies each: one stacked call of 8 flops an item, and the stack's overhead.
⍝ `{⍵+.×⍵}⍤2 st` cost 15.6 on macOS before 22 October 2026. It now runs as `+.×⍤2⍨ st` does. The comment on `+.×⍤2⍨ st` below names their shared gap.
⍝⍝⍝ held macos 4.48: the call for each cell and the zeroing, as `+.×⍤2⍨ st`'s comment says
{⍵+.×⍵}⍤2 st
⍝⍝: 6	11	13	instructions macos 59.043 linux 50.941	allocated macos 12.512 linux 12.512

⍝ `+.×⍤2⍨ st` cost 15.3 on macOS before 22 October 2026. That day `direct` began folding four results of a row side by side in registers without checking each index, `Frames` stopped listing the cells of an argument whose frame is the result's, and the call for each cell inlined `checked_into` and `inner_into`. Each 4×4 cell now costs about 370 instructions: about 250 in `direct` for its 64 multiply-adds, about 70 in the call that locates and checks the cell, and about 25 in zeroing the output before the cells write it. The call for each cell and the zeroing, about 95 of those 370, are the gap between its cost and its expected 3. Both cases' costs also depend on the process: on 30 October 2026 they cost 4.7 to 5.5 in some processes and 7.0 to 7.6 in others, on builds from before and after that day's changes. A speed test retries a case in the same process, so a slow process fails every retry. The cause isn't placed.
⍝⍝⍝ held macos 4.43: the call for each cell and the zeroing
+.×⍤2⍨ st
⍝⍝: 6	9.6	13	instructions macos 63.727 linux 51.975	allocated macos 12.719 linux 12.719

⍝ A fold or scan by a dfn along the rows of `X` runs as one batched map over the rows. Each stage is one call, on every row at once. Each sits beside its primitive form.
⍝ Expected: `{⍺+⍵}/X` 1.5 copies: seven batched calls over 4,096 rows, about a microsecond each; `+/X` 0.25 copies: reads 32,768 items; `{⍺+0.9×⍵}\X` 2 copies: seven batched calls over 4,096 rows, and the scan's writes; `+\X` 0.75 copies: reads and writes 32,768 items, with lanes of 8 waiting on additions.
⍝⍝⍝ held macos 3.71: each batched step first copies a column of `X`, read at stride 8, out contiguously (open.md, dfn operands of scans, folds, outer products); stage 6's loop nodes
{⍺+⍵}/X
⍝⍝: 3	4.6	3.9	instructions macos 13.356 linux 8.688	allocated macos 1.502 linux 1.502

+/X
⍝⍝: 0.5	0.83	0.78	instructions macos 13.155 linux 8.837	allocated macos 1.467 linux 1.467

⍝⍝⍝ held macos 5.92: as `{⍺+⍵}/X`, plus a `memmove` that places each step's column in the result (open.md, dfn operands of scans, folds, outer products); stage 6's loop nodes
{⍺+0.9×⍵}\X
⍝⍝: 4	9.2	8.7	instructions macos 115.94 linux 92.935	allocated macos 29.062 linux 29.062

+\X
⍝⍝: 1.5	1.6	1.1	instructions macos 9.667 linux 6.455	allocated macos 4.311 linux 4.311

⍝ A dfn under `⍠` along the first axis of `X`, beside the primitive form.
⍝ Expected: `{+/⍵}⍠0 X` 0.5 copies: as `+/⍠0 X`, which reads 32,768 items and measured 0.32 on 30 October 2026.
⍝⍝⍝ held macos 1.91: the dfn's `+/` folds the transposed view of `X` in place along strided lanes, where `+/⍠0` runs column totals (open.md, dfn operand under `⍠`); Linux and wasm hold the macOS cost as an estimate
{+/⍵}⍠0 X
⍝⍝: 1	3.5	2.8	instructions macos 28.004 linux 19.296	allocated macos 5.872 linux 5.872

⍝ Row-wise dot products. `+.×` under Rank pairs each pair of rows' items in one call and folds along them, as `+/X×Y` does.
⍝ Expected: `X+.×⍤1Y` 1 copy: as `+/X×Y`, two passes below the size at which fusion waits (`fusion::waits`). The product reads 65,536 items and writes 32,768, then the fold reads them. On 30 October 2026 it measured 1.71 on macOS, and `+/X×Y` 1.5.
X+.×⍤1Y
⍝⍝: 2	170	190	instructions macos 23.297 linux 15.914	allocated macos 6.06 linux 6.06

⍝ An inner product with a dfn `g` calls `g` once, on every triple of a row, a position along the contraction and a column. `f` then folds along the contraction.
⍝ Expected: `(⍉X) +.{⍺×⍵} X` 3 copies: the operand on 262,144 triples, about 2 copies, and a fold along the contraction; `(⍉X)+.×X` 1.5 copies: on Linux and in wasm `⍉X` computes at once below the size at which fusion waits there (`fusion::waits`) and writes a copy of `X`, about 0.5 copies, then the product reads `X` and the copy; on macOS `⍉X` waits and the product reads it through its map, and its 524,288 flops are few.
⍉X +.{⍺×⍵} X
⍝⍝: 6	12	12	instructions macos 35.131 linux 31.137	allocated macos 7.912 linux 17.912

⍝ The product on a stored transpose costs about 1 copy. `⍉X` costs about 2.9 copies of its own 32,768 items on macOS, because its eight result rows lie exactly 32 KB apart and writing them in turn stalls (22 October 2026). On 31 October 2026 the case measured 1.11 to 1.16 on macOS, within its rule budget.
⍉X +.× X
⍝⍝: 3	8	7.4	instructions macos 36.625 linux 32.789	allocated macos 8.021 linux 18.021

⍝ A small convolutional layer, written for one image and mapped over a batch of 64 images of 32 by 32 pixels and 3 channels: a 3 by 3 convolution to 32 channels through windows and a product, a bias and ReLU, 2 by 2 max pooling, and a normalization of each channel over the pooled image.
⍝ Expected: 118 copies on macOS, 257 on Linux and 474 in wasm: products of 99.5 million flops, 1,518 an item, which take 51 at Accelerate's rank-k rate, 190 at 8 in a copy's time for rustymath's own kernels and 380 at 4 in wasm; one fused pass for the bias and ReLU over 1,843,200 items, about 28; the pooling's one fold along both axes of each window, which reads those items once and writes a quarter as many, about 18; and the normalization's four passes over 460,800 items, about 21. In wasm these passes cost about 94.
⍝ On 25 October 2026 its parts measured about 94 for the convolution, 55 for the bias and ReLU, 27 for the pooling and 51 for the normalization. On 23 October 2026 the convolution's 95 split into 49 for Accelerate's product and 45 for the windows that the product reads. The pooling, one fold along both axes of each window, reads its source once, but at about 3 instructions an item, most of them in the loop that folds a window's rows along its first axis (`reduce::started`), so it costs 1.5 times its expected 18. Written as two folds, `⌈/⍠2⌈/⍠2`, it measures about 39, because the first fold writes 921,600 items that the second reads back. The expected cost leaves out building the windows. Reading them in blocks through their map (`pervasive::mapped_product`) measured 128 against 94 for building them first, and 369 against 331 on rustymath's own kernels. CI measured 473 to 519 on Linux in runs 59-61, and up to 1,221 in runs 56-58 and 62, whose hosts were busy, because its data passes the runner's 1 MB L2 and shares the host's L3.
⍝⍝⍝ held macos 221: building the windows, which the expected cost leaves out, and the pooling's fold at 1.5 times its expected 18
⍝⍝⍝ held linux 958: a single-thread slowdown since the rewrite, from about 246 ns an item in the released version to about 450
⍝⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝⍝ allows forced "⌈/⍠[2 3]ₓ": a fold of windows reads its argument whole
cnn⍤3 imgs
⍝⍝: 180	770	880	instructions macos 963.712 linux 1290.515	allocated macos 1014.521 linux 1023.13

⍝⍝ static •prefs ["kinds":"static" "int":32 "float":32]
⍝ Expected at 32 bits: 59 copies on macOS, 129 on Linux and 237 in wasm, half its 64-bit cost. CI measured 295 to 305 on Linux in runs 59-61, and up to 626 in runs 56-58 and 62, as at 64 bits.
⍝⍝⍝ held macos 118: as at 64 bits
⍝⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝⍝ allows forced "⌈/⍠[2 3]ₓ": a fold of windows reads its argument whole
cnn⍤3 imgs
⍝⍝: 89	390	570	instructions macos 753.564 linux 945.438	allocated macos 520.603 linux 524.888
