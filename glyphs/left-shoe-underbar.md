

# [`⊆`](https://answerdotai.github.io/basedpl/glyphs/left-shoe-underbar.html) — Nest / Partition

Function. Keys: `Alt-Minus z`. Hanzi: `巢/段`. Ranks:
[`∞`](https://answerdotai.github.io/basedpl/glyphs/infinity.html)
monadic, `1 ∞` dyadic

`⊆Y` encloses an atom or a [simple
array](../terms.qmd#term-simple-array). Any other array stays unchanged.

``` bpl
⊆1 2               ⍝ ⊂1 2
⊆1                 ⍝ ⊂1
```

`N⊆Y` partitions along the leading axis. Each partition is a block of
major cells. `N` has one item for each major cell. A cell whose item is
0 is omitted. A new partition starts wherever `N` rises.

``` bpl
1 1 0 1 1⊆"abcde" ⍝ "ab" "de"
1 1 0 2⊆4 2ρι8ₓ    ⍝ [[0 1◇2 3] [6 7◇]]ₓ
```

Items of `N` are nonnegative whole numbers. A unit or one-item `N`
applies to every major cell. `N⊆⍠K Y` partitions along axis `K` instead.
`N⊆¹Y` partitions each row. Unlike [`⊂`](left-shoe.qmd), a positive item
need not start a partition.

``` bpl
1 1 0 1 1⊆¹["abcde"◇"fghij"] ⍝ ["ab" "de"◇"fg" "ij"]
```

## Inverse

`⊆⁻Y` discloses a unit and returns any other array unchanged. `X⊆⁻Y`
joins the partitions `Y`. Partitioning the result by `X` must give `Y`
again, so `X` can’t hold a zero.

``` bpl
⊆⁻⊂1 2             ⍝ 1 2
1 1 2⊆⁻(1 2◇3)   ⍝ 1 2 3
```
