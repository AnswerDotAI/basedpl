

# `!` — Factorial / Binomial

Function. Hanzi: `阶/拣` (`階/揀`). Ranks: `0` monadic, `0 0` dyadic

`!Y`: Y!. Pervasive.

``` bpl
!5                ⍝ 120
!5ₓ               ⍝ 120ₓ
```

`X!Y`: binomial coefficient, Y choose X. Pervasive.

``` bpl
2!5               ⍝ 10
2ₓ!5ₓ             ⍝ 10ₓ
```

Extends through Gamma to complex arguments and to real arguments that
aren’t whole. At a pole, a negative whole float, `!Y` is an infinity or
NaN, as the gamma function gives it. Where `X!Y` has no result, such as
`0.5!¯1`, it raises `DOMAIN` under dynamic kinds and gives NaN under
[static kinds](../numbers.qmd#static-kinds). `!` of a negative machine
integer gives the largest integer: i64’s under dynamic kinds, where
integers widen, and the width’s largest for fixed widths and under
static kinds.

A machine-integer `!Y` wraps, and is `0ₓ` from `!66ₓ` at 64 bits. A
float gives `∞` from `!171`. A `ᵣ` argument past 100000 raises LIMIT.
