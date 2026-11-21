

# Performance

BPL interprets each function call once, then runs it on the whole array
in compiled Rust. Interpreting a call costs about the same whatever the
size of its arguments. On large arrays, most of the time is spent
processing the items. BPL also runs chains of calls on large arrays
together, in one pass over memory, and compiles that pass into machine
code. Under static kinds, more calls can run together in this way.

## Static kinds

Dynamic kinds, the default, suit interactive analysis. Integer
arithmetic takes the narrowest width that holds every result, up to 64
bits, and some results take their kind from their values, as `√¯4` gives
a complex number. For performance work, set static kinds with
`•prefs ["kinds":"static"]`. Under static kinds, each result’s kind
follows from its arguments’ kinds, which lets BPL know the type of every
result before a computation runs and run more calls together. Under
dynamic kinds, `⌊` gives exact integers and runs on its own. Under
static kinds, `⌊` gives floats and runs with the multiplication and the
sum:

``` bpl
y←¿200000ρ0
(•explain "+/⌊10×y").fused.glyphs      ⍝ 0ρ⊂""
•prefs ["kinds":"static"]
(•explain "+/⌊10×y").fused.glyphs      ⍝ ["×⌊+/"]
•prefs ["kinds":"dynamic"]
```

In code meant for static kinds, give each integer the width that its
values need, and pass `•csv` and `•json` a `types` record for their
columns. [Numbers](numbers.qmd#static-kinds) shows how code written for
dynamic kinds can give different results under static kinds.

Put `•prefs ["kinds":"static"]` at the start of a file or notebook cell.
Give data and constants the widths that the calculation is to use
throughout:

``` bpl
•prefs ["kinds":"static"]
x←[1 2 3]ₛ
i←[0 1 2]ₓ₄
rate←0.01ₛ
```

`ₛ` and `ₚ` select 32-bit and 64-bit floats. `ₓ₄` and `ₓ₈` select signed
32-bit and 64-bit integers. Fixed-width integers wrap on overflow.
Choose a width that holds every intermediate result you need, not only
the inputs. Plain `ₓ` selects no width: an ordinary integer has the
width of the `int` preference.

Convert imported data once, or give its columns types when reading it.
Keep loop state and branch results at compatible kinds and widths. A
[strong](numbers.qmd#weak-and-strong-numbers) 64-bit float beside 32-bit
data widens the arithmetic. A weak literal takes its partner’s width.
[Numbers](numbers.qmd) gives the promotion rules.

A wider reduction seed widens its accumulator without widening the
stored input:

``` bpl
i←1000ρ255ₓ₁
+/i                 ⍝ 24ₓ₁
0ₓ₈+/i              ⍝ 255000ₓ₈
```

A result conversion such as `(+/)ₓ₈` happens after the reduction and
cannot recover a result lost to overflow.

## Choose the intended numerical operation

Use the real forms `√ₐ`, `*ₐ`, `⊛ₐ` and `○ₐ` when values can leave a
real domain and you want real results. The kind of their result never
depends on the values of their arguments, and the same is true of the
real inverse-circle form `○⁻ₐ`. Even under dynamic kinds, their calls
can therefore join a [fused group](terms.qmd#term-fused-group) and be
[traced](terms.qmd#term-trace), provided that `○ₐ` or `○⁻ₐ` gets a
single code rather than one code for each item. Unmarked `√`, `*`, `⊛`
and `○` have no such guarantee. The real forms preserve the arguments’
promoted float width.

`fₐ` gives the real part of the principal result outside the real
domain. For example, `√ₐ¯4` gives 0, not NaN or an error. Choose that
meaning intentionally. Use Agenda when different inputs need different
calculations.

`a÷ₓb` floors the quotient to an exact integer. It is the fast form for
floor division, not exact rational division and not truncation towards
zero. For a floating quotient, convert integer data to the intended
float width before dividing. Use ordinary `÷` on exact integers when you
need its numerical meaning, rather than replacing it for speed.

Calls on 16-bit floats and on complex numbers don’t join a [fused
group](terms.qmd#term-fused-group). Each call runs on its
[kernel](terms.qmd#term-kernel), over the whole array. Narrower storage
alone does not guarantee faster execution.

Calls that run in a fused group, and reductions that BPL reassociates,
can give different last bits, infinities, NaNs and signs of zero. Finite
results follow the accuracy bounds in [Numbers](numbers.qmd#accuracy).

## Calls that wait

An elementwise call that gives 65,536 numbers or more doesn’t compute at
once. It waits, and its result is [pending](terms.qmd#term-pending). The
calls that read a pending result join it. When something needs the
items, the whole chain runs together as one fused group, block by block.
A fused group reads each input array once and writes only what it must
keep. A chain of arithmetic that ends in a reduction takes about as long
as copying its data once. [`•explain`](system/explain.qmd) lists the
calls of each fused group in its `fused` field:

``` bpl
y←¿1000000ρ0
(•explain "+/ 2×y+1.5 - 1").fused.glyphs    ⍝ ["+×-+/"]
```

Because each waiting call costs a fixed amount of planning, waiting
saves time only on large arrays.

### Which calls wait

Calls of these glyphs wait: monadic `+ - ÷ | * ! π ⊼ ⊽ ≤ ≥`, and dyadic
`+ - × ÷ | ⌈ ⌊ π` and the comparisons. Calls on Booleans, such as `∧`
and `~`, wait too.

Under dynamic kinds, some calls wait only for some arguments:

- `√` and `⊛` wait when an earlier call keeps their argument from being
  negative or ¯0, as in `√|y` and `⊛1+*y`.
- Dyadic `*` waits when its base is such a value, as in `0.5*y`. Dyadic
  `⊛` waits when both of its arguments are.
- Calls on integers wait only at a fixed width, where integers wrap.
  Under static kinds every integer wraps, and these calls always wait.

In the first line below, the comparison runs with the arithmetic and
`+/`. In the second, `|` keeps the argument of `√` from being negative,
which lets `√` run with the other calls:

``` bpl
y←¿1000000ρ0
(•explain "+/ y+1 < |y-0.5").fused.glyphs    ⍝ ["-|+<+/"]
(•explain "+/√|y-0.5").fused.glyphs       ⍝ ["-|√+/"]
```

A call can wait when it can’t fail and the type of its result doesn’t
depend on the values of its items. Under static kinds more calls meet
that condition, because a call gives NaN or a float where dynamic kinds
would give a complex number or an integer. The next example lists the
glyphs whose calls joined a fused group. Its arguments are halves of
small integers, which keeps `∨` and `∧` quick. Under static kinds, every
monadic glyph tested joins except `○`, whose results are complex, and
`+`, which returns floats unchanged without making a call:

``` bpl
y←0.5×¿200000ρ100
fuses←{0ₓ<≢(•explain ω).fused.glyphs}
monads←"+-×÷|⌈⌊*⊛√!○π⊼⊽≤≥"
dyads←"+-×÷|⌈⌊*⊛√!π∨∧=≠<≤>≥"
{fuses "+/",ω,"y"}¨monads # monads      ⍝ "+-÷|*!π⊼⊽≤≥"
{fuses "+/0.5",ω,"y"}¨dyads # dyads     ⍝ "+-×÷|⌈⌊*π=≠<≤>≥"
•prefs ["kinds":"static"] ◇ {fuses "+/",ω,"y"}¨monads # monads    ⍝ "-×÷|⌈⌊*⊛√!π⊼⊽≤≥"
•prefs ["kinds":"dynamic"]
```

A float-width suffix, a fixed-width integer suffix or `•float` on a
pending result runs with the fused group, and only the converted items
are stored. A comparison runs with a later call that reads it, as in
`y×x>0`, unless a name also holds the comparison’s result.

### Structural functions

Structural functions such as `⌽`, `⍉`, `↑`, `↓` and `↕` wait too, on
arrays of 65,536 items or more of any kind. The pending result records
only where each of its items comes from in the argument. Calls in a
fused group, reductions, other pervasive calls and `+.×` read those
items from the argument without building the result:

``` bpl
w←1000↕¿1000000ρ0
(•storage w).pending     ⍝ $t
```

### What ends the wait

A pending result is [forced](terms.qmd#term-forced) when a call needs
its items in memory. `⍋` forces the whole of a pending result that it
reads. So do display, and a Python conversion other than a NumPy view of
a structural result. A reduction such as `+/` of a pending result runs
with the fused group and forces nothing. `•explain` has a row in
`forced` for each call that computed items of a pending result, with the
number of items computed.

A selection of a few items computes only those items:

- From a pending structural result, every small selection reads the
  argument, and the result stays pending.
- From a pending elementwise result, the first small selection computes
  only the items selected, even when a name holds the result. The next
  one computes and stores the whole result.
- A selection that is empty, or that holds only fill, doesn’t count as a
  read.

``` bpl
y←¿1000000ρ0
s←2×y+1 ◇ (•explain "5↑s").forced.items      ⍝ [5]ₓ
(•explain "5↑s").forced.items                ⍝ [1000000]ₓ
(•explain "5↑s").forced.items                ⍝ ⍬ₓ
```

A chain of pending calls has two limits. A call that would make the
chain longer than 64 calls, or make it keep alive more than 8 times the
memory of its largest array, first computes the pending results that it
reads.

A pending call uses the settings that were in force when it was called.
Here a comparison waits under Mathish mode’s tolerance, and uses that
tolerance after the setting changes:

``` bpl
y←1+¿200000ρ0
c←y=y+1E¯15
•prefs ["mathish":$f]
+/c                          ⍝ 200000ₓ
+/y=y+1E¯15                  ⍝ 0ₓ
•prefs ["mathish":$t]
```

With the [`fusion`](system/prefs.qmd) setting at `$t`, BPL writes a line
for each call that forces a pending result.

## Compiled loops

BPL can compile the loop of a fused group into machine code. A [compiled
loop](terms.qmd#term-compiled-loop) keeps intermediate values in
registers. By default:

- A fused group of two or more operations compiles on its first run when
  it has 3,145,728 items or more.
- A smaller fused group of two or more operations uses a compiled loop
  that the process has already made for the same calls and types. If
  there is none, the fused group compiles once such fused groups have
  processed twice as many items as the larger of its own item count and
  65,536.
- A single operation runs on its [kernel](terms.qmd#term-kernel) and
  doesn’t compile.

Only some calls compile: those for which a machine instruction gives
exactly the result of the call’s kernel, such as `+`, `-`, `×`, `⌈` and
`⌊` on floats. Two or more such calls in a row compile together. So do
the calls of a fused group in which every call is of this kind. The
fused group’s other calls run on their kernels.

Compiling a loop takes less than a millisecond. `•explain` has a row in
`compiles` for each loop compiled, and records each run of compiled code
as a `"compiled"` route:

``` bpl
y←¿3145728ρ0
(⊂"compiled")∊(•explain "+/ 2×y+1.5 - 1").routes.route    ⍝ $t
```

The browser build compiles nothing at run time. Its fused groups run on
kernels built into the module, as [BPL on the
web](web.ipynb#differences-from-native-bpl) describes.

## Maps

Each, Rank, stencils and outer products call a dfn once, on all their
cells together, when they can. `{ω×ω+1}¨v` then costs about what `v×v+1`
costs. [Evaluation](evaluation.qmd#functions-and-effects) gives the
rules. With the `batch` setting at `$f`, a map calls its operand once
for each cell.

## The cost of a call

A call that isn’t part of a [program](#programs) pays a fixed cost to
interpret it, whatever the size of its arguments. A call of `{ω}` costs
about 1,500 machine instructions. Each call of a recursive Fibonacci
function costs about 4,600. A loop over the items of a million-item
array pays that cost a million times. Code that works on whole arrays
pays it once for each call. Compiling doesn’t remove this cost, because
BPL compiles only the loops inside a fused group.

## Names and writes

A dfn releases each local name, and `ω` and `α`, after the last
statement that reads it. A name is held until the dfn returns when a
nested dfn or an error guard’s handler mentions it, or when the dfn
reads names by text, as `⍎` does. If only a released name held a pending
result, that result then runs with the calls that read it. When a name
or another array still holds a pending result that a fused group reads,
the fused group stores that result’s items in the same pass. In `f`, the
arithmetic and the sum run as one fused group without storing an
intermediate array:

``` bpl
big←¿1000000ρ0
f←{a←ω×2 ◇ b←a+1 ◇ +/a×b}
(•explain "f big").fused.glyphs       ⍝ ["×+×+/"]
≢(•explain "f big").stored.glyphs     ⍝ 0ₓ
```

[Dfns](dfns.ipynb#scope) describes what a released name means for `⍎`.

Writing to part of an array changes the array in place when nothing else
holds it. `x,←v` and `x⍪←v` add to the end of the array’s storage.
`x←x,v` copies the whole array, because `x` still holds the old array
while `x,v` is computed. [Assignment](assignment.ipynb) describes each
form of write.

## Threads

BPL splits large calls across the machine’s cores. BPL splits a call
only when it has enough work to repay the extra threads. The `threads`
setting sets how many threads a call can use. Several BPL processes
running at once compete for the cores unless each sets `threads` to 1.
The browser build runs every call on one thread.

## Seeing what happened

[`•explain`](system/explain.qmd) runs code and records what happened:
the fused groups that ran, the loops that compiled, the calls that
forced pending results, and the maps that fell back to a call for each
cell. [`•memory`](system/memory.qmd) counts the pending arrays that a
thread holds, and the bytes that a value keeps alive.
[`•storage`](system/storage.qmd) shows how an array is held, and whether
it’s still pending.

## Programs

[`∂`](glyphs/partial.qmd) [traces](terms.qmd#term-trace) the function it
differentiates into a [program](terms.qmd#term-program) of typed steps.
The program runs through the same fused groups and compiled loops. BPL
keeps the program and replays it on later calls, for as long as the
arguments and the function’s free names have the types and shapes that
the program was built for.

BPL also builds programs for some operands that are called many times on
arrays in [compact storage](terms.qmd#term-compact-storage): the operand
of Power with a single finite count, and the operand of Each, Rank, an
outer product, or a fold or scan by a function that is not a primitive,
after its [batched call](terms.qmd#term-batched-call) fell back. For
these operands, BPL builds a program only when two calls in a row have
arguments of the same types and shapes. `∂` builds its program on the
first call. A program that already exists is replayed at once.
`•explain` has a row in `programs` for each program built or replayed.
It records nothing for the first of two such calls, while BPL waits to
see whether the second matches. A call of a kind that never gets a
program runs in the usual way, and makes no row in `programs` or
`declined`.

Under dynamic kinds, a call whose result type can depend on its values
stops a trace. On reals, those calls are `√` and every root but the cube
root `3√`, `⊛`, `!`, `0○`, and `*` whose exponent isn’t a whole number
or is the value being differentiated. An earlier call that keeps the
argument from being negative, as `⊼` does, lets such a call trace.
Otherwise `∂` runs without a program:

``` bpl
(•explain "{+/√ω}∂1 2 3").programs.event      ⍝ 0ρ⊂""
•prefs ["kinds":"static"]
(•explain "{+/√ω}∂1 2 3").programs.event      ⍝ ["built"]
•prefs ["kinds":"dynamic"]
```

[Automatic differentiation](autodiff.qmd) describes how `∂` works.

## Not built yet

The plan includes:

- One pass for several reductions of the same data. Today each
  reduction, as in `+/v ÷ ⌈/v`, makes its own pass.
- One program for a whole training step: its forward pass, gradient and
  update. Today a program holds one call of `∂` or one call of an
  operand.
- Programs for maps whose cells take different branches, so that `∂` can
  trace them.
- The GPU, through WebGPU, with 16-bit brain floats (bf16), and compiled
  code in the browser.
