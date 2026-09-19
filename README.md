# miniapl

A modern APL calculator for teaching mathematics, written in Rust, with an interactive terminal and Python API.

## Install

```bash
pip install miniapl
```

## Command line

```bash
miniapl -e '+/⍳10'       # 55
```

Run `miniapl` to open the [REPL](docs/repl.md).

## Python

```python
from miniapl import Session

with Session() as apl:
    mean = apl.fn('{(+/⍵)÷≢⍵}')
    print(mean([1, 2, 3]).py)    # 2
```

## Jupyter

The Python package installs the **APL (miniapl)** kernel. Select it in Jupyter to run APL cells with persistent names, glyph/name completion and interruption. Explicit `⎕←` output streams during execution; ordinary results follow the REPL's display rules.

The same native executable runs the kernel with `miniapl --kernel -f CONNECTION_FILE`, using kernmini for the Jupyter protocol. No Python interpreter is needed when launching the native binary directly.

## Documentation

- [Getting started](docs/getting-started.md)
- [REPL: glyph entry and display](docs/repl.md)
- [Python: arrays and functions](docs/python.md)
- [Glyph reference](docs/index.md#glyph-reference)
- [APL libraries: numeric, array, graph, string, power and tree dfns](lib/README.md)

For source installation and contributing, see [DEV.md](DEV.md).
