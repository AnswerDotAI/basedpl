# Development

## Commands

```bash
cargo test --no-default-features
cargo run -- -e '2×3+4'
cargo fastfmt
maturin develop
pytest -q
ship-rs-build
```

Rebuild with `maturin develop` after Rust changes before checking the installed extension. Cargo tests alone do not update the editable Python installation. Use `cargo fastfmt`, not `cargo fmt`.

## Structure

- `array.rs`: immutable shared arrays; checked construction, shape/data agreement, scalar normalization, recursive prototypes. No Python or function/environment ownership.
- `number.rs`: concrete Float/Exact/Complex values; checked input construction, explicit monadic/dyadic arithmetic, mixed promotion, comparison, display, and checked structural conversion. Representation is private; every Number is finite/canonical, so array construction never revalidates/reduces it. Exact integers use `BigRational` with denominator 1; complex values use `num_complex::Complex64`, normalizing exactly zero imaginary parts to Float.
- `primitive.rs`: primitive identities/glyphs, valences, array-level implementation and agreement. Singleton selection is shared; scalar shape agreement and replicate-axis agreement remain explicit. Allocation caps are separate from numeric-to-integer conversion.
- `syntax.rs`: byte-spanned lexer and structural parser, definition kind/full span, and guard position metadata. `parse` returns complete syntax, incomplete input, or a structural error without evaluating expressions.
- `eval.rs`: persistent session, explicit right-to-left category-reduction stack, and shared primitive/reduction/dfn/derived/fork calls. Structural resolution consumes one item; grammatical reduction and function application are separate. Functions have immutable shared nodes; unfinished strands/trains/bound-left arguments exist only in the binder. No per-glyph arithmetic precedence. Output and the final result are separate from errors.
- `error.rs`: retained source text, byte spans, inspectable error kinds, and readable Unicode-width diagnostics with separate call-site context. Tabs use four-column stops; other control characters are escaped.
- `cli.rs` / `main.rs`: native command and REPL. The REPL evaluates the parsed result once, not during completeness checking.
- `editor.rs`: Rustyline terminal adapter and one glyph/alias table. Only typed backtick names auto-expand; Tab is an explicit completion request. Bracketed paste/history/navigation cancel automatic expansion. Rustyline owns terminal modes, editing, in-memory history and the final newline on Ctrl-D. No history file or input rewriting in the interpreter/frontends.
- `protocol.rs`: JSON-lines encoding over ordinary Rust values and sessions. No protocol types enter evaluation or arrays.
- `python.rs`: optional PyO3 boundary, copying results and exposing a thread-affine native session. `python/miniapl/__init__.py` owns independent Python Array/Result/error wrappers. `_cli.py` forwards arguments; maturin installs the `miniapl` console script.
- `tests/core.rs`: structural array and language assertions; documentation-derived cases identify their source. `tests/cli.rs` exercises the actual native process.

`Array` stores private shape/data/prototype fields with immutable `Rc` sharing. Scalar shape is `[]` with one element; any zero dimension gives no elements without discarding other dimensions. Nonempty construction derives its prototype; empty construction requires a prototype item. No arbitrary shape/buffer triples or mutable buffers are exposed. Prototype filling memoizes shared nested subarrays for one construction (no persistent cache); checked nesting bounds recursive fill/display/transfer paths. Array elements are numbers, characters, or nested arrays—not functions.

The numeric policy and current limits are documented in README.md. `serde_json` is a frontend dependency; PyO3 remains optional and is constrained to the tested 0.29 compatibility line. JSON requests decode directly to `String`, one per line, with no object envelope or custom escaping. Responses remain structured objects. Keep JSON and Python conversions separate: they implement different external contracts.

Lexical frames are a stack with non-owning parent indices, separate from dynamic call/handler state. Nested functions see live lexical bindings, not snapshots. Functions cannot be array elements or dfn results. Dfn assignment is local, and the public API exposes arrays rather than functions. Operator derivation retains operand values without running the body or creating an invocation frame. These rules prevent local-frame references from escaping the calculator scope. Frames pop on success and error; no collector is needed. Recheck this argument before adding namespaces, nonlocal assignment, function exports/results, asynchronous calls, or tail-frame reuse. Dyalog 20.0.53963.0 reference runs and the scope boundary are recorded in `meta/PRD.md` gate L.

Catch-all `0::` guards keep a shallow binding-map checkpoint after the condition executes, when the guard is installed. Restoring it removes later introduced names as well as restoring previous values. Assignments within the condition survive rollback. This is rollback state, not snapshot closure semantics. A journal would add machinery without a demonstrated need at this scale. Guard handlers are popped before execution and unwind dynamically through ordinary calls; output is retained. Specific error numbers, interrupts, general guards, and tail-call handling remain later work. Unsupported-feature errors are not caught.

Flat binding/operator derivation and assignment chains use explicit vectors. A common evaluation-depth budget covers groups and all function representations; function construction separately limits graph depth, protecting recursive application and destruction. Neither limit is an execution-time sandbox. No stack-growth crate or CPS conversion is needed for the current subset.

Numeric comparisons use fixed relative tolerance `1e-14` when approximate, exact rational comparison otherwise. Each tolerance-sensitive operation must ship with independent inside/outside-tolerance cases: comparisons, membership/index-of, match, unique/grouping, and floor/ceiling. Reuse the rounded pair `0.3` and `0.1+0.2` across applicable operations, with an outside-tolerance control, zero/negative boundaries, and exact/mixed counterparts. Keep structural Rust assertions exact; do not use the language's comparison as the test oracle. The concrete acceptance matrix is in `meta/PRD.md` §9.4.1; unsupported consumers get executable tests when implemented, not skipped placeholders now.

Numeric semantics are Rust-owned. Python copies exact integers directly through PyO3's big-integer conversion, constructs `Fraction` values in the Python adapter, and copies non-real values through `PyComplex::from_doubles`. No decimal-string conversion or Python numeric objects enter the core. JSON independently encodes rational components as strings and complex components as a tagged numeric pair. Both exact reals and basic complex arithmetic are implemented toward the high-school/first-year-college endpoint.

Complex arithmetic extends the existing scalar/array/operator dispatch, not a second evaluator. The lexer shares one real-component scanner between ordinary and `aJb` literals. Equality uses magnitude-based tolerance separately from real ordering; counts still require exact integrality and zero imaginary part. Approximate prototypes/identities normalize to Float. Complex division scales its denominator and direction scales its input to avoid avoidable squared-magnitude overflow/underflow. The fixed finite-result policy still applies; this is not an arbitrary-precision or correctly-rounded complex implementation. Tests use independent arithmetic results and documentation-derived equality cases; Dyalog execution checks remain pending.

The Python binding uses PyO3 0.29.2's `unsendable` guard, plus ordinary RuntimeError checks at its public thread boundary. `close()` drops the native object on its owner thread; the context manager does the same. An unclosed object's destruction on a different thread is unsupported: PyO3 reports an unraisable error and skips its Rust destructor (`ThreadCheckerImpl::can_drop`). Do not hide that restriction or solve it by changing Rust ownership. Exported arrays contain only copied Python data and are independent of this restriction.

## Workspace and builds

The canonical version lives in `Cargo.toml`; Python uses `dynamic = ["version"]`. The crate produces an `rlib`, native executable, and optional `miniapl._core` extension. `python` enables PyO3; `extension-module` also enables PyO3's extension linking mode. Default Cargo builds have no Python dependency.

Keep fastws-generated Cargo patches and `.git/fastws-cargo-key` under fastws control. Preserve the pyproject source/cache keys; do not commit the workspace-generated `Cargo.lock` or manually replace workspace configuration. `meta/` is ignored planning material, never committed.

uv builds and `maturin develop --release` use the incremental `release` profile: LTO off, 16 codegen units, this package incremental. Distributed wheels use `dist`: full LTO, one codegen unit, incremental off, stripped output.

CI tests the native core/CLI/JSON process before installing the extension and running Python tests. Python tests cover boundary behavior and the real installed command, not a duplicate Rust semantic suite. A dist-profile CPython 3.13 wheel has also passed these checks in a clean temporary environment outside the workspace, with the source checkout and Rust absent from its import/command paths. The existing wheel/sdist and tagged publication flow remains unchanged.

`tests/test_repl.py` uses a real pseudo-terminal for symbol entry, ambiguity, bracketed paste, Ctrl-C recovery and Ctrl-D's final newline; piped process tests cannot exercise these paths. Rust editor tests cover matching and quoted/comment context. The editor's Enter callback records just the accepted replacement because Rustyline cannot combine replacement and submission in one command; the adapter applies it before history/evaluation and shows the glyph in its submission message. This does not reprocess an entire source string.

Do not repeat isolated wheel installs for routine feature changes. `maturin develop` plus the normal tests is the development default; reserve clean-install checks for packaging-sensitive changes or release preparation, using a small relevant subset of existing tests.

## Release

Development tests and artifact checks precede release approval. Once Jeremy approves a release, confirm the clean tree and Cargo version, then use `ship-release` with no flags. For this maturin project fastship tags/pushes the current version, leaves publication to CI, then bumps Cargo and refreshes the editable installation. There is no changelog step. First publication requires Jeremy's PyPI trusted-publisher setup. Never commit, push, tag, or publish without approval.
