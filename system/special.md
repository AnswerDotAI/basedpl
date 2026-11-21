

# `•special` — Special functions

`•special name` returns the special function called `name`, as SciPy
names it, with SciPy’s order of arguments. For a list of names,
`•special` returns a list of functions.

``` bpl
[erf gammaln]←•special "erf" "gammaln"
erf 0.5                ⍝ 0.5204998778
gammaln 5              ⍝ 3.17805383
```

Each function works item by item on real numbers and gives floats, as
`⊛` does. Exact numbers and integers are converted to floats.

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th>Name</th>
<th>Call</th>
<th>Result</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"gammaln"</code></td>
<td><code>gammaln x</code></td>
<td>ln |Γ(x)|</td>
</tr>
<tr>
<td><code>"digamma"</code></td>
<td><code>digamma x</code></td>
<td>ψ(x), the derivative of ln Γ(x)</td>
</tr>
<tr>
<td><code>"polygamma"</code></td>
<td><code>n polygamma x</code></td>
<td>ψ⁽ⁿ⁾(x), the nth derivative of ψ, for a whole n</td>
</tr>
<tr>
<td><code>"erf"</code></td>
<td><code>erf x</code></td>
<td>the error function</td>
</tr>
<tr>
<td><code>"erfc"</code></td>
<td><code>erfc x</code></td>
<td>1 − erf x, without the loss of digits for large x</td>
</tr>
<tr>
<td><code>"erfinv"</code></td>
<td><code>erfinv p</code></td>
<td>the x with erf x = p</td>
</tr>
<tr>
<td><code>"beta"</code></td>
<td><code>a beta b</code></td>
<td>B(a, b) = Γ(a) Γ(b) ÷ Γ(a + b)</td>
</tr>
<tr>
<td><code>"gammainc"</code></td>
<td><code>a gammainc x</code></td>
<td>the regularized lower incomplete gamma function P(a, x)</td>
</tr>
<tr>
<td><code>"betainc"</code></td>
<td><code>[a b] betainc x</code></td>
<td>the regularized incomplete beta function Iₓ(a, b)</td>
</tr>
</tbody>
</table>

`betainc` takes its shape parameters a and b as one pair on the left.
For arrays of pairs, apply it with Rank, which pairs each row of the
left argument with an item of the right.

``` bpl
[pg bi]←•special "polygamma" "betainc"
1 pg 1                          ⍝ 1.644934067
[2 3] bi 0.5                    ⍝ 0.6875
[2 3◇4 5] bi¹ ⁰ 0.5 0.25     ⍝ 0.6875 0.1138153076
```

`∂` differentiates each function with respect to its last argument. The
slope of `gammaln` is digamma, the slope of digamma is `1 polygamma`,
and the slope of `n polygamma` is `(n+1) polygamma`. `∂` also
differentiates `beta` with respect to a. `∂` raises `UNSUPPORTED` for a
slope with respect to the order of `polygamma`, the a of `gammainc` or
the pair of `betainc`:

``` bpl
[erf bi]←•special "erf" "betainc"
erf∂ 0.5                ⍝ 0.8787825789
{[2 3] bi ω}∂ 0.5       ⍝ 1.5
```

`0 polygamma x` is digamma. For a whole n from 1 to 170, `n polygamma x`
has a result for any x above 0. A result too large for its float kind is
`∞` or `¯∞`, as `170 polygamma 0.5` is.

An unknown name, a name that isn’t a string or a complex argument raises
`DOMAIN`. A call with the wrong number of arguments raises `SYNTAX`. A
left argument of `betainc` that isn’t one pair raises `LENGTH`.

Under dynamic kinds, `digamma`, `polygamma`, `erfinv`, `beta`,
`gammainc` and `betainc` raise `DOMAIN` when non-NaN arguments produce
NaN. This includes digamma at a negative integer and incomplete beta
outside its domain. Under [static kinds](../numbers.qmd#static-kinds),
these numeric checks are disabled and NaN is returned. Infinite results
do not raise an error.
