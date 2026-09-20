# bAsedPL

bAsedPL (aka "Based-array APL") is an APL-derived language that uses *based arrays*. It emphasizes simple, consistent notation, borrowing ideas from J and BQN.

Based arrays, named in a [1981 paper](https://dl.acm.org/doi/abs/10.1145/586656.586663) and popularized by [BQN](https://mlochbaum.github.io/BQN/doc/based.html), treats numbers, characters and functions as *atoms*, which are collected and shaped in *arrays*. The atom `3` is distinct from the scalar (rank-0 array) `⊂3`.

Work with whole arrays, define functions, and combine them with operators, through an interactive terminal or Python API. Numbers include approximate reals, complex numbers and exact rationals.

## Install

```bash
pip install basedpl
```

## Command line

```bash
basedpl -e '+/⍳10'       # 55
```

Run `basedpl` to open the [REPL](docs/repl.md).

## Python

```python
from basedpl import Session

with Session() as apl:
    mean = apl.fn('{(+/⍵)÷≢⍵}')
    print(mean([1, 2, 3]).py)    # 2
```

## Jupyter

The Python package installs the **bAsedPL** kernel. Select it in Jupyter to run APL cells with persistent names, glyph/name completion and interruption. Explicit `⎕←` output streams during execution; ordinary results follow the REPL's display rules.

The same native executable runs the kernel with `basedpl --kernel -f CONNECTION_FILE`, using kernmini for the Jupyter protocol. No Python interpreter is needed when launching the native binary directly.

## Documentation

- [Docs home](docs/index.md)
- [Getting started](docs/getting-started.md)
- [REPL: glyph entry and display](docs/repl.md)
- [Python: arrays and functions](docs/python.md)
- [Glyph reference](docs/index.md#glyph-reference)
- [APL libraries: algorithms, codecs, interpreters and puzzles](lib/README.md)

For source installation and contributing, see [DEV.md](DEV.md).
