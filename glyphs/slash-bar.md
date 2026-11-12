

# `⌿` — Reduce first

Monadic operator. Keys: `Alt-Minus /`. Hanzi: `汇` (`匯`)

`f⌿Y` reduces the first axis.

``` bpl
+⌿2 3ρι6ₓ           ⍝ [3 5 7]ₓ
```

`S f⌿ Y` starts each lane from the whole seed `S` on the right.

``` bpl
10+⌿3 2ρι6ₓ         ⍝ [16 19]
```

Identities, `⍠K` and errors follow [`/`](slash.qmd).
