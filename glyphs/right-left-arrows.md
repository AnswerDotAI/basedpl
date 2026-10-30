

# `⇄` — Optimize

Monadic operator. Keys: `Alt-4`. Hanzi: `优` (`優`)

`f⇄ X` searches from the start `X` for the point where `f` is least, and
returns that point. `X` is a number or an array of real numbers. The
point has its shape and keys. `f` takes a point and returns one real
number. `O f⇄ X` takes options from the keyed record `O`. With option
`solve`, `f` returns residuals instead, and `⇄` searches for the point
where they’re zero.

``` bpl
(⊼2-⍨)⇄10   ⍝ 2
```

The point is approximate. A search stops when a step changes `f` by less
than option `tolerance`, relative to its value. With a smaller
tolerance, the search finds the least point of Rosenbrock’s function,
`1 1`, to nearly full precision:

``` bpl
rosen←{[x y]←⍵ ⋄ ⊼1-x + 100×⊼y-⊼x}
["tolerance":1E¯14] rosen⇄ ¯1.2 1   ⍝ 1 1
```

## Methods

Option `method` chooses the method. Without it, `⇄` minimizes with a
method it chooses from the start, the bounds and whether `∂`
differentiates `f`:

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th><code>method</code></th>
<th>Method</th>
<th>Chosen when</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"brent"</code></td>
<td>Brent’s method, along one variable</td>
<td><code>X</code> is one number</td>
</tr>
<tr>
<td><code>"l-bfgs"</code></td>
<td>L-BFGS, which follows the gradient</td>
<td><code>∂</code> differentiates <code>f</code> and there are no
bounds</td>
</tr>
<tr>
<td><code>"powell"</code></td>
<td>Powell’s method of conjugate directions</td>
<td>there are bounds</td>
</tr>
<tr>
<td><code>"nelder-mead"</code></td>
<td>Nelder–Mead’s simplex</td>
<td>none of the above</td>
</tr>
<tr>
<td><code>"differential-evolution"</code></td>
<td>differential evolution, which evolves a population of points within
the bounds</td>
<td>only when named</td>
</tr>
</tbody>
</table>

`∂` differentiates `f` when `f` uses only what [`∂`](partial.qmd)
differentiates, or has a JVP rule that `f∂←g` attaches. L-BFGS takes the
gradient at each point it needs from one run of `f`, which also gives
the value there. Nelder–Mead and Powell’s method use only values of `f`.
Differential evolution suits a function with many local minima, and
needs finite bounds.

``` bpl
rosen←{[x y]←⍵ ⋄ ⊼1-x + 100×⊼y-⊼x}
["tolerance":1E¯14] rosen⇄ ¯1.2 1   ⍝ 1 1
```

## Options

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th>Option</th>
<th>Meaning</th>
<th>Default</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>method</code></td>
<td>the method, from “Methods”, “Solving” or “Linear systems”</td>
<td>chosen as each of those describes</td>
</tr>
<tr>
<td><code>lower</code>, <code>upper</code></td>
<td>the least and greatest value of each variable: one number for every
variable, or an array with the start’s shape and keys</td>
<td>no bound, as <code>¯∞</code> and <code>∞</code> give</td>
</tr>
<tr>
<td><code>tolerance</code></td>
<td>the relative change at which the method stops, or for a linear
system, the residual relative to the right-hand side</td>
<td>the square root of the working floats’ machine epsilon</td>
</tr>
<tr>
<td><code>evaluations</code></td>
<td>the most calls of <code>f</code>, or for a linear system, the most
products with A and Aᵀ</td>
<td>1000 for each variable, plus 1000, and for differential evolution
another 1000 for each member of its population; for a linear system, 20
for each variable, plus 20</td>
</tr>
<tr>
<td><code>population</code></td>
<td>the number of points differential evolution evolves</td>
<td>15 for each variable, and at least 5</td>
</tr>
<tr>
<td><code>seed</code></td>
<td>the seed of differential evolution’s random choices</td>
<td>0</td>
</tr>
<tr>
<td><code>batch</code></td>
<td>whether to call <code>f</code> once on several points</td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>report</code></td>
<td>whether to return a record of the search’s results</td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>solve</code></td>
<td>whether <code>f</code> returns residuals to bring to zero, as
“Solving” describes</td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>linear</code></td>
<td>with <code>solve</code>, whether <code>f</code> is affine, as
“Linear systems” describes</td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>bracket</code></td>
<td>with <code>solve</code> and one variable, two points where the signs
of <code>f</code> differ</td>
<td>the search steps outward from the start</td>
</tr>
<tr>
<td><code>precondition</code></td>
<td>with <code>linear</code>, a function that applies M, an
approximation to A⁻¹, as “Linear systems” describes</td>
<td>none</td>
</tr>
</tbody>
</table>

Options `lower`, `upper`, `population`, `seed` and `batch` apply only to
minimizing. Each method evaluates `f` only within the bounds, starting
from `X` moved inside them.

``` bpl
rosen←{[x y]←⍵ ⋄ ⊼1-x + 100×⊼y-⊼x}
["lower":[¯∞ 0] "upper":[0.8 ∞]] rosen⇄ 0.5 0.5   ⍝ 0.8 0.64
```

A record of named parameters makes a natural start, and the point keeps
its keys. [`⍵.(…)`](../modules.qmd#records-as-namespaces) names the
parameters inside `f`. Bounds can be records with the same keys.

``` bpl
cost←{⍵.(⊼gap-6 + ⊼margin-2×gap)}
["lower":["gap":0 "margin":0]] cost⇄ ["gap":8 "margin":12]   ⍝ ["gap":6 "margin":12]
```

With `batch`, `⇄` calls `f` once for the points that a method evaluates
together: Nelder–Mead’s first simplex and each shrink of it, and each
generation of differential evolution. The points lie along a new last
axis of the start’s shape. `f` returns a vector with one value for each
point. Each field of a record start then holds one value for each point.
A vector start needs `+⌿` where one point needs `+/`.

``` bpl
cost←{⍵.(⊼gap-6 + ⊼margin-2×gap)}
lo←["gap":0 "margin":0] ⋄ hi←["gap":20 "margin":40]
["method":"differential-evolution" "lower":lo "upper":hi "batch":$t] cost⇄ ["gap":8 "margin":12]
⍝ ["gap":6 "margin":12]
```

With `report`, the result is a record. Field `x` holds the point, and
`converged` says whether the method’s stopping test passed before the
calls reached `evaluations`. Field `value` holds `f` at the point, or
when solving, the norm of the residuals. Field `evaluations` counts the
calls of `f`, or for a linear system, the products with A and Aᵀ. The
point is the best one the search evaluated, whether or not it converged.

``` bpl
r←["method":"nelder-mead" "evaluations":10 "report":$t] {+/⊼⍵-3}⇄ 1 2
[r.evaluations r.converged]   ⍝ 10ₓ $f
```

## Solving

With option `solve`, `f` returns residuals: one or more real numbers,
the same number at every point. `["solve":$t] f⇄ X` searches from `X`
for the point where they’re all zero. Where `f` returns more residuals
than `X` has variables, the point is where the sum of their squares is
least.

``` bpl
["solve":$t] (2-⍨⊼)⇄ 1   ⍝ 1.4142135623730951
```

One variable and one residual run Brent’s method, `"brent"`. It steps
outward from `X` until `f` changes sign, then narrows that bracket.
Option `bracket` starts the search from two given points where the signs
of `f` differ. Otherwise `⇄` runs Levenberg–Marquardt,
`"levenberg-marquardt"`. It takes the Jacobian from forward differences,
or from `f`’s derivative where `f` has one and returns no more residuals
than `X` has variables. A circle and a line cross where both residuals
are zero:

``` bpl
f←{[x y]←⍵ ⋄ [⊼x + ⊼y - 4;y-x-1]}
["solve":$t "tolerance":1E¯14] f⇄ 1 0   ⍝ 1.8228756555322954 0.8228756555322954
```

A record start names a model’s parameters. The residuals are the model’s
differences from the data. Where `∂` differentiates the residuals,
Levenberg–Marquardt builds its Jacobian from one derivative along each
parameter. Otherwise it takes forward differences, which leave the
parameters accurate to about 9 digits. Here `a×*b×x` fits points that
grow about as fast as `2×*x`, with a tolerance that makes every digit
shown correct:

``` bpl
x←0 1 2 3 4 ⋄ y←2.1 5.4 15.2 40.3 109.1
["solve":$t "tolerance":1E¯12] {⍵.(a×*b×x - y)}⇄ ["a":1 "b":1]   ⍝ ["a":2.055580005 "b":0.9928844207]
```

## Linear systems

With options `solve` and `linear`, `f` is affine: f(x) is A·x−b for some
matrix A. `⇄` never forms A. It multiplies vectors by A through `f`,
and, for least squares, by Aᵀ through `∂`, which transposes the steps
that `f` takes. That suits a matrix that’s large and sparse, or never
formed at all.

``` bpl
m←[4 1⋄1 3] ⋄ b←1 2
["solve":$t "linear":$t] {m+.×⍵ - b}⇄ 0 0   ⍝ 0.09090909090909091 0.6363636363636364
```

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th><code>method</code></th>
<th>Method</th>
<th>Chosen when</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"gmres"</code></td>
<td>restarted GMRES</td>
<td>the system is square</td>
</tr>
<tr>
<td><code>"minres"</code></td>
<td>MINRES on the normal equations Aᵀ·A·x = Aᵀ·b, which takes the same
steps as LSMR</td>
<td>the system isn’t square, and <code>∂</code> differentiates
<code>f</code></td>
</tr>
<tr>
<td><code>"cg"</code></td>
<td>conjugate gradients on the normal equations, which take the same
steps as LSQR</td>
<td>only when named</td>
</tr>
</tbody>
</table>

The normal equations square A’s condition number. For an ill-conditioned
A, MINRES and CG on them reach less accuracy than LSMR and LSQR would.
On a square system, `"cg"` and `"minres"` work on A itself, which must
be symmetric, and positive definite for CG.

`∂` gives the products with Aᵀ that least squares needs:

``` bpl
m←[1 0⋄0 1⋄1 1] ⋄ b←1 2 4
f←{m+.×⍵ - b}
["solve":$t "linear":$t] f⇄ 0 0   ⍝ 1.3333333333333333 2.3333333333333335
```

`⇄` computes each product A·v as (f(t·v)−f(0))÷t, with a power of two t
that makes t·A·v larger than b. A large b therefore costs no accuracy.

Option `precondition` gives a function that applies M, an approximation
to A⁻¹, or to (Aᵀ·A)⁻¹ for least squares. It takes and returns arrays of
`X`’s shape. A good M cuts the products a solve needs. CG and MINRES
apply M at each step and need it symmetric positive definite.
Preconditioned MINRES stops on the residual measured through M,
√(rᵀ·M·r), while CG and GMRES stop on the residual itself. GMRES applies
M on the right. Calls of M don’t count against `evaluations`. For a
matrix whose diagonal spans five orders of magnitude, dividing by the
diagonal cuts the products from 22 to 9:

``` bpl
n←20 ⋄ d←10*5× ⍳n ÷ n-1 ⋄ m←d×=⊗⍨⍳n + 0.1×1=|-⊗⍨⍳n ⋄ b←n⍴1
f←{m+.×⍵ - b} ⋄ o←["solve":$t "linear":$t "report":$t "tolerance":1E¯12]
[(o f⇄ n⍴0).evaluations (o,["precondition":{⍵÷d}] f⇄ n⍴0).evaluations]   ⍝ [22 9]ₓ
```

## Data

Bind data to `f` with [`↣`](right-tail-arrow.qmd). The sum of squared
differences from `1 2 3 4` is least at their mean:

``` bpl
1 2 3 4↣{+/⊼⍺-⍵}⇄ 0   ⍝ 2.5
```

To search with a hand-written JVP rule and data, bind the data first,
then attach the derivative to the bound function:
`fit←data↣loss ⋄ fit∂←g ⋄ fit⇄ start`.

## Numbers

The search works in floats of the start’s width. 32-bit and 16-bit
floats compute in 32 bits, and exact numbers become floats of the
default width. A value of NaN counts as greater than every number.

## Errors

- `LENGTH`: bounds of another shape or other keys, a `bracket` that
  isn’t two numbers, a batched `f` that doesn’t return one value for
  each point, or an `f` that returns another number of residuals at
  another point
- `DOMAIN`: a left argument that isn’t keyed options, an unknown option
  or method, an option for another task, such as `lower` with `solve` or
  `linear` without it, a method for another task, a start or bound that
  isn’t real numbers, a lower bound above its upper bound, an `f` that
  doesn’t return one real number when minimizing, `"brent"` with more
  than one variable or residual, `"l-bfgs"` with bounds or for an `f`
  that `∂` can’t differentiate, `"differential-evolution"` without
  finite bounds or with a population below 4, `"levenberg-marquardt"`
  with a `bracket`, least squares on a linear system whose `f` `∂` can’t
  differentiate, a `precondition` that isn’t a function, a `tolerance`
  that isn’t positive, or an `evaluations`, `population` or `seed` that
  isn’t a whole number
- `UNSUPPORTED`: a point where `f` reaches code that `∂` can’t
  differentiate, during L-BFGS or least squares, as when a guard takes
  another branch. Option `method` chooses a method that uses only values
