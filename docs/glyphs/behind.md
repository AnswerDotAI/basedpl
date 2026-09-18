# `⍛` — Behind

`f⍛g Y` is `(f Y)g Y`. Useful for computing a mask then filtering.

```apl
3∘<⍛/2 7 1 8       ⍝ 7 8
```

`X f⍛g Y` is `(f X)g Y`.

```apl
3 -⍛+ 10           ⍝ 7
```
