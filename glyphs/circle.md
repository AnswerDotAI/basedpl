

# `○` — Unit circle / Circular functions

Keys: `Alt-o`. Ranks: `0` monadic, `0 0` dyadic

`○Y`: e<sup>iY</sup> = cos Y + i sin Y. Real Y is an angle in radians.
Pervasive.

``` bpl
○0                ⍝ 1
○0j1              ⍝ *¯1
```

`K○Y` applies the circular function with code `K`, from 0 to 8. `K○⁻¹Y`
applies its inverse. Pervasive; angles in radians.

``` bpl
1○0               ⍝ 0
2○0               ⍝ 1
1○⁻¹1             ⍝ 1.5707963267948966
```

<table>
<thead>
<tr>
<th><code>k</code></th>
<th><code>k○y</code></th>
<th><code>k○⁻¹y</code></th>
</tr>
</thead>
<tbody>
<tr>
<td>0</td>
<td><code>√(1−y²)</code></td>
<td><code>√(1−y²)</code></td>
</tr>
<tr>
<td>1</td>
<td>sin</td>
<td>arcsin</td>
</tr>
<tr>
<td>2</td>
<td>cos</td>
<td>arccos</td>
</tr>
<tr>
<td>3</td>
<td>tan</td>
<td>arctan</td>
</tr>
<tr>
<td>4</td>
<td><code>√(1+y²)</code></td>
<td>APL branch of <code>√(y²−1)</code></td>
</tr>
<tr>
<td>5</td>
<td>sinh</td>
<td>arcsinh</td>
</tr>
<tr>
<td>6</td>
<td>cosh</td>
<td>arccosh</td>
</tr>
<tr>
<td>7</td>
<td>tanh</td>
<td>arctanh</td>
</tr>
<tr>
<td>8</td>
<td><code>√(-1−y²)</code></td>
<td><code>-√(-1−y²)</code></td>
</tr>
</tbody>
</table>

Uses complex continuation where needed.

APL difference: there are no negative codes or codes 9 to 12. Write
`¯k○Y` as `k○⁻¹Y`. `∨Y` gives the real and imaginary parts, `|Y` the
magnitude, and `∧Y` the magnitude and phase.

## Inverse

`○⁻¹Y` gives the angle of the unit complex number `Y`. The table above
gives the inverse of each `K○`.

``` bpl
○⁻¹0j1   ⍝ π0.5
```
