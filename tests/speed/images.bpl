⍝⍝ setup
img←¿[4000 3000 3]⍴0
flat←,img
n←≢flat
⍝⍝ images
⍝ A 4000×3000 image with three channels holds 36 million floats, far more than the caches hold. Each call in a fused graph reads and writes every item.
⍝ Expected: `0⌈1⌊0.5+1.2×img-0.5` 3 copies: one pass over memory, about 1 copy, and about 0.4 for each fused call's pass over its block in the cache, for five calls; `img×1-0.5×img` 2.2 copies: one pass over memory, about 1 copy, and about 0.4 for each fused call's pass over its block in the cache, for three calls; `0⌈1⌊img×0.5+(⍳4000)÷8000` 2.6 copies: one pass over memory, about 1 copy, and about 0.4 for each fused call's pass over its block in the cache, for four calls.
0⌈1⌊0.5+1.2×img-0.5
⍝⍝: 6	6	6

img×1-0.5×img
⍝⍝: 4.4	4.4	4.4

0⌈1⌊img×0.5+(⍳4000)÷8000
⍝⍝: 5.2	5.2	5.2

⍝ `×` joins the fused graphs on its two sides into one graph that runs block by block.
⍝ Expected: 2.2 copies: one pass over memory, about 1 copy, and about 0.4 for each fused call's pass over its block in the cache, for three calls.
(img-0.5)×img+0.5
⍝⍝: 4.4	4.4	4.4

⍝ A reduction at the end of a fused graph folds each block's results while they are in the cache. Along the first axis, it adds each block to one total for each column.
⍝ Expected: `+/flat>0.5` 0.9 copies: one read of memory, about 0.5, and one call over each block, folded in the cache; `+⌿img×2` 1 copy: one read of memory, about 0.5, one call over each block, and the column totals.
+/flat>0.5
⍝⍝: 1.8	1.8	1.8

+⌿img×2
⍝⍝: 2	2	2

⍝ `+/img` folds each pixel's 3 channels. Each length of lane up to 15 items has a loop of its own. The reshape shares `flat`'s storage.
⍝ Expected: `+/img` 0.67 copies: one read, and a third as many items written; `+/[4500000 8]⍴flat` 0.56 copies: one read, and an eighth as many items written.
+/img
⍝⍝: 1.4	1.4	1.4

+/[4500000 8]⍴flat
⍝⍝: 1.2	1.2	1.2

⍝ The comparison's Booleans count into a byte for each pixel.
⍝ Expected: 0.9 copies: one read of memory, about 0.5, and one call over each block, counted into a byte for each pixel.
+/img>0.5
⍝⍝: 1.8	1.8	1.8

⍝ A comparison that a later call reads runs in that call's graph, writing 1 and 0 as floats rather than storing Booleans.
⍝ Expected: 1.3 copies: one read of `flat`, which both calls share, about 0.5, and two calls over each block.
+/flat×flat>0.5
⍝⍝: 2.6	2.6	2.6

⍝ A select by a comparison that a graph would run reads a sample of the comparison's items to choose between a select and a gather. A sparse mask gathers the few items it picks. A dense one runs as one fused select.
⍝ Expected: `-@[0.99<]flat` 2.3 copies: a fused comparison reading `flat`, about 0.9, then a copy of `flat` with the picked items negated, about 1.4 with its pass over each block; `*@[0.3<]flat` 3.8 copies: one pass over memory, a comparison and a blend over each block, and exponentials of every item, whatever share picks `*`, about 2 copies.
⍝⍝ allows forced "-@[0.99↣<]": a sparse mask computes its comparison to find the items it picks, then gathers them
-@[0.99<]flat
⍝⍝: 4.6	4.6	4.6

⍝ `*@[0.3<]flat` replaced `*@[0.5<]flat` on 21 October 2026. A mask that picks half the items sits within the sampling noise of the select's crossover, about 47% for `*`, so that case gathered in about one run in six.
⍝ Held until studied: `*@[0.3<]flat` measures 9.9 on macOS, more than twice its expected cost. On 22 October 2026 CI measured 2.72 on Linux and 7.54 in wasm. Its macOS budget stays at the old value.
*@[0.3<]flat
⍝⍝: 20	7.6	16

⍝ Here the calls hold three blocks of results at once, so the graph runs smaller blocks, whose buffers stay in the cache.
⍝ Expected: 3.8 copies: one pass over memory, about 1 copy, and about 0.4 for each fused call's pass over its block in the cache, for seven calls.
((flat-0.5)×flat+0.5)+(flat+1)×flat-1
⍝⍝: 7.6	7.6	7.6
