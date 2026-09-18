# `⊃` — First / Pick

`⊃Y` returns the first item, disclosing one enclosure. Empty arrays give their prototype.

```apl
⊃1 2 3             ⍝ 1
⊃(1 2)(3 4)        ⍝ 1 2
```

`P⊃Y` follows an index path through nesting. Path items can be coordinate vectors. An empty path returns `Y`.

```apl
2 1⊃(1 2)(3 4)     ⍝ 3
⍬⊃1 2              ⍝ 1 2
```

INDEX: position outside the array. Empty [Each](each.md) uses prototype selection instead.
