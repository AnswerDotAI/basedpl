# `[ ]` — Literal / Index / Axis

`[A ⋄ B]` assembles cells with fill. Newlines also separate cells.

```apl
[1 2 ⋄ 3 4]        ⍝ 2 2⍴1 2 3 4
[1 2 ⋄ 3]          ⍝ 2 2⍴1 2 3 0
```

After an array, brackets index from 1. Semicolons separate axes; an empty field selects the whole axis.

```apl
v←10 20 30 ⋄ v[3 1] ⍝ 30 10
m←2 3⍴⍳6 ⋄ m[;2]  ⍝ 2 5
```

After a function, brackets qualify axes.

```apl
+/[1]2 3⍴⍳6        ⍝ 5 7 9
```

See [axis rules](../rules.md#axes-and-indices), [assignment](assign.md) and [Squad](squad.md).
