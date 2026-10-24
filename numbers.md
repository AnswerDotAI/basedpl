

# Numbers

A bare number such as `0.5` is approximate: a float of the default
width, 64 bits unless [`•prefs`](system/prefs.qmd) sets 16 or 32. A
number marked with `ₓ`, such as `3ₓ`, is an ordinary integer, exact
under the default dynamic kinds. `1r3` is an exact rational. A
byte-count suffix such as `ₓ₄` selects a fixed-width integer.
Comparisons give Booleans, written `$t` and `$f`.

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
is part of the number, and <a href="glyphs/minus.qmd"><code>-</code></a>
is a function</td>
</tr>
<tr>
<td><code>3ₓ</code></td>
<td>An exact integer</td>
</tr>
<tr>
<td><code>3ₓ₁</code>, <code>3ₓ₂</code>, <code>3ₓ₄</code>,
<code>3ₓ₈</code></td>
<td>A fixed-width integer occupying 1, 2, 4 or 8 bytes</td>
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
<td><code>1.5ₕ</code>, <code>1.5ₛ</code>, <code>1.5ₚ</code></td>
<td>A 16-bit, 32-bit or 64-bit float, whatever the default width</td>
</tr>
<tr>
<td><code>2j3ₛ</code>, <code>2j3ₚ</code></td>
<td>A complex number with 32-bit or 64-bit parts. <code>2j3ₕ</code> has
32-bit parts too</td>
</tr>
<tr>
<td><code>⍬</code>, <code>⍬ₓ</code>, <code>⍬ₕ</code>, <code>⍬ₛ</code>,
<code>⍬ₚ</code></td>
<td>The approximate, the exact, and the 16-bit, 32-bit and 64-bit <a
href="glyphs/zilde.qmd">empty vector</a></td>
</tr>
<tr>
<td><code>[…]ₓ</code>, <code>(…)ₓ</code></td>
<td>A list, or rows, in which every number is exact</td>
</tr>
<tr>
<td><code>[…]ₕ</code>, <code>[…]ₛ</code>, <code>[…]ₚ</code></td>
<td>A list, or rows, in which every real number is a 16-bit, 32-bit or
64-bit float, and every complex number has parts of that width, or
32-bit parts after <code>ₕ</code></td>
</tr>
<tr>
<td><code>xⱼy</code>, <code>xₑy</code>, <code>xᵣy</code></td>
<td>Between two items, such as names: <code>x+y×0ⱼ1</code>,
<code>x×10*y</code> and the exact quotient of <code>xₓ</code> by
<code>yₓ</code>, as one term</td>
</tr>
</tbody>
</table>

A plain `e`, `j` or `r` starts a name unless the rest of a number
follows it, as in `1e5`, `1r3` and `2j3`. Results display with the
subscript forms, as in `1ᵣ3`, `2ⱼ3` and `1ₑ20`. A subscript form that
the rest of a number doesn’t follow, or one between two other items such
as names, joins the items on each side into one term, as a literal is
one, with the value the table gives. A literal on the left gives what
the all-literal form gives: with `n←3`, `2ₑn` is `2ₑ3`.

``` bpl
1j2+3j4              ⍝ 4j6
1E¯3                 ⍝ 0.001
¯2+5                 ⍝ 3
2J3≡2j3              ⍝ $t
1ᵣ3≡1r3              ⍝ $t
edges←3 ⋄ 2edges     ⍝ 2 3
x←2 ⋄ n←3 ⋄ xⱼn      ⍝ 2j3
2ₑn                  ⍝ 2000
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

A function keeps exact integers exact only if every finite result it
gives for them is rational. Arithmetic, `!`, and `*` with an integer
exponent keep them exact. `√`, `⍟`, `π`, `○` and monadic `*` give
approximate results, whole ones included. An exact function gives the
float `∞` at a pole, such as `÷0ₓ`:

``` bpl
2ₓ*¯2ₓ               ⍝ 1r4
!20ₓ                 ⍝ 2432902008176640000ₓ
√4ₓ                  ⍝ 2
÷0ₓ                  ⍝ ∞
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
float within `1E¯14` of one, whatever the [`mathish`
setting](system/prefs.qmd):

``` bpl
⍳10×0.1×3          ⍝ 0 1 2
```

## Numbers in one array

Numbers that an operation puts in one array share a kind. A float among
exact integers makes them all floats, and a complex number makes every
number complex:

``` bpl
⍳3ₓ , 0.5            ⍝ 0 1 2 0.5
⍳2ₓ , 1j2            ⍝ 0 1 1j2
1ₓ,0.5 ÷ 2ₓ          ⍝ 0.5 0.25
```

Each number written in a literal list or in brackets keeps its own
exactness, even beside numbers of the other kind. Floats written beside
complex numbers become complex. Their values don’t change:

``` bpl
1ₓ 0.5÷2ₓ            ⍝ 1r2 0.25
[1ₓ 0.5]÷2ₓ          ⍝ 1r2 0.25
```

A write keeps each number’s exactness too. A float assigned among exact
integers, or an exact integer among floats, leaves the array mixed,
however the array was made. A Boolean written among other numbers, or a
number written among Booleans, leaves it mixed in the same way. Mixed
storage is slower. To keep an array compact, write values of its own
kind, as the [style guide](style.qmd) shows.
[`•explain`](system/explain.qmd) lists each write that left an array
mixed, in its `mixed` record. To give an existing array one kind,
convert it: `⌊` makes exact integers, `×1` makes floats, and `+0ₓ`
compacts items that already share a kind:

``` bpl
r←["a":2ₓ "b":3ₓ]
r.b←2.5
r                    ⍝ ["a":2ₓ "b":2.5]
m←3⍴$f
m.[[1]]←1
m                    ⍝ $f 1 $f
```

Around literals, `[…]ₓ` and `(…)ₓ` make every number exact, at any
depth. Each number must be whole. After a list or group that holds
anything else, as after a name, `ₓ` floors to exact integers, as
[Superscripts and subscripts](scripts.ipynb) describes. A vector of
exact integers displays as `[…]ₓ`, and rows of them as `(…)ₓ`. `⍕`
writes them the same way:

``` bpl
[1 0 1]ₓ≡1ₓ 0ₓ 1ₓ    ⍝ $t
[[1 2] 3]ₓ≡[1ₓ 2ₓ;3ₓ]   ⍝ $t
(0 1⋄2 3)ₓ≡[[0 1]ₓ [2 3]ₓ]  ⍝ $t
1↑⍬ₓ                 ⍝ [0]ₓ
⍕1ₓ ¯2ₓ              ⍝ "[1 ¯2]ₓ"
```

## Weak and strong numbers

An ordinary integer literal such as `1ₓ`, or an unmarked float or
complex literal, is weak. Python `int`, `float` and `complex` arguments
are weak too. Arithmetic on weak values gives weak ordinary results. A
strong operand makes the ordinary result strong.

An explicit float-width literal or a fixed-width integer is strong. So
is a number read by `•json` or `•csv`. Constructing an array makes its
items strong. Selecting an item preserves that strength.

## Storage

`(•storage x).kind` names an array’s storage kind. Ordinary integers
report `integer` regardless of their byte width. Fixed-width integers
report their type, such as `uint8` for `ₓ₁`. Mixed storage keeps each
item’s own kind. A literal list of exact and approximate numbers uses
it:

``` bpl
x←1ₓ 0.5 2ₓ
(•storage x).kind      ⍝ "mixed"
```

Rationals and exact integers beyond 64 bits have no compact storage. An
array that holds one uses mixed storage, as does an array that holds
numbers together with characters, nested arrays or functions. A JSON
object or a Python dict reads as a literal of its fields does, so its
storage is mixed only where the fields need it.

Selecting from, catenating or assigning into mixed storage keeps each
item’s kind. A selection or catenation whose items one compact storage
holds uses it, as the same items written in brackets do: `1↓'a' 1ₓ 2ₓ`
is held as integers. An assignment into mixed storage leaves it mixed.
Arithmetic builds fresh storage from its results, and each result keeps
its own kind. Every result of `1×x` is approximate, which makes it a
float array. `1ₓ×x` keeps the exact results exact:

``` bpl
x←1ₓ 0.5 2ₓ
(•storage 1×x).kind    ⍝ "float64"
(•storage 1ₓ×x).kind   ⍝ "mixed"
```

Display depends on an array’s items, not on how they’re stored. Boxed
display marks an array whose items differ in kind with `+`, and one
whose items are all exact integers with `ₓ`, whichever storage holds
them.

Ordinary integer storage keeps each item in 1, 2, 4 or 8 bytes. One byte
holds 0 to 255. Literals, reductions and positions take the fewest bytes
that hold every item. Pervasive arithmetic keeps its arguments’ size,
and moves to the next size up when a result doesn’t fit. An exact result
beyond 8 bytes becomes a big integer in mixed storage. The size never
changes an ordinary result, and `•storage` doesn’t show it. Imported
typed integer buffers use the fixed-width rules below.

## Fixed-width integers

The suffix gives a number of bytes. `ₓ₁` selects unsigned `u8`; `ₓ₂`,
`ₓ₄` and `ₓ₈` select signed `i16`, `i32` and `i64`. The width stays with
a value through selection, rebuilding and arithmetic. Integer results
wrap at their selected width rather than growing. The `int` preference
does not override an explicit fixed width.

``` bpl
255ₓ₁+1ₓ            ⍝ 0ₓ₁
255ₓ₁+↑[1]ₓ         ⍝ 256ₓ
32767ₓ₂+1ₓ          ⍝ ¯32768ₓ₂
[200 100]ₓ₁<300ₓ     ⍝ $t $t
```

Two fixed integers compute at the wider type. In arithmetic, a [weak
ordinary integer](#weak-and-strong-numbers) takes the fixed type if it
fits, and raises otherwise. A strong ordinary integer makes the result
ordinary, under the current `kinds` setting. Booleans take the fixed
type too. Floats follow their existing promotion rules. True division
gives 32-bit floats for `u8` and `i16`, and 64-bit floats for `i32` and
`i64`, whatever the `float` preference. Comparisons give Booleans
without requiring a weak argument to fit the fixed type.

Fixed integer literals must fit their type. After a value, a sized
suffix converts to that type. Integer narrowing wraps. Float and
rational conversion floors, then clamps to the target range. NaN
converts to zero; infinities convert to the range’s bounds. Typed empty
vectors use the same suffix, as in `⍬ₓ₁`.

Plain `ₓ` removes a fixed integer’s width rule without copying array
data. The ordinary result is strong. Its arithmetic then follows
ordinary integer rules: exact under dynamic kinds, and the existing
computation width and wrapping under static kinds.

Reductions retain the promoted fixed type. A suffix on a function
converts its result after computation. To widen the accumulator itself,
use a [typed reduction seed](glyphs/slash.qmd). [Superscripts and
subscripts](scripts.ipynb#subscripts) covers the suffix syntax.

## Float widths

Floats come in three widths: 64, 32 and 16 bits. `•storage` gives their
kind as `"float64"`, `"float32"` and `"float16"`.
[`•float`](system/float.qmd) converts an array to any of them. A 64-bit
float holds about 16 significant digits, a 32-bit float about 7 and a
16-bit float about 3. They take 8, 4 and 2 bytes. The largest finite
16-bit float is 65504.

Bare numbers are floats of the default width: 64 bits, or 16 or 32 after
[`•prefs`](system/prefs.qmd) `["float":16]` or `["float":32]`. Code
parsed after the change reads its numbers at the new width. In a REPL or
a notebook, that is the next input. Leading `•prefs` calls on a literal
record run before the rest of their code is parsed. A script, a notebook
cell, a module or `⍎` text that starts with `•prefs ["float":32]`
therefore reads its own numbers at 32 bits. Numbers that `•csv`, `•json`
and `•vfi` read also take the default width. So do random numbers, the
numbers that `•image` reads, and floats that a function gives from exact
arguments, such as `√2ₓ`. Times from `•time` and `•date` are always
64-bit, because a 32-bit float holds seconds since 1970 only to within
about two minutes. `ₕ` marks a 16-bit float, `ₛ` a 32-bit float and `ₚ`
a 64-bit float, whatever the default. Display marks floats of the other
widths, once after an array’s notation. `•literal⁻¹` marks every float.
Its text therefore reads back the same at any default.

``` bpl
{(•storage ⍵).kind}¨[1.5ₕ 1.5ₛ [1.5 2.5]ₚ ⍬ₛ]   ⍝ "float16" "float32" "float64" "float32"
[1.5 2.5]ₛ                          ⍝ [1.5 2.5]ₛ
•literal⁻¹ 0.5                     ⍝ "0.5ₚ"
```

An operation on floats computes at their width. An exact operand rounds
to the width that the operation computes at. Floats of two widths
compute at the wider, except that a [weak
number](#weak-and-strong-numbers) takes the width of the floats beside
it. Writes, catenation and arrays built from items follow the same rule:
`v,2.5`, `v.[0]←2.5` and `[v₀ 2.5]` keep a 32-bit `v`’s width. No
operation widens floats on its own.

``` bpl
v←32 •float 1.5 2.5
{(•storage ⍵).kind}¨[v×2ₓ v×v *v v×2 v×2ₚ v×v₀ v,2.5 [v₀ 2.5]]   ⍝ "float32" "float32" "float32" "float32" "float64" "float32" "float32" "float32"
•float v                     ⍝ 32ₓ
(32 •float 1.5)×16777217ₓ    ⍝ 25165824
```

Complex numbers have 64-bit or 32-bit parts. `•storage` gives their kind
as `"complex128"` and `"complex64"`, after their whole size. They follow
the rules for floats. A complex number takes the width of the floats it
comes from, and a 16-bit float that becomes complex takes 32-bit parts.
A complex number beside a float computes at the wider of their widths. A
complex result with no imaginary part is a float of its parts’ width.
`ₛ`, `ₚ` and `ₕ` mark the width of the parts, and display marks parts
that aren’t of the default width:

``` bpl
{(•storage ⍵).kind}¨[[2j3]ₛ;[2j3]ₕ;√[¯4]ₛ;[2j3]ₛ×1.5]   ⍝ "complex64" "complex64" "complex64" "complex64"
√¯4ₛ                                       ⍝ 0j2ₛ
```

Each operation on 16-bit floats computes in 32 bits and rounds its
result to 16 bits. For `+ - × ÷` and `√`, that gives the result of
16-bit arithmetic. Folds, scans and inner products keep their running
totals in 32 bits. Only their results round to 16 bits. A 16-bit total
of these numbers would stop growing at 256:

``` bpl
+/(16 •float 4096⍴0.1)   ⍝ 409.5ₕ
```

An exact operand converts straight to 32 bits, not to 16. A count larger
than 65,504, the largest 16-bit float, still divides a 16-bit sum:

``` bpl
h←16 •float 100000⍴1 0
+/÷≢ h                   ⍝ 0.49975586ₕ
```

[Power](glyphs/star-diaeresis.qmd) `⍣∞` stops at two results within
relative tolerance `1E¯14` of each other, or within 4 units in the last
place for 16-bit and 32-bit results. See [Mathish mode](#mathish-mode)
for comparison and division rules.

## Mathish mode

[Mathish mode](mathish.qmd) is on by default. For [Mathish
types](terms.qmd#term-mathish-types), floating comparisons use relative
tolerance `1E¯14`, measured against the larger magnitude, and `0÷0` and
`1⍟1` give zero. Disable it with `•prefs ["mathish":$f]` for exact
floating comparisons and NaN at floating zero divided by zero.

The calculation’s kind and `kinds` setting decide whether these
conventions apply. Explicit 64-bit float suffixes and imported buffers
follow the same rule as literals. Static calculations and fixed-integer
operations never use Mathish conventions, even when their floating
computation uses 64 bits. The narrower floats keep exact comparisons and
raw division. Acceptance of integer and Boolean arguments and
convergence stopping rules are independent of Mathish mode.

## Infinities and NaN

Floats include `∞`, `¯∞`, NaN and `¯0` besides finite numbers. An
undefined result is NaN rather than an error, except for Mathish mode’s
zero divided by zero. `1÷0` is `∞`, and `⍟0` is `¯∞`. NaN is written
`$n`:

``` bpl
[0÷0 ∞-∞]            ⍝ 0 $n
[1÷0 ⍟0]             ⍝ ∞ ¯∞
```

Under `=`, NaN equals nothing, as in IEEE. `≡`, search, grade and Key
treat NaN as one value, and grade puts it after every other number:

``` bpl
$n=$n                ⍝ $f
$n≡$n                ⍝ $t
```

Complex arithmetic follows the `num_complex` crate. A complex number can
have infinite or NaN parts.

An infinity or NaN is a float. Joining one to exact integers makes them
floats, as any float does:

``` bpl
⍳3ₓ , ∞              ⍝ 0 1 2 ∞
(•storage ⍳3ₓ , ∞).kind   ⍝ "float64"
```

CSV and JSON imports give NaN for a missing number. An integer column
with gaps then becomes floats. A column that also holds an integer too
large for a float to hold exactly keeps each item’s kind, in mixed
storage.

`⌊` and `⌈` return an exact argument unchanged beside an infinity:

``` bpl
3ₓ⌊∞                 ⍝ 3ₓ
```

The [`⌈`](glyphs/upstile.qmd) and [`⌊`](glyphs/downstile.qmd) pages give
their results beside NaN. Arithmetic with an infinity gives floats.

## Static kinds

Some functions choose a result’s kind from their arguments’ values.
`√¯4` is complex because the argument is negative, `1j0` is a float
because its imaginary part is zero, and an exact integer that overflows
grows. Under [`•prefs`](system/prefs.qmd) `["kinds":"static"]`, a
result’s kind follows from the function and its arguments’ kinds alone.
A fused computation needs that, because it decides each result’s kind
before it runs. Real arguments give real results, with NaN outside a
function’s domain, and complex results stay complex. `⌊`, `⌈` and
monadic `×` of floats give floats, and integer `÷` gives floats. With
`ₓ` after them they give integers, as Rust’s `as` converts a float: NaN
gives 0, and a value past the `int` width’s range gives its largest or
smallest integer:

``` bpl
•prefs ["kinds":"static"]
√¯4 9                ⍝ $n 3
0j1×0j1              ⍝ ¯1ⱼ0
⌊2.5 ∞               ⍝ 2 ∞
7ₓ÷2ₓ                ⍝ 3.5
⌊ₓ2.5 ¯2.5           ⍝ [2 ¯3]ₓ
7ₓ÷ₓ2ₓ               ⍝ 3ₓ
⌊ₓ $n ∞              ⍝ [0 9223372036854775807]ₓ
0÷0                  ⍝ $n
0.3=0.1+0.2          ⍝ $f
•prefs ["kinds":"dynamic"]
```

Ordinary integers compute at the width that the `int` pref sets, 64 bits
by default, or at a wider integer’s width, and wrap where a result
doesn’t fit. Fixed integers retain their explicit type. An integer
beside a float takes the float’s width. A weak number beside floats
takes their width, as under dynamic kinds. Exact fractions stay exact.
Floating comparisons are exact. Under `•prefs ["int":32]`, an ordinary
integer literal that doesn’t fit 32 bits is a LIMIT error:

``` bpl
•prefs ["kinds":"static"]
9223372036854775807ₓ+1ₓ    ⍝ ¯9223372036854775808ₓ
(•storage (32 •float 1 2)×2ₚ).kind    ⍝ "float64"
(•storage (32 •float 1 2)×2ₓ).kind    ⍝ "float32"
•prefs ["kinds":"dynamic"]
```

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
x>2 # x              ⍝ 3 4 5
4>2 ⌷ "no" "yes"     ⍝ "yes"
```

A float used as a Boolean follows the [integer-acceptance
rule](#exact-and-approximate) for 0 or 1.

Arithmetic on Booleans gives integers. `∧`, `∨`, `⊽`, `⊼`, `⌊` and `⌈`
on two Booleans give a Boolean:

``` bpl
$t+$t                ⍝ 2ₓ
$t∧$f                ⍝ $f
```

Boolean storage takes one byte for each item. Boxed display marks it
with `$`:

``` bpl
(•storage $t $f).kind   ⍝ "bool"
```
