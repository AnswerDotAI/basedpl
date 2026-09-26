

# `⍶` — Left operand

In a defined operator, `⍶` denotes its left operand: function or array.

``` apl
apply←{⍶ ⍵} ⋄ (-apply)3 ⍝ ¯3
add←{⍶+⍵} ⋄ (10 add)3 ⍝ 13
```

`⍺` and `⍵` remain the derived function’s arguments.
