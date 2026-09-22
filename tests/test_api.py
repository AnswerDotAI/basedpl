import operator
from fractions import Fraction
from pathlib import Path
import numpy as np, pytest
from fastcore.test import test_eq as teq
from basedpl import (Array, Session, AplError, plus, times, subtract, divide, exponent, sign, tally, iota,
    reshape, shape, floor, logarithm, reverse, transpose, fork, atop, first, pick, not_)


def test_documentation_examples():
    for path in (Path(__file__).resolve().parents[1]/'docs').glob('*.md'):
        namespace = {}
        for block in path.read_text().split('```python\n')[1:]: exec(compile(block.split('```', 1)[0], str(path), 'exec'), namespace)


def test_load(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    source = tmp_path/'defs.apl'
    source.write_text('twice←{2×⍵}\nx←7\n')
    (tmp_path/'main.apl').write_text("•LOAD 'defs.apl'\ntwice x")
    with Session() as apl:
        assert apl("•load 'main.apl'").py == 14
        assert apl('twice x').py == 14
        assert apl("{loaded←•LOAD 'defs.apl' ⋄ twice ⍵}3").py == 6
        source.write_text('⎕←x\n1÷0')
        with pytest.raises(AplError, match=r'defs.apl:2') as err: apl("•LOAD 'defs.apl'")
        assert err.value.output == ['7']
        source.write_text('⎕←9\n{∇⍵}0')
        apl.timeout = .001
        with pytest.raises(AplError, match='TIMEOUT'): apl("•LOAD 'defs.apl'")
        apl.timeout = None
        assert apl('x').py == 7
        with pytest.raises(AplError, match='VALUE'): apl("•LOAD 'missing.apl'")


def test_data_io(tmp_path):
    source, dest = tmp_path/'sales.json', tmp_path/'sales.csv'
    source.write_text('{"price":[10.5,20.0],"qty":[2,4]}', encoding='utf-8')
    with Session() as apl:
        read, write, json, csv = (apl.fn(s) for s in ('•NGET', '•NPUT', '•JSON', '•CSV'))
        table = json(read({'path': str(source), 'encoding': 'UTF-8'}))
        encoded = csv(table, '').py
        teq(write(encoded, str(dest)).py, len(encoded.encode('utf-8')))
        teq(apl.fn('≡')(csv(read(str(dest))), table).py, 1)
        with pytest.raises(AplError, match='VALUE'): write('replacement', str(dest))
        teq(dest.read_text(), encoded)
        teq(write('é\r\n', {'path': str(dest), 'overwrite': 1}).py, 4)
        teq(read(str(dest)).py, 'é\r\n')
        with pytest.raises(AplError, match='encoding'): write('bad', dict(path=str(dest), overwrite=1, encoding='UTF-16'))
        teq(dest.read_bytes(), 'é\r\n'.encode())
        dest.write_bytes(b'\xff')
        with pytest.raises(AplError, match='VALUE'): read(str(dest))
        with pytest.raises(AplError, match='VALUE'): read(str(tmp_path/'absent'))


def test_array_surface():
    a = Array([[1, 2, 3], [4, 5, 6]])
    assert len(a) == 2
    np.testing.assert_array_equal(a + [10, 20], [[11, 12, 13], [24, 25, 26]])
    np.testing.assert_array_equal(10 - a, [[9, 8, 7], [6, 5, 4]])
    np.testing.assert_array_equal(a[2, :], [4, 5, 6])
    np.testing.assert_array_equal(a[:, [1, 3]], [[1, 3], [4, 6]])
    np.testing.assert_array_equal(list(a)[0], [1, 2, 3])
    with pytest.raises(AplError): a[0, 1]
    with pytest.raises(TypeError): a[1:2, :]
    assert (Array(7) % 3).py == 1 and (Array(7) // 3).py == 2
    assert (Array(2) ** 3).py == 8 and (Array(3) / 2).py == Fraction(3, 2)
    assert operator.index(Array(3)) == 3 and int(Array(3.5)) == 3 and float(Array(Fraction(1, 2))) == .5
    assert bool(Array([1])) and not bool(Array(0))
    with pytest.raises(ValueError): bool(a)
    with pytest.raises(TypeError): operator.index(Array(3.))
    with pytest.raises(TypeError): hash(a)
    np.testing.assert_array_equal(a > 3, [[0, 0, 0], [1, 1, 1]])
    np.testing.assert_array_equal(~(a > 3), a <= 3)
    np.testing.assert_array_equal(a @ transpose(a), [[14, 32], [32, 77]])
    assert Array(2).apl == '2x' and Array(2.).apl == '2'
    assert repr(Array(2)) != repr(Array(2.))

def test_keyed_arrays():
    with Session() as apl:
        t = apl("('b':1 2),('a':('x':'hi'),('n':3))")
        assert list(t.py) == ['b', 'a'] and t.py['a'] == dict(x='hi', n=3) and t.shape == (2,)
        np.testing.assert_array_equal(t.py['b'], [1, 2])
        assert repr(t).startswith("{'b': ")
        d = dict(z=1, y=dict(k=[1, 2, 3]), e={})
        assert list(apl('⍳[1]t', t=d).py) == ['z', 'y', 'e'] and apl('(t.y.k[2])+t.e≡⍬:⍬', t=d).py == 3
        assert (Array(dict(a=1, b=2)) + Array(dict(b=10))).py == dict(a=1, b=12)
        with pytest.raises(TypeError): Array({1: 2})
        k = Array(dict(qty=4, price=1, tax=2))
        assert k['price'].py == 1 and k['price'].is_atom and k[2].py == 1
        assert list(k[['tax', 'qty']].py) == ['tax', 'qty']
        assert k.axis_keys == (('qty', 'price', 'tax'),)
        with pytest.raises(AplError, match='INDEX'): k['missing']
        with pytest.raises(AplError, match='DOMAIN'): k[['qty', 'qty']]

def test_axis_keys_dataframe():
    import pandas as pd
    m = Array([[10, 2], [20, 4]], axis_keys=[['alice', 'bob'], ['price', 'qty']])
    assert m.axis_keys == (('alice', 'bob'), ('price', 'qty'))
    assert m['alice'].py == dict(price=10, qty=2)
    assert m[:, 'qty'].py == dict(alice=2, bob=4)
    pd.testing.assert_frame_equal(m.py, pd.DataFrame([[10, 2], [20, 4]], index=['alice', 'bob'], columns=['price', 'qty']))
    np.testing.assert_array_equal(m.np, [[10, 2], [20, 4]])
    h = Array(np.arange(8).reshape(2, 2, 2), axis_keys=[['aa', 'bb'], None, ['xx', 'yy']])
    pd.testing.assert_frame_equal(h.py, pd.DataFrame(np.arange(8).reshape(4, 2),
        index=pd.MultiIndex.from_product([['aa', 'bb'], [1, 2]]), columns=['xx', 'yy']))
    pd.testing.assert_frame_equal(Array(3).df, pd.DataFrame([[3]], index=[1], columns=[1]))
    pd.testing.assert_frame_equal(Array([3, 4]).df, pd.DataFrame([3, 4], index=[1, 2], columns=[1]))
    assert Array([[1, 2]]).axis_keys == (None, None)
    for keys in ([None], [['a', 'a'], None], [None, ['a']]):
        with pytest.raises(ValueError): Array([[1, 2], [3, 4]], axis_keys=keys)

def test_based_values():
    with Session() as apl:
        atom, unit, vector = [apl(code) for code in ('3', '⊂3', ',3')]
        assert atom.is_atom and not unit.is_atom and not vector.is_atom
        assert atom.shape == unit.shape == () and vector.shape == (1,)
        assert isinstance(atom.py, float) and isinstance(unit.py, np.ndarray) and unit.py.shape == ()
        for value in (atom, unit, vector):
            apl(value=value)
            assert apl('value').is_atom == value.is_atom
            assert apl('value').shape == value.shape
        assert Array(3).is_atom and not Array(np.array(3)).is_atom
        assert apl('v←3 4 ⋄ v[1]').is_atom
        assert not apl('v[⊂1]').is_atom
        assert apl('1⊃v').is_atom
        assert Array([3, 4])[1].is_atom and not Array([3, 4])[np.array(1)].is_atom
        assert apl('fs←+˘×')[2](3, 4).py == 12


def test_words_binding_and_operators():
    np.testing.assert_array_equal(iota(3), [1, 2, 3])
    np.testing.assert_array_equal(shape(reshape([2, 3], iota(6))), [2, 3])
    assert times(2.)(3).py == 6. and type(times(2.)(3).py) is float
    assert times(2)(3).py == 6 and type(times(2)(3).py) is int
    assert subtract(2.)(5).py == 3 and subtract.left(2.)(5).py == -3
    assert divide.left(1)(3).py == Fraction(1, 3)
    assert sign(-3).py == -1 and exponent(2)(3).py == 9
    assert not_(0).py == 1
    mean = plus.reduce / tally
    assert mean([1, 2, 4]).py == Fraction(7, 3)
    assert fork(plus.reduce, divide, tally)([1, 2, 4]).py == Fraction(7, 3)
    assert (plus.reduce + 2)([1, 2]).py == 5
    assert (2 - plus.reduce)([1, 2]).py == -1
    np.testing.assert_array_equal(plus.scan([1, 2, 3]), [1, 3, 6])
    np.testing.assert_array_equal(subtract.scan([1, 2, 3]), [1, -1, -4])
    np.testing.assert_array_equal(plus.scan(10, [1, 2, 3]), [11, 13, 16])
    assert type(tally('abc').py) is int and shape(Array([1., 2.])).apl == '2x'
    assert (Array('abc') + 1).py == 'bcd'
    np.testing.assert_array_equal(plus.reduce[1](Array([[1, 2], [3, 4]])), [4, 6])
    np.testing.assert_array_equal(plus.reduce[2, 3](np.arange(1, 9).reshape(2, 2, 2)), [10, 26])
    np.testing.assert_array_equal(times.outer([1, 2], [3, 4]), [[3, 4], [6, 8]])
    assert (plus @ times)([1, 2], [3, 4]).py == 11
    assert (subtract(1) ** 3)(10).py == 7
    assert (times(2.) ** -1)(10).py == 5
    assert (floor << times(10.))(1.25).py == 12
    assert (times(10.) >> floor)(1.25).py == 12
    assert atop(floor, times(10.))(1.25).py == 12
    assert plus.under(logarithm)(2., 3.).py == pytest.approx(6.)
    np.testing.assert_array_equal(reverse.rank(1)(Array([[1, 2], [3, 4]])), [[2, 1], [4, 3]])
    np.testing.assert_array_equal(times(2).at([2, 4])([1, 2, 3, 4]), [1, 4, 3, 8])
    with pytest.raises(TypeError): plus @ Array(2)
    with pytest.raises(ValueError, match='DOMAIN'): plus.over(3)
    with pytest.raises(ValueError, match='DOMAIN'): plus.stencil(times)
    assert subtract.commute.reduce([1, 2, 3]).py == 0
    teq(times(2).power.each(3)([1, 2]).py, [8, 16])
    teq(subtract.left.each(2)([1, 2, 3]).py, [1, 0, -1])
    teq(plus.left(1).power.each.power(2)(3)([0, 10]).py, [6, 16])
    teq(plus.power.reduce[1](1)([[1, 2], [3, 4]]).py, [4, 6])
    teq((subtract.left.each + 10)(2)([3, 4]).py, [9, 8])


def test_math_construction():
    from basedpl import prime, prime_mode, factors, factor_spec, polynomial, polyval, windows
    f = plus.left(1).with_inverse(subtract(1))
    np.testing.assert_array_equal(f.power([2, -1, 0])(10), [12, 9, 10])
    np.testing.assert_array_equal(f.history(-2)(10), [10, 9, 8])
    with Session() as s:
        stop = s('limit←13 ⋄ {⍺≥limit}')
        np.testing.assert_array_equal(f.history(stop)(10), [10, 11, 12, 13])
    np.testing.assert_array_equal(windows(2, [1, 2, 3]), [[1, 2], [2, 3]])
    assert prime(10).py == 29 and prime_mode(1, 29).py == 1
    np.testing.assert_array_equal(factors(700), [2, 2, 5, 5, 7])
    np.testing.assert_array_equal(factor_spec(float('inf'), 700), [2, 0, 2, 1])
    np.testing.assert_array_equal(polynomial([2, [1, 3]]), [6, -8, 2])
    p = polyval.left([1, 2, 3])
    assert p.derivative(2).py == 14 and p.derivative.derivative(2).py == 6
    np.testing.assert_array_equal(p.derivative([10, 20], [1, 2]), [80, 280])


def test_axis_names():
    data = np.arange(6).reshape(2, 3)
    m = Array(data, axis_names=('city', 'month'))
    month = Array([10, 20, 30], axis_names=('month',))
    teq((m + month).np, data + [10, 20, 30])
    teq((m + [10, 20]).np, data + np.array([[10], [20]]))
    teq((month + m).axis_names, ('month', 'city'))
    teq((month + m).np, (data + [10, 20, 30]).T)
    teq((m + transpose(m)).np, data * 2)
    a = Array(data, axis_names=('city', None))
    b = Array(data.T, axis_names=(None, 'city'))
    teq((a + b).np, data * 2)
    product = Array(np.arange(12).reshape(4, 3), axis_names=('product', None))
    teq((a + product).axis_names, ('city', None, 'product'))
    teq((a + product).np, data[:, :, None] + product.np.T[None, :, :])
    teq(plus.reduce['month'](m).axis_names, ('city',))
    teq(plus.reduce['month'](m).np, [3, 12])
    keyed = Array(m, axis_keys=(('Paris', 'London'), ('Jan', 'Feb', 'Mar')))
    other = Array([30, 20, 10], axis_names=('month',), axis_keys=(('Mar', 'Feb', 'Jan'),))
    teq((keyed + other).np, data + [10, 20, 30])
    assert keyed.df.index.name == 'city' and keyed.df.columns.name == 'month'
    with Session() as s:
        s(M=keyed)
        teq(s('⍳[0]M').py, ['city', 'month'])
        renamed = s("('town' ⋄ 2):[0]M")
        teq(renamed.axis_names, ('town', None))
        teq(renamed.axis_keys, keyed.axis_keys)
        teq(s('⍳[0]M').py, ['city', 'month'])
        teq(s(':[0]M').axis_names, (None, None))
        teq(s('(⍳[0]M):[0](:[0]M)').axis_names, keyed.axis_names)
        teq(s("⍳[0]'items':[0]1 2").py, ['items'])
        teq(s('⍳[0]7').py, [])
        for code in ["'city' 'city':[0]M", '(1 1):[0]M']:
            with pytest.raises(AplError, match='DOMAIN'): s(code)
        with pytest.raises(AplError, match='LENGTH'): s("'city':[0]M")
        teq(s("'Paris'⌷['city']M").np, data[0])
        teq(s('M[2]').axis_names, ('month',))
        teq(s("+/['city' 'month']M").np, 15)
        teq(s('⍉M').axis_names, ('month', 'city'))
        teq(s('2 3⍴M').axis_names, ('city', 'month'))
        teq(s('3 2⍴M').axis_names, (None, None))
        teq(s(':M').axis_names, ('city', 'month'))
        teq(s('+/¨⊂[2]M').axis_names, ('city',))
        teq(s('⌽⍤1⊢M').axis_names, ('city', 'month'))
        with pytest.raises(AplError, match='INDEX'): s("+/['missing']M")
    v = Array([1, 2], axis_names=('city',))
    teq(times.outer(v, v).axis_names, (None, None))
    teq(reshape([4], v).axis_names, (None,))
    teq(plus.scan(v).axis_names, ('city',))
    with Session() as s:
        s(V=v)
        for code, names in [('2/V', ('city',)), ('1↕V', (None, None)), ('{⍵}⌺1⊢V', (None, None)), ('V,V', ('city',))]:
            teq(s(code).axis_names, names)
    with pytest.raises(ValueError, match='DOMAIN'): Array(data, axis_names=('city', 'city'))
    with pytest.raises(ValueError): Array(data, axis_names=('city',))


def test_mathematical_monads():
    from basedpl import sqrt, root, square, double, decrement, increment, pi_ratio, cis, real_imag, polar, classify, binary_encode, binary_decode
    assert sqrt(Fraction(1, 9)).py == Fraction(1, 3) and root(3, -8).py == -2
    assert square(3).py == 9 and double(3).py == 6 and decrement(increment(3)).py == 3
    assert complex(cis(pi_ratio(1, 2)).py) == pytest.approx(1j)
    np.testing.assert_array_equal(real_imag(3+4j), [3., 4.])
    np.testing.assert_allclose(polar(1j), [1., np.pi/2])
    np.testing.assert_array_equal(classify('aba'), [[1, 0, 1], [0, 1, 0]])
    np.testing.assert_array_equal(binary_decode(binary_encode([2, 5])), [2, 5])


def test_function_arrays():
    fs = Array([plus, times])
    assert pick(2, fs)(2, 3).py == 6
    assert fs.py[0](2, 3).py == 5
    with Session() as a, Session() as b:
        fs = a('offset←10 ⋄ {offset+⍵}˘+')
        a(offset=20)
        for f in [first(fs), first(list(fs)[0]), fs.py[0], first(Array(fs.np)[1]), a.fn('{↑⍵}')(fs)]: assert f(3).py == 23
        assert pick(2, reverse(fs))(3).py == 23
        a(fs=fs)
        assert a('f←1⊃fs ⋄ f 3').py == 23
        with pytest.raises(ValueError): b(fs=fs)
        with pytest.raises(ValueError): Array([fs[1], b.fn('+')])


def test_retained_and_late_bound_functions(capsys):
    with Session() as a, Session() as b:
        with pytest.raises(AplError) as caught: a.fn('{')
        assert caught.value.source == '{'
        a(k=2)
        f = a('{k+⍵}')
        assert f(3).py == 5
        a(k=10)
        assert f(3).py == 13
        late = a.fn('g')
        a('g←+')
        composed = late + times(2)
        assert composed(3).py == 9
        assert late.reduce([]).py == 0
        assert (a.fn('+') @ a.fn('×'))([], []).py == 0
        np.testing.assert_array_equal(a.fn('⌽')[1]([[1, 2], [3, 4]]), [[3, 4], [1, 2]])
        a('scale←2∘×')
        assert (a.fn('scale') ** -1)(6).py == 3
        a(fold=late.reduce)
        assert a('fold ⍬').py == 0
        a(loop=a.fn('loop'))
        with pytest.raises(AplError, match='LIMIT'): a('loop 1')
        assert a('1+1').py == 2
        assert a('count←{⍵=0:0 ⋄ 1+∇⍵-1} ⋄ count 500').py == 500
        a('g←-')
        assert composed(3).py == 3
        saved = a('g')
        a('g←+')
        assert saved(3).py == -3 and late(3).py == 3
        assert b('f 5', f=times(2)).py == 10
        with pytest.raises(ValueError, match='session'): b(f=f)
        with pytest.raises(ValueError, match='session'): f + b.fn('+')
        a('outer←{k←100 ⋄ ⍶ ⍵}', f=f)
        assert a('f outer 1').py == 11
        g = a('{⎕←⍵ ⋄ ⍵}')
        assert (g + g)(2).py == 4 and capsys.readouterr().out == '2x\n2x\n'
        assert a.eval('g', g=g).value(2).py == 2
        assert capsys.readouterr().out == '2x\n'
    with pytest.raises(RuntimeError, match='closed'): f(3)
    assert saved(3).py == -3
