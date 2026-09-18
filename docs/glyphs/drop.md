# `↓` — Split / Drop

`↓Y` nests vectors along the last axis.

```apl
↓2 3⍴⍳6            ⍝ (1 2 3)(4 5 6)
```

`N↓Y` drops from the front; negative `N` drops from the end.

```apl
2↓1 2 3 4          ⍝ 3 4
¯2↓1 2 3 4         ⍝ 1 2
```

`[K]` selects the split axis or drop axes.

```apl
↓[1]2 3⍴⍳6         ⍝ (1 4)(2 5)(3 6)
```

Vector counts apply to successive leading axes. Overdrop gives an empty axis.

```apl
1 1↓2 3⍴⍳6         ⍝ 1 2⍴5 6
5↓1 2              ⍝ ⍬
```

See [Mix / Take](take.md).
