# Development

## Commands

```bash
maturin develop && pytest -q
ship-rs-build
```

## Versioning

The canonical version lives in `Cargo.toml`. `pyproject.toml` gets the Python package version from Cargo via `dynamic = ["version"]`.

## Build profiles

uv builds and `maturin develop --release` use the incremental `release` profile for fast local iteration. CI builds distributed wheels with `dist`, which enables full LTO and one codegen unit, disables incremental compilation, and strips the result.

## Release

1. Run `maturin develop && pytest -q`.
2. Confirm the release version in `Cargo.toml` (`[package].version`).
3. Run `ship-release`.

Fastship commits the changelog, pushes the version tag for GitHub Actions, then bumps and pushes `Cargo.toml`.
