

# `←` — Assign

`N←Y` binds a name to an array, function or operator. Chained assignment
returns the assigned value silently.

``` apl
x←3 ⋄ x+2          ⍝ 5
sum←+/ ⋄ sum 1 2 3 ⍝ 6
```

`←` binds tightly on its left and loosely on its right. The target is
the unit immediately before `←`: a name, a group, or arrays applied to
each other such as `x[1]`. The value is everything to its right, so
spaces there group units as usual.

``` apl
x←1+2 × 3 ⋄ x      ⍝ 9
```

`N f←Y` updates with `N f Y`. An array applied to positions is a
selective target, so `x[1]←9` updates item 1. Other selections need
parentheses: `(f N)←Y` updates the items that `f` selects, as
[Index](squad.qmd) `⌷` does. Assignment to a [dot index](dot.qmd), such
as `x.(1)←9`, goes through `⌷`.

``` apl
x←10 ⋄ x+←3 ⋄ x    ⍝ 13
x←1 2 3 ⋄ x[1]←9 ⋄ x ⍝ 1 9 3
x←1 2 3 ⋄ x[0 2]+←10 ⋄ x ⍝ 11 2 13
x←1 2 3 ⋄ x.(1)←9 ⋄ x ⍝ 1 9 3
x←1 2 3 ⋄ (2↑x)←8 9 ⋄ x ⍝ 8 9 3
```

Brackets destructure: each name gets the corresponding item.

``` apl
[a b]←10 20 ⋄ a+b  ⍝ 30
```

Independent assignments need `⋄`, as in `a←1 ⋄ b←2`.

Inside dfns, plain assignment is local. Modified and selective array
updates target the nearest existing binding. Arrays have value
semantics.
