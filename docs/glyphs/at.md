# `@` — At

`A@I Y` replaces selected items with `A`, returning a new array.

```apl
0@2 4⊢⍳5           ⍝ 1 0 3 0 5
```

`f@I Y` applies `f` to the selection, then puts it back.

```apl
⌽@2 4⊢⍳5           ⍝ 1 4 3 2 5
```

A function right operand computes a Boolean selection mask.

```apl
0@(2∘|)⍳5          ⍝ 0 2 0 4 0
```

Indices include nested paths. Dyadic calls pass the left argument to the replacement function. Repeated indices are written in order; the last replacement wins.
