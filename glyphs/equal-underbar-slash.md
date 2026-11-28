

# [`≢`](https://answerdotai.github.io/basedpl/glyphs/equal-underbar-slash.html) — Tally / Not match

Function. Keys: `Alt-?`. Hanzi: `数/异` (`數/異`). Ranks:
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html)
monadic, `∞ ∞` dyadic

`≢Y` counts major cells: first-axis length, or 1 for a unit.

``` bpl
≢2 3ρ0             ⍝ 2ₓ
≢42                ⍝ 1ₓ
≢⍬                 ⍝ 0ₓ
```

`X≢Y` negates [Match](equal-underbar.qmd).

``` bpl
1≢,1               ⍝ $t
```
