

# `•detach` — Detach

`•detach x` gives `x` with every `∂` tag removed. Under `∂`, no
derivative flows back through the result, at any level of nested `∂`.
Outside `∂`, `•detach x` gives `x`. Nested arrays and records are
detached throughout.

``` bpl
{+/⍵×⍵}∂ 1 2 3            ⍝ 2 4 6
{+/⍵×•detach ⍵}∂ 1 2 3    ⍝ 1 2 3
{⍵×•detach ⍵}∂∂ 3         ⍝ 0
```

A value computed before a `∂` call is already a constant inside it.
`•detach` is needed only where the cut falls inside one call: a term
that comes from the argument but must carry no gradient, such as a
target built from the same parameters, or statistics treated as
constants.

Apply `•detach` to a computation’s inputs rather than to its result.
`net •detach θ` runs `net` on plain values, at the cost of running it
without `∂`, as PyTorch’s `torch.no_grad()` does. `•detach net θ` runs
`net` under `∂` first and then discards its derivative, as `.detach()`
does. Only the first form saves the work.

[`∂`](../glyphs/partial.qmd) differentiates a function.
