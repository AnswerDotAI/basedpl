

# `⍎` — Execute

Keys: `Alt-o b`. Ranks: `1` monadic, `∞ ∞` dyadic

`⍎Y` evaluates character code in the current lexical scope.

``` bpl
⍎"2+3"             ⍝ 5
x←7 ⋄ ⍎"x+1"       ⍝ 8
```

On a character matrix, `⍎` evaluates each row. On an array of strings,
it evaluates each string.

``` bpl
⍎["1+1"⋄"2×3"]    ⍝ 2 6
⍎"1+1" "2×3"        ⍝ 2 6
```

Assignments and explicit output in `⍎Y` take effect in the current
scope.

`T⍎Y` evaluates `Y` with each key of the keyed vector `T` as a name for
its item, as [Dot](dot.qmd)’s `T.(expr)` does. Names that aren’t keys
resolve in the current scope. Assignments stay inside the evaluation,
and leave `T` unchanged.

``` bpl
T←["a":1 "b":2] ⋄ T⍎"a+b"      ⍝ 3
T←["a":1] ⋄ T⍎"a←5 ⋄ a+1"      ⍝ 6
```

## Inverse

`⍎⁻¹Y` writes `Y` as BPL source that `⍎` reads back, as `•literal⁻¹`
does.

``` bpl
⍎⁻¹1 2.5   ⍝ "[1 2.5]ₚ"
```

## Errors

- `DOMAIN`: `T` isn’t a keyed vector
