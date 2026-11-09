⍝⍝ setup
⍝ Cases whose counts check that their costs grow linearly and that they allocate what their results need, with no time budgets. A cost that stops growing linearly counts far more for each item at 65,536 items: a quadratic one thousands of times as much. Pervasive and structural functions write their result once. A reshape that keeps the number of items shares its argument's storage. Indexing builds a table of its positions. Search builds a table of the items it searches. Format builds each number's text once, then converts it to characters. `arrays.bpl` counts the other functions of this kind.
n←65536 size 512
v←¿n⍴0 ⋄ w←¿n⍴0 ⋄ iv←⍳nₓ ⋄ b←0=3|iv ⋄ i←¿n⍴nₓ ⋄ m←[n÷2 2]⍴v ⋄ q←iv÷7
rows←1024 size 8 ⋄ pairs←[rows 2]⍴v
⍝⍝ growth
*v
⍝⍝: -	13	-	instructions macos 25.801 linux 12.258	allocated macos 8.119 linux 8.119

iv+1ₓ
⍝⍝: -	2	-	instructions macos 5.907 linux 3.716	allocated macos 2.181 linux 2.181

iv×0.5
⍝⍝: -	3	-	instructions macos 8 linux 4.473	allocated macos 12.177 linux 12.177

v×iv
⍝⍝: -	4.5	-	instructions macos 7.848 linux 4.474	allocated macos 12.159 linux 12.159

⌈/iv
⍝⍝: -	0.63	-	instructions macos 6.803 linux 4.054	allocated macos 0.349 linux 0.349

((≢v)÷2)↑v
⍝⍝: -	1.2	-	instructions macos 5.99 linux 3.326	allocated macos 4.218 linux 4.218

((≢v)÷2)↓v
⍝⍝: -	1.1	-	instructions macos 5.99 linux 3.311	allocated macos 4.218 linux 4.218

(⍴m)⍴v
⍝⍝: -	0.5	-	instructions macos 3.42 linux 1.903	allocated macos 0.149 linux 0.149

b⊂iv
⍝⍝: -	310	-	instructions macos 628.117 linux 510.434	allocated macos 58.752 linux 58.752

iv⍳i
⍝⍝: -	14	-	instructions macos 32.791 linux 32.512	allocated macos 10.063 linux 10.063

i∊iv
⍝⍝: -	15	-	instructions macos 33.054 linux 32.761	allocated macos 9.063 linux 9.063

≠i
⍝⍝: -	9.5	-	instructions macos 23.13 linux 21.663	allocated macos 5.062 linux 5.062

∪i
⍝⍝: -	15	-	instructions macos 34.312 linux 33.828	allocated macos 6.343 linux 6.343

⍕v
⍝⍝: -	750	-	instructions macos 1625.051 linux 1762.745	allocated macos 149.162 linux 149.162

⍕q
⍝⍝: -	780	-	instructions macos 1624.526 linux 1752.256	allocated macos 107.537 linux 107.537

{⍵+1}¨iv
⍝⍝: -	3.4	-	instructions macos 10 linux 6.152	allocated macos 12.29 linux 12.29

⍝ A loop of single-item updates, linear in its count.
x←0×iv ⋄ {x.[⍵]←1}¨iv
⍝⍝: -	13000	-	instructions macos 25981.534 linux 21388.039	allocated macos 1711.391 linux 1711.391

⍝ A loop of appends, linear in its count.
r←⍬ ⋄ {r,←⍵}¨iv
⍝⍝: -	5500	-	instructions macos 10849.049 linux 10683.663	allocated macos 153.548 linux 153.548

⍝ A gradient through a split into rows that a mix puts back together, where each row reads a few items of one record and of one shared cotangent. Batching declines the split's rows under `∂` ("an argument that isn't compact": they're carried cells, `batch.rs`), and the per-item fallback stops the trace ("a function called on each item", `program/trace.rs`), so each row runs on its own. Its count for each item shows that cost until batching takes carried cells. It records nodes and values for each row.
{y←{⍵×⍵}¨↓⍵ ⋄ z←⊃₀y ⋄ +/,z}∂ pairs
⍝⍝: -	2000	-	instructions macos 4453.965 linux 3790.572	allocated macos 738.229 linux 738.261
