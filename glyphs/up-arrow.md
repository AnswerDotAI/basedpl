

# [`↑`](https://answerdotai.github.io/basedpl/glyphs/up-arrow.html) — First / Take

Function. Keys: `Alt-k`. Hanzi: `取`. Ranks:
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html)
monadic, `1 ∞` dyadic

`↑Y` is `0⊃Y`: the first item of a vector, else the first major cell. On
a scalar it returns the value held; on an atom, the atom. Empty leading
axis: a cell of fill.

``` bpl
↑[[1 2] [3 4 5]]  ⍝ 1 2
↑2 3ρι6ₓ           ⍝ [0 1 2]ₓ
↑⊂7               ⍝ 7
↑0 3ρ0            ⍝ 0 0 0
```

`N↑Y` takes leading items; negative `N` takes from the end. Overtake
inserts fill. [`⍠`](quad-colon.qmd#fill) sets the fill, for Take and
First.

``` bpl
4↑1 2              ⍝ 1 2 0 0
¯4↑1 2             ⍝ 0 0 1 2
```

Vector counts apply to successive leading axes. `N↑⍠K Y` applies the
counts to axes `K`.

``` bpl
1 2↑2 3ρι6ₓ         ⍝ 1 2ρ[0 1]ₓ
```

## Inverse

`↑⁻Y` encloses `Y`.

``` bpl
↑⁻5   ⍝ ⊂5
```
