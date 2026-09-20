# Shared language rules

[Glyph index](index.md#glyph-reference)

## Reading expressions

Within an expression, functions take everything to their right as the right argument. Parentheses override grouping; arithmetic has no precedence. Monads take one argument, dyads two.

```apl
2×3+4                ⍝ 14
(2×3)+4              ⍝ 10
10-3-2               ⍝ 9
```

Operators bind before function application. Names hold values or operators, resolved at execution time. Arrays contain numbers, characters, functions and arrays.

`→` separates pipeline stages, evaluated left-to-right. Each stage uses ordinary APL binding; each function receives the previous result as its right argument. `←` encloses the whole pipeline.

```apl
total←1+2×3 → 2∘× ⋄ total   ⍝ 14
```

## Numbers

Bare numbers are approximate (`f64`). `x` marks exact integers; `r` marks exact rationals. Exact arithmetic grows as needed. Mixing exact and approximate gives approximate.

```apl
1÷3                  ⍝ 0.3333333333333333
1x÷3x                ⍝ 1r3
1r3+1r6              ⍝ 1r2
1r2+0.5              ⍝ 1
9223372036854775807x+1x ⍝ 9223372036854775808x
```

`ajb`: a + bi, with approximate components. `J` also parses. `¯` marks negative literals and exponents.

```apl
1j2+3j4              ⍝ 4j6
1E¯3                 ⍝ 0.001
```

Predicates, positions, tally and shape return exact integers. Iota and random integer generation preserve exact arguments. Mixed arrays retain each element's domain.

Reals include `∞` and `¯∞`. DOMAIN: NaN, non-finite complex components, undefined infinity arithmetic, `⍟0`, division by zero except `0÷0=1`.

## Arrays, nesting and fill

Numbers, characters and functions are atoms. Arrays are rectangular collections of values. Shape lists axis lengths; rank is shape's length. Scalars, vectors and matrices are arrays of rank 0, 1 and 2. A scalar is distinct from an atom. Atoms have rank zero for shape operations. Constructors such as Enclose, Ravel and Reshape create arrays. Ravel order is row-major. Zero dimensions retain the other dimensions.

```apl
⍴3                   ⍝ 0⍴0x
⍴,3                  ⍝ ,1x
⍴2 3⍴⍳6              ⍝ 2x 3x
⍴0 3⍴0               ⍝ 0x 3x
```

Adjacent values form a strand, preserving every value. `⊂` encloses; `↑` retrieves the first item. Enclosure always adds an array layer.

```apl
↑(1 2)(3 4)          ⍝ 1 2
(⊂3)≡3              ⍝ 0x
(⊂3)=3              ⍝ ⊂1x
(⊂3)+4              ⍝ ⊂7
≢¨(1 2 3)(4 5)       ⍝ 3x 2x
```

Fill follows the first item's prototype: zero, space, or recursively filled nesting. Empty arrays retain a prototype. Assembly pads unequal cells with fill.

```apl
5↑1 2                ⍝ 1 2 0 0 0
3↑''                 ⍝ '   '
↑0⍴(1 2⋄ 3 4 5)      ⍝ 0 0
```

## Agreement and pervasion

Scalar functions align leading axes. Missing trailing dimensions count as 1. Equal dimensions agree; unit dimensions expand, including to zero. Unit axes remain. Other mismatches: LENGTH.

```apl
(2 3⍴⍳6)+10 20       ⍝ 2 3⍴11 12 13 24 25 26
(2 1⍴10 20)+1 3⍴1 2 3 ⍝ 2 3⍴11 12 13 21 22 23
⍴(1 1⍴10)+1 2 3      ⍝ 3x 1x
```

Pervasion repeats these rules inside nested items. Each and rank frames also use leading agreement. Products, replication, indexing and assignment have their own rules.

This extends APL agreement. NumPy aligns trailing axes.

## Axes and indices

Indices and axes start at 1. Counts and indices must be exactly integral, without tolerance. `m[;2]` selects column 2; `m[2;]` selects row 2.

Axis qualifiers attach to functions: `+[2]`, `+/[1]`. `/ \ ⌽ ,` default to the last axis; `⌿ ⍀ ⊖ ⍪` to the first. Structural functions also accept axis lists or fractional insertion positions: `0.5` before axis 1, `1.5` between axes 1 and 2.

## Equality and ordering

Exact/exact comparison is exact. Approximate comparison uses relative tolerance `1E¯14`. Infinity equals itself, never a finite number.

```apl
0.3=0.1+0.2          ⍝ 1x
1r3=1x÷3x            ⍝ 1x
```

Search, membership, match and grouping use tolerant comparison. Tolerance is not transitive; search/grouping uses the first matching representative.

Grade and interval index ignore tolerance. Order: numbers, characters, nested arrays. Numbers compare by value; complex numbers by real then imaginary part; characters by code point. Nested arrays compare rank, then ravel lexicographically, then shape, ignoring prototypes. Grade is stable. Scalar ordering (`< ≤ > ≥ ⌊ ⌈`) requires reals.

## Functions and effects

Dfns use lexical scope. Plain assignment is local. Modified/indexed/selective updates target the nearest binding. Arrays have value semantics: updating one name leaves other copies unchanged.

```apl
a←1 2 ⋄ b←a ⋄ a[1]←9 ⋄ b ⍝ 1 2
```

A dfn returns its first result-producing non-assignment expression, or its final assignment silently. Top-level assignments also retain their value without display.

Evaluation is right-to-left, including fork arms. Separator-based literal elements evaluate left-to-right.

Empty Each/rank calls the operand on prototypes. Empty scan makes no calls. Generic reduction associates right; float sum/product may reassociate. All scans accumulate left-to-right.

## Errors

| Error | Meaning |
|---|---|
| DOMAIN | Invalid values or unknown inverse |
| RANK / LENGTH | Invalid rank or shape/count agreement |
| INDEX | Invalid position |
| SYNTAX | Invalid syntax or call form |
| VALUE | Undefined name or missing value |

Errors retain source locations. Unsupported features and resource limits have separate errors.
