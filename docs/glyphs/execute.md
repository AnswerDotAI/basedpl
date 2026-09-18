# `⍎` — Execute

`⍎Y` evaluates character code in the current lexical scope.

```apl
⍎'2+3'             ⍝ 5
x←7 ⋄ ⍎'x+1'       ⍝ 8
```

`''⍎Y` also uses the current scope.

```apl
''⍎'2+3'           ⍝ 5
```

Assignments and explicit output take effect in the current session.
