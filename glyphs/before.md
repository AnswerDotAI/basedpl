

# `↣` — Before

Keys: `Alt-.`

`f↣g Y` is `(f Y)g Y`, where `f` prepares the left argument of `g`.
Operators bind from the left. For example, `3↣<↣#` is `(3↣<)↣#`, which
keeps the items greater than 3.

``` bpl
-↣+ 5              ⍝ 0
3↣<↣#2 7 1 8       ⍝ 7 8
' '↣≠↣⊆"a bc  d"   ⍝ "a" "bc" "d"
```

`X f↣g Y` is `(f X)g Y`.

``` bpl
10 -↣+ 3           ⍝ ¯7
```

`A↣f` binds the left argument. The bound function takes one argument. A
train binds the same way: `10- y` is `(10↣-)y`.

``` bpl
10↣- 3             ⍝ 7
32+1.8↣× 100       ⍝ 212
```

See [After](after.qmd), [Atop / Rank](rank.qmd) and [Over](over.qmd).
