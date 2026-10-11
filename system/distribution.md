

# `•distribution` — Distributions

`params •distribution name` returns the distribution called `name`, with
parameters `params`. `•distribution name` uses the distribution’s
standard parameters, listed in the table below. A distribution is a
keyed vector of four functions:

- `sample shape` draws random values. `g sample shape` draws them from a
  generator made by `•rand`.
- `density x` gives the probability density, or the probability mass for
  a discrete distribution.
- `cdf x` gives P(X ≤ x).
- `quantile p` inverts the CDF. `cdf⁻¹` is `quantile`. `quantile⁻¹` is
  `cdf`.

``` bpl
d←•distribution "normal"
d.cdf 0                         ⍝ 0.5
b←10 0.5 •distribution "binomial"
b.quantile 0.5                  ⍝ 5ₓ
```

Parameters are finite real units or vectors. Scale, shape, rate and
degrees of freedom are positive, except where stated.
[Distributions](../distributions.ipynb) shows them in use.

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th>Name</th>
<th>Parameters</th>
<th>Standard</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"normal"</code></td>
<td>μ σ: mean, standard deviation</td>
<td>0 1</td>
</tr>
<tr>
<td><code>"uniform"</code></td>
<td>a b: lower and upper bounds, a &lt; b</td>
<td>0 1</td>
</tr>
<tr>
<td><code>"bernoulli"</code></td>
<td>p ∈ [0,1]</td>
<td></td>
</tr>
<tr>
<td><code>"binomial"</code></td>
<td>n p: integer trials n ≥ 0, p ∈ [0,1]</td>
<td></td>
</tr>
<tr>
<td><code>"poisson"</code></td>
<td>λ ≥ 0: mean</td>
<td></td>
</tr>
<tr>
<td><code>"beta"</code></td>
<td>α β: shapes</td>
<td></td>
</tr>
<tr>
<td><code>"gamma"</code></td>
<td>k θ: shape, scale, with mean kθ</td>
<td></td>
</tr>
<tr>
<td><code>"inversegamma"</code></td>
<td>α β: shape, scale, with density ∝ x⁻⁽ᵅ⁺¹⁾ exp(−β/x)</td>
<td></td>
</tr>
<tr>
<td><code>"exponential"</code></td>
<td>λ: rate, with mean 1/λ</td>
<td>1</td>
</tr>
<tr>
<td><code>"chisquared"</code></td>
<td>ν: degrees of freedom</td>
<td></td>
</tr>
<tr>
<td><code>"student"</code></td>
<td>ν: degrees of freedom, with location 0 and scale 1</td>
<td></td>
</tr>
<tr>
<td><code>"fisher"</code></td>
<td>ν₁ ν₂: degrees of freedom</td>
<td></td>
</tr>
<tr>
<td><code>"cauchy"</code>, <code>"laplace"</code>,
<code>"logistic"</code></td>
<td>location, scale</td>
<td>0 1</td>
</tr>
<tr>
<td><code>"lognormal"</code></td>
<td>μ σ: mean and standard deviation of log(X)</td>
<td>0 1</td>
</tr>
<tr>
<td><code>"weibull"</code></td>
<td>k λ: shape, scale</td>
<td></td>
</tr>
</tbody>
</table>

Errors: `DOMAIN` for an unknown name, a name that isn’t a string, a
monadic call for a distribution with no standard parameters, invalid
parameters, non-real inputs or p ∉ \[0,1\]; `LENGTH` for the wrong
number of parameters; `RANK` for matrix parameters or shapes; `SYNTAX`
for a dyadic call to `density`, `cdf` or `quantile`; `LIMIT` for
oversized shapes or sampler ranges.
