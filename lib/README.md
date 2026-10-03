# BPL libraries

BPL utilities and Dyalog dfns adapted from [April's ports](https://github.com/phantomics/april/tree/master/libraries/dfns) and the Dyalog `dfns` workspace. John Scholes, who created dfns, collected that workspace as examples of the style. These ports keep that purpose. Each function is an example of a dfn, including those that compute what a BPL primitive already gives.

Each file is a module. `•load` runs it and returns a record of its names. Take the names you use, with paths relative to the repository root:

```bpl
[phinary]←•load "lib/numeric.bpl"
phinary 42                     ⍝ "10100010.00100001"
```

The same call works from Python: `bpl('•load "lib/numeric.bpl"')` returns the record as a dict. A file that uses another loads it by a `./` path, relative to its own location, and keeps it under a private name such as `_array`. [Namespaces](../nbs/namespaces.qmd) covers modules.

| File | Contents | Loads |
|---|---|---|
| `array.bpl` | Array algorithms, lists, displays, selection, scans | |
| `numeric.bpl` | Number theory, fractions, linear algebra, Phinary, FFT | `graph.bpl` |
| `graph.bpl` | Traversals, paths, components, assignment, exact cover | `array.bpl` |
| `string.bpl` | Search/replace, wrapping, justification, whitespace | |
| `power.bpl` | Iteration, trajectories, numerical inversion | |
| `tree.bpl` | AVL, red-black, splay and binary search trees | |
| `dyalog.bpl` | Compression, dates, puzzles, Lisp, parsing, text and macro expansion | `array.bpl` |
| `regex.bpl` | `pattern (f regex_replace) text`: transform matched strings | |

The ports count positions from 0, as the rest of BPL does. They use based arrays, `↑` for First, `⊃` for Mix, `⍶`/`⍹` operands, and seeded reductions.

BPL's `≡` returns an unsigned depth, as Dyalog's does when `⎕ML` is 2 or more. Where an original under the default `⎕ML` writes `|≡`, its port writes `≡`. Dyalog's First takes the first element in ravel order, while `↑` takes the first major cell, so a port takes the first element of a matrix with `↑,⍵`.

`dyalog.bpl` includes `cal [year month]` and `cal year` for calendars, `packZ` for LZW compression (`0 packZ` expands; a negative bit limit returns the dictionary), `variables unify expressions` for structural unification with an occurs check, and `digits ratsum` for repeating-unit rational addition/negation.

Examples in `tests/reference/{april,aplcart,dyalog}.bpl` load these files and retain independent upstream or Dyalog expectations. Case-specific setup stays in the tests. `tests/reference/lib.bpl` covers the branches those examples miss, with expectations worked out from each function's definition or from published results. Unported Dyalog definitions remain in the reference inventory with their outstanding dependencies.
