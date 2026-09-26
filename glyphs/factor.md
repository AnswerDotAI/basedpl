

# `⨸` — Factor

`⨸N`: sorted prime factors, with multiplicity. J’s `q:` family. Positive
integral input; exact integer results.

``` apl
⨸700                 ⍝ 2ₓ 2ₓ 5ₓ 5ₓ 7ₓ
⨸1                   ⍝ 0⍴0ₓ
⨸⍣¯1 ⨸700            ⍝ 700ₓ
```

Inverse: product. `K⨸N` selects exponents or a factor table.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>K</th>
<th>Result</th>
</tr>
</thead>
<tbody>
<tr>
<td>Nonnegative integer</td>
<td>Exponents of the first K primes; remaining factors are omitted</td>
</tr>
<tr>
<td><code>∞</code></td>
<td>Exponents through the largest factor</td>
</tr>
<tr>
<td>Negative integer</td>
<td>Last <code>\|K</code> distinct factors and their exponents, in two
rows</td>
</tr>
<tr>
<td><code>¯∞</code></td>
<td>Complete two-row factor/exponent table</td>
</tr>
</tbody>
</table>

``` apl
2⨸700                ⍝ 2ₓ 0ₓ
∞⨸700                ⍝ 2ₓ 0ₓ 2ₓ 1ₓ
¯2⨸700               ⍝ [5ₓ 7ₓ ⋄ 2ₓ 1ₓ]
¯∞⨸700               ⍝ [2ₓ 5ₓ 7ₓ ⋄ 2ₓ 2ₓ 1ₓ]
0⨸700                ⍝ 0⍴0ₓ
¯∞⨸1                 ⍝ 2 0⍴0ₓ
```

Scalar cells; results assemble with fill. Large factors follow
[Prime’s](prime.qmd) probable-prime policy.

## Errors

- `DOMAIN`: nonpositive/non-integral `N`; non-integral finite `K`
