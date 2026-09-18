# `←` — Assign

`N←Y` binds a name to an array, function or operator. Chained assignment returns the assigned value silently.

```apl
x←3 ⋄ x+2          ⍝ 5
sum←+/ ⋄ sum 1 2 3 ⍝ 6
```

`N f←Y` updates with `N f Y`. Bracket and selective forms update selected items.

```apl
x←10 ⋄ x+←3 ⋄ x    ⍝ 13
x←1 2 3 ⋄ x[2]←9 ⋄ x ⍝ 1 9 3
x←1 2 3 ⋄ (2↑x)←8 9 ⋄ x ⍝ 8 9 3
```

Strands assign corresponding items.

```apl
(a b)←10 20 ⋄ a+b  ⍝ 30
```

Inside dfns, plain assignment is local. Modified/indexed/selective array updates target the nearest existing binding. Arrays have value semantics.
