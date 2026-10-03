

# System functions

A system function is a built-in function whose name starts with
[`•`](glyphs/bullet.qmd), as in `•ucs` or `•json`. Type `•` with Alt-y.
Names are case-insensitive. `•a` and `•d` are constant arrays, and every
other name is a function. Many system functions take their options as a
left argument. `]help •name` shows the full help for one name.

In Python, the workspace object `bpl` has each system function under its
name without `•`. Options become keyword arguments, as in
`bpl.json('{"a": null}', fill=0)`. [Python](python.ipynb) covers the
calling conventions.

<!-- help -->

## Text

<!-- help •a -->

### `•a` — Alphabet

`•a` is the uppercase Latin alphabet. It’s read-only.

``` bpl
3↑•a               ⍝ "ABC"
•a⍳"CAB"           ⍝ [2 0 1]ₓ
```

<!-- help •d -->

### `•d` — Digits

`•d` is `"0123456789"`. It’s read-only.

``` bpl
"a2b"∊•d           ⍝ $f $t $f
```

<!-- help •c -->

### `•c` — Case

`•c Y` folds case. `1•c Y` converts to upper case, and `¯1•c Y` to lower
case. `•c` uses Unicode’s simple case mappings. It keeps the argument’s
shape and nesting, and leaves items that aren’t characters unchanged.

``` bpl
•c "AbC"           ⍝ "abc"
1•c "AbC"          ⍝ "ABC"
```

<!-- help •ucs -->

### `•ucs` — Unicode

`•ucs Y` converts characters to exact integer code points, and code
points to characters, keeping the shape of `Y`. `E•ucs Y` encodes
characters, or decodes integer code units, where `E` is `"UTF-8"`,
`"UTF-16"` or `"UTF-32"`. The encoding forms take vectors. An invalid
code point or a malformed encoding is a `DOMAIN` error.

``` bpl
•ucs "ABC"         ⍝ [65 66 67]ₓ
•ucs 65 66 67      ⍝ "ABC"
"UTF-8"•ucs 'é'    ⍝ [195 169]ₓ
"UTF-8"•ucs 195 169 ⍝ "é"
```

<!-- help •r -->

### `•r` — Regular expressions

`•r pattern` compiles a Rust regex. It returns a keyed vector of
functions that share the pattern: `match`, `position`, `length`,
`groups` and `replace`. Positions count characters from 0.

`template p.replace text` expands `$0`, `$1`, `${name}` and `$$` in
`template`. Flags go in the pattern, as in `(?i)`. [Regular
expressions](regex.ipynb) covers each function.

Errors: `DOMAIN` for invalid patterns, including look-around and
backreferences, and for non-text arguments; `SYNTAX` for the wrong
valence; `LIMIT` for oversized results.

<!-- help -->

## Data and files

<!-- help •csv -->

### `•csv` — Read CSV

`•csv text` parses CSV into a vector of columns. Headers become keys.
Numeric columns become numbers. Missing numeric cells become NaN. An
integer column with missing cells stays exact. Missing text cells become
`""`.

`X •csv text` takes options on the left: `header`, `separator`,
`quotechar`, `doublequote`, `escapechar`, `decimal`, `thousands`,
`trim`, `fill`, `text_columns`, `numeric_columns` and `missing`.
[CSV](data.ipynb#csv) describes them.

Errors: `DOMAIN` for invalid options or duplicate headers; `LENGTH` for
unequal record widths.

<!-- help •tocsv -->

### `•tocsv` — Write CSV

`•tocsv T` returns CSV text for a vector of columns. Keys supply the
header. Column lengths must agree.

`X •tocsv T` takes options on the left: `header`, `separator`,
`quotechar`, `doublequote`, `escapechar`, `decimal`, `thousands`,
`trim`, `fill`, `forcequotes` and `lineending`. NaN writes as an empty
cell, and so does the exact value given as `fill`. `"forcequotes":2`
quotes every field. [CSV](data.ipynb#csv) describes the options.

Errors: `DOMAIN` for nonintegral rationals, complex numbers, functions
or nested cells; `LENGTH` for unequal columns.

<!-- help •json -->

### `•json` — Read JSON

`•json text` parses JSON. Objects become keyed vectors. Arrays become
vectors. Strings become character vectors. Integers stay exact. `true`
and `false` become `$t` and `$f`.

`["fill":v] •json text` replaces `null` with `v`. The default is NaN. An
integer array with a `null` stays exact. [JSON](data.ipynb#json) covers
the conversions.

Errors: `DOMAIN` for malformed JSON, with its line and column.

<!-- help •tojson -->

### `•tojson` — Write JSON

`•tojson Y` returns JSON text. Keyed axes become objects. Unkeyed axes
become arrays. Character vectors become strings. Keyed entries that hold
functions are left out.

NaN exports as `null`. `["fill":v] •tojson Y` also writes `v` as `null`.
[JSON](data.ipynb#json) covers the conversions.

Errors: `DOMAIN` for an infinity that isn’t `fill`, out-of-range floats,
nonintegral rationals, complex numbers and other functions.

<!-- help •vfi -->

### `•vfi` — Numeric input

`•vfi text` returns two vectors, `[valid numbers]`, for the
whitespace-separated fields of `text`. An invalid field has the flag
`0ₓ` and the value `0`. Fields are parsed as numbers, never executed.

`separators •vfi text` splits on each character in `separators` instead.
It trims whitespace around fields. An empty field is a valid `0`.
[Numeric input](data.ipynb#numeric-input) covers it.

Errors: `DOMAIN` when either argument is not text.

<!-- help •nget -->

### `•nget` — Read a file

`•nget path` reads a UTF-8 text file. `•nget "-"` reads the rest of
standard input. A path that starts with `./` or `../` is relative to the
file holding the code, as for [`•load`](#load).

`X •nget path` takes options on the left: `binary` (`1` reads a vector
of byte values) and `encoding` (`"UTF-8"`). [Files](data.ipynb#files)
covers bytes and options.

Errors: `VALUE` for missing files, invalid UTF-8, other file errors and
standard input that the frontend doesn’t provide; `DOMAIN` for invalid
options.

<!-- help •nput -->

### `•nput` — Write a file

`path •nput data` writes `data` to a new file. It returns the number of
bytes written. Text is written as UTF-8. An array of numbers is written
as bytes, each from 0 to 255, in ravel order. A path that starts with
`./` or `../` is relative to the file holding the code, as for
[`•load`](#load).

`X •nput data` takes options on the left: `path`, `overwrite` (`1`
replaces an existing file) and `encoding` (`"UTF-8"`, for text). Plain
text on the left is the path. [Files](data.ipynb#files) covers bytes and
options.

Errors: `VALUE` for an existing file without `overwrite`, and for other
file errors; `DOMAIN` for invalid options or byte values.

<!-- help -->

## Display

<!-- help •element -->

### `•element` — XML elements

`•element tag` returns an element function for the XML tag `tag`. Call
it with attributes on the left and children on the right, as in
`["r":10] circle ""`. An empty right argument, `""` or `⍬`, gives no
children. The result is a keyed vector with `tag`, `attrs` and
`children` entries. Notebooks display it as HTML. [XML and
SVG](xml.ipynb) covers element trees.

Errors: `DOMAIN` for invalid tag or attribute names.

<!-- help •xml -->

### `•xml` — XML text

`•xml tree` returns the XML text of an element tree. It escapes `&`,
`<`, `>` and `"` in text and attribute values. A numeric vector
attribute becomes space-separated numbers. Children are text, elements
or vectors of children. An element with no children closes itself. [XML
and SVG](xml.ipynb) covers element trees.

Errors: `DOMAIN` for invalid names or attribute values.

<!-- help •svg -->

### `•svg` — SVG pictures

`X •svg children` returns an `svg` element with attributes `X`. It adds
`xmlns` for the SVG namespace and `viewBox="0 0 100 100"`. Attributes in
`X` replace these. Notebooks display it as a picture. [XML and
SVG](xml.ipynb) covers element trees.

<!-- help •mime -->

### `•mime` — Rich display

`•mime Y` returns the MIME bundle that display uses for `Y`. The bundle
is a keyed vector from MIME types to text. It always has `text/plain`.
If `Y` has a renderer, `•mime` calls it with `Y` as `⍵` and adds its
entries.

`F •mime Y` returns `Y` with the renderer that `F` holds, as in
`{["text/html":"<b>",(⍕⍵),"</b>"]}ᵘ •mime Y`. A MIME type on the left
displays `Y` itself as that type, as in `"text/markdown" •mime "*hi*"`.
An atom becomes a scalar, because only an array can hold a renderer.

A result keeps a renderer when it has an item for each item of its
argument, as pervasive functions, Each and scans do. Rearranging or
selecting from an array without removing an axis also keeps it, as
reversal, transpose, take, replicate and catenation do. Changes in place
keep it. Other functions drop it, as reductions and `⍴` do. When two
arguments have different renderers, the result has none. Match ignores
renderers.

Display shows the text form when a renderer fails. Only a direct `•mime`
call reports the error. [Rich display](xml.ipynb#rich-display) covers
renderers.

Errors: `DOMAIN` for a left argument that holds neither a function nor
text.

<!-- help •plot -->

### `•plot` — Plots

`X •plot Y` returns a plot spec: a keyed vector holding the data `Y` and
the settings `X`. Its renderer displays the spec as an SVG chart. Plain
text on the left is shorthand for `mark`. [Plots](plot.ipynb) shows each
mark and setting with examples.

The structure of `Y` chooses the series and axes:

- A vector plots its values against `0…n-1`.
- A keyed vector of numbers uses its keys as x labels.
- A matrix plots one series per row. Row keys name the series. Column
  keys label x.
- With the `cell` mark, each row of a matrix is a row of cells, coloured
  by value. Row keys label the rows, and row 0 is at the top.
- A table, a keyed vector of equal-length columns, plots each column as
  a series. A column named `x` supplies the x values.
- A vector or matrix of plots draws a figure.

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th>Setting</th>
<th>Holds</th>
<th>Default</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>mark</code></td>
<td><code>"line"</code>, <code>"point"</code>, <code>"bar"</code> or
<code>"cell"</code></td>
<td><code>"line"</code></td>
</tr>
<tr>
<td><code>title</code></td>
<td>Chart title</td>
<td>none</td>
</tr>
<tr>
<td><code>width</code>, <code>height</code></td>
<td>Size in pixels</td>
<td><code>600</code>, <code>400</code></td>
</tr>
<tr>
<td><code>x</code>, <code>y</code></td>
<td><code>title</code>, <code>scale</code> (<code>"linear"</code> or
<code>"log"</code>), <code>ticks</code>, and <code>axis</code>
(<code>$f</code> hides that axis)</td>
<td></td>
</tr>
<tr>
<td><code>legend</code></td>
<td><code>position</code> (<code>"end"</code> or a corner) and
<code>border</code></td>
<td>none</td>
</tr>
<tr>
<td><code>grid</code></td>
<td><code>$f</code> hides the grid lines</td>
<td><code>$t</code>, or <code>$f</code> for cells</td>
</tr>
<tr>
<td><code>axes</code></td>
<td><code>$f</code> hides both axes, with their ticks and titles. An
<code>axis</code> setting in <code>x</code> or <code>y</code> overrides
it for that axis</td>
<td><code>$t</code></td>
</tr>
<tr>
<td><code>flip</code></td>
<td><code>$t</code> swaps the axes</td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>palette</code></td>
<td>Colours for numbers: <code>"viridis"</code>, <code>"gray"</code>, or
a list of colours</td>
<td><code>"viridis"</code></td>
</tr>
<tr>
<td><code>colorbar</code></td>
<td><code>$t</code> shows the colour scale beside the plot</td>
<td><code>$f</code></td>
</tr>
<tr>
<td><code>color</code>, <code>size</code>, <code>labels</code></td>
<td>Styles for every series</td>
<td></td>
</tr>
<tr>
<td><code>series</code></td>
<td>Styles for one series, keyed by its name</td>
<td></td>
</tr>
<tr>
<td><code>widths</code>, <code>heights</code>, <code>share</code></td>
<td>Figure cell sizes and shared axis ranges</td>
<td></td>
</tr>
</tbody>
</table>

`color` also takes one number per point. `palette` turns these numbers
into colours, over the range of every series in the plot. A cell takes
its colour from its own value unless `color` is set.

A direct [`•mime`](#mime) call on a spec reports these errors: `DOMAIN`
for unknown settings or values, and for cells mixed with other marks;
`LENGTH` when series, colours, sizes or labels don’t match the x values;
`RANK` for data that isn’t a vector, matrix or table.

<!-- help •image -->

### `•image` — Images

`•image Y` returns a picture: numbers from 0 to 1, with axes for rows,
columns and up to four channels, that notebooks display as an image. One
channel is grey, and three are red, green and blue. A second or fourth
channel is alpha. `Y` is the path of a PNG or JPEG file, the bytes of
one, or the numbers themselves. Values outside 0 to 1 are clipped when
the picture is displayed or encoded.

`kind •image Y` encodes the picture `Y` as `"png"` or `"jpeg"` bytes.
JPEG drops alpha. [`•nput`](#nput) writes the bytes to a file, as in
`"out.png" •nput "png" •image Y`.

Errors: `DOMAIN` for an unknown `kind`, invalid image bytes, or values
that aren’t real numbers; `RANK` for a shape that isn’t a picture;
`VALUE` for file errors.

<!-- help •prefs -->

### `•prefs` — Display settings

`•prefs Y` applies the settings in the record `Y` to this session’s
display, and returns every setting. `•prefs ⍬` changes nothing.

<table>
<colgroup>
<col style="width: 33%" />
<col style="width: 33%" />
<col style="width: 33%" />
</colgroup>
<thead>
<tr>
<th>Setting</th>
<th>Holds</th>
<th>Default</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>box</code></td>
<td><code>$t</code> draws arrays in boxes</td>
<td><code>$t</code> in the REPL and the BPL Jupyter kernel,
<code>$f</code> elsewhere</td>
</tr>
<tr>
<td><code>trees</code></td>
<td><code>$t</code> shows a function as a tree while <code>box</code> is
on</td>
<td>as <code>box</code></td>
</tr>
<tr>
<td><code>fns</code></td>
<td><code>$f</code> leaves output made inside functions unboxed</td>
<td>as <code>box</code></td>
</tr>
<tr>
<td><code>limit</code></td>
<td>The most items display shows in full. <code>∞</code> shows every
item</td>
<td><code>1000</code></td>
</tr>
<tr>
<td><code>edges</code></td>
<td>Positions shown at each end of a long axis</td>
<td><code>3</code></td>
</tr>
</tbody>
</table>

Display shows part of an array of more than `limit` items: the first and
last `edges` positions of each axis longer than twice `edges`. `…`
replaces the hidden columns, `⋮` the hidden rows, and `⋱` sits where
they cross. A nested array follows the same rule on its own. `⎕←`,
[`⍕`](glyphs/format.qmd) and source text always hold every item.

Errors: `DOMAIN` for an unknown setting or an invalid value.
<!-- help -->

## Random numbers and distributions

<!-- help •rand -->

### `•rand` — Generator

`•rand seed` returns a generator: a keyed vector of two functions that
draw from one stream of random numbers. The seed is a nonnegative
integer. The same seed gives the same draws.

- `roll Y` works like `¿Y`.
- `X deal Y` works like `X¿Y`.

A distribution’s `sample` takes a generator on its left, as in
`g d.sample 3`. Copies of a generator share its stream. Drawing from one
copy moves every copy on.

Errors: `DOMAIN` for a seed that is not a nonnegative integer, or a left
argument to `sample` that is not a generator; `LENGTH` or `RANK` for
more than one seed; `SYNTAX` for a dyadic call to `•rand`.

<!-- help •normal •uniform •beta •bernoulli •binomial •cauchy •chisquared •exponential •fisher •gamma •inversegamma •laplace •lognormal •logistic •poisson •student •weibull -->

### Distributions

A distribution constructor, such as `•normal 0 1`, returns a keyed
vector of four functions:

- `sample shape` draws random values. `g sample shape` draws them from a
  generator made by `•rand`.
- `density x` gives the probability density, or the probability mass for
  a discrete distribution.
- `cdf x` gives P(X ≤ x).
- `quantile p` inverts the CDF.

Parameters are finite real units or vectors. Scale, shape, rate and
degrees of freedom are positive, except where stated.
[Distributions](distributions.ipynb) shows them in use.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Constructor</th>
<th>Parameters</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>•normal</code></td>
<td>μ σ: mean, standard deviation</td>
</tr>
<tr>
<td><code>•uniform</code></td>
<td>a b: lower and upper bounds, a &lt; b</td>
</tr>
<tr>
<td><code>•bernoulli</code></td>
<td>p ∈ [0,1]</td>
</tr>
<tr>
<td><code>•binomial</code></td>
<td>n p: integer trials n ≥ 0, p ∈ [0,1]</td>
</tr>
<tr>
<td><code>•poisson</code></td>
<td>λ ≥ 0: mean</td>
</tr>
<tr>
<td><code>•beta</code></td>
<td>α β: shapes</td>
</tr>
<tr>
<td><code>•gamma</code></td>
<td>k θ: shape, scale, with mean kθ</td>
</tr>
<tr>
<td><code>•inversegamma</code></td>
<td>α β: shape, scale, with density ∝ x⁻⁽ᵅ⁺¹⁾ exp(−β/x)</td>
</tr>
<tr>
<td><code>•exponential</code></td>
<td>λ: rate, with mean 1/λ</td>
</tr>
<tr>
<td><code>•chisquared</code></td>
<td>ν: degrees of freedom</td>
</tr>
<tr>
<td><code>•student</code></td>
<td>ν: degrees of freedom, with location 0 and scale 1</td>
</tr>
<tr>
<td><code>•fisher</code></td>
<td>ν₁ ν₂: degrees of freedom</td>
</tr>
<tr>
<td><code>•cauchy</code>, <code>•laplace</code>,
<code>•logistic</code></td>
<td>location, scale</td>
</tr>
<tr>
<td><code>•lognormal</code></td>
<td>μ σ: mean and standard deviation of log(X)</td>
</tr>
<tr>
<td><code>•weibull</code></td>
<td>k λ: shape, scale</td>
</tr>
</tbody>
</table>

Errors: `DOMAIN` for invalid parameters, non-real inputs or p ∉ \[0,1\];
`LENGTH` for the wrong number of parameters; `RANK` for matrix
parameters or shapes; `SYNTAX` for dyadic calls other than `sample`;
`LIMIT` for oversized shapes or sampler ranges.

<!-- help -->

## Programs

<!-- help •load -->

### `•load` — Load

`•load path` runs the BPL file at `path` as a [module](modules.qmd),
with names of its own. It returns a record of the module’s public names:
the names its top level assigns that don’t start with `_`. The file adds
no names to the caller, and shows nothing except explicit output. Each
call runs the file again.

Destructure the record to take names, as in `[a b]←•load path`, or keep
it and read names with a dot, as in `m←•load path` and then `m.a`.

A path that starts with `./` or `../` is relative to the file holding
the code. Other relative paths are relative to the working directory.
`•nget` and `•nput` follow the same rule.

Errors: `VALUE` for a file that can’t be read; `DOMAIN` for a file that
loads itself, directly or through other files.

<!-- help •signal -->

### `•signal` — Signal

`•signal "DOMAIN ERROR"` raises an ordinary BPL error. It is monadic and
returns no value. An [error guard](glyphs/error-guard.qmd) catches it in
the same way as an error from a primitive.

``` bpl
positive←{⍵≤0?•signal "DOMAIN ERROR";⍵}
safe←{11::0 ⋄ positive ⍵}
safe¯3    ⍝ 0
safe4     ⍝ 4
```

The argument is one of these uppercase names: `SYNTAX ERROR`,
`INDEX ERROR`, `RANK ERROR`, `LENGTH ERROR`, `VALUE ERROR`,
`LIMIT ERROR` or `DOMAIN ERROR`. Their guard numbers are 2, 3, 4, 5, 6,
10 and 11. Numeric codes and other names are a `DOMAIN` error.
Interrupts, timeouts and unsupported-feature errors can’t be signalled.

<!-- help •storage -->

### `•storage` — Storage

`•storage Y` names the storage that holds the items of `Y`: `"boolean"`,
`"integer"`, `"float"`, `"complex"`, `"character"` or `"mixed"`. An atom
gives its own kind, which can also be `"rational"` or `"function"`.
Compact storage holds items of one kind. Mixed storage keeps each item’s
kind. Boxed display marks the same storage on its bottom edge.
[Storage](numbers.qmd#storage) describes each storage.

<!-- help •time -->

### `•time` — Timing

`•time t` gives the seconds since the time `t`. Time counts from the
session’s start on a monotonic clock. `•time 0` is the current reading.
`t←•time 0` starts a timer, and `•time t` then gives the seconds since
it started.

`F •time x` calls each function in `F` on `x` for about 0.1 s. The
result is each function’s fastest time per call in seconds, with the
shape and keys of `F`. With `F←["sum":+/ "max":⌈/]`, `F •time x` labels
each time. Time a dyadic function with its left argument bound, as in
`2↣⍴`. [Timing](repl.qmd#timing) has examples.

Errors: `DOMAIN` for a left argument that holds anything but functions,
or a `t` that isn’t a number. An error from a timed function stops the
timing.

<!-- help -->

## Introspection

<!-- help •nc -->

### `•nc` — Name class

`•nc names` gives the class of each name: `¯1` invalid, `0` undefined,
`2` value, `3` function, `4` operator. A character vector names one
binding. An array of strings keeps its shape.
[Introspection](introspection.ipynb) has examples.

<!-- help •nl -->

### `•nl` — Name list

`prefix •nl classes` lists the visible user names in `classes` that
begin with `prefix`, as a sorted vector of strings. `•nl classes` lists
them all. [Introspection](introspection.ipynb) has examples.

Errors: `DOMAIN` for unsupported classes; `RANK` for a class matrix.

<!-- help •src -->

### `•src` — Source

`•src name` returns the definition text of a function or operator,
including comments. [Introspection](introspection.ipynb) has examples.

Errors: `VALUE` for an undefined name; `DOMAIN` for an array.

<!-- help •ex -->

### `•ex` — Erase

`•ex names` erases the nearest binding of each name. An outer binding
can then become visible. It returns `1` when the name is gone, and `0`
for an invalid or protected name. [Introspection](introspection.ipynb)
has examples.
