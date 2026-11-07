⍝⍝ setup
⍝ Four models written for static kinds from the documentation alone, to check the evaluator's rewrite. Each model has a forward pass, a scalar loss, its gradient through ∂, a training step and a loop of steps, in two spellings that compute the same thing. `32•float` makes every data array and parameter record 32-bit under either kinds. Under dynamic kinds, `√e` of the exact `e` and `m÷keep` of the Boolean mask are 64-bit, so attention and the dropped inputs compute at 64 bits there. Costs count copies of n 64-bit floats, about 7.9 µs on the development Mac.
n←65536
⍝ Uniform parameters scaled by the square root of the fan-in, the first axis.
init←{(¯0.5+¿⍵⍴0)÷√⍵₀}
⍝ The generator that draws the dropout masks.
gen←•rand 7
⍝ Transformer block: B sequences of T tokens with d features, causal attention with H heads of e features and an MLP with f hidden units, each after a layer norm and inside a residual connection.
[B T d H f]←2 64 64 4 128ₓ
e←d÷ₓH
tx←32•float ¯0.5+¿[B T d]⍴0
ty←32•float ¯0.5+¿[B T d]⍴0
tp←32•float["wq":init[d d] "wk":init[d d] "wv":init[d d] "wo":init[d d] "ga":d⍴1 "ba":d⍴0 "wu":init[d f] "bu":f⍴0 "wd":init[f d] "bd":d⍴0 "gm":d⍴1 "bm":d⍴0]
causal←32•float ¯1E9× ⍳T <⊗ ⍳T
mean←+/÷≢
norm←{c←⍵-mean⍤1⁝⍵ ⋄ c÷√1E¯5+mean⍤1⁝⊼c}
ln←{[g b]←⍺ ⋄ b(+⍤1)g(×⍤1)norm ⍵}
heads←[0 2 [1 H;3 ∞]]⍉
attend←{
  [wq wk wv wo]←⍺
  x←⍵
  [q k v]←{heads x+.×⍵}¨[wq wk wv]
  s←causal(+⍤2) q+.×⍤2 ⍉⍤2⁝k ÷ √e
  w←*s-⌈/s
  (heads⁻ w÷+/w +.×⍤2 v)+.×wo
}
mlp←{[wu bu wd bd]←⍺ ⋄ bd(+⍤1)(0⌈bu(+⍤1)⍵+.×wu)+.×wd}
block←{
  [wq wk wv wo ga ba wu bu wd bd gm bm]←⍺
  h←⍵+[wq wk wv wo]attend⁝[ga ba]ln⁝⍵
  h+[wu bu wd bd]mlp⁝[gm bm]ln⁝h
}
tloss←{[x y]←⍺ ⋄ o←⍵ block x ⋄ +/÷≢ ⊼,y-o}
tstep←{⍵ - 0.2× [tx ty]tloss∂⍵}
⍝ The same block in index notation: b sequence, t token, i feature, u the token that t reads, h head, j feature within a head, k hidden unit.
blocki←{
  [wq wk wv wo ga ba wu bu wd bd gm bm]←⍺
  [[b t i] u [h j] k]←•index[⍴⍵ T [H e] f]
  lnorm←{[g c]←⍺ ⋄ z←⍵-(+/⍠i⁝⍵)÷d ⋄ c.[i]+g.[i]×z÷√1E¯5+(+/⍠i⁝⊼z)÷d}
  x←⍵.[b t i]
  z←[ga ba]lnorm x
  zu←(:[b t i]⍉⁻z).[b u i]
  q←z+.×⍠i⁝wq.[i j+e×h]
  kk←zu+.×⍠i⁝wk.[i j+e×h]
  vv←zu+.×⍠i⁝wv.[i j+e×h]
  s←causal.[t u]+q+.×⍠j⁝kk ÷ √e
  w←*s-⌈/⍠u⁝s
  o←(w÷+/⍠u⁝w)+.×⍠u⁝vv
  r←x++/⍠[h j]⁝o×wo.[j+e×h i]
  m←0⌈bu.[k]+([gm bm]lnorm r)+.×⍠i⁝wu.[i k]
  :[b t i]⍉⁻r+bd.[i]+m+.×⍠k⁝wd.[k i]
}
tlossi←{[x y]←⍺ ⋄ o←⍵ blocki x ⋄ +/÷≢ ⊼,y-o}
tstepi←{⍵ - 0.2× [tx ty]tlossi∂⍵}
⍝ Convolutional net: N images of S by S pixels, F filters of 3 by 3, a leaky ReLU, 2 by 2 max pooling and a dense layer to K classes, with a cross-entropy loss. `image` handles one image; Rank maps it over the batch.
[N S K F]←32 16 10 8ₓ
cx←32•float ¯0.5+¿[N S S]⍴0
cy←(¿N⍴K) =⊗ ⍳K
cp←32•float["k":init[9 F] "c":F⍴0 "w":init[49×F ; K] "b":K⍴0]
lsm←{c←⍵-⌈/⍵ ⋄ c-⍟+/*c}
image←{
  [k c w b]←⍺
  a←c(+⍤1)([14 14 9]⍴3 3↕⍵)+.×k
  r←(0<)⍚[0.01× ⊢]a
  p←⌈/₂ ₃[[2 2]⋄[2 2]]↕r
  b+(,p)+.×w
}
closs←{[x y]←⍺ ⋄ z←⍵(image⍤∞ 2)x ⋄ -+/÷≢ +/y×lsm z}
cstep←{⍵ - 0.1× [cx cy]closs∂⍵}
⍝ The same net in index notation over the whole batch: n image, i j output pixel, s t position in the filter, f filter, p q pooled pixel, u v position in the pool, o class.
convi←{
  [k c w b]←⍺
  [[n i j] [s t] f [p q] [u v] o]←•index[[≢⍵ 14 14] [3 3] F [7 7] [2 2] K]
  a←c.[f]++/⍠[s t]⁝⍵.[n i+s j+t]×k.[t+3×s f]
  r←(0<)⍚[0.01× ⊢]a
  m←⌈/⍠[u v]⁝(:[n i j f]⍉⁻r).[n u+2×p v+2×q f]
  :[n o]⍉⁻b.[o]++/⍠[p q f]⁝m×w.[f+F×q+7×p o]
}
clossi←{[x y]←⍺ ⋄ z←⍵ convi x ⋄ -+/÷≢ +/y×lsm z}
cstepi←{⍵ - 0.1× [cx cy]clossi∂⍵}
⍝ ODE: P damped pendulums, θ'' = -k sin θ - c θ', over M classical Runge-Kutta steps of dt through `⍣`. The loss fits k and c to observed angles. `rkstep` takes the field as its operand; `swing` is the field of every pendulum at once, on a 2 by P state.
[P M]←128 50ₓ
dt←0.04
rkstep←{
  ka←⍺ ⍶ ⍵
  kb←⍺ ⍶ ⍵+ka×dt÷2
  kc←⍺ ⍶ ⍵+kb×dt÷2
  kd←⍺ ⍶ ⍵+kc×dt
  ⍵+(dt÷6)×ka+kd+2×kb+kc
}
swing←{[th om]←⍵ ⋄ [om⋄(-⍺.k×1○th)-⍺.c×om]}
sa←32•float[¯1+2×¿P⍴0⋄P⍴0]
truth←32•float["k":4 "c":0.3]
ot←(⊃truth(swing rkstep⍣(⍳≥M))sa).[: 0]
op←32•float["k":3 "c":0.1]
oloss←{[s y]←⍺ ⋄ t←⊃⍵(swing rkstep⍣(⍳≥M))s ⋄ +/÷≢ ⊼,y-t.[: 0]}
ostep←{⍵ - 10× [sa ot]oloss∂⍵}
⍝ `pendulum` is the field of one pendulum; Rank maps it over a P by 2 state.
pendulum←{[th om]←⍵ ⋄ [om ; (-⍺.k×1○th)-⍺.c×om]}
sb←⍉sa
olossp←{[s y]←⍺ ⋄ t←⊃⍵((pendulum⍤∞ 1)rkstep⍣(⍳≥M))s ⋄ +/÷≢ ⊼,y-t.[: : 0]}
ostepp←{⍵ - 10× [sb ot]olossp∂⍵}
⍝ Logistic regression on R samples of D features, R×D = n, with inverted dropout of the inputs, its mask drawn from `gen` at each step. `xent` is the cross-entropy through the logits, with the mask applied by multiplication. `xentp` works from probabilities, with Agenda picking each sample's log-likelihood by its label, and its step applies the mask by Agenda.
[R D]←1024 64ₓ
keep←0.8
lx←32•float ¯0.5+¿[R D]⍴0
ly←0<lx+.×¯0.5+¿D⍴0
lp←32•float["w":D⍴0 "b":0]
xent←{[x y]←⍺ ⋄ [w b]←⍵ ⋄ z←b+x+.×w ⋄ +/÷≢ (⍟1+*z)-y×z}
lstep←{m←keep>gen.roll(⍴lx)⍴0 ⋄ ⍵ - 0.5× [lx×m÷keep ; ly]xent∂⍵}
xentp←{[x y]←⍺ ⋄ p←÷1+*-⍵.b+x+.×⍵.w ⋄ -+/÷≢ y⍚[⍟1-⊢ ⍟]p}
lstepp←{m←keep>gen.roll(⍴lx)⍴0 ⋄ ⍵ - 0.5× [m⍚[0 ÷↢keep]lx ; ly]xentp∂⍵}

⍝⍝ transformer •prefs ["kinds":"static"]
⍝ The block's forward pass and mean squared error, in array form.
⍝ Expected: 10 copies. The products cost about 4.2: the three projections and the output projection of 128 rows, about 0.3 each, the scores and the weighted values, stacks of eight products of 64-row matrices, about 0.9 each, and the MLP's two products, about 0.6 each. The softmax's exponential of 32,768 scores costs about 2.1, at 4.2 copies for n items. About 25 other passes over 8,192 to 32,768 items cost about 3, at half a copy for n 32-bit items, since no array reaches the 65,536 items at which calls fuse. About 60 primitive calls and 8 dfn calls cost about 1. The products and the exponential dominate.
[tx ty]tloss tp
⍝⍝: 20	20	20

⍝ The same loss in index notation.
⍝ Expected: 10 copies, as in array form. Each contraction runs as a matrix product and each read through computed positions as a map, as the page on models in index notation says.
[tx ty]tlossi tp
⍝⍝: 20	20	20

⍝ The gradient with respect to the 12 parameter arrays, in array form.
⍝ Expected: 25 copies: the forward pass, about 10; the transpose's two products for each forward product, about 8.4; and about two passes for each forward pass other than the exponential, about 6. ∂ replays a program, so its calls add about 0.5.
[tx ty]tloss∂ tp
⍝⍝: 50	50	50

⍝ The same gradient in index notation.
⍝ Expected: 25 copies, as in array form.
[tx ty]tlossi∂ tp
⍝⍝: 50	50	50

⍝ Five steps of gradient descent, in array form.
⍝ Expected: 130 copies: five gradients of about 25, and five updates of the 33,216 parameters, two passes and 24 calls each, about 0.8.
tstep⍣5 tp
⍝⍝: 200	200	200

⍝ Five steps of gradient descent, in index notation.
⍝ Expected: 130 copies, as in array form.
tstepi⍣5 tp
⍝⍝: 200	200	200

⍝⍝ convnet •prefs ["kinds":"static"]
⍝ The net's forward pass and cross-entropy for 32 images, with Rank mapping `image` over them.
⍝ Expected: 3.7 copies. The convolution, a product of 6,272 windows of 9 pixels with 8 filters, costs about 1.7, what `[6272 9]+.×[9 8]` takes. The bias, the activation and the pooling make three passes over its 50,176 results, about 1.2. The dense layer's product, about 0.3, and about 30 calls, about 0.5, make the rest. The convolution's product dominates.
[cx cy]closs cp
⍝⍝: 7.4	7.4	7.4

⍝ The same loss in index notation.
⍝ Expected: 3.7 copies, as with Rank. The fold over the filter's positions runs as one product, and the reads through computed positions as maps.
[cx cy]clossi cp
⍝⍝: 7.4	7.4	7.4

⍝ The gradient with respect to the filters, the dense layer and their biases, with Rank mapping `image`.
⍝ Expected: 8 copies: the forward pass, about 3.7; the filters' gradient, a product of the windows' transpose with the convolution's 50,176 gradients, about 1.7; the gradients of the pooling, the activation and the bias, three passes over 50,176 items, about 1.2; the dense layer's two products, about 0.6; and the record, about 0.5.
[cx cy]closs∂ cp
⍝⍝: 16	16	16

⍝ The same gradient in index notation.
⍝ Expected: 8 copies, as with Rank.
[cx cy]clossi∂ cp
⍝⍝: 16	16	16

⍝ Five steps of gradient descent, with Rank mapping `image`.
⍝ Expected: 40 copies: five gradients of about 8, and five updates of 4,010 parameters.
cstep⍣5 cp
⍝⍝: 80	80	80

⍝ Five steps of gradient descent, in index notation.
⍝ Expected: 40 copies, as with Rank.
cstepi⍣5 cp
⍝⍝: 80	80	80

⍝⍝ ode •prefs ["kinds":"static"]
⍝ The trajectory of 128 pendulums over 50 steps and its squared error against the observed angles, with the field on the whole 2 by 128 state.
⍝ Expected: 38 copies. Each Runge-Kutta step calls the field 4 times, with about 9 primitive calls on 128 items each, and makes about 16 more primitive calls on 256 items: about 2,600 primitive calls at about 100 ns each, their work included, and 250 dfn calls at about 150 ns each. Calls dominate. The sines of 25,600 angles cost about 1.6 of the total.
[sa ot]oloss op
⍝⍝: 76	76	76

⍝ The same loss with the field written for one pendulum and mapped by Rank over a 128 by 2 state.
⍝ Expected: 38 copies, as with the field on the whole state. Rank runs the field once for each call, on all 128 pendulums together, in a batched call.
[sb ot]olossp op
⍝⍝: 76	76	76

⍝ The gradient with respect to k and c, through all 50 steps, with the field on the whole state.
⍝ Expected: 55 copies. A program holds the forward pass's 2,850 calls as steps, and its transpose as about twice as many, at about 50 ns a step on arrays of 128 to 256 items: a program of 31 steps on 128 items replays in about 3 µs. Steps dominate. Their arithmetic is about 6.
[sa ot]oloss∂ op
⍝⍝: 83	83	83

⍝ The same gradient with Rank mapping `pendulum`.
⍝ Expected: 55 copies, as with the field on the whole state.
[sb ot]olossp∂ op
⍝⍝: 83	83	83

⍝ Five steps of gradient descent, with the field on the whole state.
⍝ Expected: 275 copies: five gradients of about 55.
ostep⍣5 op
⍝⍝: 420	420	420

⍝ Five steps of gradient descent, with Rank mapping `pendulum`.
⍝ Expected: 275 copies, as with the field on the whole state.
ostepp⍣5 op
⍝⍝: 420	420	420

⍝⍝ logistic •prefs ["kinds":"static"]
⍝ The cross-entropy of 1,024 samples of 64 features, through the logits.
⍝ Expected: 0.9 copies. The product reads the 65,536 inputs once, half a copy at 32 bits. The softplus's exponential and logarithm of 1,024 logits cost about 0.16, and about 15 calls about 0.2. Reading the inputs dominates.
[lx ly]xent lp
⍝⍝: 1.8	1.8	1.8

⍝ The same loss from probabilities, with Agenda picking each sample's log-likelihood by its label.
⍝ Expected: 0.9 copies, as through the logits: the sigmoid's exponential and the logarithms of 1,024 probabilities cost what the softplus costs.
[lx ly]xentp lp
⍝⍝: 1.8	1.8	1.8

⍝ The gradient with respect to the weights and the bias, through the logits.
⍝ Expected: 1.5 copies: the forward pass, about 0.9, and a second read of the inputs for the weights' gradient, 0.5.
[lx ly]xent∂ lp
⍝⍝: 3	3	3

⍝ The same gradient from probabilities.
⍝ Expected: 1.5 copies, as through the logits.
[lx ly]xentp∂ lp
⍝⍝: 3	3	3

⍝ Five steps of gradient descent, each with a new dropout mask from `gen` applied by multiplication.
⍝ Expected: 58 copies, about 11.5 a step. Drawing 65,536 uniform numbers costs about 8.3, at 1 ns each. The 65,536 zeros that the draw reads, the mask and the dropped inputs cost about 1.5, and the gradient about 1.5. The draws dominate.
lstep⍣5 lp
⍝⍝: 87	87	87

⍝ Five steps of gradient descent from probabilities, each with a new dropout mask from `gen` applied by Agenda.
⍝ Expected: 58 copies, as with multiplication. Agenda applies the mask in one pass, as a masked select.
lstepp⍣5 lp
⍝⍝: 87	87	87

