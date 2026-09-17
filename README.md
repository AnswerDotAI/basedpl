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

The REPL continues unclosed parentheses/definitions across lines, retains names, and recovers after errors. Piped input uses the same session without prompts; `miniapl -` executes all of stdin as one source. Batch errors exit with status 1; bad command arguments exit with status 2. Output already produced is preserved.

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

State persists. JSON/APL errors return structured errors and do not terminate the process. Stdout contains only protocol lines. Newlines in APL source must be escaped inside the JSON string. EOF ends the process normally. Values preserve shape, flat data, and prototype; ordinary floats are JSON numbers, exact values are `{"rational":["numerator","denominator"]}` with decimal integer strings (including denominator `"1"`), characters are strings, and nested elements use the same array object form. Error spans are byte ranges into their accompanying source text; `calls` lists enclosing defined-function call sites without replacing the originating span. There is no streaming, multiplexing, Jupyter dependency, or Python round-trip.

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

Python receives independent `int` values for exact integers, `fractions.Fraction` for non-integer exact values, and `float` for ordinary numbers. JSON uses the rational representation above to avoid client-side rounding. Complex arithmetic is part of the endpoint but is not implemented yet.

## Implemented subset and limits

- Right-to-left evaluation; numeric vectors/stranding; names and assignment; `⋄`/newline statements; parentheses; `⍝` comments; explicit `⎕←` output.
- Monadic/dyadic `+ - × ÷`, scalar extension and shape agreement; monadic `⍳ ⍴ ≢ ,`.
- Numeric dyadic `= ≠ < ≤ > ≥`: exact/exact comparisons are exact; comparisons involving floats use fixed Dyalog-style relative tolerance `1e-14`. No configurable `⎕CT`.
- Vector reduction calls its operand through the same dispatch as ordinary application. It is right-associated: `-/1 2 3` is 2. Empty reduction currently has identities only for `+` and `×`.
- `/` retains its hybrid role when named or parenthesized: `r←/ ⋄ +r 1 2 3` gives 6, while `1 0 1 r 2 4 6` gives `2 6`. Replication extends singleton vectors as well as scalars: `(,2)/3 4` gives `3 3 4 4`, and `1 0 1/,3` gives `3 3`. Scalar arithmetic also supports singleton extension.
- Dfns with local assignments, nested lexical definitions, and explicit array results; three-function forks; monadic user-defined operators with function or array operands. For example, `apply←{⍺⍺ ⍵} ⋄ (-apply)3` gives ¯3, and `offset←{⍺⍺+⍵} ⋄ (2 offset)3` gives 5.
- Names follow lexical nesting, not the dynamic caller. Recursive and mutually referring definitions work. Boolean guards allow `fact←{⍵=0:1 ⋄ ⍵×∇⍵-1}`; `fact 6` gives 720.
- Catch-all `0::` error guards restore bindings to before guard evaluation, including removing newly introduced locals. The selected guard is inactive in its handler; earlier guards can catch handler failures. Output is not rolled back. Specific error-number guards and interrupt handling are not implemented. Unsupported features are deliberately not caught by guards.
- Functions share immutable nodes rather than copying their trees. Lexical links refer to active stack-owned frames; functions cannot escape via array results, local assignment, or the public API. This is not a claim about future namespace/export features. Definitions retain full source spans; errors retain their origin and defined-function call sites.
- Escaping local captures, default left arguments, complete train/operator/guard families, tail-call optimization, axes, general reshape, strings, indexing, modern array literals, and `⍛` await later checkpoints. Unsupported combinations fail explicitly. Search/match/grouping and floor/ceiling still await implementation and their cross-operation tolerance tests.
- Fixed 1-based iota. Counts must be integral and nonnegative; iota and replication currently cap generated results at 1,000,000 elements. Array nesting, syntax nesting, function-graph depth, and combined evaluation/call nesting each have a 128-level limit. Flat binding and assignment chains are iterative. Diagnostic carets use Unicode display width; tabs render at four-column stops.
- Ordinary finite real `f64` numbers: `42`, `1.5`, `.5`, `¯2`, `1E¯3` (`e` also works). `-` is a function; use `¯` inside literals. Integers through ±2⁵³ are representable exactly; larger ordinary literals can round. Decimal arithmetic is not generally exact. Underflow rounds, potentially to zero; overflow/non-finite literals are domain errors. Negative zero is normalized. Opt into arbitrary-precision exact reals with `x`/`r` as above.
- Division follows Dyalog's default `⎕DIV=0`: `0÷0` is 1; other zero divisors and `÷0` error. See [Dyalog's division contract](https://docs.dyalog.com/20.0/language-reference-guide/system-functions/div/). No configurable system variable is added.

The shared Rust array representation already preserves shaped/nested values and typed empties. Matrix/nested display and their broader APL operations are not claimed complete. Core values, evaluation, and ownership contain no Python or JSON types.

## Development

```bash
cargo test --no-default-features
cargo fastfmt
maturin develop
pytest -q
```

Maturin enables the optional PyO3 adapter. Rebuild it after Rust changes before checking the installed command or Python API. See [DEV.md](DEV.md) for architecture, build, and release conventions.
