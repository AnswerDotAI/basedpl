# `⍎` — Execute

`⍎Y` evaluates character code in the current lexical scope.

```apl
⍎'2+3'             ⍝ 5
x←7 ⋄ ⍎'x+1'       ⍝ 8
```

`X⍎Y` runs no code. It looks up key `Y` in the keyed array `X`, exactly as `Y⊃X` does.

```apl
T←('a':10 ⋄ 'b':20)
T⍎'b'              ⍝ 20
```

Assignments and explicit output take effect in the current session.
