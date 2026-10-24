

# `>` — Sort down / Greater

Function. Hanzi: `大`. Ranks: `∞` monadic, `0 0` dyadic

`>Y` sorts the major cells of `Y` descending, as `[⍒Y]⌷Y` does. Ties
keep their order.

``` bpl
>30 10 20           ⍝ 30 20 10
>"b" "a" "c"        ⍝ "c" "b" "a"
```

`X>Y` tests X \> Y for reals, and for characters by code point. A number
is less than any character, as [`⍋`](delta-stile.qmd) orders them.
Pervasive; tolerant.

``` bpl
1 2 3>2            ⍝ $f $f $t
"abd">"abc"        ⍝ $f $f $t
'a'>3              ⍝ $t
```

See [`<`](less-than.qmd) to sort up, and [`⍒`](del-stile.qmd) for the
indices that sort.

## Fast combinations

These trains and compositions give the result of applying their parts in
turn, faster. [Trains](../evaluation.qmd#trains) says which spellings
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
<td><code>k↑&gt; Y</code> or <code>(k↑)∘&gt;</code></td>
<td>a vector of reals or characters without keys, with the count at most
a quarter of its length</td>
<td>selects the <code>k</code> largest items, or the <code>-k</code>
smallest, without ordering the rest</td>
</tr>
<tr>
<td><code>↑&gt; Y</code>, <code>i⌷&gt; Y</code> or
<code>i⊃&gt; Y</code></td>
<td>the same vectors</td>
<td>selects the item of rank <code>i</code> from the top without
ordering the rest</td>
</tr>
</tbody>
</table>

``` bpl
v←30 10 20 50 40 60 70 80 90 0
2↑> v                               ⍝ 90 80
↑> v                                ⍝ 90
(•explain "↑> v").fast.combination  ⍝ ["partial sort"]
```
