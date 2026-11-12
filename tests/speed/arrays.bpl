⍝⍝ setup
side←256 size 32
n←side×side
v←¿nρ0
w←¿nρ0
s←v-0.5
m←[side side]ρv
jv←¿nρ1000ₓ
kv←¿nρ1000000000000ₓ
b←0ₓ=¿nρ3ₓ
i←¿nρ⌊ₓn
cnt←¿nρ3ₓ
c←nρ"the quick brown fox jumps over the lazy dog"
nd←•distribution "normal"
lg←•distribution "lognormal"
ud←•distribution "uniform"
ex←•distribution "exponential"
ca←•distribution "cauchy"
lp←•distribution "laplace"
lo←•distribution "logistic"
wb←1.5 2 •distribution "weibull"
coeffs←0.5 ¯1.25 0.75 2 ¯0.125 0.0625 ¯1.5 0.25
ms←[n÷64 8 8]ρv
ss←ms+0 2 1⍉ms
sps←ss(+⍤2)16×=⊗⍨ι8
t←[n÷16 16]ρv
tq←¿[n÷4 24]ρ0
z←v+0j1×w
zm←[(128 size 16) (128 size 16)]↑m+0j1×⌽₀m
bm←[side side]ρb
jm←[side side]ρjv
um←[2×side 2×side]ρ(256×w)ₓ₁
cm←[side side]ρc
[ir ic]←•index side side
ia←ir.[¿sideρside]
id←•index side
lut←256↑v
ux←(¿nρ256)ₓ₁
⍝⍝ copies
⍝ Sharing storage copies nothing.
⍝ Expected: 0 copies each: shares storage.
,m
⍝⍝: 0.5	0.5	0.5	instructions macos 0.938 linux 0.397	allocated macos 0.032 linux 0.032

ρm ρ v
⍝⍝: 0.5	0.5	0.5	instructions macos 1.654 linux 0.968	allocated macos 0.074 linux 0.074

⍝ Reading each item once, with almost nothing to write.
⍝ Expected: `+/v`, `0+/v` (the kernel's sum, then the seed), `-/v`, `⌈/v`, `+/m` and `+⌿m` 0.5 copies: reads 8 bytes an item and writes nothing; `+/jv` and `-/jv` 0.13 copies: reads 2 bytes an item; `+/z` 1 copy: reads 16 bytes an item; `∧/b` 0.06 copies: reads 1 byte an item.
⍝ Wasm on CI's x86 runner: under Node 26's V8, `+/v` costs what it costs natively, 0.48 to 0.51 copies in the runs of 28 and 29 October 2026, against 1.0 to 1.3 under Node 22. `-/v` folds its items in pairs and costs about 0.7 there. On this Mac's V8 the order reverses: `+/v` costs 0.6 and `-/v` 0.8. On this Mac's V8, the only one that can be timed here, a pair fold raised `+/v` from 0.60 to 0.82 copies. `0+/v` runs the same kernel as `+/v` and has the same budgets.
+/v
⍝⍝: 1	0.68	0.76	instructions macos 3.762 linux 2.183	allocated macos 0.171 linux 0.171

0+/v
⍝⍝: 1	0.72	0.84	instructions macos 4.212 linux 2.752	allocated macos 0.189 linux 0.189

+/jv
⍝⍝: 0.5	0.5	0.53	instructions macos 3.631 linux 2.567	allocated macos 0.175 linux 0.175

+/z
⍝⍝: 2	1.8	2.4	instructions macos 4.566 linux 2.401	allocated macos 0.172 linux 0.172

-/v
⍝⍝: 1	1.7	0.95	instructions macos 3.144 linux 1.749	allocated macos 0.123 linux 0.123

-/jv
⍝⍝: 0.5	0.5	0.53	instructions macos 3.024 linux 2.086	allocated macos 0.128 linux 0.128

⌈/v
⍝⍝: 1	0.69	0.7	instructions macos 3.797 linux 2.587	allocated macos 0.171 linux 0.171

∧/b
⍝⍝: 0.5	0.5	0.5	instructions macos 3.047 linux 2.124	allocated macos 0.171 linux 0.171

+/m
⍝⍝: 1	1.1	0.98	instructions macos 4.929 linux 3.064	allocated macos 0.304 linux 0.304

+⌿m
⍝⍝: 1	0.93	0.95	instructions macos 4.584 linux 2.984	allocated macos 0.304 linux 0.304

⍝ A fold over several axes of a simple array needs no call per cell. `+ × ⌈ ⌊` fold each axis in turn. Other primitives fold each cell's ravel once.
⍝ Expected: 0.5 copies each: reads 8 bytes an item and writes nothing.
+/₁ ₂ms
⍝⍝: 1	1.1	1.3	instructions macos 6.138 linux 3.93	allocated macos 0.401 linux 0.401

-/₁ ₂ms
⍝⍝: 1	2.5	1.8	instructions macos 5.786 linux 4.347	allocated macos 0.474 linux 0.474

⍝ A cheap ordered right fold keeps each consecutive lane's running value in a local.
⍝ Expected: 6 copies: about 3 cycles an item, because the running value is usually below the next item and most steps skip the division.
|/jv
⍝⍝: 12	19	40	instructions macos 16.1 linux 17.508	allocated macos 0.574 linux 0.574

⍝ Reading one argument and writing each item once.
⍝ Expected: `-v` and `⌽v` 1 copy: one read and one write per item; `-jv` 0.25 copies: 2-byte reads and writes; `~b`, `∧\b` and `∨\b` 0.13 copies: 1-byte reads and writes; `1⌽v` 1 copy: two contiguous copies; `⌽₀m` 1 copy: contiguous row copies; `1↓v` 1 copy: one copy; `(n÷2)↑v` 0.5 copies: copies half the items; `nρ0` 0.5 copies: writes 8 bytes an item.
-v
⍝⍝: 2	1.9	2.1	instructions macos 2.995 linux 1.746	allocated macos 8.06 linux 8.06

-jv
⍝⍝: 0.5	0.5	0.7	instructions macos 2.609 linux 1.747	allocated macos 2.063 linux 2.063

~b
⍝⍝: 0.5	0.5	0.5	instructions macos 2.102 linux 1.236	allocated macos 1.062 linux 1.062

∧\b
⍝⍝: 0.5	0.5	0.5	instructions macos 2.091 linux 1.311	allocated macos 1.09 linux 1.09

∨\b
⍝⍝: 0.5	0.5	0.5	instructions macos 2.088 linux 1.42	allocated macos 1.09 linux 1.09

⌽v
⍝⍝: 2	1.9	2.7	instructions macos 2.814 linux 1.475	allocated macos 8.057 linux 8.057

⍝ In wasm on CI's x86 runner, `1⌽v` and `1↓v` cost about 0.8 copies in some runs and 1.6 to 1.9 in others. The cause is upstream. V8 runs wasm's `memory.copy` with the host's `memmove`. On CI's Ubuntu 24.04, glibc 2.39's `memmove` copies runs of 4 KB to 1 MB with `rep movsb`. Zen 3 and Zen 4 run `rep movsb` many times slower when the destination sits a little past the source modulo 4 KB (glibc bug 30994, fixed in glibc 2.40). The browser build's allocator puts each equal-sized block 16 bytes further along modulo 4 KB than the one before. A copy into the next block then lands 8 to 16 bytes past its source, in the slow case. The build keeps `memory.copy`: a loop of its own made these copies 1.6 times slower on this Mac's Node.
1⌽v
⍝⍝: 2	1.8	1.4	instructions macos 3.052 linux 0.852	allocated macos 8.058 linux 8.058

⌽₀m
⍝⍝: 2	1.9	1.9	instructions macos 3.306 linux 1.995	allocated macos 8.067 linux 8.067

⍝ In wasm on CI, `1↓v` varies as `1⌽v` does, for the reason given above `1⌽v`.
1↓v
⍝⍝: 2	1.8	1.3	instructions macos 3.286 linux 0.995	allocated macos 8.063 linux 8.063

n÷2 ↑ v
⍝⍝: 1	0.66	0.79	instructions macos 3.037 linux 1.449	allocated macos 4.082 linux 4.082

⍝ Expected: 0.5 copies: writes 8 bytes an item.
nρ0
⍝⍝: 1	1.2	1.3	instructions macos 1.989 linux 1.22	allocated macos 8.057 linux 8.057

⍝ Reading two arguments and writing each item once.
⍝ Expected: `v+w`, `kv+kv` and `v⌊w` 1.5 copies: two 8-byte reads and one write per item; `jv+jv` and `jv∧ᵇjv` 0.38 copies: 2-byte reads and writes; `v×2` 1 copy: one read and one write per item; `jv×2.5` 0.63 copies: its products are floats, with 2-byte reads and 8-byte writes; `jv=jv` 0.31 copies: two 2-byte reads and a 1-byte write; `b×jv` 0.31 copies: 1- and 2-byte reads, 2-byte writes; `b×kv` 1.1 copies: 1- and 8-byte reads, 8-byte writes; `b∧b` 0.19 copies: 1-byte reads and writes; `c=c` 0.56 copies: two 4-byte reads and a 1-byte write; `ι√n ×⊗ ι√n` 0.5 copies: writes 8 bytes an item.
⍝⍝⍝ route vector kernel
v+w
⍝⍝: 3	4.5	3.7	instructions macos 3.482 linux 2.137	allocated macos 8.079 linux 8.079

jv+jv
⍝⍝: 0.76	0.62	0.75	instructions macos 3.373 linux 2.243	allocated macos 2.085 linux 2.085

kv+kv
⍝⍝: 3	2.4	3.2	instructions macos 6.175 linux 3.712	allocated macos 8.085 linux 8.085

v×2
⍝⍝: 2	2.1	1.9	instructions macos 3.341 linux 1.939	allocated macos 8.085 linux 8.085

jv×2.5
⍝⍝: 1.3	1.5	2.2	instructions macos 5.528 linux 3.206	allocated macos 10.089 linux 10.089

jv=jv
⍝⍝: 0.62	0.5	1.1	instructions macos 2.405 linux 1.583	allocated macos 1.081 linux 1.081

jv∧ᵇjv
⍝⍝: 0.76	0.5	0.73	instructions macos 4.215 linux 2.387	allocated macos 2.112 linux 2.112

b×jv
⍝⍝: 0.62	0.54	0.69	instructions macos 2.998 linux 1.968	allocated macos 2.585 linux 2.585

⍝⍝⍝ held linux 2.81: Syd1's Boolean-by-integer product costs 2.2 times the 1.25 its Linux budget allows; cause not traced
b×kv
⍝⍝: 2.2	2.5	3.8	instructions macos 9.212 linux 6.623	allocated macos 10.085 linux 10.085

v⌊w
⍝⍝: 3	4.4	3.8	instructions macos 3.983 linux 2.105	allocated macos 8.079 linux 8.079

b∧b
⍝⍝: 0.5	0.5	0.5	instructions macos 2.522 linux 1.542	allocated macos 1.083 linux 1.083

c=c
⍝⍝: 1.2	0.77	2	instructions macos 2.943 linux 1.905	allocated macos 1.1 linux 1.1

⍝ Expected: 0.73 copies: 0.5 for writing 8 bytes an item, and one node, about 0.23 copies of 65,536 floats (`fusion::MAC_NODE_NS`, 1,800 ns), because the call waits under the cost rule and nothing fuses with it (decision 13). Jeremy accepted a lone 65,536-item call's node on 30 October 2026, as for `c=c`. Main set this expected cost on 31 October 2026, for Jeremy's review. In the browser the call runs at once, because the cost rule's limit there, from the browser's node estimate of 4 × `MAC_NODE_NS`, is about 830 KB, above the call's 512 KB. CI measured it there at 1.06 copies.
ι√n ×⊗ ι√n
⍝⍝: 1.5	1.7	1.9	instructions macos 6.694 linux 4.935	allocated macos 8.379 linux 8.379

⍝ Products of 64-bit integers that leave 64 bits become exact big integers, each held in an allocation of its own.
⍝ Expected: 220 copies: about 25 ns an item for an allocation, a free and a 128-bit product.
kv×kv
⍝⍝: 330	2.4	3.3	instructions macos 7.695 linux 5.727	allocated macos 8.085 linux 8.085

⍝ A tolerant gcd of floats walks the continued fraction of each pair's ratio in 64-bit integers wherever the ratio fits.
⍝ Expected: 700 copies: about 35 continued-fraction steps per pair, each waiting on a float division of about 10 cycles.
v∨w
⍝⍝: 910	1300	1400	instructions macos 1945.9 linux 1964.33	allocated macos 8.079 linux 8.079

⍝ Compressing by a mask can't be vectorised, because each kept item's place depends on the items before it.
⍝ Expected: `b#v` 2 copies: about 1 cycle an item, since each kept item's place depends on the items before it; `⍸b` 2 copies: about 1 cycle an item, as compression.
b#v
⍝⍝: 4	6.3	5.8	instructions macos 12.925 linux 13.124	allocated macos 2.736 linux 2.736

⍸b
⍝⍝: 4	2.4	3.1	instructions macos 5.162 linux 4.856	allocated macos 0.708 linux 0.708

⍝ Replicating by counts of 0 to 2 writes four copies of each item where its run starts. The next item overwrites the copies past its count. A group of 4 items whose counts are all at most 4 writes after one check for the group (`gather::repeat`). With a check and a taken branch for each item, the loop cost about a third more in the speed test's binary than through `bpl`: `cnt#v` measured 7.1 copies there against 5.3, and `(0+cnt)#v` 9.7 against 8.1.
⍝ Expected: 4 copies: about 2 cycles an item for four stores, which the next item overwrites past its count.
⍝⍝⍝ held linux 8.55: Syd1's replication by counts costs 2.1 times the 4 its Linux budget allows; cause not traced
cnt#v
⍝⍝: 8	8	13	instructions macos 29.183 linux 27.182	allocated macos 16.078 linux 16.078

⍝ Counts held as floats are checked for whole numbers 256 at a time before they replicate.
⍝ Expected: 6 copies: as `cnt#v`, plus about 1 copy for the add that makes the counts floats, and about 1 more for the whole-number check than `cnt#v` pays to widen its 1-byte counts, because the check rounds each float and writes it as a 64-bit integer.
⍝ It has cost 25 to 26 in wasm in every CI run from 53 to 62, where `cnt#v` costs about 6.7.
0+cnt # v
⍝⍝: 12	22	48	instructions macos 41.452 linux 36.785	allocated macos 26.158 linux 26.158

⍝ A range with an approximate length converts each position from an integer to a float. Wasm has no vector instruction for that, so there each block of 8 positions adds 0 to 7 to the block's first float.
⍝ Expected: 0.5 copies: writes 8 bytes an item.
ιn
⍝⍝: 1	1.8	1.9	instructions macos 9.054 linux 6.507	allocated macos 2.032 linux 2.032

⍝ Comparing within tolerance takes a subtraction, the magnitudes, a scale and a comparison for each item. Tolerant floor raises each item by its scaled magnitude, capped below a half, before it floors.
⍝ Expected: `⌊v` 2.75 copies: about 11 vector operations an item, 6 for the tolerance and 5 for the conversion to a 64-bit integer, its range check, and the select and flag for an item that doesn't fit; `v<w` 3 copies: about 12 vector operations an item: 10 for tolerant equality (an exact comparison, a subtraction, three magnitudes, the larger magnitude, a scale, a comparison and an OR), then an exact comparison and an AND-NOT; `jv<500ₓ` and `jv<500` 0.19 copies: a 2-byte read and a 1-byte write, where the whole float 500 compares as the integer it stands for. `⌊v` and `v<w` were 1.5 copies, from a count of the tolerance's operations alone.
⌊v
⍝⍝: 5.5	4	7	instructions macos 8.351 linux 7.684	allocated macos 1.06 linux 1.06

v<w
⍝⍝: 6	3.9	6.7	instructions macos 3.747 linux 5.921	allocated macos 1.079 linux 1.079

jv<500ₓ
⍝⍝: 0.5	0.5	1.1	instructions macos 2.561 linux 1.699	allocated macos 1.095 linux 1.095

jv<500
⍝⍝: 0.5	0.5	1.1	instructions macos 2.633 linux 1.781	allocated macos 1.095 linux 1.095

⍝ Writing twice as many items, or writing in a different order from reading.
⍝ Expected: `v,w` 2 copies: writes twice as many items as `v` holds; `⍉m` 1.5 copies: one copy, with cache blocking for the strided side.
⍝ In wasm on CI's x86 runner, `v,w` costs about 1.8 copies in some runs and about 2.6 in others, for the reason given above `1⌽v`.
v,w
⍝⍝: 4	6.2	4.8	instructions macos 5.21 linux 1.225	allocated macos 16.107 linux 16.107

⍉m
⍝⍝: 3	4.1	3.7	instructions macos 3.228 linux 3.35	allocated macos 8.081 linux 8.081

⍝ Windows copy each item once for every window that holds it. Padded windows first copy the argument with its padding.
⍝ Expected: `3↕v` 2 copies: reads each item once and writes it three times; `3 3↕m` 7 copies: reads each item once and writes it nine times, which costs what `[254 254 3 3]ρv` costs to write as many items into a fresh result, 6.5 to 7.2 on macOS (22 October 2026); `¯3 ¯3↕m` 8 copies: as `3 3↕m`, plus one padded copy.
⍝ `3↕v` measures 2.7 on macOS since short cells copy each run of adjacent items as one block (22 October 2026), down from 4 to 4.84.
3↕v
⍝⍝: 4	6.8	8.3	instructions macos 11.227 linux 9.333	allocated macos 24.096 linux 24.096

⍝ `3 3↕m` measures 10.1 on macOS since the short-cell loop tests a cell's direction once a call, not once a run (22 October 2026), down from 12 to 12.5.
3 3↕m
⍝⍝: 14	20	26	instructions macos 46.581 linux 49.66	allocated macos 70.1 linux 70.096

⍝ `¯3 ¯3↕m` measures 11.5 on macOS since the short-cell loop tests a cell's direction once a call, not once a run (22 October 2026), down from 13.2 to 13.3.
¯3 ¯3↕m
⍝⍝: 16	25	29	instructions macos 53.443 linux 55.837	allocated macos 80.752 linux 80.748

⍝ A small-table index finds the items' range in one vectorised pass, then writes each item's first position into the table, one item at a time.
⍝ Expected: 0.65 copies: the table pass stops once every key has its first position, which for `jv`'s 1,000 keys comes after about 7,500 items, at about 2 cycles each, 0.45; a pass for the keys' range, 0.13; and writing the rest of the mask, 0.06. It was 2 copies, from a pass over every item.
≠jv
⍝⍝: 1.3	1.6	1.3	instructions macos 10.264 linux 9.994	allocated macos 1.276 linux 1.276

⍝ A search also looks each item of the other argument up in the table, and unique then compresses by the mask.
⍝ Expected: `jvιjv`, `jv∊jv` and `cιc` 4 copies: about 1 cycle an item to build the table and 1 to look up; `∪jv` and `∪c` 2 copies: the unique mask, then a compression of few items.
jvιjv
⍝⍝: 8	6.6	7.8	instructions macos 23.535 linux 23.458	allocated macos 6.276 linux 6.276

jv∊jv
⍝⍝: 8	7.1	9.6	instructions macos 23.822 linux 23.728	allocated macos 5.276 linux 5.276

cιc
⍝⍝: 8	9	11	instructions macos 20.947 linux 21.377	allocated macos 6.054 linux 6.054

∪jv
⍝⍝: 4	2.8	2.9	instructions macos 14.793 linux 14.509	allocated macos 1.406 linux 1.406

∪c
⍝⍝: 4	3.2	5.1	instructions macos 8.045 linux 7.523	allocated macos 1.069 linux 1.069

⍝ Key classifies the keys once. `(≢⊢)⌸` counts each class without building its group, and a fold of `⊢` such as `(+/⊢)⌸` folds each group of a vector in one pass. Any other operand, `{≢ω}` included, runs once for each group, on that group's positions.
⍝ Expected: `{≢ω}⌸b` 2 copies: classifying, then writing each group's positions; `(≢⊢)⌸b` 0.13 copies: reads 1 byte an item and counts.
{≢ω}⌸b
⍝⍝: 4	5.8	5.3	instructions macos 17.014 linux 13.668	allocated macos 7.076 linux 7.076

(≢⊢)⌸b
⍝⍝: 0.5	1.8	1.9	instructions macos 3.038 linux 2.084	allocated macos 0.127 linux 0.127

⍝ `(+/⊢)` folds each group of the values in one pass. The keys `i` make about 41,000 groups, each of which the general path would call the operand for.
⍝ Expected: 15 copies: keys below 65,536 index a table of 32-bit positions directly. First sight of each key takes about 4.3, numbering each item through the table about 3.5, the fold about 3.8, filling the table and writing each class about 2.3, and allocation about 0.9, by the shares of a profile on 30 October 2026, when the case measured 1,790 ps an item on macOS. CI measured 19.9 on Linux.
i (+/⊢)⌸ v
⍝⍝: 30	29	26	instructions macos 59.227 linux 58.684	allocated macos 24.258 linux 24.258

⍝ Layout cohorts gather equally sized groups and run the dfn once per group size through the dense batch evaluator. The specialized `(+/⊢)` route remains one native grouped fold.
⍝ Expected: 15 copies, as `i (+/⊢)⌸ v`: the same classification and one fold of each group.
⍝⍝⍝ held macos 224.8 linux 361.7: cohorts still gather and dispatch per distinct group size instead of running one flat segmented fold
i{+/ω}⌸v
⍝⍝: 30	29	26	instructions macos 763.952 linux 882.254	allocated macos 111.616 linux 111.616

⍝ Classify of Booleans compares each item with the first. Groups of Booleans write each class's positions 8 items at a time, from a table indexed by the byte of those items' classes.
⍝ Expected: `⍷b` 1 copy: writes 4-byte positions, 8 at a time from a table; `=b` 0.13 copies: 1-byte reads and writes.
⍷b
⍝⍝: 2	5.8	5.1	instructions macos 16.377 linux 13.065	allocated macos 7.061 linux 7.061

=b
⍝⍝: 0.5	0.5	0.55	instructions macos 1.456 linux 0.701	allocated macos 2.032 linux 2.032

⍝ Interval index sorts its boundaries, then reads each value's count of boundaries at or below it from a table over their keys.
⍝ Floats' keys span too much for a table, so sixteen values binary-search the sorted floats together.
⍝ Expected: `(<jv)⍸jv` 6 copies: a counting sort of the boundaries, then one table read per value; `(<w)⍸v` 128 copies: a comparison sort of the boundaries, about 64 copies, and 16 binary-search steps per value, about 64.
<jv ⍸ jv
⍝⍝: 12	16	16	instructions macos 44.1 linux 40.729	allocated macos 8.787 linux 8.787

<w ⍸ v
⍝⍝: 200	160	160	instructions macos 308.307 linux 326.543	allocated macos 38.055 linux 38.055

⍝ Each item waits for the result before it. In scans by `+` and `⌈`, only one item in each group of four waits for the group before it.
⍝ Expected: `+\v` 1.5 copies: one 3-cycle addition waits for each group of four items; `⌈\v` 1.5 copies: one maximum waits for each group of four items; `+\jv` and `+\b` 1.5 copies: a first pass writes each group's four running sums, then a second adds the running total to them as one vector, so one integer addition of 1 cycle waits for each group; `≠\b` 0.5 copies: word-wide bit operations; `-\v` 1.5 copies: as `+\v`, each item after the first negated as it is added; `-\jv` 1.5 copies: as `+\jv`; `⌈\jv` 1 copy: a vectorised running maximum; `+\z` 2 copies: 16-byte reads and writes, with grouped additions.
⍝ Changed on 21 October 2026: `+\jv`, `+\b` and `-\jv` expected 2 copies, for one addition waiting for each item. In one loop, the compiler had rewritten `+\jv`'s grouped integer sums into that chain, at 2.1 copies. Integer sums take `grouped`'s first pass, which makes their grouping hold in every build. Float scans keep the one loop, where the first pass cost about half a copy, and so do integer `⌈\` and `⌊\`, where it cost about a third more.
⍝ Changed on 22 October 2026: the first pass writes each group's four running sums, in the order the outputs take them, and the group's sum to a buffer of its own. With only the group's sum, x86-64 builds vectorised the second pass by assembling each group's sums with byte and word inserts, and `+\b` cost 3.16 copies on CI's runner against 1.65 for the one loop. The second pass now adds the total to one loaded vector, and the total adds the group's sum from its own buffer, so one scalar addition waits for each group. On this Mac `+\b` went from 1.26 copies to 1.10, `-\jv` from 1.47 to 1.38 and `+\jv` from 1.32 to 1.27.
⍝ Wasm: `+\jv`, `+\b` and `-\jv` 2 copies, for the shuffles that build each group's running sums without a deinterleaving load such as aarch64's `ld4`. On this Mac's V8 the layout of 22 October 2026 took `+\b` from 2.13 copies to 1.6, its cost before the first pass, `+\jv` from 2.2 to 1.86 and `-\jv` from 2.45 to 2.28. CI's x86 V8 measured the one loop at 1.6 to 1.9.
+\v
⍝⍝: 3	2.9	3.1	instructions macos 6.07 linux 5.126	allocated macos 8.092 linux 8.092

⌈\v
⍝⍝: 3	2.5	2.4	instructions macos 6.072 linux 3.919	allocated macos 8.092 linux 8.092

+\jv
⍝⍝: 3	2.3	2.9	instructions macos 6.795 linux 6.892	allocated macos 5.095 linux 5.095

⍝⍝⍝ held linux 1.92: Syd1's Boolean running sum costs 2.1 times the 0.9 its Linux budget allows; cause not traced
+\b
⍝⍝: 3	1.8	3.6	instructions macos 8.472 linux 8.838	allocated macos 2.592 linux 2.592

≠\b
⍝⍝: 1	2.1	2.2	instructions macos 4.359 linux 3.67	allocated macos 1.09 linux 1.09

-\v
⍝⍝: 3	3.1	3.4	instructions macos 5.74 linux 4.546	allocated macos 8.092 linux 8.092

-\jv
⍝⍝: 3	2.3	3.2	instructions macos 9.454 linux 9.139	allocated macos 7.095 linux 7.095

⌈\jv
⍝⍝: 2	0.64	0.9	instructions macos 3.103 linux 2.452	allocated macos 2.094 linux 2.094

+\z
⍝⍝: 4	6.2	5.5	instructions macos 6.071 linux 5.371	allocated macos 16.094 linux 16.094

⍝ A fold or scan by `ᵇ`'s and, or, xor or xnor runs as rustymath's `And`, `Or`, `Xor` or `Xnor` reduction, which the table of facts names. Booleans fold by those tables as they fold by `∧`, `∨`, `≠` and `=`. Other tables, such as nand and nor, fold and scan through the integer kernels, one item at a time.
⍝ Expected: `∧ᵇ/kv` 0.5 copies: reads 8 bytes an item and writes nothing, and the fold vectorises; `∨ᵇ\jv` 1.5 copies, and 2 in wasm: a running or of 2-byte items through rustymath's `grouped`, as `+\jv` scans.
⍝ `∧ᵇ/kv` measures 0.57 on macOS, and 0.62 to 0.73 in wasm in seven CI runs from 22 October 2026. CI measured 0.43 on Linux on 22 October 2026.
∧ᵇ/kv
⍝⍝: 1	0.66	0.76	instructions macos 4.166 linux 2.648	allocated macos 0.184 linux 0.184

∨ᵇ\jv
⍝⍝: 3	3.4	5.1	instructions macos 8.126 linux 7.875	allocated macos 2.103 linux 2.103

⍝ After a lane's first result, a comparison scan's results are 0 or 1. Comparing each item at both, then picking between the results 64 at a time, lets items compare side by side and gives Booleans directly. Rustymath's ordered scan doesn't do that yet. It steps each item from the result before it, at the items' kind (`operation.rs` `chained`). A seeded scan's results then convert to Booleans.
⍝ Expected, from the pick 64 at a time: `<\v`, `0<\v` and `=⍀m` 3 copies: two tolerant comparisons an item; `≠\jv` and `0ₓ=\jv` 1 copy: an exact comparison an item.
<\v
⍝⍝: 6	7.6	16	instructions macos 9.755 linux 9.205	allocated macos 10.091 linux 10.091

=⍀m
⍝⍝: 6	5.9	15	instructions macos 4.849 linux 6.183	allocated macos 8.138 linux 8.138

≠\jv
⍝⍝: 2	3.1	4.7	instructions macos 8.46 linux 7.643	allocated macos 4.088 linux 4.088

0ₓ=\jv
⍝⍝: 2	2.9	4.4	instructions macos 6.883 linux 5.863	allocated macos 3.119 linux 3.119

0<\v
⍝⍝: 6	6.1	11	instructions macos 8.566 linux 8.2	allocated macos 3.107 linux 3.107

⍝ Scans along short lanes make no call for each lane. Comparison and Boolean scans restart lanes inside each 64-bit word of results. Other scans run lanes under 64 items eight side by side, so eight chains of additions are always in flight. Run one lane at a time, as before 21 October 2026, their cost moved with where the code sat: `+\X` cost 1.8 copies in the test binary against 0.9 through `bpl`, and `+\[16384 4]ρv` 3.4 against 2. Scans along a leading axis make no call for each row: `+⍀[16384 4]ρv` cost 28 copies while each row of 4 items made two calls that stayed out of line.
⍝ Expected: `=\[32768 2]ρjv` 4 copies and `<\[16384 4]ρv` 6, set on the Mac on 22 October 2026 by a route for comparison scans along short lanes that's since gone (basedpl's `Scan::compared` through rustymath's `comparison_scan`). Both now run rustymath's ordered scan: `scan_read`, then `ordered_scan` with one kernel step for each item; `≠\[16384 4]ρb` 0.5 copies: word-wide bit operations; `+\[16384 4]ρv` 2 copies: eight lanes run side by side, about 1 cycle an item; `+⍀[16384 4]ρv` 3 copies: each row waits on the row before, which it reads back from the output through a store-to-load forwarding before it adds, about 6 cycles for each row of 4 items.
=\[n÷2;2]ρjv
⍝⍝: 8	6.7	8.8	instructions macos 8.979 linux 9.36	allocated macos 2.16 linux 2.16

<\[n÷4;4]ρv
⍝⍝: 12	14	23	instructions macos 9.494 linux 14.869	allocated macos 8.167 linux 8.167

≠\[n÷4;4]ρb
⍝⍝: 1	2	2.3	instructions macos 5.918 linux 5.064	allocated macos 1.162 linux 1.162

+\[n÷4;4]ρv
⍝⍝: 4	3.4	3	instructions macos 8.155 linux 6.877	allocated macos 8.167 linux 8.167

+⍀[n÷4;4]ρv
⍝⍝: 6	4.6	3.4	instructions macos 15.227 linux 9.853	allocated macos 8.162 linux 8.162

⍝ A residue scan waits on the result before each item. An exact seed, or a whole float seed that stands for an integer, keeps it in integers.
⍝ Expected: `7ₓ|\jv` 16 copies: each item waits for an integer division of about 8 cycles; `7|\jv` 16.5 copies: the same scan, then its results as 8-byte floats.
7ₓ|\jv
⍝⍝: 32	56	62	instructions macos 20.81 linux 21.786	allocated macos 2.12 linux 2.12

7|\jv
⍝⍝: 33	63	64	instructions macos 23.193 linux 23.123	allocated macos 10.141 linux 10.141

⍝ An inverse scan by `+`, `-`, `=` or `≠` combines each item with the item before it, in one pass.
⍝ Expected: `+\⁻v` 1 copy: one read and one write per item; `≠\⁻b` 0.13 copies: 1-byte reads and writes.
+\⁻v
⍝⍝: 2	1.9	2.4	instructions macos 3.258 linux 2.113	allocated macos 8.105 linux 8.105

≠\⁻b
⍝⍝: 0.5	0.5	0.5	instructions macos 2.558 linux 1.593	allocated macos 3.105 linux 3.105

⍝ Each item waits for a read from a random place.
⍝ Expected: 3 copies: about 1.5 cycles an item for an independent read in the cache, plus the 4-byte positions.
⍝ It measured 5.3 to 6.3 copies from 4,096 to 262,144 items, and about 6.5 at a million. It measures about 4.1 at 65,536 items since the one pass that checks the index values folds their least and greatest in the values' own type (`IndexValue::bounds`, 22 October 2026). A compact integer or whole-float index array becomes a table that reads its values in place (`structure::index_table`), and the walk takes items straight through the values (`Table::take_into`, through `gather::take`). The rest of the gap is that checking pass and `take`'s bounds check on every read.
[i]⌷v
⍝⍝: 6	7	7.3	instructions macos 13.783 linux 13.716	allocated macos 8.146 linux 8.146

⍝ Named arrays of positions that share no axis keep their own positions, and positions that step evenly read as strides (`structure::joined`, `Table::affine`, 26 October 2026).
⍝ Expected: `m.[ir ic]` 0.5 copies: it reads every item in order, so the result shares `m`'s storage, and the rest is the call's fixed cost, about 46,000 instructions as counted on a 16 by 16 matrix; `m.[id id]` 0.5 copies: the 256 items of the diagonal through one stride, and the fixed cost; `m.[ir 1↓ic]`, `m.[ir ic-1]` and `m.[ia ic]` 1.5 copies: one copy through strides, or through 256 row positions and a stride, and the fixed cost.
⍝ Before, `.[]` summed every paired array's positions into one table of 65,536 offsets, one item at a time, and each of these but the diagonal cost 66 to 70 copies.
m.[ir ic]
⍝⍝: 1	0.61	0.69	instructions macos 4.43 linux 2.942	allocated macos 0.32 linux 0.32

m.[id id]
⍝⍝: 1	0.7	0.72	instructions macos 4.479 linux 3.039	allocated macos 0.379 linux 0.379

m.[ir 1↓ic]
⍝⍝: 3	3	3	instructions macos 7.687 linux 5.18	allocated macos 8.339 linux 8.34

⍝ `ic-1` reads each row shifted by one column, with its first position counting back from the end, as `¯1⌽` does.
m.[ir ic-1]
⍝⍝: 3	3.7	4.1	instructions macos 11.255 linux 8.85	allocated macos 8.61 linux 8.611

m.[ia ic]
⍝⍝: 3	2.6	2.4	instructions macos 6.61 linux 4.427	allocated macos 8.329 linux 8.33

⍝ Positions in fixed-width storage read as ordinary integers do.
⍝ Expected: 2.5 copies: about 1.5 cycles an item for a read from a 256-item table in the cache, plus the 1-byte positions. Before 26 October 2026, `keyed::Selector::of` made a value of each position to look for key names, and this cost about 42 copies.
lut.[ux]
⍝⍝: 5	5.4	3.6	instructions macos 10.875 linux 9.793	allocated macos 8.198 linux 8.198

⍝ An indexed assignment copies the array once, then writes each value at its index.
⍝ Expected: 4 copies: one copy, then about 1.5 cycles an item for each scattered write.
{
  x←v
  x.[i]←w
  x
}0
⍝⍝: 8	12	12	instructions macos 16.981 linux 20.28	allocated macos 8.214 linux 8.214

⍝ A modified assignment with repeated indices combines each value into the array at its index, in turn, in place.
⍝ Expected: 5 copies: one copy, then about 2 cycles an item for each scattered read and write.
{
  x←v
  x.[i]+←w
  x
}0
⍝⍝: 10	16	15	instructions macos 23.923 linux 19.294	allocated macos 8.282 linux 8.282

⍝ Under copies the argument, applies the function to what the structural function selects, and writes the result back through the same map.
⍝ Expected: `-@⌽ v` 3 copies: one copy, the negation read through the reversed map, as `-⌽v` costs, and the reversed write; `-@⍉ m` 4 copies: one copy, the negation read through the transpose, about 1.5 copies as `⍉m` costs, and the write back in tiles, about 1.5 copies.
⍝ `-@⍉ m` measures about 6.4 on macOS and 6.7 in wasm. Its negation runs as a separate pass over the gathered transpose, about 1 copy, where the expected cost reads through the transpose.
⍝⍝⍝ gap forced "-@⌽": Under should run its function through the selection's map in the fused pass and write the results back, as the expected cost assumes
-@⌽ v
⍝⍝: 6	13	12	instructions macos 9.633 linux 4.479	allocated macos 24.343 linux 24.343

⍝⍝⍝ gap forced "-@⍉": Under should run its function through the selection's map in the fused pass and write the results back, as the expected cost assumes
-@⍉ m
⍝⍝: 8	17	16	instructions macos 9.542 linux 7.955	allocated macos 24.392 linux 24.392

⍝ Expected: `(2×)@((n+2ₓ)↑) v` and `(2×)@(¯1⌽(n+2ₓ)↑) v` 3 copies each: one copy, the doubling read through the overtake's map, and the write back of the items that the overtake took from `v`. The write back walks the overtake's steps and skips its fills. They measure about 4 on macOS: the doubling reads a copy that the map makes first.
⍝⍝⍝ gap forced "2↣×@(65538↣↑)": Under should run its function through the selection's map in the fused pass and write the results back, as the expected cost assumes
(2×)@(n+2ₓ ↑)v
⍝⍝: 6	12	11	instructions macos 11.479 linux 5.084	allocated macos 24.443 linux 24.443

⍝⍝⍝ gap forced "2↣×@(¯1↣⌽∘(65538↣↑))": Under should run its function through the selection's map in the fused pass and write the results back, as the expected cost assumes
(2×)@(¯1⌽(n+2ₓ)↑) v
⍝⍝: 6	13	12	instructions macos 13.003 linux 6.163	allocated macos 24.579 linux 24.579

⍝ Few distinct indices take the same loop. `10ₓ|jv` gives integer positions, which a table reads in place (`structure::index_table`).
⍝ Expected: `{h←10ρ0 ◇ h.[10ₓ|jv]+←1 ◇ h}0` 3 copies: the integer residue, about 1 copy, and about 1 cycle an item for the 10 counters; `{h←10ρ0ₓ ◇ h.[10ₓ|jv]+←1ₓ ◇ h}0` 3 copies: as the float counters.
⍝ About 1 copy goes to the residue and the rest to the modified write, which combines its offsets a run at a time, as the comment on `{x←v ◇ x.[i]+←w ◇ x}0` describes.
⍝⍝⍝ held macos 7.45: the histograms' gap (open.md, "Kernels, memory and threads")
{
  h←10ρ0
  h.[10ₓ|jv]+←1
  h
}0
⍝⍝: 6	12	11	instructions macos 22.199 linux 19.469	allocated macos 3.363 linux 3.363

⍝ Its offsets combine a run at a time without a list.
⍝⍝⍝ held macos 7.93: the histograms' gap (open.md, "Kernels, memory and threads")
{
  h←10ρ0ₓ
  h.[10ₓ|jv]+←1ₓ
  h
}0
⍝⍝: 6	20	19	instructions macos 31.805 linux 31.098	allocated macos 3.384 linux 3.384

⍝ `@` with a mask changes the items or rows that the mask picks. `⍚` with a selector for each cell runs each case once, on the cells that pick it.
⍝ Expected: `√@[0≤]s` 4 copies: a tolerant comparison, about 1.5 copies, and square roots of about 2; `(0≤)⍚[0 √] s` 4 copies: as `√@[0≤]s`; `cnt⍚[- ⊢ 10×] v` 4 copies: each case on every item, about 1 copy each, and one pick; `-@[{side↑b}] m` 1.5 copies: one copy of `m`, and half its rows negated; `(side↑b)⍚[⊢ -] m` 1.5 copies: as `-@[{side↑b}] m`.
⍝⍝⍝ held macos 7.77: open.md lists the masked selects among the speed audit's outliers
√@[0≤]s
⍝⍝: 8	18	19	instructions macos 25.895 linux 22.221	allocated macos 27.213 linux 27.213

⍝⍝⍝ held macos 7.92: open.md lists the masked selects among the speed audit's outliers
(0≤)⍚[0 √] s
⍝⍝: 8	18	21	instructions macos 28.151 linux 25.038	allocated macos 38.334 linux 38.334

⍝⍝⍝ held macos 16.6: open.md lists the masked selects among the speed audit's outliers
cnt⍚[- ⊢ 10×] v
⍝⍝: 8	31	27	instructions macos 56.97 linux 49.559	allocated macos 50.911 linux 50.911

-@[{side↑b}] m
⍝⍝: 3	5	5.5	instructions macos 8.783 linux 4.903	allocated macos 14.85 linux 14.85

(side↑b)⍚[⊢ -] m
⍝⍝: 3	5.5	5.4	instructions macos 10.467 linux 6.007	allocated macos 22.859 linux 22.859

⍝ A select runs cases that can't fail on every item and keeps the result of the case each item picks, where enough items pick a case that computes.
⍝ Expected: `-@[0>]s` 2 copies: a tolerant comparison and a negation, blended; `(0>)⍚[- 2×] s` 2.5 copies: a tolerant comparison and two cases, blended.
⍝⍝⍝ held macos 4.23: open.md lists the masked selects among the speed audit's outliers
-@[0>]s
⍝⍝: 4	11	17	instructions macos 11.997 linux 8.348	allocated macos 17.254 linux 17.254

(0>)⍚[- 2×] s
⍝⍝: 5	13	20	instructions macos 15.283 linux 10.681	allocated macos 25.383 linux 25.383

⍝ An approximate divisor reads the integers as floats. Each item then needs a float division and a floor within tolerance.
⍝ Expected: 3.5 copies: a conversion, a float division of about 1 cycle an item, and a tolerant floor.
2.5|jv
⍝⍝: 7	7.2	13	instructions macos 8.447 linux 8.778	allocated macos 10.089 linux 10.089

⍝ A whole divisor that stands for an integer divides the integers as that integer, with multiplications, and the residues become 8-byte floats.
⍝ Expected: 2.5 copies: the integer residue, and the writes of 8-byte floats in place of 2-byte integers. On macOS the residue, which divides in 16-bit lanes (`integer::Short`), measures about 0.9 copies, and the second pass that converts the residues to floats about 0.6.
⍝⍝⍝ held linux 3.99: Syd1's residue and float writes cost 2.3 times the 1.75 its Linux budget allows; cause not traced
3|jv
⍝⍝: 5	3.5	5.4	instructions macos 8.482 linux 8.834	allocated macos 10.089 linux 10.089

⍝ rustymath's `exp` evaluates a polynomial of 10 terms for each item, and `ln` one of 7 terms and a division.
⍝ Expected: `*s` 5.75 copies: rustymath's f64 `Exp`, which takes each block of 64 items within ±708 through a form with no clamp and one power of two, as rustymath's `tests/speed.txt` counts it; `⊛v` 6.5 copies: rustymath's f64 `Ln`, which takes each block of 64 positive, normal, finite items through a form with no special values, as rustymath's `tests/speed.txt` counts it. `*s` was 6.5 copies until 23 October 2026, when every item took the clamp and two powers of two, and 4.5 before that, from a count of 18 that left out the clamp and building 2^n. `⊛v` was 9 copies until 23 October 2026, when every item took the full form with its special values, and 5.3 before that, from a count that left out the split, the selects and the check for NaN.
*s
⍝⍝: 12	10	36	instructions macos 23.74 linux 11.263	allocated macos 8.06 linux 8.06

⊛v
⍝⍝: 13	9.9	36	instructions macos 27.178 linux 17.199	allocated macos 8.06 linux 8.06

⍝ A dynamic square root or logarithm of a real array with negatives gives complex results. The checked real kernel fails at the first negative item, and the call runs again at the complex type (`pervasive::searched`), computing each result from its real item (`Formula::real_complex`): √|x| placed by the sign, or ln|x| with 0 or π.
⍝ Expected, as timed on 6 November 2026: `√s` 6 copies: about 4.4 instructions an item for the failed real pass and 4.5 for the complex results; `⊛s` 16 copies: about 16.4 for the failed real pass and 17 for the complex results. Wasm: twice each, without fused multiply-add. They were 35 and 115 copies while the rerun promoted each item to complex.
√s
⍝⍝: 12.5	36	41	instructions macos 10.216 linux 9.888	allocated macos 24.06 linux 24.06

⊛s
⍝⍝: 33	86	210	instructions macos 51.487 linux 34.395	allocated macos 24.06 linux 24.06

⍝ rustymath's sine and cosine reduce each item by π in three fused multiply-adds, and evaluate a polynomial of 7 terms. Tangent reduces by π/2, evaluates a polynomial of 9 terms at half the angle, and divides once.
⍝ Expected: `1○v` and `2○v` 7 copies: about 28 vector operations an item, for the range check, the reduction, the polynomial, the sign flip, the select at 0 and the test that sends rare items to the second pass; `3○v` 8 copies: about 32 vector operations an item, for the range check, the reduction, the polynomial at half the angle, the selects that pick the quotient's terms and the test, with a division that overlaps them. `1○v` and `2○v` were 4 copies, from a count of 16 that took in only the reduction and the polynomial, then 7.3 copies until sine stopped setting the sign of a zero result, which the select at 0 already gives. `3○v` was 6.3 copies, from a count of 17 that left out the range check, the selects and the test, and charged the division 2 copies.
1○v
⍝⍝: 14	12	39	instructions macos 24.112 linux 13.882	allocated macos 8.06 linux 8.06

2○v
⍝⍝: 14	12	44	instructions macos 25.486 linux 14.507	allocated macos 8.06 linux 8.06

3○v
⍝⍝: 16	17	57	instructions macos 28.361 linux 15.007	allocated macos 8.06 linux 8.06

⍝ The hyperbolic functions build on rustymath's `exp` and `expm1`, with a division. The inverse circle functions evaluate a polynomial of 10 or 12 terms after a reduction with a square root or a division.
⍝ Expected: `5○v` 10 copies: about 40 vector operations an item, for `expm1`'s 34 and the arithmetic around it, with a division that overlaps them; `7○v` 7.5 copies: rustymath's f64 `Tanh`, whose `expm1` needs no guards for |x| up to 20, as rustymath's `tests/speed.txt` counts it; `6○v` 7.25 copies: about 29 vector operations an item, for `exp`'s 26 and the arithmetic around it, with a division that overlaps them; `1○⁻v` 7 copies and `2○⁻v` 7.5 copies: about 28 and 30 vector operations an item, for the reduction, the polynomial of 12 terms, the selects that undo the reduction and the check for a NaN made from a number, with a square root that overlaps them; `3○⁻v` 7.25 copies: about 29 vector operations an item, for the selects that pick one of three intervals, the polynomial of 10 terms, the selects of the added multiple of π/4 and the sign, with a division that overlaps them. `5○v` and `7○v` were 10.5 copies until `expm1` stopped selecting -0 for a zero argument, which only set the sign of a zero result. `5○v`, `6○v` and `7○v` were 11, 6.5 and 6.5 copies before that, from `exp`'s old count of 18, two exponentials for `5○v` where it computes one `expm1`, and 2 copies for each division. `1○⁻v`, `2○⁻v` and `3○⁻v` were 6, 6 and 5.5 copies, from counts of 16, 16 and 14 that left out most of the selects and the check, and charged the square root or division 2 copies.
5○v
⍝⍝: 20	20	58	instructions macos 29.363 linux 14.887	allocated macos 8.06 linux 8.06

6○v
⍝⍝: 15	14	49	instructions macos 24.865 linux 11.889	allocated macos 8.06 linux 8.06

7○v
⍝⍝: 15	14	54	instructions macos 28.364 linux 14.637	allocated macos 8.06 linux 8.06

1○⁻v
⍝⍝: 14	16	52	instructions macos 25.481 linux 14.215	allocated macos 8.084 linux 8.084

2○⁻v
⍝⍝: 15	17	52	instructions macos 27.481 linux 15.465	allocated macos 8.084 linux 8.084

3○⁻v
⍝⍝: 15	17	51	instructions macos 23.543 linux 12.774	allocated macos 8.084 linux 8.084

⍝ The inverse hyperbolic functions build on `ln`'s reduction, after a square root or a division. `4○` is √(1 + x²), computed as rustymath's `hypot` of x and 1.
⍝ Expected: `5○⁻v` 10.75 copies: about 43 vector operations an item, for the forms near 0 and beyond 2^52, `ln`'s split, the polynomial and the selects for NaN and ∞, with a square root and a division that overlap them; `4○v` 4 copies: about 16 vector operations an item, for scaling x and 1 by the power of two that keeps x² finite and the selects for the larger and for ∞, with a square root that overlaps them. `5○⁻v` was 7.3 copies, from a square root and a `log1p` charged the old way. `4○v` was 2.5 copies, from a square root charged 2 copies and two operations, without `hypot`'s scaling.
5○⁻v
⍝⍝: 22	26	57	instructions macos 30.044 linux 17.771	allocated macos 8.084 linux 8.084

4○v
⍝⍝: 8	5.5	17	instructions macos 12.358 linux 5.634	allocated macos 8.06 linux 8.06

⍝ Monadic `○`, cis, computes each result from its real item through `Cis`'s real-argument body (`Formula::real_complex`): the cosine and sine formulas, with arguments past 2^23 taking the complex form.
⍝ Expected: 14 copies: sine's 26 and cosine's 28 vector operations, as rustymath's speed test counts them for `Sin` and `Cos`, and 16-byte writes. It measured 24 copies on 6 November 2026: the complex pairs it writes keep the loop at two lanes, where `1○v` runs four.
⍝ In wasm, CI's runner gave 47 under Node 22's V8 on 25 October 2026, and 55 to 58 under Node 26's from 28 October 2026. V8's newer x86-64 code caused the rise: under Node 26 on Apple silicon, the 0.1.75 release and a build of 29 October 2026 cost the same. Its wasm budget is set from Node 26's 57.
○v
⍝⍝: 28	37	80	instructions macos 165.738 linux 98.98	allocated macos 16.06 linux 16.06

⍝ A constant exponent that is whole and from 1 to 8 in magnitude squares and multiplies, and one of 0.5 or 1.5 starts from the square root (`rustymath::Exponent`), each in a loop of its own. Another exponent calls rustymath's `pow` for each item, which at f64 reads tables that NEON and wasm can't vectorize.
⍝ Expected: `v*2` 1 copy: one read and one write per item; `v*0.5` 2 copies: one square root of each item, as `√v` takes, which measured 1.88 on 30 October 2026; `v*w` 15 copies: a vectorised `pow` in extra precision, about twice `⊛` and `*` together. Before 30 October 2026, `v*0.5` took `pow`'s route, at 13 times `√v`'s cost.
v*2
⍝⍝: 2	2.1	2.2	instructions macos 3.308 linux 2.201	allocated macos 8.085 linux 8.085

v*0.5
⍝⍝: 4	33	82	instructions macos 12.931 linux 9.518	allocated macos 8.085 linux 8.085

v*w
⍝⍝: 30	32	82	instructions macos 90.966 linux 39.186	allocated macos 8.079 linux 8.079

⍝ Bases 2 and 10 and degrees 2 and 3 have loops of their own.
⍝ Expected: `2⊛v` and `10⊛v` 6.5 copies: as `⊛v`, through the fast form of rustymath's `Logb` for bases 2 and 10; `3√v` 9.25 copies: about 37 vector operations an item, for the step for subnormals, the first estimate from the exponent bits, three Newton steps, the last correction, the select for 0, ∞ and NaN, and the sign. `2⊛v` and `10⊛v` were 9 copies, as `⊛v` was. `3√v` was 4 copies, from a count of 15 that took in only the Newton steps.
2⊛v
⍝⍝: 13	12	36	instructions macos 20.724 linux 11.1	allocated macos 8.085 linux 8.085

10⊛v
⍝⍝: 13	11	37	instructions macos 20.725 linux 11.104	allocated macos 8.085 linux 8.085

3√v
⍝⍝: 19	18	60	instructions macos 26.548 linux 18.006	allocated macos 8.085 linux 8.085

⍝ Exact integers read as floats for a function whose results are always approximate.
⍝ Expected: `⊛jv` 7 copies: as `⊛v`, with about 2 operations to convert each integer; `*jv` 7 copies: as `*s`, with the same conversion; `jv⊛2ₓ` 10.5 copies: a conversion and a logarithm of each base, a division, and a second pass that writes the items whose base is 2 or 10 from `log2` or `log10`; `√jv` 2 copies: a conversion and a square root. `⊛jv` was 9.5 copies, as `⊛v` was 9, and `*jv` and `jv⊛2ₓ` were 4.5 and 7.3 copies, from the old count for `*s`.
⊛jv
⍝⍝: 14	12	40	instructions macos 29.427 linux 18.489	allocated macos 10.062 linux 10.062

*jv
⍝⍝: 14	15	49	instructions macos 25.986 linux 12.553	allocated macos 10.062 linux 10.062

jv⊛2ₓ
⍝⍝: 21	21	63	instructions macos 31.358 linux 18.987	allocated macos 10.089 linux 10.089

√jv
⍝⍝: 4	6.1	5.9	instructions macos 8.144 linux 6.471	allocated macos 10.062 linux 10.062

⍝ `!` calls rustymath's `gamma`, which sums its Lanczos series in double words. A distribution's density, cdf and quantile each run over the array as one slice. The quantile's `erfcinv` sorts each block of items by region, then evaluates each region's rational approximation over its items, with no iteration. `⌻` evaluates a polynomial of degree 7 by Horner's rule.
⍝ Expected: `!v` 132 copies: `Gamma`'s 530 vector operations, as rustymath's speed test counts them, for a double-word Lanczos sum and quotient, a double-word ln of the exponent, `exp_plus`, and `sinpi` for the reflection; `nd.density v` 7.8 copies: `exp`'s 26 vector operations and five more an item, with `sd`'s reciprocals computed once; `nd.cdf v` 8 copies: its inputs all lie where `erfc` takes `erfc_near`, a ratio of polynomials in x² with one division and no exponential, about 31 vector operations an item with the argument and `erfc`'s range checks; `nd.quantile v` 22.5 copies: `Erfcinv`'s 85 vector operations, 21 copies as rustymath's speed test counts them, the check that each probability lies from 0 to 1, and the scaling on each side; `coeffs⌻v` 2.75 copies: a read and a write, and seven fused multiply-adds an item. `nd.density v` was 5.5 copies, from the old count for `exp`. `!v`, `nd.quantile v` and `coeffs⌻v` were 80, 12 and 1.8 copies, from counts that left out most of `gamma`'s and `erfcinv`'s pieces, and the reads and writes.
⍝ Its wasm expected cost is 264, twice its native one, because wasm's SIMD has no fused multiply-add.
!v
⍝⍝: 200	160	540	instructions macos 409.237 linux 157.492	allocated macos 8.06 linux 8.06

nd.density v
⍝⍝: 16	15	53	instructions macos 32.649 linux 14.195	allocated macos 8.162 linux 8.162

⍝ `nd.cdf v` measured 15.8 and 16.8 copies on Linux in two CI runs of v0.1.62, with two divisions an item, and 16.5 in v0.1.64's run after its argument multiplied by the reciprocal of `sd`. x86-64 builds still divided for each item, because the closure read `sd` through a reference. Since 22 October 2026 `erfc_line` computes the reciprocal once, which took macOS from 11.1 copies to 9.65.
nd.cdf v
⍝⍝: 16	16	53	instructions macos 31.654 linux 16.195	allocated macos 16.158 linux 16.158

⍝ `nd.quantile v` measures 31 on macOS, where `Erfcinv`'s loop takes 86% of it and retires about 91 instructions an item, nearly all in its rational approximations. Since 22 October 2026 `regions` computes each item's arguments once, which took about 5 instructions an item off. Its wasm expected cost is 45, twice its native one, because wasm's SIMD has no fused multiply-add.
nd.quantile v
⍝⍝: 45	67	100	instructions macos 119.019 linux 91.713	allocated macos 16.162 linux 16.162

⍝ The log-normal density takes ln x from `Ln`'s slice form, then the normal density at it over x. The log-normal cdf and quantile run the normal slice forms at ln x, the quantile through an `exp`.
⍝ Expected: `lg.density v` 18 copies: `Ln`'s 36 vector operations, the normal density's 31, and a clamp, a division and a select, about 72 an item; `lg.cdf v` 20 copies: `ln` and the argument, about 38 operations, `erfc`'s mixed route, about 33, and 7 for its range checks and the result; `lg.quantile v` 29 copies: `nd.quantile v`'s expected 22.5, and an `exp` of 26 operations.
⍝ Wasm: twice each native cost, so `lg.density v` 36 copies. Wasm's SIMD has no fused multiply-add, so `FUSED` is false there, and each `mla` in `ln`, `exp`, `erfc` and the normal formulas is a multiply and an add. `nd.density v` costs 16.2 in wasm against 9.8 natively, and `lg.density v` 35.3 against 22.7.
lg.density v
⍝⍝: 36	28	95	instructions macos 51.081 linux 26.265	allocated macos 16.162 linux 16.162

⍝ `nd.cdf ⊛v` alone costs 35 on macOS.
⍝⍝⍝ held macos 39.3: its `erfc` arguments, -ln v/√2, span `ERFC_SPLIT`, so every block takes `erfc`'s mixed route, which on AArch64 picks `erfc_near` or `erfc_far` with a branch for each item
lg.cdf v
⍝⍝: 40	81	130	instructions macos 105.753 linux 77.047	allocated macos 16.158 linux 16.158

⍝ `lg.quantile v` measures 39 on macOS and 41 on Linux.
lg.quantile v
⍝⍝: 58	74	140	instructions macos 140.719 linux 101.154	allocated macos 16.162 linux 16.162

⍝ The other closed-form laws compute at their arguments' width. A law's logarithm, arctangent, tangent, power or root comes from rustymath's slice form of `Log1p`, `Ln`, `Atan`, `Tan`, `Pow` or `Root`, a block at a time, between a pass that writes the block's arguments and a pass that finishes its results and checks them for NaN. The uniform and Laplace cdfs are single loops.
⍝ Expected: `ud.cdf v` 2.5 copies: a division of about 1 cycle an item, which the subtraction and the clamp's compares and selects overlap; `ex.quantile v` 13.25 copies: `Log1p`'s 40 vector operations, a pass that negates each item, and a pass of about 9 for the range check, the scaling and the NaN check; `ca.cdf v` 12.5 copies: `Atan`'s 29 operations, as `3○⁻v` counts them, a pass of about 6 with a division that picks z or -1/z, and a pass of about 13 that rebuilds the cdf and checks it; `ca.quantile v` 13.5 copies: `Tan`'s 32 operations, as `3○v` counts them, a pass of about 7 that picks the argument's region, and a pass of about 15 with a division that rebuilds the quantile and checks it; `lp.cdf v` 9.25 copies: about 37 operations in one loop, for the full `exp` of -|z|, the select between the two halves and the NaN check; `lp.quantile v` 9.25 copies: `Ln`'s 26 operations, a pass of about 4 and a pass of about 7; `lo.quantile v` 10 copies: `Ln`'s 26 operations, a pass whose division of about 1 cycle an item forms the odds, and a pass of about 4; `wb.cdf v` 24.5 copies: `Pow`'s 15 copies, as `v*w` expects them, a pass of 3 operations, and a pass of about 34 for the full `expm1` and the NaN check; `wb.quantile v` 28.5 copies: `Log1p`'s 10 copies, a pass that negates its results, `Root`'s cost taken as `Pow`'s 15, and two short passes.
⍝ macOS: at f64 `Pow` and `Root` read tables that NEON can't gather, and `v*w` measures about 28 copies on this Mac against its expected 15. With that cost, `wb.cdf v` and `wb.quantile v` expect 37.5 and 41.5 copies on macOS, and measured 36 and 37 on 30 October 2026. Wasm: twice each native cost, because wasm's SIMD has no fused multiply-add, as for `lg.density v`, and for the Weibull cases twice the macOS cost, 75 and 83, because wasm can't gather either. CI measured them at 67 to 71 and 71 to 73 in wasm in runs 56 to 62. `ud.cdf v` multiplies and adds nothing, so its wasm budget is its native one.
ud.cdf v
⍝⍝: 5	4.7	6.5	instructions macos 8.76 linux 5.176	allocated macos 8.158 linux 8.158

ex.quantile v
⍝⍝: 27	25	56	instructions macos 35.1 linux 19.984	allocated macos 16.162 linux 16.162

ca.cdf v
⍝⍝: 25	25	64	instructions macos 36.86 linux 21.518	allocated macos 16.158 linux 16.158

ca.quantile v
⍝⍝: 27	28	75	instructions macos 49.577 linux 24.379	allocated macos 16.162 linux 16.162

lp.cdf v
⍝⍝: 19	17	54	instructions macos 29.659 linux 15.569	allocated macos 8.158 linux 8.158

lp.quantile v
⍝⍝: 19	18	47	instructions macos 31.185 linux 18.12	allocated macos 16.162 linux 16.162

lo.quantile v
⍝⍝: 20	18	46	instructions macos 28.21 linux 17.109	allocated macos 16.162 linux 16.162

wb.cdf v
⍝⍝: 75	51	110	instructions macos 37.135 linux 19.097	allocated macos 16.158 linux 16.158

wb.quantile v
⍝⍝: 83	59	110	instructions macos 115.899 linux 56.576	allocated macos 16.162 linux 16.162

⍝ `coeffs⌻v` measures 3.46 on macOS. Wasm: 4.5 copies, because wasm's SIMD has no fused multiply-add, so each of the seven is a multiply and an add.
coeffs⌻v
⍝⍝: 5.5	2.6	5.6	instructions macos 15.346 linux 5.444	allocated macos 8.055 linux 8.055

⍝ Each item goes into a hash table, and each item of the other argument is looked up in it. A tolerant search looks in two buckets for each item, where an exact search looks in one.
⍝ Expected: 60 copies: about 30 cycles an item to insert and look up in a hash table. Linux and wasm, which runs on the same runner: 125 copies. Scalar work costs about 1.7 times as many copies on CI's x86 runner as on macOS, as `fib 16` and the exact `vιw` show. The tolerant table and its chains, about 2.9 MB, also overflow that runner's 1 MB L2, where macOS's L2 holds them, which adds about a quarter. CI measured 121 to 133.
vιw
⍝⍝: 90	95	150	instructions macos 92.847 linux 200.737	allocated macos 64.032 linux 52.033

⍝ A radix sort moves every item once for each byte that differs between the keys. Keys that span a small range, as `jv`'s and `b`'s do, take a counting sort, which places each item once.
⍝ Expected: `⍋v` 50 copies: a pass that counts each byte of every key, about 14, then a pass for each of the 8 bytes, at about 4.5 copies each. The count pass makes 8 counts for each key, each a load, an increment and a store, at about one a cycle. A byte pass moves each key and its 4-byte position to the next place in its byte's bucket. Nearly every key of `v` shares its top byte, which holds a float's sign and the top 7 bits of its exponent. That byte's pass moves each block of 64 keys that share it with one copy, at about the cost of another pass. `⍋jv` 6 copies: a counting sort, with a histogram pass and one scattered write of 4-byte positions. The histogram costs about 2 cycles an item, because each count is a load, an increment and a store that the next item with the same key waits for, where a copy costs about half a cycle an item. Linux: `⍋jv` 10 copies, because scalar work costs about 1.7 times as many copies on CI's x86 runner, as `vιw`'s comment says; CI measured 10.24. `⍋b` 1 copy: two passes writing 4-byte positions.
⍝ Linux and wasm: `⍋v` keeps 1.5 times its costs on 29 October 2026, 82.6 and 63.7, until CI times the block pass.
⍋v
⍝⍝: 75	130	110	instructions macos 165.552 linux 166.661	allocated macos 26.042 linux 26.042

⍋jv
⍝⍝: 12	15	12	instructions macos 25.643 linux 23.318	allocated macos 8.776 linux 8.776

⍋b
⍝⍝: 2	4.2	5.2	instructions macos 8.519 linux 7.279	allocated macos 8.044 linux 8.044

⍝ A sort of keys that span a small range counts them, then writes each key as many times as it occurs. It needs no positions.
⍝ Expected: `<jv` 3.5 copies: a histogram pass, at about 2 cycles an item as for `⍋jv`, then each key written as many times as it occurs; `<b` 0.25 copies: counts, then writes.
<jv
⍝⍝: 7	5.2	6.7	instructions macos 16.393 linux 13.875	allocated macos 2.52 linux 2.52

<b
⍝⍝: 0.5	1.9	1.9	instructions macos 2.046 linux 1.302	allocated macos 1.033 linux 1.033

⍝ Sorting under `∂` grades the primal once, then reads the primal and the tangent at the positions the grade gives.
⍝ Expected: 56 copies: a grade, about 50, and reads of the primal and the tangent at its positions, about 3 each.
{+/ ι≢ω × <ω}∂ v
⍝⍝: 84	190	170	instructions macos 226.991 linux 218.018	allocated macos 79.548 linux 79.546

⍝ Taking a few items of a grade or a sort selects them, without sorting the rest. `search::view_selected` reads the vector once. A block of 64 items with no item below the greatest one kept so far costs a compare and an OR for each vector.
⍝ Expected: `↑∘< v` 1 copy: that pass, at two instructions for each vector where `⌊/v`'s minimum takes one. `(10↑)∘⍋ v` and `(10↑)∘< v` 1.5 copies: the pass, and the blocks that hold one of the roughly 90 items that pass the bound on the way, which read item by item.
(10↑)∘⍋ v
⍝⍝: 3	2.2	2.4	instructions macos 11.857 linux 9.845	allocated macos 0.366 linux 0.366

(10↑)∘< v
⍝⍝: 3	2.2	2.6	instructions macos 13.036 linux 10.703	allocated macos 0.476 linux 0.476

↑∘< v
⍝⍝: 2	1.4	1.8	instructions macos 7.594 linux 5.077	allocated macos 0.333 linux 0.333

⍝ The ranks `⍋⍋` invert the first grade instead of grading it again.
⍝ Expected: 52 copies: a grade, about 50, and one scattered write to invert it.
⍝ Linux and wasm keep 1.5 times their costs on 29 October 2026, 85.2 and 68.2, until CI times the grade's block pass.
⍋⍋ v
⍝⍝: 78	130	110	instructions macos 177.574 linux 177.94	allocated macos 32.233 linux 32.233

⍝ Unique sorts the reals, then compares each with the next one.
⍝ Expected: 58 copies: a sort of the order keys with their positions, about 50 as for `⍋v`, then about 8 for the pass that builds the keys, a walk that compares each sorted key with the next within tolerance and marks the first position of each run, and a compression.
⍝ Linux and wasm keep 1.5 times their costs on 29 October 2026, 100 and 84.6, until CI times the sort's block pass.
∪v
⍝⍝: 87	150	130	instructions macos 75.29 linux 234.546	allocated macos 55.041 linux 41.04

⍝ Rank calls a fold, a structural function or a fold of a ravel once on the whole stack, along the cells' axes, rather than once for each cell.
⍝ Expected: `+/⍤1 t` and `(+/,)⍤2 ms` 0.5 copies: one pass reads 8 bytes an item and writes nothing; `(1 1↣⌷)⍤2 ms` 0.02 copies: reads one item of each cell; `⌽₀⍤2 ms` and `⌽⍤1 ms` 1 copy: one read and one write per item.
⍝ `+/⍤1 t` measures 0.71 on macOS: float sums fold rows of 16 to 63 items eight rows side by side (`Reduction::rows` in rustymath), where a call of `slice` for each row cost about 4.5 ns.
+/⍤1 t
⍝⍝: 1	1.3	1.2	instructions macos 5.457 linux 3.845	allocated macos 0.788 linux 0.788

⍝ `(+/,)⍤2 ms` folds through `in_turn`, which folds the last axis's 8-item lanes, then the next axis's, at the cost of a second pass and the totals kept between the passes. One pass over each cell's 64 items measured 1.11 on macOS, because of the same gap as `+/⍤1 t`: the call and setup for each row.
(+/,)⍤2 ms
⍝⍝: 1	1.1	1.4	instructions macos 6.928 linux 4.427	allocated macos 0.476 linux 0.476

(1 1↣⌷)⍤2 ms
⍝⍝: 0.5	0.5	0.54	instructions macos 3.621 linux 2.494	allocated macos 0.434 linux 0.434

⍝ `⌽₀⍤2 ms` measures 1.45 on macOS since short cells copy each run of adjacent items as one block (22 October 2026), down from 2.65 to 3.55. In wasm it measured 1.14 to 1.25 in seven CI runs from 22 October 2026. CI measured 1.08 on Linux on 22 October 2026.
⌽₀⍤2 ms
⍝⍝: 2	2.2	2.9	instructions macos 4.282 linux 2.661	allocated macos 8.135 linux 8.135

⌽⍤1 ms
⍝⍝: 2	2	2.9	instructions macos 3.741 linux 2.207	allocated macos 8.126 linux 8.126

⍝ The inner products in `rustymath` run `+.×`, `⌈.+` and `⌊.+` on floats and integers, `+.×` on complex numbers, Boolean `∨.∧` and `∧.∨`, and the counts `+.=` and `+.≠`. `rustymath`'s own speed test times those products. Each case here checks that basedpl sends one kind of pair to them. A pair that misses them takes the bounded generic path, several times slower. A case with `⍤2` makes one call for each of 1024 matrices of 8×8.
⍝ Expected: `+.×⍤2⍨ ms` 4 copies: one stacked call of 16 flops an item, about 2 copies, plus the stack's overhead; `m+.×m` 8.8 copies on macOS, 64 on Linux and 128 in wasm: 512 flops an item, at Accelerate's rate for large square products, as rustymath's `linalg::PlusTimes` expects, 8 in a copy's time for rustymath's own kernels and 4 in wasm; `m⌈.+m` 80 copies, and 160 in wasm: 512 operations an item, at about 8 in a copy's time natively and 4 in wasm; `bm∨.∧bm` and `bm∧.∨bm` 2 copies: bit-packed, about 4 word operations an item; `jm+.×jm` 32 copies: 512 integer operations an item; `jm+.=jm` and `cm+.=⍉cm` 32 copies: 256 vectorised comparisons and counts an item; `zm+.×zm` 5.9 copies on macOS, 32 on Linux and 64 in wasm: 256 real flops an item, at Accelerate's rate for complex 128×128 products, which counts complex copies, each taking 2.1 times as long as one of the `n` floats, 8 in a copy's time for rustymath's own kernels and 4 in wasm.
⍝⍝⍝ held macos 9.35: open.md's "Speed budgets" lists the gap of stacked products of 4×4 cells: the call that locates and checks each cell, and zeroing the output
+.×⍤2⍨ ms
⍝⍝: 8	28	30	instructions macos 42.399 linux 61.442	allocated macos 9.18 linux 29.18

m+.×m
⍝⍝: 18	79	140	instructions macos 15.951 linux 85.509	allocated macos 8.902 linux 25.152

⍝ The same product in index notation. The inner product along `ic` runs as one matrix product, and never builds the 16,777,216 products.
⍝ Expected: 8.8 copies on macOS, 64 on Linux and 128 in wasm, as `m+.×m`, and the call's fixed cost.
m.[ir ic]+.×⍠ic⁝m.[ic id]
⍝⍝: 18	84	140	instructions macos 40.124 linux 105.904	allocated macos 26.748 linux 42.998

⍝ On 29 October 2026 `m⌈.+m` cost 93 on Linux.
⍝⍝⍝ held macos 143: the cause isn't known yet
m⌈.+m
⍝⍝: 120	95	140	instructions macos 198.66 linux 118.032	allocated macos 25.075 linux 24.825

⍝⍝⍝ held macos 5.44: the cause isn't known yet
bm∨.∧bm
⍝⍝: 4	8.6	28	instructions macos 28.493 linux 25.252	allocated macos 2.075 linux 2.075

⍝⍝⍝ held macos 5.85: the cause isn't known yet
bm∧.∨bm
⍝⍝: 4	9.4	29	instructions macos 29.748 linux 26.926	allocated macos 2.075 linux 2.075

jm+.×jm
⍝⍝: 64	79	170	instructions macos 31.103 linux 99.101	allocated macos 52.865 linux 69.115

⍝ Expected: `um+.×um` 85 copies on macOS, 260 on Linux and 515 in wasm. It multiplies 512×512 `u8` matrices with wrapping steps through rustymath's `Wrapped<PlusTimes>`, 4,096 operations an item, whose sums pass what `f32` holds exactly. On macOS they run in Accelerate's `f64` product at about 58 flops in a copy's time, as `m+.×m` does, 70 copies, after converting both operands to `f64`, 8, and before converting the results to `i32`, 4, then wrapping them to `u8`, 3. Elsewhere they run on rustymath's `i32` kernel, which converts the bytes as it packs them, at about 16 integer operations in a copy's time on AVX2's eight 32-bit lanes and 8 on wasm's four, 256 and 512 copies, then wrap to `u8`, 3. On 25 October 2026 it measured 94 on macOS, in runs where `m+.×m` measured 10.2. With `--native` it measured 428: the `i32` kernel does about 10 operations in a copy's time on NEON's four lanes.
um+.×um
⍝⍝: 130	560	660	instructions macos 47.178 linux 320.254	allocated macos 77.238 linux 101.488

⍝ Expected: `umₓ₄+.×umₓ₄` costs what `um+.×um` costs. Each `ₓ₄` leaves its conversion to `i32` pending, and the product reads the `u8` matrices through both conversions (`pervasive::widened_product`) on the same kernels, so no `i32` copy is stored. On 25 October 2026 it measured 107 on macOS, where `um+.×um` measured 102, and 409 on the build from before, which stored both copies and ran rustymath's `i32` kernel.
umₓ₄+.×umₓ₄
⍝⍝: 130	570	750	instructions macos 58.668 linux 329.804	allocated macos 97.205 linux 121.448

jm+.=jm
⍝⍝: 64	59	74	instructions macos 69.218 linux 45.498	allocated macos 35.885 linux 35.823

⍝⍝⍝ allows forced "+.=": an inner product reads its arguments whole
cm+.=⍉cm
⍝⍝: 64	67	79	instructions macos 71.909 linux 50.528	allocated macos 39.968 linux 39.906

zm+.×zm
⍝⍝: 12	55	77	instructions macos 12.023 linux 60.44	allocated macos 4.902 linux 41.152

⍝ Matrix division, `•decompose`, `•decompose⁻` and the determinant `-.×` run on rustymath. Its own speed test times each factorisation on a 256×256 matrix, the size of `m`, and least squares on a 4096×16 matrix, the size of `t`. On macOS, Accelerate runs matrix products and triangular solves, and the SVD, the Hermitian eigendecomposition and the real nonsymmetric eigendecomposition from the sizes that rustymath's DEV.md gives. rustymath's own code runs LU, Householder QR, Cholesky, the complex nonsymmetric eigendecomposition and the smaller matrices' decompositions, with their block products and triangular solves on Accelerate. On Linux it runs all of them. These cases time basedpl's part on top of each factorisation: reading the arguments, writing the results, the calls in a `⍤2` stack, each path of `⌹`, the route from monadic `-.×` to LU, and rebuilding a matrix from factors. `tq` is tall enough for `⌹` and `"qr"` to take the CholeskyQR2 path.
⍝ Expected, taking each factorisation's cost from rustymath's speed test on the same platform: `⌹⍤2 ms` 30 copies: 1,024 inverses of 8×8, each about 1,000 cycles of small-matrix work; `ms⌹⍤2 sps` 45 copies: 1,024 solves of 8×8 with 8 right sides, about 1.5 times the inverses; `⌹m` 42 copies on macOS and 133 on Linux: `linalg::Lu::inverse`, 40 and 131, then a read and a write; `(side↑w)⌹m` 25 copies on macOS and 52 on Linux: `linalg::Lu::solve`, 24 and 51, then a read; `((n÷16)↑w)⌹t` 14 copies on macOS: `linalg::lstsq` on the same shape, 13, then a read; `⌹t` 16 copies on macOS: the same tall QR, about 13, then two products with 16 columns and a write; `((n÷4)↑w)⌹tq` 29 copies on macOS: CholeskyQR2 forms two Gram matrices of `tq`'s 24 columns, 288 flops an item each, at about 40 flops in a copy's time, 7.2 each, which Accelerate measured for this Gram product, far above its tall thin Gram rate, whose 4096×16 product spends most of its time on a fixed cost, then solves with a 24×24 triangle, 144 flops an item at the rank-k rate, 4.8, copies `tq`, which holds 6 copies of data, 6, and scans `tq` and multiplies it by `b`, about 3.5; `"qr"•decompose tq` 38 copies on macOS: the same, with a second triangular solve for Q, 4.8, in place of the product with `b`, then writing Q, 6; `"qr"•decompose⍤2 ms` 60 copies: 1,024 QRs of 8×8, each about 2,000 cycles of small-matrix work; `"qr"•decompose m` 56 copies on macOS and 194 on Linux: `linalg::qr`, 54 and 192, then writing Q and R; `"cholesky"•decompose⍤2 sps` 30 copies: 1,024 Cholesky factors of 8×8, each about 1,000 cycles; `"svd"•decompose⍤2 ms` 400 copies: 1,024 iterative SVDs of 8×8, each about 10,000 operations; `"svd"•decompose⁻ f` 13 copies on macOS and 68 on Linux: a 256×256 product, which costs what `m+.×m` costs, 8.8 and 64, and four passes that scale U by `s`, transpose V, convert the product and write it; `["kind":"svd" "rank":8]•decompose m` 22 copies on macOS and 66 on Linux: Halko's range finder, with 18 columns and two power iterations, makes six products of 256×256 by 18 columns, 36 flops an item each, five thin QRs and the SVD of an 18×256 matrix, which on Accelerate cost 5.4 for the three products of `m` by 256×18, at about 20 flops in a copy's time, measured for that shape, between the matrix-vector and rank-k rates, 10.3 for the two of `m`'s transpose at the matrix-vector rate, 1.2 for Qᴴ times `m` at the rank-k rate, and 4.6 for the QRs and the SVD, about 32 flops an item at the matrix-vector rate, while products this thin run at about 3.5 flops in a copy's time on rustymath's own kernels, which cost 66 copies on macOS with `--native`; `"eigen"•decompose⁻ h` 54 copies on macOS and 200 on Linux: `linalg::Lu::inverse`, 40 and 131, three passes for the vectors' condition number, scaling the vectors by the values, then a 256×256 product, 8.8 and 64, and a write; `"eigen"•decompose⍤2 ss` 300 copies: 1,024 symmetric eigendecompositions of 8×8, each about 5,000 operations; `"eigen"•decompose⍤2 ms` 700 copies: 1,024 nonsymmetric eigendecompositions of 8×8, each about 13,000 operations; `⌹zm` 21 copies on macOS and 67 on Linux: a complex 128×128 inverse, 8n³/3 complex multiply-adds, about 341 real ones an item, at the rate `linalg::Lu::inverse` reaches on a 256×256 matrix, about 17 and 5.2 in a copy's time, then a read and a write; `-.×m` 24 copies on macOS and 51 on Linux: the LU in `linalg::Lu::solve`, whose solve costs under a copy; `-.×⍤2 ms` 20 copies: 1,024 LUs of 8×8. In wasm, which rustymath's speed test doesn't time, the expected costs stay the flop counts at about 4 in a copy's time: `⌹m` 128, `(side↑w)⌹m` 42, `((n÷16)↑w)⌹t` 8, `⌹t` 12, `((n÷4)↑w)⌹tq` 95, `"qr"•decompose tq` 130, `"qr"•decompose m` 170, `"svd"•decompose⁻ f` 128, `"eigen"•decompose⁻ h` 128, `⌹zm` 86 and `-.×m` 42.
⌹⍤2 ms
⍝⍝: 60	88	100	instructions macos 108.664 linux 172.735	allocated macos 8.171 linux 8.483

ms⌹⍤2 sps
⍝⍝: 90	76	99	instructions macos 169.092 linux 206.351	allocated macos 23.214 linux 23.371

⌹m
⍝⍝: 84	140	210	instructions macos 56.032 linux 195.551	allocated macos 19.333 linux 58.489

side↑w ⌹ m
⍝⍝: 50	73	79	instructions macos 46.318 linux 84.593	allocated macos 11.672 linux 25.047

⍝ `((n÷16)↑w)⌹t` takes `linalg::lstsq`'s CholeskyQR2 route on macOS, and Householder on rustymath's own kernels on Linux and in wasm, where `tall_for_qr` gives 16-column matrices to Householder since 23 October 2026.
(n÷16 ↑ w)⌹t
⍝⍝: 28	46	53	instructions macos 39.225 linux 44.126	allocated macos 11.281 linux 19.348

⍝ `⌹t` forms U from the tall QR's factors and then multiplies it out. Taking the pseudoinverse as R's pseudoinverse times Q's transpose would save about 10 copies.
⍝⍝⍝ held macos 35.3: it forms U and multiplies it out, as above; no Linux expected cost yet for rustymath's own kernels, which take the wasm expected cost here
⌹t
⍝⍝: 32	100	88	instructions macos 66.232 linux 117.338	allocated macos 42.622 linux 69.75

⍝ On macOS, Accelerate runs most of `((n÷4)↑w)⌹tq`, as it does `"qr"•decompose tq` below, at about half the rates above, so its macOS expected cost is the 53 it measures.
(n÷4 ↑ w)⌹tq
⍝⍝: 80	270	300	instructions macos 133.807 linux 415.253	allocated macos 55.239 linux 223.668

⍝ On macOS, Accelerate's Gram products and triangular solves take about four fifths of `"qr"•decompose tq`'s time and our own code under 1% (`profile`, 29 October 2026). They run at about half the rates above, so its macOS expected cost is the 72 it measures.
"qr"•decompose tq
⍝⍝: 110	390	410	instructions macos 90.22 linux 501.105	allocated macos 99.307 linux 322.479

⍝ `"qr"•decompose⍤2 ms` measured about 126 on macOS and 225 on CI before 22 October 2026, about twice its expected cost. Each 8×8 cell retires about 24,000 instructions. `leaf_qr`'s reflections take about 40% of them, two `Zip` adapters that the test profile leaves out of line about 12%, and allocations about 6%, most of them basedpl's, which builds a Q and an R array for each cell of the nested result. Since 22 October 2026 a stack's cells factor in one reused `QrWork` and write into the two outputs directly, which cut the case's instructions by 12%. On 23 October 2026 it measures 46 on macOS, and CI's last two runs measured 65 and 68 on Linux and 61 and 68 in wasm. Every budget is twice its expected cost.
"qr"•decompose⍤2 ms
⍝⍝: 90	89	77	instructions macos 186.167 linux 220.656	allocated macos 21.567 linux 21.833

"qr"•decompose m
⍝⍝: 84	210	300	instructions macos 101.284 linux 306.275	allocated macos 71.682 linux 152.089

"cholesky"•decompose⍤2 sps
⍝⍝: 60	51	45	instructions macos 97.146 linux 104.193	allocated macos 26.845 linux 26.846

⍝ `"svd"•decompose⍤2 ms` runs rustymath's own SVD on every platform, because 8×8 lies below rustymath's `GESDD_FROM`. On macOS it measures about 509, against 775 on Accelerate's `?gesdd`, and CI measured 608.
"svd"•decompose⍤2 ms
⍝⍝: 600	430	390	instructions macos 970.586 linux 914.832	allocated macos 114.273 linux 114.274

⍝ `["kind":"svd" "rank":8]•decompose m` in wasm: 82 copies, as CI's x86 V8 measured it on 22 October 2026, at 15.7 ns an item against a copy of 0.192 ns. That's about 2.8 flops in a copy's time, where the group's wasm rate of 4 would give 58. Thin products take 70% of its instructions on rustymath's own kernels, at 70 to 80% of a square product's flops per instruction. CI measured 79 to 92 in seven runs from 22 October 2026. This Mac's V8 takes about the same time, 14.3 ns an item, against a copy 1.6 times faster than CI's, and measures 118 to 121 copies, up to 153 under load.
⍝ CI measured 74.5 to 83.5 in wasm in runs 52 to 62.
["kind":"svd" "rank":8]•decompose m
⍝⍝: 44	84	120	instructions macos 124.491 linux 247.441	allocated macos 29.457 linux 140.483

⍝ `"eigen"•decompose⍤2 ss` runs rustymath's own Hermitian solver on every platform, because 8×8 lies below rustymath's `HEEVD_FROM`. On macOS it measures about 298, against 392 on Accelerate's `?heevd`.
⍝ CI measured 241 to 252 on Linux in runs 56 to 61, and 442 in run 62, whose host was busy.
"eigen"•decompose⍤2 ss
⍝⍝: 450	270	250	instructions macos 583.994 linux 557.885	allocated macos 48.392 linux 48.393

"eigen"•decompose⍤2 ms
⍝⍝: 910	670	720	instructions macos 1373.757 linux 1422.279	allocated macos 110.783 linux 130.784

⍝ `⌹zm` measures 28.4 on macOS and 94 on CI, about 1.35 times its expected cost on both. Its complex inverse costs 3.4 to 4 times a real one of the same size, as its four real multiply-adds for each complex one predict, and less than a solve with as many right-hand sides. A 128×128 inverse runs at about 0.7 of the rate a 256×256 one reaches, on Accelerate and on rustymath's own kernels alike.
⌹zm
⍝⍝: 42	96	130	instructions macos 38.25 linux 172.24	allocated macos 9.396 linux 56.833

-.×m
⍝⍝: 48	72	76	instructions macos 39.151 linux 80.984	allocated macos 11.395 linux 24.77

-.×⍤2 ms
⍝⍝: 40	48	46	instructions macos 77.051 linux 82.107	allocated macos 0.366 linux 0.522

⍝⍝ exact •prefs ["mathish":$f]
⍝ Plain IEEE comparisons and `floor`, with nothing to scale by a tolerance.
⍝ Expected: `⌊v` 1.5 copies: a floor, a conversion to a 64-bit integer, a range check, and a select and a success flag for an item that doesn't fit, about 6 vector operations an item, with an 8-byte write; `v<w` and `v=w` 1.1 copies: two 8-byte reads and a 1-byte write; `3|v` 2.5 copies: a float division of about 1 cycle an item and a floor. `⌊v` was 0.75 copies, from a narrow write and no conversion.
⍝ In wasm, CI's runner gave `⌊v` 2.34, `v<w` 1.87 and `v=w` 1.85 under Node 22's V8 on 25 October 2026, and 3.29 to 3.48, 2.43 to 2.49 and 2.43 to 2.47 under Node 26's from 28 October 2026. V8's newer x86-64 code caused the rise: under Node 26 on Apple silicon, the 0.1.75 release and a build of 29 October 2026 cost the same. Their wasm budgets are set from Node 26's costs.
⌊v
⍝⍝: 3	3.5	6.1	instructions macos 8.329 linux 6.005	allocated macos 1.06 linux 1.06

v<w
⍝⍝: 2.2	2.5	4.1	instructions macos 3.71 linux 3.122	allocated macos 1.079 linux 1.079

v=w
⍝⍝: 2.2	2.5	4.1	instructions macos 3.703 linux 3.116	allocated macos 1.079 linux 1.079

3|v
⍝⍝: 5	3.5	4	instructions macos 6.231 linux 4.2	allocated macos 8.085 linux 8.085

⍝ Each real indexes its `order_key` in a hash map, as exact data does.
⍝ Expected: `vιw` 60 copies: about 30 cycles an item to insert and look up in a hash table; `∪v` 30 copies: one hash insert an item.
⍝ Linux and wasm: exact `∪v` 50 copies. Scalar hash work costs about 1.7 times as many copies on CI's Zen 4 as here, as for `vιw` above, and CI measured 47 to 51. On wasm32, hashbrown probes 4-byte control groups (its generic `u32` word, where NEON gives 8 and SSE2 16), and foldhash, without wasm's `wide-arithmetic`, approximates each 64-by-64-bit multiply with four 32-bit ones; CI's x86 V8 measured 46 to 48, and this Mac's V8 42 to 51.
vιw
⍝⍝: 90	74	95	instructions macos 92.6 linux 99.622	allocated macos 64.032 linux 64.033

∪v
⍝⍝: 60	66	69	instructions macos 75.29 linux 88.062	allocated macos 55.041 linux 55.041

⍝⍝ widths •prefs ["float":32]
⍝ Half the bytes to read and write, and twice the lanes in each SIMD instruction.
⍝ Expected: `-v`, `v+w`, `v×2`, `√v`, `+⌿m`, `+/v`, `⌈/v` and `m+.×m` 0.5 of the 64-bit time: half the bytes, and twice the lanes; `v<w` 0.53 of the 64-bit time: half the bytes read, the same 1-byte writes; `0.5<v` 0.56 of the 64-bit time: half the bytes read, the same 1-byte writes; `+\v` 0.75 of the 64-bit time: twice the lanes, but the same latency for each waiting addition.
-v
⍝⍝: 0.75	0.71	0.7	instructions macos 2.381 linux 1.352	allocated macos 4.06 linux 4.06

v+w
⍝⍝: 0.75	0.49	0.53	instructions macos 2.724 linux 1.652	allocated macos 4.079 linux 4.079

v×2
⍝⍝: 0.75	0.67	0.65	instructions macos 2.7 linux 1.613	allocated macos 4.085 linux 4.085

v<w
⍝⍝: 0.78	0.62	0.76	instructions macos 2.762 linux 1.936	allocated macos 1.079 linux 1.079

0.5<v
⍝⍝: 0.81	0.59	0.65	instructions macos 2.706 linux 1.9	allocated macos 1.085 linux 1.085

√v
⍝⍝: 0.75	0.52	0.54	instructions macos 3.817 linux 2.696	allocated macos 4.06 linux 4.06

+⌿m
⍝⍝: 0.75	1	0.9	instructions macos 4.241 linux 2.664	allocated macos 0.273 linux 0.273

+/v
⍝⍝: 0.75	0.8	0.87	instructions macos 3.29 linux 1.982	allocated macos 0.171 linux 0.171

⌈/v
⍝⍝: 0.75	0.82	0.86	instructions macos 3.232 linux 2.108	allocated macos 0.171 linux 0.171

+\v
⍝⍝: 1	0.99	0.81	instructions macos 4.359 linux 5.156	allocated macos 4.091 linux 4.091

m+.×m
⍝⍝: 0.75	0.72	0.76	instructions macos 10.034 linux 47.224	allocated macos 4.902 linux 13.027

⍝ `⌊` reads half the bytes and writes 32-bit integers in place of 64-bit ones.
⍝ Expected: 0.83 of the 64-bit time: the 32-bit session floors within tolerance, about 10 vector operations an item on four lanes, where `[exact]` floors exactly, about 6 on two lanes. It was 0.5, from a count of bytes, which this loop's arithmetic outweighs.
⌊v
⍝⍝: 1.08	0.74	0.78	instructions macos 4.272 linux 3.637	allocated macos 1.06 linux 1.06

⍝ An LU decomposition in 32 bits.
⍝ Expected: 0.5 of the 64-bit time: half the bytes, and twice the lanes.
⌹m
⍝⍝: 0.75	0.88	0.83	instructions macos 39.258 linux 135.097	allocated macos 9.771 linux 29.349

⍝ rustymath's `exp` and `ln` run shorter polynomials at f32, with twice the lanes in each SIMD instruction.
⍝ Expected: 0.4 of the 64-bit time each: shorter polynomials and twice the lanes.
*s
⍝⍝: 0.65	0.47	0.54	instructions macos 10.267 linux 5.02	allocated macos 4.06 linux 4.06

⊛v
⍝⍝: 0.65	0.63	0.55	instructions macos 15.455 linux 7.613	allocated macos 4.06 linux 4.06

⍝ rustymath's sine and cosine reduce in f32 with fused multiply-adds, and run a polynomial of 4 terms in place of 7.
⍝ Expected: `1○v`, `2○v`, `3○v`, `7○v`, `1○⁻v` and `3○⁻v` 0.4 of the 64-bit time: 4 polynomial terms in place of 7, and twice the lanes; `v*2` and `v*w` 0.5 of the 64-bit time: half the bytes, and twice the lanes.
1○v
⍝⍝: 0.65	0.71	0.75	instructions macos 11.389 linux 6.707	allocated macos 4.06 linux 4.06

2○v
⍝⍝: 0.65	0.71	0.75	instructions macos 12.077 linux 7.145	allocated macos 4.06 linux 4.06

3○v
⍝⍝: 0.65	0.67	0.7	instructions macos 12.263 linux 6.77	allocated macos 4.06 linux 4.06

7○v
⍝⍝: 0.65	0.58	0.54	instructions macos 12.641 linux 6.649	allocated macos 4.06 linux 4.06

1○⁻v
⍝⍝: 0.65	0.58	0.49	instructions macos 10.258 linux 6.01	allocated macos 4.084 linux 4.084

3○⁻v
⍝⍝: 0.65	0.58	0.56	instructions macos 9.819 linux 5.412	allocated macos 4.084 linux 4.084

v*2
⍝⍝: 0.75	0.65	0.72	instructions macos 2.709 linux 1.751	allocated macos 4.085 linux 4.085

⍝ In wasm, CI's runner gave `v*w` 0.62 under Node 22's V8 on 25 October 2026, and 0.82 to 0.84 under Node 26's from 28 October 2026. V8's newer x86-64 code caused the rise: under Node 26 on Apple silicon, the 0.1.75 release and a build of 29 October 2026 give the same ratio. Its wasm budget is set from Node 26's 0.83.
v*w
⍝⍝: 0.75	0.85	1.09	instructions macos 38.765 linux 23.703	allocated macos 4.079 linux 4.079

⍝ Grade sorts keys and positions of half the bytes.
⍝ Expected: 0.5 of the 64-bit time: half the bytes, and twice the lanes.
⍋v
⍝⍝: 0.75	0.71	0.69	instructions macos 94.499 linux 87.854	allocated macos 18.042 linux 18.042

⍝ A hash lookup costs about the same at either width. These budgets catch a 32-bit search that costs much more than a 64-bit one.
⍝ Expected: 1 of the 64-bit time each: a hash lookup costs the same at either width.
vιw
⍝⍝: 1.25	1.19	1.07	instructions macos 88.548 linux 95.586	allocated macos 40.032 linux 40.033

∪v
⍝⍝: 1.25	1.25	1.23	instructions macos 74.844 linux 89.249	allocated macos 35.04 linux 35.04

⍝ The closed-form laws' slice forms run at f32 with shorter polynomials and twice the lanes. `Pow` computes an f32 power in f64 with no tables, where its f64 power reads tables that NEON can't gather.
⍝ Expected: `ex.quantile v` 0.45 of the 64-bit time: `Log1p`'s 35 operations on four lanes against 40 on two, and passes that move half the bytes; `lp.cdf v` 0.4 of the 64-bit time, as `*s`: `exp`'s shorter polynomial and twice the lanes; `wb.cdf v` 0.5 of the 64-bit time, as `v*w` measures it.
ex.quantile v
⍝⍝: 0.7	0.64	0.61	instructions macos 19.433 linux 9.918	allocated macos 8.162 linux 8.162

lp.cdf v
⍝⍝: 0.65	0.71	0.57	instructions macos 13.231 linux 7.511	allocated macos 4.158 linux 4.158

wb.cdf v
⍝⍝: 0.75	0.75	0.87	instructions macos 17.144 linux 9.567	allocated macos 8.158 linux 8.158

⍝⍝ static •prefs ["kinds":"static" "int":64]
⍝ Under static kinds, a sum of 64-bit integers wraps, so it sums without testing whether its total fits.
⍝ Expected: 0.5 copies, as `+/v`: reading 8 bytes an item and writing nothing moves half of a copy's 16 bytes.
+/kv
⍝⍝: 1	0.64	0.78	instructions macos 3.727 linux 2.187	allocated macos 0.171 linux 0.171

⍝ The same 64-bit integers scanned: the running sums wrap at the `int` width, and nothing bounds them first.
⍝ Expected: 1.5 copies, as `+\v`: reading and writing 8 bytes an item moves one copy's 16 bytes, and the scan adds half a copy, because one addition waits for each group of items. On 25 October 2026 it measured 2.3 to 2.6 copies on macOS. The compiler turns the scan into one chain through the running total, one addition for each item, which costs about a cycle an item, 2 copies on this Mac. The grouped scan's first pass, which keeps the chain to one addition for each group of four, cost more, because a 128-bit vector holds only two 64-bit sums. A scan with that wait, at 64 bits, needs a study.
+\kv
⍝⍝: 3	2.8	2.7	instructions macos 5.721 linux 3.724	allocated macos 8.092 linux 8.092

⍝ A whole float beside 64-bit integers compares as the integer it stands for, which a 32-bit float holds exactly.
⍝ Expected: 0.56 copies: reading 8 bytes an item and writing 1 moves 9 of a copy's 16 bytes.
kv=3
⍝⍝: 1.2	2	4	instructions macos 3.493 linux 3.024	allocated macos 1.089 linux 1.089
