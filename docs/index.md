# bAsedPL

bAsedPL is an APL-derived language that uses *based arrays*. It emphasizes simple, consistent notation, borrowing ideas from J and BQN.

Based arrays, named in a [1981 paper](https://dl.acm.org/doi/abs/10.1145/586656.586663) and popularized by [BQN](https://mlochbaum.github.io/BQN/doc/based.html), treats numbers, characters and functions as *atoms*, which are collected and shaped in *arrays*. The atom `3` is distinct from the scalar (rank-0 array) `⊂3`.

Work with whole arrays, define functions, and combine them with operators, through an interactive terminal or Python API. Numbers include approximate reals, complex numbers and exact rationals.

```apl
+/⍳10                  ⍝ 55
avg←+/÷≢ ⋄ avg 2 4 9   ⍝ 5
1r3+1r6                ⍝ 1r2
```

bAsedPL uses based arrays, named in a [1981 paper](https://dl.acm.org/doi/abs/10.1145/586656.586663) and popularized by [BQN](https://mlochbaum.github.io/BQN/doc/based.html). Numbers, characters and functions are atoms. Arrays are shaped collections of values: the atom `3` is distinct from the scalar (rank-0 array) `⊂3`. bAsedPL distinguishes two application rules. Structural mapping (arithmetic, Each and indexing) preserves the mapped container, including scalars. Cell application (Rank and search) consumes complete cells, returning one result directly or assembling results over surrounding batch axes. See the [language rules](rules.md#arrays-nesting-and-fill) for the value model and examples.

## Using bAsedPL

- [Getting started](getting-started.md): install and try a calculation.
- [REPL](repl.md): type glyphs, edit input and display arrays/functions.
- [Command line](cli.md): expressions, files and pipes.
- [Python](python.md): sessions, arrays, NumPy and callable APL functions.
- [Process interfaces](processes.md): JSON lines and interruptible workers.
- [Language rules](rules.md): evaluation, numbers, agreement, axes and fill.

## Glyph reference

Each name below links to its definitions and examples. Examples show an equivalent APL value after `⍝`. A monad takes one argument; a dyad takes two. Operators take functions or arrays and derive functions.

## Functions

| Glyph | Monad | Dyad |
|---|---|---|
| `+` [Plus](glyphs/plus.md) | Conjugate | Add |
| `-` [Minus](glyphs/minus.md) | Negate | Subtract |
| `×` [Times](glyphs/times.md) | Direction | Multiply |
| `÷` [Divide](glyphs/divide.md) | Reciprocal | Divide |
| `⌊` [Floor](glyphs/floor.md) | Floor | Minimum |
| `⌈` [Ceiling](glyphs/ceiling.md) | Ceiling | Maximum |
| `\|` [Stile](glyphs/stile.md) | Magnitude | Residue |
| `*` [Star](glyphs/star.md) | Exponential | Power |
| `⍟` [Log](glyphs/log.md) | Natural log | Logarithm |
| `○` [Circle](glyphs/circle.md) | Unit circle | Circular functions |
| `π` [Pi](glyphs/pi.md) | Pi times | Pi fraction |
| `√` [Root](glyphs/root.md) | Square root | Nth root |
| `!` [Factorial](glyphs/factorial.md) | Factorial | Binomial |
| `ℙ` [Prime](glyphs/prime.md) | Nth prime (one-based) | Prime operations |
| `Ⓠ` [Factor](glyphs/factor.md) | Prime factors | Exponents / factor table |
| `Ⓟ` [Polynomial](glyphs/polynomial.md) | Roots / coefficients | Evaluate |
| `?` [Question](glyphs/question.md) | Roll | Deal |
| `∨` [Or](glyphs/or.md) | Real / imaginary parts | OR / GCD |
| `∧` [And](glyphs/and.md) | Magnitude / angle | AND / LCM |
| `⍲` [Nand](glyphs/nand.md) | Square | NAND |
| `⍱` [Nor](glyphs/nor.md) | Double | NOR |
| `~` [Tilde](glyphs/tilde.md) | NOT | Without |
| `=` [Equal](glyphs/equal.md) | Self-classify | Equal |
| `≠` [Not equal](glyphs/not-equal.md) | Unique mask | Not equal |
| `<` [Less](glyphs/less.md) | — | Less |
| `≤` [Less or equal](glyphs/less-or-equal.md) | Decrement | Less or equal |
| `>` [Greater](glyphs/greater.md) | — | Greater |
| `≥` [Greater or equal](glyphs/greater-or-equal.md) | Increment | Greater or equal |
| `≡` [Match](glyphs/match.md) | Depth | Match |
| `≢` [Tally](glyphs/tally.md) | Tally | Not match |
| `⍴` [Rho](glyphs/rho.md) | Shape | Reshape |
| `,` [Comma](glyphs/comma.md) | Ravel | Catenate |
| `⍪` [Table](glyphs/table.md) | Table | Catenate first |
| `⌽` [Reverse](glyphs/reverse.md) | Reverse last | Rotate last |
| `⊖` [Reverse first](glyphs/reverse-first.md) | Reverse first | Rotate first |
| `⍉` [Transpose](glyphs/transpose.md) | Transpose | Reorder / diagonal axes |
| `↑` [Take](glyphs/take.md) | First | Take |
| `↓` [Drop](glyphs/drop.md) | Split | Drop |
| `↕` [Windows](glyphs/windows.md) | — | Full leading-axis windows |
| `⊂` [Enclose](glyphs/enclose.md) | Enclose | Partitioned enclose |
| `⊆` [Nest](glyphs/nest.md) | Nest | Partition |
| `⊃` [Mix](glyphs/mix.md) | Mix | Pick |
| `⌷` [Squad](glyphs/squad.md) | Identity | Index |
| `⍳` [Iota](glyphs/iota.md) | Index generator | Index of |
| `⍸` [Where](glyphs/where.md) | Where | Interval index |
| `∊` [Member](glyphs/member.md) | Enlist | Membership |
| `∪` [Union](glyphs/union.md) | Unique | Union |
| `∩` [Intersection](glyphs/intersection.md) | — | Intersection |
| `⍷` [Find](glyphs/find.md) | — | Find |
| `⍋` [Grade up](glyphs/grade-up.md) | Grade up | Grade with collation |
| `⍒` [Grade down](glyphs/grade-down.md) | Grade down | Grade with collation |
| `/` [Slash](glyphs/slash.md) | — | Replicate last |
| `⌿` [Slash bar](glyphs/slash-bar.md) | — | Replicate first |
| `\` [Backslash](glyphs/backslash.md) | — | Expand last |
| `⍀` [Backslash bar](glyphs/backslash-bar.md) | — | Expand first |
| `⊤` [Encode](glyphs/encode.md) | Binary encode | Encode |
| `⊥` [Decode](glyphs/decode.md) | Binary decode | Decode |
| `⌹` [Domino](glyphs/domino.md) | Matrix inverse | Matrix divide |
| `⊣` [Left](glyphs/left.md) | Identity | Left |
| `⊢` [Right](glyphs/right.md) | Identity | Right |
| `⍎` [Execute](glyphs/execute.md) | Execute | Execute in current scope (`''⍎Y`) |
| `⍕` [Format](glyphs/format.md) | Format | Format by specification |

## Operators

`f`, `g` are functions; `a`, `n`, `r` are arrays or numbers.

| Glyph | Form | Meaning |
|---|---|---|
| `/` [Slash](glyphs/slash.md) | `f/` | Reduce / seeded reduce last |
| `⌿` [Slash bar](glyphs/slash-bar.md) | `f⌿` | Reduce / seeded reduce first |
| `\` [Backslash](glyphs/backslash.md) | `f\` | Scan / seeded scan last |
| `⍀` [Backslash bar](glyphs/backslash-bar.md) | `f⍀` | Scan / seeded scan first |
| `¨` [Each](glyphs/each.md) | `f¨` | Each |
| `⍨` [Commute](glyphs/commute.md) | `f⍨`, `a⍨` | Self / commute / constant |
| `∘` [Compose](glyphs/compose.md) | `f∘g`, `a∘f`, `f∘a` | Compose / bind |
| `⍤` [Rank](glyphs/rank.md) | `f⍤g`, `f⍤r` | Atop / rank |
| `⍥` [Over](glyphs/over.md) | `f⍥g` | Over |
| `⍛` [Behind](glyphs/behind.md) | `f⍛g` | Behind |
| `.` [Dot](glyphs/dot.md) | `f.g` | Inner product |
| `⌝` [Outer](glyphs/outer.md) | `g⌝` | Outer product |
| `⌸` [Key](glyphs/key.md) | `f⌸` | Key |
| `⍣` [Power](glyphs/power.md) | `f⍣n`, `f⍣g`, `f⍣[p]` | Iterate / invert / repeat until; enclose count or predicate for history |
| `⇄` [Inverse pair](glyphs/inverse-pair.md) | `f⇄g` | Attach an explicit inverse |
| `⌾` [Under](glyphs/under.md) | `f⌾g` | Transform, apply, inverse-transform |
| `∂` [Derivative](glyphs/derivative.md) | `f∂` | Gradient / vector–Jacobian product |
| `◶` [Agenda](glyphs/agenda.md) | `selector◶cases` | Select and call one function |
| `@` [At](glyphs/at.md) | `f@a`, `a@g` | Functional amend |
| `⌺` [Stencil](glyphs/stencil.md) | `f⌺a` | Stencil |

## Syntax and literals

| Form | Meaning |
|---|---|
| `˘` [Strand](glyphs/strand.md) | Form a vector of values: `1˘+˘'abc'` |
| `←` [Assign](glyphs/assign.md) | Assignment, including modified/indexed/selective forms |
| `→` [Pipe](glyphs/pipe.md) | Left-to-right function application |
| `(…)` [Parentheses](glyphs/parentheses.md) | Grouping / nested array literals / trains |
| `[…]` [Brackets](glyphs/brackets.md) | Enclosure, array literals, indexing and axes |
| `;` [Semicolon](glyphs/semicolon.md) | Index-axis separator |
| `{…}` [Braces](glyphs/braces.md) | Defined function or operator |
| `⍺` [Alpha](glyphs/alpha.md), `⍵` [Omega](glyphs/omega.md) | Left / right argument |
| `⍶` [Alpha underbar](glyphs/alpha-underbar.md), `⍹` [Omega underbar](glyphs/omega-underbar.md) | Left / right operand |
| `∇` [Nabla](glyphs/nabla.md), `∇∇` [Nabla nabla](glyphs/nabla-nabla.md) | Function / operator self-reference |
| `:` [Colon](glyphs/colon.md), `::` [Error guard](glyphs/error-guard.md) | Boolean / error guard |
| `⋄` [Diamond](glyphs/diamond.md) | Statement or literal separator |
| `⍝` [Comment](glyphs/comment.md) | Comment |
| `⎕←` [Quad](glyphs/quad.md) | Explicit output |
| `⍬` [Zilde](glyphs/zilde.md) | Empty numeric vector |
| `'…'` [Quote](glyphs/quote.md) | Character literal |
| `¯` [Overbar](glyphs/overbar.md) | Negative literal sign |
| `∞` [Infinity](glyphs/infinity.md) | Real infinity |

Numeric notation `x`, `r`, `j`, `E`: see [numbers](rules.md#numbers).

## System names

Names are case-insensitive. `•A` and `•D` are constant arrays; the other names are functions, usable with operators and composition.

| Name | Meaning |
|---|---|
| `•A` [Alphabet](glyphs/alphabet.md) | Uppercase Latin alphabet |
| `•D` [Digits](glyphs/digits.md) | Decimal digits |
| `•C` [Case](glyphs/case.md) | Unicode case conversion |
| `•UCS` [Unicode](glyphs/unicode.md) | Unicode code points / encodings |
| `•LOAD` [Load](glyphs/load.md) | Evaluate an APL source file |

Index origin: 1. Comparison tolerance: `1E¯14`.
