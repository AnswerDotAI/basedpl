

# `•decompose` — Matrix decompositions

`kind •decompose m` factors the matrix `m` and returns the factors as a
record. `kind` names the decomposition:

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th><code>kind</code></th>
<th>Fields</th>
<th>Factors</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"svd"</code></td>
<td><code>u</code>, <code>s</code>, <code>v</code></td>
<td><code>m</code> is <code>u×⍤1s +.× +⍉v</code>, with the singular
values <code>s</code> in descending order</td>
</tr>
<tr>
<td><code>"qr"</code></td>
<td><code>q</code>, <code>r</code></td>
<td><code>m</code> is <code>q+.×r</code>, with orthonormal columns in
<code>q</code>, and <code>r</code> upper triangular</td>
</tr>
<tr>
<td><code>"eigen"</code></td>
<td><code>values</code>, <code>vectors</code></td>
<td>each column of <code>vectors</code> is an eigenvector of
<code>m</code>, for the eigenvalue at the same position in
<code>values</code></td>
</tr>
<tr>
<td><code>"cholesky"</code></td>
<td><code>l</code></td>
<td><code>m</code> is <code>l+.×+⍉l</code>, with <code>l</code> lower
triangular</td>
</tr>
</tbody>
</table>

The SVD and the QR decomposition are thin. For an `r`-by-`c` matrix, `u`
and `q` have `r⌊c` columns. The factors are floats. A real matrix gives
real factors, apart from complex eigenvalues and their eigenvectors. A
Hermitian matrix, one equal to its conjugate transpose, has real
eigenvalues in ascending order and orthonormal eigenvectors.
[`⌹`](../glyphs/domino.qmd) solves linear systems.

``` bpl
("svd" •decompose 3 2⍴1 2 3 4 5 6).s     ⍝ 9.525518091565104 0.5143005806586441
("eigen" •decompose 2 2⍴2 1 1 2).values  ⍝ [1 3]
```

`kind •decompose⁻¹ factors` multiplies a record of factors back into the
matrix, as the table shows. For `"eigen"`, the matrix is
`(vectors×⍤1 values)+.×⌹vectors`. Eigenvectors that aren’t independent,
such as those of the defective matrix `[1 1⋄0 1]`, can’t determine the
matrix.

``` bpl
f←"cholesky" •decompose 2 2⍴4 2 2 3
"cholesky" •decompose⁻¹ f   ⍝ [4 2⋄2 3]
```

Errors: `RANK` for an argument that isn’t a matrix, or a factor of the
wrong rank; `LENGTH` for `"eigen"` or `"cholesky"` of a matrix that
isn’t square, or factors whose sizes don’t agree; `DOMAIN` for an
unknown decomposition, an empty or nonnumeric matrix, `"cholesky"` of a
matrix that isn’t Hermitian positive definite, a missing factor, or
eigenvectors that aren’t independent.
