# `⍥` — Over

`f⍥g Y` is `f(g Y)`.

```apl
(+/⍥,)2 2⍴⍳4       ⍝ 10
```

`X f⍥g Y` is `(g X)f(g Y)`.

```apl
1 2 +⍥≢ 3 4 5      ⍝ 5x
```

Compare [Compose](compose.md), which transforms only the right argument.
