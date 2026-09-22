# `•ucs` — Unicode

`•ucs Y` converts characters ↔ exact integer code points, preserving shape.

```apl
•ucs 'ABC'         ⍝ 65x 66x 67x
•ucs 65 66 67      ⍝ 'ABC'
```

`E•ucs Y` encodes characters or decodes integer units. `E` is `'UTF-8'`, `'UTF-16'` or `'UTF-32'`.

```apl
'UTF-8'•ucs 'é'    ⍝ 195x 169x
'UTF-8'•ucs 195 169 ⍝ ,'é'
```

Encoding forms take vectors; scalars become singletons. DOMAIN: invalid code point or malformed encoding.
