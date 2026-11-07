⍝⍝ setup
⍝ Cases whose counts check that their costs grow linearly and that they allocate what their results need, with no time budgets. A cost that stops growing linearly counts far more for each item at 65,536 items: a quadratic one thousands of times as much. Pervasive and structural functions write their result once. A reshape that keeps the number of items shares its argument's storage. Indexing builds a table of its positions. Search builds a table of the items it searches. Format builds each number's text once, then converts it to characters. `arrays.bpl` counts the other functions of this kind.
n←65536 size 512
v←¿n⍴0 ⋄ w←¿n⍴0 ⋄ iv←⍳nₓ ⋄ b←0=3|iv ⋄ i←¿n⍴nₓ ⋄ m←[n÷2 2]⍴v ⋄ q←iv÷7
rows←1024 size 8 ⋄ pairs←[rows 2]⍴v
⍝⍝ growth
*v
⍝⍝: -	13	-	instructions linux 12.258	allocated linux 8.119

iv+1ₓ
⍝⍝: -	2	-	instructions linux 3.716	allocated linux 2.181

iv×0.5
⍝⍝: -	3	-	instructions linux 4.473	allocated linux 12.177

v×iv
⍝⍝: -	4.5	-	instructions linux 4.474	allocated linux 12.159

⌈/iv
⍝⍝: -	0.63	-	instructions linux 4.054	allocated linux 0.349

((≢v)÷2)↑v
⍝⍝: -	1.2	-	instructions linux 3.326	allocated linux 4.218

((≢v)÷2)↓v
⍝⍝: -	1.1	-	instructions linux 3.311	allocated linux 4.218

(⍴m)⍴v
⍝⍝: -	0.5	-	instructions linux 1.903	allocated linux 0.149

b⊂iv
⍝⍝: -	310	-	instructions linux 510.434	allocated linux 58.752

iv⍳i
⍝⍝: -	14	-	instructions linux 32.512	allocated linux 10.063

i∊iv
⍝⍝: -	15	-	instructions linux 32.761	allocated linux 9.063

≠i
⍝⍝: -	9.5	-	instructions linux 21.663	allocated linux 5.062

∪i
⍝⍝: -	15	-	instructions linux 33.828	allocated linux 6.343

⍕v
⍝⍝: -	750	-	instructions linux 1762.745	allocated linux 149.162

⍕q
⍝⍝: -	780	-	instructions linux 1752.256	allocated linux 107.537

{⍵+1}¨iv
⍝⍝: -	3.4	-	instructions linux 6.152	allocated linux 12.29

⍝ A loop of single-item updates, linear in its count.
x←0×iv ⋄ {x.[⍵]←1}¨iv
⍝⍝: -	13000	-	instructions linux 21388.039	allocated linux 1711.391

⍝ A loop of appends, linear in its count.
r←⍬ ⋄ {r,←⍵}¨iv
⍝⍝: -	5500	-	instructions linux 10683.663	allocated linux 153.548

⍝ A gradient through a split into rows that a mix puts back together, where each row reads a few items of one record and of one shared cotangent. Batching declines the split's rows under `∂` ("an argument that isn't compact": they're carried cells, `batch.rs`), and the per-item fallback stops the trace ("a function called on each item", `program/trace.rs`), so each row runs on its own. Its count for each item shows that cost until batching takes carried cells. It records nodes and values for each row.
{y←{⍵×⍵}¨↓⍵ ⋄ z←⊃₀y ⋄ +/,z}∂ pairs
⍝⍝: -	2000	-	instructions linux 3790.572	allocated linux 738.261
