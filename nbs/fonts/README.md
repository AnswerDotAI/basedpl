# SAX2B

SAX2B adds BPL's missing superscripts and subscripts to SAX2. Existing glyphs are unchanged. The subscript letters `ₓ ᵣ ₑ ⱼ` have full character-cell advances for box borders.

The tailed arrows `↣ ↢` reuse SAX2's right-arrow outline and arrowhead. They retain its character-cell width and vertical alignment.

Rebuild from the repository root in a Python kernel with `fonttools` and `brotli` installed:

```python
from basedpl.font import build
build()
```

This writes `SAX2B.ttf`, `SAX2B.woff2`, and `LICENSE-SAX2` here. The BPL symbols `⇄`, `∂`, and `⨸` are still missing.
