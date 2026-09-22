# Probability distributions

Construct a distribution, then sample or evaluate it:

```apl
n←•normal 0 1
⍴n.sample 2 3             ⍝ 2ₓ 3ₓ
n.cdf 0                  ⍝ 0.5
n.quantile 0.5           ⍝ 0
n.density 0              ⍝ ÷√2×π1
```

Each constructor returns a keyed vector of functions. All four methods are monadic:

| Method | Argument → result |
|---|---|
| `sample` | Shape → independent random draws |
| `density` | x → probability density, or probability mass for discrete distributions |
| `cdf` | x → P(X ≤ x) |
| `quantile` | p ∈ [0,1] → inverse CDF |

`sample 100` returns a vector; `sample ⍬` returns a rank-0 array. Zero dimensions give empty arrays. Continuous samples are floats; discrete samples are exact integers. Results use compact storage where possible.

```apl
b←•binomial 2 0.5
b.density 0 1 2          ⍝ 0.25 0.5 0.25
b.cdf 0.5 1.5           ⍝ 0.25 0.75
b.quantile 0.25 0.5 1   ⍝ 0ₓ 1ₓ 2ₓ
```

Evaluation pervades arrays, preserving shape, nesting, keys and axis names. Parameters belong to the constructed distribution.

```apl
u←•uniform 0 1
u.cdf ('low' 'high':0.2 0.8)   ⍝ 'low' 'high':0.2 0.8
```

## Constructors

Parameters are finite real scalars or vectors. Scale, shape, rate and degrees of freedom are positive, except where stated.

| Constructor | Parameters |
|---|---|
| `•normal` | μ σ: mean, standard deviation |
| `•uniform` | a b: lower, upper bounds; a < b |
| `•bernoulli` | p ∈ [0,1] |
| `•binomial` | n p: integer trials n ≥ 0; p ∈ [0,1] |
| `•poisson` | λ ≥ 0: mean/rate |
| `•beta` | α β: shapes |
| `•gamma` | k θ: shape, scale; mean kθ |
| `•inversegamma` | α β: shape, scale; density ∝ x⁻⁽ᵅ⁺¹⁾ exp(−β/x) |
| `•exponential` | λ: rate; mean 1/λ |
| `•chisquared` | ν: degrees of freedom |
| `•student` | ν: degrees of freedom; location 0, scale 1 |
| `•fisher` | ν₁ ν₂: F degrees of freedom |
| `•cauchy` | location, scale |
| `•laplace` | location, scale |
| `•logistic` | location, scale |
| `•lognormal` | μ σ: mean and standard deviation of log(X) |
| `•weibull` | k λ: shape, scale |

Gamma takes **scale**, whereas exponential takes **rate**. These describe the same distribution:

```apl
g←•gamma 1 2
e←•exponential 0.5
(g.cdf 2) = e.cdf 2      ⍝ 1ₓ
```

Quantile endpoints give the support bounds, including infinity. Discrete quantiles return the smallest supported integer whose CDF reaches p; p=0 gives the lower support bound. Certain events stay constant at both endpoints.

```apl
n←•normal 0 1
n.quantile 0 1           ⍝ ¯∞ ∞
p←•poisson 0
p.sample 3               ⍝ 0ₓ 0ₓ 0ₓ
```

## Python

The same functions accept Python/NumPy values:

```python
from basedpl import Session

with Session() as apl:
    normal = apl.fn('•normal')([0., 1.])
    draws = normal['sample']([2, 3]).np
    assert draws.shape == (2, 3)
    assert normal['cdf'](0.).py == 0.5
```

Numerical routines use [statrs](https://docs.rs/statrs/latest/statrs/); logistic uses its closed-form CDF and inverse. Random draws use the thread-local RNG. Binomial trials and Poisson rate are limited to 2⁵³ by the floating-point samplers. Gamma scale must have a finite reciprocal; uniform intervals must fit the sampler's finite range.

Errors: DOMAIN for invalid parameters, non-real inputs or p ∉ [0,1]; LENGTH for wrong parameter count; RANK for matrix parameters/shapes; SYNTAX for dyadic calls; LIMIT for oversized shapes or sampler ranges.
