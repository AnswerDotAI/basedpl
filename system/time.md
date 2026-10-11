

# `•time` — Timing

`•time t` gives the seconds since the moment `t`. Moments count seconds
from the Unix epoch, as [`•date`](date.qmd) reads and writes them.
`•time 0` is the current moment. `t←•time 0` starts a timer, and
`•time t` then gives the seconds since it started.

`F •time x` calls each function in `F` on `x` for about 0.1 s. The
result is each function’s fastest time per call in seconds, with the
shape and keys of `F`. With `F←["sum":+/ "max":⌈/]`, `F •time x` labels
each time. Time a dyadic function with its left argument bound, as in
`2↣⍴`. [Timing](../repl.qmd#timing) has examples.

Errors: `DOMAIN` for a left argument that holds anything but functions,
or a `t` that isn’t a number. An error from a timed function stops the
timing.
