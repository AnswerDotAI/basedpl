⍝⍝ setup
mx←¯0.5+¿[(1024 size 8) 64]⍴0
my←1○+/mx
mp←["w":0.25×¯0.5+¿[64 256]⍴0 "b":256⍴0 "v":0.1×¯0.5+¿256⍴0 "c":0]
⍝ Mean squared error of a one-hidden-layer tanh network.
mlp←{
  [x y]←⍺
  [w b v c]←⍵
  h←7○b(+⍤1)x+.×w
  +/÷≢ ⊼y-c+h+.×v
}
gx←¿[(1000 size 8) 32]⍴0
gi←=⊗⍨⍳32
glow←>⊗⍨⍳32 ⋄ components←25 size 4
gp←["alpha":¿components⍴0 "mu":¿[components 32]⍴0 "q":0.1×¿[components 32]⍴0 "l":0.1×¿[components 32 32]⍴0]
⍝ Gaussian-mixture objective with a Wishart prior, in index notation. `n` names the points, `k` the components, and `d` and `e` the dimensions. The log-sum-exp shifts read detached values, which `∂` treats as constants.
gmm←{
  [alpha mu q l]←⍵
  [[n d] [k e]]←•index[⍴⍺ ⍴mu]
  L←l.[k d e]×glow.[d e]
  Q←L + *q.[k d] × gi.[d e]
  r←⍺.[n e]-mu.[k e]
  sq←+/⍠d⊼+/⍠e⁝Q×r
  t←alpha.[k]++/⍠d⁝q.[k d] - 0.5×sq
  m←⌈/⍠k•detach t
  am←⌈/•detach alpha
  +/m+⍟+/⍠k*t-m + (0.5× +/,⊼*q + +/,⊼L) - ≢⍺ × am+⍟+/*alpha-am
}
⍝ The same objective in rank notation, with the index form's detached shifts. `⍉⍤2` transposes each component's matrix. The two spellings compute one objective and share each case's expected cost.
gmmr←{
  [alpha mu q l]←⍵
  L←l×⍤2 glow
  Q←L+(*q)×¹ ²gi
  sq←+/⊼(⍺(-⍤1)² ¹mu)+.×⍤2⍉⍤2⁝Q
  t←(alpha++/q)-0.5×sq
  m←⌈⌿•detach t
  am←⌈/•detach alpha
  (+/m+⍟+⌿*t-⍤1m) + (0.5×(+/,⊼*q)++/,⊼L) - (≢⍺)×am+⍟+/*alpha-am
}
kx←¿[(16384 size 16) 32]⍴0
kc←¿[(64 size 4) 32]⍴0
⍝ Sum of squared distances to the nearest cluster centres.
kmeans←{+/⌊⌿(+/⊼⍵ +⊗ +/⊼⍺)-2×⍵+.×⍉⍺}
⍝ The same sum in index notation, which subtracts before it squares. `n` names the points, `c` the centres and `d` the dimensions.
kmeansi←{
  [[n d] c]←•index[⍴⍺ ≢⍵]
  +/⌊/⍠c+/⍠d⊼⍺.[n d]-⍵.[c d]
}
sg←¿[(128 size 8) (128 size 8)]⍴0
sk←[1 2 1⋄2 4 2⋄1 2 1]÷16
n←65536 size 1024
dx←¿(1048576 size 1024)⍴0
dv←¿n⍴0
dw←dv-0.5
dp←["w":¿[64 256]⍴0 "b":¿256⍴0 "v":¿256⍴0 "c":1]
rw←{0.1×¯0.5+¿⍵⍴0}
tokens←64 size 8 ⋄ tid←¿[(4 size 2) tokens]⍴512ₓ
ttg←¿[(4 size 2) tokens]⍴512ₓ
tmk←¯1E9× ⍳tokens <⊗ ⍳tokens
tp←["E":rw[512 128] "P":rw[tokens 128] "ga":128⍴1 "ba":128⍴0 "wqkv":rw[128 3 4 32] "wo":rw[128 128] "gb":128⍴1 "bb":128⍴0 "wu":rw[128 512] "cu":512⍴0 "wd":rw[512 128] "cd":128⍴0 "gc":128⍴1 "bc":128⍴0 "U":rw[128 512]]
gelu_scale←√2÷π1
⍝ GELU activation using its tanh approximation.
gelu←{0.5×⍵×1+7○gelu_scale×⍵+0.044715×⍵*3}
⍝ Mean token cross-entropy of a pre-norm transformer decoder block, in index notation. `b` names the sequences, `t` and `u` the tokens, `d` the features, `r` the query, key and value, `h` the heads, `j` a head's features, `f` the MLP's units and `c` the classes. `norm` normalizes along `d`, with a learned gain and bias.
dec←{
  [ids tg]←⍺
  [E P ga ba wqkv wo gb bb wu cu wd cd gc bc U]←⍵
  [[b t] u [d r h j] f c]←•index[⍴ids ≢P ⍴wqkv ≢cu ≢⍉U]
  norm←{[g o]←⍺ ⋄ z←⍵-(+/÷≢)⍠d⍵ ⋄ o.[d]+g.[d]×z÷√1E¯5+(+/÷≢)⍠d⊼z}
  x←E.[ids.[b t] d]+P.[t d]
  ⍝ q, k and v, each by sequence, token, head and the head's feature.
  [q k v]←:[r b t h j]⍉⁻+/⍠d([ga ba]norm x)×wqkv.[d r h j]
  s←tmk.[t u] + +/⍠j⁝q.[b t h j]×k.[b u h j] ÷ √≢j
  e←*s-⌈/⍠u⁝s
  x←x++/⍠[h j](+/⍠u e÷+/⍠u⁝e × v.[b u h j])×wo.[j+(≢j)×h d]
  a←[gb bb]norm x
  x←x+cd.[d]++/⍠f(gelu cu.[f]++/⍠d⁝a×wu.[d f])×wd.[f d]
  lg←+/⍠d([gc bc]norm x)×U.[d c]
  m←⌈/⍠c⁝lg
  +/÷≢ , m+⍟+/⍠c*lg-m - lg.[b t tg.[b t]]
}
cx←¿[(8 size 2) 3 (32 size 8) (32 size 8)]⍴0
ck←0.2×¯0.5+¿[16 3 3 3]⍴0
cy←¿[(8 size 2) 16 (32 size 8) (32 size 8)]⍴0
⍝ A 3 by 3 convolution of 8 images of 3 channels, each 32 by 32, into 16 channels, in index notation. `•edge` gives NaN for a position past an edge, and a NaN position reads 0, so the images are padded with zeros.
conv←{
  [[n c i j] o a b]←•index[⍴⍺ ≢⍵ 3 3]
  [h w]←2↓⍴⍺
  [n o i j]⍉⁻+/⍠[c a b]⍺.[n c h•edge⁝i+a-1 w•edge⁝j+b-1]×⍵.[o c a b]
}
⍝ Mean squared error of one convolutional layer with tanh units, against the targets `cy`.
closs←{+/÷≢ ,⊼cy-7○⍺conv⍵}
hu←¿[(512 size 16) (512 size 16)]⍴0
⍝ One step of heat diffusion: each cell moves towards the mean of its four neighbours. A neighbour past an edge is the edge cell itself (`clamp`).
heat←{
  [i j]←•index⍴⍵ ⋄ [h w]←⍴⍵ ⋄ clamp←•edge"clamp"
  : ⍵ + 0.2× ⍵.[(h clamp i-1) j]+⍵.[(h clamp i+1) j]+⍵.[i (w clamp j-1)]+⍵.[i (w clamp j+1)] - 4×⍵
}
⍝⍝ objectives
⍝ Models and stencils, timed without `∂`. For each function that `[derivatives]` differentiates, this is the cost that its gradient's cost is measured against.
⍝ The mean squared error of a network with one hidden layer of 256 tanh units, over 1024 inputs of 64 values. Its parameters are in a record.
⍝ Expected: 61 copies on macOS, 108 on Linux and 214 in wasm: a product of 33.5 million flops, 512 an item, which takes 17 at Accelerate's rank-k rate, 64 at 8 in a copy's time for rustymath's own kernels and 128 at 4 in wasm; `7○` on 262,144 items, 40 natively, at the 10 copies for each 65,536 items that rustymath's speed test expects of `Tanh`, and 80 in wasm, where vector arithmetic costs about twice; and the other passes, about 4 natively and 6 in wasm.
⍝⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
[mx my] mlp mp
⍝⍝: 92	120	330	instructions macos 176.198 linux 272.348	allocated macos 100.071 linux 124.321

⍝ ADBench's Gaussian mixture objective, for 1000 points in 32 dimensions and 25 components, without its constant terms and with its Wishart prior at gamma 1 and m 0. It's written in index notation. Each component's precision factor has the exponentials of `q` on its diagonal and `l` below it.
⍝ Expected: 63 copies on macOS, 135 on Linux and 247 in wasm: 25 products of 1000×32 by 32×32, 781 flops an item in all, which take 26 at Accelerate's rank-k rate, 98 at 8 in a copy's time for rustymath's own kernels and 195 at 4 in wasm; and three passes over 800,000 items, about 12 copies each, 37 natively and 52 in wasm.
⍝ On 27 October 2026, after a change to its period step, the index form measured 72.5 on macOS, against 72 for the tacit form earlier that day, and up to 99 during a video call. CI measured 198 to 237 on Linux in most runs from 56 to 62.
gx gmm gp
⍝⍝: 95	240	400	instructions macos 280.837 linux 265.107	allocated macos 136.945 linux 161.973

⍝ The same objective in rank notation (`gmmr`).
⍝ Expected: 63 copies on macOS, 135 on Linux and 247 in wasm, as in index notation. On 1 November 2026 it measured 77.5 to 82.1 on macOS, against 74.6 to 82.6 for the index form. Its `+.×⍤2` computes the 800,000-item difference once, as the index form's contraction writes its laid-out copy of it once.
⍝⍝⍝ allows forced "+.×⍤2": a product reads its arguments whole
gx gmmr gp
⍝⍝: 95	230	370	instructions macos 177.259 linux 173.933	allocated macos 122.572 linux 147.072

⍝ The k-means cost sums the squared distance from each of 16384 points to the nearest of 64 centres, in 32 dimensions.
⍝ Expected: 84 copies on macOS, 178 on Linux and 326 in wasm: a product of 64×32 by 32×16384, 67 million flops, 1,024 an item, which takes 34 at Accelerate's rank-k rate, 128 at 8 in a copy's time for rustymath's own kernels and 256 at 4 in wasm; and passes over a million distances, about 50 natively and 70 in wasm.
⍝ CI measured 247 to 267 on Linux in runs 56 and 59-61, and 311 to 338 in wasm in most runs from 56 to 62.
kx kmeans kc
⍝⍝: 130	270	460	instructions macos 67.787 linux 79.84	allocated macos 69.627 linux 86.19

⍝ The same cost in index notation. Both forms are kept because they trade speed for accuracy. The tacit form expands each squared distance as |x|²+|c|²−2x·c, which runs as one matrix product and stays faster, but loses accuracy where a point is near a centre, because the terms nearly cancel. The index form subtracts first, so its error stays proportional to the distance.
⍝ Expected: 208 copies natively and 410 in wasm: 33.5 million differences, each squared and summed, 1,536 flops an item, at 8 in a copy's time for rustymath's own kernels and 4 in wasm; and the million distances, written and then read by the `⌊` fold, about 16 natively and 24 in wasm.
⍝ On 27 October 2026 it measured 1,089 on macOS, while its fold copied `x.[n d]` across all 33,554,432 items of the grid ("repeats of each item"). Since that fold reads the repeats in place, it measures about 780.
⍝⍝⍝ held macos 781: the fold over the grid runs at about 0.2 ns an item, where a fold that kept its sums in registers would come near the expected cost
kx kmeansi kc
⍝⍝: 320	1100	2200	instructions macos 405.564 linux 387.535	allocated macos 24.619 linux 24.623

⍝ A pre-norm transformer decoder block over 4 sequences of 64 tokens: width 128, 4 heads of width 32, an MLP of width 512 with GELU in its tanh form, and a cross-entropy loss over 512 tokens. Its parameters are in a record. `dec` gives the mean loss, and is written in index notation.
⍝ Expected: 155 copies on macOS, 354 on Linux and 684 in wasm: products of 142.6 million flops, 2,176 an item, which take 72.5 at Accelerate's rank-k rate, 272 at 8 in a copy's time for rustymath's own kernels and 544 at 4 in wasm; and the elementwise passes, about 82 natively and 140 in wasm. Those are GELU's `7○` and its polynomial on 131,072 items, about 30; the cross-entropy's exponentials on 131,072 items, about 18; the attention's scaling, mask and softmax on 65,536 items, about 13; three layer norms, about 11; and the bias adds and residuals, about 10.
⍝ On 27 October 2026 it measured 173 to 202 on macOS, the higher figure during a video call. About half the gap comes from layout: agreement by name puts the left operand's axes first, so lines such as `g.[d]×z` and `cu.[f]+…` lay their results out transposed, and later reads copy them back. A change to that order, listed in `meta/open.md`, would remove those copies.
⍝ CI measured 390 to 448 on Linux in runs 56-61, and 639 to 667 in wasm in runs 57-59 and 61.
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
[tid ttg] dec tp
⍝⍝: 240	480	790	instructions macos 2066.642 linux 3840.821	allocated macos 1000.483 linux 1423.529

⍝ A 3 by 3 blur of a 128 by 128 grid. `⌺` calls its operand once, on a batch of all the windows.
⍝ Expected: 1.5 copies: nine multiply-adds for each of 16,384 windows, read from the cache.
⍝⍝⍝ held macos 6.07: open.md lists the stencils among the speed audit's outliers that compiling didn't fix
{+/,sk×⍵}⌺3 3 sg
⍝⍝: 3	15	16	instructions macos 40.571 linux 31.014	allocated macos 25.868 linux 25.868

⍝ One step of heat diffusion on a 512 by 512 grid, 4 times `n`, reading each cell's four neighbours through `•edge`'s clamped positions.
⍝ Expected: 9 copies: one fused pass of seven calls over 262,144 items, which reads each neighbour from forward runs, as `hu+0.2×(hu+hu+hu+hu)-4×hu` measures it. Wasm, twice that.
⍝ On 27 October 2026 it measured 26 to 31 on macOS.
⍝⍝⍝ held macos 26: its two column-shifted reads through `•edge`'s clamped positions copy their whole input, because a run of positions doesn't end at a row's edge, and ending runs there costs more than the copy saves
heat hu
⍝⍝: 18	58	94	instructions macos 130.672 linux 94.234	allocated macos 182.123 linux 182.146

⍝ The loss of one convolutional layer with tanh units: 8 images of 3 channels, each 32 by 32, into 16 channels through a 3 by 3 kernel, against the targets `cy`.
⍝ Expected: 29 copies on macOS, 39 on Linux and 78 in wasm: the windows' copy, 221,184 items, about 3.4; a product of 3.5 million multiply-adds, 108 flops an item, which takes 3.6 at Accelerate's rank-k rate, 13.5 at 8 in a copy's time for rustymath's own kernels and 27 at 4 in wasm; `7○` on 131,072 items, 20 natively and 40 in wasm; and the loss's pass, about 2.
⍝ On 27 October 2026 it measured 111 to 125 on macOS, because a fold computed its argument, a copy of 1,572,864 items, instead of reading through its group. Since the fold along `c`, `a` and `b` runs as one product (`eval::folded_along`), it measures 56 to 75, the higher figures during a video call. In one timing during the call, the convolution took about half of the case's time, against its expected 7 copies of 29, and the same convolution over the interior windows alone, without `•edge`, took about two thirds of that for 88% as many windows. `•explain` declines to read 352,256 items in place: the windows and the result's transpose.
⍝⍝⍝ held macos 55: the gap from the expected 29 isn't placed
⍝⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
cx closs ck
⍝⍝: 58	110	190	instructions macos 288.556 linux 268.612	allocated macos 146.179 linux 150.113

⍝⍝ derivatives
⍝ `∂` beside the function it differentiates. A unit argument takes one pass, whose tangent is an array of the result's shape. The bar's "Primal cost" row in the design notes asks that the primal keeps its kernels, and the tangent adds whole-array calls beside it.
⍝ Expected: `1○dx` 112 copies: sines of a million items, 16 times `n`, at 7 copies each, as `1○v` counts them; `{1○dx+⍵}∂0` 116 copies: cosines of a million items at 7 copies each, with the sum fused into their pass. `∂` runs no sines, because its result is the tangent alone.
⍝ In wasm both expect twice their native costs, 224 and 232, as `1○v` does.
1○dx
⍝⍝: 170	120	470	instructions macos 93.045 linux 53.277	allocated macos 32.119 linux 32.119

⍝ CI measured 124 to 128 on Linux in runs 56 and 59-61, and 389 to 423 in wasm in runs 59-61. On 30 October 2026 it measured 144 on macOS.
{1○dx+⍵}∂0
⍝⍝: 180	170	580	instructions macos 107.898 linux 62.442	allocated macos 36.253 linux 36.253

⍝ A chain of fusible calls, whose tangent fuses too.
⍝ Expected: `(dx-0.5)×dx+0.5` 24 copies: one fused pass over a million items, 16 times `n`; `{(dx-⍵)×dx+⍵}∂0.5` 24 copies: the tangent's fused pass. `∂` skips the primal, because the result holds only the tangent.
⍝ CI measured 17 to 21 on Linux in most runs from 56 to 62.
dx-0.5 × dx+0.5
⍝⍝: 48	53	94	instructions macos 26.586 linux 17.285	allocated macos 96.462 linux 96.462

{dx-⍵ × dx+⍵}∂0.5
⍝⍝: 48	53	120	instructions macos 40.359 linux 29.646	allocated macos 38.296 linux 38.296

⍝ Gradients of sums of an elementwise function of 65,536 items.
⍝ Expected: `{+/√⍵}∂ dv` 3 copies: a square root and a division of each item, in one pass; `{+/|⍵}∂ dw` 1.2 copies: the sign of each item times the cotangent, in one pass. Their forward folds `+/√dv` and `+/|dw` cost 2.39 and 1.61 copies on 30 October 2026.
⍝⍝⍝ gap stopped "a result whose type depends on values": the gradient of a sum of square roots should keep a guarded real trace
{+/√⍵}∂ dv
⍝⍝: 6	19	17	instructions macos 38.021 linux 27.43	allocated macos 41.768 linux 41.761

{+/|⍵}∂ dw
⍝⍝: 2.4	14	27	instructions macos 10.821 linux 7.053	allocated macos 20.277 linux 20.277

⍝ `√ₐ` gives a real result whatever the values, so its gradient keeps its trace under dynamic kinds and replays as one compiled stage with its two recorded selects. Expected: `{+/√ₐ⍵}∂ dv` 3 copies, as `{+/√⍵}∂ dv` expects: a square root and a division of each item, in one pass. `+/√ₐdv` costs what `+/√dv` costs: 3.14 and 3.04 copies on 1 November 2026.
{+/√ₐ⍵}∂ dv
⍝⍝: 6	20	16	instructions macos 28.452 linux 19.041	allocated macos 31.326 linux 31.326
+/√ₐdv
⍝⍝: 6	5.7	4.7	instructions macos 11.904 linux 7.263	allocated macos 8.461 linux 8.461

⍝ A JVP of the MLP's loss in `[objectives]` along a fixed direction `dp`, which takes one pass through `+.×`, Rank, `7○` and `+/`.
⍝ Expected: 122 copies on macOS, 216 on Linux and 428 in wasm: the primal and one tangent pass of the same size.
⍝⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝⍝ allows itemwise "item pairs": `mp+⍵×dp` pairs the parameter record's few arrays one at a time, each by a whole-array call
⍝⍝⍝ gap stopped "a result whose type depends on values": the MLP JVP should trace record-valued parameter arithmetic; a pervasive call on one item inside `∂`'s rule for `mp+⍵×dp` plans no settled type, under static kinds too
{[mx my] mlp mp+⍵×dp}∂0
⍝⍝: 190	230	500	instructions macos 322.269 linux 560.694	allocated macos 170.494 linux 218.994

⍝ Gradients of the MLP, Gaussian mixture and k-means objectives in `[objectives]`, each from one run that records its linear calls and one transpose of the record. The bar's "Gradient cost" row asks for a small multiple of the objective's own cost.
⍝ Expected: `[mx my] mlp∂ mp` 120 copies on macOS, 261 on Linux and 520 in wasm: the primal, a transpose with two products, which take 17 each at Accelerate's rank-k rate, 64 on rustymath's own kernels and 128 in wasm, and `7○`'s derivative with the rest, about 25 natively and 50 in wasm; `gx gmm∂ gp` 189 copies on macOS, 405 on Linux and 741 in wasm: about three times the objective; `kx kmeans∂ kc` 252 copies on macOS, 534 on Linux and 978 in wasm: about three times the objective.
[mx my] mlp∂ mp
⍝⍝: 180	260	530	instructions macos 660.617 linux 900.047	allocated macos 429.907 linux 495.039

⍝ The tacit form that this index form replaced measured up to 932 on Linux on CI on 22 and 23 October 2026, more than twice its expected cost. CI measured the index form at 484 to 511 on Linux in most runs from 56 to 62, and 602 to 963 in wasm.
⍝ On 27 October 2026 the index form measured 345 to 375 on macOS during a video call, against 239 for the tacit form earlier that day. Both forms trace under dynamic kinds too, since range facts show that `⍟` reads a non-negative argument. Traced, under static kinds, the index form's gradient is the faster.
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
gx gmm∂ gp
⍝⍝: 290	730	920	instructions macos 250.523 linux 301.063	allocated macos 225.177 linux 284.71

⍝ The gradient of the rank spelling.
⍝ Expected: 189 copies on macOS, 405 on Linux and 741 in wasm, as in index notation. On 1 November 2026 it measured 173 to 179 on macOS, against 152 to 158 for the index form, with one more contraction; once a fold of a product sums out the points first, 153.0 against 148.4 (plan, gs13_'s entry).
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
gx gmmr∂ gp
⍝⍝: 290	670	990	instructions macos 253.56 linux 366.331	allocated macos 242.228 linux 325.228

⍝ CI measured 459 to 489 on Linux in most runs from 56 to 62.
kx kmeans∂ kc
⍝⍝: 380	730	1000	instructions macos 106.299 linux 139.244	allocated macos 105.591 linux 139.153

⍝ The gradient of the index form of the k-means cost in `[objectives]`.
⍝ Expected: 624 copies natively and 1,230 in wasm: about three times the objective.
⍝ On 27 October 2026 it measured 2,113 to 2,312 on macOS, and 2,116 later that day. Rewriting the contraction over the subtraction would avoid storing the grid, but changes rounding, and Jeremy put that choice after the release, as `meta/open.md` records. In CI's first four Linux runs, on 28 October 2026, each run's lowest cost after its retries was 10,728 to 12,682 copies, about 1,900 ns an item. Single timings within a run ranged from 10,728 to 18,156. On macOS with rustymath's own linear algebra, which Linux runs, it measures 285 ns an item, 22% more than with Accelerate. Most of the rest comes from the grid. On CI's runner, passes over it run at memory speed, and each grid-sized array that a call writes fresh pays first-touch page faults of about 2.3 ns an item. The test fails a case only when every retry is over its budget. In wasm CI measured 5,013 to 7,400 in runs 56 to 62.
⍝⍝⍝ held macos 1942: its gradient stores the 268 MB difference grid (open.md, "kmeans' difference grid")
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
kx kmeansi∂ kc
⍝⍝: 820	6300	6000	instructions macos 782.68 linux 654.044	allocated macos 305.649 linux 305.659

⍝ The gradient of the decoder block in `[objectives]`, from one run that records its linear calls and one transpose of the record.
⍝ Expected: 465 copies on macOS, 1060 on Linux and 2050 in wasm: about three times the objective.
⍝ On 27 October 2026 the index form measured 810 to 864 on macOS, with other work running, against 432 for the tacit form it replaced. Under dynamic kinds, `√` and `⍟` stop both forms' traces, so `∂` runs both without a program, and the index form's run is the slower. Since a cotangent's sum along adjacent axes runs as one fold, which a product takes (`eval::folded_along`), it measures 630 to 649. Part of the rest is the copies from agreement by name's axis order that the objective's comment describes. It now measures 582 on macOS. CI measured 1,277 to 1,501 on Linux in most runs from 56 to 62, and 1,904 to 2,416 in wasm.
⍝⍝⍝ gap stopped "a result whose type depends on values": normalization should keep guarded real traces through square root and logarithm
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
⍝⍝⍝ gap forced "+/": the gradient's fold of 65,536 items should run in its pending argument's fused pass
[tid ttg] dec∂ tp
⍝⍝: 700	1500	2400	instructions macos 11875.721 linux 18200.413	allocated macos 6953.7 linux 8143.266

⍝ The gradient of the convolutional layer's loss in `[objectives]`, with respect to its kernel.
⍝ Expected: 87 copies on macOS, 117 on Linux and 234 in wasm: about three times the objective.
⍝ On 27 October 2026 it measured 112 to 122 on macOS. CI measured 180 to 188 on Linux in most runs from 56 to 62, and 227 to 234 in wasm.
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
cx closs∂ ck
⍝⍝: 140	300	380	instructions macos 303.36 linux 351.103	allocated macos 224.399 linux 264.819

⍝⍝ static •prefs ["kinds":"static" "int":32 "float":32]
⍝ Expected at 32 bits: the decoder block 78 copies on macOS, 177 on Linux and 342 in wasm, and its gradient 233, 530 and 1025, half their 64-bit costs. On 27 October 2026 the index form measured 104 and 367 on macOS, against 68 and 190 for the tacit form it replaced. Since a cotangent's sum along adjacent axes runs as one fold, which a product takes (`eval::folded_along`), the gradient measures 187 to 188, and the decoder block measures 102 to 105. The decoder block now measures 101 on macOS. CI measured it at 247 to 284 on Linux, and 341 to 361 in wasm in most runs from 56 to 62.
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
[tid ttg] dec tp
⍝⍝: 120	320	480	instructions macos 1342.373 linux 2353.918	allocated macos 552.581 linux 764.127

⍝ CI measured 568 to 647 on Linux in runs 59-61, and 848 to 1,117 in wasm in every run but 58.
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
[tid ttg] dec∂ tp
⍝⍝: 350	690	1100	instructions macos 2598.654 linux 6293.95	allocated macos 1645.815 linux 2240.716

⍝ Expected at 32 bits: 34 copies on macOS, 80 on Linux and 148 in wasm. The products, the square and fold along `d`, and the rest take half their 64-bit time. CI's widths case `m+.×m` shows this for products: f32 takes 0.47 to 0.60 of the f64 time natively and 0.50 to 0.52 in wasm. Building `r` doesn't take half: on rustymath's own kernels, the products and the building of `r` that they read cost about 24 copies more than one product of the same size from stored matrices, at either width. That's 12 more than half natively, and twice that in wasm. Where those copies go isn't placed. On macOS, where Accelerate computes the products, the case takes 0.53 of its 64-bit time. On 29 October 2026 it measured 38.7 on macOS, and 105 with rustymath's own kernels, against 123 before `rustymath::linalg::product` moved each tile's rows whole. In runs 56 to 62 CI measured 124 to 142 on Linux and 139 to 175 in wasm. The square and fold costs about twice one read of the products' 800,000 results, at both widths.
gx gmm gp
⍝⍝: 68	150	210	instructions macos 241.835 linux 229.724	allocated macos 77.145 linux 89.672

⍝ The same objective in rank notation, at 32 bits.
⍝ Expected: 34 copies on macOS, 80 on Linux and 148 in wasm, as in index notation. On 1 November 2026 it measured 40.6 on macOS, against 40.5 for the index form. Its `+.×⍤2ₛ` computes the 800,000-item difference here too.
⍝⍝⍝ allows forced "+.×⍤2ₛ": a product reads its arguments whole
gx gmmr gp
⍝⍝: 68	130	210	instructions macos 143.966 linux 143.507	allocated macos 66.744 linux 78.994

⍝ Expected at 32 bits: the gradient 95 copies on macOS, 203 on Linux and 371 in wasm, half its 64-bit costs. It runs from a program here, where `⍟` stops the 64-bit section's trace under dynamic kinds. On 29 October 2026 it measured 77 on macOS. CI measured 249 to 298 on Linux in runs 56 to 62.
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
gx gmm∂ gp
⍝⍝: 150	320	430	instructions macos 205.647 linux 233.894	allocated macos 135.637 linux 165.67

⍝ The gradient of the rank spelling, at 32 bits.
⍝ Expected: 95 copies on macOS, 203 on Linux and 371 in wasm, as in index notation. On 1 November 2026 it measured 83.3 on macOS, against 78.0 for the index form, and 78.0 once a fold of a product sums out the points first (plan, gs13_'s entry).
⍝⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
gx gmmr∂ gp
⍝⍝: 150	410	610	instructions macos 207.964 linux 277.243	allocated macos 142.694 linux 184.194
