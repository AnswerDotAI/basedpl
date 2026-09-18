# `⎕C` — Case conversion

`⎕C Y` folds case.

```apl
⎕C 'AbC'           ⍝ 'abc'
```

`1⎕C Y` uppercases; `¯1⎕C Y` lowercases.

```apl
1⎕C 'AbC'          ⍝ 'ABC'
¯1⎕C 'AbC'         ⍝ 'abc'
```

Unicode simple mappings preserve shape and nesting; non-character leaves stay unchanged.
