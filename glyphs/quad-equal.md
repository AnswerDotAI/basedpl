

# `⌸` — Key

Monadic operator. Keys: `Alt-q =`. Hanzi: `组` (`組`)

`K f⌸Y` groups major cells of `Y` by the corresponding major cells of
`K`, the keys. Calls `key f group`; groups appear in first-occurrence
order.

``` bpl
1 1 2{+/ω}⌸10 20 30 ⍝ 30 30
```

`f⌸Y` is `Y f⌸ι≢Y`: the major cells of `Y` are the keys, and their
positions are the values.

Keys match with [tolerance](../evaluation.qmd#equality-and-ordering), as
in `≠`. Results are assembled with fill. With empty input, `f` is called
once on [prototypes](../terms.qmd#term-prototype), to give the result’s
cell shape.

## Fast combinations

BPL computes each of these combinations with a dedicated algorithm.
[Trains](../evaluation.qmd#trains) says which spellings BPL recognises.

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th>Combination</th>
<th>Applies to</th>
<th>Benefit</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>≢ᵘ⌸Y</code> or <code>K ≢ᵘ⌸Y</code></td>
<td>any keys</td>
<td>counts each group’s major cells without building the groups</td>
</tr>
<tr>
<td><code>K f/ᵘ⌸Y</code> or <code>K f⌿ᵘ⌸Y</code>, with <code>f</code> of
<code>+ ⌈ ⌊</code>, or <code>×</code> of floats</td>
<td>a vector <code>Y</code> of floats, integers or Booleans</td>
<td>folds every group in one pass, without a call of the operand for
each group</td>
</tr>
</tbody>
</table>

[Unary](superscript-u.qmd) `ᵘ` makes the operand ignore the key and use
the group as its right argument.

``` bpl
≢ᵘ⌸"banana"                                ⍝ [1 3 2]ₓ
1 1 2 +/ᵘ⌸ 10 20 30                        ⍝ 30 30
(•explain "1 1 2 +/ᵘ⌸ 10 20 30").fast.rule  ⍝ ["group folds"]
```
