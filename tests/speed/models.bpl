⍝⍝ setup
mx←¯0.5+¿[1024 64]⍴0
my←1○+/mx
mp←["w":0.25×¯0.5+¿[64 256]⍴0 "b":256⍴0 "v":0.1×¯0.5+¿256⍴0 "c":0]
⍝ Mean squared error of a one-hidden-layer tanh network.
mlp←{
  [x y]←⍺
  [w b v c]←⍵
  h←7○b(+⍤1)x+.×w
  +/÷≢ ⊼y-c+h+.×v
}
gx←¿[1000 32]⍴0
gi←=⊗⍨⍳32
glow←>⊗⍨⍳32
gp←["alpha":¿25⍴0 "mu":¿[25 32]⍴0 "q":0.1×¿[25 32]⍴0 "l":0.1×¿[25 32 32]⍴0]
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
kx←¿[16384 32]⍴0
kc←¿[64 32]⍴0
⍝ Sum of squared distances to the nearest cluster centres.
kmeans←{+/⌊⌿(+/⊼⍵ +⊗ +/⊼⍺)-2×⍵+.×⍉⍺}
⍝ The same sum in index notation, which subtracts before it squares. `n` names the points, `c` the centres and `d` the dimensions.
kmeansi←{
  [[n d] c]←•index[⍴⍺ ≢⍵]
  +/⌊/⍠c+/⍠d⊼⍺.[n d]-⍵.[c d]
}
sg←¿[128 128]⍴0
sk←[1 2 1⋄2 4 2⋄1 2 1]÷16
n←65536
dx←¿1048576⍴0
dp←["w":¿[64 256]⍴0 "b":¿256⍴0 "v":¿256⍴0 "c":1]
rw←{0.1×¯0.5+¿⍵⍴0}
tid←¿[4 64]⍴512ₓ
ttg←¿[4 64]⍴512ₓ
tmk←¯1E9× ⍳64 <⊗ ⍳64
tp←["E":rw[512 128] "P":rw[64 128] "ga":128⍴1 "ba":128⍴0 "wqkv":rw[128 3 4 32] "wo":rw[128 128] "gb":128⍴1 "bb":128⍴0 "wu":rw[128 512] "cu":512⍴0 "wd":rw[512 128] "cd":128⍴0 "gc":128⍴1 "bc":128⍴0 "U":rw[128 512]]
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
  [q k v]←:[r b t h j]⍉⁻¹+/⍠d([ga ba]norm x)×wqkv.[d r h j]
  s←tmk.[t u] + +/⍠j⁝q.[b t h j]×k.[b u h j] ÷ √≢j
  e←*s-⌈/⍠u⁝s
  x←x++/⍠[h j](+/⍠u e÷+/⍠u⁝e × v.[b u h j])×wo.[j+(≢j)×h d]
  a←[gb bb]norm x
  x←x+cd.[d]++/⍠f(gelu cu.[f]++/⍠d⁝a×wu.[d f])×wd.[f d]
  lg←+/⍠d([gc bc]norm x)×U.[d c]
  m←⌈/⍠c⁝lg
  +/÷≢ , m+⍟+/⍠c*lg-m - lg.[b t tg.[b t]]
}
cx←¿[8 3 32 32]⍴0
ck←0.2×¯0.5+¿[16 3 3 3]⍴0
cy←¿[8 16 32 32]⍴0
⍝ A 3 by 3 convolution of 8 images of 3 channels, each 32 by 32, into 16 channels, in index notation. `•edge` gives NaN for a position past an edge, and a NaN position reads 0, so the images are padded with zeros.
conv←{
  [[n c i j] o a b]←•index[⍴⍺ ≢⍵ 3 3]
  [h w]←2↓⍴⍺
  [n o i j]⍉⁻¹+/⍠[c a b]⍺.[n c h•edge⁝i+a-1 w•edge⁝j+b-1]×⍵.[o c a b]
}
⍝ Mean squared error of one convolutional layer with tanh units, against the targets `cy`.
closs←{+/÷≢ ,⊼cy-7○⍺conv⍵}
hu←¿[512 512]⍴0
⍝ One step of heat diffusion: each cell moves towards the mean of its four neighbours. A neighbour past an edge is the edge cell itself (`clamp`).
heat←{
  [i j]←•index⍴⍵ ⋄ [h w]←⍴⍵ ⋄ clamp←•edge"clamp"
  : ⍵ + 0.2× ⍵.[(h clamp i-1) j]+⍵.[(h clamp i+1) j]+⍵.[i (w clamp j-1)]+⍵.[i (w clamp j+1)] - 4×⍵
}
⍝⍝ objectives
⍝ Models and stencils, timed without `∂`. For each function that `[derivatives]` differentiates, this is the cost that its gradient's cost is measured against.
⍝ The mean squared error of a network with one hidden layer of 256 tanh units, over 1024 inputs of 64 values. Its parameters are in a record.
⍝ Expected: 61 copies on macOS, 108 on Linux and 214 in wasm: a product of 33.5 million flops, 512 an item, which takes 17 at Accelerate's rank-k rate, 64 at 8 in a copy's time for rustymath's own kernels and 128 at 4 in wasm; `7○` on 262,144 items, 40 natively, at the 10 copies for each 65,536 items that rustymath's speed test expects of `Tanh`, and 80 in wasm, where vector arithmetic costs about twice; and the other passes, about 4 natively and 6 in wasm.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
[mx my] mlp mp
⍝⍝: 130	220	430

⍝ ADBench's Gaussian mixture objective, for 1000 points in 32 dimensions and 25 components, without its constant terms and with its Wishart prior at gamma 1 and m 0. It's written in index notation. Each component's precision factor has the exponentials of `q` on its diagonal and `l` below it.
⍝ Expected: 63 copies on macOS, 135 on Linux and 247 in wasm: 25 products of 1000×32 by 32×32, 781 flops an item in all, which take 26 at Accelerate's rank-k rate, 98 at 8 in a copy's time for rustymath's own kernels and 195 at 4 in wasm; and three passes over 800,000 items, about 12 copies each, 37 natively and 52 in wasm.
⍝ On 22 and 23 October 2026 CI measured up to 244 on Linux for the tacit form that this index form replaced, about twice its Linux expected cost. CI hasn't timed the index form.
⍝ On 27 October 2026, after `per_`'s period step, the index form measured 72.5 on macOS, against 72 for the tacit form earlier that day, and up to 99 during a video call. Its macOS budget follows its expected cost. Since the index form now costs what the tacit form costs, the Linux and wasm budgets are twice the tacit form's cost there: its CI figure of 244 on Linux, and its expected 247 in wasm, where no CI figure is recorded.
gx gmm gp
⍝⍝: 95	490	500

⍝ The k-means cost sums the squared distance from each of 16384 points to the nearest of 64 centres, in 32 dimensions.
⍝ Expected: 84 copies on macOS, 178 on Linux and 326 in wasm: a product of 64×32 by 32×16384, 67 million flops, 1,024 an item, which takes 34 at Accelerate's rank-k rate, 128 at 8 in a copy's time for rustymath's own kernels and 256 at 4 in wasm; and passes over a million distances, about 50 natively and 70 in wasm.
⍝ Held until studied: on 22 and 23 October 2026 CI measured up to 339 on Linux, about twice its Linux expected cost, where a budget of twice it would fail by chance. Its Linux budget stays at the old value.
kx kmeans kc
⍝⍝: 170	600	660

⍝ The same cost in index notation. Both forms are kept because they trade speed for accuracy. The tacit form expands each squared distance as |x|²+|c|²−2x·c, which runs as one matrix product and stays faster, but loses accuracy where a point is near a centre, because the terms nearly cancel. The index form subtracts first, so its error stays proportional to the distance.
⍝ Expected: 208 copies natively and 410 in wasm: 33.5 million differences, each squared and summed, 1,536 flops an item, at 8 in a copy's time for rustymath's own kernels and 4 in wasm; and the million distances, written and then read by the `⌊` fold, about 16 natively and 24 in wasm.
⍝ On 27 October 2026 it measured 1,089 on macOS, while its fold copied `x.[n d]` across all 33,554,432 items of the grid ("repeats of each item"). Since `fax_` let that fold read the repeats in place, it measures 832 to 853. The fold over the grid still runs at about 0.2 ns an item, where a fold that kept its sums in registers would come near the expected cost (`ixg_`). That fold needs a design and is under "Next release" in `meta/open.md`. Until then, the macOS budget is 1.3 times today's cost, 832, and the Linux and wasm budgets stay at twice estimates of the earlier cost, 1,089 on Linux, because the fold runs no product, and twice that in wasm.
kx kmeansi kc
⍝⍝: 1100	2200	4400

⍝ A pre-norm transformer decoder block over 4 sequences of 64 tokens: width 128, 4 heads of width 32, an MLP of width 512 with GELU in its tanh form, and a cross-entropy loss over 512 tokens. Its parameters are in a record. `dec` gives the mean loss, and is written in index notation.
⍝ Expected: 155 copies on macOS, 354 on Linux and 684 in wasm: products of 142.6 million flops, 2,176 an item, which take 72.5 at Accelerate's rank-k rate, 272 at 8 in a copy's time for rustymath's own kernels and 544 at 4 in wasm; and the elementwise passes, about 82 natively and 140 in wasm. Those are GELU's `7○` and its polynomial on 131,072 items, about 30; the cross-entropy's exponentials on 131,072 items, about 18; the attention's scaling, mask and softmax on 65,536 items, about 13; three layer norms, about 11; and the bias adds and residuals, about 10.
⍝ On 27 October 2026 it measured 173 to 202 on macOS, the higher figure during a video call. `ixg_` placed about half the gap: agreement by name puts the left operand's axes first, so lines such as `g.[d]×z` and `cu.[f]+…` lay their results out transposed, and later reads copy them back. Option (a) for that order, under "Next release" in `meta/open.md`, removes those copies. Until then, the macOS budget is 1.5 times today's cost, 173. The Linux and wasm budgets stay at twice their estimates.
[tid ttg] dec tp
⍝⍝: 260	710	1400

⍝ A 3 by 3 blur of a 128 by 128 grid. `⌺` calls its operand once, on a batch of all the windows.
⍝ Expected: 1.5 copies: nine multiply-adds for each of 16,384 windows, read from the cache.
⍝ Held until studied: `{+/,sk×⍵}⌺3 3 sg` measures 10.25 on macOS and has old Linux and wasm budgets that suggest about 19 and 19.5, more than twice its expected cost. On 22 October 2026 CI measured 10.9 on Linux and 10.3 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{+/,sk×⍵}⌺3 3 sg
⍝⍝: 21	38	39

⍝ One step of heat diffusion on a 512 by 512 grid, 4 times `n`, reading each cell's four neighbours through `•edge`'s clamped positions.
⍝ Expected: 9 copies: one fused pass of seven calls over 262,144 items, which reads each neighbour from forward runs, as `hu+0.2×(hu+hu+hu+hu)-4×hu` measures it. Wasm, twice that.
⍝ On 27 October 2026 it measured 26 to 31 on macOS. Its two column-shifted reads through `•edge`'s clamped positions copy their whole input, because a run of positions doesn't end at a row's edge, and ending runs there costs more than the copy saves (`hcr_`). A fix needs a design and is under "Next release" in `meta/open.md`. Until then, the macOS budget is twice today's cost, 26, and the Linux and wasm budgets stay at twice estimates of 31 on Linux and 62 in wasm.
heat hu
⍝⍝: 53	62	124

⍝ The loss of one convolutional layer with tanh units: 8 images of 3 channels, each 32 by 32, into 16 channels through a 3 by 3 kernel, against the targets `cy`.
⍝ Expected: 29 copies on macOS, 39 on Linux and 78 in wasm: the windows' copy, 221,184 items, about 3.4; a product of 3.5 million multiply-adds, 108 flops an item, which takes 3.6 at Accelerate's rank-k rate, 13.5 at 8 in a copy's time for rustymath's own kernels and 27 at 4 in wasm; `7○` on 131,072 items, 20 natively and 40 in wasm; and the loss's pass, about 2.
⍝ On 27 October 2026 it measured 111 to 125 on macOS, because a fold computed its argument, a copy of 1,572,864 items, instead of reading through its group. Since the fold along `c`, `a` and `b` runs as one product (`eval::folded_along`), it measures 56 to 75, the higher figures during a video call. The gap from the expected 29 isn't placed. In one timing during the call, the convolution took about half of the case's time, against its expected 7 copies of 29, and the same convolution over the interior windows alone, without `•edge`, took about two thirds of that for 88% as many windows. `•explain` declines to read 352,256 items in place: the windows and the result's transpose. Until the gap is placed and fixed, the macOS budget is 1.5 times today's cost, 56. The Linux and wasm budgets stay at twice estimates of 69 on Linux, where the product takes 10 more, and 138 in wasm, twice Linux.
cx closs ck
⍝⍝: 85	140	280

⍝⍝ derivatives
⍝ `∂` beside the function it differentiates. A unit argument takes one pass, whose tangent is an array of the result's shape. The bar's "Primal cost" row in the design notes asks that the primal keeps its kernels, and the tangent adds whole-array calls beside it.
⍝ Expected: `1○dx` 112 copies: sines of a million items, 16 times `n`, at 7 copies each, as `1○v` counts them; `{1○dx+⍵}∂0` 116 copies: cosines of a million items at 7 copies each, with the sum fused into their pass. `∂` runs no sines, because its result is the tangent alone.
⍝ On CI's wasm job, `1○dx` costs 250 to 323 copies, about 2.3 times its native cost. Its wasm budget stays at 430.
1○dx
⍝⍝: 224	224	450

{1○dx+⍵}∂0
⍝⍝: 232	232	470

⍝ A chain of fusible calls, whose tangent fuses too.
⍝ Expected: `(dx-0.5)×dx+0.5` 24 copies: one fused pass over a million items, 16 times `n`; `{(dx-⍵)×dx+⍵}∂0.5` 24 copies: the tangent's fused pass. `∂` skips the primal, because the result holds only the tangent.
⍝ Held until studied: `(dx-0.5)×dx+0.5` has old Linux and wasm budgets that suggest about 85 and 70, more than twice its expected cost. On 22 October 2026 CI measured 96.7 on Linux and 66.9 in wasm. Its Linux and wasm budgets stay at the old value.
dx-0.5 × dx+0.5
⍝⍝: 48	170	140

⍝ Held until studied: `{(dx-⍵)×dx+⍵}∂0.5` measures 48 copies on macOS, twice its expected cost, and has old Linux and wasm budgets that suggest about 110 and 110. On 22 and 23 October 2026 CI measured up to 104 on Linux and 76.6 in wasm. All three budgets stay at their old values.
{dx-⍵ × dx+⍵}∂0.5
⍝⍝: 96	220	220

⍝ A JVP of the MLP's loss in `[objectives]` along a fixed direction `dp`, which takes one pass through `+.×`, Rank, `7○` and `+/`.
⍝ Expected: 122 copies on macOS, 216 on Linux and 428 in wasm: the primal and one tangent pass of the same size.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝ allows itemwise "item pairs": `mp+⍵×dp` pairs the parameter record's few arrays one at a time, each by a whole-array call
{[mx my] mlp mp+⍵×dp}∂0
⍝⍝: 250	440	860

⍝ Gradients of the MLP, Gaussian mixture and k-means objectives in `[objectives]`, each from one run that records its linear calls and one transpose of the record. The bar's "Gradient cost" row asks for a small multiple of the objective's own cost.
⍝ Expected: `[mx my] mlp∂ mp` 120 copies on macOS, 261 on Linux and 520 in wasm: the primal, a transpose with two products, which take 17 each at Accelerate's rank-k rate, 64 on rustymath's own kernels and 128 in wasm, and `7○`'s derivative with the rest, about 25 natively and 50 in wasm; `gx gmm∂ gp` 189 copies on macOS, 405 on Linux and 741 in wasm: about three times the objective; `kx kmeans∂ kc` 252 copies on macOS, 534 on Linux and 978 in wasm: about three times the objective.
[mx my] mlp∂ mp
⍝⍝: 240	530	1100

⍝ The tacit form that this index form replaced measured up to 932 on Linux on CI on 22 and 23 October 2026, more than twice its expected cost. CI hasn't timed the index form. Its Linux and wasm budgets are twice estimates of today's cost there: the tacit form's cost, plus up to 140 for the index form's eager gather, which costs 110 to 140 more than the tacit form on macOS. On Linux that's the tacit form's CI figure of 932 plus 140, about 1,070. In wasm, where no CI figure is recorded, it's the expected 741 plus 140, about 880.
⍝ On 27 October 2026 the index form measured 345 to 375 on macOS during a video call, against 239 for the tacit form earlier that day. Under dynamic kinds, `⍟` stops both forms' traces, so `∂` runs both without a program. The index form's eager gradient gathers repeated items for its `sq` line, 825,600 items at about 1.7 ns each. Traced, under static kinds, the index form's gradient is the faster (`ixg_`). A fix for the eager gather needs a design and is under "Next release" in `meta/open.md`. Until then, the macOS budget is 1.5 times today's cost, 345.
⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
gx gmm∂ gp
⍝⍝: 520	2200	1800

⍝ Held until studied: `kx kmeans∂ kc` measured up to 1267 on Linux on CI on 22 and 23 October 2026, more than twice its expected cost. Its Linux budget stays at the old value.
kx kmeans∂ kc
⍝⍝: 510	1800	2000

⍝ The gradient of the index form of the k-means cost in `[objectives]`.
⍝ Expected: 624 copies natively and 1,230 in wasm: about three times the objective.
⍝ On 27 October 2026 it measured 2,113 to 2,312 on macOS, and 2,116 later that day. Its gradient stores the 268 MB difference grid. Rewriting the contraction over the subtraction would avoid that, but changes rounding, and Jeremy put that choice after the release, under "Next release" in `meta/open.md` (`ixg_`'s mechanism 4). Until then, the macOS budget is 1.3 times today's cost, 2,116. The Linux and wasm budgets stay at twice estimates of the earlier cost, 2,312 on Linux and twice that in wasm.
⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
kx kmeansi∂ kc
⍝⍝: 2800	4700	9400

⍝ The gradient of the decoder block in `[objectives]`, from one run that records its linear calls and one transpose of the record.
⍝ Expected: 465 copies on macOS, 1060 on Linux and 2050 in wasm: about three times the objective.
⍝ On 27 October 2026 the index form measured 810 to 864 on macOS, with other work running, against 432 for the tacit form it replaced. Under dynamic kinds, `√` and `⍟` stop both forms' traces, so `∂` runs both without a program, and the index form's run is the slower. Since a cotangent's sum along adjacent axes runs as one fold, which a product takes (`eval::folded_along`), it measures 630 to 649. Part of the rest is the copies from agreement by name's axis order that the objective's comment describes. Until option (a) for that order removes them, the macOS budget is 1.3 times today's cost, 630. CI hasn't timed the index form, and no CI figure is recorded for the tacit form. The Linux and wasm budgets are twice estimates of today's cost there: the expected 1,060 and 2,050, plus the 198 by which the index form exceeds the tacit form on macOS, about 1,260 and 2,250.
[tid ttg] dec∂ tp
⍝⍝: 820	2600	4500

⍝ The gradient of the convolutional layer's loss in `[objectives]`, with respect to its kernel.
⍝ Expected: 87 copies on macOS, 117 on Linux and 234 in wasm: about three times the objective.
⍝ On 27 October 2026 it measured 112 to 122 on macOS. It runs the objective, whose gap isn't placed, as the objective's comment says. Until that gap is placed and fixed, the macOS budget is 1.5 times today's cost, 112. The Linux and wasm budgets stay at twice their estimates.
⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
cx closs∂ ck
⍝⍝: 170	235	470

⍝⍝ static •prefs ["kinds":"static" "int":32 "float":32]
⍝ Expected at 32 bits: the decoder block 78 copies on macOS, 177 on Linux and 342 in wasm, and its gradient 233, 530 and 1025, half their 64-bit costs. On 27 October 2026 the index form measured 104 and 367 on macOS, against 68 and 190 for the tacit form it replaced. Since a cotangent's sum along adjacent axes runs as one fold, which a product takes (`eval::folded_along`), the gradient measures 187 to 188, and the decoder block measures 102 to 105. The decoder block has the 64-bit form's gap from agreement by name, so its macOS budget is 1.5 times today's cost, 102. The gradient's macOS budget follows its expected cost.
[tid ttg] dec tp
⍝⍝: 160	360	690

[tid ttg] dec∂ tp
⍝⍝: 350	1100	2100

⍝ Expected at 32 bits: the Gaussian mixture 32 copies on macOS, 68 on Linux and 124 in wasm, and its gradient 95, 203 and 371, half their 64-bit costs. Its gradient runs from a program here, where `⍟` stops the 64-bit section's trace under dynamic kinds. On 27 October 2026 they measured 39 and 80 on macOS, and their macOS budgets follow their expected costs.
gx gmm gp
⍝⍝: 64	140	250

⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
gx gmm∂ gp
⍝⍝: 150	410	750
