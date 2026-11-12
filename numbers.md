

# Numbers

A bare number such as `0.5` is approximate: a float of the default
width: 64 bits under dynamic kinds and 32 under static kinds, unless
[`•prefs`](system/prefs.qmd) sets a width. A number marked with `ₓ`,
such as `3ₓ`, is an ordinary machine integer. `3ᵣ` is an unbounded
integer and `1r3` an exact rational in that same domain. A byte-count
suffix such as `ₓ₄` selects a fixed-width integer. Comparisons give
Booleans, written `$t` and `$f`.

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
<td>An ordinary machine integer</td>
</tr>
<tr>
<td><code>3ₓ₁</code>, <code>3ₓ₂</code>, <code>3ₓ₄</code>,
<code>3ₓ₈</code></td>
<td>A fixed-width integer occupying 1, 2, 4 or 8 bytes</td>
</tr>
<tr>
<td><code>3ᵣ</code>, <code>1r3</code>, <code>1ᵣ3</code></td>
<td>An unbounded integer or exact rational</td>
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
<td>A list, or rows, in which every number is a machine integer</td>
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
<code>x×10*y</code> and a rational from integer components, as one
term</td>
</tr>
</tbody>
</table>

A plain `e`, `j` or `r` starts a name unless the rest of a number
follows it, as in `1e5`, `1r3` and `2j3`. Results display with the
subscript forms, as in `1ᵣ3`, `2ⱼ3` and `1ₑ20`. Between two items, `ⱼ`,
`ₑ` and `ᵣ` join them into one term with the value the table gives. A
trailing `ᵣ` instead converts its left item to the unbounded domain. A
literal on the left gives what the all-literal form gives: with `n←3`,
`2ₑn` is `2ₑ3`.

``` bpl
1j2+3j4              ⍝ 4j6
1E¯3                 ⍝ 0.001
¯2+5                 ⍝ 3
2J3≡2j3              ⍝ $t
1ᵣ3≡1r3              ⍝ $t
edges←3 ◇ 2edges     ⍝ 2 3
x←2 ◇ n←3 ◇ xⱼn      ⍝ 2j3
2ₑn                  ⍝ 2000
```

## Exact and approximate

Ordinary `ₓ` integers use machine arithmetic, like integer types in C,
Rust and Swift. They are not floats that happen to hold whole values.
Integer `÷` returns an integer. BPL floors negative quotients, whereas
those languages truncate towards zero:

``` bpl
1÷3                    ⍝ 0.3333333333333333
1ₓ÷3ₓ                  ⍝ 0ₓ
¯3ₓ÷2ₓ                 ⍝ ¯2ₓ
1ₓ÷ₐ3ₓ                 ⍝ 0.3333333333333333
```

Dynamic integer arithmetic computes at the narrowest width that holds
every result its arguments’ widths can give, and wraps at the 64-bit
limit. Explicit fixed-width integers wrap at their declared width. A
machine literal outside the supported range raises LIMIT ERROR. A count
from `≢` or `ρ` past 64 bits gives the largest 64-bit integer.
Arithmetic overflow wraps, including the minimum signed integer divided
by minus one. Neither overflow nor division automatically enters
unbounded arithmetic:

``` bpl
9223372036854775807ₓ+1ₓ ⍝ ¯9223372036854775808ₓ
```

Use `ᵣ` for unbounded integers and rationals. `3ᵣ` means `3ᵣ1`, and `vᵣ`
converts a value into this domain. A whole result stays in that domain
and displays with a trailing `ᵣ`. Mixing a machine integer with a `ᵣ`
number keeps the unbounded domain. A trailing `ᵣ` keeps a finite real
float’s stored binary value exactly: `0.5ᵣ` is `1ᵣ2`, but `0.1ᵣ` is not
`1ᵣ10`. Write `1ᵣ10` for that exact decimal fraction. The two-item
joiner `xᵣy` floors its components. Mixing with a float follows the
usual approximate promotion:

``` bpl
1ᵣ÷3ᵣ                  ⍝ 1ᵣ3
1ₓ÷ᵣ3ₓ                 ⍝ 1ᵣ3
1ᵣ3+1ᵣ6                ⍝ 1ᵣ2
3ᵣ÷1ᵣ                  ⍝ 3ᵣ
9223372036854775807ᵣ+1ₓ ⍝ 9223372036854775808ᵣ
1ᵣ2+0.5                ⍝ 1
```

Select the domain before computing. `÷ₐ` selects float division, and
`÷ᵣ` selects rational division. Converting a floored or wrapped result
afterwards cannot recover the lost value. The mean train `+/÷≢` follows
its input domain:

``` bpl
+/÷≢ 1ₓ 2ₓ             ⍝ 1ₓ
+/÷≢ 1 2               ⍝ 1.5
+/÷≢ 1ᵣ 2ᵣ             ⍝ 3ᵣ2
```

An exact function keeps unbounded arguments exact where its finite
result is rational. `√`, `⊛`, `π`, `○` and monadic `*` give approximate
results, whole ones included:

``` bpl
2ᵣ*¯2ₓ                 ⍝ 1ᵣ4
!20ᵣ                   ⍝ 2432902008176640000ᵣ
√4ₓ                    ⍝ 2
```

Matrix division and matrix inverses use floating arithmetic for
machine-integer inputs. Use `ᵣ` inputs for exact rational arithmetic.
Matrix products, determinants and permanents keep integer arithmetic.

Predicates give Booleans. Positions, tally, shape, and monadic `⌊`, `⌈`
and `×` give machine integers under dynamic kinds. Float-to-integer
conversion floors and saturates at the machine range:

``` bpl
⌊2.5 ¯2.5              ⍝ [2 ¯3]ₓ
⌊2*70                  ⍝ 9223372036854775807ₓ
```

Iota and random integer generation keep machine-integer arguments in
that domain. With [Mathish](mathish.qmd) on, an argument that must be a
whole number, such as an index or a count, accepts a float within
`1E¯14` of one. With Mathish off, the float must be exactly whole:

``` bpl
ι10×0.1×3              ⍝ 0 1 2
```

## Numbers in one array

Numbers that an operation puts in one array share a kind. A float among
exact integers makes them all floats, and a complex number makes every
number complex:

``` bpl
ι3ₓ , 0.5            ⍝ 0 1 2 0.5
ι2ₓ , 1j2            ⍝ 0 1 1j2
1ₓ,0.5 ÷ 2ₓ          ⍝ 0.5 0.25
```

Each number written in a literal list or in brackets keeps its own
exactness, even beside numbers of the other kind. Floats written beside
complex numbers become complex. Their values don’t change:

``` bpl
1ₓ 0.5÷2ₓ            ⍝ 0ₓ 0.25
[1ₓ 0.5]÷2ₓ          ⍝ 0ₓ 0.25
```

Under dynamic kinds, a write keeps each number’s exactness too. A float
assigned among exact integers, or an exact integer among floats, leaves
the array mixed, however the array was made. A Boolean written among
other numbers, or a number written among Booleans, leaves it mixed in
the same way. Under static kinds, the whole array takes the compact type
that holds both its items and the items written, as catenating them
would: a float written among bytes makes the array floats. Only items
that no compact type holds, such as characters among numbers, leave it
mixed. Mixed storage is slower. To keep an array compact, write values
of its own kind, as the [style guide](style.qmd) shows.
[`•explain`](system/explain.qmd) lists each write that left an array
mixed, in its `mixed` record. To give an existing array one kind,
convert it: `⌊` makes exact integers, `×1` makes floats, and `+0ₓ`
compacts items that already share a kind:

``` bpl
r←["a":2ₓ "b":3ₓ]
r.b←2.5
r                    ⍝ ["a":2ₓ "b":2.5]
m←3ρ$f
m.[[1]]←1
m                    ⍝ $f 1 $f
```

Around literals, `[…]ₓ` and `(…)ₓ` make every number a machine integer,
at any depth. A fraction floors: `[1.5 2]ₓ` is `[1 2]ₓ`, `¯2.5ₓ` is
`¯3ₓ` and `1r3ₓ` is `0ₓ`. An infinity saturates to the largest or
smallest 64-bit integer, and a finite literal past 64 bits raises LIMIT.
After a list or group that holds anything else, as after a name, `ₓ`
floors to exact integers, as [Superscripts and
subscripts](scripts.ipynb) describes. A vector of exact integers
displays as `[…]ₓ`, and rows of them as `(…)ₓ`. `⍕` writes them the same
way:

``` bpl
[1 0 1]ₓ≡1ₓ 0ₓ 1ₓ    ⍝ $t
[[1 2] 3]ₓ≡[1ₓ 2ₓ;3ₓ]   ⍝ $t
(0 1◇2 3)ₓ≡[[0 1]ₓ [2 3]ₓ]  ⍝ $t
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

Unbounded integers and rationals in the `ᵣ` domain have no compact
storage. An array that holds one uses mixed storage, as does an array
that holds numbers together with characters, nested arrays or functions.
A JSON object or a Python dict reads as a literal of its fields does, so
its storage is mixed only where the fields need it.

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
whose items are all ordinary machine integers with `ₓ`, whichever
storage holds them.

Ordinary integer storage keeps each item in 1, 2, 4 or 8 bytes. One byte
holds 0 to 255. Dynamic arithmetic computes at the narrowest width that
holds every result its arguments’ widths can give: the sum of two 1-byte
integers takes 2 bytes. A result beyond 8 bytes wraps at the 64-bit
range. `ᵣ` values never use this ordinary storage. `•storage` reports
the ordinary kind without its byte width. Imported typed integer buffers
use the fixed-width rules below.

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
200ₓ₁∧199ₓ₁         ⍝ 120ₓ₁
[200 100]ₓ₁<300ₓ     ⍝ $t $t
```

Two fixed integers compute at the wider type. In arithmetic, catenation,
union and writes, a [weak ordinary integer](#weak-and-strong-numbers)
and a fixed integer use their common integer type. The result stays
fixed. A weak integer literal retained as a constant contributes the
smallest type that holds it: `u8`, `i16`, `i32` or `i64`. For example,
`1ₓ₁+1ₓ` computes at `u8`, while `1ₓ₁+256ₓ` widens to `i16`.

A number read through a name or received as a function body’s argument
uses its declared kind, without inspecting its value for a smaller
width. This applies even when the caller passes a literal. Captured
numbers follow the same rule. An ordinary `i32` argument beside `u8`
therefore computes at `i32`, whether its value is 1 or 256. Literal
narrowing applies to promotion with fixed integers; an array of ordinary
integers retains its ordinary storage rules. A strong ordinary integer
makes the result ordinary, under the current `kinds` setting. Booleans
take the fixed type. Floats follow their existing promotion rules.

Integer `÷` floors at the promoted integer type. `÷ₐ` gives 32-bit
floats for `u8` and `i16`, and 64-bit floats for `i32` and `i64`, no
wider than the `float` preference. Under static kinds, where the default
is 32 bits, `i32` and `i64` use 32-bit floats for `÷ₐ` too. Comparisons
give Booleans.

Functions that rearrange or select items keep the fixed type, as sort
and `↑` do. So do functions whose results can’t exceed a fixed argument:
`ι`, `↦`, roll, deal, `⊤` and monadic `⨸`. Functions built from `+`, `-`
and `×` compute at the fixed type and wrap, as `+/` does. These are `⊥`,
polynomial evaluation with `⌻`, `!`, the determinant `-.×` and the
permanent `+.×`. A result of `∧`, `∨` or `!` that doesn’t fit the fixed
type wraps, as a sum does. Positions, counts and values that can exceed
the argument’s range are ordinary integers, such as the results of `⍸`,
`⍋`, `≢` and `⍭`. A function with no kernel at the fixed width gives
what static kinds give at that width: `○` on `u8` gives complex numbers
with 32-bit parts. `⌹` and polar `∧` give fixed integers floats of the
width that `÷ₐ` gives. A roll of fixed integers gives fixed integers,
and a bound of 0, which rolls a float, is a DOMAIN error.

``` bpl
ι3ₓ₁            ⍝ [0 1 2]ₓ₁
2ₓ⊥(9ρ1ₓ)ₓ₁     ⍝ 255ₓ₁
[0 1]ₓ₁,9ₓ      ⍝ [0 1 9]ₓ₁
⍸[0 1 1]ₓ₁      ⍝ [1 2]ₓ
```

`ι256ₓ₁` fails, because the literal `256ₓ₁` doesn’t fit `u8`. `ιₓ₁ 256ₓ`
gives 0 to 255 as `u8`.

Fixed integer literals must fit their type. After a value, a sized
suffix converts to that type. Integer narrowing wraps. Float and
rational conversion floors, as `⌊` does, then clamps to the target
range. NaN converts to zero; infinities convert to the range’s bounds.
Typed empty vectors use the same suffix, as in `⍬ₓ₁`.

Plain `ₓ` makes a fixed integer ordinary. The result is strong. Under
dynamic kinds it shares the array’s data without copying, and its
arithmetic follows ordinary machine-integer rules. Under static kinds it
converts the array to the `int` width, the type of every ordinary
integer there.

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

Bare numbers are floats of the default width: 64 bits under dynamic
kinds and 32 under [static kinds](#static-kinds), until
[`•prefs`](system/prefs.qmd) `["float":16]`, `["float":32]` or
`["float":64]` sets one width for both. A numeric token takes the active
settings when its containing statement executes. This rule applies in a
script, a notebook cell, a module and `⍎` text. A leading
`•prefs ["float":32]` sets the width for the statements that follow it.
A literal bound into a function takes its width when that function is
constructed; a literal in a dfn body takes its width when the body’s
statement runs. Changing the preferences between those statements can
give them different widths. Existing values keep their widths.

Numbers that `•csv`, `•json` and `•vfi` read also take the default
width. So do random numbers, the numbers that `•image` reads, and floats
that a function gives from exact arguments, such as `√2ₓ`. Times from
`•time` and `•date` are always 64-bit, because a 32-bit float holds
seconds since 1970 only to within about two minutes.

`ₕ` marks a 16-bit float, `ₛ` a 32-bit float and `ₚ` a 64-bit float,
whatever the default. Display marks floats of the other widths, once
after an array’s notation. `•literal⁻` marks every float. Its text
therefore reads back the same at any default.

``` bpl
⊢.kind¨•storage¨[1.5ₕ 1.5ₛ [1.5 2.5]ₚ ⍬ₛ]       ⍝ "float16" "float32" "float64" "float32"
[1.5 2.5]ₛ                          ⍝ [1.5 2.5]ₛ
•literal⁻ 0.5                     ⍝ "0.5ₚ"
```

An operation on floats computes at their width. An exact operand rounds
to the width that the operation computes at. Floats of two widths
compute at the wider, except that a [weak
number](#weak-and-strong-numbers) takes the width of the floats beside
it. Writes, catenation and arrays built from items follow the same rule:
`v,2.5`, `v₀←2.5` and `[v₀ 2.5]` keep a 32-bit `v`’s width. No operation
widens floats on its own.

``` bpl
v←32 •float 1.5 2.5
⊢.kind¨•storage¨[v×2ₓ v×v *v v×2 v×2ₚ v×v₀ v,2.5 [v₀ 2.5]]       ⍝ "float32" "float32" "float32" "float32" "float64" "float32" "float32" "float32"
•float v                     ⍝ 32ₓ
32•float1.5 × 16777217ₓ    ⍝ 25165824
```

Complex numbers have 64-bit or 32-bit parts. `•storage` gives their kind
as `"complex128"` and `"complex64"`, after their whole size. They follow
the rules for floats. A complex number takes the width of the floats it
comes from, and a 16-bit float that becomes complex takes 32-bit parts.
A complex number beside a float computes at the wider of their widths.
Complex results stay complex, even where the imaginary part becomes
zero. A complex atom displays its zero imaginary part, as `¯1ⱼ0`. An
array omits an item’s zero imaginary part only when another item has a
nonzero imaginary part. A literal is the one exception: under dynamic
kinds, `1j0` reads as the float `1`. A real-valued function, such as
absolute value `|`, still returns real storage. Use `ₐ` to take the real
part at the same width. `ₛ`, `ₚ` and `ₕ` mark the width of the parts,
and display marks parts that aren’t of the default width:

``` bpl
⊢.kind¨•storage¨[[2j3]ₛ;[2j3]ₕ;√[¯4]ₛ;[2j3]ₛ×1.5]       ⍝ "complex64" "complex64" "complex64" "complex64"
√¯4ₛ                                       ⍝ 0j2ₛ
0j1×0j1                                    ⍝ ¯1ⱼ0
0j1×0j1 1                                  ⍝ ¯1 0ⱼ1
```

Each operation on 16-bit floats computes in 32 bits and rounds its
result to 16 bits. For `+ - × ÷` and `√`, that gives the result of
16-bit arithmetic. Folds, scans and inner products keep their running
totals in 32 bits. Only their results round to 16 bits. A 16-bit total
of these numbers would stop growing at 256:

``` bpl
+/ 16•float4096ρ0.1   ⍝ 409.5ₕ
```

An exact operand converts straight to 32 bits, not to 16. A count larger
than 65,504, the largest 16-bit float, still divides a 16-bit sum:

``` bpl
h←16 •float 100000ρ1 0
+/÷≢ h                   ⍝ 0.49975586ₕ
```

[Power](glyphs/star-diaeresis.qmd) `⍣∞` stops at two results within
relative tolerance `1E¯14` of each other, or within 4 units in the last
place for 16-bit and 32-bit results. See [Mathish mode](#mathish-mode)
for comparison and division rules.

## Accuracy

BPL inherits rustymath’s floating-point accuracy guarantees and
[exceptions](https://answerdotai.github.io/rustymath/accuracy.html).
Pervasive functions give results within 4 units in the last place of the
correctly rounded result unless that page specifies otherwise.
BPL-specific qualifications follow below.

Rustymath’s Accuracy page also specifies which results are exact. With
Mathish mode off, `=` compares floats exactly:

``` bpl
•prefs ["mathish":$f]
7*5 = 16807          ⍝ $t
3√27 = 3             ⍝ $t
10⊛1000 = 3          ⍝ $t
○π0.5 = 0j1          ⍝ $t
•prefs ["mathish":$t]
```

Floating-point results need not agree bit for bit across platforms,
builds, runs, thread counts, array sizes or execution routes. Pervasive
functions retain the finite accuracy bounds above, including subnormals.
Results involving infinities or NaNs, the choice of NaN and the sign of
zero can differ as rustymath’s [Accuracy
page](https://answerdotai.github.io/rustymath/accuracy.html) describes.
FMA and reassociation can overflow differently near the range limits.
`⌈` and `⌊` can give either argument where one is NaN or both are zeros
of opposite signs. Compare floats from different routes within a
tolerance, as Mathish mode’s `=` does.

Algebraic reductions such as `+/` promise no association. Floating-point
cancellation can make reassociated results differ beyond the last bits.
A dfn whose one body is `α p ω`, for a primitive `p`, computes as `p`:
`{α+ω}/` has `+/`’s freedoms and its identity for an empty axis. Other
dfn bodies retain the right-to-left calls specified by
[Reduce](glyphs/slash.qmd).

Extra accuracy from a rewrite is not promised for literal-only
expressions or intermediates already read as values. A form that BPL
doesn’t recognise as a [fast
combination](terms.qmd#term-fast-combination), such as one that names a
constant, can give different digits, beyond the last bits where the
combination is more accurate. `⊛1+x` computes through `log1p` and keeps
the digits of a tiny `x`. `one←1 ◇ ⊛one+x` computes `⊛` of `1+x`, which
loses them.

## Mathish mode

[Mathish mode](mathish.qmd) is on by default. For [Mathish
types](terms.qmd#term-mathish-types), floating comparisons use relative
tolerance `1E¯14`, measured against the larger magnitude, and `0÷0` and
`1⊛1` give zero. Disable it with `•prefs ["mathish":$f]` for exact
floating comparisons and NaN at floating zero divided by zero.

The calculation’s kind and `kinds` setting decide whether these
conventions apply. Explicit 64-bit float suffixes and imported buffers
follow the same rule as literals. Static calculations and fixed-integer
operations never use Mathish conventions, even when their floating
computation uses 64 bits. The narrower floats keep exact comparisons and
raw division. Convergence stopping rules are independent of Mathish
mode.

## Infinities and NaN

Floats include `∞`, `¯∞`, NaN and `¯0` besides finite numbers. An
undefined result is NaN rather than an error, except for Mathish mode’s
zero divided by zero. `1÷0` is `∞`, and `⊛0` is `¯∞`. NaN is written
`$n`:

``` bpl
[0÷0 ∞-∞]            ⍝ 0 $n
[1÷0 ⊛0]             ⍝ ∞ ¯∞
```

Under `=`, NaN equals nothing, as in IEEE. `≡`, search, grade and Key
treat NaN as one value, and grade puts it after every other number:

``` bpl
$n=$n                ⍝ $f
$n≡$n                ⍝ $t
```

Complex arithmetic follows the `num_complex` crate. A complex number can
have infinite or NaN parts.

Putting an infinity or NaN into a numeric array uses float storage.
Joining one to exact integers makes them floats:

``` bpl
ι3ₓ , ∞              ⍝ 0 1 2 ∞
(•storage ι3ₓ , ∞).kind   ⍝ "float64"
```

CSV and JSON imports give NaN for a missing number. An integer column
with gaps then becomes floats. A column that also holds an integer too
large for a float to hold exactly keeps each item’s kind, in mixed
storage.

Bare `∞` and `¯∞` are symbolic bounds. Every finite exact number lies
between them, even beyond floating-point range. Minimum and maximum
preserve the selected exact argument:

``` bpl
3ₓ⌊∞                 ⍝ 3ₓ
¯∞⌈1ᵣ3                ⍝ 1ᵣ3
```

The [`⌈`](glyphs/upstile.qmd) and [`⌊`](glyphs/downstile.qmd) pages give
their results beside NaN. Explicitly float-converted infinities (`∞ₐ`,
`∞ₛ`) and computed infinities (`1÷0`) follow ordinary float promotion.
Arithmetic with a symbolic infinity also gives floats.

A native float array uses ordinary numeric promotion for every item
under minimum and maximum, including infinite items. Selecting an exact
argument at those positions does not change individual result kinds.

## Static kinds

Some functions choose a result’s kind from their arguments’ values.
`√¯4` is complex because the argument is negative, and `1j0` is a float
because its imaginary part is zero. Under [`•prefs`](system/prefs.qmd)
`["kinds":"static"]`, a result’s kind follows from the function and its
arguments’ kinds alone. A fused computation needs that, because it
decides each result’s kind before it runs. Real arguments give real
results, with NaN outside a function’s domain, and complex results stay
complex. `⌊`, `⌈` and monadic `×` of floats give floats. Integer `÷`
floors under both settings. With `ₓ` after them they give integers, as
Rust’s `as` converts a float: NaN gives 0, and a value past the `int`
width’s range gives its largest or smallest integer:

``` bpl
•prefs ["kinds":"static"]
√¯4 9                ⍝ $n 3
0j1×0j1              ⍝ ¯1ⱼ0
⌊2.5 ∞               ⍝ 2 ∞
7ₓ÷2ₓ                ⍝ 3ₓ
⌊ₓ2.5 ¯2.5           ⍝ [2 ¯3]ₓ
7ₓ÷ₓ2ₓ               ⍝ 3ₓ
⌊ₓ $n ∞              ⍝ [0 2147483647]ₓ
0÷0                  ⍝ $n
0.7=0.1+0.6          ⍝ $f
•prefs ["kinds":"dynamic"]
```

Ordinary integers compute at the width that the `int` pref sets, 32 bits
by default, or at a wider integer’s width, and wrap where a result
doesn’t fit. Display marks them, and fixed integers of that width, with
plain `ₓ`. Their storage takes that width too, whatever their values. A
static result read again under dynamic kinds widens as any dynamic
integer does. Code that wants narrow storage writes a fixed type, such
as `ₓ₁`. New floats take 32 bits, unless the `float` pref sets a width.
Fixed integers retain their explicit type. An integer beside a float
takes the float’s width. A weak number beside floats takes their width,
as under dynamic kinds. The explicit `ᵣ` domain stays unbounded and
exact, including whole results. `⌹` and polar `∧` give integers floats
of the width that `÷ₐ` gives them. A roll of integers gives integers,
and a bound of 0, which rolls a float, is a DOMAIN error. Floating
comparisons are exact. An ordinary integer literal that doesn’t fit the
`int` width is a LIMIT error. A count or position that doesn’t fit it,
such as the length that `≢` gives of an axis too long for it, gives the
width’s largest integer. [`•csv`](system/csv.qmd) and
[`•json`](system/json.qmd) read integers at the `int` width too, and
their `types` option gives a column another type:

``` bpl
•prefs ["kinds":"static"]
2147483647ₓ+1ₓ             ⍝ ¯2147483648ₓ
[1 2]ₓ₄+1ₓ                 ⍝ [2 3]ₓ
(•memory ι100ₓ).bytes      ⍝ 400ₓ
(•memory ι100ₓ₁).bytes     ⍝ 100ₓ
(•storage (32•float1 2)×2ₚ).kind    ⍝ "float64"
(•storage (32•float1 2)×2ₓ).kind    ⍝ "float32"
•prefs ["kinds":"dynamic"]
```

Under static kinds, other invalid numeric inputs give unspecified
results instead of errors, and errors of structure, type, names and the
host are still raised. Under dynamic kinds, invalid numeric inputs raise
their errors. In both modes, a read outside an axis gives the fill and a
write there writes nothing, as [Indexing](indexing.ipynb) describes.

[Performance](performance.qmd) explains why static kinds are the setting
for performance work. Code for static kinds gives each integer the width
that its values need and gives `•csv` and `•json` a `types` record for
their columns. Code written for dynamic kinds can give other results.
Dynamic kinds give this total of positions exactly. Static kinds wrap it
at the 32-bit range, unless a 64-bit seed widens the sum:

``` bpl
+/ι100000ₓ                 ⍝ 4999950000ₓ
•prefs ["kinds":"static"]
+/ι100000ₓ                 ⍝ 704982704ₓ
0ₓ₈+/ι100000ₓ              ⍝ 4999950000ₓ₈
•prefs ["kinds":"dynamic"]
```

Under static kinds, a write whose items have another kind converts the
whole array to the compact type that holds both kinds, where one exists.
The array’s type then doesn’t depend on which items the write selects.
Under dynamic kinds, each written item keeps its own kind in mixed
storage:

``` bpl
v←[1 2 3]ₓ₁ ◇ v.[[1]]←0.5 ◇ (•storage v).kind    ⍝ "mixed"
•prefs ["kinds":"static"]
v←[1 2 3]ₓ₁ ◇ v.[[1]]←0.5 ◇ (•storage v).kind    ⍝ "float32"
•prefs ["kinds":"dynamic"]
```

Results computed apart join the same way: the cells of a map, such as
Each, Rank, an outer product, a stencil or Key, the states that `⍣`
keeps for a list of counts, and the cases of a compacting select, such
as `⍚` or a guard. Under static kinds the result takes one compact type,
so its type doesn’t depend on which case each cell picks, or on whether
the map ran on all its cells at once. Under dynamic kinds, each cell’s
result keeps its own kind:

``` bpl
{(0<ω)⍚[(1<) (2×)]ω}¨¯1.5 2 3      ⍝ $f 4 6
•prefs ["kinds":"static"]
{(0<ω)⍚[(1<) (2×)]ω}¨¯1.5 2 3      ⍝ 0 4 6
•prefs ["kinds":"dynamic"]
```

## Approximate

`ₐ` converts to real floats, as `ₓ` converts to exact integers. After a
value, a real float keeps its width, a complex number gives its real
part at its parts’ width, and an exact number or a Boolean gives a float
of the default width:

``` bpl
(3j4)ₐ                            ⍝ 3
(1r3)ₐ                            ⍝ 0.3333333333
x←(1.5j2 2j3)ₛ ◇ [xₐ;(•storage xₐ).kind]   ⍝ [[1.5 2]ₛ "float32"]
```

`√`, `⊛`, `*`, `○` and `○⁻` keep their own real-part rule under `ₐ`, as
below. After any other pervasive function, `fₐ` converts the arguments
to real floats of the width that their float promotion gives, weak
literals included, before it calls `f`. `!ₐ25ₓ` is about `1.55ₑ25`,
computed in floats rather than in overflowing integers. A complex
argument gives its real part first, so `1j2×ₐ1j2` is `1`, and a
comparison gives Booleans. After a function that isn’t pervasive, `fₐ`
converts its results: `≢ₐ` gives the count as a float. Float32 arguments
give float32 results, even where `f` itself gives float64 or complex
numbers. The width follows from the arguments’ kinds and the
preferences, never from values. `÷ₐ` selects float division before
computing, rather than converting an integer quotient. Outside the real
domain a result is the real part of the principal value. `√ₐ¯4` is 0,
the real part of `0j2`. With a negative base, `¯8*ₐ÷3` is 1, the real
part of the principal root 1+i√3, not `¯2`. The real part of arcsin of 2
is π÷2:

``` bpl
√ₐ¯4 9                            ⍝ 0 3
⊛ₐ¯1                              ⍝ 0
¯8*ₐ÷3                            ⍝ 1
1○⁻ₐ 2                           ⍝ 1.570796327
x←(4 9)ₛ ◇ (•storage √ₐx).kind     ⍝ "float32"
```

`√ₐ`, `⊛ₐ` and `*ₐ` each run as one kernel, and so do `○ₐ` and `○⁻ₐ`
with a constant code. Because their result’s kind never depends on
values, they fuse and trace under dynamic kinds as they do under static
kinds. Under [`∂`](glyphs/partial.qmd), their slopes are the real
part’s: `√ₐ` has slope 0 at and below 0, `⊛ₐ` has slope 1÷x, and
arcsin’s real part has slope 0 beyond ¯1 and 1. Other marked pervasive
functions convert their arguments before `f` runs, and a marked function
that isn’t pervasive converts the results that `f` gives. Under static
kinds, a NaN that `f` gives outside its domain stays NaN:

``` bpl
{+/√ₐω}∂¯1 4                      ⍝ 0 0.25
{+/1○⁻ₐω}∂¯2 0.5                 ⍝ 0 1.154700538
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
