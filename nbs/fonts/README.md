# SAX2B

SAX2B adds BPL's missing superscripts and subscripts to SAX2. Existing glyphs are unchanged. The subscript `ₓ` has a full character-cell advance for integer box borders.

Rebuild from the repository root in a Python kernel with `fonttools` and `brotli` installed:

```python
from basedpl.font import build
build()
```

This writes `SAX2B.ttf`, `SAX2B.woff2`, and `LICENSE-SAX2` here. The BPL symbols `⇄`, `∂`, and `⨸` are still missing.
