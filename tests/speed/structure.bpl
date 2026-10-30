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
images←["b":8 "h":224 "w":224 "c":3]⍴¿1204224⍴0
split←{["b" "patch":["i" "j"] "pixel":["y" "x" "c"]]⍉⁻¹["b":0 "h":["i":1 ∞;"y":3 16] "w":["j":2 ∞;"x":4 16] "c":5]⍉⍵}
rows←["b":0 "h":["i":1 ∞;"y":2 16] "w":3 "c":4]⍉
⍝⍝ structure
⍝ A structural glyph on a million floats gives a pending copy, which the call around it reads through, with no pass of its own over memory. A forward run is read in place. A reversed one is copied into a buffer for each block.
⍝ Expected: `2×⌽sv+1` 1.25 copies: one fused pass, with an 8-byte read and write an item, and a block buffer for the reversal; `2×1⌽sv+1` and `2× 10↓sv + 1` 1 copy: one fused pass, with an 8-byte read and write an item; `+/ ⌽sv × sv` 1 copy: two 8-byte reads of `sv` an item, folded.
⍝ CI measured 1.28 to 1.66 on Linux and 2.78 to 3.19 in wasm in most runs from 56 to 62.
⍝⍝⍝ held wasm 5: the cause isn't known yet; busy hosts reach 5
⍝⍝⍝ busy linux 2.7: its data passes CI's L2 (open.md, "Footprint copies")
2×⌽sv+1
⍝⍝: 2.5	2.5	2.5

⍝ CI measured 1.09 to 1.58 on Linux in most runs from 56 to 62.
⍝⍝⍝ held wasm 2.07: the cause isn't known yet
⍝⍝⍝ busy linux 2.58: its data passes CI's L2 (open.md, "Footprint copies")
2×1⌽sv+1
⍝⍝: 2	2	2

⍝⍝⍝ held wasm 4.32: the cause isn't known yet
⍝⍝⍝ route fusion
2× 10↓sv + 1
⍝⍝: 2	2	2

⍝⍝⍝ held linux 2.31 wasm 3.31: the cause isn't known yet
+/ ⌽sv × sv
⍝⍝: 2	2	2

⍝ A reversed read moves only under a call that reads one array. `sv×sv+0.5` reads two, so it runs forward first, and `2×` then reads its result backwards.
⍝ Expected: 2 copies: two passes, forward then reversed.
⍝ CI measured 2.12 to 2.32 on Linux and 3.89 to 4.34 in wasm in most runs from 56 to 62.
⍝⍝⍝ held wasm 9.95: the cause isn't known yet; busy hosts reach 9.95
⍝⍝⍝ busy linux 6.43: its data passes CI's L2 (open.md, "Footprint copies")
2×⌽sv×sv+0.5
⍝⍝: 4	4	4

⍝ A rotate of an overtake that carries only fill across the end composes with it into one padded read of the source (`Steps::read_through`).
⍝ Expected: 1 copy: one pass that reads `sv` through one padded map, with an 8-byte read and write an item.
⍝ On CI's Linux it measured 0.92 copies in v0.1.73, then 1.74 and 1.46 in v0.1.74 and v0.1.75. This Mac retires the same instructions on builds from before and after. The cause is inferred, because this Mac can't run x86-64. The read runs one item behind the write. When glibc maps both 8 MB arrays at the same offset within a page, each load's address matches the low 12 bits of the store one item before it, and Zen 4 stalls on that match (4K aliasing). The allocations and frees before the case decide whether the arrays share an offset. The speed suite's move in v0.1.74 changed them.
⍝⍝⍝ held wasm ~1.25: the cause isn't known; the estimate keeps the wasm budget it had before 30 October 2026
2×¯1⌽(n+2)↑sv
⍝⍝: 2	2	2

⍝ Floor returns exact integers, so this reads a pending copy through an ordinary kernel rather than fusion.
⍝ Expected: 3.25 copies: a reversed copy of each block into a buffer, about 0.5 copies, then a tolerant floor of about 11 vector operations an item, with 64-bit writes. It was 2 copies, from the old count for a tolerant floor.
⍝ CI measured 4.75 to 5.07 in wasm in runs 56-61.
⍝⍝⍝ busy wasm 6.41: its data passes CI's L2 (open.md, "Footprint copies")
⍝⍝⍝ held linux 3.575: margin against noise for a case within 10% of its budget (30 October 2026)
⌊⌽sv
⍝⍝: 6.5	6.5	6.5

⍝ Stride-two columns gather through the shared reader before their contiguous subtraction.
⍝ Expected: 1.25 copies: two strided reads of `sm`'s lines and a write of half as many items.
[: 1]⌷sm - [: 0]⌷sm
⍝⍝: 2.5	2.5	2.5

⍝ A join along the last axis writes each row whole, a lane from each argument in turn, where the lanes are short. Lanes of one length up to 8 move as blocks. `sm` has lanes of 2.
⍝ Expected: 2 copies: writes twice as many items as `sm` holds.
⍝ `sm,sm` measures 1.4 on macOS since a join writes each row whole, a lane from each argument in turn, straight into the result (`gather::interleave`, 22 October 2026), down from 4.3 to 4.6, which a fill and a strided pass for each argument cost. CI measured 1.31 to 1.42 on Linux in most runs from 56 to 62.
⍝⍝⍝ busy linux 5.14: its data passes CI's L2 (open.md, "Footprint copies")
sm,sm
⍝⍝: 4	4	4

⍝ A join of 8-bit integers to floats converts the integers in one pass first, then joins as `sm,sm` does. `ism` holds exact positions, which take a byte each.
⍝ Expected: 2.6 copies: a pass that converts `ism`'s bytes into floats, about 0.6 copies with 1-byte reads and 8-byte writes, then the join's 2 copies, as for `sm,sm`.
⍝ `ism,sm` measures 2.9 on macOS since a join writes each row whole (`gather::interleave`, 22 October 2026), down from 4.2 to 4.4. The conversion pass runs one item at a time, about 3 instructions an item. CI measured 2.23 to 2.44 on Linux and 3.28 to 3.72 in wasm in most runs from 56 to 62.
⍝⍝⍝ busy linux 7.69 wasm 6.66: its data passes CI's L2 (open.md, "Footprint copies")
ism,sm
⍝⍝: 5.2	5.2	5.2

⍝ Two of the four columns of `s_4`, picked by position, read as a stride. The copy walks each row's two items in one loop of fixed width. A list of positions with uneven steps, and a rotation of each row, take the same loop through a table of offsets.
⍝ Expected: `⌊[:;0 1]⌷s_4` 2.1 copies: the strided gather, about 0.75 copies, as `[:;0 1]⌷s_4` alone costs, then a tolerant floor of about 11 vector operations an item on half the items, with 64-bit writes; `⌊[:;0 1 3]⌷s_4` 2.9 copies: a gather of three quarters of the items, about 0.875 copies, and the floor on them; `⌊1⌽s_4` 3.75 copies: a gather of every item through a table of offsets, about 1 copy, and the floor on all of them. They were 1.5, 1.7 and 2.5 copies, from narrow writes and the old count for a tolerant floor.
⌊[:;0 1]⌷s_4
⍝⍝: 4.2	4.2	4.2

⍝⍝⍝ held wasm 3.19: margin against noise for a case within 10% of its budget (30 October 2026)
⌊[:;0 1 3]⌷s_4
⍝⍝: 5.8	5.8	5.8

⍝ CI measured 4.28 to 4.54 on Linux in runs 56 and 59-61.
⍝⍝⍝ busy linux 7.49: its data passes CI's L2 (open.md, "Footprint copies")
⌊1⌽s_4
⍝⍝: 7.5	7.5	7.5

⍝ A write of two columns goes through the same loop. It overwrites `w_4` in place.
⍝ Expected: 1 copy: each 64-byte line of `w_4` holds two rows, and the write replaces half of each row, so it reads and writes every line of `w_4` once.
[:;0 1]⌷w_4 ←p_2
⍝⍝: 2	2	2

⍝ Booleans written into floats would leave the array mixed, as a literal of the same items would, so the case converts them first, as the style page says. `1×bp_2` computes the floats, and the write copies them into two columns.
⍝ Expected: 1.5 copies. Computing `1×bp_2` reads 1 byte and writes 8 bytes for each of half the items, about 0.5. Each 64-byte cache line of `w_4` holds two rows whose first two items the write replaces, so the write reads and writes every line of `w_4` once, about 1 copy.
⍝ In wasm it measured 1.64 to 2.12 in seven CI runs from 22 October 2026.
[:;0 1]⌷w_4 ←1×bp_2
⍝⍝: 3	3	3

⍝ A write of two columns of 32-bit floats into 64-bit floats converts each item as the same loop writes it, and the array stays 64-bit floats.
⍝ Expected: 1 copy. Each 64-byte cache line of `w_4` holds two rows whose first two items the write replaces, so the write reads and writes every line of `w_4` once. Reading 4 bytes of `f_2` for each of half the items adds little beside that.
[:;0 1]⌷w_4 ←f_2
⍝⍝: 2	2	2

⍝ A fold by a reduction reads a pending copy through its map. A copy that reads a contiguous block of its source with its axes in another order, such as a transpose, a reversal, or 2 by 2 windows that don't overlap, folds that block in its own order, at the cost of folding the source. Totals that come out in another order stay a pending copy, which the next fold reads the same way.
⍝ Expected: 0.5 copies each: reads 8 bytes an item and writes nothing.
⍝ Wasm on CI's x86 runner: under Node 26's V8, these folds cost what they cost natively, 0.36 to 0.46 copies in the runs of 28 and 29 October 2026.
⍝⍝⍝ route mapped fold
+/⍉sq
⍝⍝: 1	1	1

+⌿⊖sq
⍝⍝: 1	1	1

+/⌽sv
⍝⍝: 1	1	1

⍝ A copy whose runs hold at least 128 items, rustymath's `reduce::RUN`, folds each run where it lies, with no copy. Half the columns of `sq` give runs of 512.
⍝ Expected: 0.5 copies: reads 8 bytes for each of half the items, and the call into the fold for each run costs about what reading its items costs.
⍝⍝⍝ route in place
+/,[:;⍳512ₓ]⌷sq
⍝⍝: 1	1	1

⍝ Expected: 0.5 copies.
⍝⍝⍝ held macos 1.56 linux 1.17 wasm 1.35: open.md lists wide and 2-D window folds among the speed audit's outliers
⌈/⌈/[[2 2]⋄[2 2]]↕sq
⍝⍝: 1	1	1

+/⍉cimg
⍝⍝: 1	1	1

⍝ A fold of windows slides over its source and never builds the windows. Windows of up to 8 rows fold directly, finishing each block of a lane while it is in cache. Wider windows take van Herk-Gil-Werman, at a cost that doesn't grow with the width. Windows of a matrix fold in two separable passes.
⍝ Expected: `+/3↕sv` and `⌈/3↕sv` 1 copy: one read and one write per window, sliding; `+/50↕sv` 1 copy: a running sum, with one addition and one subtraction per window; `⌈/50↕sv` 2 copies: van Herk-Gil-Werman, with two passes of running maxima and a combine; `+/+/3 3↕sq` 2 copies: two separable sliding passes.
⍝ On 25 October 2026 `+/3↕sv` measured 3.66 on macOS, for the reason `⌈/3↕sv`'s comment gives, and it measures 1.45 after the same change. CI then measured 0.85 on Linux and 1.26 in wasm, against 5.09 and 3.41 on 22 October 2026. All three budgets are twice its expected cost.
⍝⍝⍝ route sliding fold
+/3↕sv
⍝⍝: 2	2	2

⍝ On 25 October 2026 `⌈/3↕sv` measured 3.66 on macOS, because the direct folds took one pass over the whole lane for each row of a window, which reads and writes `sv`'s 8 MB three times. Direct folds now finish each block of a lane in cache, with a window's first three rows in one pass, and `⌈/3↕sv` measures 1.55 on macOS. CI then measured 0.86 on Linux and 1.33 in wasm, against 5.85 and 2.6 on 23 October 2026. All three budgets are twice its expected cost.
⌈/3↕sv
⍝⍝: 2	2	2

⍝ CI measured 5.15 to 5.36 on Linux and 4.23 to 4.41 in wasm in most runs from 56 to 62.
⍝⍝⍝ held macos 3.85 linux 14 wasm 18.8: open.md lists wide and 2-D window folds among the speed audit's outliers; busy hosts reach the Linux and wasm costs
+/50↕sv
⍝⍝: 2	2	2

⍝ CI measured 4.93 to 5.4 on Linux and 7.41 to 7.7 in wasm in most runs from 56 to 62.
⍝⍝⍝ held macos 5.29 linux 13.3 wasm 20.1: open.md lists wide and 2-D window folds among the speed audit's outliers; busy hosts reach the Linux and wasm costs
⌈/50↕sv
⍝⍝: 4	4	4

⍝ CI measured 2.18 to 2.54 on Linux and 2.3 to 2.6 in wasm in most runs from 56 to 62.
⍝⍝⍝ busy linux 5.83 wasm 5.46: its data passes CI's L2 (open.md, "Footprint copies")
+/+/3 3↕sq
⍝⍝: 4	4	4

⍝ Padded windows of a vector fold in a block stage that reads their source in place, through the padding of each edge mode.
⍝ Expected: about 1 copy each: one block stage reads the source once for each of a window's 3 positions, in cache, and writes the sums.
⍝ CI measured 1.82 to 2.55 on Linux and 2.46 to 2.72 in wasm in most runs from 56 to 62.
⍝ On CI's Linux the same code measured 7.97 copies on 22 October 2026, 4.77 on 23 October and in v0.1.73, then 6.98 and 6.75 in v0.1.74 and v0.1.75. The cause is inferred, as for `2×¯1⌽(n+2)↑sv`. The padded copy writes each item one place after where it reads it. When glibc maps the source and the copy at the same offset within a page, Zen 4 stalls on 4K aliasing.
⍝⍝⍝ held linux 3.69 wasm 5.41: the cause isn't placed, apart from the 4K aliasing above on Linux; busy hosts reach these costs
+/¯3↕sv
⍝⍝: 2	2	2

⍝⍝⍝ held macos 2.13 linux 1.93 wasm 5.43: open.md lists wide and 2-D window folds among the speed audit's outliers
+/["size":3 "edge":"wrap"]↕sv
⍝⍝: 2	2	2

⍝ A copy's gradient reads its cotangent: a reversal's through the inverse map, windows' through a correlation that reads the cotangent's frames at each window position, and a sum of windows' through a sliding sum of the padded cotangent.
⍝ Expected: `{+/sv×⌽⍵}∂ sv` 2 copies: the primal's pass and a reversed read of the cotangent; `{+/, 3↕sv × 3↕⍵}∂ sv` 6 copies: the primal and a correlation of the cotangent, each a few sliding passes; `{+/,+/50↕⍵}∂ sv` nothing: its gradient depends on neither `⍵` nor any other array, so its trace holds it as one constant; `{+/⊼+/50↕⍵}∂ sv` 5.5 copies: the primal's running sum, about 1, its squared sum, about 0.5, the cotangent of the window sums, about 1, and a sliding sum of the padded cotangent, about 3.
⍝ CI measured 1.84 to 1.96 on Linux and 2.05 to 2.3 in wasm in most runs from 56 to 62.
⍝⍝⍝ busy linux 4.89 wasm 3.86: its data passes CI's L2 (open.md, "Footprint copies")
⍝⍝⍝ route trace
{+/sv×⌽⍵}∂ sv
⍝⍝: 4	4	4

⍝ CI measured 13.2 to 15.8 on Linux and 12.3 to 13.9 in wasm in most runs from 56 to 62.
⍝⍝⍝ held macos 13.1 linux 43.4 wasm 31.4: the cause isn't known yet; busy hosts reach the Linux and wasm costs
{+/, 3↕sv × 3↕⍵}∂ sv
⍝⍝: 12	12	12

⍝ `{+/,+/50↕⍵}∂ sv` checks that a linear function's gradient becomes a constant. Its gradient depends on neither `⍵` nor any other array, so its trace holds it as one constant, `v0 ← held [1 2 … 50 50 …]`, and each call returns it, at about 0.004 copies on every platform. Its budgets are the 0.5 floor. A trace that computed the gradient on every call would cost several copies and fail them.
{+/,+/50↕⍵}∂ sv
⍝⍝: 0.5	0.5	0.5

⍝ On 29 October 2026 this case cost 10.2 on macOS, spelled then with `⊼`. Since 30 October, `²` under `∂` costs what `⊼` does. CI hasn't timed it. Its Linux and wasm estimates scale its macOS cost by `+/50↕sv`'s highest ratios to macOS in runs 56 to 62, 3.6 on Linux and 4.9 in wasm.
⍝⍝⍝ held macos 10.2 linux ~37.1 wasm ~49.8: its window sums, forward and transposed, run at the cost of `+/50↕sv`, which open.md lists among wide window folds
⍝⍝⍝ gap forced "+/": the transpose's sliding sum should read the padded cotangent in place, as a fold of padded windows does
⍝⍝⍝ route trace
{+/(+/50↕⍵)²}∂ sv
⍝⍝: 11	11	11

⍝ A product of windows reads them a block of rows at a time, straight into the matrix product, and never builds the windows matrix. A product of a single-channel image's windows with one filter adds each column of the windows, read where it lies, into the result. Its gradient for the kernel reads them again in blocks. Its gradient for the image is one product of the windows of the zero-padded cotangent with the kernel reversed.
⍝ Expected: `[64516 144]⍴3 3↕cimg +.× ck` 19 copies on macOS, 71 on Linux and 142 in wasm: a product of 595 million flops, 567 an item, at Accelerate's rank-k rate, at 8 in a copy's time for rustymath's own kernels and at 4 in wasm. Gathering the windows adds nothing: on 30 October 2026 the case cost 24.5 on macOS and 92 with rustymath's own kernels, against 24.8 and 92 for a product from the stored windows matrix. `{+/, [64516 144]⍴3 3↕cimg +.× ⍵}∂ ck` costs the same, because the function is linear in the kernel: `∂`'s program runs no primal, only one product, of the transposed windows with the broadcast seed. `{+/, [64516 144]⍴3 3↕⍵ +.× ck}∂ cimg` 27 copies on macOS, 106 on Linux and 212 in wasm, because the function is linear in the image: its program runs one product, of 604 million flops, between the windows of the zero-padded cotangent and the kernel reversed. That product has its own shape, 65,536 rows of 288 items times 16 columns: from a stored windows matrix it cost 27 on macOS and 106 with rustymath's own kernels on 30 October 2026, where the forward product's rate gives 19 and 72. Linux and wasm scale 72 and 144 by that 106 against 72.
⍝ CI measured 70 to 74 on Linux in runs 56 and 59-61.
⍝⍝⍝ busy linux 150: its data passes CI's L2 (open.md, "Footprint copies")
⍝⍝⍝ route mapped product
[64516 144]⍴3 3↕cimg +.× ck
⍝⍝: 38	110	220

⍝ 3.25 copies, for 9 multiply-adds per window (2.25), one read and one write. On macOS, 5.5 copies, because NEON's two lanes of f64 take twice the multiply-adds' time (4.5). `([1044484 9]⍴3 3↕sq)+.×kq` measured 7.49 on macOS on 22 October 2026, until rustymath's `by_columns` kept a strip of 8 results in registers while all nine columns add into it. On 27 October 2026 rustymath widened the strip to 32 results (`COLUMN_STRIP`), which took it from 6.11 to 5.23 in alternating runs. Strips of 8 for the 30 results that strips of 32 leave in each 1,022-wide row, with each result written rather than added to the zeroed output, brought it to 4.35 to 4.49 on 28 October 2026. On 22 October 2026 CI measured 5.36 on Linux and 5.85 in wasm. In runs 56 to 62 it measured 3.38 to 3.63 on Linux, and 4.98 in run 62, whose host was busy, because its data passes the runner's 1 MB L2.
⍝ In runs 56 to 62 CI measured 8.61 to 9.69 in wasm, where it measured 5.85 on 22 October 2026. Its expected cost there is 5.5, as on macOS, since wasm's SIMD also has two lanes of f64.
⍝⍝⍝ held wasm 9.69: the cause isn't known yet
⍝⍝⍝ route mapped product
[1044484 9]⍴3 3↕sq +.× kq
⍝⍝: 11	6.5	11

⍝ CI measured 80 to 87 on Linux in runs 56 and 59-61.
⍝⍝⍝ busy linux 158: its data passes CI's L2 (open.md, "Footprint copies")
⍝⍝⍝ route mapped product
{+/, [64516 144]⍴3 3↕cimg +.× ⍵}∂ ck
⍝⍝: 38	110	220

⍝ On 30 October 2026 it cost 48.9 on macOS and 125 with rustymath's own kernels, against 27 and 106 for its product from a stored matrix, while its program built the 144 MiB windows matrix of the padded cotangent whole. CI measured 190 on Linux in run 61. Before run 61, whose speed test first asked for huge pages through the memory guard, it measured 443 to 470 there. In wasm it measured 204 to 276 in runs 56 to 62. Later on 30 October 2026, with the program's copy of those windows left pending and read through their map, it cost about 41.3 on macOS (6,173 to 5,218 ps an item).
⍝⍝⍝ held linux 298: the product computes the arrays that the `gap` line below names before it reads their windows; busy hosts reach 298 on Linux
⍝⍝⍝ gap forced "+.×": the product computes the cotangent's 2,064,512 items, a broadcast of the seed, then its zero-padded copy's 2,130,048, before it reads the padded copy's windows through their map. The transposed product should run as nine shifted products of the cotangent with the kernel's slices, each added into the output at its offset, reading the cotangent where it lies (open.md, "The image gradient")
{+/, [64516 144]⍴3 3↕⍵ +.× ck}∂ cimg
⍝⍝: 54	160	320

⍝ A transpose of a million floats is a pending copy. A product reads it where it lies, as a transposed matrix. A fused call reads it in blocks of whole rows, as many as a cache line holds items.
⍝ Expected: `⍉sq +.× vq` 0.5 copies: reads the matrix once; `2×⍉sq` 1.25 copies: one read and one write per item, through a blocked transpose.
⍝ On macOS, Accelerate's product reads the matrix about four times as fast as a copy on one core, at 0.11 copies, so its macOS expected cost is 0.12.
⍝⍝⍝ held wasm 1.22: the cause isn't known yet
⍉sq +.× vq
⍝⍝: 0.5	1	1

⍝ On 27 October 2026 `2×⍉sq` measured 2.2 to 2.6 on macOS, with readings up to 3.5 while a video call ran. On 22 October 2026 CI measured 4.18 on Linux and 2.76 in wasm.
⍝⍝⍝ held macos 2.2 linux ~6 wasm ~5.5: today's route transposes into a block buffer of 128 KB, which doesn't fit in L1, and reads each source line twice, so the transpose costs about 2 copies rather than 1.25 (open.md, "Tiled transpose"); Linux and wasm hold estimates that keep their budgets from before 22 October 2026, since CI hasn't measured it since
2×⍉sq
⍝⍝: 2.5	2.5	2.5

⍝ `[1 2 0]⍉cimg` moves `cimg`'s contiguous axis first, and its last two axes merge into one axis of 65,536 items a stride of 16 apart. A block stage would read each block as one strided run, so it gathers the whole copy once through the tiled transpose first (`Mapping::tiles_whole`).
⍝ Expected: 2.25 copies: the transpose into the gathered copy, 1.25 as for `2×⍉sq`, then one fused pass over it. On 27 October 2026 it measured 3.4 to 3.6 on macOS, against 7.5 without the gathered copy, and later that day 3.6 to 4.2, with readings up to 5.2 while a video call ran. Nearly all its cost is the same transpose as `2×⍉sq`'s, which CI measured at 4.18 on Linux and 2.76 in wasm, against 2.2 on macOS. Scaling 3.6 by those ratios gives estimates of about 6.9 on Linux and 4.5 in wasm.
⍝⍝⍝ held linux ~6.9 wasm ~4.5: the transpose into the gathered copy costs about 2 copies rather than 1.25, for the reason `2×⍉sq`'s comment gives (open.md, "Tiled transpose")
2×[1 2 0]⍉cimg
⍝⍝: 4.5	4.5	4.5

⍝ `⍉⁻¹` splits and merges axes as einops does, and the result stays a view. `split` cuts 8 images of 224×224×3, 1,204,224 items, into 196 patches of 16×16×3, which reorders them. `rows` cuts each image's rows into 14 runs of 16 without reordering.
⍝ Expected: `+/,2×split⁝images` 0.9 copies: one fused pass over 1.15 times `n`, which reads each item once and writes nothing, about 0.6, and a block buffer that gathers the patches' runs of 48 items, about 0.3, at the 0.25 copies for each `n` items of `2×⌽sv+1`'s buffer; `+/,2×rows⁝images` 0.6 copies: the same pass, reading `images` in place. Wasm, twice the native cost, as `+/v`'s fold.
⍝ On 27 October 2026, with other work running, `+/,2×split(images)` measured 2.0 copies against 1.0 for `+/,2×images`, so its buffer cost about twice what `2×⌽sv+1`'s did in the same run.
⍝⍝⍝ held wasm 1.98: margin against noise for a case within 10% of its budget (30 October 2026)
+/,2×split⁝images
⍝⍝: 1.8	1.8	3.6

+/,2×rows⁝images
⍝⍝: 1.2	1.2	2.4

⍝ A select by a comparison runs as one program with the comparison and the cases: `0.5<sv` stays a lane mask, and `-` runs on every item.
⍝ Expected: 1.25 copies: one fused pass with a comparison, a negation and a blend.
⍝ CI measured 2.97 to 3.87 on Linux and 3.51 to 3.83 in wasm in most runs from 56 to 62.
⍝⍝⍝ held macos 3.04 linux 5.69 wasm 5.12: open.md lists masked selects among the speed audit's outliers; busy hosts reach the Linux and wasm costs
(0.5<)⍚[⊢ -] sv
⍝⍝: 2.5	2.5	2.5
