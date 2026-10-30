

# `⨸` — Factors / Factor spec

Function. Keys: `Alt-o u`. Hanzi: `因`. Ranks: `0` monadic, `0 0` dyadic

`⨸N`: sorted prime factors, with multiplicity. J’s `q:` family. Positive
whole input; exact integer results.

``` bpl
⨸700                 ⍝ [2 2 5 5 7]ₓ
⨸1                   ⍝ ⍬ₓ
⨸⁻¹⨸700              ⍝ 700ₓ
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
<td>Nonnegative whole number</td>
<td>Exponents of the first K primes; remaining factors are omitted</td>
</tr>
<tr>
<td><code>∞</code></td>
<td>Exponents through the largest factor</td>
</tr>
<tr>
<td>Negative whole number</td>
<td>Last <code>\|K</code> distinct factors and their exponents, in two
rows</td>
</tr>
<tr>
<td><code>¯∞</code></td>
<td>Complete two-row factor/exponent table</td>
</tr>
</tbody>
</table>

``` bpl
2⨸700                ⍝ [2 0]ₓ
∞⨸700                ⍝ [2 0 2 1]ₓ
¯2⨸700               ⍝ [5 7⋄2 1]ₓ
¯∞⨸700               ⍝ [2 5 7⋄2 2 1]ₓ
0⨸700                ⍝ ⍬ₓ
¯∞⨸1                 ⍝ 2 0⍴0ₓ
```

Unit cells; results assemble with fill. Large factors follow
[Prime’s](stile-tilde.qmd) probable-prime policy.

## Errors

- `DOMAIN`: an `N` that isn’t positive and whole; a finite `K` that
  isn’t whole
