

# `[ ]` — Vector / Array notation

`[A B C]` is a vector of the items `A`, `B` and `C`, whatever their
number. Inside brackets, spaces separate items, and a line break means
`⋄`. Items can be subjects or functions, and they are evaluated left to
right. `[Y]` is a one-item vector, and `[]` is the empty vector `⍬`. A
strand of literals needs no brackets: `1 2 3` is `[1 2 3]`.

``` bpl
a←1 ⋄ b←2
[a+b a×b]          ⍝ 3 2
≢[+/ ÷ ≢]          ⍝ 3ₓ
[5]≡,5             ⍝ $t
[]≡⍬               ⍝ $t
```

`[A;B C]` separates items with `;`. Inside each item, spaces separate
runs, as they do outside brackets.

``` bpl
[1 2;3 4]          ⍝ [[1 2] [3 4]]
x←2 4 9
[+/x ; +/x ÷ ≢x]   ⍝ 15 5
```

`[A⋄B]` is an array whose major cells are `A` and `B`, padded with fill
to a common shape. `[A⋄]` has the single major cell `A`. A row that is
one number is a cell of rank 0.

``` bpl
[1 2⋄3]          ⍝ [1 2⋄3 0]
⍴[1 2 3⋄]         ⍝ [1 3]ₓ
[1⋄2]            ⍝ 1 2
⍴[[1]⋄[2]]       ⍝ [2 1]ₓ
```

In `[…]ₓ`, every number is exact, at any depth. Each item must be a
literal, and each number must be whole.

``` bpl
[1 0 1]ₓ≡1ₓ 0ₓ 1ₓ  ⍝ $t
```

A bracket list with any `key:value` item is one [keyed
vector](../keyed.ipynb). An item without a key is a position with no
key.

``` bpl
["aa":1 "bb":2]              ⍝ "aa" "bb":1 2
["a":1 "b":(•ucs 65 66)]     ⍝ "a" "b":[1 "AB"]
```

As an assignment target, `[a b]←Y` [destructures](assign.qmd). Brackets
don’t index. [Index](squad.qmd) `⌷` selects. [Types of
brackets](../bracket-types.ipynb) compares brackets with parentheses and
braces.

## Errors

- `SYNTAX`: a trailing `;`, or both `;` and `⋄` in one pair of brackets
