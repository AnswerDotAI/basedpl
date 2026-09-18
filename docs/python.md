# Python

[Home](index.md) · [Install](getting-started.md#install) · [Process workers](processes.md#interruptible-worker)

## Sessions

A `Session` retains APL names. Calls return an `Array`; `.py` converts to Python values.

```python
from miniapl import Session

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

`Array` retains shape, nesting, numeric domain and empty prototypes. `.shape` is a tuple; `.apl` is APL display. `.py` converts scalars to Python numbers and character vectors to strings. Other arrays become NumPy arrays.

```python
from fractions import Fraction
from miniapl import Array

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

`.np` or `np.asarray(a)` always produces an ndarray. Simple numeric arrays use `int64`, `float64` or `complex128`; fractions, large integers, nesting and mixed exact/approximate values use `object`. Character arrays use `U1`.

```python
import numpy as np

a = Array([[1, 2, 3], [4, 5, 6]])
np.testing.assert_array_equal(a.np, [[1, 2, 3], [4, 5, 6]])
```

Python/NumPy conversions copy. Passing an `Array` back to miniapl shares its immutable value. Keep `Array` to preserve empty prototypes and exact nesting. NumPy is needed only for ndarray conversion.

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

## Word functions

Primitives also have Python names. Dyadic names bind the right argument when called with one argument; `.left(x)` binds the left.

```python
from miniapl import plus, times, subtract, tally

assert times(2)(3).py == 6
assert subtract(2)(5).py == 3       # 5−2
assert subtract.left(2)(5).py == -3 # 2−5

mean = plus.reduce() / tally
assert mean([1, 2, 4]).py == Fraction(7, 3)
```

Names distinguish valences: `sign`/`times`, `shape`/`reshape`, `iota`/`index_of`, `mix`/`take`. `times(2.)` binds an approximate number; `times(2)` an exact one. Operator methods use the underlying APL function.

| Python | APL |
|---|---|
| `f.reduce()`, `f.scan()` | `f/`, `f\` |
| `f.each()`, `f.commute()` | `f¨`, `f⍨` |
| `f.outer()`, `f.inner(g)` | `∘.f`, `f.g` |
| `f.key()`, `f.stencil(s)` | `f⌸`, `f⌺s` |
| `f.rank(r)`, `f.atop(g)` | `f⍤r`, `f⍤g` |
| `f.beside(g)`, `f.over(g)`, `f.behind(g)` | `f∘g`, `f⍥g`, `f⍛g` |
| `f.power(n)`, `f.at(i)` | `f⍣n`, `f@i` |
| `f[k]` | `f[k]` axis qualifier |

Arithmetic between functions makes forks: `f+g` is `(f+g)`. `f @ g` is inner product; `f << g` is compose; `f >> g` reverses composition; `f ** n` is power. Python's precedence applies when building these expressions. `f.under(g)` computes inverse-`g` after `f` after `g`.

## Errors and interruption

`AplError` provides the diagnostic, `kind`, `message`, `source`, byte `span`, call context and prior `output`. Completed assignments survive errors. `.eval()` retains output on the exception; ordinary calls print it before raising.

Ctrl-C interrupts evaluation. From another thread, call `apl.interrupt()`. `Session(timeout=2)` gives each evaluation a two-second deadline; adjust `apl.timeout`, or set it to `None`.

Sessions use a Rust worker thread and remain usable after cooperative interruption. Native-library calls can delay cancellation. For a hard-kill fallback, use a [process worker](processes.md#interruptible-worker).

Close the session after its last use:

```python
apl.close()
```
