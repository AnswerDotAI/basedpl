

# `⍒` — Grade down / Grade down by

Function. Keys: `Alt-Backslash g`. Hanzi: `降`. Ranks: `∞` monadic,
`∞ ∞` dyadic

`⍒Y` gives indices that sort major cells descending. Stable, with the
same [ordering](delta-stile.qmd) as Grade up.

``` bpl
⍒30 10 20          ⍝ [0 2 1]ₓ
⍒2 1 2 1           ⍝ [0 2 1 3]ₓ
```

`C⍒Y` uses character collation `C`.

``` bpl
"cba"⍒"abc"        ⍝ [0 1 2]ₓ
```

See [`>`](greater-than.qmd) to sort.

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
<td><code>k↑⍒Y</code>, <code>k↑⍒ Y</code> or <code>(k↑)∘⍒</code></td>
<td>a vector of reals or characters without keys, with the count at most
a quarter of its length</td>
<td>selects the first <code>k</code> positions of the descending grade,
or the last <code>-k</code>, without ordering the rest</td>
</tr>
<tr>
<td><code>↑⍒Y</code>, <code>i⌷⍒Y</code> or <code>i⊃⍒Y</code></td>
<td>the same vectors</td>
<td>selects one position without ordering the rest</td>
</tr>
<tr>
<td><code>⍋⍒Y</code>, <code>⍋⍒ Y</code> or <code>⍋∘⍒</code></td>
<td>any <code>Y</code> without keys on its first axis</td>
<td>gives each major cell’s rank in descending order by inverting the
grade, without grading it again</td>
</tr>
</tbody>
</table>

``` bpl
v←30 10 20 50 40 60 70 80 90 0
2↑⍒ v                                ⍝ [8 7]ₓ
⍋⍒ v                                 ⍝ [6 8 7 4 5 3 2 1 0 9]ₓ
(•explain "2↑⍒ v").fast.combination  ⍝ ["partial grade"]
```

## Inverse

`⍒⁻¹P` gives an array whose descending grade is the permutation `P`. Any
other `P` is a `DOMAIN` error.

``` bpl
⍒⁻¹2 0 1   ⍝ [1 0 2]ₓ
```
