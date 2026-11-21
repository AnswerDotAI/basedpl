

# `?` — Predicate

Syntax. Hanzi: `若`

In a dfn, `C?` ends a predicate. When `C` is true, the rest of the body
runs. When `C` is false, the next body, after `;`, runs.

``` bpl
{ω<0?-ω;ω}¯3      ⍝ 3
{ω<0?-ω;ω}4       ⍝ 4
```

Statements before a predicate run first. After it, a body can run
several statements, or hold more predicates.

``` bpl
{ω<0?a←-ω ◇ a×2;ω}¯3   ⍝ 6
{ω>0?ω<10?ω;0}15       ⍝ 0
```

`C` must be one truth value (`$t`, `$f`, 1 or 0), or an array holding
one. A body that ends after its predicate returns no result. If every
predicate fails, the call raises `DOMAIN`. See [Dfns](../dfns.ipynb).
