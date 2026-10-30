⍝⍝ setup
n←1048576
pid←10000¿1000000ₓ
prods←["id":pid "cat":¿10000⍴20ₓ "cost":0.5+50×¿10000⍴0]
snames←("S",⍕)¨⍳50ₓ
sales←["store":(•csv •csv⁻¹ ["store":[¿n⍴50ₓ]⌷snames]).store "product":[¿n⍴10000ₓ]⌷pid "qty":≥¿n⍴10ₓ "price":1+99×¿n⍴0]
⍝ Empirical quantiles by sorting and selecting positions.
qs←{
  q←<⍵
  i←⌊ₓ⍺×≢q
  [i]⌷q
}
⍝ Join sales to products and summarize stores, categories and revenue.
tstats←{
  p←⍺
  s←⍵
  j←p.id⍳s.product
  cat←[j]⌷p.cat
  cost←[j]⌷p.cost
  rev←s.qty×s.price
  mar←rev-s.qty×cost
  st←s.store {≢⍵ , +⌿⍵}⌸ ⍉[rev⋄mar]
  cq←cat {0.5 0.9 qs ⍵}⌸ s.price
  rq←0.5 0.9 0.99 qs rev
  ["stores":st "cats":cq "revenue":rq]
}
⍝⍝ tables
⍝ Statistics over a table of a million sales, held as columns in a record, beside a table of 10,000 products. `tstats` joins each sale to its product through `⍳` on the product ids, sums the revenue and margin of each of 50 stores by grouping on the store names, finds the median and 90th percentile of the price in each of 20 categories, and finds quantiles of the revenue. `qs` finds quantiles by sorting.
⍝ Expected: 83 copies: the join's lookup of a million integers among 10,000 ids, about 4, and two gathers through its positions, about 6; the revenue and margin, about 4.5; grouping by the store names, about 20, for looking up a million items that share 50 arrays at about 2 ns each, and a pass that sums each group; grouping the prices by category and sorting each group, about 26; and sorting the revenue, about 20.
⍝ Its expected cost is 83, or 141 on Linux and 116 in wasm, where scalar work costs 1.7 and 1.4 times as many copies. Key calls the dfn on each group's rows, as the gap line below names: with integer keys the grouping costs about 31, where the group-fold idiom `(+/⊢)⌸` costs 6.3. Classifying the store names costs about 28 on macOS, because `•csv` stores each distinct text of a column once, and classification groups items by the array they share before it reads any (`search::shared_classes`). On Linux CI measured 223 to 295 in most runs from 56 to 62, and up to 549 in runs 58 and 62, whose hosts were busy.
⍝⍝⍝ held macos 222 wasm 295: Key calls the dfn on each group's rows
⍝⍝⍝ busy linux 549: its data passes CI's L2 (open.md, "Footprint copies")
⍝⍝⍝ allows forced "{0.5 0.9 qs ⍵}⌸": Key classifies every key, so it computes the pending category gather whole
⍝⍝⍝ gap itemwise "key groups": each Key should call its dfn once on all its groups, rather than once for each group
prods tstats sales
⍝⍝: 130	220	180

⍝⍝ static •prefs ["kinds":"static" "int":32 "float":32]
⍝ As at 64 bits, Key calls the dfn on each group's rows. Expected at 32 bits: about 50 copies, or 85 on Linux and 70 in wasm. Grouping by the store names, about 20, doesn't depend on the widths. The numeric parts, about 60 at 64 bits, cost about half. On Linux CI measured 160 to 218 in most runs from 56 to 62, and 318 in run 62, whose host was busy.
⍝⍝⍝ held macos 153 wasm 202: Key calls the dfn on each group's rows
⍝⍝⍝ busy linux 318: its data passes CI's L2 (open.md, "Footprint copies")
⍝⍝⍝ allows forced "{0.5 0.9 qs ⍵}⌸": Key classifies every key, so it computes the pending category gather whole
⍝⍝⍝ gap itemwise "key groups": each Key should call its dfn once on all its groups, rather than once for each group
prods tstats sales
⍝⍝: 75	130	110
