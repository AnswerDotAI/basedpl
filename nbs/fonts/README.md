# BPL fonts

SAX2B extends SAX2 with BPL's missing superscripts, subscripts and arrows. BQN386B extends BQN386 for array output. Both include `⁝` as three half-size period dots spaced 1/8 em apart, within the normal character-cell width. Their colons are unchanged.

SAX2B's subscript letters `ₓ ᵣ ₑ ⱼ` have full character-cell advances for box borders. Its arrows `↣ ↢ ↦` reuse SAX2's right-arrow outline. The diagonals of `/ ⌿` are 5% thicker.

Both fonts offer compact numeric subscripts through the OpenType feature `ss01`. `SUBSCRIPT_SPACING` in `basedpl.font` sets the fraction of each inter-subscript gap and intervening space retained, currently `0.5`. Rebuild after changing it. The shared CSS enables the feature in rendered source code. Character-cell metrics and array-output spacing stay unchanged. Fixed-cell terminals do not use the compact spacing.

Rebuild from the repository root in a Python kernel with `fonttools` and `brotli` installed:

```python
from basedpl.font import build
build()
```

This writes installable TTF files and licences into `python/basedpl/assets/fonts`, SAX2B's WOFF2 into `python/basedpl/browser`, and BQN386B's WOFF2 into `nbs/fonts`. Run `basedpl-install` to install both fonts for your user. The source versions are pinned in `basedpl.font`. BQN386B has its own family name to distinguish it from an installed upstream BQN386.

The BPL symbols `⇄`, `∂`, and `⨸` are still missing from SAX2B.
