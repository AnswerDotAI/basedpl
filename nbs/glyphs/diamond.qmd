# `⋄` — Separator

At statement level, `⋄` separates expressions. Newline does the same.

```apl
x←2 ⋄ y←3 ⋄ x+y    ⍝ 5
```

Inside array literals, `⋄` separates items/cells. A trailing separator makes a singleton.

```apl
(42 ⋄)             ⍝ ,42
[1 2 ⋄ 3 4]        ⍝ 2 2⍴1 2 3 4
```
