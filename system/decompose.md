

# `•decompose` — Matrix decompositions

`kind •decompose m` factors the matrix `m` and returns the factors as a
record. `kind` names the decomposition, as text or as the `kind` of a
record of options:

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
<tr>
<td><code>"lu"</code></td>
<td><code>l</code>, <code>u</code>, <code>p</code></td>
<td><code>[p]⌷m</code> is <code>l+.×u</code>, with <code>l</code> lower
triangular with 1 on its diagonal, <code>u</code> upper triangular, and
<code>p</code> the rows’ positions</td>
</tr>
</tbody>
</table>

The SVD and the QR decomposition are thin, unless option `complete` asks
for the complete QR. For an `r`-by-`c` matrix, `u` and `q` have `r⌊c`
columns. The factors are floats, apart from `"lu"`’s positions `p`. A
real matrix gives real factors, apart from complex eigenvalues and their
eigenvectors. A Hermitian matrix, one equal to its conjugate transpose,
has real eigenvalues in ascending order and orthonormal eigenvectors.
Each eigenvector of a real matrix has unit length.
[`⌹`](../glyphs/domino.qmd) solves linear systems.

``` bpl
("svd" •decompose [1 2⋄3 4⋄5 6]).s     ⍝ 9.525518091565104 0.5143005806586441
("eigen" •decompose [2 1⋄1 2]).values  ⍝ [1 3]
```

Where option `vectors` is false, `"svd"` and `"eigen"` give only `s` or
`values`, in less time. It’s true by default. Where option `complete` is
true, `"qr"` gives a square `q`, and an `r` with as many rows as `m`.
It’s false by default.

``` bpl
(["kind":"svd" "vectors":$f]•decompose [1 2⋄3 4⋄5 6]).s   ⍝ 9.525518091565104 0.5143005806586441
⍴(["kind":"qr" "complete":$t]•decompose [1 2⋄3 4⋄5 6]).q  ⍝ [3 3]ₓ
```

Option `rank`, a positive integer, makes `"svd"` give only the first
`rank` singular values, with their vectors unless `vectors` is false. A
randomized method computes them, in far less time than the full SVD of a
large matrix takes. The result is an approximation. The faster the
singular values beyond the `rank`th decay, the smaller its error. For a
`rank` within 10 of the matrix’s smaller side, the result is the full
SVD’s first `rank` columns.

``` bpl
(["kind":"svd" "rank":1]•decompose [1 2⋄3 4⋄5 6]).s   ⍝ [9.525518091565104]
⍴(["kind":"svd" "rank":1]•decompose [1 2⋄3 4⋄5 6]).v  ⍝ [2 1]ₓ
```

The determinant is `-.×m`. The `"lu"` factors give its sign, and the
logarithm of its magnitude, which can’t overflow:

``` bpl
f←"lu" •decompose [1 2⋄3 4]
f.p                       ⍝ [1 0]ₓ
d←0 0⍉f.u
(-.× f.p=⊗⍳≢f.p)××/×d     ⍝ ¯1ₓ
+/⍟|d                     ⍝ 0.6931471805599453
```

`kind •decompose⁻¹ factors` multiplies a record of factors back into the
matrix, as the table shows. For `"eigen"`, the matrix is
`(vectors×⍤1 values)+.×⌹vectors`, and for `"lu"` it’s `[⍋p]⌷l+.×u`.
Eigenvectors that aren’t independent, such as those of the defective
matrix `[1 1⋄0 1]`, can’t determine the matrix.

``` bpl
f←"cholesky" •decompose [4 2⋄2 3]
"cholesky" •decompose⁻¹ f   ⍝ [4 2⋄2 3]
```

Errors: `RANK` for an argument that isn’t a matrix, or a factor of the
wrong rank; `LENGTH` for `"eigen"`, `"cholesky"` or `"lu"` of a matrix
that isn’t square, or factors whose sizes don’t agree; `DOMAIN` for an
unknown decomposition or option, a `vectors` of false for a kind other
than `"svd"` and `"eigen"`, a `complete` of true for a kind other than
`"qr"`, a `rank` that isn’t a positive integer or that’s given for a
kind other than `"svd"`, an empty or nonnumeric matrix, `"cholesky"` of
a matrix that isn’t Hermitian positive definite, a missing factor,
eigenvectors that aren’t independent, or a `p` that doesn’t hold each
position once.
