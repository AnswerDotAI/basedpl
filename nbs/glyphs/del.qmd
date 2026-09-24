# `∇` — Function self-reference

`∇` calls the current dfn or derived function recursively.

```apl
{⍵=0:1 ⋄ ⍵×∇⍵-1}5 ⍝ 120
```

Tail calls reuse execution frames. An active error guard prevents tail-call reuse.
