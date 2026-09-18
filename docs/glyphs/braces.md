# `{ }` — Dfn / Dop

`{…}` defines a function with arguments `⍺` and `⍵`. Names follow lexical scope.

```apl
{⍵×2}3             ⍝ 6
2{⍺+⍵}3            ⍝ 5
```

The first result-producing non-assignment expression returns. Guards choose a result; a final assignment returns silently.

```apl
{⍵<0:-⍵ ⋄ ⍵}¯3     ⍝ 3
{x←⍵+1}3           ⍝ 4
```

Using `⍺⍺` defines a monadic operator; `⍵⍵` makes it dyadic.

```apl
twice←{⍺⍺ ⍺⍺ ⍵} ⋄ (-twice)3 ⍝ 3
```

See [defaults](alpha.md), [recursion](nabla.md) and [error guards](error-guard.md).
