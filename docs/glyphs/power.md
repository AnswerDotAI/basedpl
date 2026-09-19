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

Array counts give that frame of result cells, assembled with fill.

```apl
(1∘+)⍣3 ¯2 0 3⊢10     ⍝ 13 8 10 13
⍴({1÷0}⍣(0 2⍴0))'ab'  ⍝ 0x 2x 2x
```

Positive steps run first, then inverse steps from `Y`; repeated counts reuse states. Empty counts use `Y`'s cell shape/fill directly.

See [History](history.md) and [Inverse pair](inverse-pair.md). Convergence: `f⍣≡`.

DOMAIN: non-integral count, non-Boolean predicate, unknown inverse.
