

# `⍟` — Natural log / Logarithm

Function. Keys: `Alt-o 8`. Hanzi: `对` (`對`). Ranks: `0` monadic, `0 0`
dyadic

`⍟Y`: ln Y. Pervasive.

``` bpl
⍟1                ⍝ 0
```

`X⍟Y`: log<sub>X</sub> Y. Pervasive.

``` bpl
2⍟8               ⍝ 3
10⍟100            ⍝ 2
1⍟1               ⍝ 0
```

Uses complex logarithms where needed. `⍟0` is `¯∞`. `1⍟1` gives zero
when its floating calculation has a [Mathish
type](../terms.qmd#term-mathish-types) and [Mathish
mode](../numbers.qmd#mathish-mode) is enabled. Other base-1 cases give
an infinity or NaN.

## Inverse

`⍟⁻¹` is `*`. With the right argument fixed, `(⍟↢X)⁻¹Y` is `X*÷Y`.

``` bpl
2⍟⁻¹3      ⍝ 8
(⍟↢8)⁻¹3   ⍝ 2
```

## Fast combinations

BPL computes this combination by an algorithm of its own, in any
spelling. [Trains](../evaluation.qmd#trains) says which spellings BPL
recognises.

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
<td><code>⍟1+Y</code>, <code>⍟Y+1</code> or <code>⍟∘(1+)</code></td>
<td>real <code>Y</code></td>
<td>log1p keeps the digits of a small <code>Y</code> that
<code>1+Y</code> rounds away</td>
</tr>
</tbody>
</table>

At an item below ¯1 under dynamic kinds, and on complex numbers, `⍟1+Y`
computes `⍟` of `1+Y`. Softplus, `⍟1+*Y`, stays accurate for a very
negative `Y`:

``` bpl
⍟1+1E¯17          ⍝ 1ₑ¯17
⍟1+*¯40           ⍝ 4.248354255ₑ¯18
```
