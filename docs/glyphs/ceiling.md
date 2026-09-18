# `⌈` — Ceiling / Maximum

`⌈Y` rounds up. Pervasive; tolerant near integers. Complex ceiling is `-⌊-Y`.

```apl
⌈1.2 ¯1.2         ⍝ 2 ¯1
⌈3r2              ⍝ 2x
```

`X⌈Y` takes the real maximum. Pervasive. Empty reduction identity: `¯∞`.

```apl
3⌈1 5             ⍝ 3 5
```
