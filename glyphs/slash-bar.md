

# `⌿` — Reduce first

Keys: `Alt-Minus /`

`f⌿Y` reduces the first axis.

``` bpl
+⌿2 3⍴⍳6           ⍝ 3 5 7
```

`S f⌿ Y` starts each lane from the whole seed `S` on the right.

``` bpl
10+⌿3 2⍴⍳6         ⍝ 16 19
```

Identities, `⍠K` and errors follow [`/`](slash.qmd).
