# `⍎` — Execute

`⍎Y` evaluates character code in the current lexical scope.

```apl
⍎'2+3'             ⍝ 5
x←7 ⋄ ⍎'x+1'       ⍝ 8
```

`X⍎Y` selects `Y⊃X`, using numeric coordinates or axis keys.

```apl
T←'aa' 'bb':10 20
T⍎'bb'             ⍝ 20
10 20⍎2            ⍝ 20
```

Assignments and explicit output take effect in the current session.
