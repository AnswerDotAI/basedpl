

# `∪` — Unique / Union

`∪Y` keeps the first of each matching major cell. Uses tolerant
matching.

``` apl
∪3 1 3 2 1         ⍝ 3 1 2
```

`X∪Y` appends items of `Y` absent from `X`. Order and repetitions are
retained.

``` apl
1 2∪2 3            ⍝ 1 2 3
1 1∪2 2            ⍝ 1 1 2 2
```
