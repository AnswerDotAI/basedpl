

# `•memory` — Memory

`•memory x` gives a record of what this thread holds, and of what `x`
keeps alive. `•memory 0` is the usual spelling for the counts alone.

- `pending` counts the live [pending](../terms.qmd#term-pending) arrays.
- `bytes` counts the bytes that `x` keeps alive: its items, the base of
  a [view](../terms.qmd#term-view), the arrays that a pending array
  reads, and the parts of a value that `∂` is differentiating. An
  allocation that `x` reaches twice is counted once.

`pending` counts the pending arrays that this thread made, even when
another thread holds one. Because `•memory` reads no items, it never
[forces](../terms.qmd#term-forced) a pending array. It accepts values
that `∂` is differentiating.

``` bpl
{+/ω×ω}∂ 1 2 3     ⍝ 2 4 6
•memory 0          ⍝ ["pending":0ₓ "bytes":0ₓ]
```

A reshape is a view, which keeps all of its base’s items alive:

``` bpl
v←ι1000ₓ
m←2 500ρv
(•memory m).bytes  ⍝ 2000ₓ
```

[`•explain`](explain.qmd) reports what happened while an expression ran.
