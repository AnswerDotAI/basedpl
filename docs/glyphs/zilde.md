# `⍬` — Empty vector

`⍬` is an empty approximate numeric vector: `0⍴0`.

```apl
⍴⍬                 ⍝ ,0x
⍬,1 2              ⍝ 1 2
```

Other empty types retain their prototypes.

```apl
3↑0⍴0x             ⍝ 0x 0x 0x
3↑''               ⍝ '   '
```
