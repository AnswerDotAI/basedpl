

# `#` — Runs / Replicate

Ranks: `∞` monadic, `1 ∞` dyadic

`#Y` gives the runs of adjacent matching major cells of `Y`: the length
of each run, as exact integers, and the cell that repeats. It matches
cells as [`∪`](union.qmd) does. `#/` rebuilds `Y`, because `N#Y` repeats
each cell by its count. `#⁻¹` decodes too, so [Under](under.qmd) can
change the runs. Here it doubles each length:

``` bpl
#"aaabcc"               ⍝ [[3 1 2]ₓ "abc"]
#/#"aaabcc"             ⍝ "aaabcc"
⊽@(0⊃)@# "aab"          ⍝ "aaaabb"
```

A unit gives the rank-0 length 1 and the unit itself. When `Y`’s leading
axis has keys, a third item holds the key of every position, and `#⁻¹`
attaches them again. `#/` doesn’t use that item.

``` bpl
#["x":1 "y":1 "z":2]    ⍝ (2ₓ 1ₓ⋄1 2⋄"x" "y" "z")
```

`N#Y` repeats each major cell of `Y` by its count in `N`. Boolean counts
filter.

``` bpl
1 0 1#10 20 30          ⍝ 10 30
2 1#10 20               ⍝ 10 10 20
0 2#[1 2⋄3 4]         ⍝ [3 4⋄3 4]
```

A negative count replaces its cell with that many fills. A single count
applies to every cell. A single cell serves every count.

``` bpl
1 ¯2 1#10 20 30         ⍝ 10 0 0 30
2 ¯1 1#"abc"            ⍝ "aa c"
2#3 4                   ⍝ 3 3 4 4
1 0 2#5                 ⍝ 5 5 5
```

With [Axis](axis.qmd), `#⍠¯1` replicates along the last axis.

``` bpl
1 0 1#⍠¯1 [1 2 3⋄4 5 6]   ⍝ [1 3⋄4 6]
```

## Expand

With `N` fixed, [`⁻¹`](../scripts.ipynb#superscripts) inverts `N#`. The
inverse gives one cell for each count. A positive count gives the first
of its copies, and a zero or negative count gives a fill. With a Boolean
mask, the inverse expands `Y`. The result has the next cell of `Y`
wherever the mask has 1, and a fill wherever it has 0.

``` bpl
1 0 1#⁻¹10 20           ⍝ 10 0 20
1 0 1 #⁻¹⍠¯1 [1 2⋄3 4]      ⍝ [1 0 2⋄3 0 4]
2 1#⁻¹10 10 20          ⍝ 10 20
1 ¯2 1#⁻¹10 0 0 30      ⍝ 10 0 30
```

The counts must use every cell once, unless `Y` has one cell. A single
count undoes a replicate by that count.

``` bpl
1 0 1#⁻¹5               ⍝ 5 0 5
2#⁻¹1 1 2 2             ⍝ 1 2
```

[Under](under.qmd) and [selective assignment](assign.qmd) both keep the
cells that `N` drops.

``` bpl
-@ 1 0 1# 1 2 3         ⍝ ¯1 2 ¯3
x←1 2 3
(1 0 1#x)←0
x                       ⍝ 0 2 0
```

APL difference: `#` replaces `/`, `⌿`, `\` and `⍀`. Dyalog’s expand with
counts other than 0 and 1, `N\Y`, is `1⌈|N # N>0 #⁻¹ Y`.

## Errors

- `LENGTH`: counts that don’t match the cells, or an inverse that
  doesn’t use every cell
- `DOMAIN`: counts that aren’t integers, or one inverse count that
  doesn’t divide the number of cells
