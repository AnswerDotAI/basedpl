# `↑` — Mix / Take

`↑Y` assembles nested items into cells, padding to a common shape.

```apl
↑(1 2)(3 4 5)      ⍝ 2 3⍴1 2 0 3 4 5
```

`N↑Y` takes leading items; negative `N` takes from the end. Overtake inserts fill.

```apl
4↑1 2              ⍝ 1 2 0 0
¯4↑1 2             ⍝ 0 0 1 2
```

Vector counts apply to successive leading axes. `[K]` selects axes.

```apl
1 2↑2 3⍴⍳6         ⍝ 1 2⍴1 2
```

On Mix, axes place cell dimensions among frame dimensions; fractional positions insert before an axis.
