# `'` — Character literal

Single quotes delimit characters. One character is scalar; longer or empty literals are vectors.

```apl
⍴'a'               ⍝ 0⍴0x
⍴,'a'              ⍝ ,1x
⍴'abc'             ⍝ ,3x
```

Double an embedded quote.

```apl
≢'can''t'          ⍝ 5x
```
