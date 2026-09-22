# `'` — Character literal

Single quotes delimit characters. One character is scalar; longer or empty literals are vectors.

```apl
⍴'a'               ⍝ 0⍴0ₓ
⍴,'a'              ⍝ ,1ₓ
⍴'abc'             ⍝ ,3ₓ
```

Double an embedded quote.

```apl
≢'can''t'          ⍝ 5ₓ
```
