# `⌝` — Outer product

`X g⌝Y` applies `g` to every pair. Result shape: `(⍴X),⍴Y`.

```apl
1 2+⌝10 20         ⍝ 2 2⍴11 21 12 22
(⍳3)=⌝⍳3           ⍝ 3 3⍴1x 0x 0x 0x 1x 0x 0x 0x 1x
```

Array-valued pair results remain nested. See [inner product](dot.md).
