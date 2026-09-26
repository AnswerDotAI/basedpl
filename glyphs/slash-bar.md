

# `⌿` — Replicate / Reduce first

`N⌿Y` replicates first-axis cells.

``` apl
1 0⌿2 3⍴⍳6         ⍝ 1 3⍴0 1 2
```

`f⌿Y` reduces the first axis.

``` apl
+⌿2 3⍴⍳6           ⍝ 3 5 7
```

`S f⌿ Y` starts each lane from the whole seed `S` on the right.

``` apl
10+⌿3 2⍴⍳6         ⍝ 16 19
```

Counts, identities and `⍠K` follow [`/`](slash.qmd).
