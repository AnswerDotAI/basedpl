# `⍉` — Transpose

`⍉Y` reverses axis order.

```apl
⍉2 3⍴⍳6            ⍝ 3 2⍴1 4 2 5 3 6
```

In `A⍉Y`, each item of `A` names the result axis for the corresponding source axis. Labels start at 1 with no gaps.

```apl
2 1⍉2 3⍴⍳6         ⍝ 3 2⍴1 4 2 5 3 6
```

Repeated labels select diagonals, taking the smallest participating dimension.

```apl
1 1⍉3 3⍴⍳9         ⍝ 1 5 9
```
