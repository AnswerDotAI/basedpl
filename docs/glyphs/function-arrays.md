# `⊙` — Function arrays

`f⊙g` ties two functions into a vector. Either operand can also be a scalar function element or a vector of functions. Chaining preserves their order.

```apl
fs←+⊙×⊙÷ ⋄ mul←2⊃fs ⋄ 2 mul 3                  ⍝ 6
fs←+/⊙{⍵×⍵}⊙(3∘+) ⋄ square←2⊃fs ⋄ square 4     ⍝ 16
fs←+⊙× ⋄ ≢fs⊙÷                                 ⍝ 3x
```

Ordinary `⊃` retrieves the function as a callable. Monadic `⊃` discloses the first item. Dyadic `⊃` follows a path through nested arrays. An empty path returns its argument unchanged. Further path steps cannot traverse a function. Bracket indexing keeps array role.

```apl
fs←+⊙×⊙÷ ⋄ div←⊃⌽fs ⋄ 6 div 3                 ⍝ 2
fs←+⊙× ⋄ result←(⊂1 2 3),fs[2] ⋄ f←2⊃result ⋄ 2 f 3 ⍝ 6
x←(+⊙×)(-⊙÷) ⋄ f←1 2⊃x ⋄ 2 f 3               ⍝ 6
fs←+⊙× ⋄ f←{2⊃⍵}fs ⋄ 2 f 3                   ⍝ 6
```

Function elements are shared handles, not source strings. Reshape, selection, catenate and other structural operations move them without executing them. Display encloses each function in `⟨…⟩`. A function's fill is that same function. Equality and match compare handle identity, not mathematical equivalence. Functions have no grade or interval ordering.

Dfns can return functions. Captured local functions can be used while their defining call is active. Returning or assigning them into a longer-lived scope raises DOMAIN ERROR. This includes arrays and empty prototypes retaining those functions. Escaping closures are not supported.

Python `Array` accepts miniapl functions as elements. Indexing keeps array role. The word functions `first` and `pick` return selected callables. `.py` and `.np` expose callable function elements and retain their originating session. Arrays containing session-bound functions cannot be passed to another session. JSON export rejects functions and function arrays. It never reconstructs executable functions from source text.

## `◶` — Agenda

`selector◶cases` derives a function that selects a branch on each call. `cases` must be a nonempty vector of functions. The selector can be a scalar index or a function returning one. Indices are integral and start at 1.

```apl
cases←-⊙⊢ ⋄ abs←{1+⍵≥0}◶cases ⋄ abs ¯3       ⍝ 3
cases←-⊙⊢ ⋄ abs←{1+⍵≥0}◶cases ⋄ abs 3        ⍝ 3
mul←2◶(+⊙×) ⋄ 2 mul 3                        ⍝ 6
choose←{1+⍺>⍵}◶(-⊙÷) ⋄ 12 choose 3           ⍝ 4
```

A function selector runs once with the original arguments. Only the selected branch runs, with those same arguments. Non-scalar indices raise RANK ERROR. Nonnumeric or nonintegral indices raise DOMAIN ERROR. Out-of-range indices raise INDEX ERROR. Index arrays do not map over branches or construct trains.
