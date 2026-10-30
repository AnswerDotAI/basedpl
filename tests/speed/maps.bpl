⍝⍝ setup
v←¿65536⍴0
n←≢v
sg←¿[128 128]⍴0
g←0.6<¿[128 128]⍴0
X←¿[4096 8]⍴0
thz←¿8⍴0
vv←¿8⍴0
cs←¿400⍴0
big←¿[4 256 256]⍴0
st←¿[4096 4 4]⍴0
irow←(¿[10000 8]⍴100)ₓ
nest←⊂⍤1⁝irow
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
imgs←¿[64 32 32 3]⍴0
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
Y←¿[4096 8]⍴0
⍝⍝ maps
⍝ Each, Rank and `⌺` first try one call of their operand on batches of all their cells, and call it once for each cell where that call stops. Maps inside `∂` batch too. Each mapped case sits beside the same computation written with whole arrays, whose cost a batched map should approach.
⍝ Expected: `{⍵×⍵+1}¨v` 2.5 copies: as `v×v+1`; `v×v+1` 2.5 copies: two passes below the fusion threshold, one copy then two reads and a write; `{+/,⍵}⌺[3 3] sg` 2 copies: nine reads for each of 16,384 windows, and their sums; `+/+/¯3 ¯3↕sg` 0.75 copies: a padded copy and two sliding passes over 16,384 items.
⍝⍝⍝ route batching
{⍵×⍵+1}¨v
⍝⍝: 5	5	5

v×v+1
⍝⍝: 5	5	5

⍝ Each over a nested array whose 10,000 items are vectors of 8 small integers. Each passes its operand each item, as Rank passes each row of the matrix that holds them. Where every item is a compact array of one layout and dtype, a batched map stacks the items into one array and runs as Rank does.
⍝ Expected: `{⍵+1ₓ}¨nest` and `{+/⍵}¨nest` 2.5 copies: as `{⍵+1ₓ}⍤1⁝irow` and `+/⍤1 irow`, which cost 1.88 and 2.02 on 30 October 2026, and a copy of the 80,000 one-byte items into one array.
⍝ On 30 October 2026 `{⍵+1ₓ}¨nest` cost 33 times its Rank form, and `{+/⍵}¨nest` 12 times. Stacking costs about 18 ns an item, and assembling `{⍵+1ₓ}¨nest`'s 10,000 result arrays about 30 more (`meta/open.md`, "Each over a nested array of arrays"). Before then each called its dfn once for each item, at 416 and 357.
⍝⍝⍝ held macos 62.9 linux ~107 wasm ~88.1: stacking and assembling the items (open.md, "Each over a nested array of arrays"); Linux and wasm hold estimates of 1.7 and 1.4 times the macOS cost
⍝⍝⍝ route batching
{⍵+1ₓ}¨nest
⍝⍝: 5	5	5

⍝⍝⍝ held macos 24.7 linux ~42 wasm ~34.6: stacking and assembling the items (open.md, "Each over a nested array of arrays"); Linux and wasm hold estimates of 1.7 and 1.4 times the macOS cost
⍝⍝⍝ route batching
{+/⍵}¨nest
⍝⍝: 5	5	5

⍝⍝⍝ held macos 5.37 linux 7.02 wasm 7.04: open.md lists the stencils among the speed audit's outliers that compiling didn't fix
⍝⍝⍝ gap forced "+/": a stencil kernel should sum each window where it lies in the grid. Until then, `+/` stores the batch's 147,456 window items first, because 3-item runs fold more slowly through the window map
{+/,⍵}⌺[3 3] sg
⍝⍝: 4	4	4

⍝⍝⍝ held macos 4.19 linux 4.45 wasm 4.31: open.md lists wide and 2-D window folds among the speed audit's outliers
+/+/¯3 ¯3↕sg
⍝⍝: 1.5	1.5	1.5

⍝ The Game of Life on a 128 by 128 Boolean grid, as a stencil and with padded windows.
⍝ Expected: `life⌺[3 3] g` 1.5 copies: a few passes over nine 1-byte reads for each of 16,384 windows; `life⌺["size":[3 3] "edge":"wrap"] g` 1.5 copies: as the stencil with fill; `lifew g` 0.75 copies: sliding sums and comparisons over 16,384 items.
⍝⍝⍝ held macos 4.54 linux 7.19 wasm 7.25: open.md lists the stencils among the speed audit's outliers that compiling didn't fix
⍝⍝⍝ gap forced "+/": a stencil kernel should sum each window where it lies in the grid. Until then, `+/` stores the batch's 147,456 window items first, because 3-item runs fold more slowly through the window map
life⌺[3 3] g
⍝⍝: 3	3	3

⍝⍝⍝ held macos 4.8 linux 7.47 wasm 8.26: open.md lists the stencils among the speed audit's outliers that compiling didn't fix
⍝⍝⍝ gap forced "+/": a stencil kernel should sum each window where it lies in the grid. Until then, `+/` stores the batch's 147,456 window items first, because 3-item runs fold more slowly through the window map
life⌺["size":[3 3] "edge":"wrap"] g
⍝⍝: 3	3	3

⍝⍝⍝ held macos 4.15 linux 4.34 wasm 4.83: open.md lists wide and 2-D window folds among the speed audit's outliers
lifew g
⍝⍝: 1.5	1.5	1.5

⍝ A guard on each cell's value runs as `⍚` runs a selector for each cell. Each side runs once, on the cells that take it. In the first case every cell takes the second body. In the second, about half the cells take each body, as in the `⍚` beside it.
⍝ Expected: `{⍵<0?0;⍵}¨v` 1.5 copies: one tolerant comparison, then the whole argument as the result; `{⍵<0.5?0;√⍵}¨v` 6 copies: a tolerant comparison, a split and a join of each side, and square roots of half the items; `(0.5≤)⍚[0 √] v` 4 copies: a tolerant comparison, square roots of every item, and a blend.
⍝⍝⍝ allows forced "{⍵<0?0;⍵}": a guard reads its predicate's items to split the cells
{⍵<0?0;⍵}¨v
⍝⍝: 3	3	6

⍝⍝⍝ held wasm 11.6: the cause isn't known yet
⍝⍝⍝ allows forced "{⍵<0.5?0;√⍵}": a guard reads its predicate's items to split the cells
{⍵<0.5?0;√⍵}¨v
⍝⍝: 12	12	12

⍝⍝⍝ held macos 7.66 linux 8.62 wasm 10.1: open.md lists the masked selects among the speed audit's outliers
⍝⍝⍝ gap forced "0.5↣≤⍚[0⍨ √]": the select should run in its mask's fused pass, which `√` blocks because it can't run on every item under dynamic kinds (open.md, "Masked selects and `⍚`")
(0.5≤)⍚[0 √] v
⍝⍝: 8	8	8

⍝ The gradient of a loss mapped over the rows of `X`, whose parameter every row shares, then per-example gradients, then a Hessian–vector product through the mapped loss. Each sits beside its form with `+.×`, or its closed form.
⍝ Expected: `loss∂ thz` 4 copies: the primal over 32,768 items, about 1 copy, with sines of 4,096 items at 7 copies for each 65,536, about 0.45, and a transpose of about 3; `lossw∂ thz` 3 copies: a product over 32,768 items, `1○` over 4,096 items, about 0.45, and its transpose; `{f∂ ⍵}⍤1 X` 9 copies: sines and cosines of 32,768 items, half of `n`, at 7 copies each as `1○v` counts them, then the squares, the row sums and each item's product of sine and cosine, about 2; `2×(1○X)×2○X` 7.5 copies: sines and cosines of 32,768 items, half of `n`, at 7 copies each as `1○v` counts them, and two products; `hv∂ thz` 8 copies: about twice the gradient; `hvw∂ thz` 6 copies: about twice the gradient.
⍝ On 27 October 2026 `loss∂ thz` measured 6.02 on macOS. Its wasm budget is twice its native budget, as `1○v`'s are. On 22 October 2026 CI measured 8.09 in wasm.
⍝⍝⍝ gap stopped "a function called on each item": the row loss should trace as one batched map
loss∂ thz
⍝⍝: 8	8	16

⍝ On 27 October 2026 `lossw∂ thz` measured 5.22 on macOS, close to twice its expected cost. Each of its two products of the 4096×8 `X` and a vector costs about 1.6 copies, where reading `X` costs 0.25: `Xb+.×thz` measured 1.59. On 22 October 2026 CI measured 8.39 on Linux and 7.21 in wasm.
⍝⍝⍝ held wasm 6.61: each of its two products of `X` and a vector cost about 1.6 copies before rustymath's narrow-row kernels of 30 October 2026, which CI hasn't yet timed in wasm
lossw∂ thz
⍝⍝: 6	6	6

⍝ On 27 October 2026 `{f∂ ⍵}⍤1 X` measured 12.38 on macOS. Its budgets are twice the expected cost: 18 on macOS and Linux, and 36 in wasm, where `1○v`'s budgets in arrays.bpl give sine and cosine twice their native cost. On 22 October 2026 CI measured 11.6 on Linux and 20.6 in wasm.
{f∂ ⍵}⍤1 X
⍝⍝: 18	18	36

⍝ `•explain` shows no fused stage: `1○X` and `2○X` each run over all of `X`, one after the other. On 27 October 2026 it measured 8.24 on macOS. The budgets are twice the expected cost: 15 on macOS and Linux, and 30 in wasm, where `1○v`'s budgets in arrays.bpl give sine and cosine twice their native cost. On 22 October 2026 CI measured 16.1 in wasm.
2× 1○X × 2○X
⍝⍝: 15	15	30

⍝⍝⍝ held linux 15.6 wasm 18.9: the cause isn't known yet
⍝⍝⍝ gap stopped "a function called on each item": the mapped row loss should trace through its nested derivative
hv∂ thz
⍝⍝: 16	16	16

hvw∂ thz
⍝⍝: 12	12	12

⍝ An attached rule inside Each, beside the same rule on the whole array.
⍝ Expected: `{+/sq¨⍵}∂ v` 3 copies: as `{+/sq ⍵}∂ v`; `{+/sq ⍵}∂ v` 3 copies: the primal and the attached rule over every item.
⍝⍝⍝ held linux 7.83 wasm 7.19: the cause isn't known yet
⍝⍝⍝ gap stopped "a function called on each item": Each should trace the attached derivative rule over the whole array
{+/sq¨⍵}∂ v
⍝⍝: 6	6	6

{+/sq ⍵}∂ v
⍝⍝: 6	6	6

⍝ 1000 Each calls on one atom, in turn. A map over one cell of atoms calls its operand without trying a batched call.
⍝ Expected: 27 copies: 1,000 calls of about 200 ns each.
⍝ Linux: 46 copies, because scalar work costs about 1.7 times as many copies on CI's x86 runner, as `v⍳w`'s comment in `arrays.bpl` says.
{⍵+1}¨⍣1000,0
⍝⍝: 54	92	54

⍝ A map of 400 scalars against `v`. Repeated for each cell, `v` would hold 26 million items, past the memory limit of a batched run. The map then calls its operand once for each cell, holding one copy of `v` at a time.
⍝ Expected: 1000 copies: 400 cells, each about 2.5 copies in chunks.
⍝ CI measured 812 to 852 on Linux in most runs from 56 to 62, and up to 1,232 in runs whose hosts were busy.
⍝⍝⍝ busy linux 1232: its data passes CI's L2 (open.md, "Footprint copies")
{+/ v-⍵ × v-⍵}¨cs
⍝⍝: 1300	1300	2600

⍝ Four cells of 256 by 256 floats. The batched run takes the sines of all four cells as one call, and `⍳` searches each cell's own table on slices of the batch (`searched_cells`).
⍝ Expected: 18 copies: sines of four cells of `n` items, about 16, and a linear search of each.
⍝⍝⍝ held macos 35.3 linux 144 wasm 169: the cause isn't known yet
⍝⍝⍝ allows forced "⍳": a search reads its table's items
{
  r←1○⍵
  ,r ⍳ 0.5
}⍤2 big
⍝⍝: 36	36	36

⍝ Rank of a dfn on each row of `X` makes one batched call. The subtraction pads each row's mean, `+/÷≢ ⍵`, over the row.
⍝ Expected: 1 copy each: row sums of 32,768 items and one padded subtraction.
⍝⍝⍝ held macos 1.85 linux 2.83 wasm 3.03: the cause isn't known yet
{⍵ - +/÷≢ ⍵}⍤1 X
⍝⍝: 2	2	2

⍝⍝⍝ held linux 2.57 wasm 2.81: the cause isn't known yet
X - +/X ÷ 8
⍝⍝: 2	2	2

⍝ An outer product of a dfn makes one batched call, with both arguments laid over the result's frame.
⍝ Expected: `(⍳256){⍺×⍵+1}⊗⍳256` 1 copy: the operand on views laid over the frame, and one write of `n` items; `(⍳256)×⊗1+⍳256` 0.5 copies: one write of `n` items.
⍝⍝⍝ held macos 4.54 linux 4.29 wasm 4.15: the cause isn't known yet
⍳256 {⍺×⍵+1}⊗ ⍳256
⍝⍝: 2	2	2

⍝⍝⍝ held macos 0.914 linux 1.02 wasm 1.17: the cause isn't known yet
⍳256 ×⊗ 1+⍳256
⍝⍝: 1	1	1

⍝ A roll in a batched call draws once, on the whole frame, from the stream that a call for each cell would draw from in turn.
⍝ Expected: 28 copies each: about 13 for ChaCha8's words, about 11 for `rand`'s mapping of each word into the range, one draw at a time, and about 4 for the addition and the result. They measure about 31 on macOS. `rustymath::random::roll` draws every word first, then maps the words into their ranges by Lemire's method in a separate pass, and the bounds convert to integers in one fold. Drawing each word through `next_u64` still costs about 15 instructions. `rand`'s `fill` draws the same words, but copies each 4-byte word through a call to `memmove`, which costs several times as much.
⍝ CI measured 23.6 and 23.5 on Linux on 22 October 2026, and their Linux budgets follow their expected cost.
⍝⍝⍝ held wasm 75.3: the cause isn't known yet
{⍵+¿10}¨v
⍝⍝: 56	56	56

⍝⍝⍝ held wasm 76.2: the cause isn't known yet
⍝⍝⍝ allows forced "¿": a roll reads its argument whole
v+¿n⍴10
⍝⍝: 56	56	56

⍝ The join rule runs a join of each row with itself as one call on `X`.
⍝ Expected: 1 copy each: reads 32,768 items twice and writes 65,536.
⍝ `{⍵,⍵}⍤1 X` measures 0.9 on macOS since a join writes each row whole (`gather::interleave`, 22 October 2026), down from 2.4 to 2.7. In wasm it measured 0.91 to 1.02 in seven CI runs from 22 October 2026. CI measured 0.94 on Linux on 22 October 2026.
{⍵,⍵}⍤1 X
⍝⍝: 2	2	2

⍝ `X,X` measures 0.85 on macOS since a join writes each row whole, a lane from each argument in turn, straight into the result (`gather::interleave`, 22 October 2026), down from 2.3 to 2.6. In wasm it measured 0.67 to 0.74 in seven CI runs from 22 October 2026. CI measured 0.71 on Linux on 22 October 2026.
X,X
⍝⍝: 2	2	2

⍝ A matrix product of each 4 by 4 matrix in a stack with itself runs as one call of the stacked kernel.
⍝ Expected: 3 copies each: one stacked call of 8 flops an item, and the stack's overhead.
⍝ `{⍵+.×⍵}⍤2 st` cost 15.6 on macOS before 22 October 2026. It now runs as `+.×⍤2⍨ st` does. The comment on `+.×⍤2⍨ st` below names their shared gap.
⍝⍝⍝ held macos 4.48 linux 7.14 wasm 7.44: the call for each cell and the zeroing, as `+.×⍤2⍨ st`'s comment says
{⍵+.×⍵}⍤2 st
⍝⍝: 6	6	6

⍝ `+.×⍤2⍨ st` cost 15.3 on macOS before 22 October 2026. That day `direct` began folding four results of a row side by side in registers without checking each index, `Frames` stopped listing the cells of an argument whose frame is the result's, and the call for each cell inlined `checked_into` and `inner_into`. Each 4×4 cell now costs about 370 instructions: about 250 in `direct` for its 64 multiply-adds, about 70 in the call that locates and checks the cell, and about 25 in zeroing the output before the cells write it. The call for each cell and the zeroing, about 95 of those 370, are the gap between its cost and its expected 3. Both cases' costs also depend on the process: on 30 October 2026 they cost 4.7 to 5.5 in some processes and 7.0 to 7.6 in others, on builds from before and after that day's changes. A speed test retries a case in the same process, so a slow process fails every retry. The cause isn't placed.
⍝⍝⍝ held macos 4.43 linux 7 wasm 7.33: the call for each cell and the zeroing
+.×⍤2⍨ st
⍝⍝: 6	6	6

⍝ A fold or scan by a dfn along the rows of `X` runs as one batched map over the rows. Each stage is one call, on every row at once. Each sits beside its primitive form.
⍝ Expected: `{⍺+⍵}/X` 1.5 copies: seven batched calls over 4,096 rows, about a microsecond each; `+/X` 0.25 copies: reads 32,768 items; `{⍺+0.9×⍵}\X` 2 copies: seven batched calls over 4,096 rows, and the scan's writes; `+\X` 0.75 copies: reads and writes 32,768 items, with lanes of 8 waiting on additions.
⍝⍝⍝ held macos 3.71 linux 3.2: the cause isn't known yet
⍝⍝⍝ held wasm 1.65: margin against noise for a case within 10% of its budget (30 October 2026)
{⍺+⍵}/X
⍝⍝: 3	3	3

⍝⍝⍝ held linux 0.48: the cause isn't known yet
+/X
⍝⍝: 0.5	0.5	0.5

⍝⍝⍝ held macos 5.92 linux 5.99 wasm 5.21: the cause isn't known yet
{⍺+0.9×⍵}\X
⍝⍝: 4	4	4

+\X
⍝⍝: 1.5	1.5	1.5

⍝ A dfn under `⍠` along the first axis of `X`, beside the primitive form.
⍝ Expected: `{+/⍵}⍠0 X` 0.5 copies: as `+/⍠0 X`, which reads 32,768 items and measured 0.32 on 30 October 2026.
⍝⍝⍝ held macos 1.91 linux ~1.91 wasm ~1.91: the dfn form pays a transpose of `X` that `+/⍠0` avoids; Linux and wasm hold the macOS cost as an estimate
{+/⍵}⍠0 X
⍝⍝: 1	1	1

⍝ Row-wise dot products. `+.×` under Rank pairs each pair of rows' items in one call and folds along them, as `+/X×Y` does.
⍝ Expected: `X+.×⍤1Y` 1 copy: as `+/X×Y`, two passes below fusion's `MIN_ITEMS`. The product reads 65,536 items and writes 32,768, then the fold reads them. On 30 October 2026 it measured 1.71 on macOS, and `+/X×Y` 1.5.
⍝⍝⍝ held macos 1.1: margin against noise for a case within 10% of its budget (30 October 2026)
X+.×⍤1Y
⍝⍝: 2	2	2

⍝ An inner product with a dfn `g` calls `g` once, on every triple of a row, a position along the contraction and a column. `f` then folds along the contraction.
⍝ Expected: `(⍉X) +.{⍺×⍵} X` 3 copies: the operand on 262,144 triples, about 2 copies, and a fold along the contraction; `(⍉X)+.×X` 1.5 copies: `⍉X` is eager below fusion's `MIN_ITEMS` and writes a copy of `X`, about 0.5 copies, then the product reads `X` and the copy, and its 524,288 flops are few.
⍝⍝⍝ held linux 6.09 wasm 5.71: the cause isn't known yet
⍉X +.{⍺×⍵} X
⍝⍝: 6	6	6

⍝ The product on a stored transpose costs about 1 copy. `⍉X` costs about 2.9 copies of its own 32,768 items on macOS, because its eight result rows lie exactly 32 KB apart and writing them in turn stalls (22 October 2026).
⍝⍝⍝ held macos 2.9 linux 4.71 wasm 3.94: `⍉X` writes its eight result rows 32 KB apart
⍉X +.× X
⍝⍝: 3	3	3

⍝ A small convolutional layer, written for one image and mapped over a batch of 64 images of 32 by 32 pixels and 3 channels: a 3 by 3 convolution to 32 channels through windows and a product, a bias and ReLU, 2 by 2 max pooling, and a normalization of each channel over the pooled image.
⍝ Expected: 118 copies on macOS, 257 on Linux and 474 in wasm: products of 99.5 million flops, 1,518 an item, which take 51 at Accelerate's rank-k rate, 190 at 8 in a copy's time for rustymath's own kernels and 380 at 4 in wasm; one fused pass for the bias and ReLU over 1,843,200 items, about 28; the pooling's one fold along both axes of each window, which reads those items once and writes a quarter as many, about 18; and the normalization's four passes over 460,800 items, about 21. In wasm these passes cost about 94.
⍝ On 25 October 2026 its parts measured about 94 for the convolution, 55 for the bias and ReLU, 27 for the pooling and 51 for the normalization. On 23 October 2026 the convolution's 95 split into 49 for Accelerate's product and 45 for the windows that the product reads. The pooling, one fold along both axes of each window, reads its source once, but at about 3 instructions an item, most of them in the loop that folds a window's rows along its first axis (`reduce::started`), so it costs 1.5 times its expected 18. Written as two folds, `⌈/⍠2⌈/⍠2`, it measures about 39, because the first fold writes 921,600 items that the second reads back. The expected cost leaves out building the windows. Reading them in blocks through their map (`pervasive::mapped_product`) measured 128 against 94 for building them first, and 369 against 331 on rustymath's own kernels. CI measured 473 to 519 on Linux in runs 59-61, and up to 1,221 in runs 56-58 and 62, whose hosts were busy, because its data passes the runner's 1 MB L2 and shares the host's L3.
⍝⍝⍝ held macos 221 wasm 842: building the windows, which the expected cost leaves out, and the pooling's fold at 1.5 times its expected 18
⍝⍝⍝ busy linux 1221: its data passes CI's L2 (open.md, "Footprint copies")
⍝⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝⍝ allows forced "⌈/⍠[2 3]ₓ": a fold of windows reads its argument whole
cnn⍤3 imgs
⍝⍝: 180	390	720

⍝⍝ static •prefs ["kinds":"static" "int":32 "float":32]
⍝ Expected at 32 bits: 59 copies on macOS, 129 on Linux and 237 in wasm, half its 64-bit cost. CI measured 295 to 305 on Linux in runs 59-61, and up to 626 in runs 56-58 and 62, as at 64 bits.
⍝⍝⍝ held macos 118 wasm 389: as at 64 bits
⍝⍝⍝ busy linux 626: its data passes CI's L2 (open.md, "Footprint copies")
⍝⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝⍝ allows forced "⌈/⍠[2 3]ₓ": a fold of windows reads its argument whole
cnn⍤3 imgs
⍝⍝: 89	200	360
