

# Numbers

BPL has two kinds of number. A bare number such as `0.5` is approximate:
a 64-bit float. A number marked with `ₓ`, such as `3ₓ`, is an exact
integer, and `1r3` is an exact rational. Comparisons give Booleans,
written `$t` and `$f`.

## Writing numbers

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Written</th>
<th>Value</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>3</code>, <code>0.5</code>, <code>1E¯3</code>,
<code>1ₑ¯3</code></td>
<td>Approximate numbers. <code>e</code>, <code>E</code> or
<code>ₑ</code> writes an exponent</td>
</tr>
<tr>
<td><code>¯2</code></td>
<td>A negative number. <a href="glyphs/overbar.qmd"><code>¯</code></a>
is part of the number, and <a href="glyphs/dash.qmd"><code>-</code></a>
is a function</td>
</tr>
<tr>
<td><code>3ₓ</code></td>
<td>An exact integer</td>
</tr>
<tr>
<td><code>1r3</code>, <code>1ᵣ3</code></td>
<td>An exact rational</td>
</tr>
<tr>
<td><code>2j3</code>, <code>2J3</code>, <code>2ⱼ3</code></td>
<td>A complex number, 2 + 3i, with approximate parts</td>
</tr>
<tr>
<td><code>∞</code>, <code>¯∞</code></td>
<td><a href="glyphs/infinity.qmd">Infinities</a></td>
</tr>
<tr>
<td><code>$n</code></td>
<td>NaN</td>
</tr>
<tr>
<td><code>$t</code>, <code>$f</code></td>
<td>True and false</td>
</tr>
<tr>
<td><code>⍬</code>, <code>⍬ₓ</code></td>
<td>The approximate and the exact <a href="glyphs/zilde.qmd">empty
vector</a></td>
</tr>
<tr>
<td><code>[…]ₓ</code>, <code>(…)ₓ</code></td>
<td>A list, or rows, in which every number is exact</td>
</tr>
</tbody>
</table>

A plain `e`, `j` or `r` starts a name unless the rest of a number
follows it, as in `1e5`, `1r3` and `2j3`. The subscript forms are always
part of the number, and results display with them, as in `1ᵣ3`, `2ⱼ3`
and `1ₑ20`.

``` bpl
1j2+3j4              ⍝ 4j6
1E¯3                 ⍝ 0.001
¯2+5                 ⍝ 3
2J3≡2j3              ⍝ $t
1ᵣ3≡1r3              ⍝ $t
edges←3 ⋄ 2edges     ⍝ 2 3
```

## Exact and approximate

Exact arithmetic grows as needed. Mixing exact and approximate numbers
gives an approximate result. Examples on these pages write `ₓ` for exact
input.

``` bpl
1÷3                  ⍝ 0.3333333333333333
1ₓ÷3ₓ                ⍝ 1r3
1r3+1r6              ⍝ 1r2
1r2+0.5              ⍝ 1
9223372036854775807ₓ+1ₓ ⍝ 9223372036854775808ₓ
```

Predicates, positions, tally, shape, and monadic `⌊`, `⌈` and `×` return
exact integers. A float beyond the `i64` range floors to an exact big
integer:

``` bpl
⌊2.5 ¯2.5            ⍝ [2 ¯3]ₓ
⌊2*70                ⍝ 1180591620717411303424ₓ
```

Iota and random integer generation keep an exact argument exact. An
argument that must be an integer, such as an index or a count, accepts a
float within comparison tolerance of one:

``` bpl
⍳(0.1×3)×10          ⍝ 0 1 2
```

## Numbers in one array

Numbers that an operation puts in one array share a kind. A float among
exact integers makes them all floats, and a complex number makes every
number complex:

``` bpl
(⍳3ₓ),0.5            ⍝ 0 1 2 0.5
(⍳2ₓ),1j2            ⍝ 0 1 1j2
(1ₓ,0.5)÷2ₓ          ⍝ 0.5 0.25
```

Each number written in a literal list or in brackets keeps its own
exactness, even beside numbers of the other kind. Floats written beside
complex numbers become complex. Their values don’t change:

``` bpl
1ₓ 0.5÷2ₓ            ⍝ 1r2 0.25
[1ₓ;0.5]÷2ₓ          ⍝ 1r2 0.25
```

In `[…]ₓ` and `(…)ₓ`, every number is exact, at any depth. Each item
must be a literal, and each number must be whole. A vector of exact
integers displays as `[…]ₓ`, and rows of them as `(…)ₓ`. `⍕` writes them
the same way:

``` bpl
[1 0 1]ₓ≡1ₓ 0ₓ 1ₓ    ⍝ $t
[[1 2] 3]ₓ≡[1ₓ 2ₓ;3ₓ]   ⍝ $t
(0 1 ⋄ 2 3)ₓ≡[[0 1]ₓ [2 3]ₓ]  ⍝ $t
1↑⍬ₓ                 ⍝ [0]ₓ
⍕1ₓ ¯2ₓ              ⍝ "[1 ¯2]ₓ"
```

## Storage

An array keeps its items in one of six storages: `boolean`, `integer`,
`float`, `complex`, `character` or `mixed`. `•storage` names it. Mixed
storage keeps each item’s own kind. A literal list of exact and
approximate numbers uses it:

``` bpl
x←1ₓ 0.5 2ₓ
•storage x           ⍝ "mixed"
```

An array also uses mixed storage when it holds a rational, because
rationals have no compact storage, or when it holds numbers together
with characters, nested arrays or functions. An imported JSON object is
mixed too.

Selecting from, catenating or assigning into mixed storage keeps each
item’s kind. Arithmetic builds fresh storage from its results, and each
result keeps its own kind. Every result of `1×x` is approximate, which
makes it a float array. `1ₓ×x` keeps the exact results exact:

``` bpl
x←1ₓ 0.5 2ₓ
•storage 1×x         ⍝ "float"
•storage 1ₓ×x        ⍝ "mixed"
```

Boxed display marks mixed storage with `+`, and exact integers with `ₓ`.

## Infinities and NaN

Floats follow IEEE 754. Besides finite numbers, they include `∞`, `¯∞`,
NaN and `¯0`. An undefined result is NaN rather than an error. `1÷0` is
`∞`, and `⍟0` is `¯∞`. NaN is written `$n`:

``` bpl
[0÷0;∞-∞]            ⍝ $n $n
[1÷0;⍟0]             ⍝ ∞ ¯∞
```

Under `=`, NaN equals nothing, as in IEEE. `≡`, search, grade and Key
treat NaN as one value, and grade puts it after every other number:

``` bpl
$n=$n                ⍝ $f
$n≡$n                ⍝ $t
```

Complex arithmetic follows the `num_complex` crate. A complex number can
have infinite or NaN parts.

Joining infinities or NaN to exact integers keeps the integers exact.
Exact integers, infinities and NaN share integer storage:

``` bpl
(⍳3ₓ),∞              ⍝ [0 1 2 ∞]ₓ
•storage (⍳3ₓ),∞     ⍝ "integer"
```

CSV and JSON imports give NaN for a missing number. An integer column
with gaps then stays exact.

`⌊` and `⌈` return an exact argument unchanged beside an infinity, and
return the other argument beside NaN:

``` bpl
3ₓ⌊∞                 ⍝ 3ₓ
3ₓ⌊$n                ⍝ 3ₓ
```

Arithmetic with an infinity gives floats.

## Booleans

Comparisons give Booleans. So does every function whose result is a
truth value, such as `~`, `∊`, `⍷` and `≡`. `$t` is true and `$f` is
false:

``` bpl
x←3 1 4 1 5
x>2                  ⍝ $t $f $t $f $t
```

Wherever a number is expected, `$t` is 1 and `$f` is 0. `+/` counts the
true items, `#` keeps the items a Boolean mask marks, and a Boolean used
as an index selects position 0 or 1:

``` bpl
x←3 1 4 1 5
+/x>2                ⍝ 3ₓ
(x>2)#x              ⍝ 3 4 5
(4>2)⌷"no" "yes"     ⍝ "yes"
```

Arithmetic on Booleans gives integers. `∧`, `∨`, `⍱`, `⍲`, `⌊` and `⌈`
on two Booleans give a Boolean:

``` bpl
$t+$t                ⍝ 2ₓ
$t∧$f                ⍝ $f
```

Boolean storage takes one byte for each item. Boxed display marks it
with `$`:

``` bpl
•storage $t $f       ⍝ "boolean"
```
