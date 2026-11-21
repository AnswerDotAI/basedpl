

# `∇` — Recursion

Argument. Keys: `Alt-g`. Hanzi: `递` (`遞`)

`∇` calls the current dfn or derived function recursively.

``` bpl
{ω=0?1;ω×∇≤ω}5 ⍝ 120
```

A tail call adds no evaluation depth, unless an error guard is active.
