

# `⍣` — Power

Dyadic operator. Keys: `Alt-1 8`. Hanzi: `次/止`

`f⍣N Y` applies `f` `N` times. For nonnegative whole counts, the exact
rules are:

<table>
<thead>
<tr>
<th>Call</th>
<th>Equivalent</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>(f⍣0)Y</code></td>
<td><code>Y</code></td>
</tr>
<tr>
<td><code>(f⍣(N+1))Y</code></td>
<td><code>f((f⍣N)Y)</code></td>
</tr>
<tr>
<td><code>X(f⍣0)Y</code></td>
<td><code>Y</code></td>
</tr>
<tr>
<td><code>X(f⍣(N+1))Y</code></td>
<td><code>X f(X(f⍣N)Y)</code></td>
</tr>
</tbody>
</table>

In dyadic calls, each result becomes the next right argument, while the
left argument stays fixed. For example, `2(+⍣3)1` is `2+(2+(2+1))`.

The [separator `⁝`](tricolon.qmd) keeps a literal count apart from a
literal argument, as in `⊽⍣3⁝1`. With a left argument, put parentheses
around the derived function.

``` bpl
⊽⍣3⁝1             ⍝ 8
2(+⍣3)1            ⍝ 7
```

For a negative count, Power applies the known inverse of `f`.
[`f⁻`](superscript-minus.qmd) is `f⍣¯1`. Repeat with `f⍣2`; `f²` instead
means rank 2.

``` bpl
⊽⍣¯1⁝10           ⍝ 5
10 -\⍣¯1 [9 7 4]     ⍝ 1 2 3
```

A vector of counts gives one item for each count, holding the state
after that many steps. Counts `ιN` give the first `N` states, starting
with `Y`. A nested vector of counts gives a nested result, with one item
for each inner vector. A number gives the state itself; an enclosed
count gives an enclosed state. Computed counts go in parentheses or in
the run after `⍣`.

``` bpl
⊽⍣(ι5ₓ) 1               ⍝ [1 2 4 8 16]
≥⍣(-ι3ₓ) 5             ⍝ [5 4 3]
≥⍣3 ¯2 0 3⁝10        ⍝ 13 8 10 13
,↢1⍣ ι3ₓ [2]           ⍝ [[2] [2 1] [2 1 1]]
ρ {÷'a'}⍣(0 2ρ0) "ab"  ⍝ [0 2]ₓ
⊽⍣[2 [4 ¯2 1]] 1        ⍝ [4 [16 0.25 2]]
⊽⍣(⊂3) 1               ⍝ ⊂8
```

Power runs `f` for the positive counts first, then the inverse for the
negative counts, also from `Y`. Repeated counts are not recomputed.
Empty counts give an empty result with the counts’ shape.

## Computed counts

A function right operand computes the counts once from the original
arguments:

<table>
<thead>
<tr>
<th>Call</th>
<th>Equivalent</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>(f⍣g)Y</code></td>
<td><code>(f⍣(g Y))Y</code></td>
</tr>
<tr>
<td><code>X(f⍣g)Y</code></td>
<td><code>X(f⍣(X g Y))Y</code></td>
</tr>
</tbody>
</table>

The counts follow the same rules as a number or array operand. For
addition, repeat Increment as many times as the left argument says.
[Unary](superscript-u.qmd) `ᵘ` keeps Increment monadic while Power
passes the fixed left argument:

``` bpl
p←≥ᵘ⍣⊣ ◇ 2 p 3      ⍝ 5
```

## Until

A [scalar holding a function](../scripts.ipynb#a-function-as-a-scalar)
is a stopping predicate. `f⍣gˢ Y` applies `f` at least once, stopping
when `new g old` is true and returning `new`. In a dyadic call, each
step is `X f old`; the predicate still receives `new` and `old`.

``` bpl
≥⍣{α>4}ˢ0          ⍝ 5
```

## Fixed point

`f⍣∞` iterates until consecutive states match within the [convergence
tolerance](../numbers.qmd#float-widths), independently of comparison
settings. `f⍣≡ˢ` uses ordinary Match instead. `f⍣¯∞` iterates the
inverse. A vector of counts can include `∞`.

Here `n` is Newton’s step towards √2. It gives half of its argument
(`⊽⁻`, the inverse of double) plus the argument’s reciprocal (`÷`).

``` bpl
n←⊽⁻+÷
n⍣∞⁝1                 ⍝ 1.414213562373095
n⍣[0 ∞]1               ⍝ 1 1.414213562373095
```

## History

The until form keeps every state when its predicate is in a one-item
vector: `f⍣[g]`. The states run from `Y` to the first state where `g`
holds. `f` always runs at least once. For example, `⊢⍣[≡]4` gives `4 4`.

``` bpl
≥⍣[{α=3}]0           ⍝ 0 1 2 3
⊢⍣[≡]4                ⍝ 4 4
n←⊽⁻+÷ ◇ ≢n⍣[≡]1         ⍝ 7ₓ
```

See [assigning an
inverse](../assignment.ipynb#inverses-and-derivatives).

## Inverse

`(f⍣n)⁻` is `f⍣(-n)`.

``` bpl
(⊽⍣3)⁻16   ⍝ 2
```

Inverses propagate through Atop, dyadic After and Before, bound
arguments, Each, Rank, dyadic Commute and scans. Primitive inverses
include arithmetic bindings, exp/log, circle, axis permutations,
encode/decode, enclosure/Mix/Split, Where and
[Replicate](hash.qmd#expand). An inverse that takes a square root or a
log gives the principal branch.

## Errors

- `DOMAIN`: a count that is neither whole nor infinite, non-Boolean
  predicate, unknown inverse
