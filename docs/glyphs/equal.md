# `=` — Equal

`X=Y` tests equality, pervasively.

```apl
1 2 3=2            ⍝ 0x 1x 0x
0.3=0.1+0.2        ⍝ 1x
'abc'='b'          ⍝ 0x 1x 0x
```

See [tolerance](../rules.md#equality-and-ordering) and whole-array [match `≡`](match.md).
