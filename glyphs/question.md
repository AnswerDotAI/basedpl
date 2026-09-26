

# `?` — Roll / Deal

`?N`: uniform draw from `⍳N`. `?0`: U(0,1). Pervasive.

``` apl
(?6)∊⍳6           ⍝ 1ₓ
```

`M?N`: `M` draws without replacement from `⍳N`.

``` apl
≢3?10             ⍝ 3ₓ
≢∪3?10            ⍝ 3ₓ
```

Bounds are nonnegative integers; `M≤N`. Exact arguments give exact
integer draws.
