

# `•rand` — Generator

`•rand seed` returns a generator: a keyed vector of two functions that
draw random numbers. The nonnegative whole number `seed` initializes its
state.

- `roll Y` works like `¿Y`.
- `X deal Y` works like `X¿Y`.

A distribution’s `sample` takes a generator on its left, as in
`g d.sample 3`. Copies of a generator share its state.

A [batched map](../terms.qmd#term-batched-call) can draw a whole array
at once. Draws can differ with batching, thread count or scheduling,
execution route, platform or build.

``` bpl
g←•rand 3 ◇ h←•rand 3 ◇ (g.roll 4ρ6)≡h.roll 4ρ6    ⍝ $t
```

`¿` draws from the session’s generator, which
[`•prefs ["seed":n]`](prefs.qmd) seeds.

Errors: `DOMAIN` for a seed that is not a nonnegative whole number, or a
left argument to `sample` that is not a generator; `LENGTH` or `RANK`
for more than one seed; `SYNTAX` for a dyadic call to `•rand`.
