

# `⍳` — Index generator / Index of

`⍳N` generates `0…N-1`. A negative `N` counts down, as J’s `i.` does.
Exact input gives exact coordinates.

``` apl
⍳4                 ⍝ 0 1 2 3
⍳3x                ⍝ 0ₓ 1ₓ 2ₓ
⍳¯3                ⍝ 2 1 0
```

A shape vector generates an array of coordinate vectors. A negative
length reverses its axis.

``` apl
⍳2 2               ⍝ [[0 0] [0 1] ⋄ [1 0] [1 1]]
⍳2 ¯2              ⍝ [[0 1] [0 0] ⋄ [1 1] [1 0]]
```

`⍳⍠A Y` returns the keys of axis `A`, or `0…length-1` on an unkeyed
axis. Several axes return a vector of selector vectors.

``` apl
⍳⍠0 ["price":1 "qty":2] ⍝ "price" "qty"
⍳⍠1 (2 3⍴0)         ⍝ 0ₓ 1ₓ 2ₓ
```

On an axis where only some positions have keys, `⍳⍠A Y` gives the keys
where present and positions elsewhere.

``` apl
⍳⍠0 ["aa":10 20]    ⍝ "aa" 1ₓ
```

`X⍳Y` finds the first matching major cell in `X` for each cell of `Y`.
Not found: `≢X`. Uses tolerant matching and returns exact positions.

``` apl
"abc"⍳"cabz"        ⍝ 2ₓ 0ₓ 1ₓ 3ₓ
"abc"⍳'b'           ⍝ 1ₓ
"abc"⍳⊂'b'          ⍝ 1ₓ
```

A single query returns an atom. Batch axes supply the result shape.
Enclose an array-valued query to search for it as one item.

``` apl
C←"cat" "dog"
C⍳⊂"dog"           ⍝ 1ₓ
C⍳["dog";]         ⍝ [1ₓ;]
m←[10 20 ⋄ 30 40 ⋄ 50 60]
m⍳30 40            ⍝ 1ₓ
m⍳[30 40 ⋄]        ⍝ [1ₓ;]
m⍳[30 40 ⋄ 10 20]  ⍝ 1ₓ 0ₓ
```
