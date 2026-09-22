# `⍋` — Grade up

`⍋Y` gives indices that sort major cells ascending. Stable; uses [structural ordering](../rules.md#equality-and-ordering), without tolerance.

```apl
⍋30 10 20          ⍝ 2ₓ 3ₓ 1ₓ
v←30 10 20 ⋄ v[⍋v] ⍝ 10 20 30
⍋2 1 2 1           ⍝ 2ₓ 4ₓ 1ₓ 3ₓ
```

`C⍋Y` uses a character collation array `C`.

```apl
'cba'⍋'abc'        ⍝ 3ₓ 2ₓ 1ₓ
```

See [`⍒`](grade-down.md).
