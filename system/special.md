

# `•special` — Special functions

`•special name` returns the special function called `name`, as SciPy
names it, with SciPy’s order of arguments. A list of names gives a list
of functions.

``` bpl
[erf gammaln]←•special "erf" "gammaln"
erf 0.5                ⍝ 0.5204998778
gammaln 5              ⍝ 3.17805383
```

Each function works item by item on real numbers and gives floats, as
`⍟` does. Exact numbers and integers convert to floats. A value that a
function leaves undefined, such as digamma at a negative whole number,
is NaN.

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
[2 3⋄4 5] (bi⍤1 0) 0.5 0.25     ⍝ 0.6875 0.1138153076
```

`∂` differentiates each function in its last argument. The slope of
`gammaln` is digamma, digamma’s is `1 polygamma`, and each polygamma’s
is the next order’s. `beta` also differentiates in a. The order of
`polygamma`, the a of `gammainc` and the pair of `betainc` have no
slope, and `∂` gives `UNSUPPORTED` for them:

``` bpl
[erf bi]←•special "erf" "betainc"
erf∂ 0.5                ⍝ 0.8787825789
{[2 3] bi ⍵}∂ 0.5       ⍝ 1.5
```

For an order n from 1, `polygamma` gives NaN for an x from 0 down and
for an n above 170.

Errors: `DOMAIN` for an unknown name, a name that isn’t a string, or a
complex argument; `SYNTAX` for a call with the wrong number of
arguments; `LENGTH` for a left argument of `betainc` that isn’t one
pair.
