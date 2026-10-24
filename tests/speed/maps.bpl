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
hv←{+/vv×loss∂ ⍵}
⍝ Directional derivative of lossw along vv.
hvw←{+/vv×lossw∂ ⍵}
⍝ One Game of Life update from a 3 by 3 neighbourhood.
life←{
  s←+/,⍵
  (s=3)∨(1 1⌷⍵)∧s=4
}
⍝ One Game of Life update over the whole grid.
lifew←{
  s←+/+/¯3 ¯3↕⍵
  (s=3)∨⍵∧s=4
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
⍝ Non-overlapping 2 by 2 max pooling.
pool←{⌈/⍠2⌈/⍠2 [[2 2]⋄[2 2]]↕⍵}
⍝ Normalize each channel across spatial positions.
norm←{
  x←((×/¯1↓⍴⍵),¯1↑⍴⍵)⍴⍵
  c←x-⍤1 (+⌿÷≢)x
  (⍴⍵)⍴c÷⍤1 √1E¯5+(+⌿÷≢)⊼c
}
⍝ Convolution, ReLU, max pooling and channel normalization.
cnn←{norm pool 0⌈cb+⍤1 ck conv ⍵}
⍝⍝ maps
⍝ Each, Rank and `⌺` first try one call of their operand on batches of all their cells, and call it once for each cell where that call stops. Maps inside `∂` batch too. Each mapped case sits beside the same computation written with whole arrays, whose cost a batched map should approach.
⍝ Expected: `{⍵×⍵+1}¨v` 2.5 copies: as `v×v+1`; `v×v+1` 2.5 copies: two passes below the fusion threshold, one copy then two reads and a write; `{+/,⍵}⌺[3 3] sg` 2 copies: nine reads for each of 16,384 windows, and their sums; `+/+/¯3 ¯3↕sg` 0.75 copies: a padded copy and two sliding passes over 16,384 items.
{⍵×⍵+1}¨v
⍝⍝: 5	5	5

v×v+1
⍝⍝: 5	5	5

⍝ Held until studied: `{+/,⍵}⌺[3 3] sg` measures 5.75 on macOS and has old Linux and wasm budgets that suggest about 15 and 14.5, more than twice its expected cost. On 22 October 2026 CI measured 7.31 on Linux and 6.35 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍝⍝ gap forced "+/": a stencil kernel should sum each window where it lies in the grid. Until then, `+/` stores the batch's 147,456 window items first, because 3-item runs fold more slowly through the window map
{+/,⍵}⌺[3 3] sg
⍝⍝: 15	30	29

⍝ Held until studied: `+/+/¯3 ¯3↕sg` measures 5.62 on macOS and has old Linux and wasm budgets that suggest about 11 and 12.5, more than twice its expected cost. On 22 October 2026 CI measured 4.6 on Linux and 4.45 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
+/+/¯3 ¯3↕sg
⍝⍝: 11	22	25

⍝ The Game of Life on a 128 by 128 Boolean grid, as a stencil and with padded windows.
⍝ Expected: `life⌺[3 3] g` 1.5 copies: a few passes over nine 1-byte reads for each of 16,384 windows; `life⌺["size":[3 3] "edge":"wrap"] g` 1.5 copies: as the stencil with fill; `lifew g` 0.75 copies: sliding sums and comparisons over 16,384 items.
⍝ Held until studied: `life⌺[3 3] g` measures 5.48 on macOS and has old Linux and wasm budgets that suggest about 20 and 22, more than twice its expected cost. On 22 October 2026 CI measured 6.9 on Linux and 7.29 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍝⍝ gap forced "+/": a stencil kernel should sum each window where it lies in the grid. Until then, `+/` stores the batch's 147,456 window items first, because 3-item runs fold more slowly through the window map
life⌺[3 3] g
⍝⍝: 20	40	44

⍝ Held until studied: `life⌺["size":[3 3] "edge":"wrap"] g` measures 6.31 on macOS and has old Linux and wasm budgets that suggest about 19.5 and 19.5, more than twice its expected cost. On 22 October 2026 CI measured 7.37 on Linux and 7.94 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍝⍝ gap forced "+/": a stencil kernel should sum each window where it lies in the grid. Until then, `+/` stores the batch's 147,456 window items first, because 3-item runs fold more slowly through the window map
life⌺["size":[3 3] "edge":"wrap"] g
⍝⍝: 19	39	39

⍝ Held until studied: `lifew g` measures 5.45 on macOS and has old Linux and wasm budgets that suggest about 12 and 12.5, more than twice its expected cost. On 22 October 2026 CI measured 4.29 on Linux and 4.82 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
lifew g
⍝⍝: 10	24	25

⍝ A guard on each cell's value runs as `⍚` runs a selector for each cell. Each side runs once, on the cells that take it. In the first case every cell takes the second body. In the second, about half the cells take each body, as in the `⍚` beside it.
⍝ Expected: `{⍵<0?0;⍵}¨v` 1.5 copies: one tolerant comparison, then the whole argument as the result; `{⍵<0.5?0;√⍵}¨v` 6 copies: a tolerant comparison, a split and a join of each side, and square roots of half the items; `(0.5≤)⍚[0 √] v` 4 copies: a tolerant comparison, square roots of every item, and a blend.
{⍵<0?0;⍵}¨v
⍝⍝: 3	3	6

⍝ Held until studied on Linux and in wasm: `{⍵<0.5?0;√⍵}¨v` has old Linux and wasm budgets that suggest about 25 and 27.5, more than twice its expected cost. CI measured 10.8 on Linux and 12.1 in wasm on 22 October 2026, and 10.0 to 11.5 and 12.1 to 12.2 in the two runs of 23 October, just under or at twice its expected cost, where a budget of twice it would fail by chance. Those budgets stay at the old value. On macOS it measured 12.41 on 21 October and 9.39 on 23 October. Its macOS budget is twice its expected cost.
{⍵<0.5?0;√⍵}¨v
⍝⍝: 12	50	55

⍝ Held until studied: `(0.5≤)⍚[0 √] v` measures 7.62 to 8.05 on macOS, about twice its expected cost, so a budget of twice that cost would fail by chance. Its old Linux and wasm budgets suggest about 16 and 18, more than twice its expected cost. On 22 October 2026 CI measured 8.96 on Linux and 11.5 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
(0.5≤)⍚[0 √] v
⍝⍝: 16	32	36

⍝ The gradient of a loss mapped over the rows of `X`, whose parameter every row shares, then per-example gradients, then a Hessian–vector product through the mapped loss. Each sits beside its form with `+.×`, or its closed form.
⍝ Expected: `loss∂ thz` 4 copies: the primal over 32,768 items, about 1 copy, and a transpose of about 3; `lossw∂ thz` 3 copies: a product and `1○` over 4,096 items, and its transpose; `{f∂ ⍵}⍤1 X` 5 copies: sines and cosines of 32,768 items, and their products; `2×(1○X)×2○X` 4.5 copies: sines and cosines of 32,768 items; `hv∂ thz` 8 copies: about twice the gradient; `hvw∂ thz` 6 copies: about twice the gradient.
⍝ Held until studied: `loss∂ thz` has an old wasm budget that suggests about 7500, more than twice its expected cost. On 22 October 2026 CI measured 8.09 in wasm. Its wasm budget stays at the old value.
loss∂ thz
⍝⍝: 8	8	15000

⍝ Held until studied: `lossw∂ thz` has old Linux and wasm budgets that suggest about 11.5 and 16.5, more than twice its expected cost. On 22 October 2026 CI measured 8.39 on Linux and 7.21 in wasm. Its Linux and wasm budgets stay at the old value.
lossw∂ thz
⍝⍝: 6	23	33

⍝ Held until studied: `{f∂ ⍵}⍤1 X` measures 13.01 on macOS and has old Linux and wasm budgets that suggest about 5500 and 8500, more than twice its expected cost. On 22 October 2026 CI measured 11.6 on Linux and 20.6 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{f∂ ⍵}⍤1 X
⍝⍝: 5300	11000	17000

⍝ Held until studied: `2×(1○X)×2○X` has an old wasm budget that suggests about 30.5, more than twice its expected cost. On 22 October 2026 CI measured 16.1 in wasm. Its wasm budget stays at the old value.
2×(1○X)×2○X
⍝⍝: 9	9	61

⍝ Held until studied: `hv∂ thz` has old Linux and wasm budgets that suggest about 13500 and 20000, more than twice its expected cost. On 22 October 2026 CI measured 14.5 on Linux and 16.5 in wasm. Its Linux and wasm budgets stay at the old value.
hv∂ thz
⍝⍝: 16	27000	40000

⍝ Held until studied: `hvw∂ thz` has old Linux and wasm budgets that suggest about 24.5 and 38.5, more than twice its expected cost. On 22 October 2026 CI measured 17.9 on Linux and 16.1 in wasm. Its Linux and wasm budgets stay at the old value.
hvw∂ thz
⍝⍝: 12	49	77

⍝ An attached rule inside Each, beside the same rule on the whole array.
⍝ Expected: `{+/sq¨⍵}∂ v` 3 copies: as `{+/sq ⍵}∂ v`; `{+/sq ⍵}∂ v` 3 copies: the primal and the attached rule over every item.
⍝ Held until studied: `{+/sq¨⍵}∂ v` measures 7.73 on macOS and has old Linux and wasm budgets that suggest about 35000 and 55000, more than twice its expected cost. On 22 October 2026 CI measured 7.16 on Linux and 8.18 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{+/sq¨⍵}∂ v
⍝⍝: 32000	70000	110000

⍝ Held until studied: `{+/sq ⍵}∂ v` measures 7.59 on macOS and has old Linux and wasm budgets that suggest about 15 and 17, more than twice its expected cost. On 22 October 2026 CI measured 6.84 on Linux and 8.08 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{+/sq ⍵}∂ v
⍝⍝: 15	30	34

⍝ 1000 Each calls on one atom, in turn. A map over one cell of atoms calls its operand without trying a batched call.
⍝ Expected: 27 copies: 1,000 calls of about 200 ns each.
⍝ Held until studied: `{⍵+1}¨⍣1000,0` has old Linux and wasm budgets that suggest about 75 and 140, more than twice its expected cost. On 22 October 2026 CI measured 52.5 on Linux and 46.2 in wasm. Its Linux and wasm budgets stay at the old value.
{⍵+1}¨⍣1000,0
⍝⍝: 54	150	280

⍝ A map of 400 scalars against `v`. Repeated for each cell, `v` would hold 26 million items, past the memory limit of a batched run. The map then calls its operand once for each cell, holding one copy of `v` at a time.
⍝ Expected: 1000 copies: 400 cells, each about 2.5 copies in chunks.
{+/(v-⍵)×v-⍵}¨cs
⍝⍝: 2000	2000	4000

⍝ Four cells of 256 by 256 floats, whose operand stops at `⍳`, which has no batch rule. The dfn remembers the stop. Later calls skip the batched try, which would run `1○` on every cell before stopping.
⍝ Expected: 18 copies: sines of four cells of `n` items, about 16, and a linear search of each.
⍝ Held until studied: `{r←1○⍵ ⋄ (,r)⍳0.5}⍤2 big` measures 41.85 on macOS and has old Linux and wasm budgets that suggest about 150 and 290, more than twice its expected cost. On 22 October 2026 CI measured 115 on Linux and 123 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍝⍝ allows fallbacks "an earlier stop: no batch rule": the case times a dfn that remembers its stop at `⍳`
{
  r←1○⍵
  (,r)⍳0.5
}⍤2 big
⍝⍝: 78	300	580

⍝ Rank of a dfn on each row of `X` makes one batched call. The subtraction pads each row's mean, `+/÷≢ ⍵`, over the row.
⍝ Expected: 1 copy each: row sums of 32,768 items and one padded subtraction.
⍝ Held until studied: `{⍵-+/÷≢ ⍵}⍤1 X` measures 2.6 on macOS and has old Linux and wasm budgets that suggest about 4.95 and 11, more than twice its expected cost. On 22 October 2026 CI measured 3.1 on Linux and 3.78 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{⍵-+/÷≢ ⍵}⍤1 X
⍝⍝: 5.9	9.9	22

⍝ Held until studied: `X-(+/X)÷8` has old Linux and wasm budgets that suggest about 2.85 and 6, more than twice its expected cost. On 22 October 2026 CI measured 2.35 on Linux and 2.85 in wasm. Its Linux and wasm budgets stay at the old value.
X-(+/X)÷8
⍝⍝: 2	5.7	12

⍝ An outer product of a dfn makes one batched call, with both arguments laid over the result's frame.
⍝ Expected: `(⍳256){⍺×⍵+1}⊗⍳256` 1 copy: the operand on views laid over the frame, and one write of `n` items; `(⍳256)×⊗1+⍳256` 0.5 copies: one write of `n` items.
⍝ Held until studied: `(⍳256){⍺×⍵+1}⊗⍳256` measures 4.49 on macOS and has old Linux and wasm budgets that suggest about 4.9 and 11.5, more than twice its expected cost. On 22 October 2026 CI measured 3.99 on Linux and 4.09 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
(⍳256){⍺×⍵+1}⊗⍳256
⍝⍝: 9.9	9.8	23

⍝ Held until studied: `(⍳256)×⊗1+⍳256` has old Linux and wasm budgets that suggest about 1.15 and 3, more than twice its expected cost. On 22 October 2026 CI measured 0.89 on Linux and 1.1 in wasm. Its Linux and wasm budgets stay at the old value.
(⍳256)×⊗1+⍳256
⍝⍝: 1	2.3	6

⍝ A roll in a batched call draws once, on the whole frame, from the stream that a call for each cell would draw from in turn.
⍝ Expected: 28 copies each: about 13 for ChaCha8's words, about 11 for `rand`'s mapping of each word into the range, one draw at a time, and about 4 for the addition and the result (`rl_`, 21 October 2026). They measure about 31 on macOS. `rustymath::random::roll` draws every word first, then maps the words into their ranges by Lemire's method in a separate pass, and the bounds convert to integers in one fold (`wq_`, 22 October 2026). Drawing each word through `next_u64` still costs about 15 instructions. `rand`'s `fill` draws the same words, but copies each 4-byte word through a call to `memmove`, which costs several times as much.
⍝ CI measured 23.6 and 23.5 on Linux on 22 October 2026, and their Linux budgets follow their expected cost. Held until studied: in wasm they measured 61 to 75 in seven CI runs from 22 October 2026, 2.2 to 2.7 times their expected cost, so their wasm budgets stay at the old values.
{⍵+¿10}¨v
⍝⍝: 56	56	550

v+¿n⍴10
⍝⍝: 56	56	530

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
⍝ `{⍵+.×⍵}⍤2 st` measures about 5.2 on macOS, down from 15.6, for the reasons `+.×⍤2⍨ st` gives below. Held until studied: in wasm it measured 7.6 to 9.2 in seven CI runs from 22 October 2026, about 2.8 times its expected cost. On 22 October 2026 CI measured 6.72 on Linux, where twice its expected cost, 6, would fail, and 7.41 in wasm. Its Linux budget, which suggests about 60, and its wasm budget stay at the old values.
{⍵+.×⍵}⍤2 st
⍝⍝: 6	120	310

⍝ `+.×⍤2⍨ st` measures about 5.3 on macOS, down from 15.3 on 22 October 2026. That day `direct` began folding four results of a row side by side in registers without checking each index, `Frames` stopped listing the cells of an argument whose frame is the result's, and the call for each cell inlined `checked_into` and `inner_into`. Each 4×4 cell now costs about 370 instructions: about 250 in `direct` for its 64 multiply-adds, about 70 in the call that locates and checks the cell, and about 25 in zeroing the output before the cells write it. Held until studied: in wasm it measured 7.5 to 9.0 in seven CI runs from 22 October 2026, about 2.8 times its expected cost. On 22 October 2026 CI measured 6.54 on Linux, where twice its expected cost, 6, would fail, and 7.36 in wasm. Its Linux budget, which suggests about 60, and its wasm budget stay at the old values.
+.×⍤2⍨ st
⍝⍝: 6	120	270

⍝ A fold or scan by a dfn along the rows of `X` runs as one batched map over the rows. Each stage is one call, on every row at once. Each sits beside its primitive form.
⍝ Expected: `{⍺+⍵}/X` 1.5 copies: seven batched calls over 4,096 rows, about a microsecond each; `+/X` 0.25 copies: reads 32,768 items; `{⍺+0.9×⍵}\X` 2 copies: seven batched calls over 4,096 rows, and the scan's writes; `+\X` 0.75 copies: reads and writes 32,768 items, with lanes of 8 waiting on additions.
⍝ Held until studied: `{⍺+⍵}/X` measures 3.77 on macOS and has old Linux and wasm budgets that suggest about 8 and 8.5, more than twice its expected cost. On 22 October 2026 CI measured 3.14 on Linux and 2.6 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{⍺+⍵}/X
⍝⍝: 7.6	16	17

⍝ Held until studied: `+/X` has old Linux and wasm budgets that suggest about 0.75 and 1, more than twice its expected cost. On 22 October 2026 CI measured 0.46 on Linux and 0.43 in wasm. Its Linux and wasm budgets stay at the old value.
+/X
⍝⍝: 0.5	1.5	2

⍝ Held until studied: `{⍺+0.9×⍵}\X` measures 6.43 on macOS and has old Linux and wasm budgets that suggest about 13.5 and 16.5, more than twice its expected cost. On 22 October 2026 CI measured 6.2 on Linux and 5.17 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{⍺+0.9×⍵}\X
⍝⍝: 14	27	33

+\X
⍝⍝: 1.5	1.5	1.5

⍝ An inner product with a dfn `g` calls `g` once, on every triple of a row, a position along the contraction and a column. `f` then folds along the contraction.
⍝ Expected: `(⍉X) +.{⍺×⍵} X` 3 copies: the operand on 262,144 triples, about 2 copies, and a fold along the contraction; `(⍉X)+.×X` 1.5 copies: `⍉X` is eager below fusion's `MIN_ITEMS` and writes a copy of `X`, about 0.5 copies, then the product reads `X` and the copy, and its 524,288 flops are few.
⍝ Held until studied: `(⍉X) +.{⍺×⍵} X` measures 14.32 on macOS and has old Linux and wasm budgets that suggest about 28 and 36.5, more than twice its expected cost. On 22 October 2026 CI measured 16.6 on Linux and 17.9 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
(⍉X) +.{⍺×⍵} X
⍝⍝: 28	56	73

⍝ Held until studied: `(⍉X)+.×X` measures 3.47 on macOS and has old Linux and wasm budgets that suggest about 9 and 12.5, more than twice its expected cost. On 22 October 2026 CI measured 4.48 on Linux and 3.75 in wasm. Its macOS, Linux and wasm budgets stay at the old value. Studied on 22 October 2026: the product on a stored transpose costs about 1 copy. `⍉X` costs about 2.9 copies of its own 32,768 items on macOS, where a 4100-row `X` costs about 2.2: its eight result rows lie exactly 32 KB apart, and writing them in turn stalls.
(⍉X)+.×X
⍝⍝: 8	18	25

⍝ A small convolutional layer, written for one image and mapped over a batch of 64 images of 32 by 32 pixels and 3 channels: a 3 by 3 convolution to 32 channels through windows and a product, a bias and ReLU, 2 by 2 max pooling, and a normalization of each channel over the pooled image.
⍝ Expected: 118 copies on macOS, 257 on Linux and 474 in wasm: products of 99.5 million flops, 1,518 an item, which take 51 at Accelerate's rank-k rate, 190 at 8 in a copy's time for rustymath's own kernels and 380 at 4 in wasm; one fused pass for the bias and ReLU over 1,843,200 items, about 28; the pooling's read of those items and write of a quarter as many, about 18; and the normalization's four passes over 460,800 items, about 21. In wasm these passes cost about 94.
⍝ Held until studied: on 23 October 2026 `cnn⍤3 imgs` measured 255 on macOS, about twice its expected cost. Its parts measured about 95 for the convolution, 49 of them Accelerate's product and 45 the windows that the product reads, 54 for the bias and ReLU, 55 for the pooling and 52 for the normalization. The expected cost leaves out building the windows. Reading them in blocks through their map (`pervasive::mapped_product`) measured 128 against 94 for building them first, and 369 against 331 on rustymath's own kernels. Its macOS budget is twice its cost. Its Linux and wasm budgets scale the macOS budget by the ratios of the expected costs, until CI measures them.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝ allows forced "⌈/": a fold of windows reads its argument whole
⍝⍝ gap forced "+⌿": the variance's `+⌿` computes the centred values `c`, 460,800 items, on their own because a name holds them, where its group should compute them as one of its calls
⍝⍝ gap stored "held outside the graph": the normalization's centred values, which two calls read, should compute in the group that reads them
cnn⍤3 imgs
⍝⍝: 510	1200	2100

⍝⍝ static •prefs ["kinds":"static" "int":32 "float":32]
⍝ Held until studied, as at 64 bits. Expected at 32 bits: 59 copies on macOS, 129 on Linux and 237 in wasm, half its 64-bit cost. On 24 October 2026 it measured 132 on macOS, about twice that, as at 64 bits. Its macOS budget is twice its cost. Its Linux and wasm budgets scale the macOS budget by the ratios of the expected costs, until CI measures them.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝ allows forced "⌈/": a fold of windows reads its argument whole
⍝⍝ gap forced "+⌿": the variance's `+⌿` computes the centred values `c`, 460,800 items, on their own because a name holds them, where its group should compute them as one of its calls
⍝⍝ gap stored "held outside the graph": the normalization's centred values, which two calls read, should compute in the group that reads them
cnn⍤3 imgs
⍝⍝: 270	580	1100
