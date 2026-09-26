

# `⍒` — Grade down

`⍒Y` gives indices that sort major cells descending. Stable, with the
same [ordering](grade-up.qmd) as Grade up.

``` apl
⍒30 10 20          ⍝ 0ₓ 2ₓ 1ₓ
⍒2 1 2 1           ⍝ 0ₓ 2ₓ 1ₓ 3ₓ
```

`C⍒Y` uses character collation `C`.

``` apl
"cba"⍒"abc"        ⍝ 0ₓ 1ₓ 2ₓ
```
