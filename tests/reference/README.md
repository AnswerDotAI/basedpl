# Reference acceptance cases

The `.bpl` files are the tests, and you edit active cases in them directly. `provenance/` records where each case came from.

`core.bpl` holds BPL's own semantic cases. `lib.bpl` covers library code that the upstream examples leave untested. The other files cover ngn assertions/example programs, April core and library assertions/demos/setup, APLcart main/tacit catalogue rows, selected Dyalog documentation examples, and selected BQN documentation and BQNcrate examples.

Active cases use BPL spellings: `π` for APL's monadic `○`, `g⊗` for `∘.g`, `⍶`/`⍹` for `⍺⍺`/`⍵⍵`, `⍢` for `∇∇`, `•name` for system names, with output still written `⎕←`, `$a` and `$d` for `⎕A` and `⎕D`, `"…"` for multi-character strings written `'…'`, `[I]⌷Y` for first-axis bracket indexing `Y[I]`, `[I;J]⌷Y` for indexing several axes, and `f⍠A` for bracket axes `f[A]`. They follow the spacing rules in `meta/spacing.md`: vectors that aren't strands use brackets, `[x]` is a one-item vector, parentheses round a literal group it, spaces separate runs, and `↣` and `↢` replace `∘` and `⍛`. Positions and axes count from 0, so the positions in upstream code and in expectations captured with `⎕IO←1` are converted.

Run the active cases with:

```bash
cargo test --test reference -- --nocapture
```

Each reference case has a two-second deadline.

To run active cases, set `BASEDPL_CASE` to an ID or ID prefix:

```bash
BASEDPL_CASE=ngn:177 cargo test --test reference enabled_reference_cases -- --nocapture
```

To run the active cases of one file, set `BASEDPL_SOURCE` to its name without `.bpl`:

```bash
BASEDPL_SOURCE=core cargo test --test reference enabled_reference_cases -- --nocapture
```

Pending cases do not count as passing tests. New failures in active cases fail the suite. Code and expectation execute in separate fresh sessions. Tests compare shape, axis keys and names, nesting, data and prototype. Numeric comparisons use `rtol=1e-13` and `atol=1e-13` unless a case specifies its own. The bound is `max(absolute, relative × max(|actual|, |expected|))`. The default absolute tolerance treats every value below `1e-13` as zero. Round such residues to zero in expectations. The tolerances apply only to approximate numbers. Each number's exactness must match, and exact numbers compare exactly. Shape and nesting remain exact. Display expectations remain separate from array expectations.

## BPL record format

```bpl
⍝ ngn:177 — sin(pi/6) = .5
1e¯10>|.5-1○○÷6   ⍝ 1

```

Each record starts with `⍝ ID — description`, or `⍝ — description` without an ID. The description is optional. Add a short description when the purpose would not be obvious to a quick reader.

Write `code   ⍝ expected` when the whole line is under 70 characters and reads clearly. A multiline case puts its expectation on its last line in the same way, when that line and the expectation total under 40 characters. Readers accept inline expectations of any length. The first comment outside a quoted string separates code from expectation if it is on the last line. Expressions with existing code comments or significant separator whitespace keep the expectation on its own line. So do errors, multiline expectations and explicit-output assertions.

Longer single-line expressions use one line each. A case with more than two top-level statements, or with two on a line wider than 60 characters, puts each statement on its own line instead of separating them with `⋄`. When either expression is multiline and the expectation isn't inline, an exact `⍝ =>` line separates code from expectation. An empty line separates records. A record ends at the next case header, section heading or EOF. At EOF the separator and final newline are optional. Other blank lines belong to the expressions.

Errors use `⍝ error: DOMAIN ERROR` on the expectation line. A no-result expectation is the BPL expression `{}0`. A case overrides the default tolerances with an optional suffix on the header, such as `[rtol=1e-10]`. These are comparison tolerances, not APL `⎕CT`.

Group cases with `⍝⍝ Section name`. Sections are labels, not shared sessions. Each case must supply its own definitions and setup.

File cases can use `testpath`. When mentioned in the source, the runner binds it to a new path in a per-case temporary directory, removed after the check. The file does not yet exist. Write fixture contents before reading them. This works in scans, activation and the Rust suite.

```bpl
⍝⍝ Evaluation order

⍝ — The right argument prints before the left
(⎕←1)+(⎕←2)
3
⍝ ⎕: 2\n1

```

An optional final `⍝ ⎕: text` checks explicit output. Write `\n` for a newline and `\\` for a literal backslash. Output appears as a terminal shows it, without the final line ending: each `⎕←` ends a line, and text written with `"-" •nput` doesn't. Implicit display is disabled. The marker also checks output preceding an expected error. An empty `⍝ ⎕:` asserts silence. Omit it when output is not the subject of the test.

An optional `⍝ input: text` line, just before any `⍝ ⎕:` line, gives the lines that `⎕` reads, with the same escapes. `•nget "-"` reads the lines that remain. Without it, reading `⎕` is a `VALUE` error. Case headers, section headings, `⍝ =>`, `⍝ input:` and `⍝ ⎕:` lines are reserved fixture syntax.

During conversion, comments are extracted from descriptions, unchanged ngn assertions, or leading comments in example programs. Known import/review boilerplate is removed from reasons and adaptations. Other clauses are retained on the same header line. Comments are not paraphrased or corrected. Converted comments can therefore contain inaccurate source wording or lack a description where none can be extracted.

Use `basedpl.bpltests.parse(text)` to read records as `Case` objects with `id`, `comment`, `code`, `expect`, `rtol`, `atol`, `section`, optional `input` and `output` text and the header's `line`. `render(cases)` writes them back. The conversion checks preserve source text and reproduce captured values, shapes and recursive prototypes; they do not infer new expectations from the program under test.

Use `check_file(path)` from `basedpl.bpltests` to run a reference file through the installed extension and list its failures. It applies the same comparison and tolerances as `tests/reference.rs`. `check_page(path)` does the same for the BPL examples in a `.qmd` page.

## Attribution

Adapted ngn tests are copyright 2011–2018 Nikolay G. Nikolov under the included `LICENSE-ngn` (MIT). Adapted April tests are copyright 2017 Andrew Sengul under the included `LICENSE-april` (Apache-2.0). APLcart is copyright 2019 Adám Brudzewsky under the included `LICENSE-aplcart` (MIT, with its stated exception for table/website content). BQN documentation examples are copyright 2020 Marshall Lochbaum under the included `LICENSE-bqn` (ISC). BQNcrate is copyright 2019 Adám Brudzewsky and 2020 Marshall Lochbaum under the included `LICENSE-bqncrate` (MIT, with its stated exception for table/website content). The extraction, structured expectations, status annotations and documented dialect adaptations are basedpl modifications. These data and tools are test-only, not interpreter dependencies.

Dyalog documentation examples are copyright © 1982–2025 Dyalog Limited, licensed under [Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/), reproduced in `LICENSE-dyalog` including its warranty disclaimer. Source links and adaptations are recorded in `provenance/README.md`. These fixtures retain that licence; they do not change the interpreter's Apache-2.0 licence or imply Dyalog endorsement.
