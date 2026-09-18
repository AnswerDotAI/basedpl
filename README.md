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

In the interactive terminal, type a **backtick followed by a symbol name** or abbreviation. Tab replaces it; a non-letter replaces it and also enters that character. For example:

| Type | Becomes |
| --- | --- |
| `` `io `` then Tab | `⍳` |
| `` `iota5 `` | `⍳5` |
| `` 2`times3 `` | `2×3` |
| `` v`assign `` then Space | `v← ` |
| `` `scan `` then Tab | `\` |

Names are case-insensitive. Multi-part names display with hyphens, such as `left-arrow`; type letters only. Exact names win, then prefixes, then abbreviations that retain the first letter and any later letters in order: `lar` selects `left-arrow`, and `grup` selects `grade-up`. Ambiguous matches are never guessed. Minus remains a terminator: typing `` `lar- `` inserts `←-`.

Python frontends can use `miniapl.symbols`, the REPL's catalogue of `(glyph, space-separated aliases)` pairs.

Matching names appear as you type; press Tab twice to list ambiguous choices, or backtick then Tab twice to browse the catalogue. Enter also accepts a unique name and submits the line, showing the accepted glyph after an arrow.

Arrow keys edit and recall this session's history. Ctrl-C cancels the current input, including unfinished multiline expressions; Ctrl-D exits on a fresh line. Expansion is disabled in strings/comments and for bracketed pastes. This is an input method, not APL syntax: files, pipes, `-e`, Python and JSON use actual glyphs. The catalogue includes planned primitives as well as those implemented below.

```text
$ miniapl -e '¯2+1÷0'
DOMAIN ERROR: division by zero
 --> <expression>:1:5
¯2+1÷0
    ^
```

### Vim

The `editors/vim` package highlights APL comments, quoted strings and glyphs using the current color scheme. It detects `.apl` files. Install it from the repository root:

```bash
mkdir -p ~/.vim/pack/dev/start
ln -s "$PWD/editors/vim" ~/.vim/pack/dev/start/miniapl
```

New Vim sessions load it automatically. In an existing session, run `:packadd miniapl`, then `:setfiletype apl` in an APL buffer. Run the syntax checks with `vim -Nu NONE -n -i NONE -es -V1 -S editors/vim/test.vim`.

## Python

```python
import numpy as np
from fractions import Fraction
from miniapl import Session, Array, AplError

with Session() as apl:
    apl(x=np.arange(1, 6))       # bind variables; returns None
    assert apl('+/x').py == 15
    assert apl('+/x', x=[1, 2, 3]).py == 6  # bind, then evaluate
    apl['x'] = [[1, 2], [3, 4]]
    assert apl['x'].shape == (2, 2)

    mean = apl.fn('{(+/⍵)÷≢⍵}')
    assert mean([1, 2, 3]).py == 2
    assert mean([1, 2, 4]).py == Fraction(7, 3)
    np.testing.assert_array_equal(apl.fn('+')([1, 2, 3], 10), [11, 12, 13])

    r = apl.eval('⎕←x ⋄ x+1x', x=3)
    assert r.value.py == 4 and r.output == ['3x']  # captured, not printed
```

`apl(...)` returns an immutable `Array` or `Function` and prints only explicit output (`⎕←` and display commands). `apl.eval(...)` returns `Result(value, output)` without printing. Both suppress implicit display, including intermediate expressions inside `⍎`. Output is captured during execution, not streamed. In notebooks, a trailing Python `;` suppresses automatic display of the return value when explicit output is sufficient.

`apl.run(...)` captures APL session display in `Result(value, output)`. It includes implicit expression output and explicit output. Assignments remain silent. Notebook frontends use this method to render cells as APL sessions.

Keyword arguments are persistent APL bindings; the optional source argument is positional-only, so `apl(source=3, timeout=4)` binds those names too. Indexing evaluates an expression, and indexed assignment binds an ordinary name without executing generated source. Native arrays and functions pass directly as values, without temporary globals.

### Python values

`Array` retains the native Rust value, including nesting, exact numbers and empty prototypes. Passing it back to a session shares that immutable value without serialization or copying. Arrays can outlive their session and move between Python threads or sessions. `Array(value)` also accepts the Python inputs below. `repr(a)` uses Python numbers: exact `2`, approximate `2.`. `a.apl` uses APL display: `2x`, `2`. `a.shape` is a tuple.

Use `a.py` for Python values according to this table. Use `a.np` or `np.asarray(a)` for an ndarray, including for scalars and strings. Conversions copy; `copy=False` is rejected. NumPy is optional and imported only when a conversion produces an ndarray. Scalar and string `.py` conversions do not require it.

| APL result | `.py` value |
| --- | --- |
| Numeric scalar | `int`, `float`, `complex` or `Fraction` |
| Exact integer array fitting signed 64 bits | `int64` ndarray |
| Floating-point array | `float64` ndarray |
| Float/complex array | `complex128` ndarray |
| Large integers, fractions, mixed exact/approximate values, or nested arrays | `object` ndarray |
| Character scalar/vector | Python string |
| Higher-rank character array | `U1` ndarray |

Python integers (including NumPy integers) and booleans enter APL as exact numbers; floats remain approximate. `Fraction` and complex values retain their numeric meaning. Real infinities round-trip as Python/NumPy floating infinities. Rectangular lists/tuples become ordinary arrays; ragged ones become nested arrays. Explicit object ndarrays retain their shape and nesting. Strings become character vectors. Python/NumPy inputs are copied, including non-contiguous arrays; modifying them or a converted ndarray never changes session state. Cyclic containers, NaN, complex infinities, wider-than-64-bit floats, wider-than-128-bit complex values, bytes, datetime and structured dtypes are rejected.

The native `Array` boundary is lossless. Explicit `.py`/`.np` conversions are Pythonic rather than lossless APL serialization: numeric dtypes select domains, boxed scalars become zero-dimensional object ndarrays, and custom empty prototypes are not retained. Empty lists and empty object ndarrays use a floating zero prototype on input; numeric/character ndarray dtypes supply their corresponding prototype. Integers beyond Python's decimal-string digit limit transfer without changing that process-wide setting.

A final array assignment returns its array without printing it. An unassigned function expression returns a `Function`. Function/operator assignments and empty input return `None`, not the preceding statement's value. Evaluations are not transactions: assignments completed before a runtime error remain in the session.

Assignments bind inside expressions: `1+a←3` returns 4 and stores 3. Modified assignment passes through its right argument. `a b+←3 4` updates the targets from left to right.

### Array operations and word functions

```python
from miniapl import Array, plus, times, subtract, divide, tally, floor

a = Array([[1, 2, 3], [4, 5, 6]])
a + [10, 20]                 # leading-axis agreement: [[11, 12, 13], [24, 25, 26]]
a[2, :]                      # origin one: [4, 5, 6]
a[:, [1, 3]]                 # [[1, 3], [4, 6]]
mean = plus.reduce() / tally  # +/÷≢
assert mean([1, 2, 4]).py == Fraction(7, 3)
assert times(2)(3).py == 6     # exact right argument: ×∘2x
assert times(2.)(3).py == 6.   # approximate right argument: ×∘2
assert subtract.left(2)(5).py == -3
assert (plus @ times)([1, 2], [3, 4]).py == 11
assert (floor << times(10.))(1.25).py == 12
```

Array arithmetic and comparisons use miniapl semantics. `%` swaps the arguments of APL residue; `//` floors division; `@` is matrix/inner product; `&` and `|` are LCM/GCD (and/or on Booleans). Comparisons return arrays. `len` and iteration use major cells. `int`, `float`, `bool` and integer indexing conversion require simple singletons; arrays are unhashable. Use `member(x, y)` instead of Python `in`. Array indexing uses origin-one APL brackets, index arrays and full `:` axes; bounded slices are not supported.

Word primitives have separate monadic and dyadic names: `sign`/`times`, `shape`/`reshape`, `iota`/`index_of`, `mix`/`take`. Calling a dyadic name with one argument binds the right argument; `.left(x)` binds the left. Calling it with two arguments applies it. Operator operands use the underlying APL function: `plus.reduce()` is reduction with dyadic `+`, not conjugation. The word table is in `python/miniapl/functions.py`.

Monadic operators are methods: `.reduce()`, `.scan()`, `.each()`, `.commute()`, `.outer()`, `.key()`. Dyadic operators are `.inner(g)`, `.rank(r)`, `.beside(g)`, `.atop(g)`, `.over(g)`, `.behind(g)`, `.power(n)`, `.at(indices)` and `.stencil(spec)`. `.under(g)` composes the existing inverse of `g` with `f` over `g`; it is computational under, without structural replacement of untouched data. `f[axis]` qualifies an axis. `fork(f, g, h)` and `atop(f, g)` construct trains.

Arithmetic between functions constructs a fork, including constant arms. `f @ g` is inner product; mixing a function and an array with `@` raises. `f << g` is `f∘g`; `f >> g` reverses that composition. `f ** n` is power, with `-1` requesting an inverse. Python determines grouping: `**` binds more tightly than arithmetic, shifts less tightly, and `@` groups left at multiplication precedence. Once built, a function executes through the APL evaluator, including right-before-left fork evaluation. `repr(f)` shows APL syntax.

### Retained and late-bound functions

`f = apl('g')` retains the current function node. `f = apl.fn('g')` resolves `g` on each call and follows redefinitions. Both are composable `Function` objects; one argument supplies `⍵`, two supply `⍺, ⍵`. Late-bound operands resolve once per function call, including inside operators and trains. `apl('{⍵×2}')` exports a dfn directly. No argument is converted to APL source.

Primitive-built functions are session-independent. Exported dfns and `.fn()` retain their originating Python session for global lookup. Composing a free function with a session-bound function uses that session; combining different sessions raises. A session-bound function cannot be assigned to a different session. Closing its session prevents further calls. Free functions remain usable. Export rejects active lexical-frame references anywhere in the function graph; functions still cannot be array elements or dfn results. Python callbacks are not operands.

### Errors, interruption and lifetime

`AplError` carries the Rust-rendered diagnostic, kind, message, retained source, UTF-8 byte span, call-site context (`calls`) and output produced before failure. `apl(...)` and `.fn()` print that output before raising; `.eval()` retains it on the exception without printing.

`Session` uses one persistent Rust worker thread, serializing evaluations. Calls may come from different Python threads. Ctrl-C interrupts the current evaluation; `apl.interrupt()` does the same from another thread. Cooperative interruption preserves the session and completed assignments. Use `Session(timeout=2)` for a per-evaluation deadline in seconds; change `apl.timeout` to adjust it, or set it to `None` for no deadline. Waiting releases the GIL and checks Python signals. Native-library calls and individual BigInt operations can delay cancellation. The thread is never forcibly killed, and requests are never replayed. Use the separate process `Worker` below when execution needs a hard-kill fallback.

Prefer `with Session()` or explicit `.close()`. Closing rejects new requests and interrupts active work. The worker destroys its Rust session on its owning thread after that work unwinds. Closing does not wait for an uninterruptible native operation. Abandoned sessions close when collected or Python exits. Returned arrays remain usable.

## Rust embedding

Run interpreter workloads inside `miniapl::with_stack(|| { ... })` to use a 256 MiB execution stack. Wrap the session workflow, not each individual call. Direct `Session` methods use the caller's stack. CLI and Python workers configure their stacks automatically. Evaluation nesting is limited to 1,024 steps.

`Session::eval_with` and `Session::call_with` accept `EvalOptions`. Set `echo: false` to suppress implicit expression display while retaining explicit `⎕←` output, including output before an error. This also suppresses intermediate implicit output inside `⍎`. Explicit display commands such as `]Display` still produce output. Echo defaults to true and is selected separately for each evaluation.

```rust
use miniapl::{with_stack, Array, EvalOptions, Session};

with_stack(|| {
    let mut s = Session::new();
    let r = s.call_with("{(+/⍵)÷≢⍵}", &[Array::integers(vec![3], vec![1, 2, 3]).unwrap()],
        EvalOptions { echo: false, ..EvalOptions::default() });
    assert!(r.error.is_none() && r.output.is_empty());
    assert_eq!(r.value, Some(Array::integers(vec![], vec![2]).unwrap()));
});
```

`call` / `call_with` resolve one function expression in the current session, then pass one array as `⍵` or two arrays as `⍺, ⍵`. Arguments are not serialized into APL source and no temporary names are assigned. Function names, dfns, trains and derived functions are accepted. `Evaluation.function` holds an unshy, exportable function result. `set_function` binds a retained function; `call_function_with` calls it without reparsing. `Function::late_bound` parses an expression for resolution on each call. Rust callers supply the session for global lookup. All function exports reject active lexical-frame references. Calls use the same deadlines and interruption support as evaluation.

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

State persists. JSON/APL errors return structured errors and do not terminate the process. Stdout contains only protocol lines. Newlines in APL source must be escaped inside the JSON string. EOF ends the process normally. Values preserve shape, flat data, and prototype. Exact integers are arbitrary-sized JSON integers. Finite floats retain a decimal point or exponent. Infinities use `{"infinity":1}` and `{"infinity":-1}`. Fractions use `{"rational":["numerator","denominator"]}` with decimal integer strings. Non-real complex values use `{"complex":[real,imaginary]}`, characters are strings, and nested elements use the same array object form. JavaScript clients need a parser that preserves large integers; ordinary `JSON.parse` can round them. Error spans are byte ranges into their accompanying source text; `calls` lists enclosing defined-function call sites without replacing the originating span. There is no streaming, multiplexing, Jupyter dependency, or Python round-trip.

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

The underlying `--worker` protocol uses JSON objects, one per line. Send `{"id":1,"code":"+/⍳10","timeout_ms":2000}` and receive `{"id":1,"result":...}`. Send `{"interrupt":1}` to cancel that request; interrupt messages have no reply. Use one outstanding evaluation per client. Stdout contains responses only. Malformed JSON or invalid IDs/timeouts/echo types terminate the worker. The simpler `--json` mode remains unchanged.

Worker requests also accept `bindings`, mapping names to arrays in the response encoding, and `echo` (default true). Bindings are installed before evaluation and persist even if APL execution fails. A bindings-only request returns a null value and no output. Use `call` instead of `code`, plus `args` containing one or two encoded arrays, to invoke a function directly:

```json
{"id":1,"bindings":{"x":{"shape":[3],"data":[1,2,3],"prototype":0}},"code":"+/x","echo":false}
{"id":2,"call":"+","args":[{"shape":[],"data":[2],"prototype":0},{"shape":[],"data":[3],"prototype":0}],"echo":false}
```

JSON integer inputs are exact; decimal/exponent inputs are floating-point. Infinities, fractions, complex numbers, nested arrays and empty prototypes use the same encodings in both directions. Invalid operations or array payloads return `REQUEST ERROR` without terminating the worker. Binding batches are not transactions. `Worker.request(payload, timeout=...)` exposes these operations without constructing the outer request ID. Use the infinity tags in payloads; raw Python infinity/NaN would produce invalid JSON and is rejected before sending.

The process worker remains separate from the default thread-backed Python `Session`. Rust callers use `Session::eval_with` or `call_with` with `EvalOptions`; clone its `InterruptHandle` to cancel from another thread. No Jupyter or MCP dependency enters the interpreter.

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

Exact fractions are reduced, denominators positive, and exact integer results display with `x`. Only integer components are accepted in `x`/`r` literals; use `¯` for signs. An exact value too large for a required float conversion gives a domain error. Merely placing exact and approximate values in one array does not convert them. Empty prototypes and reduction identities retain the numeric domain. Predicates, positions, shape and tally produce exact integers regardless of contents or prototypes. Iota and roll/deal retain their argument-domain rules. `?0` remains approximate.

Python `.py` conversion produces `int` for exact integers, `fractions.Fraction` for non-integer exact values, and `float` for ordinary real numbers. JSON uses the numeric encodings above.

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

Python `.py` conversion produces native `complex` values; JSON uses the tagged pair above. Real-normalized results such as the conjugate product convert to a Python float/JSON number. Equality and inequality use the same fixed `1e-14` tolerance with complex magnitudes, following [Dyalog's equality rule](https://docs.dyalog.com/20.0/language-reference-guide/primitive-functions/equal-to/), rather than testing components separately. Relational ordering and numeric counts such as reshape dimensions require real values; tolerant coercion of near-real complex values is not implemented. Powers, roots, logarithms and circle functions support complex arguments. For example, `¯1*0.5` gives `0j1` and `1+*○0j1` gives `0`.

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

Character literals use single quotes, with doubled quotes inside (`'can''t'`). Character ± integral number shifts the Unicode code point. Number + character also works. Character − character returns an exact integer. For example, `'a'+3` is `'d'`, and `'d'-'a'` is `3x`. Invalid offsets, surrogate results, number − character and character + character give DOMAIN ERROR. One character is a scalar; comma makes it a singleton vector. `⊂` encloses and `⊃` discloses. `⍬` is the numeric empty vector. Arithmetic extends recursively through nested arrays. Parenthesized literals preserve nesting; bracket literals assemble rows/cells with fill. Literal elements evaluate left-to-right.

`⎕A` is `'ABCDEFGHIJKLMNOPQRSTUVWXYZ'`; `⎕D` is `'0123456789'`. Their names are case-insensitive and cannot be assigned. `⎕C` folds case; `1⎕C` uppercases and `¯1⎕C` lowercases. Unicode simple mappings preserve shape and nesting, without expanding characters such as `ß` into `SS`.

`⎕UCS` converts characters to exact integer code points and back, preserving shape. A left argument of `'UTF-8'`, `'UTF-16'` or `'UTF-32'` encodes or decodes a vector. Scalar input becomes a singleton vector. Malformed encodings and surrogate code points error. The optional encoding flag `0` is accepted; signed-byte flag `83` is out of scope. Other system names are rejected as unsupported.

- Right-to-left evaluation; numeric vectors/stranding; names and assignment; `⋄`/newline statements; parentheses; `⍝` comments; explicit `⎕←` output.
- Monadic/dyadic `+ - × ÷`, scalar extension and shape agreement; monadic `⍳ ⍴ ≢ ,`; general reshape, take/drop, enclosure/disclosure, Mix/Split, catenate/table, reverse/rotate, and transpose (including diagonal axes).
- Numeric dyadic `= ≠ < ≤ > ≥`: exact/exact comparisons are exact; comparisons involving floats use fixed Dyalog-style relative tolerance `1e-14`. No configurable `⎕CT`.
- Scalar maths: `| ⌊ ⌈ * ⍟ ○ ! ∨ ∧ ⍲ ⍱ ~`, including exact rational operations where closed in that domain. Roll/deal `?` uses an unseeded generator; `?0` lies strictly between 0 and 1.
- Search and sets: match/depth, not-match, membership/enlist, index-of, where/interval-index, nub sieve, unique/union/intersection/without and find. Tolerance-sensitive searches share numeric comparison. Interval-index uses the structural ordering below. Min/max retain numeric ordering.
- Last/first-axis reduction and scan (`/ ⌿ \ ⍀`) support primitive and user-defined operands. Generic reduction is right-associated: `-/1 2 3` is 2. Primitive float `+/` and `×/` permit compiler reassociation for speed; their grouping is unspecified, including with axes. Cancellation, rounding and overflow can differ from Dyalog or across builds. For example, `+/1E100 ¯1E100 1` has no promised fixed-order answer. Exact arithmetic remains exact. Every scan accumulates left-to-right: `-\1 2 3` gives `1 ¯1 ¯4`. Seeded scan applies the seed before the first input: `10 -\1 2 3` gives `9 7 4`. The seed is scalar or matches the unscanned axes. Empty scans and unseeded singletons make no operand calls. Numeric cumulative sums retain direct linear-time loops. N-wise reduction accepts zero and negative widths. Empty reductions use known primitive identities; unknown identities give domain errors. Empty min/max use `∞`/`¯∞`, including exact empties. Replication/expansion support matrix axes and signed counts for fill.
- Bracket indexing uses origin 1: `m[2;1]`, `m[;2]`, `v[3 1]`. Coordinate arrays and nested paths are supported. Indexed, modified, strand and selective assignment work, including Each selectors and repeated updates. Axis qualifiers support folds, replication/expansion, reverse/rotate, catenate, take/drop, scalar functions, ravel, mix and enclosure. Fractional axes insert dimensions for ravel, mix and laminate; multi-axis selectors support take/drop, enclosure and squad.
- `/` retains its hybrid role when named or parenthesized: `r←/ ⋄ +r 1 2 3` gives 6, while `1 0 1 r 2 4 6` gives `2 6`. Replication extends singleton vectors as well as scalars: `(,2)/3 4` gives `3 3 4 4`, and `1 0 1/,3` gives `3 3`. Scalar arithmetic also supports singleton extension.
- Dfns with nested lexical definitions, lazy default arguments (`⍺←2`), and silent/no-result behavior. Plain name assignment is local; modified, indexed and selective array updates target the nearest existing lexical binding. Execute-created definitions use ordinary lexical capture too. Monadic/dyadic dops accept function, array or hybrid operands. For example, `apply←{⍺⍺ ⍵} ⋄ (-apply)3` gives ¯3, and `op←{⍺⍺+⍵⍵×⍵} ⋄ (2 op 3)4` gives 14.
- Atops, forks, constant arms and longer trains; `⊣ ⊢`; composition/binding `∘`, rank/atop `⍤`, over `⍥`, behind `⍛`, Each `¨` and commute/constant `⍨`. `3∘<⍛/2 7 1 8` gives `7 8`. Rank uses shared padded cell assembly and the frame agreement above. Empty Each/rank invokes the operand to obtain a prototype, including its output/errors. Empty Each enables prototype-call mode throughout compositions, dfns, dops and helpers. Pick can select fill for nonnegative out-of-bounds indices in this mode. Thus `{100⊃'abc'}¨⍬` returns `''`, while `{100⊃'abc'}0` gives INDEX ERROR. Other errors and explicit output are retained.
- Inner and outer products use shared operand calls: `1 2 3+.×10 12 14` gives 76; `(⍳3)∘.=⍳3` is an identity matrix. Singleton contraction extension, nested results and empty products are supported.
- Key `⌸` groups by the first matching representative. Counted, predicate and inverse power `⍣` use shared function calls. Known inverses cover arithmetic bindings, powers/logs, circle codes ¯7 through 7, permutations/rotations, encode/decode, where, supported compositions/Each/rank and dyadic Behind. Left/right binding and dyadic commute propagate the fixed argument. Seeded and unseeded scans invert through supported operand inverses: `10 (-\⍣¯1)9 7 4` gives `1 2 3`. Product scans cannot recover values after a zero accumulator. Axis forms include supported scalar arithmetic, scans, reversal/rotation and enclosure/split/mix. General forks, monadic Behind and arbitrary dfn inversion remain unsupported. Unknown inverses give DOMAIN ERROR.
- `×∘*⍨⍣¯1` computes the principal Lambert W branch. Real arguments must be at least `¯1÷*1`; complex arguments use the principal complex branch. Results are approximate.
- `⍎` executes character code in the current lexical scope, retaining output and defining source. `⍕` supports monadic text and dyadic numeric field specifications, including exact fixed-point formatting. Neither operation depends on REPL boxing settings.
- Grade `⍋ ⍒` is stable. Grade and interval-index share untoleranced structural ordering: numbers, then characters, then nested arrays. Numbers compare by value across representations. Complex values compare real part then imaginary part. Characters compare by code point. Nested arrays compare rank, then ravelled items lexicographically, then shape. Prototypes do not participate. Dyadic grade supports character collation. Pick `⊃` follows nested coordinate paths. Nest `⊆`, partition `⊆` and partitioned enclosure `⊂` support typed empties; both partitions accept axis qualifiers.
- Squad `⌷` uses the bracket-selection path, including shaped indices, missing trailing axes and a single-axis qualifier. Monadic `⌷` returns its array unchanged.
- Encode/decode `⊤ ⊥` support numeric arrays, mixed bases, exact arithmetic and complex values. At `@` accepts replacement arrays or functions, indices or masks, and nested paths. Stencil `⌺` supports window sizes, movements, signed padding arguments and shared result assembly.
- Matrix inverse/divide `⌹` uses rational elimination for all-exact inputs and faer thin SVD otherwise. Rectangular full-column-rank inputs support least squares. Singular and underdetermined systems error. Numerical rank uses machine epsilon × max dimension × largest singular value, not comparison tolerance.
- Names follow lexical nesting, not the dynamic caller. Recursive and mutually referring definitions work. Boolean guards allow `fact←{⍵=0:1 ⋄ ⍵×∇⍵-1}`; `fact 6` gives 720.
- Catch-all `0::` and numbered error guards (`11::`, `6 11::`) restore the installing function's local bindings to their state after evaluating the guard condition. Later local assignments are undone, including modified assignments and newly introduced names. Outer/global writes and output are not rolled back. The selected guard is inactive in its handler; earlier guards can catch handler failures. Empty ordinary/error guard results return no value. Cancellation and unsupported features are not caught by guards.
- Functions share immutable nodes rather than copying their trees. Lexical links refer to active stack-owned frames; public exports reject these links throughout the function graph. Definitions retain full source spans; errors retain their origin and defined-function call sites.
- Direct dfn/dop tail calls run in a loop, including mutual recursion, selected guards and parenthesized returns. Needed lexical frames are retained. Installed error guards disable tail-frame reuse. Calls embedded in further computation remain depth-limited. Function-valued dfn results are syntax errors, as in Dyalog. Mutable namespaces and escaping lexical closures are outside the current calculator scope.
- Fixed 1-based iota. Counts must be integral and nonnegative; generated arrays are capped at 1,000,000 elements. Array nesting, syntax nesting and function-graph depth have a 128-level limit. Combined non-tail evaluation/call nesting and retained lexical frames are limited to 64. Flat binding, assignment chains and supported tail recursion are iterative. Diagnostic carets use Unicode display width; tabs render at four-column stops.
- Ordinary real `f64` numbers: `42`, `1.5`, `.5`, `¯2`, `1E¯3`, `∞`, `¯∞` (`e` also works). `-` is a function; use `¯` inside literals. Integers through ±2⁵³ are representable exactly; larger ordinary literals can round. Decimal arithmetic is not generally exact. Underflow can round to zero; real float overflow produces infinity. Negative zero is normalized. Opt into arbitrary-precision exact reals with `x`/`r` as above.
- Real infinities work in arithmetic, comparison, search, grade and formatting. Infinity equals itself and never a finite number; tolerance applies only to finite comparisons. Huge exact numbers compare with infinities without approximate conversion. Min/max with infinity preserves a selected finite exact value. NaN and complex infinities are rejected. `∞-∞`, `0×∞` and `∞÷∞` give DOMAIN ERROR. Division by zero, logarithm of zero and factorial poles still error (with the existing `0÷0=1` exception). Structural counts, gcd/lcm and matrix factorization require finite inputs. Out-of-range exact-to-float conversion still errors.
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

The [reference acceptance corpus](tests/reference/README.md) covers ngn, April, APLcart and Dyalog documentation examples. Active cases live in `tests/reference/*.apl` and run in ordinary Rust tests. The tracked `tests/reference/inventory/*.jsonl` files retain original records, independent expectations, adaptations and candidates for activation, including pending, scope-question and excluded entries. Activation checks reviewed cases, appends them to `.apl`, and marks their inventory records active. Edit active tests directly in `.apl`; the linked guide documents the record format, `Corpus` API and activation commands. The initial character-conjugation, nested-scalar-split and strand/index binding defects are fixed.
