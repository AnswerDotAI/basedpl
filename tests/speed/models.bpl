⍝⍝ setup
mx←¯0.5+¿[1024 64]⍴0
my←1○+/mx
mp←["w":0.25×¯0.5+¿[64 256]⍴0 "b":256⍴0 "v":0.1×¯0.5+¿256⍴0 "c":0]
⍝ Mean squared error of a one-hidden-layer tanh network.
mlp←{
  [x y]←⍺
  [w b v c]←⍵
  h←7○b+⍤1 x+.×w
  (+/÷≢)⊼y-c+h+.×v
}
gx←¿[1000 32]⍴0
gi←=⊗⍨⍳32
glow←>⊗⍨⍳32
gp←["alpha":¿25⍴0 "mu":¿[25 32]⍴0 "q":0.1×¿[25 32]⍴0 "l":0.1×¿[25 32 32]⍴0]
⍝ Gaussian-mixture objective with a Wishart prior.
gmm←{
  [alpha mu q l]←⍵
  L←l×⍤2 glow
  Q←L+(*q)×⍤[1 2]gi
  sq←+/⊼(⍺(-⍤1)⍤[2 1]mu)+.×⍤2[0 2 1]⍉Q
  t←(alpha++/q)-0.5×sq
  m←⌈⌿t
  am←⌈/alpha
  (+/m+⍟+⌿*t-⍤1m) + (0.5×(+/,⊼*q)++/,⊼L) - (≢⍺)×am+⍟+/*alpha-am
}
kx←¿[16384 32]⍴0
kc←¿[64 32]⍴0
⍝ Sum of squared distances to the nearest cluster centres.
kmeans←{+/⌊⌿((+/⊼⍵)+⊗+/⊼⍺)-2×⍵+.×⍉⍺}
sg←¿[128 128]⍴0
sk←[1 2 1⋄2 4 2⋄1 2 1]÷16
n←65536
dx←¿1048576⍴0
dp←["w":¿[64 256]⍴0 "b":¿256⍴0 "v":¿256⍴0 "c":1]
rw←{0.1×¯0.5+¿⍵⍴0}
nh←4ₓ
tid←¿[4 64]⍴512ₓ
ttg←¿[4 64]⍴512ₓ
tmk←¯1E9×(⍳64)<⊗⍳64
tp←["E":rw[512 128] "P":rw[64 128] "ga":128⍴1 "ba":128⍴0 "wq":rw[128 128] "wk":rw[128 128] "wv":rw[128 128] "wo":rw[128 128] "gb":128⍴1 "bb":128⍴0 "wu":rw[128 512] "cu":512⍴0 "wd":rw[512 128] "cd":128⍴0 "gc":128⍴1 "bc":128⍴0 "U":rw[128 512]]
⍝ Layer normalization over the last axis, with learned gain and bias.
ln←{
  [g b]←⍺
  c←⍵-(+/÷≢)⍤1⍵
  b+⍤1 g×⍤1 c÷√1E¯5+(+/÷≢)⍤1⊼c
}
⍝ Split each token's features into attention heads.
hs←{0 2 1 3⍉((¯1↓⍴⍵),nh,(¯1↑⍴⍵)÷ₓnh)⍴⍵}
⍝ Merge attention heads back into each token's feature vector.
hm←{
  h←0 2 1 3⍉⍵
  ((¯2↓⍴h),×/¯2↑⍴h)⍴h
}
gelu_scale←√2÷π1
⍝ GELU activation using its tanh approximation.
gelu←{0.5×⍵×1+7○gelu_scale×⍵+0.044715×⍵*3}
⍝ Mean token cross-entropy of a pre-norm transformer decoder block.
dec←{
  [ids tg]←⍺
  [E P ga ba wq wk wv wo gb bb wu cu wd cd gc bc U]←⍵
  x←E.[ids]+⍤2 P
  a←[ga ba]ln x
  q←hs a+.×wq
  k←hs a+.×wk
  v←hs a+.×wv
  s←tmk+⍤2 (q+.×⍤2[0 1 3 2]⍉k)÷√32
  e←*s-⌈/s
  x←x+(hm (e÷+/e)+.×⍤2 v)+.×wo
  a←[gb bb]ln x
  x←x+cd+⍤1 (gelu cu+⍤1 a+.×wu)+.×wd
  lg←([gc bc]ln x)+.×U
  m←⌈/lg
  +/÷≢ ,(m+⍟+/*lg-m)-tg⌷⍤[0 1]lg
}
⍝⍝ objectives
⍝ Functions that `∂` will differentiate, timed without `∂`. Each gives the cost that its gradient's cost will be measured against.
⍝ The mean squared error of a network with one hidden layer of 256 tanh units, over 1024 inputs of 64 values. Its parameters are in a record.
⍝ Expected: 61 copies on macOS, 108 on Linux and 214 in wasm: a product of 33.5 million flops, 512 an item, which takes 17 at Accelerate's rank-k rate, 64 at 8 in a copy's time for rustymath's own kernels and 128 at 4 in wasm; `7○` on 262,144 items, 40 natively, at the 10 copies for each 65,536 items that rustymath's speed test expects of `Tanh`, and 80 in wasm, where vector arithmetic costs about twice; and the other passes, about 4 natively and 6 in wasm.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
[mx my] mlp mp
⍝⍝: 130	220	430

⍝ ADBench's Gaussian mixture objective, for 1000 points in 32 dimensions and 25 components, without its constant terms and with its Wishart prior at gamma 1 and m 0. Each component's precision factor has the exponentials of `q` on its diagonal and `l` below it.
⍝ Expected: 63 copies on macOS, 135 on Linux and 247 in wasm: 25 products of 1000×32 by 32×32, 781 flops an item in all, which take 26 at Accelerate's rank-k rate, 98 at 8 in a copy's time for rustymath's own kernels and 195 at 4 in wasm; and three passes over 800,000 items, about 12 copies each, 37 natively and 52 in wasm.
⍝ Held until studied: on 22 and 23 October 2026 CI measured up to 244 on Linux, about twice its Linux expected cost, where a budget of twice it would fail by chance. Its Linux budget stays at the old value.
⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
gx gmm gp
⍝⍝: 130	480	500

⍝ The k-means cost sums the squared distance from each of 16384 points to the nearest of 64 centres, in 32 dimensions.
⍝ Expected: 84 copies on macOS, 178 on Linux and 326 in wasm: a product of 64×32 by 32×16384, 67 million flops, 1,024 an item, which takes 34 at Accelerate's rank-k rate, 128 at 8 in a copy's time for rustymath's own kernels and 256 at 4 in wasm; and passes over a million distances, about 50 natively and 70 in wasm.
⍝ Held until studied: on 22 and 23 October 2026 CI measured up to 339 on Linux, about twice its Linux expected cost, where a budget of twice it would fail by chance. Its Linux budget stays at the old value.
kx kmeans kc
⍝⍝: 170	600	660

⍝ A pre-norm transformer decoder block over 4 sequences of 64 tokens: width 128, 4 heads of width 32, an MLP of width 512 with GELU in its tanh form, and a cross-entropy loss over 512 tokens. Its parameters are in a record. `dec` gives the mean loss.
⍝ Expected: 155 copies on macOS, 354 on Linux and 684 in wasm: products of 142.6 million flops, 2,176 an item, which take 72.5 at Accelerate's rank-k rate, 272 at 8 in a copy's time for rustymath's own kernels and 544 at 4 in wasm; and the elementwise passes, about 82 natively and 140 in wasm. Those are GELU's `7○` and its polynomial on 131,072 items, about 30; the cross-entropy's exponentials on 131,072 items, about 18; the attention's scaling, mask and softmax on 65,536 items, about 13; three layer norms, about 11; and the head reshapes, bias adds and residuals, about 10.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
[tid ttg] dec tp
⍝⍝: 310	710	1400

⍝ A 3 by 3 blur of a 128 by 128 grid. `⌺` calls its operand once, on a batch of all the windows.
⍝ Expected: 1.5 copies: nine multiply-adds for each of 16,384 windows, read from the cache.
⍝ Held until studied: `{+/,sk×⍵}⌺3 3 sg` measures 10.25 on macOS and has old Linux and wasm budgets that suggest about 19 and 19.5, more than twice its expected cost. On 22 October 2026 CI measured 10.9 on Linux and 10.3 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{+/,sk×⍵}⌺3 3 sg
⍝⍝: 21	38	39

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
(dx-0.5)×dx+0.5
⍝⍝: 48	170	140

⍝ Held until studied: `{(dx-⍵)×dx+⍵}∂0.5` measures 48 copies on macOS, twice its expected cost, and has old Linux and wasm budgets that suggest about 110 and 110. On 22 and 23 October 2026 CI measured up to 104 on Linux and 76.6 in wasm. All three budgets stay at their old values.
{(dx-⍵)×dx+⍵}∂0.5
⍝⍝: 96	220	220

⍝ A JVP of the training loss in `[objectives]` along a fixed direction `dp`, which takes one pass through `+.×`, Rank, `7○` and `+/`.
⍝ Expected: 122 copies on macOS, 216 on Linux and 428 in wasm: the primal and one tangent pass of the same size.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝ allows itemwise "item pairs": `mp+⍵×dp` pairs the parameter record's few arrays one at a time, each by a whole-array call
{[mx my] mlp mp+⍵×dp}∂0
⍝⍝: 250	440	860

⍝ Gradients of the three objectives in `[objectives]`, each from one run that records its linear calls and one transpose of the record. The bar's "Gradient cost" row asks for a small multiple of the objective's own cost.
⍝ Expected: `[mx my] mlp∂ mp` 120 copies on macOS, 261 on Linux and 520 in wasm: the primal, a transpose with two products, which take 17 each at Accelerate's rank-k rate, 64 on rustymath's own kernels and 128 in wasm, and `7○`'s derivative with the rest, about 25 natively and 50 in wasm; `gx gmm∂ gp` 189 copies on macOS, 405 on Linux and 741 in wasm: about three times the objective; `kx kmeans∂ kc` 252 copies on macOS, 534 on Linux and 978 in wasm: about three times the objective.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝ gap stored "held outside the graph": the transpose computes `7○`'s derivative, 262,144 items, on its own, where the group that reads it should compute it
⍝⍝ gap forced "+/": the gradient of `b` should fold its pending argument along axis 0, rather than compute 524,288 items first
[mx my] mlp∂ mp
⍝⍝: 240	530	1100

⍝ Held until studied: `gx gmm∂ gp` measured up to 932 on Linux on CI on 22 and 23 October 2026, more than twice its expected cost. Its Linux budget stays at the old value.
⍝⍝ allows forced "+.×⍤2": a matrix product reads its arguments whole
gx gmm∂ gp
⍝⍝: 380	5500	1500

⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝ Held until studied: `kx kmeans∂ kc` measured up to 1267 on Linux on CI on 22 and 23 October 2026, more than twice its expected cost. Its Linux budget stays at the old value.
⍝⍝ gap forced "⌊⌿": the gradient's `⌊⌿` should fold its pending distances, as `kx kmeans kc` does, rather than compute 2,097,152 items first
kx kmeans∂ kc
⍝⍝: 510	1800	2000

⍝ The gradient of the decoder block in `[objectives]`, from one run that records its linear calls and one transpose of the record.
⍝ Expected: 465 copies on macOS, 1060 on Linux and 2050 in wasm: about three times the objective.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝ gap forced "+/": the gradients of the biases and layer-norm gains should fold their pending arguments along axis 0, rather than compute up to 262,144 items first
⍝⍝ gap stored "held outside the graph": the transpose computes GELU's intermediate values, 131,072 items each, on their own, where the group that reads them should compute them
[tid ttg] dec∂ tp
⍝⍝: 930	2100	4100

⍝⍝ static •prefs ["kinds":"static" "int":32 "float":32]
⍝ Expected at 32 bits: the decoder block 78 copies on macOS, 177 on Linux and 342 in wasm, and its gradient 233, 530 and 1025, half their 64-bit costs. On 24 October 2026 they measured 69 and 198 on macOS.
⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
[tid ttg] dec tp
⍝⍝: 160	360	690

⍝⍝ allows forced "+.×": a matrix product reads its arguments whole
⍝⍝ gap forced "+/": the gradients of the biases and layer-norm gains should fold their pending arguments along axis 0, rather than compute up to 262,144 items first
⍝⍝ gap stored "held outside the graph": the transpose computes GELU's intermediate values, 131,072 items each, on their own, where the group that reads them should compute them
[tid ttg] dec∂ tp
⍝⍝: 470	1100	2100
