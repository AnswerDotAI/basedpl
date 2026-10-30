

# `<` — Sort up / Less

Function. Hanzi: `小`. Ranks: `∞` monadic, `0 0` dyadic

`<Y` sorts the major cells of `Y` ascending, as `[⍋Y]⌷Y` does. Ties keep
their order.

``` bpl
<30 10 20           ⍝ 10 20 30
<"banana"           ⍝ "aaabnn"
<[3 1⋄1 2⋄2 0]  ⍝ [1 2⋄2 0⋄3 1]
```

`X<Y` tests X \< Y for reals, and for characters by code point. A number
is less than any character, as [`⍋`](delta-stile.qmd) orders them.
Pervasive; tolerant.

``` bpl
1 2 3<2            ⍝ $t $f $f
"abc"<"abd"        ⍝ $f $f $t
3<'a'              ⍝ $t
```

See [`>`](greater-than.qmd) to sort down, and [`⍋`](delta-stile.qmd) for
the indices that sort.

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
<td><code>k↑&lt;Y</code>, <code>k↑&lt; Y</code> or
<code>(k↑)∘&lt;</code></td>
<td>a vector of reals or characters without keys, with the count at most
a quarter of its length</td>
<td>selects the <code>k</code> smallest items, or the <code>-k</code>
largest, without ordering the rest</td>
</tr>
<tr>
<td><code>↑&lt;Y</code>, <code>i⌷&lt;Y</code> or
<code>i⊃&lt;Y</code></td>
<td>the same vectors</td>
<td>selects the item of rank <code>i</code>, such as a median, without
ordering the rest</td>
</tr>
</tbody>
</table>

``` bpl
v←30 10 20 50 40 60 70 80 90 0
2↑< v                                ⍝ 0 10
5⌷< v                                ⍝ 50
(•explain "5⌷< v").fast.combination  ⍝ ["partial sort"]
```
