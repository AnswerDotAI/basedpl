# `⎕` — Output

`⎕←Y` explicitly prints `Y` and retains its value.

```apl
1+⎕←2              ⍝ 3
```

The example prints `2` before returning `3`. Explicit output is separate from implicit result display and is captured by the evaluation API.

System names: [•A](quad-a.md), [•D](quad-d.md), [•C](quad-c.md), [•UCS](quad-ucs.md).
