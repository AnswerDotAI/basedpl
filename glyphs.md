

# Glyph reference

Each name below links to its definitions and examples. Examples show an
equivalent APL value after `⍝`. A monad takes one argument; a dyad takes
two. Operators take functions or arrays and derive functions. Notes give
glyph-specific differences from Dyalog APL.

## Functions

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
<th>Monad</th>
<th>Dyad</th>
<th>Note</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>+</code> <a href="glyphs/add.qmd">Add</a></td>
<td>Conjugate</td>
<td>Add</td>
<td>Character offsets as BQN: <code>'a'+3</code> is
<code>'d'</code></td>
</tr>
<tr>
<td><code>-</code> <a href="glyphs/dash.qmd">Dash</a></td>
<td>Negate</td>
<td>Subtract</td>
<td>Character difference as BQN: <code>'d'-'a'</code> is
<code>3x</code></td>
</tr>
<tr>
<td><code>×</code> <a href="glyphs/mul.qmd">Mul</a></td>
<td>Direction</td>
<td>Multiply</td>
<td></td>
</tr>
<tr>
<td><code>÷</code> <a href="glyphs/div.qmd">Div</a></td>
<td>Reciprocal</td>
<td>Divide</td>
<td>Exact rationals as J: <code>1x÷3x</code> is <code>1r3</code></td>
</tr>
<tr>
<td><code>⌊</code> <a href="glyphs/floor.qmd">Floor</a></td>
<td>Floor</td>
<td>Minimum</td>
<td><code>⌊/⍬</code> is <code>∞</code></td>
</tr>
<tr>
<td><code>⌈</code> <a href="glyphs/ceiling.qmd">Ceiling</a></td>
<td>Ceiling</td>
<td>Maximum</td>
<td><code>⌈/⍬</code> is <code>¯∞</code></td>
</tr>
<tr>
<td><code>\|</code> <a href="glyphs/stile.qmd">Stile</a></td>
<td>Magnitude</td>
<td>Residue</td>
<td></td>
</tr>
<tr>
<td><code>*</code> <a href="glyphs/star.qmd">Star</a></td>
<td>Exponential</td>
<td>Exponent</td>
<td></td>
</tr>
<tr>
<td><code>⍟</code> <a href="glyphs/log.qmd">Log</a></td>
<td>Natural log</td>
<td>Logarithm</td>
<td></td>
</tr>
<tr>
<td><code>○</code> <a href="glyphs/circle.qmd">Circle</a></td>
<td>Unit circle</td>
<td>Circular functions</td>
<td>Monad is <code>*0j1×Y</code>, a unit-circle point. Pi times is
<code>π</code></td>
</tr>
<tr>
<td><code>π</code> <a href="glyphs/pi.qmd">Pi</a></td>
<td>Pi times</td>
<td>Pi fraction</td>
<td>Monad takes over Pi times from <code>○</code>. <code>XπY</code> is
Xπ÷Y</td>
</tr>
<tr>
<td><code>√</code> <a href="glyphs/root.qmd">Root</a></td>
<td>Square root</td>
<td>Nth root</td>
<td>As BQN’s <code>√</code>, with complex results: <code>√¯4</code> is
<code>0j2</code></td>
</tr>
<tr>
<td><code>!</code> <a href="glyphs/factorial.qmd">Factorial</a></td>
<td>Factorial</td>
<td>Binomial</td>
<td></td>
</tr>
<tr>
<td><code>ℙ</code> <a href="glyphs/prime.qmd">Prime</a></td>
<td>Nth prime, counting from 0</td>
<td>Prime operations</td>
<td>As J’s <code>p:</code>, with the same codes</td>
</tr>
<tr>
<td><code>⨸</code> <a href="glyphs/factor.qmd">Factor</a></td>
<td>Prime factors</td>
<td>Exponents / factor table</td>
<td>As J’s <code>q:</code>, with <code>∞</code> and <code>¯∞</code> for
J’s <code>_</code> and <code>__</code></td>
</tr>
<tr>
<td><code>⊛</code> <a href="glyphs/polynomial.qmd">Polynomial</a></td>
<td>Roots / coefficients</td>
<td>Evaluate</td>
<td>As J’s <code>p.</code>: coefficients are constant-first</td>
</tr>
<tr>
<td><code>?</code> <a href="glyphs/question.qmd">Question</a></td>
<td>Roll</td>
<td>Deal</td>
<td></td>
</tr>
<tr>
<td><code>∨</code> <a href="glyphs/or.qmd">Or</a></td>
<td>Real / imaginary parts</td>
<td>OR / GCD</td>
<td>Monad as J’s <code>+.</code></td>
</tr>
<tr>
<td><code>∧</code> <a href="glyphs/and.qmd">And</a></td>
<td>Magnitude / angle</td>
<td>AND / LCM</td>
<td>Monad as J’s <code>*.</code></td>
</tr>
<tr>
<td><code>⍲</code> <a href="glyphs/nand.qmd">Nand</a></td>
<td>Square</td>
<td>NAND</td>
<td>Monad as J’s <code>*:</code></td>
</tr>
<tr>
<td><code>⍱</code> <a href="glyphs/nor.qmd">Nor</a></td>
<td>Double</td>
<td>NOR</td>
<td>Monad as J’s <code>+:</code></td>
</tr>
<tr>
<td><code>~</code> <a href="glyphs/tilde.qmd">Tilde</a></td>
<td>NOT</td>
<td>Without</td>
<td></td>
</tr>
<tr>
<td><code>=</code> <a href="glyphs/equal.qmd">Equal</a></td>
<td>Self-classify</td>
<td>Equal</td>
<td>Monad as J: one mask row per distinct major cell</td>
</tr>
<tr>
<td><code>≠</code> <a href="glyphs/not-equal.qmd">Not equal</a></td>
<td>Unique mask</td>
<td>Not equal</td>
<td></td>
</tr>
<tr>
<td><code>&lt;</code> <a href="glyphs/less.qmd">Less</a></td>
<td>—</td>
<td>Less</td>
<td></td>
</tr>
<tr>
<td><code>≤</code> <a href="glyphs/less-or-equal.qmd">Less or
equal</a></td>
<td>Decrement</td>
<td>Less or equal</td>
<td>Monad as J’s <code>&lt;:</code></td>
</tr>
<tr>
<td><code>&gt;</code> <a href="glyphs/greater.qmd">Greater</a></td>
<td>—</td>
<td>Greater</td>
<td></td>
</tr>
<tr>
<td><code>≥</code> <a href="glyphs/greater-or-equal.qmd">Greater or
equal</a></td>
<td>Increment</td>
<td>Greater or equal</td>
<td>Monad as J’s <code>&gt;:</code></td>
</tr>
<tr>
<td><code>≡</code> <a href="glyphs/match.qmd">Match</a></td>
<td>Depth</td>
<td>Match</td>
<td></td>
</tr>
<tr>
<td><code>≢</code> <a href="glyphs/tally.qmd">Tally</a></td>
<td>Tally</td>
<td>Not match</td>
<td></td>
</tr>
<tr>
<td><code>⍴</code> <a href="glyphs/rho.qmd">Rho</a></td>
<td>Shape</td>
<td>Reshape</td>
<td><code>⍬⍴Y</code> gives a rank-0 array: <code>⍬⍴1 2</code> is
<code>⊂1</code></td>
</tr>
<tr>
<td><code>,</code> <a href="glyphs/comma.qmd">Comma</a></td>
<td>Ravel</td>
<td>Catenate</td>
<td></td>
</tr>
<tr>
<td><code>⍪</code> <a href="glyphs/table.qmd">Table</a></td>
<td>Table</td>
<td>Catenate first</td>
<td></td>
</tr>
<tr>
<td><code>⌽</code> <a href="glyphs/reverse.qmd">Reverse</a></td>
<td>Reverse last</td>
<td>Rotate last</td>
<td></td>
</tr>
<tr>
<td><code>⊖</code> <a href="glyphs/reverse-first.qmd">Reverse
first</a></td>
<td>Reverse first</td>
<td>Rotate first</td>
<td></td>
</tr>
<tr>
<td><code>⍉</code> <a href="glyphs/transpose.qmd">Transpose</a></td>
<td>Transpose</td>
<td>Reorder / diagonal axes</td>
<td></td>
</tr>
<tr>
<td><code>↑</code> <a href="glyphs/take.qmd">Take</a></td>
<td>First</td>
<td>Take</td>
<td>Monad as Dyalog <code>⎕ML≥2</code>, but takes a major cell:
<code>↑2 3⍴⍳6</code> is <code>0 1 2</code></td>
</tr>
<tr>
<td><code>↓</code> <a href="glyphs/drop.qmd">Drop</a></td>
<td>Split</td>
<td>Drop</td>
<td></td>
</tr>
<tr>
<td><code>↕</code> <a href="glyphs/windows.qmd">Windows</a></td>
<td>—</td>
<td>Leading-axis windows</td>
<td>As BQN’s <code>↕</code>, plus movements and padding (negative sizes)
as in <code>⌺</code>. Replaces windowed reduce: <code>+/2↕1 2 3 4</code>
is <code>3 5 7</code></td>
</tr>
<tr>
<td><code>⊂</code> <a href="glyphs/enclose.qmd">Enclose</a></td>
<td>Enclose</td>
<td>Partitioned enclose</td>
<td>Encloses atoms too, as BQN: <code>(⊂3)≡3</code> is
<code>0x</code></td>
</tr>
<tr>
<td><code>⊆</code> <a href="glyphs/nest.qmd">Nest</a></td>
<td>Nest</td>
<td>Partition</td>
<td></td>
</tr>
<tr>
<td><code>⊃</code> <a href="glyphs/mix.qmd">Mix</a></td>
<td>Mix</td>
<td>Pick</td>
<td>Monad as Dyalog <code>⎕ML≥2</code>. Pick takes cells:
<code>1⊃2 3⍴⍳6</code> is <code>3 4 5</code>. A string picks by <a
href="keyed.qmd">key</a></td>
</tr>
<tr>
<td><code>⌷</code> <a href="glyphs/squad.qmd">Squad</a></td>
<td>Identity</td>
<td>Index</td>
<td>Negative positions count from the end. <code>∞</code> takes a whole
axis and <code>¯∞</code> reverses it</td>
</tr>
<tr>
<td><code>⍳</code> <a href="glyphs/iota.qmd">Iota</a></td>
<td>Index generator</td>
<td>Index of</td>
<td><code>⍳⍠A Y</code> returns axis selectors. A negative length counts
down, as J: <code>⍳¯3</code> is <code>2 1 0</code></td>
</tr>
<tr>
<td><code>⍸</code> <a href="glyphs/where.qmd">Where</a></td>
<td>Where</td>
<td>Interval index</td>
<td>Dyad counts the boundaries at or below each value, as BQN’s
<code>⍋</code>: <code>1 3 5⍸0 3 9</code> is <code>0 2 3</code></td>
</tr>
<tr>
<td><code>∊</code> <a href="glyphs/member.qmd">Member</a></td>
<td>Enlist</td>
<td>Membership</td>
<td></td>
</tr>
<tr>
<td><code>∪</code> <a href="glyphs/union.qmd">Union</a></td>
<td>Unique</td>
<td>Union</td>
<td></td>
</tr>
<tr>
<td><code>∩</code> <a
href="glyphs/intersection.qmd">Intersection</a></td>
<td>—</td>
<td>Intersection</td>
<td></td>
</tr>
<tr>
<td><code>⍷</code> <a href="glyphs/find.qmd">Find</a></td>
<td>—</td>
<td>Find</td>
<td></td>
</tr>
<tr>
<td><code>⍋</code> <a href="glyphs/grade-up.qmd">Grade up</a></td>
<td>Grade up</td>
<td>Grade with collation</td>
<td></td>
</tr>
<tr>
<td><code>⍒</code> <a href="glyphs/grade-down.qmd">Grade down</a></td>
<td>Grade down</td>
<td>Grade with collation</td>
<td></td>
</tr>
<tr>
<td><code>/</code> <a href="glyphs/slash.qmd">Slash</a></td>
<td>—</td>
<td>Replicate last</td>
<td></td>
</tr>
<tr>
<td><code>⌿</code> <a href="glyphs/slash-bar.qmd">Slash bar</a></td>
<td>—</td>
<td>Replicate first</td>
<td></td>
</tr>
<tr>
<td><code>\</code> <a href="glyphs/backslash.qmd">Backslash</a></td>
<td>—</td>
<td>Expand last</td>
<td></td>
</tr>
<tr>
<td><code>⍀</code> <a href="glyphs/backslash-bar.qmd">Backslash
bar</a></td>
<td>—</td>
<td>Expand first</td>
<td></td>
</tr>
<tr>
<td><code>⊤</code> <a href="glyphs/encode.qmd">Encode</a></td>
<td>Binary encode</td>
<td>Encode</td>
<td>Monad as J’s <code>#:</code>, but digits run along the first
axis</td>
</tr>
<tr>
<td><code>⊥</code> <a href="glyphs/decode.qmd">Decode</a></td>
<td>Binary decode</td>
<td>Decode</td>
<td>Monad as J’s <code>#.</code>, but digits run along the first
axis</td>
</tr>
<tr>
<td><code>⌹</code> <a href="glyphs/domino.qmd">Domino</a></td>
<td>Matrix inverse</td>
<td>Matrix divide</td>
<td></td>
</tr>
<tr>
<td><code>⊣</code> <a href="glyphs/left.qmd">Left</a></td>
<td>Identity</td>
<td>Left</td>
<td></td>
</tr>
<tr>
<td><code>⊢</code> <a href="glyphs/right.qmd">Right</a></td>
<td>Identity</td>
<td>Right</td>
<td></td>
</tr>
<tr>
<td><code>⍎</code> <a href="glyphs/execute.qmd">Execute</a></td>
<td>Execute</td>
<td>Keyed lookup: <code>X⍎Y</code> is <code>Y⊃X</code></td>
<td>Dyad runs no code. Dyalog executes <code>Y</code> in namespace
<code>X</code></td>
</tr>
<tr>
<td><code>⍕</code> <a href="glyphs/format.qmd">Format</a></td>
<td>Format</td>
<td>Format by specification</td>
<td></td>
</tr>
</tbody>
</table>

## Operators

`f`, `g` are functions; `a`, `n`, `r` are arrays or numbers.

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
<th>Form</th>
<th>Meaning</th>
<th>Note</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>/</code> <a href="glyphs/slash.qmd">Slash</a></td>
<td><code>f/</code></td>
<td>Reduce / seeded reduce last</td>
<td>Seed as BQN’s <code>´</code>: <code>10 -/1 2 3</code> is
<code>¯8</code>. For windows use <code>↕</code></td>
</tr>
<tr>
<td><code>⌿</code> <a href="glyphs/slash-bar.qmd">Slash bar</a></td>
<td><code>f⌿</code></td>
<td>Reduce / seeded reduce first</td>
<td>Seed and windows as <code>/</code></td>
</tr>
<tr>
<td><code>\</code> <a href="glyphs/backslash.qmd">Backslash</a></td>
<td><code>f\</code></td>
<td>Scan / seeded scan last</td>
<td>Accumulates left to right as BQN’s Scan: <code>-\1 2 3</code> is
<code>1 ¯1 ¯4</code></td>
</tr>
<tr>
<td><code>⍀</code> <a href="glyphs/backslash-bar.qmd">Backslash
bar</a></td>
<td><code>f⍀</code></td>
<td>Scan / seeded scan first</td>
<td>Left to right as <code>\</code></td>
</tr>
<tr>
<td><code>¨</code> <a href="glyphs/each.qmd">Each</a></td>
<td><code>f¨</code></td>
<td>Each</td>
<td></td>
</tr>
<tr>
<td><code>⍨</code> <a href="glyphs/commute.qmd">Commute</a></td>
<td><code>f⍨</code>, <code>a⍨</code></td>
<td>Self / commute / constant</td>
<td></td>
</tr>
<tr>
<td><code>⊸</code> <a href="glyphs/before.qmd">Before</a></td>
<td><code>f⊸g</code>, <code>a⊸f</code></td>
<td>Before / bind left</td>
<td>As BQN’s <code>⊸</code></td>
</tr>
<tr>
<td><code>⟜</code> <a href="glyphs/after.qmd">After</a></td>
<td><code>f⟜g</code>, <code>f⟜a</code></td>
<td>After / bind right</td>
<td>As BQN’s <code>⟜</code></td>
</tr>
<tr>
<td><code>⍤</code> <a href="glyphs/rank.qmd">Rank</a></td>
<td><code>f⍤g</code>, <code>f⍤r</code></td>
<td>Atop / rank</td>
<td></td>
</tr>
<tr>
<td><code>⍠</code> <a href="glyphs/axis.qmd">Axis</a></td>
<td><code>f⍠A</code></td>
<td>Apply along axes</td>
<td>Axes are numbers or names, in place of Dyalog’s
<code>f[A]</code></td>
</tr>
<tr>
<td><code>⍥</code> <a href="glyphs/over.qmd">Over</a></td>
<td><code>f⍥g</code></td>
<td>Over</td>
<td></td>
</tr>
<tr>
<td><code>.</code> <a href="glyphs/dot.qmd">Dot</a></td>
<td><code>f.g</code></td>
<td>Inner product</td>
<td>Outer product is <code>g⌝</code>. After an array,
<code>T.name</code> is <code>"name"⊃T</code>, and <code>x.(I)</code> is
<code>(I)⌷x</code></td>
</tr>
<tr>
<td><code>⌝</code> <a href="glyphs/outer-product.qmd">Outer
product</a></td>
<td><code>g⌝</code></td>
<td>Outer product</td>
<td>As BQN’s <code>⌜</code>; <code>∘.g</code> is a <code>SYNTAX</code>
error</td>
</tr>
<tr>
<td><code>⌸</code> <a href="glyphs/key.qmd">Key</a></td>
<td><code>f⌸</code></td>
<td>Key</td>
<td></td>
</tr>
<tr>
<td><code>⍣</code> <a href="glyphs/power.qmd">Power</a></td>
<td><code>f⍣n</code>, <code>f⍣∞</code>, <code>f⍣g</code>,
<code>f⍣[g;]</code></td>
<td>Iterate / invert / fixed point / repeat until / history until</td>
<td>A list of counts gives the result for each count, as J’s
<code>^:</code> and BQN’s <code>⍟</code>: <code>(1+⍣3 ¯2 0 3)10</code>
is <code>13 8 10 13</code></td>
</tr>
<tr>
<td><code>⇄</code> <a href="glyphs/inverse-pair.qmd">Inverse
pair</a></td>
<td><code>f⇄g</code></td>
<td>Attach an explicit inverse</td>
<td>As J’s <code>:.</code></td>
</tr>
<tr>
<td><code>⌾</code> <a href="glyphs/under.qmd">Under</a></td>
<td><code>f⌾g</code></td>
<td>Transform, apply, inverse-transform</td>
<td>As J’s <code>&amp;.</code></td>
</tr>
<tr>
<td><code>∂</code> <a href="glyphs/derivative.qmd">Derivative</a></td>
<td><code>f∂</code></td>
<td>Gradient / vector–Jacobian product</td>
<td></td>
</tr>
<tr>
<td><code>◶</code> <a href="glyphs/agenda.qmd">Agenda</a></td>
<td><code>selector◶cases</code></td>
<td>Select and call one function</td>
<td>As BQN’s <code>◶</code></td>
</tr>
<tr>
<td><code>@</code> <a href="glyphs/at.qmd">At</a></td>
<td><code>f@a</code>, <code>a@g</code></td>
<td>Functional amend</td>
<td></td>
</tr>
<tr>
<td><code>⌺</code> <a href="glyphs/stencil.qmd">Stencil</a></td>
<td><code>f⌺a</code></td>
<td>Stencil</td>
<td></td>
</tr>
</tbody>
</table>

## Syntax and literals

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Form</th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>←</code> <a href="glyphs/assign.qmd">Assign</a></td>
<td>Assignment, including modified and selective forms</td>
</tr>
<tr>
<td><code>→</code> <a href="glyphs/pipe.qmd">Pipe</a></td>
<td>Left-to-right function application</td>
</tr>
<tr>
<td><code>(…)</code> <a
href="glyphs/parentheses.qmd">Parentheses</a></td>
<td>Grouping</td>
</tr>
<tr>
<td><code>[…]</code> <a href="glyphs/brackets.qmd">Brackets</a></td>
<td>Lists and arrays: <code>[a b c]</code>, <code>[a;b c]</code>,
<code>[1 2 ⋄ 3 4]</code>. One item only groups: <code>[x]</code> is
<code>x</code>, and <code>[x;]</code> is a one-item list</td>
</tr>
<tr>
<td><code>{…}</code> <a href="glyphs/braces.qmd">Braces</a></td>
<td>Defined function or operator</td>
</tr>
<tr>
<td><code>⍺</code> <a href="glyphs/alpha.qmd">Alpha</a>, <code>⍵</code>
<a href="glyphs/omega.qmd">Omega</a></td>
<td>Left / right argument</td>
</tr>
<tr>
<td><code>⍶</code> <a href="glyphs/alpha-underbar.qmd">Alpha
underbar</a>, <code>⍹</code> <a href="glyphs/omega-underbar.qmd">Omega
underbar</a></td>
<td>Left / right operand</td>
</tr>
<tr>
<td><code>∇</code> <a href="glyphs/del.qmd">Del</a>, <code>⍢</code> <a
href="glyphs/del-diaeresis.qmd">Del diaeresis</a></td>
<td>Function / operator self-reference</td>
</tr>
<tr>
<td><code>:</code> <a href="glyphs/colon.qmd">Colon</a>, <code>::</code>
<a href="glyphs/error-guard.qmd">Error guard</a></td>
<td>Unkey / key axes; Boolean guard / error guard inside dfns</td>
</tr>
<tr>
<td><code>⋄</code> <a href="glyphs/diamond.qmd">Diamond</a></td>
<td>Separates statements, and rows inside brackets</td>
</tr>
<tr>
<td><code>⍝</code> <a href="glyphs/comment.qmd">Comment</a></td>
<td>Comment</td>
</tr>
<tr>
<td><code>⎕←</code> <a href="glyphs/quad.qmd">Quad</a></td>
<td>Explicit output</td>
</tr>
<tr>
<td><code>•</code> <a href="glyphs/bullet.qmd">Bullet</a></td>
<td><a href="#system-names">System name</a> prefix</td>
</tr>
<tr>
<td><code>⍬</code> <a href="glyphs/zilde.qmd">Zilde</a></td>
<td>Empty numeric vector</td>
</tr>
<tr>
<td><code>'…'</code> <code>"…"</code> <a
href="glyphs/quote.qmd">Quote</a></td>
<td>Character and string literals</td>
</tr>
<tr>
<td><code>¯</code> <a href="glyphs/overbar.qmd">Overbar</a></td>
<td>Negative literal sign</td>
</tr>
<tr>
<td><code>∞</code> <a href="glyphs/infinity.qmd">Infinity</a></td>
<td>Real infinity</td>
</tr>
</tbody>
</table>

Numeric notation `x`, `r`, `j`, `E`: see [numbers](rules.qmd#numbers).

## System names

Names are case-insensitive. `•a` and `•d` are constant arrays; the other
names are functions, usable with operators and composition.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Name</th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>•a</code> <a href="glyphs/alphabet.qmd">Alphabet</a></td>
<td>Uppercase Latin alphabet</td>
</tr>
<tr>
<td><code>•d</code> <a href="glyphs/digits.qmd">Digits</a></td>
<td>Decimal digits</td>
</tr>
<tr>
<td><code>•c</code> <a href="glyphs/case.qmd">Case</a></td>
<td>Unicode case conversion</td>
</tr>
<tr>
<td><code>•csv</code> <a href="data.ipynb#csv">CSV</a></td>
<td>CSV text → keyed column vectors</td>
</tr>
<tr>
<td><code>•tocsv</code> <a href="data.ipynb#csv">CSV</a></td>
<td>Keyed column vectors → CSV text</td>
</tr>
<tr>
<td><code>•r</code> <a href="regex.ipynb">Regex</a></td>
<td>Compiled search, captures and replacement</td>
</tr>
<tr>
<td><code>•normal</code>, <code>•binomial</code>, … <a
href="distributions.ipynb">Distributions</a></td>
<td>Sampling, density, CDF and quantiles; 17 families</td>
</tr>
<tr>
<td><code>•json</code> <a href="data.ipynb#json">JSON</a></td>
<td>JSON text → arrays and keyed vectors</td>
</tr>
<tr>
<td><code>•tojson</code> <a href="data.ipynb#json">JSON</a></td>
<td>Arrays and keyed vectors → JSON text</td>
</tr>
<tr>
<td><code>•vfi</code> <a href="data.ipynb#numeric-input">Numeric
input</a></td>
<td>Parse numeric fields with a validity mask</td>
</tr>
<tr>
<td><code>•nget</code> <a href="data.ipynb#files">Read</a></td>
<td>Read UTF-8 text or bytes</td>
</tr>
<tr>
<td><code>•nput</code> <a href="data.ipynb#files">Write</a></td>
<td>Write UTF-8 text or bytes</td>
</tr>
<tr>
<td><code>•element</code>, <code>•xml</code>, <code>•svg</code> <a
href="xml.ipynb">XML and SVG</a></td>
<td>Build, serialize and display element trees</td>
</tr>
<tr>
<td><code>•mime</code> <a href="xml.ipynb#rich-display">Rich
display</a></td>
<td>The MIME bundle that display uses</td>
</tr>
<tr>
<td><code>•plot</code> <a href="plot.ipynb">Plots</a></td>
<td>Charts from vectors, matrices and tables</td>
</tr>
<tr>
<td><code>•ucs</code> <a href="glyphs/unicode.qmd">Unicode</a></td>
<td>Unicode code points / encodings</td>
</tr>
<tr>
<td><code>•load</code> <a href="glyphs/load.qmd">Load</a></td>
<td>Evaluate an APL source file</td>
</tr>
<tr>
<td><code>•signal</code> <a
href="glyphs/error-guard.qmd#signal">Signal</a></td>
<td>Raise an ordinary APL error</td>
</tr>
<tr>
<td><code>•nc</code> <a href="introspection.ipynb">Name class</a></td>
<td>Classify visible names</td>
</tr>
<tr>
<td><code>•nl</code> <a href="introspection.ipynb">Name list</a></td>
<td>List names by class and prefix</td>
</tr>
<tr>
<td><code>•src</code> <a href="introspection.ipynb">Source</a></td>
<td>Function/operator source</td>
</tr>
<tr>
<td><code>•ex</code> <a href="introspection.ipynb">Expunge</a></td>
<td>Erase bindings</td>
</tr>
</tbody>
</table>

Index origin: 0. Negative positions and axes count from the end.
Comparison tolerance: `1E¯14`.

## Language-wide differences from Dyalog APL

- [Lists](glyphs/brackets.qmd): brackets write lists, `[a b c]`, and a
  list of literals can leave them out, `1 2 3`. Brackets round one item
  only group it, so `[x]` is `x`. Nothing else strands, so arrays next
  to each other don’t form a list.
- [Units](rules.qmd): outside brackets, a space separates units, and
  each unit is evaluated first: `a+b × c+d` is `(a+b)×(c+d)`. A unit
  that ends in a function is a left section: `2×`.
- [Array application](glyphs/squad.qmd): an array next to an argument
  selects from it, as K does: `v 0` is the first item of `v`.
- [Atoms](rules.qmd#arrays-nesting-and-fill): numbers, characters and
  functions are atoms, distinct from scalars, as BQN. Enclosing always
  adds a layer: `(⊂3)≡3` is `0x`.
- [Numbers](rules.qmd#numbers): bare numbers are approximate. `x` and
  `r` mark exact integers and rationals, as J: `1x÷3x` is `1r3`.
  Predicates, positions, tally and shape are exact.
- [Agreement](rules.qmd#agreement-and-pervasion): scalar functions, Each
  and Rank align leading axes, and unit dimensions expand:
  `(2 3⍴⍳6)+10 20` is `[10 11 12 ⋄ 23 24 25]`.
- [System names](#system-names): written `•name`, as BQN, not `⎕NAME`.
  Most Dyalog system functions and variables are not included.
- [Axis keys](keyed.qmd) name data. `K:Y` evaluates its keys. `T.a` is
  `"a"⊃T`, and `X⍎Y` is `Y⊃X`.
