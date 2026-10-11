

# `¨` — Each

Keys: `Alt-1`

`f¨Y` applies `f` to each item and collects the returned values in `Y`’s
shape. A call that gives no result contributes `⍬`. On atoms it calls
`f` directly.

``` bpl
≢¨[1 2;3 4 5]      ⍝ [2 3]ₓ
```

`X f¨Y` pairs items using leading agreement.

``` bpl
1 2+¨10            ⍝ 11 12
```

On empty arguments, Each calls `f` once using prototypes. If that call
fails, the result is empty, with prototype 0. Prototype mode permits
Pick to select fill.

``` bpl
{100⊃"abc"}¨⍬      ⍝ ""
```

Other errors and explicit output remain observable in that call.

## Inverse

`f¨⁻¹` is `f⁻¹¨`.

``` bpl
⊽¨⁻¹[2 4;6]   ⍝ [1 2;3]
```
