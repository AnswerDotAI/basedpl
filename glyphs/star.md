

# `*` — Exponential / Exponent

Function. Hanzi: `幂` (`冪`). Ranks: `0` monadic, `0 0` dyadic

`*Y`: e<sup>Y</sup>. Pervasive.

``` bpl
*0                ⍝ 1
```

`X*Y`: X<sup>Y</sup>. Whole powers of exact operands stay exact.

``` bpl
2*3               ⍝ 8
2ₓ*¯3ₓ            ⍝ 1r8
```

Other powers use principal complex branches where needed.

``` bpl
¯1*0.5            ⍝ 0j1
```

See also function [power `⍣`](star-diaeresis.qmd).

## Inverse

`*⁻¹` is `⍟`. With the right argument fixed, `(*↢X)⁻¹Y` is `Y*÷X`.

``` bpl
2*⁻¹8      ⍝ 3
(*↢2)⁻¹9   ⍝ 3
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
<td><code>¯1+*Y</code> or <code>(*Y)-1</code></td>
<td>real <code>Y</code></td>
<td>expm1 keeps the digits of a small <code>Y</code> that subtracting 1
from <code>*Y</code> cancels</td>
</tr>
</tbody>
</table>

On complex numbers, `¯1+*Y` computes `*Y`, then subtracts 1.

``` bpl
¯1+*1E¯17         ⍝ 1ₑ¯17
```
