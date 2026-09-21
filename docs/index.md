# bAsedPL

bAsedPL (aka "Based-array APL") is an APL-derived language that uses *based arrays*. It emphasizes simple, consistent notation, borrowing ideas from J and BQN.

Based arrays, named in a [1981 paper](https://dl.acm.org/doi/abs/10.1145/586656.586663) and popularized by [BQN](https://mlochbaum.github.io/BQN/doc/based.html), treats numbers, characters and functions as *atoms*, which are collected and shaped in *arrays*. The atom `3` is distinct from the scalar (rank-0 array) `⊂3`.

Work with whole arrays, define functions, and combine them with operators, through an interactive terminal or Python API. Numbers include approximate reals, complex numbers and exact rationals.

```apl
+/⍳10                  ⍝ 55
avg←+/÷≢ ⋄ avg 2 4 9   ⍝ 5
1r3+1r6                ⍝ 1r2
```

bAsedPL distinguishes two application rules. Structural mapping (arithmetic, Each and indexing) preserves the mapped container, including scalars. Cell application (Rank and search) consumes complete cells, returning one result directly or assembling results over surrounding batch axes. See the [language rules](rules.md#arrays-nesting-and-fill) for the value model and examples.

## Using bAsedPL

- [Getting started](getting-started.md): install and try a calculation.
- [REPL](repl.md): type glyphs, edit input and display arrays/functions.
- [Command line](cli.md): expressions, files and pipes.
- [Python](python.md): sessions, arrays, NumPy and callable APL functions.
- [Process interfaces](processes.md): JSON lines and interruptible workers.
- [Language rules](rules.md): evaluation, numbers, agreement, axes and fill.

## Glyph reference

Each name below links to its definitions and examples. Examples show an equivalent APL value after `⍝`. A monad takes one argument; a dyad takes two. Operators take functions or arrays and derive functions. Notes give glyph-specific differences from Dyalog APL.

## Functions

| Glyph | Monad | Dyad | Note |
|---|---|---|---|
| `+` [Plus](glyphs/plus.md) | Conjugate | Add | Character offsets as BQN: `'a'+3` is `'d'` |
| `-` [Minus](glyphs/minus.md) | Negate | Subtract | Character difference as BQN: `'d'-'a'` is `3x` |
| `×` [Times](glyphs/times.md) | Direction | Multiply | |
| `÷` [Divide](glyphs/divide.md) | Reciprocal | Divide | Exact rationals as J: `1x÷3x` is `1r3` |
| `⌊` [Floor](glyphs/floor.md) | Floor | Minimum | `⌊/⍬` is `∞` |
| `⌈` [Ceiling](glyphs/ceiling.md) | Ceiling | Maximum | `⌈/⍬` is `¯∞` |
| `\|` [Stile](glyphs/stile.md) | Magnitude | Residue | |
| `*` [Star](glyphs/star.md) | Exponential | Power | |
| `⍟` [Log](glyphs/log.md) | Natural log | Logarithm | |
| `○` [Circle](glyphs/circle.md) | Unit circle | Circular functions | Monad is `*0j1×Y`, a unit-circle point. Pi times is `π` |
| `π` [Pi](glyphs/pi.md) | Pi times | Pi fraction | Monad takes over Pi times from `○`. `XπY` is Xπ÷Y |
| `√` [Root](glyphs/root.md) | Square root | Nth root | As BQN's `√`, with complex results: `√¯4` is `0j2` |
| `!` [Factorial](glyphs/factorial.md) | Factorial | Binomial | |
| `ℙ` [Prime](glyphs/prime.md) | Nth prime (one-based) | Prime operations | As J's `p:` with the same codes, but one-based |
| `Ⓠ` [Factor](glyphs/factor.md) | Prime factors | Exponents / factor table | As J's `q:`, with `∞` and `¯∞` for J's `_` and `__` |
| `Ⓟ` [Polynomial](glyphs/polynomial.md) | Roots / coefficients | Evaluate | As J's `p.`: coefficients are constant-first |
| `?` [Question](glyphs/question.md) | Roll | Deal | |
| `∨` [Or](glyphs/or.md) | Real / imaginary parts | OR / GCD | Monad as J's `+.` |
| `∧` [And](glyphs/and.md) | Magnitude / angle | AND / LCM | Monad as J's `*.` |
| `⍲` [Nand](glyphs/nand.md) | Square | NAND | Monad as J's `*:` |
| `⍱` [Nor](glyphs/nor.md) | Double | NOR | Monad as J's `+:` |
| `~` [Tilde](glyphs/tilde.md) | NOT | Without | |
| `=` [Equal](glyphs/equal.md) | Self-classify | Equal | Monad as J: one mask row per distinct major cell |
| `≠` [Not equal](glyphs/not-equal.md) | Unique mask | Not equal | |
| `<` [Less](glyphs/less.md) | — | Less | |
| `≤` [Less or equal](glyphs/less-or-equal.md) | Decrement | Less or equal | Monad as J's `<:` |
| `>` [Greater](glyphs/greater.md) | — | Greater | |
| `≥` [Greater or equal](glyphs/greater-or-equal.md) | Increment | Greater or equal | Monad as J's `>:` |
| `≡` [Match](glyphs/match.md) | Depth | Match | |
| `≢` [Tally](glyphs/tally.md) | Tally | Not match | |
| `⍴` [Rho](glyphs/rho.md) | Shape | Reshape | `⍬⍴Y` gives a rank-0 array: `⍬⍴1 2` is `⊂1` |
| `,` [Comma](glyphs/comma.md) | Ravel | Catenate | |
| `⍪` [Table](glyphs/table.md) | Table | Catenate first | |
| `⌽` [Reverse](glyphs/reverse.md) | Reverse last | Rotate last | |
| `⊖` [Reverse first](glyphs/reverse-first.md) | Reverse first | Rotate first | |
| `⍉` [Transpose](glyphs/transpose.md) | Transpose | Reorder / diagonal axes | |
| `↑` [Take](glyphs/take.md) | First | Take | Monad as Dyalog `⎕ML≥2`, but takes a major cell: `↑2 3⍴⍳6` is `1 2 3` |
| `↓` [Drop](glyphs/drop.md) | Split | Drop | |
| `↕` [Windows](glyphs/windows.md) | — | Full leading-axis windows | As BQN's `↕`. Replaces windowed reduce: `+/2↕1 2 3 4` is `3 5 7` |
| `⊂` [Enclose](glyphs/enclose.md) | Enclose | Partitioned enclose | Encloses atoms too, as BQN: `(⊂3)≡3` is `0x` |
| `⊆` [Nest](glyphs/nest.md) | Nest | Partition | |
| `⊃` [Mix](glyphs/mix.md) | Mix | Pick | Monad as Dyalog `⎕ML≥2`. Pick takes cells: `2⊃2 3⍴⍳6` is `4 5 6` |
| `⌷` [Squad](glyphs/squad.md) | Identity | Index | |
| `⍳` [Iota](glyphs/iota.md) | Index generator | Index of | |
| `⍸` [Where](glyphs/where.md) | Where | Interval index | |
| `∊` [Member](glyphs/member.md) | Enlist | Membership | |
| `∪` [Union](glyphs/union.md) | Unique | Union | |
| `∩` [Intersection](glyphs/intersection.md) | — | Intersection | |
| `⍷` [Find](glyphs/find.md) | — | Find | |
| `⍋` [Grade up](glyphs/grade-up.md) | Grade up | Grade with collation | |
| `⍒` [Grade down](glyphs/grade-down.md) | Grade down | Grade with collation | |
| `/` [Slash](glyphs/slash.md) | — | Replicate last | |
| `⌿` [Slash bar](glyphs/slash-bar.md) | — | Replicate first | |
| `\` [Backslash](glyphs/backslash.md) | — | Expand last | |
| `⍀` [Backslash bar](glyphs/backslash-bar.md) | — | Expand first | |
| `⊤` [Encode](glyphs/encode.md) | Binary encode | Encode | Monad as J's `#:`, but digits run along the first axis |
| `⊥` [Decode](glyphs/decode.md) | Binary decode | Decode | Monad as J's `#.`, but digits run along the first axis |
| `⌹` [Domino](glyphs/domino.md) | Matrix inverse | Matrix divide | |
| `⊣` [Left](glyphs/left.md) | Identity | Left | |
| `⊢` [Right](glyphs/right.md) | Identity | Right | |
| `⍎` [Execute](glyphs/execute.md) | Execute | Execute in current scope (`''⍎Y`) | |
| `⍕` [Format](glyphs/format.md) | Format | Format by specification | |

## Operators

`f`, `g` are functions; `a`, `n`, `r` are arrays or numbers.

| Glyph | Form | Meaning | Note |
|---|---|---|---|
| `/` [Slash](glyphs/slash.md) | `f/` | Reduce / seeded reduce last | Seed as BQN's `´`: `10 -/1 2 3` is `¯8`. For windows use `↕` |
| `⌿` [Slash bar](glyphs/slash-bar.md) | `f⌿` | Reduce / seeded reduce first | Seed and windows as `/` |
| `\` [Backslash](glyphs/backslash.md) | `f\` | Scan / seeded scan last | Accumulates left to right as BQN's Scan: `-\1 2 3` is `1 ¯1 ¯4` |
| `⍀` [Backslash bar](glyphs/backslash-bar.md) | `f⍀` | Scan / seeded scan first | Left to right as `\` |
| `¨` [Each](glyphs/each.md) | `f¨` | Each | |
| `⍨` [Commute](glyphs/commute.md) | `f⍨`, `a⍨` | Self / commute / constant | |
| `∘` [Compose](glyphs/compose.md) | `f∘g`, `a∘f`, `f∘a` | Compose / bind | |
| `⍤` [Rank](glyphs/rank.md) | `f⍤g`, `f⍤r` | Atop / rank | |
| `⍥` [Over](glyphs/over.md) | `f⍥g` | Over | |
| `⍛` [Behind](glyphs/behind.md) | `f⍛g` | Behind | |
| `.` [Dot](glyphs/dot.md) | `f.g` | Inner product | Outer product is `g⌝` |
| `⌝` [Outer product](glyphs/outer-product.md) | `g⌝` | Outer product | As BQN's `⌜`; `∘.g` is a SYNTAX ERROR |
| `⌸` [Key](glyphs/key.md) | `f⌸` | Key | |
| `⍣` [Power](glyphs/power.md) | `f⍣n`, `f⍣g`, `f⍣[p]` | Iterate / invert / repeat until; enclose count or predicate for history | Array counts as J's `^:`: `(1∘+)⍣3 ¯2 0 3⊢10` is `13 8 10 13` |
| `⇄` [Inverse pair](glyphs/inverse-pair.md) | `f⇄g` | Attach an explicit inverse | As J's `:.` |
| `⌾` [Under](glyphs/under.md) | `f⌾g` | Transform, apply, inverse-transform | As J's `&.` |
| `∂` [Derivative](glyphs/derivative.md) | `f∂` | Gradient / vector–Jacobian product | |
| `◶` [Agenda](glyphs/agenda.md) | `selector◶cases` | Select and call one function | As BQN's `◶`, but one-based |
| `@` [At](glyphs/at.md) | `f@a`, `a@g` | Functional amend | |
| `⌺` [Stencil](glyphs/stencil.md) | `f⌺a` | Stencil | |

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
| `∇` [Del](glyphs/del.md), `⍢` [Del diaeresis](glyphs/del-diaeresis.md) | Function / operator self-reference |
| `:` [Colon](glyphs/colon.md), `::` [Error guard](glyphs/error-guard.md) | Boolean / error guard |
| `⋄` [Diamond](glyphs/diamond.md) | Statement or literal separator |
| `⍝` [Comment](glyphs/comment.md) | Comment |
| `⎕←` [Quad](glyphs/quad.md) | Explicit output |
| `•` [Bullet](glyphs/bullet.md) | [System name](#system-names) prefix |
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
| `•SIGNAL` [Signal](glyphs/error-guard.md#signal) | Raise an ordinary APL error |

Index origin: 1. Comparison tolerance: `1E¯14`.

## Language-wide differences from Dyalog APL

- [Atoms](rules.md#arrays-nesting-and-fill): numbers, characters and functions are atoms, distinct from scalars, as BQN. Enclosing always adds a layer: `(⊂3)≡3` is `0x`.
- [Numbers](rules.md#numbers): bare numbers are approximate. `x` and `r` mark exact integers and rationals, as J: `1x÷3x` is `1r3`. Predicates, positions, tally and shape are exact.
- [Agreement](rules.md#agreement-and-pervasion): scalar functions, Each and Rank align leading axes, and unit dimensions expand: `(2 3⍴⍳6)+10 20` is `2 3⍴11 12 13 24 25 26`.
- [System names](#system-names): written `•NAME`, as BQN, not `⎕NAME`. Most Dyalog system functions and variables are not included.
- Namespaces: none.
