⍝ distribution:normal — Normal location, standard deviation and quantile
d←•normal 3 2 ⋄ [d.cdf 3;d.quantile 0.5;d.density 3]
[0.5 3 1÷2×√2×π1]

⍝ distribution:uniform — Uniform endpoints and density
d←•uniform 2 6 ⋄ [d.quantile 0 0.25 1;d.density 1 3 7]
[2 3 6;0 0.25 0]

⍝ distribution:beta — Uniform beta and symmetric beta
a←•beta 1 1 ⋄ b←•beta 2 2 ⋄ [a.quantile 0.25 0.75;b.density 0.5;b.cdf 0.5]
[0.25 0.75;1.5;0.5]

⍝ distribution:binomial — Mass at integers, floor for cdf
d←•binomial 2 0.5 ⋄ [d.density ¯1 0 0.5 1 2 3;d.cdf ¯1 0.5 1.5 2;d.quantile 0 0.25 0.5 1]
[0 0.25 0 0.5 0.25 0;0 0.25 0.75 1;0ₓ 0ₓ 1ₓ 2ₓ]

⍝ distribution:degenerate — Certain events and zero-rate Poisson
b←•binomial 3 1 ⋄ p←•poisson 0 ⋄ [b.quantile 0 0.5 1;p.sample 3;p.quantile 0 1]
[3ₓ 3ₓ 3ₓ;0ₓ 0ₓ 0ₓ;0ₓ 0ₓ]

⍝ distribution:poisson — Poisson mass and unbounded quantile
d←•poisson 2 ⋄ [d.density 0 1;d.quantile 0 1;d.cdf 0]
[1 2×*¯2;0 ∞;*¯2]

⍝ distribution:gamma — Gamma uses scale, exponential uses rate
g←•gamma 1 2 ⋄ e←•exponential 0.5 ⋄ [g.density 2;e.quantile 1-*¯1;g.cdf 2]
[0.5×*¯1 2 1-*¯1]

⍝ distribution:inversegamma — Inverse gamma shape and scale
d←•inversegamma 1 2 ⋄ [d.cdf 2;d.density 2;d.quantile *¯1]
[*¯1 0.5×*¯1 2]

⍝ distribution:chisquared — Two degrees of freedom is exponential
d←•chisquared 2 ⋄ [d.density 2;d.cdf 2]
[0.5×*¯1 1-*¯1]

⍝ distribution:cauchy-student — Student t with one degree of freedom
c←•cauchy 0 1 ⋄ t←•student 1 ⋄ [c.cdf 1;t.cdf 1;c.density 0;t.quantile 0.5]
[0.75 0.75 ÷π1 0]

⍝ distribution:fisher — F with equal degrees of freedom
d←•fisher 2 2 ⋄ [d.cdf 1;d.density 1;d.quantile 0.5]
0.5 0.25 1

⍝ distribution:laplace-logistic — Location and scale
a←•laplace 3 2 ⋄ b←•logistic 3 2 ⋄ [a.density 3;b.density 3;a.cdf 3;b.quantile 0.5]
0.25 0.125 0.5 3

⍝ distribution:lognormal-weibull — Log-normal median and Weibull scale
a←•lognormal 0 1 ⋄ b←•weibull 1 2 ⋄ [a.quantile 0.5;b.quantile 1-*¯1]
1 2

⍝ distribution:tails — Infinite endpoints and stable logistic tails
d←•logistic 0 1 ⋄ [d.cdf ¯∞ ¯1000 1000 ∞;d.density ¯∞ ∞;d.quantile 0 1]
[0 0 1 1;0 0;¯∞ ∞]

⍝ distribution:layout — Nested, keyed and empty inputs
d←•uniform 0 1 ⋄ [d.cdf "a" "b":0.25 0.5;d.cdf [[0.25 0.5] 0⍴0]]
[["a":0.25 "b":0.5] [[0.25 0.5] 0⍴0]]

⍝ distribution:shapes — Shape argument controls scalar and empty draws
d←•normal 0 1 ⋄ [⍴d.sample ⍬;⍴d.sample 2 0 3;⍴d.sample 2 3]
[⍬;2ₓ 0ₓ 3ₓ;2ₓ 3ₓ]

⍝ distribution:bernoulli — All four methods on a certain event
d←•bernoulli 1 ⋄ [d.sample 3;d.density 0 1;d.cdf 0 1;d.quantile 0 0.5 1]
[1ₓ 1ₓ 1ₓ;0 1;0 1;1ₓ 1ₓ 1ₓ]

⍝ distribution:bad-probability — Quantile validates before entering the library
d←•normal 0 1 ⋄ d.quantile 1.01
⍝ error: DOMAIN ERROR

⍝ distribution:bad-shape — Sampling needs integer dimensions
d←•normal 0 1 ⋄ d.sample 1.5
⍝ error: DOMAIN ERROR

⍝ distribution:bad-scale — Gamma scale cannot be zero
•gamma 1 0
⍝ error: DOMAIN ERROR

⍝ distribution:bad-parameter — Parameters must be finite
•normal ∞ 1
⍝ error: DOMAIN ERROR

⍝ distribution:bad-count — Fixed constructor arity
•normal 0
⍝ error: LENGTH ERROR

⍝ distribution:wide-uniform — Reject intervals that would panic in the sampler
•uniform ¯1E308 1E308
⍝ error: DOMAIN ERROR

⍝ distribution:bad-valence — Only sample takes a left argument, and it must be a generator
d←•normal 0 1 ⋄ 2 d.sample 3
⍝ error: DOMAIN ERROR

⍝ —
d←•normal 0 1 ⋄ 2 d.density 3
⍝ error: SYNTAX ERROR

⍝ distribution:sample-limit — Check allocations before drawing
d←•normal 0 1 ⋄ d.sample 1000001
⍝ error: LIMIT ERROR

⍝ distribution:integer-limit — Trial count is checked before floating-point rounding
•binomial 9007199254740993ₓ 1
⍝ error: LIMIT ERROR

⍝ distribution:generator — The same seed gives the same draws
g←•rand 42 ⋄ h←•rand 42 ⋄ d←•normal 0 1
[(g.roll 6 6 6) ≡ h.roll 6 6 6;(g d.sample 4) ≡ h d.sample 4;(3 g.deal 10) ≡ 3 h.deal 10]
⍝ =>
1ₓ 1ₓ 1ₓ

⍝ distribution:generator-stream — Copies of a generator draw from one stream
g←•rand 42 ⋄ h←g ⋄ a←g.roll 1000 1000 ⋄ b←h.roll 1000 1000 ⋄ k←•rand 42 ⋄ (a,b)≡k.roll 4⍴1000   ⍝ 1ₓ

⍝ distribution:generator-values — A seed fixes the draws, which pins the generator algorithm
(•rand 42).roll 6 6 6   ⍝ 4 1 5

⍝ distribution:generator-seed — The seed is one nonnegative integer
•rand ¯1
⍝ error: DOMAIN ERROR

⍝ —
•rand 1 2
⍝ error: LENGTH ERROR
