# `.` — Inner product

`X f.g Y` pairs items with `g`, then reduces with `f`. It contracts the last axis of `X` with the first of `Y`.

```apl
1 2 3+.×4 5 6      ⍝ 32
(2 2⍴1 0 0 1)+.×2 2⍴⍳4 ⍝ 2 2⍴⍳4
```

Singleton contraction axes extend. Empty contractions use the reduction identity. `∘.g` is [outer product](outer.md).
