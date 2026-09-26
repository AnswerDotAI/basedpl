

# `( )` — Group / Train

Parentheses only group. They hold one expression wherever they appear,
including inside [brackets](brackets.qmd).

``` apl
(2×3)+4            ⍝ 10
x←2 4 9
[(+/x) ≢x]         ⍝ 15 3
```

A line break inside parentheses counts as a space. So an expression can
continue across lines. Outside brackets, spaces separate units, and each
unit is evaluated first. So this means `(1+2)×(3+4)`:

``` apl
total←(1+2
       × 3+4)
total              ⍝ 21
```

`⋄` inside parentheses is a `SYNTAX` error. Brackets write lists:
`[a b]` is a vector, and `[a;]` is a one-item vector.

Two functions form an atop: `(f g)Y` is `f(g Y)`.

``` apl
(- +/)1 2 3        ⍝ ¯6
```

Three form a fork: `(f g h)Y` is `(f Y)g(h Y)`. Dyadic forks pass both
arguments to each arm.

``` apl
(+/÷≢)2 4 9        ⍝ 5
(+/ ÷ ≢)2 4 9      ⍝ 5
```

Longer trains group from the right. A fork’s left arm can be a constant.
The right arm evaluates first.
