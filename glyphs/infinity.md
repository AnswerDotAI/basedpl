

# [`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html) — Infinity

Literal. Keys: `Alt-8`. Hanzi: `永`

[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html) and
`¯∞` are approximate real infinities.

``` bpl
∞+3                ⍝ ∞
3÷∞                ⍝ 0
⌊/⍬                ⍝ ∞
⌈/⍬                ⍝ ¯∞
```

`¯∞` \< every finite real \<
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html). Each
infinity equals itself. `⌊/` and `⌈/` of an empty array are infinite,
even for exact arrays. Under dynamic kinds, the functions that
[Infinities and NaN](../numbers.qmd#infinities-and-nan) lists raise
`DOMAIN` where they have no result, as `∞∨2` does. The following
undefined results give NaN. Results at infinities and NaNs can differ by
execution route, as [Accuracy](../numbers.qmd#accuracy) describes:

``` bpl
∞-∞                ⍝ $n
0×∞                ⍝ $n
```

[Infinities and NaN](../numbers.qmd#infinities-and-nan) covers how
infinities combine with exact numbers.

## Errors

- `DOMAIN`: an infinite index or count
