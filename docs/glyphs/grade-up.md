# `⍋` — Grade up

`⍋Y` gives indices that sort major cells ascending. Stable; uses [structural ordering](../rules.md#equality-and-ordering), without tolerance.

```apl
⍋30 10 20          ⍝ 2x 3x 1x
v←30 10 20 ⋄ v[⍋v] ⍝ 10 20 30
⍋2 1 2 1           ⍝ 2x 4x 1x 3x
```

`C⍋Y` uses a character collation array `C`.

```apl
'cba'⍋'abc'        ⍝ 3x 2x 1x
```

See [`⍒`](grade-down.md).
