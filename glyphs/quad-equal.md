

# `⌸` — Key

Monadic operator. Keys: `Alt-q =`. Hanzi: `组` (`組`)

`K f⌸Y` groups major cells of `Y` by corresponding keys in `K`. Calls
`key f group`; groups appear in first-occurrence order.

``` bpl
1 1 2{+/⍵}⌸10 20 30 ⍝ 30 30
```

`f⌸Y` uses `Y` as keys and their positions as values.

Keys match with [tolerance](../evaluation.qmd#equality-and-ordering), as
in `≠`. Results assemble with fill. Empty input calls `f` once for its
prototype.

## Fast combinations

BPL computes each of these combinations by an algorithm of its own, in
any spelling. [Trains](../evaluation.qmd#trains) says which spellings
BPL recognises.

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
<td><code>≢⊢ ⌸Y</code> or <code>K (≢⊢)⌸Y</code></td>
<td>any keys</td>
<td>counts each group’s major cells without building the groups</td>
</tr>
<tr>
<td><code>K (f/⊢)⌸Y</code> or <code>K (f⌿⊢)⌸Y</code>, with
<code>f</code> of <code>+ ⌈ ⌊</code>, or <code>×</code> of floats</td>
<td>a vector <code>Y</code> of floats, integers or Booleans</td>
<td>folds every group in one pass, without a call of the operand for
each group</td>
</tr>
</tbody>
</table>

``` bpl
≢⊢ ⌸"banana"                                       ⍝ [1 3 2]ₓ
1 1 2(+/⊢)⌸10 20 30                                ⍝ 30 30
(•explain "1 1 2(+/⊢)⌸10 20 30").fast.combination  ⍝ ["group folds"]
```
