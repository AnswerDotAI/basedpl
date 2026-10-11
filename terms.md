

# Terms

The [Language overview](overview.ipynb) introduces these terms
informally, as they come up.

Every [value](#term-value) is an [atom](#term-atom) or an
[array](#term-array): atoms are [numbers](#term-number),
[characters](#term-character), [functions](#term-function) and
[operators](#term-operator), [scalars](#term-scalar) are arrays of
[rank](#term-rank) 0, [units](#term-unit) are atoms and scalars, and
[subjects](#term-subject) are all values except functions and operators.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Term</th>
<th>Meaning</th>
</tr>
</thead>
<tbody>
<tr>
<td><span id="term-value">value</span></td>
<td>a number, character, function, operator or array</td>
</tr>
<tr>
<td><span id="term-atom">atom</span></td>
<td>a value that isn’t an array: a number, character, function or
operator</td>
</tr>
<tr>
<td><span id="term-number">number</span></td>
<td>an approximate real or complex number, or an exact integer or
rational, as <a href="numbers.qmd">Numbers</a> describes</td>
</tr>
<tr>
<td><span id="term-character">character</span></td>
<td>one Unicode character, written in single quotes:
<code>'a'</code></td>
</tr>
<tr>
<td><span id="term-primitive">primitive</span></td>
<td>a built-in function or operator, written as one glyph:
<code>+</code> or <code>/</code></td>
</tr>
<tr>
<td><span id="term-dfn">dfn</span></td>
<td>a function or operator written in braces, as <a
href="dfns.ipynb">Dfns</a> describes: <code>{⍵+1}</code></td>
</tr>
<tr>
<td><span id="term-function">function</span></td>
<td>a primitive, train, derived function, or dfn that doesn’t use
<code>⍶</code> or <code>⍹</code>, which applies to arguments</td>
</tr>
<tr>
<td><span id="term-operator">operator</span></td>
<td>a primitive, or a dfn that uses <code>⍶</code> or <code>⍹</code>,
which takes operands and derives a function: <code>/</code> derives
<code>+/</code> from <code>+</code></td>
</tr>
<tr>
<td><span id="term-derived-function">derived function</span></td>
<td>a function that an operator derives: <code>+/</code>, or
<code>2↣×</code></td>
</tr>
<tr>
<td><span id="term-pervasive-function">pervasive function</span></td>
<td>a function that applies to each atom of its arguments, through any
nesting: <code>≥[2 ⊂3]</code> is <code>[3 ⊂4]</code></td>
</tr>
<tr>
<td><span id="term-array">array</span></td>
<td>a rectangular collection of values along zero or more axes</td>
</tr>
<tr>
<td><span id="term-rank">rank</span></td>
<td>the number of axes: a vector has rank 1, and atoms and scalars have
rank 0</td>
</tr>
<tr>
<td><span id="term-vector">vector</span></td>
<td>an array of rank 1</td>
</tr>
<tr>
<td><span id="term-matrix">matrix</span></td>
<td>an array of rank 2</td>
</tr>
<tr>
<td><span id="term-string">string</span></td>
<td>a vector of characters, written in double quotes:
<code>"abc"</code></td>
</tr>
<tr>
<td><span id="term-scalar">scalar</span></td>
<td>an array of rank 0, holding one value: <code>⊂5</code> is a scalar
holding the number 5, and <code>5</code> is a number</td>
</tr>
<tr>
<td><span id="term-unit">unit</span></td>
<td>a rank-0 value: a number, character, function, operator or
scalar</td>
</tr>
<tr>
<td><span id="term-subject">subject</span></td>
<td>a value that isn’t a function or operator: a number, character or
array</td>
</tr>
<tr>
<td><span id="term-item">item</span></td>
<td>the value at one position of an array: <code>[1 [2 3]]</code> has
the items <code>1</code> and <code>[2 3]</code></td>
</tr>
<tr>
<td><span id="term-position">position</span></td>
<td>an index along an axis, counting from 0. Negative positions count
from the end</td>
</tr>
<tr>
<td><span id="term-axis">axis</span></td>
<td>one of the dimensions of an array, numbered from 0. Each axis has a
length, and can have a name and keys</td>
</tr>
<tr>
<td><span id="term-key">key</span></td>
<td>a string that labels a position along an axis, as <a
href="keyed.ipynb">Keyed arrays</a> describes</td>
</tr>
<tr>
<td><span id="term-record">record</span></td>
<td>a vector with keys, written as <code>key:value</code> items in
brackets: <code>["a":1 "b":2]</code></td>
</tr>
<tr>
<td><span id="term-cell">cell</span></td>
<td>a sub-array made of an array’s last k axes, called a k-cell: a
matrix’s rows are its 1-cells. A 0-cell is the unit at one position, the
atom there or the array there enclosed. When k is at least the array’s
rank, the whole array is its one cell</td>
</tr>
<tr>
<td><span id="term-major-cell">major cell</span></td>
<td>a cell made of every axis but the first: a matrix’s rows are its
major cells</td>
</tr>
<tr>
<td><span id="term-frame">frame</span></td>
<td>the axes before the cells, with one cell at each position</td>
</tr>
<tr>
<td><span id="term-assembly">assembly</span></td>
<td>building one array from results, one for each position of a frame:
the shape is the frame followed by the results’ shape, each result fills
its position with its items, and shorter results are padded with
fill</td>
</tr>
<tr>
<td><span id="term-fill">fill</span></td>
<td>the value an operation uses where an array has no item, as when
<code>5↑1 2</code> gives <code>1 2 0 0 0</code>. It comes from the
array’s prototype: its first item with each number made 0, each
character made a space, and each function kept</td>
</tr>
<tr>
<td><span id="term-selection">selection</span></td>
<td>values read from an argument together with their places in it, as a
list of positions or a function such as <code>1↓</code> reads them</td>
</tr>
<tr>
<td><span id="term-glyph">glyph</span></td>
<td>a symbol for a primitive or for syntax, as the <a
href="glyphs.qmd">glyph reference</a> lists</td>
</tr>
<tr>
<td><span id="term-name">name</span></td>
<td>a word that refers to a value, as <a
href="syntax.qmd#names">Syntax</a> describes: <code>x</code> or
<code>avg</code></td>
</tr>
<tr>
<td><span id="term-statement">statement</span></td>
<td>the code between two newlines or <code>⋄</code> separators. Inside
brackets and parentheses, these separate rows instead</td>
</tr>
<tr>
<td><span id="term-literal">literal</span></td>
<td>a number, character, string, <code>∞</code>, <code>⍬</code>, or a
constant: <code>$t</code> true, <code>$f</code> false or <code>$n</code>
NaN</td>
</tr>
<tr>
<td><span id="term-group">group</span></td>
<td>brackets, parentheses or braces with what they hold:
<code>(1+2)</code>, <code>[a b]</code> or <code>{⍵}</code></td>
</tr>
<tr>
<td><span id="term-token">token</span></td>
<td>a glyph, name, literal or group</td>
</tr>
<tr>
<td><span id="term-run">run</span></td>
<td>a sequence of tokens with no spaces between them</td>
</tr>
<tr>
<td><span id="term-strand">strand</span></td>
<td>subjects side by side, read as a vector without brackets:
<code>1 2 3</code> is <code>[1 2 3]</code>, and <code>a b</code> is
<code>[a b]</code></td>
</tr>
<tr>
<td><span id="term-argument">argument</span></td>
<td>a subject a function applies to</td>
</tr>
<tr>
<td><span id="term-operand">operand</span></td>
<td>a value an operator applies to</td>
</tr>
<tr>
<td><span id="term-monadic">monadic</span></td>
<td>taking one argument, or for an operator, one operand:
<code>-5</code> is a monadic call</td>
</tr>
<tr>
<td><span id="term-dyadic">dyadic</span></td>
<td>taking two arguments, or for an operator, two operands:
<code>3-5</code> is a dyadic call</td>
</tr>
<tr>
<td><span id="term-expression">expression</span></td>
<td>a run, group or statement that ends in a subject</td>
</tr>
<tr>
<td><span id="term-train">train</span></td>
<td>a run, group or statement that ends in a function</td>
</tr>
<tr>
<td><span id="term-part">part</span></td>
<td>one of the pieces a train is built from: a literal strand, or a
token with the operators bound to it, such as <code>2</code>,
<code>+/</code> or <code>(+×-)</code></td>
</tr>
<tr>
<td><span id="term-fork">fork</span></td>
<td>three parts of a train, <code>f g h</code>, giving
<code>(f ⍵) g (h ⍵)</code></td>
</tr>
<tr>
<td><span id="term-tine">tine</span></td>
<td>one of the two outer parts of a fork</td>
</tr>
<tr>
<td><span id="term-atop">atop</span></td>
<td>two parts of a train, <code>f g</code>, giving
<code>f (g ⍵)</code></td>
</tr>
<tr>
<td><span id="term-bind">bind</span></td>
<td>a subject directly before a part, fixing that part’s left argument:
<code>2×</code> is <code>2↣×</code>, and in <code>2\|#⊢</code> the
<code>2</code> binds <code>\|</code></td>
</tr>
</tbody>
</table>
