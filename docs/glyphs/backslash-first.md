# `⍀` — Expand / Scan first

`N⍀Y` expands first-axis cells.

```apl
1 0 1⍀2 2⍴⍳4       ⍝ 3 2⍴1 2 0 0 3 4
```

`f⍀Y` scans down the first axis.

```apl
+⍀2 3⍴⍳6           ⍝ 2 3⍴1 2 3 5 7 9
```

`S f⍀Y` supplies a seed.

```apl
10 20+⍀2 2⍴⍳4      ⍝ 2 2⍴11 22 14 26
```

Counts, seeds and `[K]` follow [`\`](backslash.md).
