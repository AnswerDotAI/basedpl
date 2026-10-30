

# `⍋` — Grade up / Grade up by

Function. Keys: `Alt-Backslash 7`. Hanzi: `升`. Ranks: `∞` monadic,
`∞ ∞` dyadic

`⍋Y` gives indices that sort major cells ascending. Stable; uses
[structural ordering](../evaluation.qmd#equality-and-ordering), without
tolerance.

``` bpl
⍋30 10 20          ⍝ [1 2 0]ₓ
v←30 10 20 ⋄ [⍋v]⌷v ⍝ 10 20 30
⍋2 1 2 1           ⍝ [1 3 0 2]ₓ
```

The positions keep the name of `Y`’s first axis. Selecting with a grade
along axis `i` sorts each column of a matrix with axes `i` and `j`:

``` bpl
x←["i":3 "j":4]⍴3 1 4 1 5 9 2 6 5 3 5 8
x.[(⍋⍠"i" x)]      ⍝ ["i":3 "j":4]⍴[3 1 2 1⋄5 3 4 6⋄5 9 5 8]
```

`C⍋Y` uses a character collation array `C`.

``` bpl
"cba"⍋"abc"        ⍝ [2 1 0]ₓ
```

See [`⍒`](del-stile.qmd), and [`<`](less-than.qmd) to sort.

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
<td><code>k↑⍋Y</code>, <code>k↑⍋ Y</code> or <code>(k↑)∘⍋</code></td>
<td>a vector of reals or characters without keys, with the count at most
a quarter of its length</td>
<td>selects the first <code>k</code> positions of the grade, or the last
<code>-k</code>, without ordering the rest</td>
</tr>
<tr>
<td><code>↑⍋Y</code>, <code>i⌷⍋Y</code> or <code>i⊃⍋Y</code></td>
<td>the same vectors</td>
<td>selects one position without ordering the rest</td>
</tr>
<tr>
<td><code>⍋⍋Y</code>, <code>⍋⍋ Y</code> or <code>⍋∘⍋</code></td>
<td>any <code>Y</code> without keys on its first axis</td>
<td>gives each major cell’s rank by inverting the first grade, without
grading it again</td>
</tr>
</tbody>
</table>

``` bpl
v←30 10 20 50 40 60 70 80 90 0
2↑⍋ v                                ⍝ [9 1]ₓ
⍋⍋ v                                 ⍝ [3 1 2 5 4 6 7 8 9 0]ₓ
(•explain "2↑⍋ v").fast.combination  ⍝ ["partial grade"]
```

## Inverse

`⍋⁻¹P` gives an array whose grade is the permutation `P`. Any other `P`
is a `DOMAIN` error.

``` bpl
⍋⁻¹2 0 1   ⍝ [1 2 0]ₓ
```
