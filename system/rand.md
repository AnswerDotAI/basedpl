

# `•rand` — Generator

`•rand seed` returns a generator: a keyed vector of two functions that
draw from one stream of random numbers. The seed is a nonnegative
integer. The same seed gives the same draws.

- `roll Y` works like `¿Y`.
- `X deal Y` works like `X¿Y`.

A distribution’s `sample` takes a generator on its left, as in
`g d.sample 3`. Copies of a generator share its stream. Drawing from one
copy moves every copy on.

In a [map](../terms.qmd#term-map), the cells take the generator’s
numbers in turn. A [batched
call](../evaluation.qmd#functions-and-effects) takes every cell’s
numbers for one draw before its next draw. A map whose cells each draw
once gives the same numbers either way. A guard that splits the cells is
the exception: each side of the guard draws for its own cells in turn.
Here the two cells take the stream’s next four numbers:

``` bpl
g←•rand 3 ⋄ a←{g.roll ⍵⍴6}¨1 3 ⋄ h←•rand 3 ⋄ ((0⊃a),1⊃a)≡h.roll 4⍴6    ⍝ $t
```

`¿` draws from the session’s own stream, which
[`•prefs ["seed":n]`](prefs.qmd) seeds.

Errors: `DOMAIN` for a seed that is not a nonnegative integer, or a left
argument to `sample` that is not a generator; `LENGTH` or `RANK` for
more than one seed; `SYNTAX` for a dyadic call to `•rand`.
