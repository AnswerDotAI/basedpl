# Provenance

The JSONL files here record provenance: where each reference case came from, and its original upstream form. The tests are the `.bpl` files in `tests/reference/`. The inventory never runs, and a change to BPL never requires editing it.

The files retain original records, independent expectations, adaptations and candidates for activation. Entries are not removed because BPL cannot execute them yet. Excluded records are kept only where they still inform a decision. Records keep their source dialect's notation, such as `∘.g` for outer product and `⎕` names.

Source entries are not necessarily executable tests. Many APLcart recipes have unbound arguments and no expected result. They need concrete examples. Library cases need their definitions and setup. Use the scanner below for current counts and failures; fixture reasons describe their last review, not necessarily today's implementation. Progress notes belong in `meta/`, not this README.

Each JSONL inventory row has a stable `id`, `code`, and `status`. `code` records the case as it was ported. A `reason` records adaptations or remaining work. Its original source, expectation or recipe is retained. Optional `relative_tolerance` and `absolute_tolerance` override the default comparison tolerances.

| Status | Meaning |
|---|---|
| `active` | Exported to the `.bpl` corpus. |
| `duplicate` | Same code and expectation as the record named in `duplicate_of`. Only that record is exported, once it's active. |
| `setup` | Upstream initialization retained for self-contained cases; no standalone assertion. |
| `pending` | Intended coverage that still needs implementation, an origin/dialect adaptation, concrete inputs, or an expectation. |
| `question` | Retain until the scope/semantic decision is resolved. The question list is in `meta/reference-questions.md`. |
| `excluded` | Conflicts with an explicit calculator exclusion, or records a decided difference from the source. Do not execute. Keep the source and rationale. |

To enable a case:

1. Find its `id` in the inventory. Check its recipe, prerequisites and original expectation. Do not treat another dialect as the specification.
2. Supply concrete `code` and `expected` or `expected_error` if missing. Array expectations contain `shape`, flat `data`, and `prototype`. Nested arrays use the same structure. Complex elements use `{"complex":[real,imag]}`. Real infinities use `{"infinity":1}` or `{"infinity":-1}`. `expected: null` explicitly expects no result; an absent expectation remains invalid. Derive expectations independently of BPL.
3. Activate the reviewed case:

   ```python
   from basedpl.bpltests import add
   add(['april:1684'])
   ```

   This appends the case to the source's `.bpl` file and marks the inventory record active. It doesn't run the program. It rejects excluded cases, missing expectations and duplicate IDs before writing. A captured expectation's numbers are written as approximate literals. Where the language's rules give exact results, mark them with `ₓ`.

4. Run the normal suite. To check candidates without activating them, use the scanner below.

Implementation gaps stay in the JSONL inventory with `status: pending` and a `Not implemented:` reason. Record the intended result when settled. Do not turn the current failure into an active error expectation. Active error cases assert invalid language operations or explicit scope exclusions.

Keep unresolved original workloads and inputs pending with their failure details. Useful smaller-workload or exact-arithmetic variants are separate cases; passing them does not resolve the originals.

## Find cases ready to enable

Use the Python API in a kernel to inspect and edit fixtures without dumping JSONL records:

```python
from basedpl.reference import Corpus
corpus = Corpus()  # provenance/, relative to the repo cwd
corpus.find('format:', status='pending')
corpus['ngn:391', 'code', 'expected', 'oracle']
corpus.update('ngn:391', reason='nested formatting: ready to check')
corpus.update_many({'ngn:391': {'reason': 'reviewed'}, 'ngn:392': {'reason': 'reviewed'}})
```

`find` searches full ID/code/reason text and returns single-line previews of at most 180 characters, keyed by ID. Filter with `source` or `status`; `limit=None` returns all matches. `corpus[id]` returns code/status/reason. Use `corpus[id, 'code', 'expected']` for selected fields or `corpus[id, '*']` for the full record. Unknown IDs raise `KeyError`. Use `get_many(ids, *fields)` for bulk reads. Updates read fresh files, preserve unrelated fields, and report changed field names. Use `remove=['expected_error']` when replacing an error expectation with a value; `None` means JSON null, not deletion. Unknown IDs write nothing. Use these methods rather than reading and patching whole JSONL lines in the conversation.

Library recipes use the shared ports in `lib/`: start a case by taking the names it uses, as in `[phinary]←•load "lib/numeric.bpl"`. Support files in `support/` are modules too. Keep case-specific setup in the case. Combine related examples only when their combined expectation stays clear. `library_definitions(path)` reads top-level named assignments; `source_definitions(text)` does the same for a string. `library_dependencies(definitions, code)` selects transitive references for review, ignoring strings/comments but not resolving lexical shadowing. Retain original source and adaptation metadata in the inventory. The upstream checkout is only needed when reviewing new ports.

Additional Dyalog workspace ports live in `lib/dyalog.bpl`. Their APLcart inventory entries retain `original_definition`, `definition_source` and `definition_version`. Unported definitions keep `pending` status and a reason naming the remaining work. Licence confirmation for these workspace sources is pending; the Dyalog documentation licence below covers documentation examples.

Rebuild the extension with `python scripts/develop.py` after Rust changes, then scan from Python:

```python
from basedpl.reference import scan, review, activate
scan()
review(source='april')
review(status='mismatch', match='∧|∨')
activate(source='april', match=r'april:590\b')
```

`scan` checks pending cases with independent expectations and collects every outcome. A captured expectation carries no exactness, so the scan compares its numbers by value. It never edits fixtures. `show` defaults to passes; filter by source, result status or regex over ID/code/message. Use `limit` to change the display count. `details=True` dumps complete records and arrays; use it only for a narrow selection. `activate` appends the reviewed passing selection to `.bpl` and marks its inventory records active. It refuses fixture records changed since the scan and rechecks the selected cases before writing. Review dialect, origin and prerequisites before activation; a passing result alone is not that review.

Rust's `reference::check` owns comparison for the test runner, worker and private Python `_check_reference(json_case, timeout)` API. It accepts captured `expected` arrays, `expected_error` kinds, or an `expected_code` expression. Each case receives a fresh session inside a persistent worker. The scanner uses a 0.25-second cooperative deadline per case, adjustable with `--timeout`. An unresponsive process is killed after the client's grace period and replaced for the next case. The failed case is not retried. Random cases, missing expectations and scope questions are counted separately, not treated as execution failures.

The report defaults to `meta/reference-scan.json`. It contains each original fixture and its result, including actual structured values on mismatches. A case needs an explicit tolerance only when the defaults are too tight. Semantic differences do not get a tolerance. CI runs ordinary offline Rust tests; it does not need the scanner or a worker process.

## Sources and adaptations

| Source | Snapshot | Files |
|---|---|---|
| [ngn/apl](https://github.com/abrudz/ngn-apl/tree/d156d4e2b33c178e82dac55a19244014700d4cfa) | `d156d4e2b33c178e82dac55a19244014700d4cfa` | `t.apl`, `examples/*.apl` and matching `.out` |
| [April](https://github.com/phantomics/april/tree/0001af6d518d0e8fdf6a7d1688dd92a2fd9b29df) | `0001af6d518d0e8fdf6a7d1688dd92a2fd9b29df` | `spec.lisp`, every `**/demo.lisp` assertion and provision |
| [APLcart](https://github.com/abrudz/aplcart/tree/f01e91e1b08a7ca611c6c93328831425426ef8de) | `f01e91e1b08a7ca611c6c93328831425426ef8de` | `table.tsv` and `tt.tsv`, including duplicate recipes with separate provenance |
| [Dyalog documentation](https://github.com/Dyalog/documentation/tree/6ccc87c6cedb0229f2c9747037ebce8da6387df1) | `6ccc87c6cedb0229f2c9747037ebce8da6387df1` | Selected primitive-function examples in `dyalog.jsonl` |
| [BQN](https://github.com/mlochbaum/BQN/tree/5abbab967fefc68ae9b3c2620d5d38473a90cb66) | `5abbab967fefc68ae9b3c2620d5d38473a90cb66` | Selected examples in `doc/*.md` |
| [BQNcrate](https://github.com/mlochbaum/bqncrate/tree/38b775f7112461e93f546016f186a15da99f3ff9) | `38b775f7112461e93f546016f186a15da99f3ff9` | Selected rows of `table.tsv` |

Dyalog cases use `dyalog:page:example` IDs. Source pages default to `language-reference-guide/docs/primitive-functions/{page}.md`; an explicit `source` is relative to `language-reference-guide/docs/`. Cases cover pervasive maths, complex numbers, search/sets, folds, composition, rank, trains and products. The checkout is v21 documentation; expectations were captured independently in Dyalog 20.0.53963.0 with the settings below, not inferred from rounded display text. Adaptations remove prompts/comments and normalise spacing and `J` case. Per-case adaptations record changed arguments or origin. Explicit relative tolerances allow floating-point library rounding, not semantic differences.

For each new glyph, read its documented valences and select examples that establish distinct semantics. Add structured expectations, run them, and fix failures as part of that glyph's implementation rather than deferring discovered gaps.

Dyalog's two fixed-order float reduction examples are retained as explicit exclusions. BPL permits reassociation of primitive float sums/products. Generic-function reduction and primitive scan order remain tested; do not replace excluded expectations with one compiler's chosen answer.

ngn uses origin 0 and has different prototype/dialect rules. Its original expressions and expectations are retained. BPL also counts from 0, so index and axis operands can keep ngn's origin. Inventory records from BPL's 1-origin period keep their adapted code, with the upstream code in `original_code` and the changed origin in `original_origin`. Closed literal right-hand expectations were evaluated independently in Dyalog, not with BPL.

April's literal Common Lisp expectations were converted to structured values. Ordinary rational expectations represent approximate results under BPL's numeric policy, not opt-in exact `r` literals. The power alias `⋆` is written as standard `*` outside quoted text. Printed-format expectations, host wrappers, and library dependencies remain visible for review.

APLcart's TIO links were decoded offline. All 972 available decoded programs are retained. No TIO service was contacted. Small closed calculator examples were checked in Dyalog 20.0.53963.0 with `⎕IO=1`, `⎕CT=1E¯14`, `⎕DIV=0`, `⎕ML=1`, and `⎕PP=17`. Multi-output examples collect their values in an array literal. The original program remains in `example`. The import does not execute arbitrary catalogue programs.

BQN cases use `bqn:page:line` IDs for documentation examples and `bqncrate/table.tsv:line` IDs for BQNcrate rows. Each expectation is BQN's own result, computed by the JavaScript implementation in the `BQN` checkout through `basedpl.reference.bqn`. BQN prints it with `•Repr`, and the inventory's `expected_code` holds its BPL form. The `.bpl` expectations mark exact numbers and Booleans where BPL's rules give them. BQNcrate recipes leave their arguments open, so each case chooses concrete arguments and records that in `adaptation`. Related examples from one page can form one case, collected in an array literal. A case that needs a feature BPL lacks stays `pending`. A case where BPL gives a different result has status `question`.

`basedpl.reference` contains the import, reference capture, scan, review and activation functions. `basedpl.bpltests.add` appends reviewed cases. Rust tests read the checked-in `.bpl` files. They need neither the JSONL inventory, sibling clones, Dyalog, Common Lisp, Node, Python nor network access. Python converter tests also check serialization against the tracked inventory. Import a new upstream snapshot into a new directory and review it against the inventory rather than replacing reviewed statuses.

For live reference work, use `basedpl.dyalog.Apl`, which runs Dyalog, not `basedpl.Session`, which runs BPL:

```python
from basedpl.dyalog import Apl
from basedpl.reference import REFERENCE_ENCODER, dyalog_expected
with Apl() as apl:
    apl('⎕IO←1 ⋄ ⎕CT←1E¯14 ⋄ ⎕DIV←0 ⋄ ⎕ML←1 ⋄ ⎕PP←17')
    apl(REFERENCE_ENCODER)
    expected = dyalog_expected(apl, '2 3⍴⍳6')
```

Non-language repository material is not an acceptance case: ngn's browser assets, April's bundled Common Lisp parser implementation tests, and APLcart's website/quiz implementation and publication bibliography. The bibliography is `pub/pub.tsv`, not another recipe table. No algorithm or mathematical recipe was discarded for being difficult, unimplemented, or beyond the current lesson set.

Licences and attribution for the adapted sources are in `tests/reference/README.md`.
