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

### Array and function diagrams

Interactive sessions start with `]box on -style=max -trains=tree -fns=on`. Nested arrays show separate boxes, axis arrows and type markers. Empty arrays show prototypes with zero-axis markers. Type a function name to inspect its structure without calling it. Trees use labelled branches rather than Dyalog's horizontal layout.

Use `]box off` for plain output, `-trains=def` for function expressions, and `-fns=off` to leave output inside functions unboxed. `]Display expression` draws one array without changing the settings. `]box ?` reports settings. Other boxing styles are not implemented. Diagrams abbreviate large arrays/function graphs. Batch, Python and JSON sessions default to plain output and accept the same commands; array values and `⍕` are unaffected.

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

`value.to_numpy()` copies float arrays to `float64` ndarrays and float/complex mixtures to `complex128`. Shape is preserved, including scalars and empty dimensions. Exact numbers, characters and nested arrays raise `TypeError`, including typed empties. NumPy is imported only when this method is called; install it separately. The ndarray does not preserve the APL prototype or share storage with the APL value.

Use `s.set('data', Array.from_numpy(ndarray))` to import a numeric ndarray. Boolean and integer values become APL floats; integers that cannot be represented exactly are rejected. Float16/32/64 and complex64/128 are accepted. Shapes, including scalar/empty dimensions, are preserved; non-contiguous inputs are copied in logical row-major order. Non-finite values, wider floating types, strings, structured and object dtypes are rejected. `set()` accepts only `Array` values, including copied exact/nested APL results, and validates the binding name without executing source.

A final assignment returns its array without printing it. A final function/operator definition or empty input returns no array, not the preceding statement's value. Evaluations are not transactions: assignments completed before a runtime error remain in the session.

Assignments bind inside expressions: `1+a←3` returns 4 and stores 3. Modified assignment passes through its right argument. `a b+←3 4` updates the targets from left to right.

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

State persists. JSON/APL errors return structured errors and do not terminate the process. Stdout contains only protocol lines. Newlines in APL source must be escaped inside the JSON string. EOF ends the process normally. Values preserve shape, flat data, and prototype. Exact integers are arbitrary-sized JSON integers. Floats retain a decimal point or exponent. Fractions use `{"rational":["numerator","denominator"]}` with decimal integer strings. Non-real complex values use `{"complex":[real,imaginary]}`, characters are strings, and nested elements use the same array object form. JavaScript clients need a parser that preserves large integers; ordinary `JSON.parse` can round them. Error spans are byte ranges into their accompanying source text; `calls` lists enclosing defined-function call sites without replacing the originating span. There is no streaming, multiplexing, Jupyter dependency, or Python round-trip.

### Interruptible worker

Use `miniapl.worker.Worker` for a persistent subprocess with deadlines and interruption:

```python
from miniapl.worker import Worker
with Worker() as w:
    w.eval('v←⍳10')
    r = w.eval('+/v', timeout=2)  # seconds; returns the JSON response as a dict
    assert r['value']['data'] == [55]
```

Call `w.interrupt()` from another thread to interrupt its current evaluation. Cooperative cancellation returns `INTERRUPT` or `TIMEOUT`, retains captured output and leaves the session usable. Completed assignments survive; cancellation is not a transaction. APL error guards cannot catch cancellation. Ctrl-C sends the interrupt request, consumes its response within the grace period, then raises `KeyboardInterrupt`. The worker runs in a separate process session so the terminal does not kill it directly.

Evaluation checks cancellation in the binder, function calls and potentially long primitive loops. Individual native-library or big-integer operations are not preemptible. If a deadline exceeds its grace period (default 1 second), the client kills the worker and raises `TimeoutError`; that session is lost. Ctrl-C also kills the worker if its grace period expires. The client never retries an evaluation automatically. `w.diagnostics` retains recent process stderr.

The underlying `--worker` protocol uses JSON objects, one per line. Send `{"id":1,"code":"+/⍳10","timeout_ms":2000}` and receive `{"id":1,"result":...}`. Send `{"interrupt":1}` to cancel that request; interrupt messages have no reply. Use one outstanding evaluation per client. Stdout contains responses only. Malformed JSON or invalid IDs/timeouts terminate the worker. The simpler `--json` mode remains unchanged.

In-process Python sessions accept `s.eval(code, timeout=seconds)` but remain thread-affine. Rust callers use `Session::eval_with(code, EvalOptions { interrupt, timeout })`; clone its `InterruptHandle` to cancel from another thread. No Jupyter or MCP dependency enters the interpreter.

## Explicit exact arithmetic

Ordinary literals stay floating-point. Use `x` for an exact integer or `r` for a fraction. Small exact integers use `i64`; fractions and larger integers use `BigRational`. Integer arithmetic promotes overflow to BigRational without losing precision. Integral rational results normalize back to `i64` whenever they fit.

```apl
(1÷3)+(1÷6)             ⍝ float 0.5
1r3+1r6                 ⍝ exact 1r2
1x÷3x                   ⍝ exact 1r3
6x÷3x                   ⍝ exact 2x (2r1 is also accepted)
1x÷3                    ⍝ float: approximate operands win
9007199254740993x-9007199254740992x  ⍝ exact 1x
0.3=0.1+0.2             ⍝ 1x: fixed float comparison tolerance
avg←+/÷≢ ⋄ avg 1x 2x 4x ⍝ 7r3
```

Exact fractions are reduced, denominators positive, and exact integer results display with `x`. Only integer components are accepted in `x`/`r` literals; use `¯` for signs. An exact value too large for a required float conversion gives a domain error. Merely placing exact and approximate values in one array does not convert them. Empty prototypes and reduction identities retain the numeric domain. Predicates and positions produce exact integers. Iota, roll/deal, shape and tally preserve exactness when all numeric leaves of their arguments are exact; characters contribute no numeric domain, nested arrays contribute their leaves, and empties contribute their prototype. Character-only arrays have approximate shape/tally results. `?0` remains approximate.

Python receives independent `int` values for exact integers, `fractions.Fraction` for non-integer exact values, and `float` for ordinary real numbers. JSON uses the numeric encodings above.

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

### Argument agreement

Scalar functions, Each and rank frames use leading-aligned broadcasting with unit-axis expansion. Missing trailing dimensions count as 1. Equal dimensions agree; 1 expands to the other dimension, including 0; other mismatches give LENGTH ERROR. Scalar functions repeat this rule inside nested arrays. This differs from Dyalog and from NumPy's trailing alignment.

```apl
(2 3⍴⍳6)+10 20        ⍝ 11 12 13 / 24 25 26
(2 1⍴10 20)+1 3⍴1 2 3 ⍝ 11 12 13 / 21 22 23
(2 3⍴⍳6)+[2]10 20 30 ⍝ explicit column alignment
```

Unit axes are retained: `(1 1⍴10)+1 2 3` has shape `3 1`. APL's axis defaults are unchanged: `+/` sums rows of a matrix, `+⌿` sums down columns. Replication, products and assignment have their own agreement rules.

### Language coverage

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
- Bracket indexing uses origin 1: `m[2;1]`, `m[;2]`, `v[3 1]`. Coordinate arrays and nested paths are supported. Indexed, modified, strand and selective assignment work, including Each selectors and repeated updates. Axis qualifiers support folds, replication/expansion, reverse/rotate, catenate, take/drop, scalar functions, ravel, mix and enclosure. Fractional axes insert dimensions for ravel, mix and laminate; multi-axis selectors support take/drop, enclosure and squad.
- `/` retains its hybrid role when named or parenthesized: `r←/ ⋄ +r 1 2 3` gives 6, while `1 0 1 r 2 4 6` gives `2 6`. Replication extends singleton vectors as well as scalars: `(,2)/3 4` gives `3 3 4 4`, and `1 0 1/,3` gives `3 3`. Scalar arithmetic also supports singleton extension.
- Dfns with local assignments, nested lexical definitions, lazy default arguments (`⍺←2`), and silent/no-result behavior. Monadic/dyadic dops accept function, array or hybrid operands. For example, `apply←{⍺⍺ ⍵} ⋄ (-apply)3` gives ¯3, and `op←{⍺⍺+⍵⍵×⍵} ⋄ (2 op 3)4` gives 14.
- Atops, forks, constant arms and longer trains; `⊣ ⊢`; composition/binding `∘`, rank/atop `⍤`, over `⍥`, behind `⍛`, Each `¨` and commute/constant `⍨`. `3∘<⍛/2 7 1 8` gives `7 8`. Rank uses shared padded cell assembly and the frame agreement above. Empty Each/rank invokes the operand to obtain a prototype, including its output/errors.
- Inner and outer products use shared operand calls: `1 2 3+.×10 12 14` gives 76; `(⍳3)∘.=⍳3` is an identity matrix. Singleton contraction extension, nested results and empty products are supported.
- Key `⌸` groups by the first matching representative. Counted, predicate and inverse power `⍣` use shared function calls. Known inverses cover arithmetic bindings, powers/logs, circle codes ¯7 through 7, permutations/rotations, encode/decode, where, supported scans and compositions/Each/rank. Unknown inverses give DOMAIN ERROR; arbitrary dfn inversion is deferred.
- `⍎` executes character code in the current lexical scope, retaining output and defining source. `⍕` supports monadic text and dyadic numeric field specifications, including exact fixed-point formatting. Neither operation depends on REPL boxing settings.
- Grade `⍋ ⍒` is stable, with untoleranced numeric ordering, complex and nested values, and dyadic character collation. Pick `⊃` follows nested coordinate paths. Nest `⊆`, partition `⊆` and partitioned enclosure `⊂` support typed empties; both partitions accept axis qualifiers.
- Squad `⌷` uses the bracket-selection path, including shaped indices, missing trailing axes and a single-axis qualifier. Monadic `⌷` returns its array unchanged.
- Encode/decode `⊤ ⊥` support numeric arrays, mixed bases, exact arithmetic and complex values. At `@` accepts replacement arrays or functions, indices or masks, and nested paths. Stencil `⌺` supports window sizes, movements, signed padding arguments and shared result assembly.
- Matrix inverse/divide `⌹` uses rational elimination for all-exact inputs and faer thin SVD otherwise. Rectangular full-column-rank inputs support least squares. Singular and underdetermined systems error. Numerical rank uses machine epsilon × max dimension × largest singular value, not comparison tolerance.
- Names follow lexical nesting, not the dynamic caller. Recursive and mutually referring definitions work. Boolean guards allow `fact←{⍵=0:1 ⋄ ⍵×∇⍵-1}`; `fact 6` gives 720.
- Catch-all `0::` and numbered error guards (`11::`, `6 11::`) restore bindings to when the guard was installed, after evaluating its condition. Later assignments are undone, including newly introduced locals. The selected guard is inactive in its handler; earlier guards can catch handler failures. Output is not rolled back. Cancellation and unsupported features are not caught by guards.
- Functions share immutable nodes rather than copying their trees. Lexical links refer to active stack-owned frames; functions cannot escape via array results, local assignment, or the public API. This is not a claim about future namespace/export features. Definitions retain full source spans; errors retain their origin and defined-function call sites.
- Direct dfn/dop tail calls run in a loop, including mutual recursion, selected guards and parenthesized returns. Needed lexical frames are retained. Installed error guards disable tail-frame reuse. Calls embedded in further computation remain depth-limited. Function-valued dfn results are syntax errors, as in Dyalog. Mutable namespaces and function exports are outside the current calculator scope.
- Fixed 1-based iota. Counts must be integral and nonnegative; generated arrays are capped at 1,000,000 elements. Array nesting, syntax nesting and function-graph depth have a 128-level limit. Combined non-tail evaluation/call nesting and retained lexical frames are limited to 64. Flat binding, assignment chains and supported tail recursion are iterative. Diagnostic carets use Unicode display width; tabs render at four-column stops.
- Ordinary finite real `f64` numbers: `42`, `1.5`, `.5`, `¯2`, `1E¯3` (`e` also works). `-` is a function; use `¯` inside literals. Integers through ±2⁵³ are representable exactly; larger ordinary literals can round. Decimal arithmetic is not generally exact. Underflow rounds, potentially to zero; overflow/non-finite literals are domain errors. Negative zero is normalized. Opt into arbitrary-precision exact reals with `x`/`r` as above.
- Division follows Dyalog's default `⎕DIV=0`: `0÷0` is 1; other zero divisors and `÷0` error. See [Dyalog's division contract](https://docs.dyalog.com/20.0/language-reference-guide/system-functions/div/). No configurable system variable is added.

The shared Rust array representation preserves shaped/nested values and typed empties. Homogeneous float and small exact integer arrays use contiguous `f64` and `i64` buffers. Other arrays store tagged elements. Compact fractional storage is not implemented. Matrices display in rows; nested arrays support boxed diagrams or plain parenthesized output. Core values, evaluation, and ownership contain no Python or JSON types.

## Development

```bash
cargo test
cargo fastfmt
maturin develop
pytest -q
```

Maturin enables the optional PyO3 adapter. Rebuild it after Rust changes before checking the installed command or Python API. See [DEV.md](DEV.md) for architecture, build, and release conventions.

The [reference acceptance corpus](tests/reference/README.md) retains ngn, April and APLcart examples for gradual activation. Dyalog documentation examples are added with independently captured structured expectations as glyphs are implemented. Supported cases run in ordinary Rust tests. Pending and scope-question entries remain visible rather than being discarded. The initial character-conjugation, nested-scalar-split and strand/index binding defects are fixed.
