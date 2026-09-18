# Reference acceptance cases

These files retain ngn assertions/example programs, April core and library assertions/demos/setup, APLcart main/tacit catalogue rows, and selected Dyalog documentation examples. Entries are not removed because miniapl cannot execute them yet. Explicit exclusions remain in the inventory with their reason.

Source entries are not necessarily executable tests. Many APLcart recipes have unbound arguments and no expected result. They need concrete examples. Library cases need their definitions and setup. Use the scanner below for current counts and failures; fixture reasons describe their last review, not necessarily today's implementation. Progress notes belong in `meta/`, not this README.

Run the active cases with:

```bash
cargo test --test reference -- --nocapture
```

Each JSONL row has a stable `id`, `code`, and `status`. A `reason` records adaptations or remaining work. Its original source, expectation or recipe is retained. `status` is the equivalent of commenting a test out:

| Status | Meaning |
|---|---|
| `active` | Execute in normal Rust tests. An independent structured value or error kind is required. |
| `pending` | Intended coverage that still needs implementation, an origin/dialect adaptation, concrete inputs, or an expectation. |
| `question` | Retain until the scope/semantic decision is resolved. The question list is in `meta/reference-questions.md`. |
| `excluded` | Conflicts with an explicit calculator exclusion. Do not execute. Keep the source and rationale. |

To enable a case:

1. Find its `id` in the source JSONL file. Check its recipe, prerequisites and original expectation. Do not treat another dialect as the specification.
2. Supply concrete `code` and `expected` or `expected_error` if missing. Array expectations contain `shape`, flat `data`, and `prototype`. Nested arrays use the same structure. Complex elements use `{"complex":[real,imag]}`. Real infinities use `{"infinity":1}` or `{"infinity":-1}`. `expected: null` explicitly expects no result; an absent expectation remains invalid. Derive expectations independently of miniapl.
3. Run the selected case, including while it is pending:

   ```bash
   MINIAPL_CASE=april:1684 cargo test --test reference pending_reference_case -- --ignored
   ```

4. Change its status to `active` and update its reason when the assertion passes and the semantic adaptation is reviewed. Run the normal suite.

Pending cases do not catch arbitrary failures or count as passing. A case without an expectation fails explicitly when selected. New failures in active cases fail the suite. Tests compare shape, nesting, data and prototype. Numeric comparisons are exact unless a case specifies `relative_tolerance` or `absolute_tolerance`. The bound is `max(absolute, relative × max(|actual|, |expected|))`. Absolute tolerance covers numerical solver roundoff near zero. Shape and nesting remain exact. Display expectations remain separate from array expectations.

## Find cases ready to enable

Use the Python API in a kernel to inspect and edit fixtures without dumping JSONL records:

```python
from miniapl.reference import Corpus
corpus = Corpus()  # tests/reference, relative to the repo cwd
corpus.find('format:', status='pending')
corpus.get('ngn:391', 'code', 'expected', 'oracle')
corpus.update('ngn:391', reason='nested formatting: ready to check')
corpus.update_many({'ngn:391': {'reason': 'reviewed'}, 'ngn:392': {'reason': 'reviewed'}})
```

`find` searches ID/code/reason and returns concise entries keyed by ID. Filter with `source` or `status`; `limit=None` returns all matches. `get` defaults to code/status/reason; `get_many` reads a batch. Request `'*'` explicitly for full records. Updates read fresh files, preserve unrelated fields, and report changed field names. Use `remove=['expected_error']` when replacing an error expectation with a value; `None` means JSON null, not deletion. Unknown IDs write nothing. Use these methods rather than reading and patching whole JSONL lines in the conversation.

For library recipes, `library_definitions(path)` extracts column-zero named definitions. `library_dependencies(definitions, code)` selects transitive references for review. It ignores strings/comments but does not resolve lexical shadowing. Supply the relevant module and imports, then remove false dependencies on local names. Embed the reviewed definitions and preceding setup in the case. Retain `original_code`, `library_source` and `library_definitions`; do not require the library checkout at test time.

Rebuild the installed command after Rust changes, then scan:

```bash
maturin develop --release
python scripts/reference.py scan
python scripts/reference.py show --source april
python scripts/reference.py show --status mismatch --match '∧|∨'
python scripts/reference.py activate --source april --match 'april:590\b'
```

`scan` checks pending cases with independent expectations and collects every outcome. It never edits fixtures. `show` defaults to passes; filter by source, result status or regex over ID/code/message. Use `--limit` to change the display count. `--details` dumps complete records and arrays; use it only for a narrow selection. `activate` changes the reviewed passing selection to active. It refuses fixture records changed since the scan. Review dialect, origin and prerequisites before activation; a passing result alone is not that review.

Rust's `reference::check` owns comparison for the test runner, worker and private Python `_check_reference(json_case, timeout)` API. Each case receives a fresh session inside a persistent worker. The scanner uses a 0.25-second cooperative deadline per case, adjustable with `--timeout`. An unresponsive process is killed after the client's grace period and replaced for the next case. The failed case is not retried. Random cases, missing expectations and scope questions are counted separately, not treated as execution failures.

The report defaults to `meta/reference-scan.json`. It contains each original fixture and its result, including actual structured values on mismatches. Numerical rounding allowances must be explicit per case. Semantic differences do not get a tolerance. CI runs ordinary offline Rust tests; it does not need the scanner or a worker process.

## Sources and adaptations

| Source | Snapshot | Files |
|---|---|---|
| [ngn/apl](https://github.com/abrudz/ngn-apl/tree/d156d4e2b33c178e82dac55a19244014700d4cfa) | `d156d4e2b33c178e82dac55a19244014700d4cfa` | `t.apl`, `examples/*.apl` and matching `.out` |
| [April](https://github.com/phantomics/april/tree/0001af6d518d0e8fdf6a7d1688dd92a2fd9b29df) | `0001af6d518d0e8fdf6a7d1688dd92a2fd9b29df` | `spec.lisp`, every `**/demo.lisp` assertion and provision |
| [APLcart](https://github.com/abrudz/aplcart/tree/f01e91e1b08a7ca611c6c93328831425426ef8de) | `f01e91e1b08a7ca611c6c93328831425426ef8de` | `table.tsv` and `tt.tsv`, including duplicate recipes with separate provenance |
| [Dyalog documentation](https://github.com/Dyalog/documentation/tree/6ccc87c6cedb0229f2c9747037ebce8da6387df1) | `6ccc87c6cedb0229f2c9747037ebce8da6387df1` | Selected primitive-function examples in `dyalog.jsonl` |

Dyalog cases use `dyalog:page:example` IDs. Source pages default to `language-reference-guide/docs/primitive-functions/{page}.md`; an explicit `source` is relative to `language-reference-guide/docs/`. Cases cover scalar maths, complex numbers, search/sets, folds, composition, rank, trains and products. The checkout is v21 documentation; expectations were captured independently in Dyalog 20.0.53963.0 with the settings below, not inferred from rounded display text. Adaptations remove prompts/comments and normalise spacing and `J` case. Per-case adaptations record changed arguments or origin. Explicit relative tolerances allow floating-point library rounding, not semantic differences.

For each new glyph, read its documented valences and select examples that establish distinct semantics. Add structured expectations, run them, and fix failures as part of that glyph's implementation rather than deferring discovered gaps.

Dyalog's two fixed-order float reduction examples are retained as explicit exclusions. miniapl permits reassociation of primitive float sums/products. Generic-function reduction and primitive scan order remain tested; do not replace excluded expectations with one compiler's chosen answer.

ngn uses origin 0 and has different prototype/dialect rules. Its original expressions and expectations are retained. Adapt index/axis operands to origin 1 and capture values and prototypes independently in Dyalog. Changed code is retained in `original_code`; changed origin is recorded in `original_origin`. Unadapted `origin: 0` describes upstream, not miniapl's execution settings. Closed literal right-hand expectations were evaluated independently in Dyalog, not with miniapl.

April's literal Common Lisp expectations were converted to structured values. Ordinary rational expectations represent approximate results under miniapl's numeric policy, not opt-in exact `r` literals. The power alias `⋆` is written as standard `*` outside quoted text. Printed-format expectations, host wrappers, and library dependencies remain visible for review.

APLcart's TIO links were decoded offline. All 972 available decoded programs are retained. No TIO service was contacted. Small closed calculator examples were checked in Dyalog 20.0.53963.0 with `⎕IO=1`, `⎕CT=1E¯14`, `⎕DIV=0`, `⎕ML=1`, and `⎕PP=17`. Multi-output examples collect their values in an array literal. The original program remains in `example`. The import does not execute arbitrary catalogue programs.

`scripts/reference.py` contains the import and developer-only reference capture functions. CI reads the checked-in JSONL files. It needs neither the sibling clones nor Dyalog, Common Lisp, Node, Python or network access. Import a new upstream snapshot into a new directory and review it against these files rather than replacing reviewed statuses.

For live reference work, use `aplnb.dyalog.Apl`, not `aplnb.core` (which uses miniapl):

```python
from aplnb.dyalog import Apl
from scripts.reference import REFERENCE_ENCODER, dyalog_expected
with Apl() as apl:
    apl('⎕IO←1 ⋄ ⎕CT←1E¯14 ⋄ ⎕DIV←0 ⋄ ⎕ML←1 ⋄ ⎕PP←17')
    apl(REFERENCE_ENCODER)
    expected = dyalog_expected(apl, '2 3⍴⍳6')
```

Non-language repository material is not an acceptance case: ngn's browser assets, April's bundled Common Lisp parser implementation tests, and APLcart's website/quiz implementation and publication bibliography. The bibliography is `pub/pub.tsv`, not another recipe table. No algorithm or mathematical recipe was discarded for being difficult, unimplemented, or beyond the current lesson set.

## Attribution

Adapted ngn tests are copyright 2011–2018 Nikolay G. Nikolov under the included `LICENSE-ngn` (MIT). Adapted April tests are copyright 2017 Andrew Sengul under the included `LICENSE-april` (Apache-2.0). APLcart is copyright 2019 Adám Brudzewsky under the included `LICENSE-aplcart` (MIT, with its stated exception for table/website content). The extraction, structured expectations, status annotations and documented dialect adaptations are miniapl modifications. These data and tools are test-only, not interpreter dependencies.

Dyalog documentation examples are copyright © 1982–2025 Dyalog Limited, licensed under [Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/), reproduced in `LICENSE-dyalog` including its warranty disclaimer. Source links and adaptations are recorded above. These fixtures retain that licence; they do not change the interpreter's Apache-2.0 licence or imply Dyalog endorsement.
