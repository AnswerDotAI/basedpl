

# Glyph reference

Each glyph has a name, which links to its definitions and examples. A
function glyph also has a name for its monad, which takes one argument,
and for its dyad, which takes two. Operators take functions or arrays
and derive functions. Notes flag unusual differences from Dyalog APL.
Follow each glyph’s link for details and examples. Each function’s page
gives its ranks. An argument of higher rank applies the function to each
cell of that rank, as [Rank](glyphs/jot-diaeresis.qmd) does.

## Typing glyphs

The [keyboard map](keyboard.qmd) shows the Option and Option-Shift
bindings, with dead keys outlined in orange.

Hold Alt, which is Option on a Mac, and press the key in the Key column.
Keys refer to a US layout. `≤` is Alt-\< and `≥` is Alt-\>, each with
Shift held. Where the column shows a second key, release Alt, then press
the second key. For example, `Alt-o c` means Alt-o, then `c`. You can
type a shifted second key without Shift. For example, `Alt-q ;` types
`⍠`, the same as `Alt-q :`. An Alt key that takes a second key types its
own glyph when Space or an unlisted key follows. For example, Alt-o then
Space gives `○`. Backspace or Escape cancels the sequence.

Superscripts start with Alt-6. The second key is a digit, `-` then a
digit, `t` for `ᵀ`, or `u` for `ᵘ`. Subscripts start with Alt-5. The
second key is a digit, `-` then a digit, or `x`, `j`, `e` or `r` for the
number suffixes `ₓ`, `ⱼ`, `ₑ` and `ᵣ`. A glyph with no key, such as `+`,
is on an ordinary ASCII key. The arrows are on hjkl. Alt-\[ types
nothing, because terminals send it as the start of a control sequence.
The identifier characters `∆` and `⍙` are Alt-7, and Alt-Minus then `7`.

The names in these tables are the ones you type. In the REPL, a backtick
and any of them, then Tab, enters the glyph, as [named
entry](repl.qmd#named-entry) describes. Python exports the function
names, with `_` in place of `-` and a trailing `_` on Python keywords,
as in `index_of` and `and_`. It spells operators as properties with
their names, as in `f.reduce`.

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
<td><code>+</code> <a href="glyphs/plus.qmd">plus</a></td>
<td></td>
<td>conjugate</td>
<td>add</td>
<td>Character offsets as BQN</td>
</tr>
<tr>
<td><code>-</code> <a href="glyphs/minus.qmd">minus</a></td>
<td></td>
<td>negate</td>
<td>subtract</td>
<td>Character difference as BQN</td>
</tr>
<tr>
<td><code>×</code> <a href="glyphs/times.qmd">times</a></td>
<td>Alt-m</td>
<td>sign</td>
<td>multiply</td>
<td></td>
</tr>
<tr>
<td><code>÷</code> <a href="glyphs/obelus.qmd">obelus</a></td>
<td>Alt-u</td>
<td>reciprocal</td>
<td>divide</td>
<td>Exact rationals as J</td>
</tr>
<tr>
<td><code>⌊</code> <a href="glyphs/downstile.qmd">downstile</a></td>
<td>Alt-d</td>
<td>floor</td>
<td>minimum</td>
<td><code>⌊/⍬</code> is <code>∞</code></td>
</tr>
<tr>
<td><code>⌈</code> <a href="glyphs/upstile.qmd">upstile</a></td>
<td>Alt-s</td>
<td>ceiling</td>
<td>maximum</td>
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
<td><code>⍟</code> <a href="glyphs/circle-star.qmd">circle-star</a></td>
<td>Alt-o 8</td>
<td>natural-log</td>
<td>logarithm</td>
<td></td>
</tr>
<tr>
<td><code>○</code> <a href="glyphs/circle.qmd">circle</a></td>
<td>Alt-o</td>
<td>cis</td>
<td>circular</td>
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
<td><code>√</code> <a href="glyphs/radical.qmd">radical</a></td>
<td>Alt-3</td>
<td>sqrt</td>
<td>root</td>
<td>As BQN’s <code>√</code></td>
</tr>
<tr>
<td><code>!</code> <a href="glyphs/exclamation.qmd">exclamation</a></td>
<td></td>
<td>factorial</td>
<td>binomial</td>
<td></td>
</tr>
<tr>
<td><code>⍭</code> <a href="glyphs/stile-tilde.qmd">stile-tilde</a></td>
<td>Alt-Backslash</td>
<td>prime</td>
<td>prime-mode</td>
<td>Like J’s <code>p:</code>, with different codes</td>
</tr>
<tr>
<td><code>⨸</code> <a
href="glyphs/circle-divide.qmd">circle-divide</a></td>
<td>Alt-o u</td>
<td>factors</td>
<td>factor-spec</td>
<td>As J’s <code>q:</code></td>
</tr>
<tr>
<td><code>⌻</code> <a href="glyphs/quad-jot.qmd">quad-jot</a></td>
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
<td><code>∨</code> <a href="glyphs/vee.qmd">vee</a></td>
<td>Alt-9</td>
<td>real-imag</td>
<td>or</td>
<td>Monad as J’s <code>+.</code></td>
</tr>
<tr>
<td><code>∧</code> <a href="glyphs/wedge.qmd">wedge</a></td>
<td>Alt-0</td>
<td>polar</td>
<td>and</td>
<td>Monad as J’s <code>*.</code></td>
</tr>
<tr>
<td><code>⊼</code> <a href="glyphs/wedge-bar.qmd">wedge-bar</a></td>
<td>Alt-Minus 0</td>
<td>square</td>
<td>nand</td>
<td>Monad as J’s <code>*:</code></td>
</tr>
<tr>
<td><code>⊽</code> <a href="glyphs/vee-bar.qmd">vee-bar</a></td>
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
<td><code>=</code> <a href="glyphs/equals.qmd">equals</a></td>
<td></td>
<td>classify</td>
<td>equal</td>
<td>Monad as J</td>
</tr>
<tr>
<td><code>≠</code> <a href="glyphs/not-equals.qmd">not-equals</a></td>
<td>Alt-Shift-=</td>
<td>unique-mask</td>
<td>not-equal</td>
<td></td>
</tr>
<tr>
<td><code>&lt;</code> <a href="glyphs/less-than.qmd">less-than</a></td>
<td></td>
<td>sort-up</td>
<td>less</td>
<td>Monad sorts major cells</td>
</tr>
<tr>
<td><code>≤</code> <a
href="glyphs/less-than-or-equal.qmd">less-than-or-equal</a></td>
<td>Alt-&lt;</td>
<td>decrement</td>
<td>less-equal</td>
<td>Monad as J’s <code>&lt;:</code></td>
</tr>
<tr>
<td><code>&gt;</code> <a
href="glyphs/greater-than.qmd">greater-than</a></td>
<td></td>
<td>sort-down</td>
<td>greater</td>
<td>Monad sorts major cells</td>
</tr>
<tr>
<td><code>≥</code> <a
href="glyphs/greater-than-or-equal.qmd">greater-than-or-equal</a></td>
<td>Alt-&gt;</td>
<td>increment</td>
<td>greater-equal</td>
<td>Monad as J’s <code>&gt;:</code></td>
</tr>
<tr>
<td><code>≡</code> <a
href="glyphs/equal-underbar.qmd">equal-underbar</a></td>
<td>Alt-=</td>
<td>depth</td>
<td>match</td>
<td>Unsigned depth, as APL2/BQN</td>
</tr>
<tr>
<td><code>≢</code> <a
href="glyphs/equal-underbar-slash.qmd">equal-underbar-slash</a></td>
<td>Alt-?</td>
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
<td><code>⍪</code> <a href="glyphs/comma-bar.qmd">comma-bar</a></td>
<td>Alt-Minus ,</td>
<td>table</td>
<td>catenate-first</td>
<td></td>
</tr>
<tr>
<td><code>⌽</code> <a
href="glyphs/circle-stile.qmd">circle-stile</a></td>
<td>Alt-2</td>
<td>reverse</td>
<td>rotate</td>
<td></td>
</tr>
<tr>
<td><code>⊖</code> <a href="glyphs/circle-bar.qmd">circle-bar</a></td>
<td>Alt-o Minus</td>
<td>reverse-first</td>
<td>rotate-first</td>
<td></td>
</tr>
<tr>
<td><code>⍉</code> <a
href="glyphs/circle-backslash.qmd">circle-backslash</a></td>
<td>Alt-o Backslash</td>
<td>transpose</td>
<td>reorder-axes</td>
<td></td>
</tr>
<tr>
<td><code>↑</code> <a href="glyphs/up-arrow.qmd">up-arrow</a></td>
<td>Alt-k</td>
<td>first</td>
<td>take</td>
<td>Monad takes first major cell</td>
</tr>
<tr>
<td><code>↓</code> <a href="glyphs/down-arrow.qmd">down-arrow</a></td>
<td>Alt-j</td>
<td>split</td>
<td>drop</td>
<td></td>
</tr>
<tr>
<td><code>↕</code> <a
href="glyphs/up-down-arrow.qmd">up-down-arrow</a></td>
<td>Alt-K</td>
<td>pairs</td>
<td>windows</td>
<td>Monad is <code>2↕</code>; dyad as BQN’s <code>↕</code>, plus
movement/padding</td>
</tr>
<tr>
<td><code>⊂</code> <a href="glyphs/left-shoe.qmd">left-shoe</a></td>
<td>Alt-z</td>
<td>enclose</td>
<td>partitioned-enclose</td>
<td>Encloses atoms; partitions first axis</td>
</tr>
<tr>
<td><code>⊆</code> <a
href="glyphs/left-shoe-underbar.qmd">left-shoe-underbar</a></td>
<td>Alt-Minus z</td>
<td>nest</td>
<td>partition</td>
<td>Partitions first axis</td>
</tr>
<tr>
<td><code>⊃</code> <a href="glyphs/right-shoe.qmd">right-shoe</a></td>
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
<td>indices</td>
<td>index-of</td>
<td>Axis selectors; negative lengths count down</td>
</tr>
<tr>
<td><code>↦</code> <a
href="glyphs/right-arrow-from-bar.qmd">right-arrow-from-bar</a></td>
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
<td><code>⍸</code> <a
href="glyphs/iota-underbar.qmd">iota-underbar</a></td>
<td>Alt-Minus i</td>
<td>where</td>
<td>interval-index</td>
<td>Dyad as BQN’s <code>⍋</code></td>
</tr>
<tr>
<td><code>∊</code> <a href="glyphs/epsilon.qmd">epsilon</a></td>
<td>Alt-e</td>
<td>enlist</td>
<td>member</td>
<td>Major cells, as BQN</td>
</tr>
<tr>
<td><code>∪</code> <a href="glyphs/down-shoe.qmd">down-shoe</a></td>
<td>Alt-v</td>
<td>unique</td>
<td>union</td>
<td>Major cells, as BQN</td>
</tr>
<tr>
<td><code>∩</code> <a href="glyphs/up-shoe.qmd">up-shoe</a></td>
<td>Alt-c</td>
<td>duplicates</td>
<td>intersection</td>
<td>Major cells, as BQN</td>
</tr>
<tr>
<td><code>⍷</code> <a
href="glyphs/epsilon-underbar.qmd">epsilon-underbar</a></td>
<td>Alt-Minus e</td>
<td>groups</td>
<td>find</td>
<td>Monad as Dyalog’s <code>{⊂⍵}⌸</code></td>
</tr>
<tr>
<td><code>⍋</code> <a href="glyphs/delta-stile.qmd">delta-stile</a></td>
<td>Alt-Backslash 7</td>
<td>grade-up</td>
<td>grade-up-by</td>
<td></td>
</tr>
<tr>
<td><code>⍒</code> <a href="glyphs/del-stile.qmd">del-stile</a></td>
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
<td><code>⊤</code> <a href="glyphs/down-tack.qmd">down-tack</a></td>
<td>Alt-n</td>
<td>binary-encode</td>
<td>encode</td>
<td>Digits on last axis, as J</td>
</tr>
<tr>
<td><code>⊥</code> <a href="glyphs/up-tack.qmd">up-tack</a></td>
<td>Alt-b</td>
<td>binary-decode</td>
<td>decode</td>
<td>Digits on last axis, as J</td>
</tr>
<tr>
<td><code>⌹</code> <a href="glyphs/domino.qmd">domino</a></td>
<td>Alt-q u</td>
<td>matrix-inverse</td>
<td>matrix-divide</td>
<td></td>
</tr>
<tr>
<td><code>⊣</code> <a href="glyphs/left-tack.qmd">left-tack</a></td>
<td>Alt-;</td>
<td>same-left</td>
<td>left</td>
<td></td>
</tr>
<tr>
<td><code>⊢</code> <a href="glyphs/right-tack.qmd">right-tack</a></td>
<td>Alt-’</td>
<td>same</td>
<td>right</td>
<td></td>
</tr>
<tr>
<td><code>⍎</code> <a href="glyphs/hydrant.qmd">hydrant</a></td>
<td>Alt-o b</td>
<td>execute</td>
<td>execute-in</td>
<td><code>X</code> has keys on its first axis, where Dyalog takes a
namespace</td>
</tr>
<tr>
<td><code>⍕</code> <a href="glyphs/thorn.qmd">thorn</a></td>
<td>Alt-o n</td>
<td>format</td>
<td>format-spec</td>
<td></td>
</tr>
</tbody>
</table>

## Monadic operators

A monadic operator takes one operand, on its left, as in `+/`. Monad and
Dyad name the derived function’s valences. Where both valences do the
same thing, one name in the Monad column covers them. In the notes, `f`
and `g` are functions, and `a`, `n` and `r` are arrays or numbers.

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
<td><code>/</code> <a href="glyphs/slash.qmd">slash</a></td>
<td></td>
<td>reduce</td>
<td></td>
<td><code>f/</code>, along the last axis. A left argument is a seed, as
BQN’s <code>´</code></td>
</tr>
<tr>
<td><code>⌿</code> <a href="glyphs/slash-bar.qmd">slash-bar</a></td>
<td>Alt-Minus /</td>
<td>reduce-first</td>
<td></td>
<td><code>f⌿</code>, along the first axis. Seed as <code>/</code></td>
</tr>
<tr>
<td><code>\</code> <a href="glyphs/backslash.qmd">backslash</a></td>
<td></td>
<td>scan</td>
<td></td>
<td><code>f\</code>, along the last axis. A left argument is a seed.
Accumulates LTR as BQN</td>
</tr>
<tr>
<td><code>⍀</code> <a
href="glyphs/backslash-bar.qmd">backslash-bar</a></td>
<td>Alt-Minus Backslash</td>
<td>scan-first</td>
<td></td>
<td><code>f⍀</code>, along the first axis. Seed as <code>\</code>.
Accumulates LTR as BQN</td>
</tr>
<tr>
<td><code>¨</code> <a href="glyphs/diaeresis.qmd">diaeresis</a></td>
<td>Alt-1</td>
<td>each</td>
<td></td>
<td><code>f¨</code></td>
</tr>
<tr>
<td><code>⍨</code> <a
href="glyphs/tilde-diaeresis.qmd">tilde-diaeresis</a></td>
<td>Alt-1 Backtick</td>
<td>self</td>
<td>commute</td>
<td><code>f⍨</code>. <code>a⍨</code> is a constant function</td>
</tr>
<tr>
<td><code>⊗</code> <a
href="glyphs/circle-times.qmd">circle-times</a></td>
<td>Alt-o m</td>
<td>outer-product</td>
<td></td>
<td><code>g⊗</code>, as BQN’s <code>⌜</code></td>
</tr>
<tr>
<td><code>⌸</code> <a href="glyphs/quad-equal.qmd">quad-equal</a></td>
<td>Alt-q =</td>
<td>key</td>
<td></td>
<td><code>f⌸</code></td>
</tr>
<tr>
<td><code>⇄</code> <a
href="glyphs/right-left-arrows.qmd">right-left-arrows</a></td>
<td>Alt-4</td>
<td>optimize</td>
<td></td>
<td><code>f⇄</code> searches for the point where <code>f</code> is
least, or with <code>"solve":$t</code>, where its residuals are zero. A
left argument holds options</td>
</tr>
<tr>
<td><code>∂</code> <a href="glyphs/partial.qmd">partial</a></td>
<td>Alt-f</td>
<td>derivative</td>
<td></td>
<td><code>f∂</code> gives the Jacobian. The dyad differentiates
<code>⍺ f ⍵</code> with respect to <code>⍵</code></td>
</tr>
</tbody>
</table>

## Dyadic operators

A dyadic operator takes an operand on each side, as in `f⍤1`. The
columns are as for monadic operators.

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
<td><code>↣</code> <a
href="glyphs/right-tail-arrow.qmd">right-tail-arrow</a></td>
<td>Alt-.</td>
<td>before</td>
<td></td>
<td><code>f↣g</code>. <code>a↣f</code> binds a left argument. As BQN’s
<code>⊸</code></td>
</tr>
<tr>
<td><code>↢</code> <a
href="glyphs/left-tail-arrow.qmd">left-tail-arrow</a></td>
<td>Alt-,</td>
<td>after</td>
<td></td>
<td><code>f↢g</code>. <code>f↢a</code> binds a right argument. As BQN’s
<code>⟜</code></td>
</tr>
<tr>
<td><code>⍤</code> <a
href="glyphs/jot-diaeresis.qmd">jot-diaeresis</a></td>
<td>Alt-1 t</td>
<td>rank</td>
<td></td>
<td><code>f⍤r</code>, <code>f⍤g</code>. Rank 0 preserves atom/scalar
distinction</td>
</tr>
<tr>
<td><code>∘</code> <a href="glyphs/jot.qmd">jot</a></td>
<td>Alt-t</td>
<td>atop</td>
<td></td>
<td><code>f∘g</code>, as BQN’s <code>∘</code></td>
</tr>
<tr>
<td><code>⍠</code> <a href="glyphs/quad-colon.qmd">quad-colon</a></td>
<td>Alt-q ;</td>
<td>axis</td>
<td></td>
<td><code>f⍠A</code>, <code>f⍠g</code> apply <code>f</code> along named
or numbered axes</td>
</tr>
<tr>
<td><code>⍥</code> <a
href="glyphs/circle-diaeresis.qmd">circle-diaeresis</a></td>
<td>Alt-1 o</td>
<td>over</td>
<td></td>
<td><code>f⍥g</code></td>
</tr>
<tr>
<td><code>.</code> <a href="glyphs/dot.qmd">dot</a></td>
<td></td>
<td>determinant</td>
<td>inner-product</td>
<td><code>f.g</code>. The monad is J’s determinant form,
<code>u .v</code>. Also array fields and indexing</td>
</tr>
<tr>
<td><code>⍣</code> <a
href="glyphs/star-diaeresis.qmd">star-diaeresis</a></td>
<td>Alt-1 8</td>
<td>power</td>
<td></td>
<td><code>f⍣n</code> iterates or inverts, <code>f⍣∞</code> finds a fixed
point, <code>f⍣g</code> repeats until <code>g</code>, and
<code>f⍣[g]</code> keeps the history until <code>g</code>. List counts
give history, as J/BQN</td>
</tr>
<tr>
<td><code>⊘</code> <a
href="glyphs/circle-slash.qmd">circle-slash</a></td>
<td>Alt-o /</td>
<td>valences</td>
<td></td>
<td><code>f⊘g</code> calls <code>f</code> with one argument and
<code>g</code> with two, as BQN’s <code>⊘</code></td>
</tr>
<tr>
<td><code>⍚</code> <a
href="glyphs/diamond-underbar.qmd">diamond-underbar</a></td>
<td>Alt-Minus Backtick</td>
<td>agenda</td>
<td></td>
<td><code>selector⍚cases</code> selects and calls one function, as BQN’s
<code>◶</code></td>
</tr>
<tr>
<td><code>@</code> <a href="glyphs/at.qmd">at</a></td>
<td></td>
<td>under</td>
<td></td>
<td><code>f@g</code>, <code>f@a</code>, <code>f@[p]</code> apply
<code>f</code> under a selection, positions, mask or transform. As BQN’s
<code>⌾</code>, not Dyalog’s <code>@</code></td>
</tr>
<tr>
<td><code>⌺</code> <a
href="glyphs/quad-diamond.qmd">quad-diamond</a></td>
<td>Alt-q Backtick</td>
<td>stencil</td>
<td></td>
<td><code>f⌺a</code>, <code>f⌺g</code></td>
</tr>
</tbody>
</table>

## Syntax and literals

<table>
<colgroup>
<col style="width: 25%" />
<col style="width: 25%" />
<col style="width: 25%" />
<col style="width: 25%" />
</colgroup>
<thead>
<tr>
<th>Glyph</th>
<th>Key</th>
<th>Name</th>
<th>Note</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>←</code> <a href="glyphs/left-arrow.qmd">left-arrow</a></td>
<td>Alt-h</td>
<td>assign</td>
<td>Includes modified and selective assignment</td>
</tr>
<tr>
<td><code>(…)</code> <a
href="glyphs/parentheses.qmd">parentheses</a></td>
<td></td>
<td></td>
<td>Grouping. With <code>⋄</code>, a vector of rows:
<code>(4⋄4 5)</code> is <code>[[4] [4 5]]</code></td>
</tr>
<tr>
<td><code>[…]</code> <a href="glyphs/brackets.qmd">brackets</a></td>
<td></td>
<td></td>
<td>Vectors and arrays: <code>[a b c]</code>, <code>[a;b c]</code>,
<code>[1 2⋄3 4]</code>. One item makes a one-item vector:
<code>[x]</code> is <code>,⊂x</code></td>
</tr>
<tr>
<td><code>{…}</code> <a href="glyphs/braces.qmd">braces</a></td>
<td></td>
<td></td>
<td>Defined function or operator. <code>;</code> separates its
bodies</td>
</tr>
<tr>
<td><code>⍺</code> <a href="glyphs/alpha.qmd">alpha</a>, <code>⍵</code>
<a href="glyphs/omega.qmd">omega</a></td>
<td>Alt-a, Alt-w</td>
<td>left-argument, right-argument</td>
<td></td>
</tr>
<tr>
<td><code>⍶</code> <a
href="glyphs/alpha-underbar.qmd">alpha-underbar</a>, <code>⍹</code> <a
href="glyphs/omega-underbar.qmd">omega-underbar</a></td>
<td>Alt-Minus a, Alt-Minus w</td>
<td>left-operand, right-operand</td>
<td></td>
</tr>
<tr>
<td><code>∇</code> <a href="glyphs/del.qmd">del</a>, <code>⍢</code> <a
href="glyphs/del-diaeresis.qmd">del-diaeresis</a></td>
<td>Alt-g, Alt-1 g</td>
<td>recursion, operator-recursion</td>
<td>A dfn’s own function or operator</td>
</tr>
<tr>
<td><code>?</code> <a href="glyphs/question.qmd">question</a></td>
<td></td>
<td>predicate</td>
<td>Inside dfns</td>
</tr>
<tr>
<td><code>::</code> <a
href="glyphs/error-guard.qmd">error-guard</a></td>
<td></td>
<td></td>
<td>Error guard inside dfns</td>
</tr>
<tr>
<td><code>⋄</code> <a href="glyphs/diamond.qmd">diamond</a></td>
<td>Alt-Backtick</td>
<td>separator</td>
<td>Separates statements, rows inside brackets, and rows of a vector
inside parentheses</td>
</tr>
<tr>
<td><code>⍝</code> <a href="glyphs/lamp.qmd">lamp</a></td>
<td>Alt-o c</td>
<td>comment</td>
<td></td>
</tr>
<tr>
<td><code>⎕←</code> <a href="glyphs/quad.qmd">quad</a></td>
<td>Alt-q</td>
<td>io</td>
<td>Explicit output. Read alone, the next line of input</td>
</tr>
<tr>
<td><code>•</code> <a href="glyphs/bullet.qmd">bullet</a></td>
<td>Alt-y</td>
<td>system</td>
<td><a href="system-functions.qmd">System function</a> prefix</td>
</tr>
<tr>
<td><code>⍬</code> <a href="glyphs/zilde.qmd">zilde</a></td>
<td>Alt-Shift-0</td>
<td>empty</td>
<td>Empty numeric vector</td>
</tr>
<tr>
<td><code>'…'</code> <code>"…"</code> <a
href="glyphs/quote.qmd">quote</a></td>
<td></td>
<td></td>
<td>Character and string literals</td>
</tr>
<tr>
<td><code>¯</code> <a href="glyphs/overbar.qmd">overbar</a></td>
<td>Alt-Minus</td>
<td>negative</td>
<td>Negative literal sign</td>
</tr>
<tr>
<td><code>∞</code> <a href="glyphs/infinity.qmd">infinity</a></td>
<td>Alt-8</td>
<td></td>
<td>Real infinity</td>
</tr>
<tr>
<td><code>$t</code> <code>$f</code> <code>$n</code> <a
href="system/constants.qmd">constants</a></td>
<td></td>
<td></td>
<td>True, false and NaN</td>
</tr>
<tr>
<td><code>$e</code> <a href="system/constants.qmd">caught error</a></td>
<td></td>
<td></td>
<td>The error a guard’s handler caught</td>
</tr>
<tr>
<td><code>²</code> <code>⁻¹</code> <code>ᵀ</code> <code>ᵘ</code> <a
href="scripts.ipynb#superscripts">superscript</a></td>
<td>Alt-6</td>
<td></td>
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
<td></td>
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
