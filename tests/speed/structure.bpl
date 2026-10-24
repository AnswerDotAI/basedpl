⍝⍝ setup
sv←¿1048576⍴0
n←≢sv
sm←[n÷2;2]⍴sv
sq←[1024 1024]⍴sv
cimg←[256 256 16]⍴sv
ck←¿[144 32]⍴0
vq←1024↑sv
kq←¿[9 1]⍴0
s_4←[n÷4;4]⍴sv
w_4←s_4+0
p_2←[n÷4;2]⍴sv
ism←[n÷2;2]⍴⍳256ₓ
bp_2←[n÷4;2]⍴0.5<sv
f_2←32•float p_2
⍝⍝ structure
⍝ A structural glyph on a million floats gives a pending copy, which the call around it reads through, with no pass of its own over memory. A forward run is read in place. A reversed one is copied into a buffer for each block.
⍝ Expected: `2×⌽sv+1` 1.25 copies: one fused pass, with an 8-byte read and write an item, and a block buffer for the reversal; `2×1⌽sv+1` and `2×(10↓sv)+1` 1 copy: one fused pass, with an 8-byte read and write an item; `+/(⌽sv)×sv` 1 copy: two 8-byte reads of `sv` an item, folded.
⍝ Held until studied: `2×⌽sv+1` has old Linux and wasm budgets that suggest about 6 and 3.65, more than twice its expected cost. On 22 October 2026 CI measured 2.81 on Linux and 4.75 in wasm. Its Linux and wasm budgets stay at the old value.
2×⌽sv+1
⍝⍝: 2.5	12	7.3

⍝ Held until studied: `2×1⌽sv+1` has old Linux and wasm budgets that suggest about 2.2 and 2.25, more than twice its expected cost. On 22 October 2026 CI measured 3.72 on Linux and 1.52 in wasm. Its Linux and wasm budgets stay at the old value.
2×1⌽sv+1
⍝⍝: 2	4.4	4.5

⍝ Held until studied: `2×(10↓sv)+1` has old Linux and wasm budgets that suggest about 3.8 and 2.4, more than twice its expected cost. On 22 October 2026 CI measured 1.9 on Linux and 1.53 in wasm. Its Linux and wasm budgets stay at the old value.
2×(10↓sv)+1
⍝⍝: 2	7.6	4.8

⍝ Held until studied: `+/(⌽sv)×sv` measures 2.74 on macOS and has old Linux and wasm budgets that suggest about 2.95 and 4.4, more than twice its expected cost. On 22 October 2026 CI measured 2.62 on Linux and 3.49 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
+/(⌽sv)×sv
⍝⍝: 5.8	5.9	8.8

⍝ A reversed read moves only under a call that reads one array. `sv×sv+0.5` reads two, so it runs forward first, and `2×` then reads its result backwards.
⍝ Expected: 2 copies: two passes, forward then reversed.
⍝ Held until studied: `2×⌽sv×sv+0.5` has old Linux and wasm budgets that suggest about 6.5 and 9, more than twice its expected cost. On 22 October 2026 CI measured 7 on Linux and 7.63 in wasm. Its Linux and wasm budgets stay at the old value.
⍝⍝ allows stored "read out of order": `sv×sv+0.5` reads two arrays, so it runs forward first and `2×` reads its stored result backwards
2×⌽sv×sv+0.5
⍝⍝: 4	13	18

⍝ A rotate of an overtake that carries only fill across the end composes with it into one padded read of the source (`Steps::read_through`).
⍝ Expected: 1 copy: one pass that reads `sv` through one padded map, with an 8-byte read and write an item.
2×¯1⌽(n+2)↑sv
⍝⍝: 2.5	2	2.5

⍝ Floor returns exact integers, so this reads a pending copy through an ordinary kernel rather than fusion.
⍝ Expected: 3.25 copies: a reversed copy of each block into a buffer, about 0.5 copies, then a tolerant floor of about 11 vector operations an item, with 64-bit writes. It was 2 copies, from the old count for a tolerant floor.
⌊⌽sv
⍝⍝: 6.5	6.5	6.5

⍝ Stride-two columns gather through the shared reader before their contiguous subtraction.
⍝ Expected: 1.25 copies: two strided reads of `sm`'s lines and a write of half as many items.
([: 1]⌷sm)-([: 0]⌷sm)
⍝⍝: 2.5	2.5	2.5

⍝ A join along the last axis writes each row whole, a lane from each argument in turn, where the lanes are short. Lanes of one length up to 8 move as blocks. `sm` has lanes of 2.
⍝ Expected: 2 copies: writes twice as many items as `sm` holds.
⍝ `sm,sm` measures 1.4 on macOS since a join writes each row whole, a lane from each argument in turn, straight into the result (`gather::interleave`, 22 October 2026), down from 4.3 to 4.6, which a fill and a strided pass for each argument cost. Held until studied: in wasm it measured 1.3 to 4.5 in seven CI runs from 22 October 2026, more than twice its expected cost in some runs. On 22 October 2026 CI measured 4.59 on Linux, more than twice its expected cost, and 3.37 in wasm. Its Linux and wasm budgets stay at the old values.
sm,sm
⍝⍝: 4	11	24

⍝ A join of 8-bit integers to floats converts the integers in one pass first, then joins as `sm,sm` does. `ism` holds exact positions, which take a byte each.
⍝ Expected: 2.6 copies: a pass that converts `ism`'s bytes into floats, about 0.6 copies with 1-byte reads and 8-byte writes, then the join's 2 copies, as for `sm,sm`.
⍝ `ism,sm` measures 2.7 on macOS since a join writes each row whole (`gather::interleave`, 22 October 2026), down from 4.2 to 4.4. The conversion pass runs one item at a time, about 3 instructions an item. Held until studied: in wasm it measured 2.6 to 7.8 in seven CI runs from 22 October 2026, more than twice its expected cost in some runs. On 22 October 2026 CI measured 8.62 on Linux, more than three times its expected cost, and 5.87 in wasm. Its Linux and wasm budgets stay at the old values.
ism,sm
⍝⍝: 5.2	12	26

⍝ Two of the four columns of `s_4`, picked by position, read as a stride. The copy walks each row's two items in one loop of fixed width. A list of positions with uneven steps, and a rotation of each row, take the same loop through a table of offsets.
⍝ Expected: `⌊[:;0 1]⌷s_4` 2.1 copies: the strided gather, about 0.75 copies, as `[:;0 1]⌷s_4` alone costs, then a tolerant floor of about 11 vector operations an item on half the items, with 64-bit writes; `⌊[:;0 1 3]⌷s_4` 2.9 copies: a gather of three quarters of the items, about 0.875 copies, and the floor on them; `⌊1⌽s_4` 3.75 copies: a gather of every item through a table of offsets, about 1 copy, and the floor on all of them. They were 1.5, 1.7 and 2.5 copies, from narrow writes and the old count for a tolerant floor.
⌊[:;0 1]⌷s_4
⍝⍝: 4.2	4.2	4.2

⌊[:;0 1 3]⌷s_4
⍝⍝: 5.8	5.8	5.8

⌊1⌽s_4
⍝⍝: 7.5	7.5	7.5

⍝ A write of two columns goes through the same loop. It overwrites `w_4` in place.
⍝ Expected: 1 copy: each 64-byte line of `w_4` holds two rows, and the write replaces half of each row, so it reads and writes every line of `w_4` once (`bw_`, 21 October 2026).
⍝ The wasm budget stays at its old value, which suggests about 2.
[:;0 1]⌷w_4 ←p_2
⍝⍝: 2	2	4

⍝ Booleans written into floats would leave the array mixed, as a literal of the same items would, so the case converts them first, as the style page says. `1×bp_2` computes the floats, and the write copies them into two columns.
⍝ Expected: 1.5 copies. Computing `1×bp_2` reads 1 byte and writes 8 bytes for each of half the items, about 0.5. Each 64-byte cache line of `w_4` holds two rows whose first two items the write replaces, so the write reads and writes every line of `w_4` once, about 1 copy.
⍝ In wasm it measured 1.64 to 2.12 in seven CI runs from 22 October 2026.
[:;0 1]⌷w_4 ←1×bp_2
⍝⍝: 3	3	3

⍝ A write of two columns of 32-bit floats into 64-bit floats converts each item as the same loop writes it, and the array stays 64-bit floats.
⍝ Expected: 1 copy. Each 64-byte cache line of `w_4` holds two rows whose first two items the write replaces, so the write reads and writes every line of `w_4` once. Reading 4 bytes of `f_2` for each of half the items adds little beside that.
⍝ Held until studied: the wasm budget follows `[:;0 1]⌷w_4 ←p_2`'s old one for the same loop, which suggests wasm costs about twice native. In wasm it measured 1.33 to 2.69 in seven CI runs from 22 October 2026, where twice its expected cost, 2, would fail by chance.
[:;0 1]⌷w_4 ←f_2
⍝⍝: 2	2	4

⍝ A fold by a reduction reads a pending copy through its map. A copy that reads a contiguous block of its source with its axes in another order, such as a transpose, a reversal, or 2 by 2 windows that don't overlap, folds that block in its own order, at the cost of folding the source. Totals that come out in another order stay a pending copy, which the next fold reads the same way.
⍝ Expected: 0.5 copies each: reads 8 bytes an item and writes nothing.
⍝ Wasm on CI's x86 runner: 1 copy each, twice native, as `+/v`'s comment gives for wasm's fold. In seven CI runs from 22 October 2026, `+/⍉sq` and `+⌿⊖sq` measured 0.69 to 1.38 copies and `+/⌽sv` 1.13 to 1.43. `+/⌽sv` runs `+/v`'s f64 sum, which since 22 October costs about 1.4 times what it did before on CI's V8, though not on this Mac's. The sum's wasm loop changed that day: under thin LTO, LLVM runtime-unrolls the strict sum's 16-result loop twice. Turning that unrolling off in browser builds didn't change the sum's cost on CI's V8. The slowdown's cause is not known yet.
+/⍉sq
⍝⍝: 1	1	2

+⌿⊖sq
⍝⍝: 1	1	2

+/⌽sv
⍝⍝: 1	1	2

⍝ Held until studied: `⌈/⌈/[[2 2]⋄[2 2]]↕sq` measures 1.7 on macOS and has an old Linux budget that suggests about 2.3, more than twice its expected cost. On 23 October 2026 CI measured 1.64 on Linux. Its macOS and Linux budgets stay at the old value.
⌈/⌈/[[2 2]⋄[2 2]]↕sq
⍝⍝: 3.4	4.6	2

⍝ Held until studied: `+/⍉cimg` measures 1.08 on macOS and has an old Linux budget that suggests about 1.05, more than twice its expected cost. On 22 October 2026 CI measured 0.97 on Linux. Its macOS and Linux budgets stay at the old value.
+/⍉cimg
⍝⍝: 2.2	2.1	1

⍝ A fold of windows slides over its source and never builds the windows. Windows of up to 8 rows fold directly. Wider windows take van Herk-Gil-Werman, at a cost that doesn't grow with the width. Windows of a matrix fold in two separable passes.
⍝ Expected: `+/3↕sv` and `⌈/3↕sv` 1 copy: one read and one write per window, sliding; `+/50↕sv` 1 copy: a running sum, with one addition and one subtraction per window; `⌈/50↕sv` 2 copies: van Herk-Gil-Werman, with two passes of running maxima and a combine; `+/+/3 3↕sq` 2 copies: two separable sliding passes.
⍝ Held until studied: `+/3↕sv` measures 3.74 on macOS and has old Linux and wasm budgets that suggest about 5.5 and 4.2, more than twice its expected cost. On 22 October 2026 CI measured 5.09 on Linux and 3.41 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
+/3↕sv
⍝⍝: 7.7	11	8.4

⍝ Held until studied: `⌈/3↕sv` measures 4.01 on macOS and has old Linux and wasm budgets that suggest about 7.5 and 4.55, more than twice its expected cost. On 23 October 2026 CI measured 5.85 on Linux and 2.6 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⌈/3↕sv
⍝⍝: 8.3	15	9.1

⍝ Held until studied: `+/50↕sv` measures 6.97 on macOS and has old Linux and wasm budgets that suggest about 24.5 and 14, more than twice its expected cost. On 22 October 2026 CI measured 9.32 on Linux and 6.43 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
+/50↕sv
⍝⍝: 15	49	28

⍝ Held until studied: `⌈/50↕sv` measures 7.15 on macOS and has old Linux and wasm budgets that suggest about 26 and 16, more than twice its expected cost. On 23 October 2026 CI measured 8.86 on Linux and 14.2 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⌈/50↕sv
⍝⍝: 14	52	32

⍝ Held until studied: `+/+/3 3↕sq` measures 6.91 on macOS and has old Linux and wasm budgets that suggest about 6.5 and 7.5, more than twice its expected cost. On 22 October 2026 CI measured 10.7 on Linux and 7.65 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
+/+/3 3↕sq
⍝⍝: 14	13	15

⍝ Padded windows of a vector slide over one padded copy of their source.
⍝ Expected: 2 copies each: one padded copy, then one sliding pass.
⍝ Held until studied: `+/¯3↕sv` measures 4.33 on macOS and has old Linux and wasm budgets that suggest about 9 and 4.7, more than twice its expected cost. On 22 October 2026 CI measured 7.97 on Linux and 6.61 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍝⍝ allows forced "+/": padded windows slide over one padded copy of their source
+/¯3↕sv
⍝⍝: 9.3	18	9.4

⍝ Held until studied: `+/["size":3 "edge":"wrap"]↕sv` measures 4.92 on macOS and has old Linux and wasm budgets that suggest about 11 and 11, more than twice its expected cost. On 22 October 2026 CI measured 7.24 on Linux and 6.54 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍝⍝ allows forced "+/": padded windows slide over one padded copy of their source
+/["size":3 "edge":"wrap"]↕sv
⍝⍝: 11	22	22

⍝ A copy's gradient reads its cotangent: a reversal's through the inverse map, windows' through a correlation that reads the cotangent's frames at each window position, and a sum of windows' through a sliding sum of the padded cotangent.
⍝ Expected: `{+/sv×⌽⍵}∂ sv` 2 copies: the primal's pass and a reversed read of the cotangent; `{+/,(3↕sv)×3↕⍵}∂ sv` 6 copies: the primal and a correlation of the cotangent, each a few sliding passes; `{+/,+/50↕⍵}∂ sv` 3 copies: a sliding sum of the padded cotangent.
⍝ Held until studied: `{+/sv×⌽⍵}∂ sv` measures 5.49 on macOS and has old Linux and wasm budgets that suggest about 7 and 7.5, more than twice its expected cost. On 22 October 2026 CI measured 8.97 on Linux and 8.72 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{+/sv×⌽⍵}∂ sv
⍝⍝: 9.8	14	15

⍝ Held until studied: `{+/,(3↕sv)×3↕⍵}∂ sv` measures 28.85 on macOS and has old Linux and wasm budgets that suggest about 85 and 60, more than twice its expected cost. On 22 October 2026 CI measured 95.6 on Linux and 54.7 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍝⍝ gap stored "read out of order": the gradient should slide over `sv`, rather than store its 3,145,722 window items
⍝⍝ gap forced "+/": `+/` under `∂` should fold its pending argument, rather than compute its 1,048,576 items first
{+/,(3↕sv)×3↕⍵}∂ sv
⍝⍝: 63	170	120

⍝ Held until studied: `{+/,+/50↕⍵}∂ sv` measures 16.22 on macOS and has old Linux and wasm budgets that suggest about 42 and 29.5, more than twice its expected cost. On 22 October 2026 CI measured 26.6 on Linux and 17.2 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍝⍝ gap forced "+/": `+/` under `∂` should fold its pending arguments, as the objective's sliding fold does, rather than compute 2,097,152 items first
{+/,+/50↕⍵}∂ sv
⍝⍝: 34	84	59

⍝ A product of windows reads them a block of rows at a time, straight into the matrix product, and never builds the windows matrix. A product of a single-channel image's windows with one filter adds each column of the windows, read where it lies, into the result. Its gradient for the kernel reads them again in blocks. Its gradient for the image is one product of the windows of the zero-padded cotangent with the kernel reversed.
⍝ Expected: `([64516 144]⍴3 3↕cimg)+.×ck` 28 copies on macOS, 120 on Linux and 240 in wasm: a product of 595 million flops, 567 an item, which on macOS takes 19 at Accelerate's rank-k rate, and gathering the windows a block of rows at a time, about 9; `([1044484 9]⍴3 3↕sq)+.×kq` 2.5 copies: nine multiply-adds per window, read in place; `{+/,([64516 144]⍴3 3↕cimg)+.×⍵}∂ ck` and `{+/,([64516 144]⍴3 3↕⍵)+.×ck}∂ cimg` 56 copies on macOS, 240 on Linux and 480 in wasm: two products of 595 million flops, with their gathers.
([64516 144]⍴3 3↕cimg)+.×ck
⍝⍝: 56	240	480

⍝ Held until studied: `([1044484 9]⍴3 3↕sq)+.×kq` measures 5.54 on macOS, down from 7.49 on 22 October 2026 when rustymath's `by_columns` kept a strip of 8 results in registers while all nine columns add into it. Nine multiply-adds per window at NEON's two lanes of f64 take about twice the expected cost. Its old Linux and wasm budgets suggest about 8.5 and 10.5. On 22 October 2026 CI measured 5.36 on Linux and 5.85 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
([1044484 9]⍴3 3↕sq)+.×kq
⍝⍝: 16	17	21

{+/,([64516 144]⍴3 3↕cimg)+.×⍵}∂ ck
⍝⍝: 120	480	960

⍝⍝ gap forced "+.×": the image's gradient computes the pending cotangent's 2,064,512 items, then its zero-padded copy's 2,130,048. The padded copy should read the cotangent as it pads
{+/,([64516 144]⍴3 3↕⍵)+.×ck}∂ cimg
⍝⍝: 120	480	960

⍝ A transpose of a million floats is a pending copy. A product reads it where it lies, as a transposed matrix. A fused call reads it in blocks of whole rows, as many as a cache line holds items.
⍝ Expected: `(⍉sq)+.×vq` 0.5 copies: reads the matrix once; `2×⍉sq` 1.25 copies: one read and one write per item, through a blocked transpose.
⍝ Held until studied: `(⍉sq)+.×vq` has old Linux and wasm budgets that suggest about 4.35 and 5.5, more than twice its expected cost. On 22 October 2026 CI measured 5.41 on Linux and 5.39 in wasm. Its Linux and wasm budgets stay at the old value.
(⍉sq)+.×vq
⍝⍝: 1	8.7	11

⍝ Held until studied: `2×⍉sq` measures 3.43 on macOS and has old Linux and wasm budgets that suggest about 6 and 5.5, more than twice its expected cost. On 22 October 2026 CI measured 4.18 on Linux and 2.76 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
2×⍉sq
⍝⍝: 6.8	12	11

⍝ A select by a comparison runs as one graph with the comparison and the cases: `0.5<sv` stays a lane mask, and `-` runs on every item.
⍝ Expected: 1.25 copies: one fused pass with a comparison, a negation and a blend.
⍝ Held until studied: `(0.5<)⍚[⊢ -] sv` measures 3.06 on macOS and has old Linux and wasm budgets that suggest about 3.55 and 9, more than twice its expected cost. On 22 October 2026 CI measured 3.28 on Linux and 3.16 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
(0.5<)⍚[⊢ -] sv
⍝⍝: 6.6	7.1	18
