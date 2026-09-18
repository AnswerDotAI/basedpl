# `⌊` — Floor / Minimum

`⌊Y` rounds down. Pervasive; tolerant near integers.

```apl
⌊1.8 ¯1.8         ⍝ 1 ¯2
⌊3r2              ⍝ 1x
```

Complex floor uses APL's Gaussian-integer rule.

```apl
⌊1.5j0.5          ⍝ 1j1
```

`X⌊Y` takes the real minimum. Pervasive. Empty reduction identity: `∞`.

```apl
3⌊1 5             ⍝ 1 3
```
