

# `∩` — Intersection

Keys: `Alt-c`

`X∩Y` keeps the major cells of `X` found among the major cells of `Y`.
Order and repetitions are retained. Uses tolerant matching.

``` bpl
1 2 1 3∩1 3        ⍝ 1 1 3
"abcd"∩"bdf"        ⍝ "bd"
[1 2 ⋄ 3 4 ⋄ 1 2]∩[1 2 ⋄ 5 6] ⍝ [1 2 ⋄ 1 2]
```

## Errors

- `RANK`: `Y` is a unit
- `LENGTH`: the major cells of `X` and `Y` differ in shape
