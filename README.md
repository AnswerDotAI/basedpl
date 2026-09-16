# miniapl

PyO3/maturin package scaffolded by fastship.

## Development

```bash
pip install -e .[dev]
maturin develop && pytest -q
```

## Build

```bash
ship-rs-build
```

## Release

```bash
maturin develop && pytest -q
ship-release
```

`ship-release` tags the Cargo version, leaves wheel publication to GitHub Actions, then bumps the project.
