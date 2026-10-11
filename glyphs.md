

# Glyph reference

Each glyph has a name, which links to its definitions and examples. A
function glyph also has a name for its monad, which takes one argument,
and for its dyad, which takes two. Operators take functions or arrays
and derive functions. Notes flag unusual differences from Dyalog APL.
Follow each glyph’s link for details and examples. Each function’s page
gives its ranks. An argument of higher rank applies the function to each
cell of that rank, as [Rank](glyphs/rank.qmd) does.

## Typing glyphs

The [keyboard map](keyboard.qmd) shows the Option and Option-Shift
bindings, with dead keys outlined in orange.

Hold Alt, which is Option on a Mac, and press the key in the Key column.
Keys refer to a US layout. `≤` is Alt-\<, `≥` is Alt-\>, and `⌽` is
Alt-|, each with Shift held. Where the column shows a second key,
release Alt, then press the second key. For example, `Alt-o c` means
Alt-o, then `c`. You can type a shifted second key without Shift. For
example, `Alt-q ;` types `⍠`, the same as `Alt-q :`. An Alt key that
takes a second key types its own glyph when Space or an unlisted key
follows. For example, Alt-o then Space gives `○`. Backspace or Escape
cancels the sequence.

Superscripts start with Alt-6. The second key is a digit, `-` then a
digit, `t` for `ᵀ`, or `u` for `ᵘ`. Subscripts start with Alt-5. The
second key is a digit, `-` then a digit, or `x`, `j`, `e` or `r` for the
number suffixes `ₓ`, `ⱼ`, `ₑ` and `ᵣ`. A glyph with no key, such as `+`,
is on an ordinary ASCII key. The arrows are on hjkl. Alt-\[ types
nothing, because terminals send it as the start of a control sequence.
The identifier characters `∆` and `⍙` are Alt-7, and Alt-Minus then `7`.

The names are the ones you type. In the REPL, a backtick and any glyph,
monad or dyad name, then Tab, enters the glyph, as [named
entry](repl.qmd#named-entry) describes. Python exports the function
names, with `_` in place of `-` and a trailing `_` on Python keywords,
as in `index_of` and `and_`.

## Functions

<table>
<colgroup>
<col style="width: 20%" />
<col style="width: 20%" />
<col style="width: 20%" />
<col style="width: 20%" />
<col style="width: 20%" />
</colgroup>
<thead>
<tr>
<th>Glyph</th>
<th>Key</th>
<th>Monad</th>
<th>Dyad</th>
<th>Note</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>+</code> <a href="glyphs/add.qmd">add</a></td>
<td></td>
<td>conjugate</td>
<td>plus</td>
<td>Character offsets as BQN</td>
</tr>
<tr>
<td><code>-</code> <a href="glyphs/dash.qmd">dash</a></td>
<td></td>
<td>negate</td>
<td>subtract</td>
<td>Character difference as BQN</td>
</tr>
<tr>
<td><code>×</code> <a href="glyphs/mul.qmd">mul</a></td>
<td>Alt-m</td>
<td>sign</td>
<td>times</td>
<td></td>
</tr>
<tr>
<td><code>÷</code> <a href="glyphs/div.qmd">div</a></td>
<td>Alt-u</td>
<td>reciprocal</td>
<td>divide</td>
<td>Exact rationals as J</td>
</tr>
<tr>
<td><code>⌊</code> <a href="glyphs/floor.qmd">floor</a></td>
<td>Alt-d</td>
<td>floor</td>
<td>min</td>
<td><code>⌊/⍬</code> is <code>∞</code></td>
</tr>
<tr>
<td><code>⌈</code> <a href="glyphs/ceiling.qmd">ceiling</a></td>
<td>Alt-s</td>
<td>ceiling</td>
<td>max</td>
<td><code>⌈/⍬</code> is <code>¯∞</code></td>
</tr>
<tr>
<td><code>\|</code> <a href="glyphs/stile.qmd">stile</a></td>
<td></td>
<td>magnitude</td>
<td>residue</td>
<td></td>
</tr>
<tr>
<td><code>*</code> <a href="glyphs/star.qmd">star</a></td>
<td></td>
<td>exponential</td>
<td>exponent</td>
<td></td>
</tr>
<tr>
<td><code>⍟</code> <a href="glyphs/log.qmd">log</a></td>
<td>Alt-o 8</td>
<td>logarithm</td>
<td>log</td>
<td></td>
</tr>
<tr>
<td><code>○</code> <a href="glyphs/circle.qmd">circle</a></td>
<td>Alt-o</td>
<td>cis</td>
<td>circle</td>
<td>Pi-times is <code>π</code>. Codes 0 to 8, inverted by
<code>k○⁻¹</code></td>
</tr>
<tr>
<td><code>π</code> <a href="glyphs/pi.qmd">pi</a></td>
<td>Alt-p</td>
<td>pi-times</td>
<td>pi-ratio</td>
<td></td>
</tr>
<tr>
<td><code>√</code> <a href="glyphs/root.qmd">root</a></td>
<td>Alt-3</td>
<td>sqrt</td>
<td>root</td>
<td>As BQN’s <code>√</code></td>
</tr>
<tr>
<td><code>!</code> <a href="glyphs/factorial.qmd">factorial</a></td>
<td></td>
<td>factorial</td>
<td>binomial</td>
<td></td>
</tr>
<tr>
<td><code>⍭</code> <a href="glyphs/prime.qmd">prime</a></td>
<td>Alt-Backslash</td>
<td>prime</td>
<td>prime-mode</td>
<td>Like J’s <code>p:</code>, with different codes</td>
</tr>
<tr>
<td><code>⨸</code> <a href="glyphs/factor.qmd">factor</a></td>
<td>Alt-o u</td>
<td>factors</td>
<td>factor-spec</td>
<td>As J’s <code>q:</code></td>
</tr>
<tr>
<td><code>⌻</code> <a href="glyphs/polynomial.qmd">polynomial</a></td>
<td>Alt-q t</td>
<td>roots</td>
<td>polyval</td>
<td>As J’s <code>p.</code>, but coefficients come from
<code>⌻⁻¹</code></td>
</tr>
<tr>
<td><code>¿</code> <a
href="glyphs/inverted-question.qmd">inverted-question</a></td>
<td>Alt-/</td>
<td>roll</td>
<td>deal</td>
<td>Dyalog’s <code>?</code></td>
</tr>
<tr>
<td><code>∨</code> <a href="glyphs/or.qmd">or</a></td>
<td>Alt-9</td>
<td>real-imag</td>
<td>gcd</td>
<td>Monad as J’s <code>+.</code></td>
</tr>
<tr>
<td><code>∧</code> <a href="glyphs/and.qmd">and</a></td>
<td>Alt-0</td>
<td>polar</td>
<td>lcm</td>
<td>Monad as J’s <code>*.</code></td>
</tr>
<tr>
<td><code>⊼</code> <a href="glyphs/nand.qmd">nand</a></td>
<td>Alt-Minus 0</td>
<td>square</td>
<td>nand</td>
<td>Monad as J’s <code>*:</code></td>
</tr>
<tr>
<td><code>⊽</code> <a href="glyphs/nor.qmd">nor</a></td>
<td>Alt-Minus 9</td>
<td>double</td>
<td>nor</td>
<td>Monad as J’s <code>+:</code></td>
</tr>
<tr>
<td><code>~</code> <a href="glyphs/tilde.qmd">tilde</a></td>
<td></td>
<td>not</td>
<td>without</td>
<td>Major cells, as BQN</td>
</tr>
<tr>
<td><code>=</code> <a href="glyphs/equal.qmd">equal</a></td>
<td></td>
<td>classify</td>
<td>equal</td>
<td>Monad as J</td>
</tr>
<tr>
<td><code>≠</code> <a href="glyphs/not-equal.qmd">not-equal</a></td>
<td>Alt-Shift-=</td>
<td>unique-mask</td>
<td>not-equal</td>
<td></td>
</tr>
<tr>
<td><code>&lt;</code> <a href="glyphs/less.qmd">less</a></td>
<td></td>
<td>sort-up</td>
<td>less</td>
<td>Monad sorts major cells</td>
</tr>
<tr>
<td><code>≤</code> <a
href="glyphs/less-or-equal.qmd">less-or-equal</a></td>
<td>Alt-&lt;</td>
<td>decrement</td>
<td>less-equal</td>
<td>Monad as J’s <code>&lt;:</code></td>
</tr>
<tr>
<td><code>&gt;</code> <a href="glyphs/greater.qmd">greater</a></td>
<td></td>
<td>sort-down</td>
<td>greater</td>
<td>Monad sorts major cells</td>
</tr>
<tr>
<td><code>≥</code> <a
href="glyphs/greater-or-equal.qmd">greater-or-equal</a></td>
<td>Alt-&gt;</td>
<td>increment</td>
<td>greater-equal</td>
<td>Monad as J’s <code>&gt;:</code></td>
</tr>
<tr>
<td><code>≡</code> <a href="glyphs/match.qmd">match</a></td>
<td>Alt-=</td>
<td>depth</td>
<td>match</td>
<td>Unsigned depth, as APL2/BQN</td>
</tr>
<tr>
<td><code>≢</code> <a href="glyphs/tally.qmd">tally</a></td>
<td>Alt-= /</td>
<td>tally</td>
<td>not-match</td>
<td></td>
</tr>
<tr>
<td><code>⍴</code> <a href="glyphs/rho.qmd">rho</a></td>
<td>Alt-r</td>
<td>shape</td>
<td>reshape</td>
<td>Rank-0 reshape encloses</td>
</tr>
<tr>
<td><code>,</code> <a href="glyphs/comma.qmd">comma</a></td>
<td></td>
<td>ravel</td>
<td>catenate</td>
<td></td>
</tr>
<tr>
<td><code>⍪</code> <a href="glyphs/table.qmd">table</a></td>
<td>Alt-Minus ,</td>
<td>table</td>
<td>catenate-first</td>
<td></td>
</tr>
<tr>
<td><code>⌽</code> <a href="glyphs/reverse.qmd">reverse</a></td>
<td>Alt-|</td>
<td>reverse</td>
<td>rotate</td>
<td></td>
</tr>
<tr>
<td><code>⊖</code> <a
href="glyphs/reverse-first.qmd">reverse-first</a></td>
<td>Alt-o Minus</td>
<td>reverse-first</td>
<td>rotate-first</td>
<td></td>
</tr>
<tr>
<td><code>⍉</code> <a href="glyphs/transpose.qmd">transpose</a></td>
<td>Alt-o Backslash</td>
<td>transpose</td>
<td>reorder-axes</td>
<td></td>
</tr>
<tr>
<td><code>↑</code> <a href="glyphs/take.qmd">take</a></td>
<td>Alt-k</td>
<td>first</td>
<td>take</td>
<td>Monad takes first major cell</td>
</tr>
<tr>
<td><code>↓</code> <a href="glyphs/drop.qmd">drop</a></td>
<td>Alt-j</td>
<td>split</td>
<td>drop</td>
<td></td>
</tr>
<tr>
<td><code>↕</code> <a href="glyphs/windows.qmd">windows</a></td>
<td>Alt-k j</td>
<td>pairs</td>
<td>windows</td>
<td>Monad is <code>2↕</code>; dyad as BQN’s <code>↕</code>, plus
movement/padding</td>
</tr>
<tr>
<td><code>⊂</code> <a href="glyphs/enclose.qmd">enclose</a></td>
<td>Alt-z</td>
<td>enclose</td>
<td>partitioned-enclose</td>
<td>Encloses atoms; partitions first axis</td>
</tr>
<tr>
<td><code>⊆</code> <a href="glyphs/nest.qmd">nest</a></td>
<td>Alt-Minus z</td>
<td>nest</td>
<td>partition</td>
<td>Partitions first axis</td>
</tr>
<tr>
<td><code>⊃</code> <a href="glyphs/mix.qmd">mix</a></td>
<td>Alt-x</td>
<td>mix</td>
<td>pick</td>
<td>Monad as Dyalog <code>⎕ML≥2</code>; pick cells or keys</td>
</tr>
<tr>
<td><code>⌷</code> <a href="glyphs/squad.qmd">squad</a></td>
<td>Alt-q Backslash</td>
<td>materialise</td>
<td>index</td>
<td>Negative indices; <code>∞</code>/<code>¯∞</code> select/reverse
axis</td>
</tr>
<tr>
<td><code>⍳</code> <a href="glyphs/iota.qmd">iota</a></td>
<td>Alt-i</td>
<td>iota</td>
<td>index-of</td>
<td>Axis selectors; negative lengths count down</td>
</tr>
<tr>
<td><code>↦</code> <a href="glyphs/range.qmd">range</a></td>
<td>Alt-l</td>
<td>range</td>
<td>to</td>
<td>Counts up or down from <code>1</code>, <code>¯1</code> or
<code>X</code> to <code>Y</code>; not Dyalog’s branch</td>
</tr>
<tr>
<td><code>:</code> <a href="glyphs/colon.qmd">colon</a></td>
<td></td>
<td>unkey</td>
<td>keyed</td>
<td><a href="keyed.ipynb">Axis keys</a></td>
</tr>
<tr>
<td><code>⍸</code> <a href="glyphs/where.qmd">where</a></td>
<td>Alt-Minus i</td>
<td>where</td>
<td>interval-index</td>
<td>Dyad as BQN’s <code>⍋</code></td>
</tr>
<tr>
<td><code>∊</code> <a href="glyphs/member.qmd">member</a></td>
<td>Alt-e</td>
<td>enlist</td>
<td>member</td>
<td>Major cells, as BQN</td>
</tr>
<tr>
<td><code>∪</code> <a href="glyphs/union.qmd">union</a></td>
<td>Alt-v</td>
<td>unique</td>
<td>union</td>
<td>Major cells, as BQN</td>
</tr>
<tr>
<td><code>∩</code> <a
href="glyphs/intersection.qmd">intersection</a></td>
<td>Alt-c</td>
<td>duplicates</td>
<td>intersection</td>
<td>Major cells, as BQN</td>
</tr>
<tr>
<td><code>⍷</code> <a href="glyphs/find.qmd">find</a></td>
<td>Alt-Minus e</td>
<td>groups</td>
<td>find</td>
<td>Monad as Dyalog’s <code>{⊂⍵}⌸</code></td>
</tr>
<tr>
<td><code>⍋</code> <a href="glyphs/grade-up.qmd">grade-up</a></td>
<td>Alt-Backslash 7</td>
<td>grade-up</td>
<td>grade-up-by</td>
<td></td>
</tr>
<tr>
<td><code>⍒</code> <a href="glyphs/grade-down.qmd">grade-down</a></td>
<td>Alt-Backslash g</td>
<td>grade-down</td>
<td>grade-down-by</td>
<td></td>
</tr>
<tr>
<td><code>#</code> <a href="glyphs/hash.qmd">hash</a></td>
<td></td>
<td>runs</td>
<td>replicate</td>
<td>Dyad as Dyalog’s <code>⌿</code>; monad run-length encodes, and
<code>#⁻¹</code> decodes</td>
</tr>
<tr>
<td><code>⊤</code> <a href="glyphs/encode.qmd">encode</a></td>
<td>Alt-n</td>
<td>binary-encode</td>
<td>encode</td>
<td>Digits on last axis, as J</td>
</tr>
<tr>
<td><code>⊥</code> <a href="glyphs/decode.qmd">decode</a></td>
<td>Alt-b</td>
<td>binary-decode</td>
<td>decode</td>
<td>Digits on last axis, as J</td>
</tr>
<tr>
<td><code>⌹</code> <a href="glyphs/domino.qmd">domino</a></td>
<td>Alt-q u</td>
<td>inverse</td>
<td>matrix-divide</td>
<td></td>
</tr>
<tr>
<td><code>⊣</code> <a href="glyphs/left.qmd">left</a></td>
<td>Alt-;</td>
<td>same-left</td>
<td>left</td>
<td></td>
</tr>
<tr>
<td><code>⊢</code> <a href="glyphs/right.qmd">right</a></td>
<td>Alt-’</td>
<td>same</td>
<td>right</td>
<td></td>
</tr>
<tr>
<td><code>⍎</code> <a href="glyphs/execute.qmd">execute</a></td>
<td>Alt-o b</td>
<td>execute</td>
<td>execute-in</td>
<td><code>X</code> is a keyed vector, where Dyalog takes a
namespace</td>
</tr>
<tr>
<td><code>⍕</code> <a href="glyphs/format.qmd">format</a></td>
<td>Alt-o n</td>
<td>format</td>
<td>format-spec</td>
<td></td>
</tr>
</tbody>
</table>

## Operators

`f`, `g` are functions; `a`, `n`, `r` are arrays or numbers.

<table>
<colgroup>
<col style="width: 20%" />
<col style="width: 20%" />
<col style="width: 20%" />
<col style="width: 20%" />
<col style="width: 20%" />
</colgroup>
<thead>
<tr>
<th>Glyph</th>
<th>Key</th>
<th>Form</th>
<th>Meaning</th>
<th>Note</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>/</code> <a href="glyphs/slash.qmd">slash</a></td>
<td></td>
<td><code>f/</code></td>
<td>Reduce / seeded reduce last</td>
<td>Seed as BQN’s <code>´</code></td>
</tr>
<tr>
<td><code>⌿</code> <a href="glyphs/slash-bar.qmd">slash-bar</a></td>
<td>Alt-Minus /</td>
<td><code>f⌿</code></td>
<td>Reduce / seeded reduce first</td>
<td>Seed as <code>/</code></td>
</tr>
<tr>
<td><code>\</code> <a href="glyphs/backslash.qmd">backslash</a></td>
<td></td>
<td><code>f\</code></td>
<td>Scan / seeded scan last</td>
<td>Accumulate LTR as BQN</td>
</tr>
<tr>
<td><code>⍀</code> <a
href="glyphs/backslash-bar.qmd">backslash-bar</a></td>
<td>Alt-Minus Backslash</td>
<td><code>f⍀</code></td>
<td>Scan / seeded scan first</td>
<td>Accumulate LTR as BQN</td>
</tr>
<tr>
<td><code>¨</code> <a href="glyphs/each.qmd">each</a></td>
<td>Alt-1</td>
<td><code>f¨</code></td>
<td>Each</td>
<td></td>
</tr>
<tr>
<td><code>⍨</code> <a href="glyphs/commute.qmd">commute</a></td>
<td>Alt-1 Backtick</td>
<td><code>f⍨</code>, <code>a⍨</code></td>
<td>Self / commute / constant</td>
<td></td>
</tr>
<tr>
<td><code>↣</code> <a href="glyphs/before.qmd">before</a></td>
<td>Alt-.</td>
<td><code>f↣g</code>, <code>a↣f</code></td>
<td>Before / bind left</td>
<td>As BQN’s <code>⊸</code></td>
</tr>
<tr>
<td><code>↢</code> <a href="glyphs/after.qmd">after</a></td>
<td>Alt-,</td>
<td><code>f↢g</code>, <code>f↢a</code></td>
<td>After / bind right</td>
<td>As BQN’s <code>⟜</code></td>
</tr>
<tr>
<td><code>⍤</code> <a href="glyphs/rank.qmd">rank</a></td>
<td>Alt-1 t</td>
<td><code>f⍤r</code>, <code>f⍤g</code></td>
<td>Rank</td>
<td>Rank 0 preserves atom/scalar distinction</td>
</tr>
<tr>
<td><code>∘</code> <a href="glyphs/atop.qmd">atop</a></td>
<td>Alt-t</td>
<td><code>f∘g</code></td>
<td>Atop</td>
<td>As BQN’s <code>∘</code></td>
</tr>
<tr>
<td><code>⍠</code> <a href="glyphs/axis.qmd">axis</a></td>
<td>Alt-q ;</td>
<td><code>f⍠A</code>, <code>f⍠g</code></td>
<td>Apply along axes</td>
<td>Named or numbered axes</td>
</tr>
<tr>
<td><code>⍥</code> <a href="glyphs/over.qmd">over</a></td>
<td>Alt-1 o</td>
<td><code>f⍥g</code></td>
<td>Over</td>
<td></td>
</tr>
<tr>
<td><code>.</code> <a href="glyphs/dot.qmd">dot</a></td>
<td></td>
<td><code>f.g</code></td>
<td>Inner product</td>
<td>Also array fields/indexing</td>
</tr>
<tr>
<td><code>⊗</code> <a
href="glyphs/outer-product.qmd">outer-product</a></td>
<td>Alt-o m</td>
<td><code>g⊗</code></td>
<td>Outer product</td>
<td>As BQN’s <code>⌜</code></td>
</tr>
<tr>
<td><code>⌸</code> <a href="glyphs/key.qmd">key</a></td>
<td>Alt-q =</td>
<td><code>f⌸</code></td>
<td>Key</td>
<td></td>
</tr>
<tr>
<td><code>⍣</code> <a href="glyphs/power.qmd">power</a></td>
<td>Alt-1 8</td>
<td><code>f⍣n</code>, <code>f⍣∞</code>, <code>f⍣g</code>,
<code>f⍣[g]</code></td>
<td>Iterate / invert / fixed point / repeat until / history until</td>
<td>List counts give history, as J/BQN</td>
</tr>
<tr>
<td><code>⇄</code> <a
href="glyphs/inverse-pair.qmd">inverse-pair</a></td>
<td>Alt-4</td>
<td><code>f⇄g</code></td>
<td>Attach an explicit inverse</td>
<td>As J’s <code>:.</code></td>
</tr>
<tr>
<td><code>⊘</code> <a href="glyphs/valences.qmd">valences</a></td>
<td>Alt-o /</td>
<td><code>f⊘g</code></td>
<td>Call <code>f</code> with one argument, <code>g</code> with two</td>
<td>As BQN’s <code>⊘</code></td>
</tr>
<tr>
<td><code>∂</code> <a href="glyphs/derivative.qmd">derivative</a></td>
<td>Alt-f</td>
<td><code>f∂</code></td>
<td>Gradient / vector–Jacobian product</td>
<td></td>
</tr>
<tr>
<td><code>⍚</code> <a href="glyphs/agenda.qmd">agenda</a></td>
<td>Alt-Minus Backtick</td>
<td><code>selector⍚cases</code></td>
<td>Select and call one function</td>
<td>As BQN’s <code>◶</code></td>
</tr>
<tr>
<td><code>@</code> <a href="glyphs/under.qmd">under</a></td>
<td></td>
<td><code>f@g</code>, <code>f@a</code>, <code>f@[p]</code></td>
<td>Apply under a selection, positions, mask or transform</td>
<td>As BQN’s <code>⌾</code>, not Dyalog’s <code>@</code></td>
</tr>
<tr>
<td><code>⌺</code> <a href="glyphs/stencil.qmd">stencil</a></td>
<td>Alt-q Backtick</td>
<td><code>f⌺a</code>, <code>f⌺g</code></td>
<td>Stencil</td>
<td></td>
</tr>
</tbody>
</table>

## Syntax and literals

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th>Form</th>
<th>Key</th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>←</code> <a href="glyphs/assign.qmd">assign</a></td>
<td>Alt-h</td>
<td>Assignment, including modified and selective forms</td>
</tr>
<tr>
<td><code>(…)</code> <a
href="glyphs/parentheses.qmd">parentheses</a></td>
<td></td>
<td>Grouping. With <code>⋄</code>, a vector of rows:
<code>(4⋄4 5)</code> is <code>[[4] [4 5]]</code></td>
</tr>
<tr>
<td><code>[…]</code> <a href="glyphs/brackets.qmd">brackets</a></td>
<td></td>
<td>Vectors and arrays: <code>[a b c]</code>, <code>[a;b c]</code>,
<code>[1 2⋄3 4]</code>. One item makes a one-item vector:
<code>[x]</code> is <code>,⊂x</code></td>
</tr>
<tr>
<td><code>{…}</code> <a href="glyphs/braces.qmd">braces</a></td>
<td></td>
<td>Defined function or operator. <code>;</code> separates its
bodies</td>
</tr>
<tr>
<td><code>⍺</code> <a href="glyphs/alpha.qmd">alpha</a>, <code>⍵</code>
<a href="glyphs/omega.qmd">omega</a></td>
<td>Alt-a, Alt-w</td>
<td>Left / right argument</td>
</tr>
<tr>
<td><code>⍶</code> <a
href="glyphs/alpha-underbar.qmd">alpha-underbar</a>, <code>⍹</code> <a
href="glyphs/omega-underbar.qmd">omega-underbar</a></td>
<td>Alt-Minus a, Alt-Minus w</td>
<td>Left / right operand</td>
</tr>
<tr>
<td><code>∇</code> <a href="glyphs/del.qmd">del</a>, <code>⍢</code> <a
href="glyphs/del-diaeresis.qmd">del-diaeresis</a></td>
<td>Alt-g, Alt-1 g</td>
<td>Function / operator self-reference</td>
</tr>
<tr>
<td><code>?</code> <a href="glyphs/question.qmd">question</a>,
<code>::</code> <a href="glyphs/error-guard.qmd">error-guard</a></td>
<td></td>
<td>Predicate / error guard inside dfns</td>
</tr>
<tr>
<td><code>⋄</code> <a href="glyphs/diamond.qmd">diamond</a></td>
<td>Alt-Backtick</td>
<td>Separates statements, rows inside brackets, and rows of a vector
inside parentheses</td>
</tr>
<tr>
<td><code>⍝</code> <a href="glyphs/comment.qmd">comment</a></td>
<td>Alt-o c</td>
<td>Comment</td>
</tr>
<tr>
<td><code>⎕←</code> <a href="glyphs/quad.qmd">quad</a></td>
<td>Alt-q</td>
<td>Explicit output. Read alone, the next line of input</td>
</tr>
<tr>
<td><code>•</code> <a href="glyphs/bullet.qmd">bullet</a></td>
<td>Alt-y</td>
<td><a href="system-functions.qmd">System function</a> prefix</td>
</tr>
<tr>
<td><code>⍬</code> <a href="glyphs/zilde.qmd">zilde</a></td>
<td>Alt-Shift-0</td>
<td>Empty numeric vector</td>
</tr>
<tr>
<td><code>'…'</code> <code>"…"</code> <a
href="glyphs/quote.qmd">quote</a></td>
<td></td>
<td>Character and string literals</td>
</tr>
<tr>
<td><code>¯</code> <a href="glyphs/overbar.qmd">overbar</a></td>
<td>Alt-Minus</td>
<td>Negative literal sign</td>
</tr>
<tr>
<td><code>∞</code> <a href="glyphs/infinity.qmd">infinity</a></td>
<td>Alt-8</td>
<td>Real infinity</td>
</tr>
<tr>
<td><code>$t</code> <code>$f</code> <code>$n</code> <a
href="system/constants.qmd">constants</a></td>
<td></td>
<td>True, false and NaN</td>
</tr>
<tr>
<td><code>$e</code> <a href="system/constants.qmd">caught error</a></td>
<td></td>
<td>The error a guard’s handler caught</td>
</tr>
<tr>
<td><code>²</code> <code>⁻¹</code> <code>ᵀ</code> <code>ᵘ</code> <a
href="scripts.ipynb#superscripts">superscript</a></td>
<td>Alt-6</td>
<td>Postfix on the token before it, or after a space on the run before
it: <code>x²</code> is <code>x*2</code>, <code>f²</code> is
<code>f⍣2</code>, <code>f⁻¹</code> is the inverse of <code>f</code>,
<code>mᵀ</code> is <code>⍉m</code>, and <code>fᵘ</code> is a scalar
holding <code>f</code></td>
</tr>
<tr>
<td><code>₁</code> <code>₋₁</code> <a
href="scripts.ipynb#subscripts">subscript</a></td>
<td>Alt-5</td>
<td>Postfix on the array before it, or after a space on the run before
it: <code>v₁</code> is <code>1⌷v</code>, <code>v₋₁</code> is the last
major cell, and <code>v₁←5</code> assigns to that cell</td>
</tr>
</tbody>
</table>

[Numbers](numbers.qmd) covers numeric notation, such as `ₓ`, `r`, `j`
and `e`.

## System names

System names start with `•`. [System functions](system-functions.qmd)
lists them.

Index origin: 0. Negative positions and axes count from the end.
Comparison tolerance of 64-bit floats: `1E¯14`, which
[`•prefs`](system/prefs.qmd) can lower to 0. Narrower floats compare
exactly.

## Language-wide differences from Dyalog APL

- [Vectors](glyphs/brackets.qmd): a bracket list such as `[a b c]` is a
  vector. A strand of literals, such as `1 2 3`, needs no brackets.
  `[5]` is a one-item vector, the same as `,5`.
  [Parentheses](glyphs/parentheses.qmd) group: `(5)` is `5`. A scalar
  holding `5` is `⊂5`. Arrays side by side also form a vector: `a b` is
  `[a b]`. `(1 2⋄3)` is the vector of rows `[[1 2] [3]]`.
- [Runs](syntax.qmd#runs): outside brackets, spaces group an expression
  into runs, and each run acts as one token. A run that ends in a
  function is a train: `+/÷≢` is the mean, and `2×` is `2↣×`. An
  operator takes the token next to it, so after a run that ends in a
  dyadic operator, the next run is its right operand: in `⌊@ 10× x`, it
  is `10×`.
- [Indexing](glyphs/squad.qmd): brackets are used to write vectors,
  never to index. Index selects: `0⌷v` is the first item of `v`.
- [Atoms](arrays.ipynb): numbers, characters and functions are atoms,
  distinct from scalars, as BQN. Enclosing always adds a layer: `⊂3 ≡ 3`
  is `$f`.
- [Numbers](numbers.qmd): bare numbers are approximate. `ₓ` marks an
  exact integer, and `r` or `ᵣ` an exact rational, as in J: `1ₓ÷3ₓ` is
  `1ᵣ3`. Predicates, positions, tally and shape are exact.
- [Agreement](evaluation.qmd#agreement-and-pervasion): pervasive
  functions, Each and Rank align leading axes. An axis of length 1
  expands to match: `2 3⍴⍳6 + 10 20` is `[10 11 12⋄23 24 25]`.
- [Major cells](principles.ipynb#functions-act-on-major-cells):
  `∊ ~ ∪ ∩` compare major cells, `N#Y` replicates them, `N⊂Y` and `N⊆Y`
  split along the first axis. `⊤` and `⊥` put digits on the last axis,
  as J.
- [System names](system-functions.qmd): written `•name`, as BQN, not
  `⎕NAME`. Most Dyalog system functions and variables are not included.
- [Axis keys](keyed.ipynb) name data. `K:Y` evaluates its keys. `T.a` is
  `"a"⊃T`. `T.(expr)` and `T⍎Y` evaluate with the keys of `T` as names.
