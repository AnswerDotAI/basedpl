# How to contribute

## Install from source

Requires Python 3.10 or later and Rust 1.98 or later. On x86-64, it also requires a CPU with AVX2 (x86-64-v3). In your Python environment:

```bash
git clone https://github.com/AnswerDotAI/basedpl.git
cd basedpl
pip install .
```

For a standalone executable without Python, run `cargo install --path cli`. Cargo installs it in its `bin` directory, normally `~/.cargo/bin`. Put that directory on your PATH.

## Contributing

Install the development and documentation tools with `pip install -e '.[dev]'`. That also builds the extension and installs it in the editable package. The main commands are:

```bash
cargo test
cargo develop
bpl -e '2×3+4'
cargo fastfmt
pytest
ship-rs-build
```

After Rust changes, run `cargo test` for the Rust tests, then `cargo develop` to install what `cargo test` built: the extension, and the native `bpl` command in your environment. Both use one compilation of each crate. `cargo develop` comes from `fastws-cli`, one of the development tools. Run `pip install -e '.[dev]'` again only when the package metadata changes. Use `cargo fastfmt`, not `cargo fmt`.

Before a release, run `python scripts/prep.py` from the repository root. It writes the syntax highlighters' glyph lists and the macOS keyboard layout bundle. It then runs nbdev's `prepare` to export, test and clean the notebooks and render this README.

## Did you find a bug?

* Ensure the bug was not already reported by searching on GitHub under Issues.
* If you're unable to find an open issue addressing the problem, open a new one. Be sure to include a title and clear description, as much relevant information as possible, and a code sample or an executable test case demonstrating the expected behavior that is not occurring.
* Be sure to add the complete error messages.

#### Did you write a patch that fixes a bug?

* Open a new GitHub pull request with the patch.
* Ensure that your PR includes a test that fails without your patch, and pass with it.
* Ensure the PR description clearly describes the problem and solution. Include the relevant issue number if applicable.

## PR submission guidelines

* Keep each PR focused. While it's more convenient, do not combine several unrelated fixes together. Create as many branches as needing to keep each PR focused.
* Do not mix style changes/fixes with "functional" changes. It's very difficult to review such PRs and it most likely get rejected.
* Do not turn an already submitted PR into your development playground. If after you submitted PR, you discovered that more work is needed - close the PR, do the required work and then submit a new PR. Otherwise each of your commits requires attention from maintainers of the project.
* If, however, you submitted a PR and received a request for changes, you should proceed with commits inside that PR, so that the maintainer can see the incremental fixes and won't need to review the whole PR again. In the exception case where you realize it'll take many many commits to complete the requests, then it's probably best to close the PR, do the work and then submit it again. Use common sense where you'd choose one way over another.

## Do you want to contribute to the documentation?

* Docs are automatically created from the notebooks and qmd files in the nbs folder.

