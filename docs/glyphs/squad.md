# `⌷` — Index

`I⌷Y` selects axes using the same indices as [brackets](brackets.md). Omitted trailing axes remain whole. `⌷Y` is identity.

```apl
2⌷3 4⍴⍳12          ⍝ 5 6 7 8
2⌷[2]3 4⍴⍳12       ⍝ 2 6 10
(⊂3 1)⌷10 20 30    ⍝ 30 10
```

Enclose a vector to use it as one axis's index list. `[K]` selects which axes the indices address.
