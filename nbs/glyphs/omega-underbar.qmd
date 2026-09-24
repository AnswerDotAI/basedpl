# `⍹` — Right operand

`⍹` makes a defined operator dyadic and denotes its right operand.

```apl
op←{⍶+⍹×⍵} ⋄ (2 op 3)4 ⍝ 14
```

Operands can be arrays or functions; arguments remain `⍺` and `⍵`.
