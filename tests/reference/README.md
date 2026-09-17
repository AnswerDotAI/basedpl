# Reference acceptance cases

These files retain all 734 assertions and 8 example programs from ngn, all 1,138 April core assertions/demos and 675 library/demo assertions/setup entries, and all 5,747 APLcart main/tacit catalogue rows. Entries are not removed because miniapl cannot execute them yet. Explicit exclusions remain in the inventory with their reason.

The initial import has 538 active cases: 140 ngn, 324 April and 74 APLcart. There are 5,143 pending entries, 2,154 scope questions and 467 explicit exclusions. These are source entries, not a claim that 8,302 executable conformance tests exist. In particular, many APLcart entries are recipes with unbound arguments and no expected result. They still need concrete examples. Library cases need their definitions and setup.

Run the active cases with:

```bash
cargo test --test reference -- --nocapture
```

Each JSONL row has a stable `id`, `code`, `status`, and `reason`. Its original source, expectation or recipe is retained. `status` is the equivalent of commenting a test out:

| Status | Meaning |
|---|---|
| `active` | Execute in normal Rust tests. An independent structured value or error kind is required. |
| `pending` | Intended coverage that still needs implementation, an origin/dialect adaptation, concrete inputs, or an expectation. |
| `question` | Retain until the scope/semantic decision is resolved. The question list is in `meta/reference-questions.md`. |
| `excluded` | Conflicts with an explicit calculator exclusion. Do not execute. Keep the source and rationale. |

To enable a case:

1. Find its `id` in the source JSONL file. Check its recipe, prerequisites and original expectation. Do not treat another dialect as the specification.
2. Supply concrete `code` and `expected` or `expected_error` if missing. `expected` contains `shape`, flat `data`, and `prototype`. Nested arrays use the same structure. Complex elements use `{"complex":[real,imag]}`. Derive expectations independently of miniapl.
3. Run the selected case, including while it is pending:

   ```bash
   MINIAPL_CASE=april:1684 cargo test --test reference pending_reference_case -- --ignored
   ```

4. Change its status to `active` and update its reason when the assertion passes and the semantic adaptation is reviewed. Run the normal suite.

Pending cases do not catch arbitrary failures or count as passing. A case without an expectation fails explicitly when selected. New failures in active cases fail the suite. Tests compare shape, nesting, data and prototype. Two initial floating-arithmetic cases have explicit `relative_tolerance: 1e-14` for reference rounding differences. Other comparisons are exact. Display expectations remain separate from array expectations.

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

ngn uses origin 0 and has different prototype/dialect rules. Its original expressions and expectations are retained. Only origin-independent cases are active from the automatic first pass. The `origin: 0` field describes upstream, not miniapl's execution settings. Closed literal right-hand expectations were evaluated independently in Dyalog, not with miniapl.

April's literal Common Lisp expectations were converted to structured values. Ordinary rational expectations represent approximate results under miniapl's numeric policy, not opt-in exact `r` literals. The power alias `⋆` is written as standard `*` outside quoted text. Printed-format expectations, host wrappers, and library dependencies remain visible for review.

APLcart's TIO links were decoded offline. All 972 available decoded programs are retained. No TIO service was contacted. Small closed calculator examples were checked in Dyalog 20.0.53963.0 with `⎕IO=1`, `⎕CT=1E¯14`, `⎕DIV=0`, `⎕ML=1`, and `⎕PP=17`. Multi-output examples collect their values in an array literal. The original program remains in `example`. The import does not execute arbitrary catalogue programs.

`scripts/reference.py` contains the import and developer-only reference capture functions. CI reads the checked-in JSONL files. It needs neither the sibling clones nor Dyalog, Common Lisp, Node, Python or network access. Import a new upstream snapshot into a new directory and review it against these files rather than replacing reviewed statuses.

Non-language repository material is not an acceptance case: ngn's browser assets, April's bundled Common Lisp parser implementation tests, and APLcart's website/quiz implementation and publication bibliography. The bibliography is `pub/pub.tsv`, not another recipe table. No algorithm or mathematical recipe was discarded for being difficult, unimplemented, or beyond the current lesson set.

## Attribution

Adapted ngn tests are copyright 2011–2018 Nikolay G. Nikolov under the included `LICENSE-ngn` (MIT). Adapted April tests are copyright 2017 Andrew Sengul under the included `LICENSE-april` (Apache-2.0). APLcart is copyright 2019 Adám Brudzewsky under the included `LICENSE-aplcart` (MIT, with its stated exception for table/website content). The extraction, structured expectations, status annotations and documented dialect adaptations are miniapl modifications. These data and tools are test-only, not interpreter dependencies.

Dyalog documentation examples are copyright © 1982–2025 Dyalog Limited, licensed under [Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/), reproduced in `LICENSE-dyalog` including its warranty disclaimer. Source links and adaptations are recorded above. These fixtures retain that licence; they do not change the interpreter's Apache-2.0 licence or imply Dyalog endorsement.
