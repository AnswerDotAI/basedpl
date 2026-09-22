# `⍴` — Shape / Reshape

`⍴Y` gives axis lengths.

```apl
⍴2 3⍴0             ⍝ 2ₓ 3ₓ
```

`S⍴Y` reshapes, cycling or truncating the ravel of `Y`. Dimensions are nonnegative integers. Empty `Y` supplies fill.

```apl
2 3⍴1 2            ⍝ 2 3⍴1 2 1 2 1 2
3⍴''               ⍝ '   '
```

An empty shape makes a rank-zero array.

```apl
⍬⍴1 2              ⍝ ⊂1
```
