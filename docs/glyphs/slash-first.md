# `⌿` — Replicate / Reduce first

`N⌿Y` replicates first-axis cells.

```apl
1 0⌿2 3⍴⍳6         ⍝ 1 3⍴1 2 3
```

`f⌿Y` reduces the first axis.

```apl
+⌿2 3⍴⍳6           ⍝ 5 7 9
```

`N f⌿Y` performs n-wise reduction there.

```apl
2+⌿3 2⍴⍳6          ⍝ 2 2⍴4 6 8 10
```

Counts, identities and `[K]` follow [`/`](slash.md).
