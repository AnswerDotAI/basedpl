# miniapl

A small modern APL calculator in Rust, aimed at teaching mathematics. Nested lexical functions and a small guard/unwinding slice are also implemented. This is a documented subset, not a complete APL implementation.

## Run

```bash
miniapl -e 'v←⍳10 ⋄ sum←+/ ⋄ sum v' # 55
miniapl -e 'avg←+/÷≢ ⋄ avg 2 4 9'   # 5
miniapl -e 'add←{⍺+⍵} ⋄ add/1 2 3'  # 6
miniapl lesson.apl                  # source file
miniapl                             # persistent REPL
```

The installed command follows exhash: a maturin console script delegates to the Rust CLI. For a Python-independent executable, use `cargo build --release` or `cargo run -- -e '+/⍳10'`. Build profiles and workspace integration follow fastship/fastws; no separate launcher or binary-in-wheel machinery is needed.

The REPL continues unclosed delimiters across lines, retains names, and recovers after errors. Dyalog 20 array notation treats newlines inside parentheses/brackets as element/row separators, not as whitespace within an unfinished expression. Piped input uses the same session without prompts; `miniapl -` executes all of stdin as one source. Batch errors exit with status 1; bad command arguments exit with status 2. Output already produced is preserved.

### Typing APL symbols

In the interactive terminal, type a **backtick followed by a symbol name** (or a unique prefix). Tab replaces it; a non-letter replaces it and also enters that character. For example:

| Type | Becomes |
| --- | --- |
| `` `io `` then Tab | `⍳` |
| `` `iota5 `` | `⍳5` |
| `` 2`times3 `` | `2×3` |
| `` v`assign `` then Space | `v← ` |
| `` `scan `` then Tab | `\` |

Names are case-insensitive. Exact names win over longer names (`scan` versus `scanfirst`); ambiguous/unknown prefixes are never guessed. Matching names appear as you type; press Tab twice to list ambiguous choices, or backtick then Tab twice to browse the catalogue. Enter also accepts a unique name and submits the line, showing the accepted glyph after an arrow.

Arrow keys edit and recall this session's history. Ctrl-C cancels the current input, including unfinished multiline expressions; Ctrl-D exits on a fresh line. Expansion is disabled in strings/comments and for bracketed pastes. This is an input method, not APL syntax: files, pipes, `-e`, Python and JSON use actual glyphs. The catalogue includes planned primitives as well as those implemented below.

```text
$ miniapl -e '¯2+1÷0'
DOMAIN ERROR: division by zero
 --> <expression>:1:5
¯2+1÷0
    ^
```

## Python

```python
from miniapl import Session, AplError

with Session() as s:
    s.eval('v←⍳5')
    r = s.eval('+/v')
    assert r.value.shape == ()
    assert r.value.to_python() == 15
    assert r.output == ['15']
```

`Result.value` is a copied `Array` (shape, flat data, prototype), or `None`. `to_python()` returns ordinary numbers/lists and intentionally loses prototype information. Values survive session closure and can be used on other threads. NumPy is neither required nor returned automatically. `AplError` exposes kind, message, retained source, UTF-8 byte span, call-site context (`calls`), and output produced before failure.

A final assignment returns its array without printing it. A final function/operator definition or empty input returns no array, not the preceding statement's value. Evaluations are not transactions: assignments completed before a runtime error remain in the session.

**Sessions are thread-affine:** create, use, and close them on the same thread. Prefer `with Session()` or explicit `close()`. Cross-thread calls are rejected. Do not transfer an open session to another thread for destruction: PyO3's `unsendable` safeguard reports an unraisable error and skips native destruction in that unsupported case. Closing on the owner thread releases the native session first. This binding restriction does not change Rust ownership or add workers/locks to the interpreter.

## JSON-lines process interface

Run `miniapl --json` and send one JSON string per line:

```json
"v←⍳10"
"+/v"
```

Encode requests with `json.dumps(code)` in Python or `JSON.stringify(code)` in JavaScript, followed by a newline. The surrounding quotes are required; there is no request object. Each request receives a flushed response before the next request is read. The second response is equivalent to:

```json
{"value":{"shape":[],"data":[55.0],"prototype":0.0},"output":["55"],"error":null}
```

State persists. JSON/APL errors return structured errors and do not terminate the process. Stdout contains only protocol lines. Newlines in APL source must be escaped inside the JSON string. EOF ends the process normally. Values preserve shape, flat data, and prototype; ordinary floats are JSON numbers, exact values are `{"rational":["numerator","denominator"]}` with decimal integer strings (including denominator `"1"`), non-real complex values are `{"complex":[real,imaginary]}` with numeric components, characters are strings, and nested elements use the same array object form. Error spans are byte ranges into their accompanying source text; `calls` lists enclosing defined-function call sites without replacing the originating span. There is no streaming, multiplexing, Jupyter dependency, or Python round-trip.

## Explicit exact arithmetic

Ordinary literals and arithmetic stay floating-point. Use `x` for an exact integer or `r` for a fraction; exact integers are stored as rationals with denominator 1.

```apl
(1÷3)+(1÷6)             ⍝ float 0.5
1r3+1r6                 ⍝ exact 1r2
1x÷3x                   ⍝ exact 1r3
6x÷3x                   ⍝ exact 2x (2r1 is also accepted)
1x÷3                    ⍝ float: approximate operands win
9007199254740993x-9007199254740992x  ⍝ exact 1x
0.3=0.1+0.2             ⍝ 1: fixed float comparison tolerance
```

Exact fractions are reduced, denominators positive, and integer results display with `x`. Only integer components are accepted in `x`/`r` literals; use `¯` for signs. An exact value too large for a required float conversion gives a domain error. Merely placing exact and approximate values in one array does not convert them. Empty prototypes and reduction identities retain the numeric domain. Iota, shape, tally, and Boolean results are ordinary floats; passing an exact count does not make generated values exact.

Python receives independent `int` values for exact integers, `fractions.Fraction` for non-integer exact values, and `float` for ordinary real numbers. JSON uses the rational representation above to avoid client-side rounding.

## Complex arithmetic

Use `ajb` (or `aJb`) for a complex number with real part `a` and imaginary part `b`. Display uses lowercase `j`. Both components accept the ordinary decimal/exponent notation and must be finite.

```apl
(1j2)+(3j4)          ⍝ 4j6
(1j2)×(1j¯2)         ⍝ 5
+1j2                 ⍝ 1j¯2: monadic + is conjugation
÷1j2                 ⍝ 0.2j¯0.4
×3j4                 ⍝ 0.6j0.8: direction, with magnitude 1
1r2+1j2              ⍝ 1.5j2: complex arithmetic is approximate
sum←+/ ⋄ sum 1j2 3j4 ⍝ 4j6
```

Complex values use two `f64` components through `num-complex`. Promotion is exact → float → complex; array construction alone never promotes adjacent elements. An exactly zero imaginary component normalizes to an ordinary float, not an exact rational; a small nonzero component is not rounded away. Complex-derived numeric fill and empty-reduction identities are therefore ordinary floats.

Python receives copied native `complex` values; JSON uses the tagged pair above. Real-normalized results such as the conjugate product return a Python float/JSON number. Equality and inequality use the same fixed `1e-14` tolerance with complex magnitudes, following [Dyalog's equality rule](https://docs.dyalog.com/20.0/language-reference-guide/primitive-functions/equal-to/), rather than testing components separately. Ordering and structural counts require real values; tolerant coercion of near-real complex values is not implemented. Powers, roots, logarithms and circle functions support complex arguments. For example, `¯1*0.5` gives `0j1` and `1+*○0j1` gives `0`.

## Implemented subset and limits

```apl
⊃0⍴(1 2)(3 4 5)       ⍝ 0 0: structured empty prototype
3↑''                    ⍝ three spaces
m←[1 2 3 ⋄ 4 5 6]      ⍝ a matrix; 2 3⍴⍳6 is equivalent
+/m                     ⍝ 6 15: row sums
+⌿m                     ⍝ 5 7 9: column sums
↑(1 2)(3 4 5)           ⍝ mix/pad unequal rows
(42 ⋄)                  ⍝ singleton vector
[1 2 ⋄ 3]               ⍝ padded matrix: 1 2 / 3 0
```

Character literals use single quotes, with doubled quotes inside (`'can''t'`). One character is a scalar; comma makes it a singleton vector. `⊂` encloses and `⊃` discloses. `⍬` is the numeric empty vector. Arithmetic extends recursively through nested arrays. Parenthesized literals preserve nesting; bracket literals assemble rows/cells with fill. Literal elements evaluate left-to-right.

- Right-to-left evaluation; numeric vectors/stranding; names and assignment; `⋄`/newline statements; parentheses; `⍝` comments; explicit `⎕←` output.
- Monadic/dyadic `+ - × ÷`, scalar extension and shape agreement; monadic `⍳ ⍴ ≢ ,`; general reshape, take/drop, enclosure/disclosure, Mix/Split, catenate/table, reverse/rotate, and transpose (including diagonal axes).
- Numeric dyadic `= ≠ < ≤ > ≥`: exact/exact comparisons are exact; comparisons involving floats use fixed Dyalog-style relative tolerance `1e-14`. No configurable `⎕CT`.
- Scalar maths: `| ⌊ ⌈ * ⍟ ○ ! ∨ ∧ ⍲ ⍱ ~`, including exact rational operations where closed in that domain. Roll/deal `?` uses an unseeded generator; `?0` lies strictly between 0 and 1.
- Search and sets: match/depth, not-match, membership/enlist, index-of, where/interval-index, nub sieve, unique/union/intersection/without and find. Tolerance-sensitive searches share numeric comparison. Interval-index and min/max use ordinary ordering.
- Last/first-axis reduction and scan (`/ ⌿ \ ⍀`) support primitive and user-defined operands. Generic reduction is right-associated: `-/1 2 3` is 2. Primitive float `+/` and `×/` permit compiler reassociation for speed; their grouping is unspecified, including with axes. Cancellation, rounding and overflow can differ from Dyalog or across builds. For example, `+/1E100 ¯1E100 1` has no promised fixed-order answer. Exact arithmetic remains exact. Generic scan reduces successive prefixes; direct numeric primitive sum/product scans accumulate left-to-right. N-wise reduction accepts zero and negative widths. Empty reductions use known primitive identities; unknown identities give domain errors. Exact empty min/max currently errors because there is no finite rational identity. Replication/expansion support matrix axes and signed counts for fill.
- Bracket indexing uses origin 1: `m[2;1]`, `m[;2]`, `v[3 1]`. Coordinate arrays support `m[⍳⍴m]`. Single-axis qualifiers work on reductions/scans, replication/expansion, reverse/rotate, catenate and take/drop: `+/[1]m`. Indexed assignment, nested-path indexing, fractional and multi-axis qualifiers are not implemented.
- `/` retains its hybrid role when named or parenthesized: `r←/ ⋄ +r 1 2 3` gives 6, while `1 0 1 r 2 4 6` gives `2 6`. Replication extends singleton vectors as well as scalars: `(,2)/3 4` gives `3 3 4 4`, and `1 0 1/,3` gives `3 3`. Scalar arithmetic also supports singleton extension.
- Dfns with local assignments, nested lexical definitions, lazy default arguments (`⍺←2`), and silent/no-result behavior. Monadic/dyadic dops accept function, array or hybrid operands. For example, `apply←{⍺⍺ ⍵} ⋄ (-apply)3` gives ¯3, and `op←{⍺⍺+⍵⍵×⍵} ⋄ (2 op 3)4` gives 14.
- Atops, forks, constant arms and longer trains; `⊣ ⊢`; composition/binding `∘`, rank/atop `⍤`, over `⍥`, behind `⍛`, Each `¨` and commute/constant `⍨`. `3∘<⍛/2 7 1 8` gives `7 8`. Rank uses shared padded cell assembly. Its frames must match unless one is scalar. Empty Each/rank invokes the operand to obtain a prototype, including its output/errors.
- Inner and outer products use shared operand calls: `1 2 3+.×10 12 14` gives 76; `(⍳3)∘.=⍳3` is an identity matrix. Singleton contraction extension, nested results and empty products are supported.
- Key `⌸` groups by the first matching representative. Counted and predicate power `⍣` use shared function calls; negative powers remain unimplemented.
- Grade `⍋ ⍒` is stable, with untoleranced numeric ordering, complex and nested values, and dyadic character collation. Pick `⊃` follows nested coordinate paths. Nest `⊆`, partition `⊆` and partitioned enclosure `⊂` support typed empties; both partitions accept axis qualifiers.
- Squad `⌷` uses the bracket-selection path, including shaped indices, missing trailing axes and a single-axis qualifier. Monadic `⌷` returns its array unchanged.
- Names follow lexical nesting, not the dynamic caller. Recursive and mutually referring definitions work. Boolean guards allow `fact←{⍵=0:1 ⋄ ⍵×∇⍵-1}`; `fact 6` gives 720.
- Catch-all `0::` and numbered error guards (`11::`, `6 11::`) restore bindings to when the guard was installed, after evaluating its condition. Later assignments are undone, including newly introduced locals. The selected guard is inactive in its handler; earlier guards can catch handler failures. Output is not rolled back. Interrupt handling is not implemented. Unsupported features are deliberately not caught by guards.
- Functions share immutable nodes rather than copying their trees. Lexical links refer to active stack-owned frames; functions cannot escape via array results, local assignment, or the public API. This is not a claim about future namespace/export features. Definitions retain full source spans; errors retain their origin and defined-function call sites.
- Direct dfn/dop tail calls run in a loop, including mutual recursion, selected guards and parenthesized returns. Needed lexical frames are retained. Installed error guards disable tail-frame reuse. Calls embedded in further computation remain depth-limited. Function-valued dfn results are syntax errors, as in Dyalog. Mutable namespaces and function exports are outside the current calculator scope. Inverses, at, stencil, matrix division, encode/decode and formatting remain planned.
- Fixed 1-based iota. Counts must be integral and nonnegative; generated arrays are capped at 1,000,000 elements. Array nesting, syntax nesting and function-graph depth have a 128-level limit. Combined non-tail evaluation/call nesting and retained lexical frames are limited to 64. Flat binding, assignment chains and supported tail recursion are iterative. Diagnostic carets use Unicode display width; tabs render at four-column stops.
- Ordinary finite real `f64` numbers: `42`, `1.5`, `.5`, `¯2`, `1E¯3` (`e` also works). `-` is a function; use `¯` inside literals. Integers through ±2⁵³ are representable exactly; larger ordinary literals can round. Decimal arithmetic is not generally exact. Underflow rounds, potentially to zero; overflow/non-finite literals are domain errors. Negative zero is normalized. Opt into arbitrary-precision exact reals with `x`/`r` as above.
- Division follows Dyalog's default `⎕DIV=0`: `0÷0` is 1; other zero divisors and `÷0` error. See [Dyalog's division contract](https://docs.dyalog.com/20.0/language-reference-guide/system-functions/div/). No configurable system variable is added.

The shared Rust array representation preserves shaped/nested values and typed empties. Homogeneous float arrays use contiguous `f64` buffers; other arrays store tagged elements. Matrices display in rows; nested display is currently a simple parenthesized format rather than Dyalog's box drawing. Core values, evaluation, and ownership contain no Python or JSON types.

## Development

```bash
cargo test
cargo fastfmt
maturin develop
pytest -q
```

Maturin enables the optional PyO3 adapter. Rebuild it after Rust changes before checking the installed command or Python API. See [DEV.md](DEV.md) for architecture, build, and release conventions.

The [reference acceptance corpus](tests/reference/README.md) retains ngn, April and APLcart examples for gradual activation. Dyalog documentation examples are added with independently captured structured expectations as glyphs are implemented. Supported cases run in ordinary Rust tests. Pending and scope-question entries remain visible rather than being discarded. The initial character-conjugation, nested-scalar-split and strand/index binding defects are fixed.
