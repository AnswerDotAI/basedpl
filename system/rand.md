

# `•rand` — Generator

`•rand seed` returns a generator: a keyed vector of two functions that
draw from one stream of random numbers. The seed is a nonnegative
integer. The same seed gives the same draws.

- `roll Y` works like `¿Y`.
- `X deal Y` works like `X¿Y`.

A distribution’s `sample` takes a generator on its left, as in
`g d.sample 3`. Copies of a generator share its stream. Drawing from one
copy moves every copy on.

Errors: `DOMAIN` for a seed that is not a nonnegative integer, or a left
argument to `sample` that is not a generator; `LENGTH` or `RANK` for
more than one seed; `SYNTAX` for a dyadic call to `•rand`.
