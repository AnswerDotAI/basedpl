

# `•detach` — Detach

`•detach x` gives the value of `x` as a constant. Under
[`∂`](../autodiff.qmd), no derivative flows through the result, at any
level of nested
[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html).
Outside
[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html),
`•detach x` gives `x`. Nested arrays and records are detached
throughout.

``` bpl
{+/ω×ω}∂ 1 2 3            ⍝ 2 4 6
{+/ω×•detach ω}∂ 1 2 3    ⍝ 1 2 3
{ω×•detach ω}∂∂ 3         ⍝ 0
```

A value computed before a
[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html) call is
already a constant inside it. `•detach` is needed only inside one
[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html) call,
for a term that comes from the argument but must carry no gradient, such
as a target built from the same parameters, or statistics treated as
constants.

Apply `•detach` to a computation’s inputs rather than to its result.
`net •detach θ` runs `net` on plain values, which costs the same as
running it without
[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html), as
PyTorch’s `torch.no_grad()` does. `•detach net θ` runs `net` under
[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html) first
and then discards its derivative, as `.detach()` does. Only
`net •detach θ` saves the work of differentiating `net`.

A common use is to detach a shift that keeps a result finite. Logsumexp
subtracts the largest item before
[`*`](https://answerdotai.github.io/basedpl/glyphs/star.html) and adds
it back after
[`⊛`](https://answerdotai.github.io/basedpl/glyphs/circle-star.html).
The shift drops out of the gradient. Detaching the input of `⌈/`, as
here, gives the same gradient and keeps
[`∂`](https://answerdotai.github.io/basedpl/glyphs/partial.html) from
differentiating `⌈/`:

``` bpl
lse←{m←⌈/•detach ω ◇ m+⊛+/*ω-m}
lse 1000 1001 1002        ⍝ 1002.407606
lse∂ 1 2 3                ⍝ 0.09003057317 0.2447284711 0.6652409558
```

[`∂`](../glyphs/partial.qmd) differentiates a function.
