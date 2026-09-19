# `•UCS` — Unicode

`•UCS Y` converts characters ↔ exact integer code points, preserving shape.

```apl
•UCS 'ABC'         ⍝ 65x 66x 67x
•UCS 65 66 67      ⍝ 'ABC'
```

`E•UCS Y` encodes characters or decodes integer units. `E` is `'UTF-8'`, `'UTF-16'` or `'UTF-32'`.

```apl
'UTF-8'•UCS 'é'    ⍝ 195x 169x
'UTF-8'•UCS 195 169 ⍝ ,'é'
```

Encoding forms take vectors; scalars become singletons. DOMAIN: invalid code point or malformed encoding.
