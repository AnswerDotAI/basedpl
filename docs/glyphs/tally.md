# `≢` — Tally / Not match

`≢Y` counts major cells: first-axis length, or 1 for a scalar.

```apl
≢2 3⍴0             ⍝ 2x
≢42                ⍝ 1x
≢⍬                 ⍝ 0x
```

`X≢Y` negates [match](match.md).

```apl
1≢,1               ⍝ 1x
```
