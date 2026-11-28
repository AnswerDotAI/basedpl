

# [`⍎`](https://answerdotai.github.io/basedpl/glyphs/hydrant.html) — Execute / Execute in

Function. Keys: `Alt-o b`. Hanzi: `行`. Ranks: `1` monadic, `∞ ∞` dyadic

`⍎Y` evaluates the string `Y` in the current lexical scope.

``` bpl
⍎"2+3"             ⍝ 5
x←7 ◇ ⍎"x+1"       ⍝ 8
```

On a character matrix,
[`⍎`](https://answerdotai.github.io/basedpl/glyphs/hydrant.html)
evaluates each row; on an array of strings, each string.

``` bpl
⍎["1+1"◇"2×3"]    ⍝ 2 6
⍎"1+1" "2×3"        ⍝ 2 6
```

Assignments and explicit output in `⍎Y` take effect in the current
scope.

`T⍎Y` evaluates `Y` with each key on the first axis of `T` as a name, as
[Dot](dot.qmd)’s `T.(expr)` does. Names that aren’t keys resolve in the
current scope. Assignments stay inside the evaluation, and leave `T`
unchanged.

``` bpl
T←["a":1 "b":2] ◇ T⍎"a+b"      ⍝ 3
T←["a":1] ◇ T⍎"a←5 ◇ a+1"      ⍝ 6
```

## Inverse

`⍎⁻Y` writes `Y` as BPL source that
[`⍎`](https://answerdotai.github.io/basedpl/glyphs/hydrant.html) reads
back, as `•literal⁻` does.

``` bpl
⍎⁻1 2.5   ⍝ "[1 2.5]ₚ"
```

## Errors

- `DOMAIN`: `T` has no keys on its first axis
