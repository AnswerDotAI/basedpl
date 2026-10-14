# Development

Keep the language concise to write, simple to read and consistent to understand. A design rule is a plan on trial. Discuss it when applying it makes a useful expression harder to read, removes something expressible or requires another special case. APL familiarity and compatibility are not costs in that decision. Change callers with an API change instead of adding compatibility layers.

Speed and simplicity come before edge cases no real user meets. Follow IEEE, Rust and CPU behaviour where it gives a reasonable result. The last bits of a result can differ between thread counts, platforms, builds and runs. Never spend speed, memory or code to make them agree. Don't add precision, sign normalization or defensive passes without an agreed need.

## You MUST read this ENTIRE section in FULL first

LLMS: Read `nbs/llms.txt` in full before doing any work. BPL differs substantially from Dyalog APL in grouping, names, numbers and operators. Use BPL syntax, not APL idioms assumed to carry over.

```bash
cargo develop
cargo test
pytest
```

Use `cargo develop` to rebuild and install the native extension and `bpl` after Rust changes. It uses the `test` profile shared with `cargo test`. Cargo tests compile missing code but don't install it. Run plain `cargo test` while you work; use one test binary for a focused check. Run `pytest` at the end of a piece of work. Give builds three minutes and test runs one minute; Cargo serializes builds in the same target directory.

Build nothing else without discussion: no scratch crates, alternate target directories, `--release`, `-p` or other-target builds. The documented build diagnostics, browser work and x86-64 inspection below are exceptions. Don't use `maturin develop`; its configuration does not reuse the normal local build.

An already-running Python process retains its loaded extension. In a kernel, use `from basedpl.worker import fresh; fresh(code)` to check the installed `bpl` in a fresh worker instead of restarting the kernel. A BPL error raises `BplError`. Check one expression and read its result before building on it. `basedpl.bpltests.check_file` and `check_page` check reference files and documentation through the worker without building Rust.

Measure with `BASEDPL_COSTS=1 cargo test --test speed -- --nocapture` or `basedpl.bpltests.time_cases(cases)`. Both count costs in copies. A copy is the time to write as many floats as the data has items, each negated, into a second buffer. Budgets are single-thread measurements. Use `time_cases(..., threads=...)` separately for scaling. `speed_costs()` reads the cases in `tests/speed.txt`. `cases=` selects expressions, `wasm=DIR` selects a browser build, `bpl=PATH` selects another native `bpl`, and `native=True` times rustymath's own linear algebra, which dev builds compile in. `logged_costs(log)` reads CI output, and `cargo_costs(path)` runs either project's speed test.

The manual `Speed` workflow measures the native and wasm builds on CI. When you review a run, look mainly for costs that changed by more than 20%, especially where several similar operations changed by similar amounts. Never raise budgets mechanically from a run's costs, because mechanical rises can lock in poor performance. Study each slowdown until you know its cause. Raise a budget only for a slowdown you've judged reasonable, never for a minor variation. `set_budgets(native, wasm, cases=cases)` applies the rule in the budget file to the cases that `cases` names. Name only the cases whose budgets should change. Without `cases`, it changes every case with a cost. Pass `run=` only when the run sets every budget. It replaces the header line that names the run all the CI figures come from. `path=` selects rustymath's `tests/speed.txt`.

To compare a change with the build before it, copy the installed `bpl` before the change, then time both copies in turn with `speed_costs(bpl=path)` or `time_cases(..., bpl=path)`. Don't restore old code in the shared tree to get a baseline. On macOS, native timings of matrix products and decompositions mostly measure Accelerate. Rustymath's DEV.md says how to time rustymath's own kernels.

Cooperating tools share `/tmp/aai-ws-timing.lock`. Timings take it exclusively. `cargo develop`, `cargo wasm`, the reference test and pytest take it shared. Other Rust tests and bare Cargo commands do not all take it. After five minutes a timing fails; a build/test proceeds without the lock. After 30 seconds, an exclusive wait prints the processes with the lock file open. The list can include processes that are only waiting for the lock. Never hold it in the shared kernel process or coordinate timings manually. A timing launched there would block behind the hold and stall other agents' kernel calls. Native timings set rustymath's current-thread count to one and request user-interactive macOS QoS to favour performance cores. QoS is not CPU affinity.

Run `cargo wasm` during development only for browser work. Agents use `cargo wasm --out DIR` and set `BASEDPL_WASM=DIR` when running `node --test tests/wasm.mjs` and `node --no-liftoff --test tests/wasm-speed.mjs`. This avoids replacing a module another agent is using. For other edits under `cfg(web)` or `host_web.rs`, check compilation with `cargo check -p basedpl --target wasm32-unknown-unknown`.

### X86-64 machine code

To inspect the x86-64 code that CI and Linux run, build the library for Linux x86-64. In basedpl:

```bash
CC_x86_64_unknown_linux_gnu=clang cargo build --lib --release --target x86_64-unknown-linux-gnu --target-dir /tmp/aai-ws-x86 -p basedpl
```

In rustymath, use `-p rustymath`. The target is Linux because macOS builds leave out rustymath's native linear algebra kernels. That target is installed here. `-p` builds only the library, because the Python extension crates can't be built for Linux on this Mac. `CC_x86_64_unknown_linux_gnu=clang` lets Apple's `clang` assemble the x86-64 source in basedpl's `psm` dependency, because this Mac has no `x86_64-linux-gnu-gcc`. Don't move that setting into `.cargo/config.toml`. CI's Linux release builds also read that file, and their manylinux container has no `clang`. The build matches CI's `test` profile in its x86-64 flags and optimization settings. Run it only to answer an x86-code question. It doesn't take the timing lock and can disturb timings running alongside it.

Read the newest `lib<crate>-<hash>.rlib` under `/tmp/aai-ws-x86/x86_64-unknown-linux-gnu/release/deps` with `objdump -d -r --no-show-raw-insn --disassemble-symbols=NAME`. `-r` identifies called functions. `objdump` warns for each archive object without that symbol. `-C` matches demangled names but splits them at commas; for a generic name containing a comma, get its mangled name from `nm` and omit `-C`. Generic code belongs to the crate that instantiates it. Don't use `--emit asm`: the resulting codegen-unit arrangement can inline differently from the build being inspected.

## Workspace, builds and CI

| Crate | Responsibility |
|---|---|
| `basedpl` | Python-free interpreter and public Rust API. |
| `cli/` (`basedpl-cli`) | `bpl`, REPL, Jupyter kernel and JSON-lines worker. |
| `py/` (`basedpl-py`) | Unpublished PyO3 extension, `basedpl._core`. |
| `wasm/` (`basedpl-wasm`) | Browser bindings. |
| `xtask/` | Browser package build tooling. |

The default workspace members are core, CLI and Python; wasm/xtask build only when requested. Frontends use the core's public API. Core/CLI Rust tests live in `tests/` and `cli/tests/`. Libraries set `test = false` to avoid separate `cfg(test)` builds. PyO3 is in its own crate, not an optional core feature. The core uses `serde_json` for the process protocol and data functions.

Rust 1.98 or later supplies algebraic float operations. `test` inherits `release`: opt-level 2, incremental compilation, LTO off, no debug information, overflow checks or debug assertions. `.cargo/config.toml` explicitly enables LLVM's SLP vectorizer and targets x86-64-v3 on x86. Target-specific rustflags repeat the SLP setting because they replace build-wide flags. `dist` disables incremental compilation, enables within-crate LTO and strips symbols. Keep these settings aligned with rustymath's.

`fastws`'s `cargo develop` builds profile `test`, installs the extension and stages `bpl` into the venv. `py/` enables PyO3's `extension-module` and supplies its link arguments without libpython. Editable installs use `fastws.build_backend`; they install metadata and a path to `python/` without compiling Rust. After a standalone `uv sync` or editable pip install, run `cargo develop`. `ws-sync` performs that rebuild after syncing. Wheels use maturin for the extension and `cargo stage` for the Python-free CLI. Linux staging runs inside the manylinux container.

Basedpl's crates pass on rustymath's default `native` feature, which rustymath's DEV.md describes. Each depends on the crate below it with `default-features = false` and makes `native` its own default. The release job builds wheels with `--no-default-features`. `cargo stage` always builds without default features. The released Mac builds therefore leave out rustymath's own linear algebra. On Apple targets, `flate2` uses the system zlib. Other targets keep its Rust backend, `miniz_oxide`.

Use published dependency ranges with the lower bounds required by the APIs in use. `Cargo.toml` names a published rustymath range; the `~/aai-ws` Cargo patch selects the sibling checkout locally. CI resolves published crates. Release rustymath changes before relying on them in basedpl CI. Don't edit fastws-generated Cargo patches, `.git/fastws-cargo-key` or workspace configuration, and don't commit the workspace-generated `Cargo.lock`. `meta/` is ignored planning material, not contributor documentation. `[tool.uv] cache-keys` tracks packaging manifests, not Rust sources.

`Test` runs native and wasm jobs on pushes to main and pull requests. Native CI runs `cargo test`, editable installation, `cargo develop`, pytest, `bpl-nb` and `nbdev-test`. Python tests use the same test-profile build. The wasm job builds with `cargo wasm --strip` and runs both Node suites. Both jobs use `ubicloud-standard-4`; speed budgets are set for that runner. They generate a lockfile before restoring `Swatinem/rust-cache`, making dependency changes part of the cache key. `Speed` is manually triggered and restores those caches.

`Release` runs separately on `v*` tags on GitHub's runners. It builds dist wheels, sdist, standalone CLI artifacts and the Windows keyboard archive, then publishes Python artifacts and the GitHub release. Its npm job publishes the stripped browser package through trusted publishing and purges jsDelivr's package files to avoid stale unversioned URLs. Release neither runs tests nor waits for the Test workflow.

### Codegen and rebuilds

LTO-off incremental builds keep an edit from recompiling every caller that inlined it. Debug line tables increase rebuild work when lines move. Related source is split into child modules to limit those shifts. Children use `use super::*` and expose items to parents/siblings with `pub(super)`; parents own public re-exports.

Generic loop instances can concentrate in the defining module's codegen unit. Mark loop helpers `#[inline]` when their instances need to compile with the caller's kernel table, including nested helpers. Kernel tables have separate modules: `booleans`, `wholes`, `float_arithmetic`, `float_kernels` and `complex_kernels`. `at_widths!` gives the concrete widths their own modules. Keep new tables separate and add kernel specializations only for measured gains.

Use `cargo build --lib --bins --profile test --timings` to inspect the normal build. Its report under `target/cargo-timings/` separates frontend and codegen time. `BASEDPL_UNITS=1 cargo test --test units -- --nocapture` lists large units and their functions. `BASEDPL_SIZES=pattern cargo test --test units -- --nocapture` measures machine code matching a demangled-name regex. `BASEDPL_SIZES` sees only basedpl's own codegen units. `BASEDPL_SIZES_DIFF=old:new cargo test --test units machine_code_growth -- --nocapture` compares two binaries, such as copies of `bpl` from before and after a change, by generic function across every crate. These commands diagnose the current compilation arrangement; don't create scratch builds to approximate it.

## Code map

| Files | Responsibility |
|---|---|
| `syntax.rs`, `eval/binder.rs` | Structural parse, runtime category binding, trains, dot access and subscripts. |
| `eval.rs`, `eval/session.rs`, `eval/operators.rs`, `eval/assign.rs` | Shared calls, sessions/modules, operators, scopes and assignments. |
| `execution.rs`, `error.rs` | Cancellation, deadlines, input/output and retained diagnostics. |
| `array.rs`, `array/storage.rs`, `array/text.rs` | Values, layouts, compact/mixed storage, gathering and source formatting. |
| `agreement.rs`, `keyed.rs`, `selection.rs` | Broadcasting, named/keyed alignment and selection/writeback. |
| `primitive.rs`, `primitive/` | Primitive metadata, ranks/axes, inverses and array operations. |
| `number.rs`, `element.rs`, `pervasive.rs`, `fusion.rs` | Numeric domains, item operations, compact kernels and pending chains. |
| `search.rs`, `number_theory.rs`, `polynomial.rs`, `distribution.rs` | Array-level adapters to rustymath. |
| `system.rs`, `data.rs` | System-function metadata, native functions, options and import rules. |
| `csv.rs`, `json.rs`, `xml.rs`, `image.rs`, `plot.rs`, `canvas.rs`, `display.rs` | Formats, rich values and rendering. |
| `protocol.rs`, `reference.rs`, `timing.rs` | Process encoding, semantic checks and measurements. |
| `host.rs`, `host_web.rs`, `js.rs` | Platform services and JavaScript values/functions. |
| `symbols.rs`, `inspection.rs` | Shared names, shortcuts, syntax/help metadata and inspection. |
| `cli/src/`, `py/src/lib.rs`, `wasm/src/lib.rs` | Frontend boundaries. |

User-facing language rules belong in `nbs/syntax.qmd`, `evaluation.qmd`, `numbers.qmd`, `arrays.qmd`, `terms.qmd` and the glyph/system pages. Define terms in `terms.qmd` before using them. The sections below record contributor contracts and ownership rather than duplicating the language reference.

## Parser, evaluator and scopes

The lexer/parser records byte spans and structural syntax without evaluating expressions. `parse_at` distinguishes complete input, incomplete input and errors at a given float width. Public `parse` uses f64; `Session::parse` uses the session width. `syntax_tree` supplies source tools with structural nodes and recovers invalid chunks as `Error` nodes. `highlight` shares the lexer.

`symbols.rs` owns APL, simplified and traditional Hanzi spellings. Parsing always accepts them together, canonicalizing token decisions while retaining original source and spans. Han characters are reserved outside strings and comments. The `notation` preference affects structured function/operator display, not stored source, serialization or structural APIs; help and editor metadata use the same spelling table.

The binder resolves runtime categories right to left. It owns unfinished strands, trains and bound arguments; shared function nodes are immutable. Grammatical reduction produces application requests instead of calling functions. Primitive/operator rows own names, operand kinds, ranks, pervasion, axis behaviour and reduction identities. Change those rows instead of duplicating metadata in consumers. A non-extending primitive above its natural ranks runs through Rank; pervasive Each/Outer/Rank paths reuse whole-array kernels.

Dot names rewrite to Pick, and dot brackets/subscripts to selection. Dot parentheses evaluate a namespace: `T.(expr)` and `T⍎text` use the keyed entries of `T` as locals, resolve other names in the caller and keep assignments local. They share the scoped execution route and lexical lifetime checks. Between functions, dot remains inner product.

Lexical frames use non-owning parent indices. Closures see live bindings, not snapshots. Plain assignment is local; modified/selective assignment updates the nearest existing binding. Retain the resolved owner and original array across modifier calls rather than looking the name up again. Returns and outer writes reject functions, including those nested in arrays/prototypes, whose frames would not survive. Tail calls retain dependencies from arguments and the called function. No collector is used; recheck this lifetime reasoning before permitting escaping lexical captures.

Module 0 owns session names. Each `•load` runs in a new module, without the caller's frame and with echo off, and returns its non-private names alphabetically. Modules remain for the session's lifetime. Recursive loading of a canonical path is a DOMAIN error. Paths starting `./` or `../` resolve against the source file; other paths use the working directory. Execute/load do not run the top-level preference header.

Dfns run statements in order and return the last statement's value. Final assignments are silent. An absent left argument propagates through the binder's absent-binding rules; default-left assignment runs only when needed. Guard handlers see bindings as they stood at failure; nothing rolls back. Wildcard `∞::` excludes interrupt, timeout and unsupported errors. Explicit named guards can catch them; catching an interrupt clears its flag.

Tail calls reuse frames while preserving lexical dependencies. Handlers disable tail reuse. Native call depth is capped at 20,000; the browser cap is 380. A separate 20,000-frame limit bounds retained lexical frames. Late-bound name resolution stops at 128 levels. These are execution limits, not a sandbox against arbitrarily deep source, arrays or function graphs.

## Values, layouts and assignment

`Value` holds a number, character, function, operator or shared array. Compact storage holds Booleans, narrow integers, width-specific floats/complex numbers or characters. Mixed storage holds `Value`s and memoizes its prototype and innermost lexical dependency. Other properties are computed when needed. An array holds its layout and its storage in one allocation. Its `Shape` holds ranks 0 to 2 inline, so a small array allocates no shape. Constructors take `impl Into<Shape>`: pass a slice, or an array such as `[n]`, rather than building a `Vec`. A shape, key or name change on a shared array makes a view of the base array's storage and copies no items. `Buffer`, `Value::buffer` and `from_buffer` are the public width-preserving frontend boundary.

Atoms, rank-0 arrays and singleton vectors are distinct. Function nodes retain handles, not lexical frames or Python objects. Functions match by their parts; dfns and generators match by identity. Operators can be values and can cross the native Python boundary. Values and function nodes are `Send + Sync`.

Numeric storage fills with zero and characters with blank. Mixed nonempty arrays derive their prototype from their first item on demand; empty arrays require one explicitly. Builders accept a value or a lazy prototype function and evaluate the function only for an empty result. Preserve function handles inside prototypes.

`Layout` owns dimensions, position keys, axis names and optional renderer metadata. Agreement unites keys and broadcasts through index maps. Missing positions read each source's prototype. Agreement pairs equal axis names first. It pairs remaining axes positionally unless both have different names. Result assembly drops colliding axis names; public name attachment rejects duplicates. Contracted axes require equal key sets and retain the first order. Nonempty key lists with no named positions become unkeyed; empty record keys survive.

`Gather` owns copying and widening. `Steps`, from rustymath's `gather`, describes strides, ranges/fills, rotations and offset tables. `Gather::walk` copies compact storage through `rustymath::gather::walk`. Structural operations should use `remap`/gathering rather than reconstructing `Value`s per item. Equal-shape cells copy in blocks. `Items` reads counts, positions and axes from compact storage without boxing every item. Shape, tally, positions and predicates have dedicated integer/Boolean builders.

Selective assignment and structural Under share label-based target discovery. Function-Under applies its selector to both arguments independently before calling its operand; labels then identify writeback into the right argument. Positions-Under passes the left argument whole. Keep those contracts distinct. Keyed values align to selected keys, and nested paths retain empty-cell structure.

Selective assignment removes the binding before writing. `Arc::make_mut` copies only when another holder needs the old array. Repeated targets must read values written by earlier occurrences. Pervasive modifiers group repeated targets into rounds or a suitable column scan; they do not apply all targets to the original values. Append updates use `Value::append` when layouts permit it; rank/key/name changes fall back to ordinary catenation. Modified destructuring calls modifiers left to right; ordinary destructuring assigns right to left. An assignment's RHS runs once.

Rank/Each/Outer assemble cells through shared layout/prototype rules. A 0-cell preserves an atom or encloses an array item. For custom operands, empty Each/Outer/Rank calls substitute prototypes for empty arguments and use prototype mode. Explicit effects remain observable when a call runs. Catchable prototype-call failures produce prototype zero, but interrupt, timeout and unsupported errors propagate. Restore prototype mode on success and error.

Axis nodes select/move axes and reuse Rank when the function has no direct axis meaning. Vector-axis reduction uses the general composed-ravel rule, preserving axis order and the whole seed. Single-axis reductions retain direct kernels. Layout-only changes share storage. Structural results retain renderers where their rank permits; scans and in-place changes copy complete layouts. Match ignores renderers.

Inverse nodes propagate fixed-argument sides through composition. `f⁻¹←g` and `f∂←g` replace the binding with a `Ruled` node, which holds the function with its hand-written inverse and derivative. Calling it calls the function. `⁻¹` and `∂` use the rules, and Under counts an attached inverse as keeping every item. Candidate-building primitive inverses validate by applying the primitive again. Inverse scans share forward scan's seed/axis validation. Agenda calls its selector once, then passes the original arguments to the selected branch. Computed operands for Rank, Axis, Stencil and Agenda use shared operand evaluation. Python constructs these nodes directly, not by generating source.

## Numbers and backend ownership

`number.rs` owns scalar kinds, promotion and exactness. Integers and BigRational share an exact domain. Overflowing scalar integer arithmetic widens to exact rationals; integral rational results narrow back when possible. Booleans are a distinct kind. Arithmetic reads them as 0/1; logical operations preserve them. Match/search/grade compare their numeric value, while reference checks also distinguish representation and exactness.

Joining compact arrays widens Boolean -> integer -> float -> complex, and float widths join at the wider width. Item-by-item construction preserves exact/approximate kinds and stays mixed when needed; only approximate real/complex items widen together. Rearranging or updating mixed storage never scans it for compaction. Fresh computations choose storage from their result path. Keep this distinction between whole-array joining and element assembly.

Bare numbers, empty literals and NaN use the parsed float width. Top-level `evaluate_code` first validates the whole syntax, runs leading literal-record `•prefs` statements, then parses the remainder at the new width. A syntax error prevents header execution. The default width also applies to imported numbers, random data and float results from exact operands. Time/date/metadata values use f64. Formatting marks non-default widths; serialization marks every float for width-independent readback.

Float operations compute at the wider operand width. Exact operands round to that width first. f16 whole operations run on f32 copies through `pervasive::halves`, rounding exact inputs to f16 before widening and rounding float outputs back. The scalar `Number` path rounds each f16 step. Complex parts have f32/f64 widths; f16 becoming complex uses f32 parts. Scalar Complex32 arithmetic computes at Complex64 and rounds, while kernels can compute at f32. These paths need not agree in their last bits.

Floats use IEEE NaN, infinity and signed zero. Comparison treats NaN as unequal; match/search treat it as one value, and grade puts it last. Exact arithmetic involving division by zero can produce those float values. Floor/ceiling retain an exact operand beside infinity and give NaN beside NaN. Monadic floor/ceiling/sign turn whole finite results into exact integers, including big integers; nonfinite results remain floats. Use `Number::float(value, width)` and explicit-width array construction outside `number.rs` instead of implicit f64 conversion.

Only f64 comparisons use the session tolerance, from 0 to `1E-14`. f32/f16 compare exactly except fixed-point convergence, which uses four ulps; f64 convergence uses `1E-14` independently of the preference. Index/count/Boolean/character-offset conversion uses `near_integer` with the fixed default tolerance, not the comparison preference. The tolerant types and objects come from rustymath. Keep independent inside/outside-tolerance tests with exact/mixed controls; don't use language comparison as its own oracle.

`search.rs` selects compact exact or tolerant backends. Exact integer/Boolean/character storage uses keys; float searches use their width. Integer haystacks accept the whole-number shortcut only for exactly whole float needles. Tolerant indexing validates whole-key matches unless the haystack is exactly whole and falls back to buckets on mismatch. Classification intentionally chains tolerance through sorted runs, so it need not match every member to one representative. Other cells use exact hashes or first-number buckets with `Value::matches` confirmation, falling back to pairwise comparison where indexing is unsuitable. Grade is stable. Position buffers use u32 and become integer storage without another pass. `⍷` and monadic Key take each class's positions from `search::class_positions`, which keeps a class's own buffer for 32-bit positions and copies narrower ones once from the shared buffer. Dyadic Key selects rows through `Grouped::each` slices. Key with the operand `≢⊢`, which `≢∘⊢` also builds, counts its classes through `search::class_counts` and builds no positions.

`Domain` and `with_field!` select working reals, Complex64 or BigRational for generic numeric algorithms. Compact storage selects the domain without scanning its items. `number::read` converts once; `Field::array` builds compact float/complex results without a `Number` per item. Each caller lists the domains it supports. Compact complex input does not take a real path merely because all imaginary parts are zero; results built through `Value::complex` can normalize to float storage.

Rustymath owns the mathematical algorithms. Basedpl owns pervasion, storage, numeric promotion, axes and language policy. Approximate matrix solve/pseudoinverse and decomposition use rustymath factorizations. On macOS, rustymath sends much of this work to Accelerate, as its DEV.md describes. No platform-specific adapter belongs here. The Cargo configuration, for Cargo's own builds, and `pyproject.toml`, for the wheels, both set the macOS deployment minimum to 15. Exact solve/pseudoinverse/determinant use rational field algorithms. Generic permanent and polynomial helpers also run at floating/complex field types. Singular/wide matrix division gives least-norm solutions. LU eligibility and SVD cutoffs are rustymath policy, not duplicated here.

`•decompose`/`•decompose⁻¹` share domain reading and `Factor` dispatch, with options for kind, vectors and complete QR. Polynomial adapters choose compact evaluation or generic field operations; gradients differentiate coefficients before evaluating. Number-theory adapters supply cancellation checks and assemble result cells. Distribution constructors expose native density/CDF/quantile/sample functions. Compact continuous calls use slice objects. Discrete samples and finite discrete quantiles are exact integers.

`•rand seed` exposes roll/deal over a shared Xoshiro256PlusPlus stream behind a mutex. Its algorithm is fixed, but range/distribution sampler changes can change draws; the reference sequence detects that. Plain `¿` uses the thread-local generator. Sampling from a generator record uses its roll function to recover the same stream.

## Kernels, fusion and threading

A dispatch selects a primitive before its loop. Concrete `Kernels` tables call rustymath objects: `many` for slices and `one`/`checked` for scalar Numbers. `at_widths!` writes f32/f64 tables once. Arithmetic, circle-code conventions and scalar policy remain in the numeric modules rather than a second evaluator.

Cheap mixed kernels in `Mixed` convert as they read through `Source`. Other kernels use `Chunked`/`Side` to convert reusable chunks before calling one homogeneous implementation. Direct conversion loops are worthwhile for cheap arithmetic; sharing expensive kernels avoids a storage-type cross-product of machine code. Compile only operand pairings that can occur. Same-type arguments can reach rustymath in one call; other shapes expand reused chunks or use block mappings to avoid per-item division.

Custom elementwise closures use `rustymath::pairs` and its uninitialized append path. Function objects use their own slice forms. Keep per-item bodies inlinable across codegen units; passing a closure by reference can introduce an uninlined `Fn` shim. Pass it by value, or wrap an existing reference in a new closure. Hot loops use slices and fixed chunks rather than iterator helpers that introduce calls in the incremental build. Avoid captured mutable flags that live in memory and block vectorization.

A failed compact kernel leaves placeholder results and triggers a wider/exact/complex or scalar path for the whole call. Complex fallback is shared with `Number` policy. Overflow detection belongs to rustymath's `Overflowing`, checked reductions and checked products. Cheap integer multiplication uses bounds to select an exact width without a per-item overflow check. Add no blanket NaN/signed-zero canonicalization pass; algorithm-specific failure checks and rustymath's SIMD kernels remain part of their algorithms.

`fusion.rs` holds pending chains of eligible shape-preserving float calls. The binder extends a chain until it must read the value, including across nested/bound calls. Chains run block by block through the same pervasive kernels. Short chains run unfused unless a supported reduction consumes them directly. The module doc owns eligibility rules; don't copy its thresholds here. Fused and ordinary folds share rustymath's `reduce::along`/`fold_into` traversal.

Folds by algebraic float operations can regroup. Contiguous subtraction/division folds use `Alternating`; quotients fall back to item-by-item folding if their products leave the normal range. Boolean scans compose truth-table maps; comparison scans compute both possible totals in bulk and use `reduce::chosen`. Seeds, axis lanes and scalar fallback preserve the language's accumulation direction. Generic reductions stay right-associated and preserve operand call order/effects. Monadic `f.g` is the exception: it memoizes each minor's expansion, and a cached minor does not repeat operand calls or their effects. Determinant/permanent fast paths use the appropriate rustymath backend.

The automatic Rayon scheduler serves rustymath elementwise `many` and `pairs`, folds through `reduce::along`, fused chains and matrix products. Scans don't split. Decompositions split in the products of their blocked updates, the passes of their Hermitian and bidiagonal reductions, and their divide-and-conquer merges. `rank` sends dyadic `+.×` and `⌹`, monadic `⌹` and `-.×`, and `•decompose` over the 2-cells of an unkeyed stack with compact storage to `primitive::stacked` and `stacked_decompositions`. They run each cell through the rustymath function it uses alone. Rustymath's `parallel::stack_with` splits whole cells across threads in rounds, with `check_now` between rounds. A caller whose checks run far apart, such as a round or a rustymath `_with` callback, calls `check_now`, because `Context::check` reads the clock only on every 64th call. The scheduler adapts to call cost, uses `RAYON_NUM_THREADS` by default and accepts current-thread overrides through `parallel::set_threads` (0 default, 1 serial). `Session::evaluation` applies a session's `•prefs ["threads":n]` to each of its evaluations, then restores the thread's own setting. A session that never sets it keeps the evaluating thread's setting, such as the timing harness's one thread. Nested Rayon work does not split again; wasm is serial. A fused chain splits in pieces of whole blocks: an elementwise chain through `parallel::extended`, and a folded one through `reduce::fold_pieces`. On one thread an elementwise chain appends straight to its result, because copying each block into a piece's slots cost up to 1.2 copies there. Small converted chunks limit the work available to the scheduler. Don't infer multicore execution from a large array alone.

Packed numeric, Boolean and equality-count inner products use `rustymath::linalg`. Integer calls use `Inner::checked` before relying on an exact compact result. Structural gather/compress and interval index also reuse rustymath. Measure generic/backend changes in both projects; callers' codegen and layout determine costs that isolated kernels miss.

## Evaluation and frontend boundaries

`eval_with`/`call_with` install `EvalOptions`. Echo defaults to true; disabling it suppresses implicit display, not explicit output or display commands. Output is ordered `Output { kind, data }` MIME bundles. An optional sink receives events as produced; otherwise the evaluation retains them. Values, functions, operators, outputs and errors remain separate results.

`Context` carries source and session. Checks at interpreter boundaries and long loops count down before reading clocks/flags or calling the poll hook. Individual BigInt operations and bounded native kernels are not preempted. Native functions receive `&Context`; evaluator functions use `&mut Context`. An optional `Input` supplies `⎕` and `•nget "-"`; without one they raise IO. Python uses cooperative cancellation. The process worker can kill an unresponsive process after a grace period, losing its state; requests are never replayed.

`Session::call` resolves and calls a function expression on existing arrays without temporary argument bindings. Retained functions use `call_function_with`. Error objects retain source and byte spans, with Unicode-width display diagnostics. Python translates spans to character offsets; process JSON keeps bytes. Native function/operator handles cross the in-process Python boundary, not process JSON.

### Python

`python/basedpl/__init__.py` owns the shared `bpl` workspace and `Array`/result/error wrappers. `fn` aliases `bpl.fn`; builtins and magics share that workspace. Plain calls print explicit output and return Array, Function or Operator values. Capturing calls return `Result`. Capture mode is positional-only because keywords bind BPL names. `_Session.eval` and `.call` are the PyO3 entry points; `request` is their internal Rust helper.

`functions.py` constructs native nodes and lazy exports from Rust metadata. Builtins win name collisions with workspace names; operation names choose valence/currying. `__all__` omits Python-builtin collisions but leaves explicit imports available. Function calls with keywords use a keyed left argument. A single positional argument stays the right argument unchanged; zero or multiple positional arguments form a vector. Operator properties and `_Pending` assemble native nodes, never generated BPL source. `printing.py` reads node parts without evaluating or resolving names.

`.fn()` retains a late-bound expression. Calls resolve affected nodes against the calling session while sharing unaffected graph nodes. This preserves primitive fast paths, inverses and reduction identities. The binder still resolves ordinary names during execution. Cyclic resolution is a LIMIT error.

Arrays retain native values. `.py` and `.np` make copies; NumPy is optional and lazy. Integer/float/complex buffers preserve supported widths, except signed bytes import as int16 and wider unsigned integers as int64; uint64 above int64 raises ValueError. Boolean/f16/complex buffers use byte/bit/part views for transfer. Exact integers/fractions cross through native integers, not decimal strings. String/object arrays convert per item. Lists follow the shared import rule and dictionaries stay mixed. `syntax.py` wraps the native syntax tree with character spans and span-editing helpers.

Each `_Session` request waits for its evaluator mutex without the GIL, then evaluates on the caller's thread with the GIL released. A separate interrupt handle supports interruption from another thread. Signal polling briefly takes the GIL. Python input and optional streaming-output callbacks also cross at explicit boundary adapters. No Python numeric objects enter core values; arrays can be dropped on any thread without an unsafe Send implementation.

### Process, CLI and notebooks

`protocol.rs` owns JSON request/value/error/output encoding. Requests are objects with fields such as code, call, args and bindings. Exact integers use arbitrary-precision JSON numbers; fractions and complex values have tags. Output bytes use base64. The worker omits eligible keyed entries holding functions/operators and returns `value: null` for other non-exportable values, retaining display output. JSON data serialization is a separate contract and can reject such values.

`cli/src/worker.rs` processes JSON lines in order. A separate reader handles interrupt controls but never evaluates BPL. IDs are optional, and bad requests don't end the session. `python/basedpl/worker.py` owns process lifetime, deadlines and hard-kill fallback. Protocol serialization walks native values directly rather than building an intermediate JSON value tree for every reply.

CLI `-e`, files and REPL stream outputs to stdout. Failed writes interrupt execution; a closed pipe exits successfully. `-e`, files and interactive input provide `StandardInput`; program text read from stdin does not. The REPL parses for completeness without evaluating twice. Rustyline owns terminal editing/history/modes; only typed backtick sequences auto-expand. Paste/history/navigation cancel automatic expansion. Shared symbol metadata supplies completion/highlighting. Keep pseudo-terminal tests for Ctrl-C/Ctrl-D and paste behaviour.

The Jupyter adapter in `cli/src/kernel.rs` uses kernmini and a `ThreadWorker`. It keeps transport/interrupts responsive, maps explicit output to streams and implicit output to results, and routes notebook input through `input_request`. The wheel and local install use the same kernelspec. `inspection.rs` embeds the glyph/system pages for help and uses the same names as the REPL and Python. Change names and shortcuts in `symbols.rs`/shared layout data, not each frontend.

### Browser

`cargo wasm` builds an incremental release-profile npm package with wasm-bindgen glue, language/input assets, grammar and canvas support. `--out DIR` leaves the docs' local playground assets alone; `--strip` removes function-name metadata for distribution. Keep wasm-bindgen glue and crate versions aligned through the workspace lock. Avoid fat LTO here because rebuild cost and enlarged stack frames conflict with browser development and recursion limits. `wasm-objdump -h DIR/basedpl_wasm_bg.wasm` lists the module's section sizes, including `name`. `twiggy diff` matches functions by full name, including crate hashes that change between builds. To compare two builds by function, compare their `twiggy top -n 1000000 -f json` listings with the hashes removed.

Browser `Session.run` streams MIME events and returns a protocol error or null. `eval` transfers values through `js.rs` but discards output and cannot export BPL functions/operators; `bind` imports page values/functions. Typed arrays transfer without another page-worker copy. JavaScript functions live in a thread-local registry and core function nodes hold slots, preserving Send/Sync values. `host_web.rs` implements the browser's clock, synchronous I/O/fetch and RegExp behaviour. File/URL reads resolve against the configured base. Unsupported native filesystem/locale operations stay unavailable; browser `•nput` writes only stdout and `•js` is browser-only.

The playground and gallery load released assets from jsDelivr, or local `nbs/playground/` assets on localhost. Shared `bpl.js` owns editors, worker startup and output. Restarting a trapped/stopped worker loses its state. `worker.js`, `page.js` and `gallery.js` own their respective protocols/UI; `nbs/driver.js` documents canvas drawing data. Gallery demos are `.bpl` sources with shared preferences, checked natively by `language_examples`. Browser recursion tests cover both V8 compilers and fail with LIMIT before stack exhaustion.

## System functions and rendering

`system.rs` owns case-insensitive system names, valence, item mapping and result prototypes. Native records use ordinary function nodes and calls. Empty native item mapping returns its declared prototype without calling the function. `data::Options` is the shared keyed-option decoder; formats don't invent separate option parsers.

`minimize.rs` implements `⇄` on `rustymath::optimize`. Its `Goal` evaluates the operand at points in the start's layout. Where `Function::differentiable` holds, `Goal` also calls `f∂` for the gradient. `∂` itself uses the same test.

CSV columns, JSON arrays and Python lists share `data::imported`: a non-fill finite float or any nonfinite float triggers float promotion when every number converts losslessly. Finite fills alone do not trigger it. Otherwise items keep their kinds. JSON objects/dictionaries stay mixed. Binary data uses exact bytes. Writes validate before opening and create exclusively unless overwrite is explicit. CSV/JSON/XML exports use `•csv⁻¹`, `•json⁻¹` and `•xml⁻¹`.

`regex.rs` shares host regex state between match/replace native functions; hosts retain their own regex syntax. `xml.rs` represents keyed element trees and distinguishes XML escaping/validation from HTML raw script/style and void-element rules. SVG uses its own renderer. `image.rs` owns pictures, their options and animation timing, as plain pixel buffers. Its `codec` module decodes and encodes those buffers through ImageIO on Apple targets (`image/imageio.rs`) and through the `image` and `png` crates elsewhere (`image/codecs.rs`). Native/browser renderers use the MIME/byte formats their frontends support. `plot.rs` stores specs and draws SVG on demand; the renderer owns defaults/validation. Rendering options belong in their user pages and implementation comments.

`display.rs` owns plain/boxed/function display, elision and MIME rendering. Display settings affect implicit presentation, not returned values or `⍕`; explicit output prints all items. `•mime` calls a renderer with echo disabled and cancellation active. Failed implicit rendering falls back to text; interrupts/timeouts propagate. `canvas.rs` serializes drawing data and typed bytes; `canvas.js` and `nbs/driver.js` own browser playback. Keep formatting, serialization and rich output as separate contracts.

## Tests and reference data

`tests/core.rs` checks public storage/layout/API behaviour, ownership, parser diagnostics and recovery. Use `equiv_in!` for session workflows and `fails_in` for errors; expected expressions run in fresh sessions. Keep constructors for representation/prototype/buffer checks. Self-contained language cases belong in `tests/reference/core.bpl`. `cli/tests/cli.rs` and Python's pseudo-terminal tests exercise the installed process/editor boundary.

`reference.rs` serves Rust tests and worker requests. Every reference case uses a fresh session; documentation examples share a session within each BPL code block. `.bpl` files define active coverage. Captured foreign expectations have no exactness; local expectations compare exactness and structural values independently. Pending gaps belong in inventory JSONL, not passing error tests. See `tests/reference/README.md` for format and review/activation workflow. `python/basedpl/reference.py`, `bpltests.py` and `scripts/reference.py` own import, capture, inventory and worker checks. `lib/*.bpl` are readable dfn examples; modules import other library files with private relative names.

`tests/scaling.rs` measures time/allocation growth and result-size allocation budgets. `tests/speed.rs` reads native/browser budgets from `speed.txt` through `timing.rs`; the worker and Python measurement tools reuse it. `tests/units.rs` reads existing incremental object files and checks unit size without building. Python tests cover conversion/cancellation/frontend behaviour, not a second Rust semantic suite. Combine related checks into readable tests.

## Documentation and generated assets

`nbs/index.ipynb` generates the homepage and README. Reference pages in `nbs/glyphs/` and `nbs/system/` describe forms/options/errors; concept pages explain them. `sidebar.yml` owns navigation. Edit source notebooks/pages, not generated outputs. `magics.ipynb` and `dyalog.ipynb` export Python modules; other Python modules are hand-written. Cargo owns versions.

```bash
bpl-nb
bpl-nb nbs/index.ipynb --save
nbdev-readme
nbdev-test --save
nbdev-test nbs/dyalog.ipynb --flags dyalog
nbdev-preview
```

Quarto renders saved outputs. `bpl-nb --save` updates notebook outputs without changing execution counts. Use it for BPL examples/figures and `nbdev-test` for Python notebook code. Rust `language_examples` checks BPL page/gallery examples; pytest checks Python page examples. Use `BASEDPL_PAGE` to focus page checks. Don't run a full documentation build for routine output updates. Release preparation runs `scripts/prep.py` after `cargo develop`; it calls editor generation and nbdev's export/test/clean/README preparation.

`basedpl.editors.write()` generates syntax grammars/highlighters, editor templates, keyboard sources/bundles and glyph metadata from shared Rust symbols and keyboard data. It also writes JetBrains assets and Windows `editors/windows/layout.h`. `layout.json` owns composed input; captured `keyboards.json` supports generation without macOS. `basedpl.keyboards.capture()` refreshes native data when needed, and `keyboards.write()` generates regional macOS bundles/maps. Regenerate keyboard sources through these functions, not one-off kernel code. See `editors/jetbrains/README.md`, `editors/windows/README.md` and `scripts/install-macos.sh` for target-specific work. Rebuild the common icon only when its design changes.

`nbs/bpl.xml`, the APL/BQN grammars and vim rules share generated glyph classes. TextMate exports feed Monaco/Shiki. `fonts.css` owns the BPL/APL font choices; rerun affected magics after changing them. `tests/highlighting.mjs`, `input.mjs`, keylayout/Windows tests and glyph-page checks cover their respective generated interfaces. All artifact generation belongs in repository functions/scripts, not ad hoc kernel transformations.

## Release

Before approval, verify successful tests for the release commit and the relevant installed artifacts. Reserve isolated wheel installs for packaging-sensitive work or release preparation, using a small relevant subset of existing tests. `ship-rs-build --profile dist` builds the distributed configuration; bare `ship-rs-build` uses release for local checks.

Confirm the clean tree and `[workspace.package].version`, then run `ship-release` without flags. Python gets that version through `dynamic = ["version"]`. Fastship pushes/tags the current version, leaves publication to the Release workflow and bumps Cargo without rebuilding. There is no changelog step. Never commit, push, tag or publish without approval.
