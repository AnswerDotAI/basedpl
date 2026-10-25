⍝⍝ setup
n←1048576
pid←10000¿1000000ₓ
prods←["id":pid "cat":¿10000⍴20ₓ "cost":0.5+50×¿10000⍴0]
snames←{"S",⍕⍵}¨⍳50ₓ
sales←["store":[¿n⍴50ₓ]⌷snames "product":[¿n⍴10000ₓ]⌷pid "qty":≥¿n⍴10ₓ "price":1+99×¿n⍴0]
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
  st←s.store {(≢⍵),+⌿⍵}⌸ ⍉[rev⋄mar]
  cq←cat {0.5 0.9 qs ⍵}⌸ s.price
  rq←0.5 0.9 0.99 qs rev
  ["stores":st "cats":cq "revenue":rq]
}
⍝⍝ tables
⍝ Statistics over a table of a million sales, held as columns in a record, beside a table of 10,000 products. `tstats` joins each sale to its product through `⍳` on the product ids, sums the revenue and margin of each of 50 stores by grouping on the store names, finds the median and 90th percentile of the price in each of 20 categories, and finds quantiles of the revenue. `qs` finds quantiles by sorting.
⍝ Expected: 133 copies: the join's lookup of a million integers among 10,000 ids, about 4, and two gathers through its positions, about 6; the revenue and margin, about 4.5; grouping by the store names, about 70, for hashing a million 3-character strings at about 8 ns each, and a pass that sums each group; grouping the prices by category and sorting each group, about 26; and sorting the revenue, about 20.
⍝ Held until studied: on 23 October 2026 `prods tstats sales` measured 341 on macOS, 2.6 times its expected cost. It measured 687 earlier that day. Since then string search reads each string's characters where they lie, and a search whose cells are all strings, characters or exact integers compares keys with no confirmation by `Value::matches` (`search::Exact`). Classifying the store names cost about 140 against the expected 70. The store names are copies of 50 strings that a selection shares, and since 25 October 2026 classification groups items by the array they share before it reads any (`search::shared_classes`): about 28. The workload then measured 213. With integer keys the grouping costs about 31, where the group-fold idiom `(+/⊢)⌸` on those keys costs 6.3, because Key calls the dfn on each group's rows. Its budgets were set from the 687 measurement: the macOS budget about twice it, and the Linux and wasm budgets 1.7 and 1.4 times the macOS budget, the ratios at which scalar work costs more copies there, until CI measures them.
⍝⍝ allows forced "{0.5 0.9 qs ⍵}⌸": Key classifies every key, so it computes the pending category gather whole
⍝⍝ gap itemwise "key groups": each Key should call its dfn once on all its groups, rather than once for each group
prods tstats sales
⍝⍝: 1400	2400	2000

⍝⍝ static •prefs ["kinds":"static" "int":32 "float":32]
⍝ Held until studied, as at 64 bits. Expected at 32 bits: about 100 copies. Grouping by the store names, about 70, doesn't depend on the widths. The numeric parts, about 60 at 64 bits, cost about half. On 24 October 2026 it measured 251 on macOS, 2.5 times that. Its macOS budget is twice its cost. Its Linux and wasm budgets are 1.7 and 1.4 times the macOS budget, as at 64 bits.
⍝⍝ allows forced "{0.5 0.9 qs ⍵}⌸": Key classifies every key, so it computes the pending category gather whole
⍝⍝ gap itemwise "key groups": each Key should call its dfn once on all its groups, rather than once for each group
prods tstats sales
⍝⍝: 510	860	710
