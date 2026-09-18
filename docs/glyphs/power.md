# `⍣` — Iterate / Invert

`f⍣N Y` applies `f` `N` times. `N=0` returns `Y`. In dyadic calls the left argument stays fixed.

```apl
(2∘×⍣3)1           ⍝ 8
2(+⍣3)1            ⍝ 7
```

Negative counts apply the known inverse.

```apl
(2∘×⍣¯1)10         ⍝ 5
10(-\⍣¯1)9 7 4     ⍝ 1 2 3
```

Inverses propagate through composition, binding, Each, rank, dyadic commute/Behind and scans. Primitive inverses include arithmetic bindings, exp/log, circle, axis permutations, encode/decode, enclosure/mix/split and Where. Square roots and logs select branches.

`f⍣g Y` iterates until `new g old` is true, returning `new`.

```apl
1(+⍣{⍺>4})0        ⍝ 5
```

RANK: non-scalar count. DOMAIN: non-integral count, non-Boolean predicate, unknown inverse.
