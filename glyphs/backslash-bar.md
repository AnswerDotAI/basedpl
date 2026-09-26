

# `⍀` — Expand / Scan first

`N⍀Y` expands first-axis cells.

``` apl
1 0 1⍀[1 2 ⋄ 3 4]  ⍝ [1 2 ⋄ 0 0 ⋄ 3 4]
```

`f⍀Y` scans down the first axis.

``` apl
+⍀[1 2 3 ⋄ 4 5 6]  ⍝ [1 2 3 ⋄ 5 7 9]
```

`S f⍀ Y` supplies a seed.

``` apl
10+⍀[1 2 ⋄ 3 4]    ⍝ [11 12 ⋄ 14 16]
```

Counts, seeds and `⍠K` follow [`\`](backslash.qmd).
