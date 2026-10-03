

# `⍭` — Prime

Keys: `Alt-Backslash ~`

`⍭N`: the prime with `N` primes before it. `⍭0` is 2. J’s `p:` family.
Integral inputs; exact integer results.

``` bpl
⍭⍳8               ⍝ [2 3 5 7 11 13 17 19]ₓ
⍭⁻¹⍭⍳4            ⍝ [0 1 2 3]ₓ
```

`K⍭N` selects an operation by the code `K`. The codes are J’s.

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

``` bpl
¯1⍭1 2 3 4 5 6     ⍝ [0 0 1 2 2 3]ₓ
1⍭¯1 0 1 2 3 4     ⍝ [0 0 0 1 1 0]ₓ
4⍭1 2 3 4 5        ⍝ [2 3 5 5 7]ₓ
5⍭1 2 3 4 5 6      ⍝ [1 1 2 2 4 2]ₓ
2⍭700               ⍝ [2 5 7 ⋄ 2 2 1]ₓ
```

Unit cells; results assemble with fill. Inverse: the number of primes
below the argument, which is a prime’s index. Primality testing is
deterministic through 64 bits, probabilistic above that (false-positive
bound `2⁻⁶⁴`).

See [Factor](factor.qmd).

## Errors

- `DOMAIN`: invalid selector/input; previous prime at/below 2
