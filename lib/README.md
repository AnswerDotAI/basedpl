# APL libraries

Dyalog dfns adapted for bAsedPL from [April's ports](https://github.com/phantomics/april/tree/master/libraries/dfns) and the Dyalog `dfns` workspace. Load a file from the repository root:

```apl
•LOAD 'lib/numeric.apl'
phinary 42                     ⍝ '10100010.00100001'
84 gcd 30                      ⍝ 6
```

The same call works from Python: `apl("•LOAD 'lib/numeric.apl'")`. Definitions enter the current scope. Paths are relative to the working directory.

| File | Contents | Loads |
|---|---|---|
| `array.apl` | Array algorithms, lists, displays, selection, scans | |
| `numeric.apl` | Number theory, fractions, linear algebra, Phinary, FFT | `graph.apl` |
| `graph.apl` | Traversals, paths, components, assignment, exact cover | `array.apl` |
| `string.apl` | Search/replace, wrapping, justification, whitespace | `array.apl` |
| `power.apl` | Iteration, trajectories, numerical inversion | `array.apl` |
| `tree.apl` | AVL, red-black, splay and binary search trees | `power.apl` |
| `dyalog.apl` | Compression, dates, puzzles, Lisp, parsing, text and macro expansion | `array.apl` |

The ports use origin one, based arrays, `↑` for First, `⊃` for Mix, `⍶`/`⍹` operands, and seeded reductions.

Examples in `tests/reference/{april,aplcart,dyalog}.apl` load these files and retain independent upstream or Dyalog expectations. Case-specific setup stays in the tests. Unported Dyalog definitions remain in the reference inventory with their outstanding dependencies.
