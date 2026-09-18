# `⍒` — Grade down

`⍒Y` gives indices that sort major cells descending. Stable, with the same [ordering](grade-up.md) as Grade up.

```apl
⍒30 10 20          ⍝ 1x 3x 2x
⍒2 1 2 1           ⍝ 1x 3x 2x 4x
```

`C⍒Y` uses character collation `C`.

```apl
'cba'⍒'abc'        ⍝ 1x 2x 3x
```
