# `≢` — Tally / Not match

`≢Y` counts major cells: first-axis length, or 1 for a scalar.

```apl
≢2 3⍴0             ⍝ 2ₓ
≢42                ⍝ 1ₓ
≢⍬                 ⍝ 0ₓ
```

`X≢Y` negates [match](match.md).

```apl
1≢,1               ⍝ 1ₓ
```
