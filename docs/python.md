# Python

[Home](index.md) · [Install](getting-started.md#install) · [Process workers](processes.md#interruptible-worker)

## Sessions

A `Session` retains APL names. Calls return an `Array`; `.py` converts to Python values.

```python
from basedpl import Session

apl = Session()
apl('v←⍳5')
assert apl('+/v').py == 15.0
```

Keywords bind values before evaluation. Bindings persist. Use indexing to read expressions or assign names.

```python
assert apl('+/x', x=[1, 2, 3]).py == 6
apl['x'] = [[1, 2], [3, 4]]
assert apl['x'].shape == (2, 2)
```

Calls print only explicit output (`⎕←` and display commands). `.eval()` captures it instead.

```python
r = apl.eval('⎕←x ⋄ x+1x', x=3)
assert r.value.py == 4
assert r.output == ['3x']
```

`.run()` also captures implicit expression display, for an APL-style notebook frontend. In ordinary Python notebooks, append `;` to suppress display of a call's return value.

Use `with Session() as apl:` or call `apl.close()` when finished. Returned arrays remain usable after closing.

## Values and conversion

`Array` retains atoms, shape, nesting, numeric domain and empty prototypes. `.is_atom` distinguishes an atom from a rank-zero array; both have `.shape == ()`. `.apl` is APL display. `.py` converts atoms to Python values and character vectors to strings. Other arrays become NumPy arrays, including rank-zero arrays. `.np` always returns an ndarray.

For [axis-keyed arrays](keyed.md), `.py` returns a dict at rank 1 and a pandas DataFrame at higher ranks. `.np` copies the values. Attach labels with `Array(data, axis_keys=[rows, cols])`; use `None` for an unkeyed axis. `.axis_keys` returns a tuple of label tuples or `None`.

`axis_names` names dimensions; `axis_keys` names positions within them. Matching axis names align before positional broadcasting. `.axis_names` returns one name or `None` per dimension.

```python
from basedpl import Array, plus
m = Array([[1, 2, 3], [4, 5, 6]], axis_names=('city', 'month'))
v = Array([10, 20, 30], axis_names=('month',))
assert (m + v).np.tolist() == [[11, 22, 33], [14, 25, 36]]
assert plus.reduce['month'](m).np.tolist() == [6, 15]
assert plus.reduce['month'](m).axis_names == ('city',)
```

`.df` converts any array to a DataFrame. The last axis supplies columns; earlier axes supply rows, using a MultiIndex above rank 2. Unkeyed axes have 1-origin labels. Install `basedpl[pandas]` for this conversion. Pandas is imported on demand.

Axis names become pandas index/column names, including MultiIndex level names.

```python
from fractions import Fraction
from basedpl import Array

assert Array(2).apl == '2x'
assert Array(2.).apl == '2'
assert Array(Fraction(1, 3)).py == Fraction(1, 3)
```

| Python input | APL value |
|---|---|
| `int`, `bool`, NumPy integers | Exact number |
| `float` | Approximate real |
| `Fraction` | Exact rational |
| `complex` | Approximate complex |
| `str` | Character vector |
| Rectangular list/tuple | Ordinary array |
| Ragged list/tuple | Nested array |
| NumPy ndarray | Array preserving shape and numeric domain |
| `dict` with string keys | [Keyed vector](keyed.md), recursively. `.py` returns a `dict` |

`.np` or `np.asarray(a)` always produces an ndarray. Simple numeric arrays use `int64`, `float64` or `complex128`; fractions, large integers, nesting and mixed exact/approximate values use `object`. Character arrays use `U1`.

```python
import numpy as np

a = Array([[1, 2, 3], [4, 5, 6]])
np.testing.assert_array_equal(a.np, [[1, 2, 3], [4, 5, 6]])
```

Python/NumPy conversions copy. Passing an `Array` back to bAsedPL shares its immutable value. Keep `Array` to preserve empty prototypes and exact nesting. NumPy is needed only for ndarray conversion.

## Array operations

Arithmetic uses APL agreement, aligning leading axes. Indexing starts at 1; `:` selects a whole axis.

```python
np.testing.assert_array_equal(a + [10, 20], [[11, 12, 13], [24, 25, 26]])
np.testing.assert_array_equal(a[2, :], [4, 5, 6])
np.testing.assert_array_equal(a[:, [1, 3]], [[1, 3], [4, 6]])
```

Comparisons return arrays. `%` uses Python's operand order for residue; `//` floors division; `@` is matrix product. `&`/`|` give LCM/GCD, hence AND/OR on Booleans. `len` and iteration use major cells. Use `member(x, y)` for membership. Bounded slices are not supported.

## Callable functions

`.fn()` creates a callable APL function. One argument supplies `⍵`; two supply `⍺, ⍵`. Arguments pass directly as values.

```python
mean = apl.fn('{(+/⍵)÷≢⍵}')
assert mean([1, 2, 4]).py == Fraction(7, 3)
assert mean([1., 2., 4.]).py == 7 / 3

add = apl.fn('+')
np.testing.assert_array_equal(add([1, 2, 3], 10), [11, 12, 13])
```

`.fn('g')` follows later redefinitions of `g`; `apl('g')` retains its current function. Dfns and late-bound functions keep their originating session for name lookup. Close that session only after their last use.

[Function arrays](glyphs/strand.md#function-arrays) retain callable handles, including their session for name lookup. `first` and `pick` return the selected function:

```python
from basedpl import first, pick
fs = apl('+˘×˘÷')
assert pick(2, fs)(2, 3).py == 6
assert first(fs)(2, 3).py == 5
```

`Array([f, g])` also constructs a function vector; `.py` and object-dtype `.np` export Python callables. Functions from different sessions cannot be combined.

## Word functions

Primitives also have Python names. Dyadic names bind the right argument when called with one argument; `.left(x)` binds the left.

```python
from basedpl import plus, times, subtract, tally

assert times(2)(3).py == 6
assert subtract(2)(5).py == 3       # 5−2
assert subtract.left(2)(5).py == -3 # 2−5

mean = plus.reduce / tally
assert mean([1, 2, 4]).py == Fraction(7, 3)
```

Names distinguish valences: `sign`/`times`, `shape`/`reshape`, `iota`/`index_of`, `first`/`take`, `mix`/`pick`. `times(2.)` binds an approximate number; `times(2)` an exact one. Operators use the underlying APL function.

`exponential` and `exponent` name the monadic and dyadic forms of `*`. `power` names the operator `⍣`, exposed as `f.power(n)`. System names beginning with `•` have no Python word aliases; they remain available inside evaluated APL.

`basedpl.symbols` is the shared naming table used by Python functions and REPL and notebook completion. Each row is `(glyph, name, monad, dyad, aliases)`: the canonical glyph name, monadic and dyadic function names (empty when absent), and space-separated extra completion aliases. It includes operators and syntax glyphs too, with empty function names. Python replaces hyphens with underscores and appends an underscore to keywords (`not` → `not_`). The table is directly JSON-serializable for JavaScript consumers.

| Python | APL |
|---|---|
| `f.reduce`, `f.scan` | `f/`, `f\` |
| `f.each`, `f.commute` | `f¨`, `f⍨` |
| `f.outer`, `f.inner(g)` | `f⌝`, `f.g` |
| `f.key`, `f.stencil(s)` | `f⌸`, `f⌺s` |
| `f.rank(r)`, `f.atop(g)` | `f⍤r`, `f⍤g` |
| `f.beside(g)`, `f.over(g)`, `f.behind(g)` | `f∘g`, `f⍥g`, `f⍛g` |
| `f.power(n)`, `f.at(i)` | `f⍣n`, `f@i` |
| `f.history(n)` | `f⍣[n]` (count or predicate) |
| `f.with_inverse(g)`, `f.under(g)` | `f⇄g`, `f⌾g` |
| `f.derivative` | `f∂` |
| `f[k]` | `f[k]` axis qualifier |

Operator properties chain in APL order: `subtract.commute.reduce` is `-⍨/`. Operators needing another operand retain a hole until supplied:

```python
twice = times(2)
assert twice.power.each(3)([1, 2]).py.tolist() == [8, 16]  # (twice⍣3)¨
```

`f.power.each(n)` constructs `(f⍣n)¨`, just like `f.power(n).each`. For multiple missing operands, successive calls fill the innermost construction first: `f.power.each.power(n)(m)` constructs `((f⍣n)¨)⍣m`. Complete the operands before passing evaluation arguments.

Arithmetic between functions makes forks: `f+g` is `(f+g)`. `f @ g` is inner product; `f << g` is compose; `f >> g` reverses composition; `f ** n` is power. Python's precedence applies when building these expressions.

Math families: `prime`/`prime_mode` (`ℙ`), `factors`/`factor_spec` (`⨸`), `polynomial`/`polyval` (`⊛`). `windows` is dyadic `↕`.

Roots: `sqrt`/`root` (`√`). Complex coordinates: `real_imag` (`∨`), `polar` (`∧`), `cis` (`○`). `pi_times`/`pi_ratio` (`π`) give π multiples/fractions. Also `square`, `double`, `decrement`, `increment`, `classify`, `binary_encode` and `binary_decode`.

```python
from basedpl import prime, factors, polyval

assert prime(10).py == 29
np.testing.assert_array_equal(factors(700), [2, 2, 5, 5, 7])
p = polyval.left([1, 2, 3])
assert p(2).py == 17
assert p.derivative(2).py == 14
np.testing.assert_array_equal(p.derivative([10, 20], [1, 2]), [80, 280])
np.testing.assert_array_equal(plus.left(1).history(3)(0), [0, 1, 2, 3])
```

## Errors and interruption

`AplError` provides the diagnostic, `kind`, `message`, `source`, byte `span`, call context and prior `output`. Completed assignments survive errors. `.eval()` retains output on the exception; ordinary calls print it before raising.

Ctrl-C interrupts evaluation. From another thread, call `apl.interrupt()`. `Session(timeout=2)` gives each evaluation a two-second deadline; adjust `apl.timeout`, or set it to `None`.

Sessions use a Rust worker thread and remain usable after cooperative interruption. Native-library calls can delay cancellation. For a hard-kill fallback, use a [process worker](processes.md#interruptible-worker).

Close the session after its last use:

```python
apl.close()
```
