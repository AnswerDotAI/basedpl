# `¨` — Each

`f¨Y` applies `f` to each item, enclosing each result.

```apl
≢¨(1 2)(3 4 5)     ⍝ 2x 3x
```

`X f¨Y` pairs items using leading agreement.

```apl
1 2+¨10            ⍝ 11 12
```

On empty arguments, Each calls `f` once using prototypes. Prototype mode permits Pick to select fill.

```apl
{100⊃'abc'}¨⍬      ⍝ ''
```

Other errors and explicit output remain observable in that call.
