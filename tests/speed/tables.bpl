⍝⍝ setup
n←1048576 size 512
products←10000 size 64 ⋄ pid←products¿1000000ₓ
prods←["id":pid "cat":¿products⍴20ₓ "cost":0.5+50×¿products⍴0]
snames←("S",⍕)¨⍳50ₓ
sales←["store":(•csv •csv⁻ ["store":[¿n⍴50ₓ]⌷snames]).store "product":[¿n⍴productsₓ]⌷pid "qty":≥¿n⍴10ₓ "price":1+99×¿n⍴0]
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
⍝ Its expected cost is 83, or 141 on Linux and 116 in wasm, where scalar work costs 1.7 and 1.4 times as many copies. Key batches each dfn over its groups, and `qs` falls back to one call per category, as the gap lines below name: with integer keys the grouping costs about 31, where the group-fold idiom `(+/⊢)⌸` costs 6.3. Classifying the store names costs about 28 on macOS, because `•csv` stores each distinct text of a column once, and classification groups items by the array they share before it reads any (`search::shared_classes`). On Linux CI measured 223 to 295 in most runs from 56 to 62, and up to 549 in runs 58 and 62, whose hosts were busy.
⍝⍝⍝ held macos 222: Key's batched qs falls back to one call per category, and each store's +⌿ reads its gathered rows
⍝⍝⍝ gap forced "⌷": Key classifies every key, which computes the pending category gather whole, by design; qs's [i]⌷q should select from every sorted category in one batched gather, and from rev's sort by partial selection
⍝⍝⍝ gap forced "+⌿": each store's +⌿ should fold its rows by class (a segment fold of matrix cells), with no gather of the rows
⍝⍝⍝ gap fallbacks "a cell's value": qs should run once on all 20 categories, with its positions computed from each cell's length
⍝⍝⍝ gap itemwise "batch cells": qs should sort and select every category in one batched call (a segmented grade and gather)
⍝⍝⍝ gap itemwise "batched items": qs's [i]⌷q should select each category's positions in one batched gather
prods tstats sales
⍝⍝: 130	280	290	instructions macos 695.708 linux 670.006	allocated macos 174.373 linux 174.373

⍝⍝ static •prefs ["kinds":"static" "int":32 "float":32]
⍝ As at 64 bits, Key batches each dfn over its groups, and `qs` falls back to one call per category. Expected at 32 bits: about 50 copies, or 85 on Linux and 70 in wasm. Grouping by the store names, about 20, doesn't depend on the widths. The numeric parts, about 60 at 64 bits, cost about half. On Linux CI measured 160 to 218 in most runs from 56 to 62, and 318 in run 62, whose host was busy.
⍝⍝⍝ held macos 153: Key's batched qs falls back to one call per category, and each store's +⌿ reads its gathered rows
⍝⍝⍝ gap forced "⌷": Key classifies every key, which computes the pending category gather whole, by design; qs's [i]⌷q should select from every sorted category in one batched gather, and from rev's sort by partial selection
⍝⍝⍝ gap forced "+⌿": each store's +⌿ should fold its rows by class (a segment fold of matrix cells), with no gather of the rows
⍝⍝⍝ gap fallbacks "a cell's value": qs should run once on all 20 categories, with its positions computed from each cell's length
⍝⍝⍝ gap itemwise "batch cells": qs should sort and select every category in one batched call (a segmented grade and gather)
⍝⍝⍝ gap itemwise "batched items": qs's [i]⌷q should select each category's positions in one batched gather
⍝⍝⍝ gap forced "<": qs's sort of each category should run as one segmented grade, not cell by cell
prods tstats sales
⍝⍝: 75	180	180	instructions macos 619.803 linux 569.705	allocated macos 122.703 linux 122.704
