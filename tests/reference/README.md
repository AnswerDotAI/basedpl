# Reference acceptance cases

The `.apl` files are the executable language tests. `core.apl` holds bAsedPL's own semantic cases. The other files cover ngn assertions/example programs, April core and library assertions/demos/setup, APLcart main/tacit catalogue rows, and selected Dyalog documentation examples. The tracked `inventory/*.jsonl` files retain original records, independent expectations, adaptations and candidates for activation. Entries are not removed because bAsedPL cannot execute them yet. Explicit exclusions remain in the inventory with their reason.

Source entries are not necessarily executable tests. Many APLcart recipes have unbound arguments and no expected result. They need concrete examples. Library cases need their definitions and setup. Use the scanner below for current counts and failures; fixture reasons describe their last review, not necessarily today's implementation. Progress notes belong in `meta/`, not this README.

Active cases use bAsedPL spellings: `π` for APL's monadic `○`, `g⌝` for `∘.g`, `⍶`/`⍹` for `⍺⍺`/`⍵⍵`, and `•Name` for system names. Original inventory sources retain their dialect's notation.

Run the active cases with:

```bash
cargo test --test reference -- --nocapture
```

To run one active case, set `BASEDPL_CASE` to its exact ID:

```bash
BASEDPL_CASE=ngn:177 cargo test --test reference enabled_reference_cases -- --nocapture
```

Every case in `.apl` runs regardless of inventory status. Edit these files directly once cases are active. Each JSONL inventory row has a stable `id`, `code`, and `status`. A `reason` records adaptations or remaining work. Its original source, expectation or recipe is retained.

| Status | Meaning |
|---|---|
| `active` | Exported to the `.apl` corpus. |
| `setup` | Upstream initialization retained for self-contained cases; no standalone assertion. |
| `pending` | Intended coverage that still needs implementation, an origin/dialect adaptation, concrete inputs, or an expectation. |
| `question` | Retain until the scope/semantic decision is resolved. The question list is in `meta/reference-questions.md`. |
| `excluded` | Conflicts with an explicit calculator exclusion. Do not execute. Keep the source and rationale. |

To enable a case:

1. Find its `id` in the inventory. Check its recipe, prerequisites and original expectation. Do not treat another dialect as the specification.
2. Supply concrete `code` and `expected` or `expected_error` if missing. Array expectations contain `shape`, flat `data`, and `prototype`. Nested arrays use the same structure. Complex elements use `{"complex":[real,imag]}`. Real infinities use `{"infinity":1}` or `{"infinity":-1}`. `expected: null` explicitly expects no result; an absent expectation remains invalid. Derive expectations independently of bAsedPL.
3. Activate the reviewed case:

   ```bash
   python -m basedpl.apltests add april:1684
   ```

   This checks the program against its independent expectation and checks the converted expectation against the captured value. It appends to the source's `.apl` file and marks the inventory record active. Excluded cases, missing expectations, failed checks and duplicate IDs are rejected before writing.

4. Run the normal suite. To check candidates without activating them, use the scanner below.

Pending cases do not count as passing tests. New failures in active cases fail the suite. Code and expectation execute in separate fresh sessions. Tests compare shape, nesting, data and prototype. Numeric comparisons are exact unless a case specifies `rtol` or `atol` (`relative_tolerance` or `absolute_tolerance` in the inventory). The bound is `max(absolute, relative × max(|actual|, |expected|))`. Absolute tolerance covers numerical solver roundoff near zero. Shape and nesting remain exact. Display expectations remain separate from array expectations.

Implementation gaps stay in the JSONL inventory with `status: pending` and a `Not implemented:` reason. Record the intended result when settled. Do not turn the current failure into an active error expectation. Active error cases assert invalid language operations or explicit scope exclusions.

Keep unresolved original workloads and inputs pending with their failure details. Useful smaller-workload or exact-arithmetic variants are separate cases; passing them does not resolve the originals.

## Find cases ready to enable

Use the Python API in a kernel to inspect and edit fixtures without dumping JSONL records:

```python
from basedpl.reference import Corpus
corpus = Corpus()  # tests/reference/inventory, relative to the repo cwd
corpus.find('format:', status='pending')
corpus['ngn:391', 'code', 'expected', 'oracle']
corpus.update('ngn:391', reason='nested formatting: ready to check')
corpus.update_many({'ngn:391': {'reason': 'reviewed'}, 'ngn:392': {'reason': 'reviewed'}})
```

`find` searches full ID/code/reason text and returns single-line previews of at most 180 characters, keyed by ID. Filter with `source` or `status`; `limit=None` returns all matches. `corpus[id]` returns code/status/reason. Use `corpus[id, 'code', 'expected']` for selected fields or `corpus[id, '*']` for the full record. Unknown IDs raise `KeyError`. Use `get_many(ids, *fields)` for bulk reads. Updates read fresh files, preserve unrelated fields, and report changed field names. Use `remove=['expected_error']` when replacing an error expectation with a value; `None` means JSON null, not deletion. Unknown IDs write nothing. Use these methods rather than reading and patching whole JSONL lines in the conversation.

Library recipes use the shared ports in `lib/`: start a case with `•LOAD 'lib/numeric.apl'`, for example. Keep case-specific setup in the case. Combine related examples only when their combined expectation stays clear. `library_definitions(path)` reads top-level named assignments; `source_definitions(text)` does the same for a string. `library_dependencies(definitions, code)` selects transitive references for review, ignoring strings/comments but not resolving lexical shadowing. Retain original source and adaptation metadata in the inventory. The upstream checkout is only needed when reviewing new ports.

Additional Dyalog workspace ports live in `lib/dyalog.apl`. Their APLcart inventory entries retain `original_definition`, `definition_source` and `definition_version`. Unported definitions keep `pending` status and a reason naming the remaining work. Licence confirmation for these workspace sources is pending; the Dyalog documentation licence below covers documentation examples.

Rebuild the installed command after Rust changes, then scan:

```bash
maturin develop --release
python scripts/reference.py scan
python scripts/reference.py show --source april
python scripts/reference.py show --status mismatch --match '∧|∨'
python scripts/reference.py activate --source april --match 'april:590\b'
```

`scan` checks pending cases with independent expectations and collects every outcome. It never edits fixtures. `show` defaults to passes; filter by source, result status or regex over ID/code/message. Use `--limit` to change the display count. `--details` dumps complete records and arrays; use it only for a narrow selection. `activate` appends the reviewed passing selection to `.apl` and marks its inventory records active. It refuses fixture records changed since the scan and rechecks the selected cases before writing. Review dialect, origin and prerequisites before activation; a passing result alone is not that review.

Rust's `reference::check` owns comparison for the test runner, worker and private Python `_check_reference(json_case, timeout)` API. It accepts captured `expected` arrays, `expected_error` kinds, or an `expected_code` expression. Each case receives a fresh session inside a persistent worker. The scanner uses a 0.25-second cooperative deadline per case, adjustable with `--timeout`. An unresponsive process is killed after the client's grace period and replaced for the next case. The failed case is not retried. Random cases, missing expectations and scope questions are counted separately, not treated as execution failures.

The report defaults to `meta/reference-scan.json`. It contains each original fixture and its result, including actual structured values on mismatches. Numerical rounding allowances must be explicit per case. Semantic differences do not get a tolerance. CI runs ordinary offline Rust tests; it does not need the scanner or a worker process.

## APL record format

```apl
⍝ ngn:177 — sin(pi/6) = .5
1e¯10>|.5-1○○÷6   ⍝ 1

```

Each record starts with `⍝ ID — description`, or `⍝ — description` without an ID. The description is optional. Add a short description when the purpose would not be obvious to a quick reader.

Write `code   ⍝ expected` when both expressions fit one line and each has fewer than 40 characters. Readers accept inline records of any length. The first comment outside a quoted string separates code from expectation. Expressions with existing code comments or significant separator whitespace retain two lines. Errors and explicit-output assertions also retain two lines.

Longer single-line expressions use one line each. If either expression is multiline, an exact `⍝ =>` line separates code from expectation. An empty line separates records. A record ends at the next case header, section heading or EOF. At EOF the separator and final newline are optional. Other blank lines belong to the expressions.

Errors use `⍝ error: DOMAIN ERROR` on the expectation line. A no-result expectation is the APL expression `{}0`. Numerical tolerances use an optional suffix on the header, such as `[rtol=1e-14 atol=1e-15]`. These are comparison tolerances, not APL `⎕CT`. `core.apl` instead uses exact Rust array equality, including numeric domains and prototypes, as its original Rust assertions did.

Group cases with `⍝⍝ Section name`. Sections are labels, not shared sessions. Each case must supply its own definitions and setup.

```apl
⍝⍝ Evaluation order

⍝ — The right argument prints before the left
(⎕←1)+(⎕←2)
3
⍝ ⎕: 2\n1

```

An optional final `⍝ ⎕: text` checks explicit output. Write `\n` for a newline and `\\` for a literal backslash. Output events are joined with newlines; implicit display is disabled. The marker also checks output preceding an expected error. An empty `⍝ ⎕:` asserts silence. Omit it when output is not the subject of the test. Case headers, section headings, `⍝ =>` and `⍝ ⎕:` lines are reserved fixture syntax.

During conversion, comments are extracted from descriptions, unchanged ngn assertions, or leading comments in example programs. Known import/review boilerplate is removed from reasons and adaptations. Other clauses are retained on the same header line. Comments are not paraphrased or corrected. Converted comments can therefore contain inaccurate source wording or lack a description where none can be extracted.

Use `basedpl.apltests.parse(text)` to read records as `Case` objects with `id`, `comment`, `code`, `expect`, `rtol`, `atol`, `section`, optional `output` text and the header's `line`. `render(cases)` writes them back. The conversion checks preserve source text and reproduce captured values, shapes and recursive prototypes; they do not infer new expectations from the program under test.

Use `add(['ngn:177'])` from `basedpl.apltests` to activate selected inventory IDs from a kernel. It is the equivalent of the `add` command above.

The converter remains available for inspecting a fresh conversion without overwriting edited tests:

```bash
python -m basedpl.apltests preview --replace
pytest -q tests/test_apltests.py
```

The output is `meta/apl-preview/{ngn,april,aplcart,dyalog,core}.apl`. The reference previews contain records marked active in the inventory. The core preview extracts any remaining fully literal `equiv!` tables and `fails` lists from Rust; it does not reproduce the curated `core.apl`. Storage, ownership, parser diagnostics, API behaviour and cross-call recovery checks remain in Rust. Preview generation does not change the inventory or executable corpus. Do not regenerate the executable corpus from the inventory after editing `.apl` tests.

## Sources and adaptations

Active cases use bAsedPL's postfix `g⌝` for outer product. The inventory retains upstream `∘.g` spellings.

System names use `•` in active cases (`•C`, `•UCS`, etc.). The inventory retains upstream `⎕` spellings. `⎕←` is output in both.

| Source | Snapshot | Files |
|---|---|---|
| [ngn/apl](https://github.com/abrudz/ngn-apl/tree/d156d4e2b33c178e82dac55a19244014700d4cfa) | `d156d4e2b33c178e82dac55a19244014700d4cfa` | `t.apl`, `examples/*.apl` and matching `.out` |
| [April](https://github.com/phantomics/april/tree/0001af6d518d0e8fdf6a7d1688dd92a2fd9b29df) | `0001af6d518d0e8fdf6a7d1688dd92a2fd9b29df` | `spec.lisp`, every `**/demo.lisp` assertion and provision |
| [APLcart](https://github.com/abrudz/aplcart/tree/f01e91e1b08a7ca611c6c93328831425426ef8de) | `f01e91e1b08a7ca611c6c93328831425426ef8de` | `table.tsv` and `tt.tsv`, including duplicate recipes with separate provenance |
| [Dyalog documentation](https://github.com/Dyalog/documentation/tree/6ccc87c6cedb0229f2c9747037ebce8da6387df1) | `6ccc87c6cedb0229f2c9747037ebce8da6387df1` | Selected primitive-function examples in `inventory/dyalog.jsonl` |

Dyalog cases use `dyalog:page:example` IDs. Source pages default to `language-reference-guide/docs/primitive-functions/{page}.md`; an explicit `source` is relative to `language-reference-guide/docs/`. Cases cover scalar maths, complex numbers, search/sets, folds, composition, rank, trains and products. The checkout is v21 documentation; expectations were captured independently in Dyalog 20.0.53963.0 with the settings below, not inferred from rounded display text. Adaptations remove prompts/comments and normalise spacing and `J` case. Per-case adaptations record changed arguments or origin. Explicit relative tolerances allow floating-point library rounding, not semantic differences.

For each new glyph, read its documented valences and select examples that establish distinct semantics. Add structured expectations, run them, and fix failures as part of that glyph's implementation rather than deferring discovered gaps.

Dyalog's two fixed-order float reduction examples are retained as explicit exclusions. bAsedPL permits reassociation of primitive float sums/products. Generic-function reduction and primitive scan order remain tested; do not replace excluded expectations with one compiler's chosen answer.

ngn uses origin 0 and has different prototype/dialect rules. Its original expressions and expectations are retained. Adapt index/axis operands to origin 1 and capture values and prototypes independently in Dyalog. Changed code is retained in `original_code`; changed origin is recorded in `original_origin`. Unadapted `origin: 0` describes upstream, not bAsedPL's execution settings. Closed literal right-hand expectations were evaluated independently in Dyalog, not with bAsedPL.

April's literal Common Lisp expectations were converted to structured values. Ordinary rational expectations represent approximate results under bAsedPL's numeric policy, not opt-in exact `r` literals. The power alias `⋆` is written as standard `*` outside quoted text. Printed-format expectations, host wrappers, and library dependencies remain visible for review.

APLcart's TIO links were decoded offline. All 972 available decoded programs are retained. No TIO service was contacted. Small closed calculator examples were checked in Dyalog 20.0.53963.0 with `⎕IO=1`, `⎕CT=1E¯14`, `⎕DIV=0`, `⎕ML=1`, and `⎕PP=17`. Multi-output examples collect their values in an array literal. The original program remains in `example`. The import does not execute arbitrary catalogue programs.

`basedpl.reference` contains the import, reference capture, scan, review and activation functions. `scripts/reference.py` is their CLI. Rust tests read the checked-in `.apl` files. They need neither the JSONL inventory, sibling clones, Dyalog, Common Lisp, Node, Python nor network access. Python converter tests also check serialization against the tracked inventory. Import a new upstream snapshot into a new directory and review it against the inventory rather than replacing reviewed statuses.

For live reference work, use `aplnb.dyalog.Apl`, not `aplnb.core` (which uses bAsedPL):

```python
from aplnb.dyalog import Apl
from basedpl.reference import REFERENCE_ENCODER, dyalog_expected
with Apl() as apl:
    apl('⎕IO←1 ⋄ ⎕CT←1E¯14 ⋄ ⎕DIV←0 ⋄ ⎕ML←1 ⋄ ⎕PP←17')
    apl(REFERENCE_ENCODER)
    expected = dyalog_expected(apl, '2 3⍴⍳6')
```

Non-language repository material is not an acceptance case: ngn's browser assets, April's bundled Common Lisp parser implementation tests, and APLcart's website/quiz implementation and publication bibliography. The bibliography is `pub/pub.tsv`, not another recipe table. No algorithm or mathematical recipe was discarded for being difficult, unimplemented, or beyond the current lesson set.

## Attribution

Adapted ngn tests are copyright 2011–2018 Nikolay G. Nikolov under the included `LICENSE-ngn` (MIT). Adapted April tests are copyright 2017 Andrew Sengul under the included `LICENSE-april` (Apache-2.0). APLcart is copyright 2019 Adám Brudzewsky under the included `LICENSE-aplcart` (MIT, with its stated exception for table/website content). The extraction, structured expectations, status annotations and documented dialect adaptations are basedpl modifications. These data and tools are test-only, not interpreter dependencies.

Dyalog documentation examples are copyright © 1982–2025 Dyalog Limited, licensed under [Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/), reproduced in `LICENSE-dyalog` including its warranty disclaimer. Source links and adaptations are recorded above. These fixtures retain that licence; they do not change the interpreter's Apache-2.0 licence or imply Dyalog endorsement.
