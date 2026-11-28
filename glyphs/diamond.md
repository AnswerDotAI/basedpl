

# [`◇`](https://answerdotai.github.io/basedpl/glyphs/diamond.html) — Separator

Syntax. Keys: `Alt-Backtick`. Hanzi: `句`

At statement level,
[`◇`](https://answerdotai.github.io/basedpl/glyphs/diamond.html)
separates expressions. Everywhere, a line break means
[`◇`](https://answerdotai.github.io/basedpl/glyphs/diamond.html), unless
it’s next to a bracket, parenthesis, brace, `;`,
[`◇`](https://answerdotai.github.io/basedpl/glyphs/diamond.html) or
[`?`](https://answerdotai.github.io/basedpl/glyphs/question.html).

``` bpl
x←2 ◇ y←3 ◇ x+y    ⍝ 5
```

Inside [brackets](brackets.qmd),
[`◇`](https://answerdotai.github.io/basedpl/glyphs/diamond.html)
separates major cells. With a trailing
[`◇`](https://answerdotai.github.io/basedpl/glyphs/diamond.html), the
result is an array of one major cell. A single number is a cell of rank
0: `[42◇]` is a one-item vector.

``` bpl
[1 2◇3 4]        ⍝ 2 2ρ1 2 3 4
ρ[1 2◇]           ⍝ [1 2]ₓ
ρ[42◇]            ⍝ [1]ₓ
```

Inside [parentheses](parentheses.qmd),
[`◇`](https://answerdotai.github.io/basedpl/glyphs/diamond.html)
separates rows, and the result is a vector with one item for each row.
Each item is a vector, even for a row with one item.

``` bpl
(1 2◇3)          ⍝ [[1 2] [3]]
```

[Types of brackets](../bracket-types.ipynb) compares
[`◇`](https://answerdotai.github.io/basedpl/glyphs/diamond.html) in
brackets, parentheses and braces.
