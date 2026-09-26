

# `⍺` — Left argument

`⍺` is the current dfn’s left argument.

``` apl
10{⍺-⍵}3           ⍝ 7
```

`⍺←expression` supplies a lazy default, evaluated only on monadic calls.

``` apl
{⍺←2 ⋄ ⍺×⍵}3       ⍝ 6
4{⍺←2 ⋄ ⍺×⍵}3      ⍝ 12
```
