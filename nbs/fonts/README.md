# BPL fonts

SAX2B extends SAX2 with BPL's missing superscripts, subscripts and arrows. BQN386B extends BQN386 for array output. Both include `⁝` as three half-size period dots spaced 1/8 em apart, within the normal character-cell width. Their colons are unchanged.

SAX2B's subscript letters `ₓ ᵣ ₑ ⱼ` have full character-cell advances for box borders. Its arrows `↣ ↢ ↦` reuse SAX2's right-arrow outline. The diagonals of `/ ⌿` are 5% thicker.

Rebuild from the repository root in a Python kernel with `fonttools` and `brotli` installed:

```python
from basedpl.font import build
build()
```

This writes TTF and WOFF2 files for both fonts, with `LICENSE-SAX2` and `LICENSE-BQN386`. The source versions are pinned in `basedpl.font`. BQN386B has its own family name to distinguish it from an installed upstream BQN386.

The BPL symbols `⇄`, `∂`, and `⨸` are still missing from SAX2B.
