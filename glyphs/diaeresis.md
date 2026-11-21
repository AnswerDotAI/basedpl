

# `¨` — Each

Monadic operator. Keys: `Alt-1`. Hanzi: `遍`

`f¨Y` applies `f` to each item and collects the returned values in `Y`’s
shape. A call that gives no result contributes `⍬`. On atoms it calls
`f` directly.

``` bpl
≢¨[1 2;3 4 5]      ⍝ [2 3]ₓ
```

`X f¨Y` pairs items using [leading
agreement](../evaluation.qmd#agreement-and-pervasion).

``` bpl
1 2+¨10            ⍝ 11 12
```

On empty arguments, Each calls `f` once using
[prototypes](../terms.qmd#term-prototype). If that call raises an error
that [`∞::`](error-guard.qmd) would catch, the result is empty, with
prototype 0. In that call, a system function that reads or writes
outside BPL returns a prototype of its result and does nothing else:
`•nget` reads no file.

``` bpl
{100⊃"abc"}¨⍬      ⍝ ""
```

INTERRUPT, TIMEOUT and UNSUPPORTED are still raised, and output from
that call still appears.

Where it can, Each calls `f` once on all the items together, in a
[batched call](../evaluation.qmd#functions-and-effects).

## Inverse

`f¨⁻` is `f⁻¨`.

``` bpl
⊽¨⁻[2 4;6]   ⍝ [1 2;3]
```
