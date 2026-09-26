

# `○` — Unit circle / Circular functions

`○Y`: e<sup>iY</sup> = cos Y + i sin Y. Real Y is an angle in radians.
Pervasive.

``` apl
○0                ⍝ 1
○0j1              ⍝ *¯1
```

`K○Y` selects a circular function. Pervasive; angles in radians.

``` apl
1○0               ⍝ 0
2○0               ⍝ 1
9 11○3j4          ⍝ 3 4
10○3j4            ⍝ 5
```

<table>
<thead>
<tr>
<th><code>k</code></th>
<th><code>k○y</code></th>
<th><code>(-k)○y</code></th>
</tr>
</thead>
<tbody>
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
<td>negative of code 8</td>
</tr>
<tr>
<td>9</td>
<td>real part</td>
<td>identity</td>
</tr>
<tr>
<td>10</td>
<td>magnitude</td>
<td>conjugate</td>
</tr>
<tr>
<td>11</td>
<td>imaginary part</td>
<td>multiply by <code>0j1</code></td>
</tr>
<tr>
<td>12</td>
<td>phase</td>
<td><code>exp(0j1×y)</code></td>
</tr>
</tbody>
</table>

Code 0 is `√(1−y²)`. Uses complex continuation where needed.
