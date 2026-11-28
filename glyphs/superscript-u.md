

# [`ᵘ`](https://answerdotai.github.io/basedpl/glyphs/superscript-u.html) — Unary

Monadic operator. Keys: `Alt-6 u`. Hanzi: `单` (`單`)

`fᵘ` calls `f` with its right argument alone. Any left argument is
ignored.

<table>
<thead>
<tr>
<th>Call</th>
<th>Definition</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>fᵘ Y</code></td>
<td><code>f Y</code></td>
</tr>
<tr>
<td><code>X fᵘ Y</code></td>
<td><code>f Y</code></td>
</tr>
</tbody>
</table>

``` bpl
10-ᵘ3                   ⍝ ¯3
```

`fᵘ` is the two-function [atop](../composition.qmd#trains) `f⊢`. The
superscript needs no parentheses when used as another operator’s
operand: `+/ᵘ⌸` sums each [Key](quad-equal.qmd) group, ignoring its key,
as `(+/⊢)⌸` does.
