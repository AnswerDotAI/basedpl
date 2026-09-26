

# `ℙ` — Prime

`ℙN`: the prime with N primes before it, so positions count from 0 and
`ℙ0` is 2. J’s `p:` family. Integral inputs; exact integer results.

``` apl
ℙ⍳8               ⍝ 2ₓ 3ₓ 5ₓ 7ₓ 11ₓ 13ₓ 17ₓ 19ₓ
ℙ⍣¯1 ℙ⍳4          ⍝ 0ₓ 1ₓ 2ₓ 3ₓ
```

`KℙN` selects an operation; codes retain J’s values.

<table>
<thead>
<tr>
<th>K</th>
<th>Result</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>¯4</code></td>
<td>Previous prime, strictly below N</td>
</tr>
<tr>
<td><code>¯1</code></td>
<td>Number of primes strictly below N</td>
</tr>
<tr>
<td><code>0</code></td>
<td>N is not prime</td>
</tr>
<tr>
<td><code>1</code></td>
<td>N is prime</td>
</tr>
<tr>
<td><code>2</code></td>
<td>Two-row distinct-factor/exponent table</td>
</tr>
<tr>
<td><code>3</code></td>
<td>Repeated prime factors</td>
</tr>
<tr>
<td><code>4</code></td>
<td>Next prime, strictly above N</td>
</tr>
<tr>
<td><code>5</code></td>
<td>Euler’s totient</td>
</tr>
</tbody>
</table>

``` apl
¯1ℙ1 2 3 4 5 6     ⍝ 0ₓ 0ₓ 1ₓ 2ₓ 2ₓ 3ₓ
1ℙ¯1 0 1 2 3 4     ⍝ 0ₓ 0ₓ 0ₓ 1ₓ 1ₓ 0ₓ
4ℙ1 2 3 4 5        ⍝ 2ₓ 3ₓ 5ₓ 5ₓ 7ₓ
5ℙ1 2 3 4 5 6      ⍝ 1ₓ 1ₓ 2ₓ 2ₓ 4ₓ 2ₓ
2ℙ700               ⍝ [2ₓ 5ₓ 7ₓ ⋄ 2ₓ 2ₓ 1ₓ]
```

Scalar cells; results assemble with fill. Inverse: the number of primes
below the argument, which is a prime’s index. Primality testing is
deterministic through 64 bits, probabilistic above that (false-positive
bound `2⁻⁶⁴`).

See [Factor](factor.qmd).

## Errors

- `DOMAIN`: invalid selector/input; previous prime at/below 2
