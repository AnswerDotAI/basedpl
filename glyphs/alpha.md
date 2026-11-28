

# [`α`](https://answerdotai.github.io/basedpl/glyphs/alpha.html) — Left argument

Argument. Keys: `Alt-a`. Hanzi: `甲`

[`α`](https://answerdotai.github.io/basedpl/glyphs/alpha.html) is the
current dfn’s left argument. In a call with one argument,
[`α`](https://answerdotai.github.io/basedpl/glyphs/alpha.html) is absent
and drops out of the expression that uses it. For example, `{α-ω}3` is
`¯3`. [Dfns](../dfns.ipynb#arguments) gives the full rule.

``` bpl
10{α-ω}3           ⍝ 7
```

`α←expression` supplies a lazy default, evaluated only on monadic calls.

``` bpl
{α←2 ◇ α×ω}3       ⍝ 6
4{α←2 ◇ α×ω}3      ⍝ 12
```
