# `≡` — Depth / Match

`≡Y` gives nesting depth. Uneven nesting gives negative depth.

```apl
≡1 2 3             ⍝ 1x
```

`X≡Y` tests equal shape and corresponding contents, using tolerant numeric equality. Returns one Boolean; [`=`](equal.md) pervades.

```apl
1 2≡1 2            ⍝ 1x
1≡,1               ⍝ 0x
```
