# `⊃` — Mix / Pick

`⊃Y` assembles items into rectangular cells, padding to a common shape with fill.

```apl
⊃1 2 3             ⍝ 1 2 3
⊃(1 2⋄ 3 4 5)      ⍝ 2 3⍴1 2 0 3 4 5
```

`P⊃Y` selects leading-axis coordinates. Remaining axes form a cell; full coordinates return the stored value. Successive Picks traverse nesting. Empty coordinates return `Y`.

```apl
2⊃2 3⍴⍳6           ⍝ 4 5 6
2 1⊃2 3⍴⍳6         ⍝ 4
1⊃2⊃(1 2⋄ 3 4)     ⍝ 3
⍬⊃1 2              ⍝ 1 2
```

Array-valued coordinate fields pair using leading-axis agreement:

```apl
(1 3)˘(2 4)⊃3 4⍴⍳12 ⍝ 2 12
```

On a [keyed array](../keyed.md), a string picks the value stored under that key. An array of strings returns an ordinary array of values with the same shape.

```apl
T←('price':1 2 3 ⋄ 'qty':4 5 6)
'qty'⊃T            ⍝ 4 5 6
('qty' 'price')⊃T  ⍝ (4 5 6 ⋄ 1 2 3)
```

On Mix, axes place cell dimensions among frame dimensions; fractional positions insert before an axis. INDEX: position outside the array. Index 1 on an empty axis gives fill, as does [First](take.md).
