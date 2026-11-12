

# `÷` — Reciprocal / Divide

Function. Keys: `Alt-u`. Hanzi: `除`. Ranks: `0` monadic, `0 0` dyadic

`÷Y`: 1/Y. Pervasive. `X÷Y`: X/Y. Pervasive. Machine integers use floor
division. Float arguments use floating division, and `ᵣ` arguments use
exact rational division. Select float or rational division explicitly
with `÷ₐ` or `÷ᵣ`:

``` bpl
÷4                ⍝ 0.25
1ₓ÷3ₓ             ⍝ 0ₓ
¯3ₓ÷2ₓ            ⍝ ¯2ₓ
1ₓ÷ₐ3ₓ            ⍝ 0.3333333333333333
1ᵣ÷3ᵣ             ⍝ 1ᵣ3
1ₓ÷ᵣ3ₓ            ⍝ 1ᵣ3
```

A nonzero float divided by zero gives an infinity. Floating zero divided
by zero gives zero for [Mathish types](../terms.qmd#term-mathish-types)
with [Mathish mode](../numbers.qmd#mathish-mode) enabled, and NaN
otherwise:

``` bpl
¯1÷0              ⍝ ¯∞
0÷0               ⍝ 0
```

An integer divided by zero gives the integer limit of its sign: i64’s
under dynamic kinds, where integers widen, and the width’s own for fixed
widths and under static kinds. An integer zero divided by zero gives
zero, with Mathish mode on or off. `÷ₓ` follows the same rule:

``` bpl
5ₓ₈÷0ₓ₈           ⍝ 9223372036854775807ₓ₈
¯5ₓ₈÷0ₓ₈          ⍝ ¯9223372036854775808ₓ₈
0ₓ÷0ₓ             ⍝ 0ₓ
```

## Inverse

`÷⁻` is `÷`. With the right argument fixed, `(÷↢X)⁻Y` is `Y×X`. These
recover the original value for nonzero floating or rational division;
integer floor division loses the remainder.

``` bpl
12÷⁻4     ⍝ 3
÷↢4 ⁻ 3   ⍝ 12
```

## Errors

- `DOMAIN`: a character argument
