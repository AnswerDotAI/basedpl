

# `⍶` — Left operand

Keys: `Alt-Minus a`

In a defined operator, `⍶` denotes its left operand: function or
subject.

``` bpl
apply←{⍶ ⍵} ⋄ -apply 3 ⍝ ¯3
add←{⍶+⍵} ⋄ 10 add 3 ⍝ 13
```

`⍺` and `⍵` remain the derived function’s arguments.
