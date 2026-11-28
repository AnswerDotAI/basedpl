

# [`≠`](https://answerdotai.github.io/basedpl/glyphs/not-equals.html) — Unique mask / Not equal

Function. Keys: `Alt-Shift-=`. Hanzi: `别/初` (`別/初`). Ranks:
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html)
monadic, `0 0` dyadic

`≠Y` marks the first occurrence of each distinct major cell. Cells are
compared with [tolerance](../evaluation.qmd#equality-and-ordering).

``` bpl
≠3 1 3 2 1         ⍝ $t $t $f $t $f
```

`X≠Y` tests inequality, pervasively.

``` bpl
1 2 3≠2            ⍝ $t $f $t
```

## Inverse

On Booleans,
[`≠`](https://answerdotai.github.io/basedpl/glyphs/not-equals.html) is
its own inverse with either argument fixed.

``` bpl
1 0≠⁻1 1   ⍝ $f $t
```
