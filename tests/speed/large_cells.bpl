⍝⍝ setup
width←20000 ◇ n←8×width ◇ cells←{¿ωρ0}¨width+8ρ0 1 ◇ keys←⌊(ιnₓ)÷width ◇ values←¿nρ0 ◇ u←nρ0.5 ◇ w←u+1
⍝⍝ reading
⍝ Large cells already amortize the operand call. Packing adds a full copy before the sums; Key also classifies and gathers its rows.
+/{+/ω}¨cells
⍝⍝: 2	3	-	instructions macos 2.96	allocated macos 0.1
keys{+/ω}⌸values
⍝⍝: 16	24	-	instructions macos 60.514	allocated macos 12.094
⍝ Joined items are read, including a source whose arithmetic is pending. Catenate and Mix each copy two source arrays before folding.
+/u,w
⍝⍝: 4	6	-	instructions macos 5.21	allocated macos 16.021
+/,⊃[u ; w]
⍝⍝: 5	8	-	instructions macos 5.745	allocated macos 24.026
