⍝⍝ setup
n←64 size 8 ◇ width←65536
cells←nρ[ιwidth;ιwidth+1]
⍝⍝ metadata
⍝ Each reads only each cell's length. Normalize by cells, not their items: packing the large cells eagerly is unnecessary work.
{≢ω}¨cells
⍝⍝: -	-	-	instructions macos 8112.062 linux 4427.281	allocated macos 524
