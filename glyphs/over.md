

# `⍥` — Over

`f⍥g Y` is `f(g Y)`.

``` apl
+/⍥, [1 2 ⋄ 3 4]   ⍝ 10
```

`X f⍥g Y` is `(g X)f(g Y)`.

``` apl
1 2 +⍥≢ 3 4 5      ⍝ 5ₓ
```

Compare [After](after.qmd), which transforms only the right argument.
