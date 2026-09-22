# `⍒` — Grade down

`⍒Y` gives indices that sort major cells descending. Stable, with the same [ordering](grade-up.md) as Grade up.

```apl
⍒30 10 20          ⍝ 1ₓ 3ₓ 2ₓ
⍒2 1 2 1           ⍝ 1ₓ 3ₓ 2ₓ 4ₓ
```

`C⍒Y` uses character collation `C`.

```apl
'cba'⍒'abc'        ⍝ 1ₓ 2ₓ 3ₓ
```
