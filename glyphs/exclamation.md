

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
aren’t whole. At a pole, a negative whole float, the result is an
infinity or NaN, as the gamma function gives it. A negative machine
integer gives the largest integer: i64’s under dynamic kinds, where
integers widen, and the width’s own for fixed widths and under static
kinds.

Past 100000, `!` of a machine integer gives `0ₓ`, and of a float `∞`:
`!100001ₓ` is `0ₓ`, and `!100001` is `∞`. A `ᵣ` argument past 100000
raises LIMIT.
