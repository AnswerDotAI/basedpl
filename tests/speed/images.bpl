⍝⍝ setup
img←¿[4000 3000 3]⍴0
flat←,img
n←≢flat
⍝⍝ images
⍝ A 4000×3000 image with three channels holds 36 million floats, far more than the caches hold. A fused program reads and writes memory once, about 1 copy, or reads it once, about 0.5, where it ends in a fold, and each of its calls passes over a block in the cache. Natively those passes overlap the pass over memory, adding about 0.25 copies to a program that writes its result; in wasm each adds about 0.4.
⍝ Expected: 1.25 copies natively for each; in wasm, `0⌈1⌊0.5+1.2×img-0.5` 3 copies, for five calls, `img×1-0.5×img` 2.2, for three, and `0⌈1⌊img×0.5+(⍳4000)÷8000` 2.6, for four.
0⌈1⌊0.5+1.2×img-0.5
⍝⍝: 2.5	2.5	6

img×1-0.5×img
⍝⍝: 2.5	2.5	4.4

0⌈1⌊img×0.5+(⍳4000)÷8000
⍝⍝: 2.5	2.5	5.2

⍝ `×` joins the pending calls on its two sides into one program that runs block by block.
⍝ Expected: 1.25 copies natively; 2.2 in wasm, for three calls.
img-0.5 × img+0.5
⍝⍝: 2.5	2.5	4.4

⍝ A reduction at the end of a fused program folds each block's results while they are in the cache. Along the first axis, it adds each block to one total for each column.
⍝ Expected: `+/flat>0.5` 0.6 copies natively: one read of memory, about 0.5, with the call over each block, folded in the cache, overlapping it; 0.9 in wasm, for one call; `+⌿img×2` 0.65 copies natively: as `+/flat>0.5`, with the column totals; 1 copy in wasm.
+/flat>0.5
⍝⍝: 1.2	1.2	1.8

⍝ On 30 October 2026 `+⌿img` cost 0.83 on macOS. The one group of `+⌿img×2`, `×+⌿`, fuses and compiles, and stores nothing.
⍝⍝⍝ held macos 1.85: the cause isn't known yet
+⌿img×2
⍝⍝: 1.3	1.3	2

⍝ `+⌿img` folds 4000 rows of 9,000 items read from memory. Each block of 8,192 column totals steps by four rows at a time, so each row's items are read in the order they lie (`reduce::columns_by`). Strips of 32 columns, each folded down every row, took 3.2 times as long (27 October 2026).
⍝ Expected: 0.6 copies: one read of memory, about 0.5, and the steps of the column totals in the cache, about 0.1.
+⌿img
⍝⍝: 1.2	1.2	1.2

⍝ `+/img` folds each pixel's 3 channels. Each length of lane up to 15 items has a loop of its own. The reshape shares `flat`'s storage.
⍝ Expected: `+/img` 0.67 copies: one read, and a third as many items written; `+/[4500000 8]⍴flat` 0.56 copies: one read, and an eighth as many items written.
+/img
⍝⍝: 1.4	1.4	1.4

+/[4500000 8]⍴flat
⍝⍝: 1.2	1.2	1.2

⍝ The comparison's Booleans count into a byte for each pixel.
⍝ Expected: 0.6 copies natively: one read of memory, about 0.5, with the count into a byte for each pixel overlapping it; 0.9 in wasm, for one call.
+/img>0.5
⍝⍝: 1.2	1.2	1.8

⍝ A comparison that a later call reads runs in that call's program, writing 1 and 0 as floats rather than storing Booleans.
⍝ Expected: 0.7 copies natively: one read of `flat`, which both calls share, about 0.5, with their passes over each block overlapping it; 1.3 in wasm, for two calls.
⍝ On 30 October 2026 `+/flat>0.5` cost 0.85 on macOS. The one group of `+/flat×flat>0.5`, `>×+/`, fuses and stores nothing, but doesn't compile, as `+/flat>0.5`'s doesn't.
+/flat×flat>0.5
⍝⍝: 1.4	1.4	2.6

⍝ A square and a halving in one compiled program. `²` lowers to multiplications, and `⊽⁻¹`'s division by 2 lowers because zero divided by zero can't arise with a nonzero constant divisor. Either one left to rustymath's kernel splits the stage, which then runs uncompiled at about 2.6 times the cost (30 October 2026, 4M items).
⍝ Expected: 0.67 copies natively: one read of memory, about 0.5, and three compiled calls over each block, folded in the cache. In wasm, which compiles nothing, 1.3 copies, as for `+/flat×flat>0.5`.
⍝⍝⍝ held wasm ~1.43: margin against noise for a case whose estimate is within 10% of its budget (30 October 2026)
+/⊽⁻¹(flat-0.5)²
⍝⍝: 1.4	1.4	2.6

⍝ A select by a comparison that a program would run reads a sample of the comparison's items to choose between a select and a gather. A sparse mask gathers the few items it picks. A dense one runs as one fused select.
⍝ Expected: `-@[0.99<]flat` 2.3 copies: a fused comparison reading `flat`, about 0.9, then a copy of `flat` with the picked items negated, about 1.4 with its pass over each block; `*@[0.3<]flat` 3.8 copies: one pass over memory, a comparison and a blend over each block, and exponentials of every item, whatever share picks `*`, about 2 copies.
⍝⍝⍝ allows forced "-@[0.99↣<]": a sparse mask computes its comparison to find the items it picks, then gathers them
-@[0.99<]flat
⍝⍝: 4.6	4.6	4.6

⍝ `*@[0.3<]flat` replaced `*@[0.5<]flat` on 21 October 2026. A mask that picks half the items sits within the sampling noise of the select's crossover, about 47% for `*`, so that case gathered in about one run in six.
⍝ Natively its exponentials cost about 2 copies of memory on CI's runner and about 3.8 on this Mac, whose memory is fast beside its cores, so its macOS expected cost is 5.6. In wasm they cost twice that, because wasm's SIMD has no fused multiply-add, and it expects 5.8.
*@[0.3<]flat
⍝⍝: 12	7.6	12

⍝ Here the calls hold three blocks of results at once, so the block stage runs smaller blocks, whose buffers stay in the cache.
⍝ Expected: 1.25 copies natively; 3.8 in wasm, for seven calls.
(flat-0.5 × flat+0.5) + flat+1 × flat-1
⍝⍝: 2.5	2.5	7.6
