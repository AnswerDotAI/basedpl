⍝ distribution:normal — Normal location, standard deviation and quantile
d←3 2 •distribution "normal" ⋄ [d.cdf 3;d.quantile 0.5;d.density 3]
[0.5 3 1÷2×√2×π1]

⍝ distribution:uniform — Uniform endpoints and density
d←2 6 •distribution "uniform" ⋄ [d.quantile 0 0.25 1;d.density 1 3 7]
[2 3 6;0 0.25 0]

⍝ distribution:beta — Uniform beta and symmetric beta
a←1 1 •distribution "beta" ⋄ b←2 2 •distribution "beta" ⋄ [a.quantile 0.25 0.75;b.density 0.5;b.cdf 0.5]
[0.25 0.75;1.5;0.5]

⍝ distribution:binomial — Mass at integers, floor for cdf
d←2 0.5 •distribution "binomial" ⋄ [d.density ¯1 0 0.5 1 2 3;d.cdf ¯1 0.5 1.5 2;d.quantile 0 0.25 0.5 1]
[0 0.25 0 0.5 0.25 0;0 0.25 0.75 1;[0 0 1 2]ₓ]

⍝ distribution:degenerate — Certain events and zero-rate Poisson
b←3 1 •distribution "binomial" ⋄ p←0 •distribution "poisson" ⋄ [b.quantile 0 0.5 1;p.sample 3;p.quantile 0 1]
[3 3 3;0 0 0;0 0]ₓ

⍝ distribution:poisson — Poisson mass and unbounded quantile
d←2 •distribution "poisson" ⋄ [d.density 0 1;d.quantile 0 1;d.cdf 0]
[1 2×*¯2;0ₓ ∞;*¯2]

⍝ distribution:gamma — Gamma uses scale, exponential uses rate
g←1 2 •distribution "gamma" ⋄ e←0.5 •distribution "exponential" ⋄ [g.density 2;e.quantile 1-*¯1;g.cdf 2]
[0.5×*¯1 2 1-*¯1]

⍝ distribution:inversegamma — Inverse gamma shape and scale
d←1 2 •distribution "inversegamma" ⋄ [d.cdf 2;d.density 2;d.quantile *¯1]
[*¯1 0.5×*¯1 2]

⍝ distribution:chisquared — Two degrees of freedom is exponential
d←2 •distribution "chisquared" ⋄ [d.density 2;d.cdf 2]
[0.5×*¯1 1-*¯1]

⍝ distribution:cauchy-student — Student t with one degree of freedom
c←0 1 •distribution "cauchy" ⋄ t←1 •distribution "student" ⋄ [c.cdf 1;t.cdf 1;c.density 0;t.quantile 0.5]
[0.75 0.75 ÷π1 0]

⍝ distribution:fisher — F with equal degrees of freedom
d←2 2 •distribution "fisher" ⋄ [d.cdf 1;d.density 1;d.quantile 0.5]
0.5 0.25 1

⍝ distribution:laplace-logistic — Location and scale
a←3 2 •distribution "laplace" ⋄ b←3 2 •distribution "logistic" ⋄ [a.density 3;b.density 3;a.cdf 3;b.quantile 0.5]
0.25 0.125 0.5 3

⍝ distribution:lognormal-weibull — Log-normal median and Weibull scale
a←0 1 •distribution "lognormal" ⋄ b←1 2 •distribution "weibull" ⋄ [a.quantile 0.5;b.quantile 1-*¯1]
1 2

⍝ distribution:tails — Infinite endpoints and stable logistic tails
d←0 1 •distribution "logistic" ⋄ [d.cdf ¯∞ ¯1000 1000 ∞;d.density ¯∞ ∞;d.quantile 0 1]
[0 0 1 1;0 0;¯∞ ∞]

⍝ distribution:layout — Nested, keyed and empty inputs
d←0 1 •distribution "uniform" ⋄ [d.cdf "a" "b":0.25 0.5;d.cdf [[0.25 0.5] 0⍴⊂[0.25 0.5]]]
[["a":0.25 "b":0.5] [[0.25 0.5] 0⍴⊂[0 0]]]

⍝ distribution:shapes — Shape argument controls unit and empty draws
d←0 1 •distribution "normal" ⋄ [⍴d.sample ⍬;⍴d.sample 2 0 3;⍴d.sample 2 3]
[⍬;2 0 3;2 3]ₓ

⍝ distribution:bernoulli — All four methods on a certain event
d←1 •distribution "bernoulli" ⋄ [d.sample 3;d.density 0 1;d.cdf 0 1;d.quantile 0 0.5 1]
[[1 1 1]ₓ;0 1;0 1;[1 1 1]ₓ]

⍝ distribution:bad-probability — Quantile validates before entering the library
d←0 1 •distribution "normal" ⋄ d.quantile 1.01
⍝ error: DOMAIN ERROR

⍝ distribution:bad-shape — Sampling needs integer dimensions
d←0 1 •distribution "normal" ⋄ d.sample 1.5
⍝ error: DOMAIN ERROR

⍝ distribution:bad-scale — Gamma scale cannot be zero
1 0 •distribution "gamma"
⍝ error: DOMAIN ERROR

⍝ distribution:bad-parameter — Parameters must be finite
∞ 1 •distribution "normal"
⍝ error: DOMAIN ERROR

⍝ distribution:bad-count — Fixed parameter count
0 •distribution "normal"
⍝ error: LENGTH ERROR

⍝ distribution:standard — Without parameters, a distribution takes its standard ones
n←•distribution "normal" ⋄ e←•distribution "exponential" ⋄ [n.cdf 0;n.quantile 0.5;e.cdf 0]
[0.5 0 0]

⍝ distribution:no-standard — A distribution without standard parameters needs them
•distribution "binomial"
⍝ error: DOMAIN ERROR

⍝ distribution:unknown — An unknown name is DOMAIN
0 1 •distribution "normale"
⍝ error: DOMAIN ERROR

⍝ distribution:wide-uniform — Reject intervals that would panic in the sampler
¯1E308 1E308 •distribution "uniform"
⍝ error: DOMAIN ERROR

⍝ distribution:bad-valence — Only sample takes a left argument, and it must be a generator
d←0 1 •distribution "normal" ⋄ 2 d.sample 3
⍝ error: DOMAIN ERROR

⍝ —
d←0 1 •distribution "normal" ⋄ 2 d.density 3
⍝ error: SYNTAX ERROR

⍝ distribution:sample-limit — Check allocations before drawing
d←0 1 •distribution "normal" ⋄ d.sample 1000000001
⍝ error: LIMIT ERROR

⍝ distribution:integer-limit — Trial count is checked before floating-point rounding
9007199254740993ₓ 1 •distribution "binomial"
⍝ error: LIMIT ERROR

⍝ distribution:generator — The same seed gives the same draws
g←•rand 42 ⋄ h←•rand 42 ⋄ d←0 1 •distribution "normal"
[(g.roll 6 6 6) ≡ h.roll 6 6 6;(g d.sample 4) ≡ h d.sample 4;(3 g.deal 10) ≡ 3 h.deal 10]
⍝ =>
$t $t $t

⍝ distribution:generator-stream — Copies of a generator draw from one stream
g←•rand 42 ⋄ h←g ⋄ a←g.roll 1000 1000 ⋄ b←h.roll 1000 1000 ⋄ k←•rand 42 ⋄ (a,b)≡k.roll 4⍴1000   ⍝ $t

⍝ distribution:generator-values — A seed fixes the draws, which pins the generator algorithm
(•rand 42).roll 6 6 6   ⍝ 4 1 5

⍝ distribution:generator-seed — The seed is one nonnegative integer
•rand ¯1
⍝ error: DOMAIN ERROR

⍝ —
•rand 1 2
⍝ error: LENGTH ERROR
