

# `<` — Sort up / Less

Ranks: `∞` monadic, `0 0` dyadic

`<Y` sorts the major cells of `Y` ascending, as `[⍋Y]⌷Y` does. Ties keep
their order.

``` bpl
<30 10 20           ⍝ 10 20 30
<"banana"           ⍝ "aaabnn"
<[3 1⋄1 2⋄2 0]  ⍝ [1 2⋄2 0⋄3 1]
```

`X<Y` tests X \< Y for reals. Pervasive; tolerant.

``` bpl
1 2 3<2            ⍝ $t $f $f
```

See [`>`](greater.qmd) to sort down, and [`⍋`](grade-up.qmd) for the
indices that sort.
