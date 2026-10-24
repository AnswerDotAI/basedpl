

# Style

These rules apply to BPL in tests, examples, documentation and
libraries.

Use lowercase `j` in complex literals throughout tests and examples,
including adapted reference cases. Reserve uppercase `J` for explicit
input-alias tests. Keep archived upstream source unchanged. Write
literal matrices in array notation, `[10 20 30⋄40 50 60]`, not as a
reshape. Keep `⍴` where the example is about reshape. A `⋄` that
separates rows has no spaces around it. A `⋄` that separates statements
has a space on each side.

Brackets are the usual way to write a list, and to build an array from
names or expressions. A bare literal list, such as `1 2 3`, is a
shorthand. Write a list of names in brackets, as in `[a b]`, even where
`a b` would also work. Write `[n 2]⍴0`, `[x],y`, `[a⋄b]` and `f⍤1 m`,
not APL’s `(n,2)⍴0`, `(⊂x),y`, `⊃[a b]` and `⊃f¨↓m`. When no item of a
bracket list contains a space, separate the items with spaces, not `;`:
write `[i j]`, not `[i;j]`.

End an operand with a space where possible. After a dyadic operator and
a space, the whole next run is the operand. For example, `⌊@ 10× x` and
`≢⊢ ⌸x` need no parentheses. When both operands of a dyadic operator are
compound, keep both in parentheses, as in `(•ucs 9)@(⍸tabs) ⍵`. An
operand keeps its parentheses when a left argument comes before it, as
in `tree (⍺⍢)/ next`. Where a literal operand would run into the
argument, a vector argument goes in touching brackets: `f⍤1[2 3]`. For a
simple numeric argument, `f⍣¯2⊢5` can read more clearly than `f⍣¯2(5)`.
Judge each line by how it reads, leaning towards leaving parentheses
out. Keep them when they clarify grouping. Report any case where none of
these forms works.

Prefer a train to naming an argument twice: `+/÷≢ v`, not `(+/v)÷≢v`.

`fib10` means `fib 10`, not an identifier (which would be `fib_10`).
Write calls that way where it’s shorter and needs no new parentheses.
Keep the space where the literal would join the next part of the
expression. A left argument must be in the same run as its function:
prefer `32•float∞` to `32 •float ∞`. `32 •float∞` gives the two items
`32 64ₓ`, because `•float∞` alone gives ∞’s width. Between two names,
keep both spaces: `x f 5`, not `x f5`.

Read a selection with `⌷`, as in `[⍋v]⌷v`. Write one literal position
with a subscript, as in `v₁` and `v₁←0`. Use `.` for other assignment
targets, such as `v.[i]←0`, for record fields, such as `T.name`, and
wherever `⌷` would be longer. For a target that is a cross product of
positions, write `⌷` with a spaced `←`, as in `[1 3;0 5]⌷a +←10`. `.`
pairs its index arrays instead: `a.[1 3;0 5]` selects only the items at
positions 1 0 and 3 5.

Positions and counts are exact, while bare numbers are floats.
Arithmetic that mixes the two gives floats. A write that mixes them
leaves the array in mixed storage. Write `⍳10ₓ` or `⍳≢x` for exact
positions, not `⍳10`. Seed an array that will hold positions or counts
with exact numbers, as in `n⍴¯1ₓ`, and start a counter at `0ₓ`. On
positions, write `≥i`, `i+1ₓ` and `2ₓ×i`, not `i+1` and `2×i`. For an
exact `v`, `+/÷≢ v` gives an exact rational. Write `+/÷1×≢ v` where a
float is meant. Where a map’s branches give Booleans and numbers,
convert the Boolean: `+b` gives exact integers, and `1×b` gives floats.
