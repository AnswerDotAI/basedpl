⍝⍝ setup
side←256
n←side×side
v←¿n⍴0
w←¿n⍴0
s←v-0.5
m←[side side]⍴v
jv←¿n⍴1000ₓ
kv←¿n⍴1000000000000ₓ
b←0ₓ=¿n⍴3ₓ
i←¿n⍴⌊ₓn
cnt←¿n⍴3ₓ
c←n⍴"the quick brown fox jumps over the lazy dog"
nd←•distribution "normal"
lg←•distribution "lognormal"
coeffs←0.5 ¯1.25 0.75 2 ¯0.125 0.0625 ¯1.5 0.25
ms←[n÷64 8 8]⍴v
ss←ms+0 2 1⍉ms
sps←ss+⍤2(16×=⊗⍨⍳8)
t←[n÷16 16]⍴v
tq←¿[n÷4 24]⍴0
z←v+0j1×w
zm←128 128↑m+0j1×⊖m
f←"svd"•decompose m
h←"eigen"•decompose m+⍉m
bm←[side side]⍴b
jm←[side side]⍴jv
cm←[side side]⍴c
⍝⍝ copies
⍝ Sharing storage copies nothing.
⍝ Expected: 0 copies each: shares storage.
,m
⍝⍝: 0.5	0.5	0.5

(⍴m)⍴v
⍝⍝: 0.5	0.5	0.5

⍝ Reading each item once, with almost nothing to write.
⍝ Expected: `+/v`, `0+/v` (the kernel's sum, then the seed), `-/v`, `⌈/v`, `+/m` and `+⌿m` 0.5 copies: reads 8 bytes an item and writes nothing; `+/jv` and `-/jv` 0.13 copies: reads 2 bytes an item; `+/z` 1 copy: reads 16 bytes an item; `∧/b` 0.06 copies: reads 1 byte an item.
⍝ Wasm on CI's x86 runner: `+/v` 1 copy, about twice its native cost. Every logged run there since 19 October measured 1.0 to 1.3 copies. `-/v` folds its items in pairs and costs about 0.7 there. On this Mac's V8 the order reverses: `+/v` costs 0.6 and `-/v` 0.8. On this Mac's V8, the only one that can be timed here, a pair fold raised `+/v` from 0.60 to 0.82 copies. `0+/v` runs the same kernel as `+/v` and has the same budgets.
+/v
⍝⍝: 1	1	2

0+/v
⍝⍝: 1	1	2

+/jv
⍝⍝: 0.5	0.5	0.5

+/z
⍝⍝: 2	2	2

-/v
⍝⍝: 1	1	1

-/jv
⍝⍝: 0.5	0.5	0.5

⌈/v
⍝⍝: 1	1	2

∧/b
⍝⍝: 0.5	0.5	0.5

+/m
⍝⍝: 1	1	2

+⌿m
⍝⍝: 1	1	2

⍝ A fold over several axes of a simple array needs no call per cell. `+ × ⌈ ⌊` fold each axis in turn. Other primitives fold each cell's ravel once.
⍝ Expected: 0.5 copies each: reads 8 bytes an item and writes nothing.
⍝ Held until studied: `+/⍠1 2 ms` has old Linux and wasm budgets that suggest about 1.4 and 1.3, more than twice its expected cost. On 22 October 2026 CI measured 1.08 on Linux and 1 in wasm. Its Linux and wasm budgets stay at the old value.
+/⍠1 2 ms
⍝⍝: 1	2.8	2.6

⍝ Held until studied: `-/⍠1 2 ms` measures 1.07 on macOS and has old Linux and wasm budgets that suggest about 1.15 and 1.7, more than twice its expected cost. On 22 October 2026 CI measured 1.18 on Linux and 0.89 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
-/⍠1 2 ms
⍝⍝: 2.3	2.3	3.4

⍝ A cheap ordered right fold keeps each consecutive lane's running value in a local.
⍝ Expected: 6 copies: about 3 cycles an item, because the running value is usually below the next item and most steps skip the division.
|/jv
⍝⍝: 12	12	12

⍝ Reading one argument and writing each item once.
⍝ Expected: `-v` and `⌽v` 1 copy: one read and one write per item; `-jv` 0.25 copies: 2-byte reads and writes; `~b`, `∧\b` and `∨\b` 0.13 copies: 1-byte reads and writes; `1⌽v` 1 copy: two contiguous copies; `⊖m` 1 copy: contiguous row copies; `1↓v` 1 copy: one copy; `(n÷2)↑v` 0.5 copies: copies half the items; `n⍴0` 0.5 copies: writes 8 bytes an item.
-v
⍝⍝: 2	2	2

-jv
⍝⍝: 0.5	0.5	0.5

~b
⍝⍝: 0.5	0.5	0.5

∧\b
⍝⍝: 0.5	0.5	0.5

∨\b
⍝⍝: 0.5	0.5	0.5

⌽v
⍝⍝: 2	2	2

1⌽v
⍝⍝: 2	2	2

⊖m
⍝⍝: 2	2	2

1↓v
⍝⍝: 2	2	2

(n÷2)↑v
⍝⍝: 1	1	1

n⍴0
⍝⍝: 1	1	1

⍝ Reading two arguments and writing each item once.
⍝ Expected: `v+w`, `kv+kv` and `v⌊w` 1.5 copies: two 8-byte reads and one write per item; `jv+jv` and `jv∧ᵇjv` 0.38 copies: 2-byte reads and writes; `v×2` 1 copy: one read and one write per item; `jv×2.5` 0.63 copies: its products are floats, with 2-byte reads and 8-byte writes; `jv=jv` 0.31 copies: two 2-byte reads and a 1-byte write; `b×jv` 0.31 copies: 1- and 2-byte reads, 2-byte writes; `b×kv` 1.1 copies: 1- and 8-byte reads, 8-byte writes; `b∧b` 0.19 copies: 1-byte reads and writes; `c=c` 0.56 copies: two 4-byte reads and a 1-byte write; `(⍳√n)×⊗⍳√n` 0.5 copies: writes 8 bytes an item.
v+w
⍝⍝: 3	3	3

jv+jv
⍝⍝: 0.76	0.76	0.76

kv+kv
⍝⍝: 3	3	6

v×2
⍝⍝: 2	2	2

jv×2.5
⍝⍝: 1.3	1.3	1.3

jv=jv
⍝⍝: 0.62	0.62	1.3

jv∧ᵇjv
⍝⍝: 0.76	0.76	0.76

b×jv
⍝⍝: 0.62	0.62	0.62

b×kv
⍝⍝: 2.2	2.2	2.2

v⌊w
⍝⍝: 3	3	3

b∧b
⍝⍝: 0.5	0.5	0.5

c=c
⍝⍝: 1.2	1.2	1.2

⍝ Held until studied: `(⍳√n)×⊗⍳√n` has old Linux and wasm budgets that suggest about 1.15 and 1.45, more than twice its expected cost. On 22 October 2026 CI measured 0.87 on Linux and 1.07 in wasm. Its Linux and wasm budgets stay at the old value.
(⍳√n)×⊗⍳√n
⍝⍝: 1	2.3	2.9

⍝ Products of 64-bit integers that leave 64 bits become exact big integers, each held in an allocation of its own.
⍝ Expected: 220 copies: about 25 ns an item for an allocation, a free and a 128-bit product.
⍝ Held until studied: `kv×kv` has an old Linux budget that suggests about 850, more than twice its expected cost. On 22 October 2026 CI measured 391 on Linux. Its Linux budget stays at the old value.
kv×kv
⍝⍝: 440	1700	440

⍝ A tolerant gcd of floats walks the continued fraction of each pair's ratio in 64-bit integers wherever the ratio fits.
⍝ Expected: 700 copies: about 35 continued-fraction steps per pair, each waiting on a float division of about 10 cycles.
⍝ Held until studied: `v∨w` has an old wasm budget that suggests about 1650, more than twice its expected cost. On 22 October 2026 CI measured 1373 in wasm. Its wasm budget stays at the old value.
v∨w
⍝⍝: 1400	1400	3300

⍝ Compressing by a mask can't be vectorised, because each kept item's place depends on the items before it.
⍝ Expected: `b#v` 2 copies: about 1 cycle an item, since each kept item's place depends on the items before it; `⍸b` 2 copies: about 1 cycle an item, as compression.
b#v
⍝⍝: 4	4	4

⍸b
⍝⍝: 4	4	4

⍝ Replicating by counts of 0 to 2 writes four copies of each item where its run starts. The next item overwrites the copies past its count. A group of 4 items whose counts are all at most 4 writes after one check for the group (`gather::repeat`). With a check and a taken branch for each item, the loop cost about a third more in the speed test's binary than through `bpl`: `cnt#v` measured 7.1 copies there against 5.3, and `(0+cnt)#v` 9.7 against 8.1 (`cp_`, 21 October 2026).
⍝ Expected: 4 copies: about 2 cycles an item for four stores, which the next item overwrites past its count.
cnt#v
⍝⍝: 8	8	8

⍝ Counts held as floats are checked for whole numbers 256 at a time before they replicate.
⍝ Expected: 6 copies: as `cnt#v`, plus about 1 copy for the add that makes the counts floats, and about 1 more for the whole-number check than `cnt#v` pays to widen its 1-byte counts, because the check rounds each float and writes it as a 64-bit integer.
⍝ Held until studied: `(0+cnt)#v` has an old wasm budget that suggests about 15, more than twice its expected cost. On 22 October 2026 CI measured 13.3 in wasm. Its wasm budget stays at the old value.
⍝ Its macOS budget of 9 comes from the earlier expected cost of 4.5, which left out the add and most of the check. It measures about 7 copies on macOS.
(0+cnt)#v
⍝⍝: 9	12	30

⍝ A range with an approximate length converts each position from an integer to a float. Wasm has no vector instruction for that, so there each block of 8 positions adds 0 to 7 to the block's first float.
⍝ Expected: 0.5 copies: writes 8 bytes an item.
⍝ Held until studied: `⍳n` has old Linux and wasm budgets that suggest about 1.15 and 1.2, more than twice its expected cost. On 22 October 2026 CI measured 1 on Linux and 1.18 in wasm. Its Linux and wasm budgets stay at the old value.
⍳n
⍝⍝: 1	2.3	2.4

⍝ Comparing within tolerance takes a subtraction, the magnitudes, a scale and a comparison for each item. Tolerant floor raises each item by its scaled magnitude, capped below a half, before it floors.
⍝ Expected: `⌊v` 2.75 copies: about 11 vector operations an item, 6 for the tolerance and 5 for the conversion to a 64-bit integer, its range check, and the select and flag for an item that doesn't fit; `v<w` 3 copies: about 12 vector operations an item: 10 for tolerant equality (an exact comparison, a subtraction, three magnitudes, the larger magnitude, a scale, a comparison and an OR), then an exact comparison and an AND-NOT; `jv<500ₓ` 0.19 copies: a 2-byte read and a 1-byte write. `⌊v` and `v<w` were 1.5 copies, from a count of the tolerance's operations alone.
⌊v
⍝⍝: 5.5	5.5	5.5

v<w
⍝⍝: 6	6	6

jv<500ₓ
⍝⍝: 0.5	0.5	0.76

⍝ Writing twice as many items, or writing in a different order from reading.
⍝ Expected: `v,w` 2 copies: writes twice as many items as `v` holds; `⍉m` 1.5 copies: one copy, with cache blocking for the strided side.
v,w
⍝⍝: 4	4	4

⍉m
⍝⍝: 3	3	3

⍝ Windows copy each item once for every window that holds it. Padded windows first copy the argument with its padding.
⍝ Expected: `3↕v` 2 copies: reads each item once and writes it three times; `3 3↕m` 7 copies: reads each item once and writes it nine times, which costs what `[254 254 3 3]⍴v` costs to write as many items into a fresh result, 6.5 to 7.2 on macOS (22 October 2026); `¯3 ¯3↕m` 8 copies: as `3 3↕m`, plus one padded copy.
⍝ `3↕v` measures 2.7 on macOS since short cells copy each run of adjacent items as one block (22 October 2026), down from 4 to 4.84. In wasm it measured 4.1 to 4.8 in seven CI runs from 22 October 2026, about 2.2 times its expected cost. On 22 October 2026 CI measured 3.17 on Linux and 2.83 in wasm. Its wasm budget stays at the old value until studied.
3↕v
⍝⍝: 4	4	13

⍝ `3 3↕m` measures 10.1 on macOS since the short-cell loop tests a cell's direction once a call, not once a run (22 October 2026), down from 12 to 12.5. In wasm it measured 15.0 to 17.5 in seven CI runs from 22 October 2026, about 2.3 times its expected cost. On 22 October 2026 CI measured 13.6 on Linux and 12.7 in wasm, where twice its expected cost, 14, would fail by chance. Its Linux and wasm budgets stay at the old values until studied.
3 3↕m
⍝⍝: 14	43	39

⍝ `¯3 ¯3↕m` measures 11.5 on macOS since the short-cell loop tests a cell's direction once a call, not once a run (22 October 2026), down from 13.2 to 13.3. In wasm it measured 16.8 to 19.4 in seven CI runs from 22 October 2026, about 2.3 times its expected cost. On 22 October 2026 CI measured 15.9 on Linux and 14.4 in wasm, where twice its expected cost, 16, would fail by chance. Its Linux and wasm budgets stay at the old values until studied.
¯3 ¯3↕m
⍝⍝: 16	47	42

⍝ A small-table index finds the items' range in one vectorised pass, then writes each item's first position into the table, one item at a time.
⍝ Expected: 2 copies: about 1 cycle an item to write each first position into the table.
≠jv
⍝⍝: 4	4	4

⍝ A search also looks each item of the other argument up in the table, and unique then compresses by the mask.
⍝ Expected: `jv⍳jv`, `jv∊jv` and `c⍳c` 4 copies: about 1 cycle an item to build the table and 1 to look up; `∪jv` and `∪c` 2 copies: the unique mask, then a compression of few items.
jv⍳jv
⍝⍝: 8	8	8

jv∊jv
⍝⍝: 8	8	8

c⍳c
⍝⍝: 8	8	8

∪jv
⍝⍝: 4	4	4

∪c
⍝⍝: 4	4	4

⍝ Key classifies the keys, then counts and places each group's positions. With `(≢⊢)` it only counts the classes.
⍝ Expected: `{≢⍵}⌸b` 2 copies: classifying, then writing each group's positions; `(≢⊢)⌸b` 0.13 copies: reads 1 byte an item and counts.
⍝⍝ gap itemwise "key groups": the dfn should run once on all the groups, as a batched map runs once on all its cells
{≢⍵}⌸b
⍝⍝: 4	4	4

(≢⊢)⌸b
⍝⍝: 0.5	0.5	0.5

⍝ `(+/⊢)` folds each group of the values in one pass. The keys `i` make about 41,000 groups, each of which the general path would call the operand for.
⍝ Expected: 8 copies: keys below 65,536 index a table directly, at about 2 cycles an item to classify and 2 to fold into the groups.
⍝ Held until studied: on 24 October 2026 `i (+/⊢)⌸ v` measured 17 on macOS, about twice its expected cost, down from 41 to 44 once classes stopped being sorted by their first positions. It takes three passes of about 10 to 15 instructions an item each: first positions, numbering each item through the table, and the fold. On 22 October 2026, before that change, CI measured 60.6 on Linux and 45.5 in wasm. Its macOS, Linux and wasm budgets stay at the old value until CI measures the new code.
i (+/⊢)⌸ v
⍝⍝: 200	370	220

⍝ Classify of Booleans compares each item with the first. Groups of Booleans write each class's positions 8 items at a time, from a table indexed by the byte of those items' classes.
⍝ Expected: `⍷b` 1 copy: writes 4-byte positions, 8 at a time from a table; `=b` 0.13 copies: 1-byte reads and writes.
⍝ Held until studied: `⍷b` has old Linux and wasm budgets that suggest about 2.65 and 2.4, more than twice its expected cost. On 22 October 2026 CI measured 2.32 on Linux and 1.9 in wasm. Its Linux and wasm budgets stay at the old value.
⍷b
⍝⍝: 2	5.3	4.8

=b
⍝⍝: 0.5	0.5	0.5

⍝ Interval index sorts its boundaries, then reads each value's count of boundaries at or below it from a table over their keys.
⍝ Floats' keys span too much for a table, so sixteen values binary-search the sorted floats together.
⍝ Expected: `(<jv)⍸jv` 6 copies: a counting sort of the boundaries, then one table read per value; `(<w)⍸v` 128 copies: a comparison sort of the boundaries, about 64 copies, and 16 binary-search steps per value, about 64.
(<jv)⍸jv
⍝⍝: 12	12	12

(<w)⍸v
⍝⍝: 260	260	260

⍝ Each item waits for the result before it. In scans by `+` and `⌈`, only one item in each group of four waits for the group before it.
⍝ Expected: `+\v` 1.5 copies: one 3-cycle addition waits for each group of four items; `⌈\v` 1.5 copies: one maximum waits for each group of four items; `+\jv` and `+\b` 1.5 copies: a first pass writes each group's four running sums, then a second adds the running total to them as one vector, so one integer addition of 1 cycle waits for each group; `≠\b` 0.5 copies: word-wide bit operations; `-\v` 1.5 copies: as `+\v`, each item after the first negated as it is added; `-\jv` 1.5 copies: as `+\jv`; `⌈\jv` 1 copy: a vectorised running maximum; `+\z` 2 copies: 16-byte reads and writes, with grouped additions.
⍝ Changed on 21 October 2026: `+\jv`, `+\b` and `-\jv` expected 2 copies, for one addition waiting for each item. In one loop, the compiler had rewritten `+\jv`'s grouped integer sums into that chain, at 2.1 copies. Integer sums take `grouped`'s first pass, which makes their grouping hold in every build. Float scans keep the one loop, where the first pass cost about half a copy, and so do integer `⌈\` and `⌊\`, where it cost about a third more.
⍝ Changed on 22 October 2026: the first pass writes each group's four running sums, in the order the outputs take them, and the group's sum to a buffer of its own. With only the group's sum, x86-64 builds vectorised the second pass by assembling each group's sums with byte and word inserts, and `+\b` cost 3.16 copies on CI's runner against 1.65 for the one loop. The second pass now adds the total to one loaded vector, and the total adds the group's sum from its own buffer, so one scalar addition waits for each group. On this Mac `+\b` went from 1.26 copies to 1.10, `-\jv` from 1.47 to 1.38 and `+\jv` from 1.32 to 1.27.
⍝ Wasm: `+\jv`, `+\b` and `-\jv` 2 copies, for the shuffles that build each group's running sums without a deinterleaving load such as aarch64's `ld4`. On this Mac's V8 the layout of 22 October 2026 took `+\b` from 2.13 copies to 1.6, its cost before the first pass, `+\jv` from 2.2 to 1.86 and `-\jv` from 2.45 to 2.28. CI's x86 V8 measured the one loop at 1.6 to 1.9.
+\v
⍝⍝: 3	3	3

⌈\v
⍝⍝: 3	3	3

+\jv
⍝⍝: 3	3	4

+\b
⍝⍝: 3	3	4

⍝ Held until studied: `≠\b` has an old wasm budget that suggests about 1.1, more than twice its expected cost. On 22 October 2026 CI measured 0.99 in wasm. Its wasm budget stays at the old value.
≠\b
⍝⍝: 1	1	2.2

-\v
⍝⍝: 3	3	3

-\jv
⍝⍝: 3	3	4

⌈\jv
⍝⍝: 2	2	2

+\z
⍝⍝: 4	4	4

⍝ A fold or scan by `ᵇ`'s and, or, xor or xnor runs as rustymath's `And`, `Or`, `Xor` or `Xnor` reduction, which the table of facts names. Booleans fold by those tables as they fold by `∧`, `∨`, `≠` and `=`. Other tables, such as nand and nor, fold and scan through the integer kernels, one item at a time.
⍝ Expected: `∧ᵇ/kv` 0.5 copies: reads 8 bytes an item and writes nothing, and the fold vectorises; `∨ᵇ\jv` 1 copy: a running or of 2-byte items.
⍝ `∧ᵇ/kv` measures 0.57 on macOS, and 0.62 to 0.73 in wasm in seven CI runs from 22 October 2026. CI measured 0.43 on Linux on 22 October 2026.
∧ᵇ/kv
⍝⍝: 1	1	1

⍝ Held until studied: `∨ᵇ\jv` measures 1.92 on macOS, about twice its expected cost, where a budget of twice that cost would fail by chance. It scans through rustymath's `grouped`, as `+\jv` does, which measures 1.53. On 22 October 2026 CI measured 1.95 on Linux and 2.55 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
∨ᵇ\jv
⍝⍝: 17	24	49

⍝ After a lane's first result, a comparison scan's totals are 0 or 1. The kernel computes each item's result at both, and rustymath picks between them 64 at a time.
⍝ Expected: `<\v` and `=⍀m` 3 copies: two tolerant comparisons an item, picked 64 at a time; `≠\jv` and `0ₓ=\jv` 1 copy: an exact comparison an item, picked 64 at a time. That misses half the work of `≠\jv` and `0ₓ=\jv`: each item is compared at both totals, into two vectors of Booleans, which `chosen` packs into words, composes and unpacks. On x86-64 the pack's 64-bit multiplies become sequences of `vpmuludq`, because AVX2 has no 64-bit multiply in its lanes.
<\v
⍝⍝: 6	6	12

=⍀m
⍝⍝: 6	6	12

≠\jv
⍝⍝: 2	2	4

0ₓ=\jv
⍝⍝: 2	2	4

⍝ Scans along short lanes make no call for each lane. Comparison and Boolean scans restart lanes inside each 64-bit word of results. Other scans run lanes under 64 items eight side by side, so eight chains of additions are always in flight. Run one lane at a time, as before 21 October 2026, their cost moved with where the code sat: `+\X` cost 1.8 copies in the test binary against 0.9 through `bpl`, and `+\[16384 4]⍴v` 3.4 against 2. Scans along a leading axis make no call for each row: `+⍀[16384 4]⍴v` cost 28 copies while each row of 4 items made two calls that stayed out of line.
⍝ Expected: `=\[32768 2]⍴jv` 1 copy: one exact comparison for each lane of two items. That misses two costs: each lane's first item keeps its value, so basedpl's `Scan::compared` builds the result as integers, and the lanes' first and second items are gathered into two vectors first, by one strided copy (`lane_items` in rustymath's `comparison_scan`). On 22 October 2026 these took this Mac from 9.8 copies to 3.9, about two fifths of it now building the result; `<\[16384 4]⍴v` 3 copies: as `<\v`. That misses the same two costs: the result keeps each lane's first item, so `Scan::compared` builds it as floats, and each lane's first two items are gathered first; `≠\[16384 4]⍴b` 0.5 copies: word-wide bit operations; `+\[16384 4]⍴v` 2 copies: eight lanes run side by side, about 1 cycle an item; `+⍀[16384 4]⍴v` 3 copies: each row waits on the row before, which it reads back from the output through a store-to-load forwarding before it adds, about 6 cycles for each row of 4 items.
⍝ Held until studied: `=\[32768 2]⍴jv` measures 3.9 on macOS and has old Linux and wasm budgets that suggest about 8.5 and 10, more than twice its expected cost. On 22 October 2026 CI measured 3.63 on Linux and 4.32 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
=\[32768 2]⍴jv
⍝⍝: 20	17	20

⍝ Held until studied: `<\[16384 4]⍴v` measures 5.6 on macOS and has old Linux and wasm budgets that suggest about 9.5 and 14.5, more than twice its expected cost. On 22 October 2026 CI measured 6.25 on Linux and 9.71 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
<\[16384 4]⍴v
⍝⍝: 13	19	29

⍝ Held until studied: `≠\[16384 4]⍴b` has old Linux and wasm budgets that suggest about 1.05 and 1.55, more than twice its expected cost. On 22 October 2026 CI measured 0.88 on Linux and 1.12 in wasm. Its Linux and wasm budgets stay at the old value.
≠\[16384 4]⍴b
⍝⍝: 1	2.1	3.1

+\[16384 4]⍴v
⍝⍝: 4	4	4

+⍀[16384 4]⍴v
⍝⍝: 6	6	6

⍝ A residue scan waits on the result before each item. Its exact seed keeps it in integers.
⍝ Expected: 16 copies: each item waits for an integer division of about 8 cycles.
7ₓ|\jv
⍝⍝: 32	32	32

⍝ An inverse scan by `+`, `-`, `=` or `≠` combines each item with the item before it, in one pass.
⍝ Expected: `+\⁻¹v` 1 copy: one read and one write per item; `≠\⁻¹b` 0.13 copies: 1-byte reads and writes.
+\⁻¹v
⍝⍝: 2	2	2

≠\⁻¹b
⍝⍝: 0.5	0.5	0.5

⍝ Each item waits for a read from a random place.
⍝ Expected: 3 copies: about 1.5 cycles an item for an independent read in the cache, plus the 4-byte positions.
⍝ It measured 5.3 to 6.3 copies from 4,096 to 262,144 items, and about 6.5 at a million. It measures about 4.1 at 65,536 items since the one pass that checks the index values folds their least and greatest in the values' own type (`IndexValue::bounds`, 22 October 2026). A compact integer or whole-float index array becomes a table that reads its values in place (`structure::index_table`), and the walk takes items straight through the values (`Table::take_into`, through `gather::take`). The rest of the gap is that checking pass and `take`'s bounds check on every read. The macOS budget follows its expected cost. CI measured 4.27 on Linux on 22 October 2026, and the Linux budget follows its expected cost too. The wasm budget stays at its old value, which suggests about 10.5. In wasm it measured 4.7 to 5.3 in seven CI runs from 22 October 2026, where twice its expected cost, 6, would fail when a run's measured copy is fast.
[i]⌷v
⍝⍝: 6	6	21

⍝ An indexed assignment copies the array once, then writes each value at its index.
⍝ Expected: 4 copies: one copy, then about 1.5 cycles an item for each scattered write.
{
  x←v
  x.[i]←w
  x
}0
⍝⍝: 8	8	8

⍝ A modified assignment with repeated indices combines each value into the array at its index, in turn, in place.
⍝ Expected: 5 copies: one copy, then about 2 cycles an item for each scattered read and write.
{
  x←v
  x.[i]+←w
  x
}0
⍝⍝: 10	10	10

⍝ Under copies the argument, applies the function to what the structural function selects, and writes the result back through the same map.
⍝ Expected: `-@⌽ v` 3 copies: one copy, the negation read through the reversed map, as `-⌽v` costs, and the reversed write; `-@⍉ m` 4 copies: one copy, the negation read through the transpose, about 1.5 copies as `⍉m` costs, and the write back in tiles, about 1.5 copies.
⍝ `-@⍉ m` measures about 6.4 on macOS and 6.7 in wasm. Its negation runs as a separate pass over the gathered transpose, about 1 copy, where the expected cost reads through the transpose.
-@⌽ v
⍝⍝: 6	6	6

-@⍉ m
⍝⍝: 8	8	8

⍝ Expected: `(2×)@((n+2ₓ)↑) v` and `(2×)@(¯1⌽(n+2ₓ)↑) v` 3 copies each: one copy, the doubling read through the overtake's map, and the write back of the items that the overtake took from `v`. The write back walks the overtake's steps and skips its fills. They measure about 4 on macOS: the doubling reads a copy that the map makes first.
(2×)@((n+2ₓ)↑) v
⍝⍝: 6	6	6

(2×)@(¯1⌽(n+2ₓ)↑) v
⍝⍝: 6	6	6

⍝ Few distinct indices take the same loop. `10ₓ|jv` gives integer positions, which a table reads in place (`structure::index_table`).
⍝ Expected: `{h←10⍴0 ⋄ h.[10ₓ|jv]+←1 ⋄ h}0` 4 copies: the integer residue, about 2 copies, and about 1 cycle an item for the 10 counters; `{h←10⍴0ₓ ⋄ h.[10ₓ|jv]+←1ₓ ⋄ h}0` 4 copies: as the float counters.
⍝ Held until studied: `{h←10⍴0 ⋄ h.[10ₓ|jv]+←1 ⋄ h}0` measures about 9.2 on macOS. About 2 copies go to the residue and the rest to the modified write, which combines its offsets a run at a time, as the comment on `{x←v ⋄ x.[i]+←w ⋄ x}0` describes. Until 24 October 2026 the case read `10|jv`, a float residue that gave float positions, and measured about 12 on macOS, 15.6 on Linux and 18.9 in wasm. Its budgets keep the values from that form until CI measures this one.
{
  h←10⍴0
  h.[10ₓ|jv]+←1
  h
}0
⍝⍝: 36	58	47

⍝ Held until studied: `{h←10⍴0ₓ ⋄ h.[10ₓ|jv]+←1ₓ ⋄ h}0` measures about 9.3 on macOS. A write that fails restores from a copy of the 10 counters rather than from a record of every write (`gather::Writes::try_combine`), and its offsets combine a run at a time without a list. Until 24 October 2026 the case read `10|jv`, and measured about 10.7 on macOS, 18 on Linux and 26.1 in wasm. Its budgets keep the values from that form until CI measures this one.
{
  h←10⍴0ₓ
  h.[10ₓ|jv]+←1ₓ
  h
}0
⍝⍝: 31	64	49

⍝ `@` with a mask changes the items or rows that the mask picks. `⍚` with a selector for each cell runs each case once, on the cells that pick it.
⍝ Expected: `√@[0≤]s` 4 copies: a tolerant comparison, about 1.5 copies, and square roots of about 2; `(0≤)⍚[0 √] s` 4 copies: as `√@[0≤]s`; `cnt⍚[- ⊢ 10×] v` 4 copies: each case on every item, about 1 copy each, and one pick; `-@[{side↑b}] m` 1.5 copies: one copy of `m`, and half its rows negated; `(side↑b)⍚[⊢ -] m` 1.5 copies: as `-@[{side↑b}] m`.
⍝ Held until studied: `√@[0≤]s` measures 7.56 to 7.88 on macOS, about twice its expected cost, so a budget of twice that cost would fail by chance. Its old Linux and wasm budgets suggest about 16 and 17.5, more than twice its expected cost. On 22 October 2026 CI measured 8.79 on Linux and 9.88 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
√@[0≤]s
⍝⍝: 15	32	35

⍝ Held until studied: `(0≤)⍚[0 √] s` measures 8 to 8.26 on macOS, about twice its expected cost, so a budget of twice that cost would fail by chance. Its old Linux and wasm budgets suggest about 17.5 and 19, more than twice its expected cost. On 22 October 2026 CI measured 8.69 on Linux and 10.4 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
(0≤)⍚[0 √] s
⍝⍝: 17	35	38

⍝ Held until studied: `cnt⍚[- ⊢ 10×] v` measures 16.51 on macOS and has old Linux and wasm budgets that suggest about 40 and 38.5, more than twice its expected cost. On 22 October 2026 CI measured 16.3 on Linux and 14.9 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
cnt⍚[- ⊢ 10×] v
⍝⍝: 34	80	77

-@[{side↑b}] m
⍝⍝: 3	3	3

(side↑b)⍚[⊢ -] m
⍝⍝: 3	3	3

⍝ A select runs cases that can't fail on every item and keeps the result of the case each item picks, where enough items pick a case that computes.
⍝ Expected: `-@[0>]s` 2 copies: a tolerant comparison and a negation, blended; `(0>)⍚[- 2×] s` 2.5 copies: a tolerant comparison and two cases, blended.
⍝ Held until studied: `-@[0>]s` measures 4.11 on macOS and has old Linux and wasm budgets that suggest about 5.5 and 12.5, more than twice its expected cost. On 22 October 2026 CI measured 4.4 on Linux and 5.99 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
-@[0>]s
⍝⍝: 8.2	11	25

⍝ Held until studied: `(0>)⍚[- 2×] s` measures 5.26 on macOS and has old Linux and wasm budgets that suggest about 7 and 15, more than twice its expected cost. On 22 October 2026 CI measured 5.42 on Linux and 6.96 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
(0>)⍚[- 2×] s
⍝⍝: 11	14	30

⍝ An approximate divisor reads the integers as floats. Each item then needs a float division and a floor within tolerance.
⍝ Expected: 3.5 copies: a conversion, a float division of about 1 cycle an item, and a tolerant floor.
2.5|jv
⍝⍝: 7	7	7

⍝ rustymath's `exp` evaluates a polynomial of 10 terms for each item, and `ln` one of 7 terms and a division.
⍝ Expected: `*s` 5.75 copies: rustymath's f64 `Exp`, which takes each block of 64 items within ±708 through a form with no clamp and one power of two, as rustymath's `tests/speed.txt` counts it; `⍟v` 6.5 copies: rustymath's f64 `Ln`, which takes each block of 64 positive, normal, finite items through a form with no special values, as rustymath's `tests/speed.txt` counts it. `*s` was 6.5 copies until 23 October 2026, when every item took the clamp and two powers of two, and 4.5 before that, from a count of 18 that left out the clamp and building 2^n. `⍟v` was 9 copies until 23 October 2026, when every item took the full form with its special values, and 5.3 before that, from a count that left out the split, the selects and the check for NaN.
*s
⍝⍝: 12	12	26

⍟v
⍝⍝: 13	13	26

⍝ rustymath's sine and cosine reduce each item by π in three fused multiply-adds, and evaluate a polynomial of 7 terms. Tangent reduces by π/2, evaluates a polynomial of 9 terms at half the angle, and divides once.
⍝ Expected: `1○v` and `2○v` 7 copies: about 28 vector operations an item, for the range check, the reduction, the polynomial, the sign flip, the select at 0 and the test that sends rare items to the second pass; `3○v` 8 copies: about 32 vector operations an item, for the range check, the reduction, the polynomial at half the angle, the selects that pick the quotient's terms and the test, with a division that overlaps them. `1○v` and `2○v` were 4 copies, from a count of 16 that took in only the reduction and the polynomial, then 7.3 copies until sine stopped setting the sign of a zero result, which the select at 0 already gives. `3○v` was 6.3 copies, from a count of 17 that left out the range check, the selects and the test, and charged the division 2 copies.
1○v
⍝⍝: 14	14	28

2○v
⍝⍝: 14	14	28

3○v
⍝⍝: 16	16	32

⍝ The hyperbolic functions build on rustymath's `exp` and `expm1`, with a division. The inverse circle functions evaluate a polynomial of 10 or 12 terms after a reduction with a square root or a division.
⍝ Expected: `5○v` 10 copies: about 40 vector operations an item, for `expm1`'s 34 and the arithmetic around it, with a division that overlaps them; `7○v` 7.5 copies: rustymath's f64 `Tanh`, whose `expm1` needs no guards for |x| up to 20, as rustymath's `tests/speed.txt` counts it; `6○v` 7.25 copies: about 29 vector operations an item, for `exp`'s 26 and the arithmetic around it, with a division that overlaps them; `1○⁻¹v` 7 copies and `2○⁻¹v` 7.5 copies: about 28 and 30 vector operations an item, for the reduction, the polynomial of 12 terms, the selects that undo the reduction and the check for a NaN made from a number, with a square root that overlaps them; `3○⁻¹v` 7.25 copies: about 29 vector operations an item, for the selects that pick one of three intervals, the polynomial of 10 terms, the selects of the added multiple of π/4 and the sign, with a division that overlaps them. `5○v` and `7○v` were 10.5 copies until `expm1` stopped selecting -0 for a zero argument, which only set the sign of a zero result. `5○v`, `6○v` and `7○v` were 11, 6.5 and 6.5 copies before that, from `exp`'s old count of 18, two exponentials for `5○v` where it computes one `expm1`, and 2 copies for each division. `1○⁻¹v`, `2○⁻¹v` and `3○⁻¹v` were 6, 6 and 5.5 copies, from counts of 16, 16 and 14 that left out most of the selects and the check, and charged the square root or division 2 copies.
5○v
⍝⍝: 20	20	40

6○v
⍝⍝: 15	15	29

7○v
⍝⍝: 15	15	40

1○⁻¹v
⍝⍝: 14	14	28

2○⁻¹v
⍝⍝: 15	15	30

3○⁻¹v
⍝⍝: 15	15	29

⍝ The inverse hyperbolic functions build on `ln`'s reduction, after a square root or a division. `4○` is √(1 + x²), computed as rustymath's `hypot` of x and 1.
⍝ Expected: `5○⁻¹v` 10.75 copies: about 43 vector operations an item, for the forms near 0 and beyond 2^52, `ln`'s split, the polynomial and the selects for NaN and ∞, with a square root and a division that overlap them; `4○v` 4 copies: about 16 vector operations an item, for scaling x and 1 by the power of two that keeps x² finite and the selects for the larger and for ∞, with a square root that overlaps them. `5○⁻¹v` was 7.3 copies, from a square root and a `log1p` charged the old way. `4○v` was 2.5 copies, from a square root charged 2 copies and two operations, without `hypot`'s scaling.
5○⁻¹v
⍝⍝: 22	22	43

4○v
⍝⍝: 8	8	8

⍝ Monadic `○`, cis, runs rustymath's `Cis` on the reals at their own width: through vForce's `vvcosisin` on macOS, and through the slice forms of `Cos` and `Sin` elsewhere.
⍝ Expected: 14 copies: sine's 26 and cosine's 28 vector operations, as rustymath's speed test counts them for `Sin` and `Cos`, and 16-byte writes. It was 6 copies, from a shared reduction that rustymath doesn't have, which would save only the reduction's few operations.
○v
⍝⍝: 28	28	56

⍝ An exponent of 2 squares each item, in a loop of its own. Another exponent calls rustymath's `pow` for each item, which at f64 reads tables that NEON and wasm can't vectorize.
⍝ Expected: `v*2` 1 copy: one read and one write per item; `v*w` 15 copies: a vectorised `pow` in extra precision, about twice `⍟` and `*` together.
v*2
⍝⍝: 2	2	2

v*w
⍝⍝: 30	30	60

⍝ Bases 2 and 10 and degrees 2 and 3 have loops of their own.
⍝ Expected: `2⍟v` and `10⍟v` 6.5 copies: as `⍟v`, through the fast form of rustymath's `Logb` for bases 2 and 10; `3√v` 9.25 copies: about 37 vector operations an item, for the step for subnormals, the first estimate from the exponent bits, three Newton steps, the last correction, the select for 0, ∞ and NaN, and the sign. `2⍟v` and `10⍟v` were 9 copies, as `⍟v` was. `3√v` was 4 copies, from a count of 15 that took in only the Newton steps.
2⍟v
⍝⍝: 13	13	26

10⍟v
⍝⍝: 13	13	26

3√v
⍝⍝: 19	19	37

⍝ Exact integers read as floats for a function whose results are always approximate.
⍝ Expected: `⍟jv` 7 copies: as `⍟v`, with about 2 operations to convert each integer; `*jv` 7 copies: as `*s`, with the same conversion; `jv⍟2ₓ` 10.5 copies: a conversion and a logarithm of each base, a division, and a second pass that writes the items whose base is 2 or 10 from `log2` or `log10`; `√jv` 2 copies: a conversion and a square root. `⍟jv` was 9.5 copies, as `⍟v` was 9, and `*jv` and `jv⍟2ₓ` were 4.5 and 7.3 copies, from the old count for `*s`.
⍟jv
⍝⍝: 14	14	28

*jv
⍝⍝: 14	14	28

jv⍟2ₓ
⍝⍝: 21	21	42

⍝ Held until studied: `√jv` has old Linux and wasm budgets that suggest about 4.15 and 6.5, more than twice its expected cost. On 22 October 2026 CI measured 3.46 on Linux and 6.01 in wasm. Its Linux and wasm budgets stay at the old value.
√jv
⍝⍝: 4	8.3	13

⍝ `!` calls rustymath's `gamma`, which sums its Lanczos series in double words. A distribution's density, cdf and quantile each run over the array as one slice. The quantile's `erfcinv` sorts each block of items by region, then evaluates each region's rational approximation over its items, with no iteration. `⌻` evaluates a polynomial of degree 7 by Horner's rule.
⍝ Expected: `!v` 132 copies: `Gamma`'s 530 vector operations, as rustymath's speed test counts them, for a double-word Lanczos sum and quotient, a double-word ln of the exponent, `exp_plus`, and `sinpi` for the reflection; `nd.density v` 7.8 copies: `exp`'s 26 vector operations and five more an item, with `sd`'s reciprocals computed once; `nd.cdf v` 8 copies: its inputs all lie where `erfc` takes `erfc_near`, a ratio of polynomials in x² with one division and no exponential, about 31 vector operations an item with the argument and `erfc`'s range checks; `nd.quantile v` 22.5 copies: `Erfcinv`'s 85 vector operations, 21 copies as rustymath's speed test counts them, the check that each probability lies from 0 to 1, and the scaling on each side; `coeffs⌻v` 2.75 copies: a read and a write, and seven fused multiply-adds an item. `nd.density v` was 5.5 copies, from the old count for `exp`. `!v`, `nd.quantile v` and `coeffs⌻v` were 80, 12 and 1.8 copies, from counts that left out most of `gamma`'s and `erfcinv`'s pieces, and the reads and writes.
⍝ `!v` measures 167 on macOS and 112 on Linux. Held until studied: its old wasm budget suggests about 325, more than twice its expected cost. Its wasm budget stays at the old value.
!v
⍝⍝: 270	270	530

nd.density v
⍝⍝: 16	16	32

⍝ `nd.cdf v` measured 15.8 and 16.8 copies on Linux in two CI runs of v0.1.62, with two divisions an item, and 16.5 in v0.1.64's run after its argument multiplied by the reciprocal of `sd`. x86-64 builds still divided for each item, because the closure read `sd` through a reference. Since 22 October 2026 `erfc_line` computes the reciprocal once, which took macOS from 11.1 copies to 9.65.
nd.cdf v
⍝⍝: 16	16	32

⍝ `nd.quantile v` measures 31 on macOS, where `Erfcinv`'s loop takes 86% of it and retires about 91 instructions an item, nearly all in its rational approximations. Since 22 October 2026 `regions` computes each item's arguments once, which took about 5 instructions an item off. Held until studied: on Linux it measures 44.9, because `Erfcinv` costs 34.2 copies there in rustymath's own speed test, 1.6 times its analysis. Apple silicon runs this loop at about 6.5 instructions a cycle, and a lower rate on CI's x86 runner would explain the gap, which no one has measured there. Its old wasm budget suggests about 75. Its Linux and wasm budgets stay at the old values.
nd.quantile v
⍝⍝: 45	45	90

⍝ The log-normal density takes ln x from `Ln`'s slice form, then the normal density at it over x. The log-normal cdf and quantile run the normal slice forms at ln x, the quantile through an `exp`.
⍝ Expected: `lg.density v` 18 copies: `Ln`'s 36 vector operations, the normal density's 31, and a clamp, a division and a select, about 72 an item; `lg.cdf v` 20 copies: `ln` and the argument, about 38 operations, `erfc`'s mixed route, about 33, and 7 for its range checks and the result; `lg.quantile v` 29 copies: `nd.quantile v`'s expected 22.5, and an `exp` of 26 operations.
⍝ Wasm: twice each native cost, so `lg.density v` 36 copies. Wasm's SIMD has no fused multiply-add, so `FUSED` is false there, and each `mla` in `ln`, `exp`, `erfc` and the normal formulas is a multiply and an add. `nd.density v` costs 16.2 in wasm against 9.8 natively, and `lg.density v` 35.3 against 22.7.
lg.density v
⍝⍝: 36	36	72

⍝ Held until studied: `lg.cdf v` measures 51.6 on macOS, about 2.6 times its expected cost. Its `erfc` arguments, -ln v/√2, span `ERFC_SPLIT`, so every block takes `erfc`'s mixed route, which on AArch64 picks `erfc_near` or `erfc_far` with a branch for each item. `nd.cdf ⍟v` alone costs 35. Its macOS and Linux budgets are twice its macOS cost, and its wasm budget is twice CI's wasm cost of 99.4.
lg.cdf v
⍝⍝: 104	104	200

⍝ `lg.quantile v` measures 38.8 on macOS and 48 on Linux. In wasm it measured 91 to 110 in seven CI runs from 22 October 2026, 1.6 to 1.9 times its wasm expected cost of 58, where a budget of 116 would fail when a run's measured copy is fast, so its wasm budget stays at its old value.
lg.quantile v
⍝⍝: 58	58	120

⍝ `coeffs⌻v` measures 3.46 on macOS. Wasm: 4.5 copies, because wasm's SIMD has no fused multiply-add, so each of the seven is a multiply and an add.
coeffs⌻v
⍝⍝: 5.5	5.5	9

⍝ Each item goes into a hash table, and each item of the other argument is looked up in it. A tolerant search looks in two buckets for each item, where an exact search looks in one.
⍝ Expected: 60 copies: about 30 cycles an item to insert and look up in a hash table. Linux: 125 copies. Scalar work costs about 1.7 times as many copies on CI's x86 runner as on macOS, as `fib 16` and the exact `v⍳w` show. The tolerant table and its chains, about 2.9 MB, also overflow that runner's 1 MB L2, where macOS's L2 holds them, which adds about a quarter. CI measured 121 to 133.
⍝ Held until studied: `v⍳w` has an old wasm budget that suggests about 160, more than twice its expected cost. On 22 October 2026 CI measured 117 in wasm. Its wasm budget stays at the old value.
v⍳w
⍝⍝: 120	250	320

⍝ A radix sort moves every item once for each byte that differs between the keys. Keys that span a small range, as `jv`'s and `b`'s do, take a counting sort, which places each item once.
⍝ Expected: `⍋v` 20 copies: a radix sort of about 7 passes that differ, each about 3 copies for keys and positions; `⍋jv` 6 copies: a counting sort, with a histogram pass and one scattered write of 4-byte positions. The histogram costs about 2 cycles an item, because each count is a load, an increment and a store that the next item with the same key waits for, where a copy costs about half a cycle an item. Linux: `⍋jv` 10 copies, because scalar work costs about 1.7 times as many copies on CI's x86 runner, as `v⍳w`'s comment says; CI measured 10.24. `⍋b` 1 copy: two passes writing 4-byte positions.
⍝ Held until studied: `⍋v` measures 50.04 on macOS and has old Linux and wasm budgets that suggest about 85 and 80, more than twice its expected cost. On 22 October 2026 CI measured 84.4 on Linux and 63.4 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍋v
⍝⍝: 140	170	160

⍋jv
⍝⍝: 12	20	12

⍝ Held until studied: `⍋b` has an old wasm budget that suggests about 2.75, more than twice its expected cost. On 22 October 2026 CI measured 2.04 in wasm. Its wasm budget stays at the old value.
⍋b
⍝⍝: 2	2	5.5

⍝ A sort of keys that span a small range counts them, then writes each key as many times as it occurs. It needs no positions.
⍝ Expected: `<jv` 3.5 copies: a histogram pass, at about 2 cycles an item as for `⍋jv`, then each key written as many times as it occurs; `<b` 0.25 copies: counts, then writes.
<jv
⍝⍝: 7	7	7

<b
⍝⍝: 0.5	0.5	0.5

⍝ Sorting under `∂` grades the primal once, then reads the primal and the tangent at the positions the grade gives.
⍝ Expected: 30 copies: a grade, about 20, and reads of the primal and the tangent at its positions, about 3 each.
⍝ Held until studied: `{+/(⍳≢⍵)×<⍵}∂ v` measures 92.93 on macOS and has old Linux and wasm budgets that suggest about 135 and 245, more than twice its expected cost. On 22 October 2026 CI measured 125 on Linux and 104 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
{+/(⍳≢⍵)×<⍵}∂ v
⍝⍝: 220	270	490

⍝ Taking a few items of a grade or a sort selects them, without sorting the rest. `search::view_selected` reads the vector once. A block of 64 items with no item below the greatest one kept so far costs a compare and an OR for each vector.
⍝ Expected: `↑∘< v` 1 copy: that pass, at two instructions for each vector where `⌊/v`'s minimum takes one. `(10↑)∘⍋ v` and `(10↑)∘< v` 1.5 copies: the pass, and the blocks that hold one of the roughly 90 items that pass the bound on the way, which read item by item. Their wasm budgets are twice that, as for the other passes of vector arithmetic in the header.
(10↑)∘⍋ v
⍝⍝: 3	3	6

(10↑)∘< v
⍝⍝: 3	3	6

↑∘< v
⍝⍝: 2	2	4

⍝ The ranks `⍋⍋` invert the first grade instead of grading it again.
⍝ Expected: 22 copies: a grade, about 20, and one scattered write to invert it.
⍝ Held until studied: `⍋⍋ v` measures 55.14 on macOS and has old Linux and wasm budgets that suggest about 145 and 160, more than twice its expected cost. On 22 October 2026 CI measured 87.8 on Linux and 68.3 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
⍋⍋ v
⍝⍝: 140	290	320

⍝ Unique sorts the reals, then compares each with the next one.
⍝ Expected: 22 copies: a sort, about 20, then a comparison of neighbours and a compression.
⍝ Held until studied: `∪v` measures 79.42 on macOS and has old Linux and wasm budgets that suggest about 125 and 115, more than twice its expected cost. On 22 October 2026 CI measured 97.3 on Linux and 74.7 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
∪v
⍝⍝: 190	250	230

⍝ Rank calls a fold, a structural function or a fold of a ravel once on the whole stack, along the cells' axes, rather than once for each cell.
⍝ Expected: `+/⍤1 t` and `(+/,)⍤2 ms` 0.5 copies: one pass reads 8 bytes an item and writes nothing; `(1 1↣⌷)⍤2 ms` 0.02 copies: reads one item of each cell; `⊖⍤2 ms` and `⌽⍤1 ms` 1 copy: one read and one write per item.
⍝ `+/⍤1 t` measures 0.71 on macOS: float sums fold rows of 16 to 63 items eight rows side by side (`Reduction::rows` in rustymath), where a call of `slice` for each row cost about 4.5 ns. In wasm it measured 0.75 to 0.86 in seven CI runs from 22 October 2026, where twice its expected cost, 1, would fail when a run's measured copy is fast. CI measured 0.79 on Linux on 22 October 2026, and its Linux budget follows its expected cost. Its wasm budget stays at the old value.
+/⍤1 t
⍝⍝: 1	1	4.4

⍝ Held until studied: `(+/,)⍤2 ms` measures 1.03 to 1.12 on macOS. Its fold goes through `in_turn`, which folds the last axis's 8-item lanes, then the next axis's, at the cost of a second pass and the totals kept between the passes. One pass over each cell's 64 items measured 1.11, because of the same gap as `+/⍤1 t`: the call and setup for each row. On 22 and 23 October 2026 CI measured up to 1.14 on Linux and 1.09 in wasm. Its macOS budget is twice its measured cost, and its Linux and wasm budgets stay at the old value.
(+/,)⍤2 ms
⍝⍝: 2.2	3	2.6

(1 1↣⌷)⍤2 ms
⍝⍝: 0.5	0.5	0.5

⍝ `⊖⍤2 ms` measures 1.45 on macOS since short cells copy each run of adjacent items as one block (22 October 2026), down from 2.65 to 3.55. In wasm it measured 1.14 to 1.25 in seven CI runs from 22 October 2026. CI measured 1.08 on Linux on 22 October 2026.
⊖⍤2 ms
⍝⍝: 2	2	2

⌽⍤1 ms
⍝⍝: 2	2	2

⍝ The inner products in `rustymath` run `+.×`, `⌈.+` and `⌊.+` on floats and integers, `+.×` on complex numbers, Boolean `∨.∧` and `∧.∨`, and the counts `+.=` and `+.≠`. `rustymath`'s own speed test times those products. Each case here checks that basedpl sends one kind of pair to them. A pair that misses them takes the bounded generic path, several times slower. A case with `⍤2` makes one call for each of 1024 matrices of 8×8.
⍝ Expected: `+.×⍤2⍨ ms` 4 copies: one stacked call of 16 flops an item, about 2 copies, plus the stack's overhead; `m+.×m` 8.8 copies on macOS, 64 on Linux and 128 in wasm: 512 flops an item, at Accelerate's rate for large square products, as rustymath's `linalg::PlusTimes` expects, 8 in a copy's time for rustymath's own kernels and 4 in wasm; `m⌈.+m` 80 copies, and 160 in wasm: 512 operations an item, at about 8 in a copy's time natively and 4 in wasm; `bm∨.∧bm` and `bm∧.∨bm` 2 copies: bit-packed, about 4 word operations an item; `jm+.×jm` 32 copies: 512 integer operations an item; `jm+.=jm` and `cm+.=⍉cm` 32 copies: 256 vectorised comparisons and counts an item; `zm+.×zm` 5.9 copies on macOS, 32 on Linux and 64 in wasm: 256 real flops an item, at Accelerate's rate for complex 128×128 products, which counts complex copies, each taking 2.1 times as long as one of the `n` floats, 8 in a copy's time for rustymath's own kernels and 4 in wasm.
⍝ Held until studied: `+.×⍤2⍨ ms` measures 11.64 on macOS and has old Linux and wasm budgets that suggest about 18 and 24, more than twice its expected cost. On 22 October 2026 CI measured 16.7 on Linux and 15.1 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
+.×⍤2⍨ ms
⍝⍝: 24	36	48

m+.×m
⍝⍝: 18	130	260

m⌈.+m
⍝⍝: 160	160	320

⍝ Held until studied: `bm∨.∧bm` measures 5.64 on macOS and has old Linux and wasm budgets that suggest about 6 and 9.5, more than twice its expected cost. On 22 October 2026 CI measured 4.86 on Linux and 9.74 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
bm∨.∧bm
⍝⍝: 12	12	19

⍝ Held until studied: `bm∧.∨bm` measures 5.4 on macOS and has old Linux and wasm budgets that suggest about 6 and 10, more than twice its expected cost. On 22 October 2026 CI measured 5.18 on Linux and 9.97 in wasm. Its macOS, Linux and wasm budgets stay at the old value.
bm∧.∨bm
⍝⍝: 12	12	20

⍝ Held until studied: `jm+.×jm` has old Linux and wasm budgets that suggest about 80 and 185, more than twice its expected cost. On 22 October 2026 CI measured 72.5 on Linux and 109 in wasm. Its Linux and wasm budgets stay at the old value.
jm+.×jm
⍝⍝: 64	160	370

jm+.=jm
⍝⍝: 64	64	64

cm+.=⍉cm
⍝⍝: 64	64	64

zm+.×zm
⍝⍝: 12	64	130

⍝ Matrix division, `•decompose`, `•decompose⁻¹` and the determinant `-.×` run on rustymath. Its own speed test times each factorisation on a 256×256 matrix, the size of `m`, and least squares on a 4096×16 matrix, the size of `t`. On macOS, Accelerate runs matrix products and triangular solves, and the SVD, the Hermitian eigendecomposition and the real nonsymmetric eigendecomposition from the sizes that rustymath's DEV.md gives. rustymath's own code runs LU, Householder QR, Cholesky, the complex nonsymmetric eigendecomposition and the smaller matrices' decompositions, with their block products and triangular solves on Accelerate. On Linux it runs all of them. These cases time basedpl's part on top of each factorisation: reading the arguments, writing the results, the calls in a `⍤2` stack, each path of `⌹`, the route from monadic `-.×` to LU, and rebuilding a matrix from factors. `tq` is tall enough for `⌹` and `"qr"` to take the CholeskyQR2 path.
⍝ Expected, taking each factorisation's cost from rustymath's speed test on the same platform: `⌹⍤2 ms` 30 copies: 1,024 inverses of 8×8, each about 1,000 cycles of small-matrix work; `ms⌹⍤2 sps` 45 copies: 1,024 solves of 8×8 with 8 right sides, about 1.5 times the inverses; `⌹m` 42 copies on macOS and 133 on Linux: `linalg::Lu::inverse`, 40 and 131, then a read and a write; `(side↑w)⌹m` 25 copies on macOS and 52 on Linux: `linalg::Lu::solve`, 24 and 51, then a read; `((n÷16)↑w)⌹t` 14 copies on macOS: `linalg::lstsq` on the same shape, 13, then a read; `⌹t` 16 copies on macOS: the same tall QR, about 13, then two products with 16 columns and a write; `((n÷4)↑w)⌹tq` 29 copies on macOS: CholeskyQR2 forms two Gram matrices of `tq`'s 24 columns, 288 flops an item each, at about 40 flops in a copy's time, 7.2 each, which Accelerate measured for this Gram product, far above its tall thin Gram rate, whose 4096×16 product spends most of its time on a fixed cost, then solves with a 24×24 triangle, 144 flops an item at the rank-k rate, 4.8, copies `tq`, which holds 6 copies of data, 6, and scans `tq` and multiplies it by `b`, about 3.5; `"qr"•decompose tq` 38 copies on macOS: the same, with a second triangular solve for Q, 4.8, in place of the product with `b`, then writing Q, 6; `"qr"•decompose⍤2 ms` 60 copies: 1,024 QRs of 8×8, each about 2,000 cycles of small-matrix work; `"qr"•decompose m` 56 copies on macOS and 194 on Linux: `linalg::qr`, 54 and 192, then writing Q and R; `"cholesky"•decompose⍤2 sps` 30 copies: 1,024 Cholesky factors of 8×8, each about 1,000 cycles; `"svd"•decompose⍤2 ms` 400 copies: 1,024 iterative SVDs of 8×8, each about 10,000 operations; `"svd"•decompose⁻¹ f` 13 copies on macOS and 68 on Linux: a 256×256 product, which costs what `m+.×m` costs, 8.8 and 64, and four passes that scale U by `s`, transpose V, convert the product and write it; `["kind":"svd" "rank":8]•decompose m` 22 copies on macOS and 66 on Linux: Halko's range finder, with 18 columns and two power iterations, makes six products of 256×256 by 18 columns, 36 flops an item each, five thin QRs and the SVD of an 18×256 matrix, which on Accelerate cost 5.4 for the three products of `m` by 256×18, at about 20 flops in a copy's time, measured for that shape, between the matrix-vector and rank-k rates, 10.3 for the two of `m`'s transpose at the matrix-vector rate, 1.2 for Qᴴ times `m` at the rank-k rate, and 4.6 for the QRs and the SVD, about 32 flops an item at the matrix-vector rate, while products this thin run at about 3.5 flops in a copy's time on rustymath's own kernels, which cost 66 copies on macOS with `--native`; `"eigen"•decompose⁻¹ h` 54 copies on macOS and 200 on Linux: `linalg::Lu::inverse`, 40 and 131, three passes for the vectors' condition number, scaling the vectors by the values, then a 256×256 product, 8.8 and 64, and a write; `"eigen"•decompose⍤2 ss` 300 copies: 1,024 symmetric eigendecompositions of 8×8, each about 5,000 operations; `"eigen"•decompose⍤2 ms` 700 copies: 1,024 nonsymmetric eigendecompositions of 8×8, each about 13,000 operations; `⌹zm` 21 copies on macOS and 67 on Linux: a complex 128×128 inverse, 8n³/3 complex multiply-adds, about 341 real ones an item, at the rate `linalg::Lu::inverse` reaches on a 256×256 matrix, about 17 and 5.2 in a copy's time, then a read and a write; `-.×m` 24 copies on macOS and 51 on Linux: the LU in `linalg::Lu::solve`, whose solve costs under a copy; `-.×⍤2 ms` 20 copies: 1,024 LUs of 8×8. In wasm, which rustymath's speed test doesn't time, the expected costs stay the flop counts at about 4 in a copy's time: `⌹m` 128, `(side↑w)⌹m` 42, `((n÷16)↑w)⌹t` 8, `⌹t` 12, `((n÷4)↑w)⌹tq` 95, `"qr"•decompose tq` 130, `"qr"•decompose m` 170, `"svd"•decompose⁻¹ f` 128, `"eigen"•decompose⁻¹ h` 128, `⌹zm` 86 and `-.×m` 42.
⍝ Held until studied: `⌹⍤2 ms` measured 63 in wasm on CI on 22 October 2026, about 2.1 times its expected cost. Its wasm budget stays at the old value.
⌹⍤2 ms
⍝⍝: 60	60	160

ms⌹⍤2 sps
⍝⍝: 90	90	90

⌹m
⍝⍝: 84	270	260

(side↑w)⌹m
⍝⍝: 50	110	84

⍝ Held until studied: `((n÷16)↑w)⌹t` takes `linalg::lstsq`'s CholeskyQR2 route on macOS, and Householder on rustymath's own kernels, on Linux and in wasm, where `tall_for_qr` gives 16-column matrices to Householder since 23 October 2026. Through Householder it measured 31.8 on CI and 31.4 in wasm, and through CholeskyQR2 35 to 41 in wasm in seven CI runs on 22 October 2026, against a wasm expected cost of 8. On 22 October 2026 CI measured 30 on Linux and 30.1 in wasm. Its Linux and wasm budgets stay at the old value.
((n÷16)↑w)⌹t
⍝⍝: 28	87	110

⍝ Held until studied: `⌹t` measures 40 on macOS, 2.5 times its expected cost, because it forms U from the tall QR's factors and then multiplies it out. Taking the pseudoinverse as R's pseudoinverse times Q's transpose would save about 10 copies (`lx_`, 22 October 2026). On the earlier route it measured 85 on CI and 72 in wasm. Its budgets stay at the old value.
⌹t
⍝⍝: 120	150	240

⍝ Held until studied: `((n÷4)↑w)⌹tq` measured 268 on CI and 263 in wasm before 22 October 2026, and 234 to 429 in wasm on today's route in seven CI runs from 22 October 2026, against a wasm expected cost of 95. On 22 October 2026 CI measured 210 on Linux on today's route. On macOS it measures 53, about twice its expected cost of 29, where a budget of twice it would fail by chance. Its macOS budget stays at the old value. Its Linux budget stays at the old value until Linux's expected cost on rustymath's own kernels is worked out, and its wasm budget stays at the old value.
((n÷4)↑w)⌹tq
⍝⍝: 80	580	700

⍝ Held until studied: `"qr"•decompose tq` measured 334 on CI and 337 in wasm before 22 October 2026, and 313 to 444 in wasm on today's route in seven CI runs from 22 October 2026, against a wasm expected cost of 130. On 22 October 2026 CI measured 270 on Linux on today's route. On macOS it measures 74, about twice its expected cost of 38, where a budget of twice it would fail by chance. Its macOS budget stays at the old value. Its Linux budget stays at the old value until Linux's expected cost on rustymath's own kernels is worked out, and its wasm budget stays at the old value.
"qr"•decompose tq
⍝⍝: 92	580	800

⍝ `"qr"•decompose⍤2 ms` measured about 126 on macOS and 225 on CI before 22 October 2026, about twice its expected cost. Each 8×8 cell retires about 24,000 instructions. `leaf_qr`'s reflections take about 40% of them, two `Zip` adapters that the test profile leaves out of line about 12%, and allocations about 6%, most of them basedpl's, which builds a Q and an R array for each cell of the nested result. Since 22 October 2026 a stack's cells factor in one reused `QrWork` and write into the two outputs directly, which cut the case's instructions by 12%. On 23 October 2026 it measures 46 on macOS, and CI's last two runs measured 65 and 68 on Linux and 61 and 68 in wasm. Every budget is twice its expected cost.
"qr"•decompose⍤2 ms
⍝⍝: 120	120	120

"qr"•decompose m
⍝⍝: 120	390	340

"cholesky"•decompose⍤2 sps
⍝⍝: 60	60	60

⍝ `"svd"•decompose⍤2 ms` runs rustymath's own SVD on every platform, because 8×8 lies below rustymath's `GESDD_FROM`. On macOS it measures about 509, against 775 on Accelerate's `?gesdd`, and CI measured 608.
"svd"•decompose⍤2 ms
⍝⍝: 800	800	800

⍝ `"svd"•decompose⁻¹ f` costs a 256×256 product plus four passes. On CI it measured 90.5 when `m+.×m` measured 79.6. Its expected cost used to take the product from rustymath's `linalg::PlusTimes`, 21 copies on Linux, a figure that basedpl's single-threaded product doesn't reach.
"svd"•decompose⁻¹ f
⍝⍝: 26	140	260

⍝ `["kind":"svd" "rank":8]•decompose m` in wasm: 82 copies, as CI's x86 V8 measured it on 22 October 2026, at 15.7 ns an item against a copy of 0.192 ns. That's about 2.8 flops in a copy's time, where the group's wasm rate of 4 would give 58. Thin products take 70% of its instructions on rustymath's own kernels, at 70 to 80% of a square product's flops per instruction. CI measured 79 to 92 in seven runs from 22 October 2026. This Mac's V8 takes about the same time, 14.3 ns an item, against a copy 1.6 times faster than CI's, and measures 118 to 121 copies, up to 153 under load.
["kind":"svd" "rank":8]•decompose m
⍝⍝: 44	140	170

⍝ Held until studied: `"eigen"•decompose⁻¹ h` measured 781 in wasm on the route that took the vectors' SVD, before 22 October 2026, and 300 to 413 on the route with one LU in seven CI runs from 22 October 2026, against a wasm expected cost of 128. On 22 October 2026 CI measured 270 in wasm. Its wasm budget stays at the old value.
"eigen"•decompose⁻¹ h
⍝⍝: 110	400	1800

⍝ `"eigen"•decompose⍤2 ss` runs rustymath's own Hermitian solver on every platform, because 8×8 lies below rustymath's `HEEVD_FROM`. On macOS it measures about 298, against 392 on Accelerate's `?heevd`.
"eigen"•decompose⍤2 ss
⍝⍝: 600	600	600

"eigen"•decompose⍤2 ms
⍝⍝: 1400	1400	1400

⍝ `⌹zm` measures 28.4 on macOS and 94 on CI, about 1.35 times its expected cost on both. Its complex inverse costs 3.4 to 4 times a real one of the same size, as its four real multiply-adds for each complex one predict, and less than a solve with as many right-hand sides. A 128×128 inverse runs at about 0.7 of the rate a 256×256 one reaches, on Accelerate and on rustymath's own kernels alike.
⌹zm
⍝⍝: 42	140	180

-.×m
⍝⍝: 48	110	84

-.×⍤2 ms
⍝⍝: 40	40	40

⍝⍝ exact •prefs ["mathish":$f]
⍝ Plain IEEE comparisons and `floor`, with nothing to scale by a tolerance.
⍝ Expected: `⌊v` 1.5 copies: a floor, a conversion to a 64-bit integer, a range check, and a select and a success flag for an item that doesn't fit, about 6 vector operations an item, with an 8-byte write; `v<w` and `v=w` 1.1 copies: two 8-byte reads and a 1-byte write; `3|v` 2.5 copies: a float division of about 1 cycle an item and a floor. `⌊v` was 0.75 copies, from a narrow write and no conversion.
⌊v
⍝⍝: 3	3	3

v<w
⍝⍝: 2.2	2.2	2.2

v=w
⍝⍝: 2.2	2.2	2.2

3|v
⍝⍝: 5	5	5

⍝ Each real indexes its `order_key` in a hash map, as exact data does.
⍝ Expected: `v⍳w` 60 copies: about 30 cycles an item to insert and look up in a hash table; `∪v` 30 copies: one hash insert an item.
⍝ Linux and wasm: exact `∪v` 50 copies. Scalar hash work costs about 1.7 times as many copies on CI's Zen 4 as here, as for `v⍳w` above, and CI measured 47 to 51. On wasm32, hashbrown probes 4-byte control groups (its generic `u32` word, where NEON gives 8 and SSE2 16), and foldhash, without wasm's `wide-arithmetic`, approximates each 64-by-64-bit multiply with four 32-bit ones; CI's x86 V8 measured 46 to 48, and this Mac's V8 42 to 51.
v⍳w
⍝⍝: 120	120	120

∪v
⍝⍝: 60	100	100

⍝⍝ widths •prefs ["float":32]
⍝ Half the bytes to read and write, and twice the lanes in each SIMD instruction.
⍝ Expected: `-v`, `v+w`, `v×2`, `√v`, `+⌿m`, `+/v`, `⌈/v` and `m+.×m` 0.5 of the 64-bit time: half the bytes, and twice the lanes; `v<w` 0.53 of the 64-bit time: half the bytes read, the same 1-byte writes; `0.5<v` 0.56 of the 64-bit time: half the bytes read, the same 1-byte writes; `+\v` 0.75 of the 64-bit time: twice the lanes, but the same latency for each waiting addition.
-v
⍝⍝: 0.75	0.75	0.75

v+w
⍝⍝: 0.75	0.75	0.75

v×2
⍝⍝: 0.75	0.75	0.75

v<w
⍝⍝: 0.78	0.78	0.78

0.5<v
⍝⍝: 0.81	0.81	0.81

√v
⍝⍝: 0.75	0.75	0.75

+⌿m
⍝⍝: 0.75	0.75	0.75

+/v
⍝⍝: 0.75	0.75	0.75

⌈/v
⍝⍝: 0.75	0.75	0.75

+\v
⍝⍝: 1	1	1

m+.×m
⍝⍝: 0.75	0.75	0.75

⍝ `⌊` reads half the bytes and writes 32-bit integers in place of 64-bit ones.
⍝ Expected: 0.83 of the 64-bit time: the 32-bit session floors within tolerance, about 10 vector operations an item on four lanes, where `[exact]` floors exactly, about 6 on two lanes. It was 0.5, from a count of bytes, which this loop's arithmetic outweighs.
⌊v
⍝⍝: 1.08	1.08	1.08

⍝ An LU decomposition in 32 bits.
⍝ Expected: 0.5 of the 64-bit time: half the bytes, and twice the lanes.
⌹m
⍝⍝: 0.75	0.75	0.75

⍝ rustymath's `exp` and `ln` run shorter polynomials at f32, with twice the lanes in each SIMD instruction.
⍝ Expected: 0.4 of the 64-bit time each: shorter polynomials and twice the lanes.
*s
⍝⍝: 0.65	0.65	0.65

⍟v
⍝⍝: 0.65	0.65	0.65

⍝ rustymath's sine and cosine reduce in f32 with fused multiply-adds, and run a polynomial of 4 terms in place of 7.
⍝ Expected: `1○v`, `2○v`, `3○v`, `7○v`, `1○⁻¹v` and `3○⁻¹v` 0.4 of the 64-bit time: 4 polynomial terms in place of 7, and twice the lanes; `v*2` and `v*w` 0.5 of the 64-bit time: half the bytes, and twice the lanes.
1○v
⍝⍝: 0.65	0.65	0.65

2○v
⍝⍝: 0.65	0.65	0.65

3○v
⍝⍝: 0.65	0.65	0.65

7○v
⍝⍝: 0.65	0.65	0.65

1○⁻¹v
⍝⍝: 0.65	0.65	0.65

3○⁻¹v
⍝⍝: 0.65	0.65	0.65

v*2
⍝⍝: 0.75	0.75	0.75

v*w
⍝⍝: 0.75	0.75	0.75

⍝ Grade sorts keys and positions of half the bytes.
⍝ Expected: 0.5 of the 64-bit time: half the bytes, and twice the lanes.
⍋v
⍝⍝: 0.75	0.75	0.75

⍝ A hash lookup costs about the same at either width. These budgets catch a 32-bit search that costs much more than a 64-bit one.
⍝ Expected: 1 of the 64-bit time each: a hash lookup costs the same at either width.
v⍳w
⍝⍝: 1.25	1.25	1.25

∪v
⍝⍝: 1.25	1.25	1.25
