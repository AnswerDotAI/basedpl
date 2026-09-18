# `( )` — Group / Literal / Train

Parentheses group expressions.

```apl
(2×3)+4            ⍝ 10
(1 2)(3 4)         ⍝ 2⍴(1 2)(3 4)
```

Separators construct nested vectors. A trailing separator makes a singleton.

```apl
(42 ⋄)             ⍝ ,42
(1 2 ⋄ 3 4)        ⍝ (1 2)(3 4)
```

Two functions form an atop: `(f g)Y` is `f(g Y)`.

```apl
(- +/)1 2 3        ⍝ ¯6
```

Three form a fork: `(f g h)Y` is `(f Y)g(h Y)`. Dyadic forks pass both arguments to each arm.

```apl
(+/÷≢)2 4 9        ⍝ 5
```

Longer trains group from the right. A fork's left arm can be a constant. The right arm evaluates first.
