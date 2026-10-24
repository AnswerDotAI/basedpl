

# Mathish mode

You can usually work with numbers in BPL as you would on paper. Adding a
tenth to two tenths gives three tenths, and comparing the result with
three tenths gives true:

``` bpl
0.3=0.1+0.2             ⍝ $t
```

Most languages give false here. Their floating-point numbers cannot
represent these decimal fractions exactly. Each operation rounds to a
nearby representable number, and two ways of calculating the same
quantity can finish a little apart.

BPL’s default Mathish mode follows APL and J in choosing numerical
conventions that suit interactive work and teaching. Tolerant comparison
overlooks small rounding differences. Defining zero divided by zero as
zero makes several useful formulas work at zero too. Neither convention
makes floating-point arithmetic exact, but they let you concentrate on
the calculation most of the time.

## Comparing calculated numbers

APL’s first implementers wanted comparisons to reflect the mathematics
behind a calculation rather than differences in its machine
representation. In [The Evolution of
APL](https://www.jsoftware.com/papers/APLEvol1.htm), Falkoff and Iverson
describe the comparison tolerance that Larry Breed called *fuzz*. They
made the allowed difference proportional to the larger magnitude of the
two numbers.

BPL uses relative tolerance `1E¯14` for its 64-bit floating comparisons.
This is small enough to ignore ordinary rounding differences without
making visibly different numbers equal:

``` bpl
7=100×0.07             ⍝ $t
2=(√2)*2               ⍝ $t
0.3=0.3001             ⍝ $f
```

You can use `=` to find values you calculated another way without first
writing an approximate-equality function. The same convention applies to
[comparisons and searches](comparison.qmd). It also keeps a
[floor](glyphs/downstile.qmd) or [ceiling](glyphs/upstile.qmd) from
jumping to the next integer because of a small rounding error.

Tolerance does not recover digits lost in an unstable calculation. BPL
also has [exact integers and
rationals](numbers.qmd#exact-and-approximate) when exact arithmetic is
the calculation you mean.

## Choosing a useful zero

Division normally asks for a quotient `q` such that `y×q=x`. At `0÷0`,
every `q` satisfies that equation. Choosing a result is a convention. In
his 1976 paper [Zero Divided by
Zero](https://www.jsoftware.com/papers/eem/0div0a.htm), Eugene McDonnell
argued for zero instead of APL’s then-current choice of one.

The identities `x÷x=1` and `0÷x=0` disagree at zero. McDonnell preferred
preserving the second. It makes formulas involving zero behave more
uniformly. For example, dividing a number by its magnitude gives its
direction, including at zero:

``` bpl
x←[¯3 0 4]
x÷|x                   ⍝ ¯1 0 1
```

Normalising counts is another useful example:

``` bpl
shares←{⍵÷+/⍵}
shares [2 3 5]         ⍝ 0.2 0.3 0.5
shares [0 0 0]         ⍝ 0 0 0
```

An empty category contributes zero rather than a NaN even when every
category is empty. The all-zero result means there were no counts; it is
not a probability distribution.

The usual mean formula also returns zero for an empty vector:

``` bpl
mean←+/÷≢
mean [2 4 9]           ⍝ 5
mean ⍬                 ⍝ 0
```

That convention is useful when empty groups should contribute nothing.
Where an empty group means missing data, check for it or disable Mathish
mode instead of interpreting its zero as a measured mean.

McDonnell also considered logarithms. Since `b⍟x` is `⍟x÷⍟b`, the case
`1⍟1` becomes another zero divided by zero. Mathish mode gives it zero
too.

## Division by zero in mathematics

[Lean’s
Mathlib](https://leanprover-community.github.io/mathlib4_docs/Mathlib/Algebra/GroupWithZero/Basic.html#div_zero)
and
[Isabelle/HOL](https://isabelle.in.tum.de/library/HOL/HOL/Fields.html)
define the inverse of zero as zero. Division then gives zero for every
numerator over zero. Theorems that cancel a divisor require it to be
nonzero. In algebra, [involutive
meadows](https://arxiv.org/abs/0901.0823) study this kind of total
division.

These are useful mathematical conventions, not a way to prove that
ordinary division has a unique answer at zero. BPL adopts the narrower
`0÷0=0` rule. A nonzero real divided by zero still gives an infinity,
and reciprocal at zero is unchanged:

``` bpl
0÷0                    ⍝ 0
1÷0                    ⍝ ∞
÷0                     ⍝ ∞
```

## Choosing numerical behaviour

The default dynamic kinds use Mathish conventions for [Mathish
types](terms.qmd#term-mathish-types).
[Numbers](numbers.qmd#mathish-mode) gives the calculation’s type rules.

For ordinary REPL work and notebook examples, the extra numerical checks
are rarely a performance concern. With very large arrays or computations
dominated by comparisons, searches or division, their cost can matter.
Measure the workload before changing its numerical behaviour.

To use exact floating comparisons and raw floating division, disable
Mathish mode:

``` bpl
•prefs ["mathish":$f]
0.3=0.1+0.2             ⍝ $f
0÷0                    ⍝ $n
•prefs ["mathish":$t]
```

Another option is [32-bit floats](numbers.qmd#float-widths), which
always use exact comparisons and raw division, even with Mathish mode
enabled:

``` bpl
0ₛ÷0ₛ                  ⍝ $nₛ
```

They use half the storage of 64-bit floats and often suit
high-throughput numerical work. They also keep only about seven
significant decimal digits. Choose the precision your calculation needs
rather than using them solely to turn off Mathish mode.

[Static kinds](numbers.qmd#static-kinds) disable Mathish conventions at
every width. They also change integer overflow and results outside a
function’s real domain. Use them when those rules suit the calculation,
rather than to change comparison alone.
