⍝⍝ setup
side←256 size 32
n←side×side
v←¿n⍴0
w←¿n⍴0
m←[side side]⍴v
rka←•literal⁻¨⍳n
rkb←•literal⁻¨⍳n
ra←rka:v
rb←rkb:w
f←"svd"•decompose m
h←"eigen"•decompose m+⍉m
⍝⍝ copies
⍝ Arithmetic on two keyed vectors whose keys are equal but were built separately, as the same 65,536 texts made twice.
⍝ Expected: 1.5 copies: as `v+w`, once the call knows that the two key vectors are equal.
⍝ On 30 October 2026 `ra+2×ra`, whose operands share one key object, cost 1.62 on macOS.
⍝ Fresh calls compare ordered key labels. Pending execution retains the resulting maps; key unions are not cached.
ra+rb
⍝⍝: 3	25	33	instructions macos 58.717 linux 42.47	allocated macos 8.096 linux 8.096

⍝ `"svd"•decompose⁻ f` costs a 256×256 product plus four passes. On CI it measured 90.5 when `m+.×m` measured 79.6. Its expected cost used to take the product from rustymath's `linalg::PlusTimes`, 21 copies on Linux, a figure that basedpl's single-threaded product doesn't reach.
"svd"•decompose⁻ f
⍝⍝: 26	85	160	instructions macos 35.999 linux 106.351	allocated macos 32.058 linux 48.308

⍝⍝⍝ held macos 76.5: about a third of its macOS time is our own code, building vectors and rustymath's LU (`profile`, 29 October 2026)
"eigen"•decompose⁻ h
⍝⍝: 81	240	390	instructions macos 138.5 linux 344.501	allocated macos 75.37 linux 130.776

