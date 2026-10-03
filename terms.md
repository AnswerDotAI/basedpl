

# Terms

The [Language overview](overview.ipynb) introduces these terms
informally, as they come up.

Every [value](#term-value) is an [atom](#term-atom) or an
[array](#term-array): atoms are [numbers](#term-number),
[characters](#term-character) and [functions](#term-function),
[scalars](#term-scalar) are arrays of [rank](#term-rank) 0,
[units](#term-unit) are atoms and scalars, and [subjects](#term-subject)
are all values except functions.

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
<td>a number, character, function or array</td>
</tr>
<tr>
<td><span id="term-atom">atom</span></td>
<td>a value that isn’t an array: a number, character or function</td>
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
<td><span id="term-function">function</span></td>
<td>a primitive, dfn, train or derived function, which applies to
arguments</td>
</tr>
<tr>
<td><span id="term-pervasive-function">pervasive function</span></td>
<td>a function that applies to each atom of its arguments, through any
nesting: <code>1+[2 ⊂3]</code> is <code>[3 ⊂4]</code></td>
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
<td><span id="term-scalar">scalar</span></td>
<td>an array of rank 0, holding one value: <code>⊂5</code> is a scalar
holding the number 5, and <code>5</code> is a number</td>
</tr>
<tr>
<td><span id="term-unit">unit</span></td>
<td>a rank-0 value: a number, character, function or scalar</td>
</tr>
<tr>
<td><span id="term-subject">subject</span></td>
<td>a value that isn’t a function: a number, character or array</td>
</tr>
<tr>
<td><span id="term-cell">cell</span></td>
<td>a sub-array made of an array’s last k axes, called a k-cell: a
matrix’s rows are its 1-cells. A 0-cell is the unit at one position, the
atom there or the array there enclosed. When k is at least the array’s
rank, the whole array is its one cell</td>
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
<td><span id="term-literal">literal</span></td>
<td>a number, character, string, <code>∞</code>, <code>⍬</code>, or a
constant: <code>$t</code> true, <code>$f</code> false or <code>$n</code>
NaN</td>
</tr>
<tr>
<td><span id="term-strand">strand</span></td>
<td>subjects side by side, read as a vector without brackets:
<code>1 2 3</code> is <code>[1 2 3]</code>, and <code>a b</code> is
<code>[a b]</code></td>
</tr>
<tr>
<td><span id="term-run">run</span></td>
<td>a sequence of tokens with no spaces between them</td>
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
<td><span id="term-expression">expression</span></td>
<td>a run, group or statement that ends in a subject</td>
</tr>
<tr>
<td><span id="term-train">train</span></td>
<td>a run, group or statement that ends in a function</td>
</tr>
<tr>
<td><span id="term-fork">fork</span></td>
<td>three parts of a train, <code>f g h</code>, giving
<code>(f ⍵) g (h ⍵)</code>, or <code>A g h</code> with a subject on the
left, giving <code>A g (h ⍵)</code></td>
</tr>
<tr>
<td><span id="term-atop">atop</span></td>
<td>two parts of a train, <code>f g</code>, giving
<code>f (g ⍵)</code></td>
</tr>
<tr>
<td><span id="term-bind">bind</span></td>
<td>a subject directly before a function in a train, fixing its left
argument: <code>2×</code> is <code>2↣×</code></td>
</tr>
</tbody>
</table>
