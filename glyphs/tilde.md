

# `~` — NOT / Without

`~Y` negates Booleans, pervasively.

``` apl
~0 1               ⍝ 1ₓ 0ₓ
```

`X~Y` removes items found in `Y`, retaining order and repetitions. Uses
tolerant membership.

``` apl
1 2 1 3~2          ⍝ 1 1 3
"banana"~"an"       ⍝ "b"
```
