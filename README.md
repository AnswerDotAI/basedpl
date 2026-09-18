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

## Documentation

- [Getting started](docs/getting-started.md)
- [REPL: glyph entry and display](docs/repl.md)
- [Python: arrays and functions](docs/python.md)
- [Glyph reference](docs/index.md#glyph-reference)

For source installation and contributing, see [DEV.md](DEV.md).
