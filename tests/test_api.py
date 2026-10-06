import http.server, operator, threading
from fractions import Fraction
from pathlib import Path
import numpy as np, pytest
import basedpl
from fastcore.test import test_eq as teq
from basedpl import (Array, bpl, BplError, plus, times, subtract, divide, exponent, sign, tally, iota,
    reshape, shape, floor, logarithm, reverse, transpose, fork, atop, first, pick, not_)


def test_documentation_examples():
    for path in (Path(__file__).resolve().parents[1]/'nbs').glob('*.qmd'):
        namespace = {}
        for block in path.read_text().split('```python\n')[1:]: exec(compile(block.split('```', 1)[0], str(path), 'exec'), namespace)


def test_builtin_attributes():
    for api in (basedpl, bpl):
        teq(api.add([1, 2], 10).py, [11, 12])
        teq((api.plus(2)(3).py, api.times(2)(3).py, api.divide(2)(3).py), (5, 6, Fraction(3, 2)))
        teq(getattr(api, '•distribution')([2, 0.5], 'binomial')['quantile'](1.).py, 2)
        assert {'add', 'plus', 'distribution', 'π'} <= set(dir(api))
        for name in ('plu', 'userfn', 'nonexistent', 'minus'):
            with pytest.raises(AttributeError): getattr(api, name)
    bpl('plus←99 ⋄ userfn←{⍵+1}')
    teq(bpl.plus(2, 3).py, 5)
    teq(bpl['plus'].py, 99)
    teq(bpl.userfn(1).py, 2)
    assert 'userfn' in dir(bpl)
    teq(bpl.execute('plus').py, 99)
    teq(bpl.execute_in({'a': 1, 'b': 2}, 'a+b').py, 3)
    teq(bpl.not_(0).py, 1)
    assert bpl.names.__func__ is type(bpl).names
    from basedpl import add
    teq(add(2).py, 2)
    assert 'plus' not in vars(basedpl)


def test_python_printer():
    from basedpl import to_python
    for code, expected in {
        '×': 'sign', '×↢2': 'times(2.)', '2↣-': 'subtract.left(2.)', '2-': 'subtract.left(2.)',
        '×↢2ₓ': 'times(2)', '÷↢1r2': 'divide(Fraction(1, 2))',
        '+/÷≢': 'plus.reduce / tally', '+.×': 'plus @ times', '×⊗': 'times.outer_product',
        '+/⍠1': 'plus.reduce[1.]', '1↣+⍣[≡]': 'plus.left(1.).history(match)', '+⌿': 'plus.reduce[0]', '-⍨': 'subtract.commute',
        '+∘×': 'conjugate.atop(sign)', '+⍥×': 'conjugate.over(sign)', '-↣+': 'negate.before(plus)', '+↢-': 'plus.after(negate)', '-⊘+': 'negate.valences(plus)',
        '{⍵<0}⍚[⊢ -]': 'fn("{⍵<0}").agenda(bpl("[⊢ -]"))', '[1 2 ⋄ 3 4]↣+': 'plus.left(bpl("[1 2 ⋄ 3 4]"))',
        '{⍵×2}': 'fn("{⍵×2}")', '{⍵×2}¨': 'fn("{⍵×2}").each', '(×↢2)⁻¹': 'times(2.).undo',
    }.items(): teq(to_python(bpl(code)), expected)
    teq(to_python(bpl('×'), dyad=True), 'times')
    teq(to_python(bpl('+∘×'), dyad=True), 'conjugate.atop(times)')
    teq(to_python(times(2.) ** 3), 'times(2.) ** 3')
    teq(to_python((plus.reduce / tally).each), '(plus.reduce / tally).each')
    teq(to_python(bpl.fn('{⎕←1 ⋄ ⍵}')), 'fn("{⎕←1 ⋄ ⍵}")')
    teq(to_python(bpl.fn('unknown')), 'fn("unknown")')
    teq(to_python(bpl.fn('(⎕←1)+')), 'fn("(⎕←1)+")')


def test_load(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    lib = tmp_path/'lib'
    lib.mkdir()
    source = lib/'defs.bpl'
    source.write_text('_half←{⍵÷2}\nhalve←{_half ⍵}\ntwice←{⍶ ⍶ ⍵}\nx←7\n')
    (lib/'main.bpl').write_text('[halve]←•load "./defs.bpl"\nq←halve 10\n')
    m = bpl('•load "lib/defs.bpl"').py
    assert sorted(m) == ['halve', 'twice', 'x']
    assert m['twice'](bpl.fn('{⍵+1}'))(3).py == 5
    assert bpl('{m←•load "lib/defs.bpl" ⋄ [halve x]←m ⋄ (halve x),-m.twice 3}0').py.tolist() == [3.5, 3]
    assert 'halve' not in bpl.names()
    assert bpl('•LOAD "lib/main.bpl"').py['q'] == 5
    (lib/'a.bpl').write_text('•load "./b.bpl"')
    (lib/'b.bpl').write_text('•load "./a.bpl"')
    with pytest.raises(BplError, match='load cycle'): bpl('•load "lib/a.bpl"')
    source.write_text('x←7\n⎕←x\nx\n1÷"a"')
    with pytest.raises(BplError, match=r'defs.bpl:4') as err: bpl('•load "lib/defs.bpl"')
    assert err.value.output == ['7']
    source.write_text('⎕←9\n{∇⍵}0')
    bpl.timeout = .001
    with pytest.raises(BplError, match='TIMEOUT'): bpl('•load "lib/defs.bpl"')
    bpl.timeout = None
    with pytest.raises(BplError, match='IO ERROR'): bpl('•load "missing.bpl"')


def test_data_io(tmp_path, monkeypatch, capsys):
    source, dest = tmp_path/'sales.json', tmp_path/'sales.csv'
    source.write_text('{"price":[10.5,20.0],"qty":[2,4]}', encoding='utf-8')
    read, write = bpl.nget, bpl.nput
    table = bpl.json(read(str(source), encoding='UTF-8'))
    encoded = bpl.csv.undo(table).py
    teq(write(encoded, path=str(dest)).py, len(encoded.encode('utf-8')))
    teq(bpl.fn('≡')(bpl.csv(read(str(dest))), table).py, 1)
    with pytest.raises(BplError, match='IO ERROR'): write('replacement', path=str(dest))
    teq(dest.read_text(), encoded)
    teq(write('é\r\n', path=str(dest), overwrite=1).py, 4)
    teq(read(str(dest)).py, 'é\r\n')
    with pytest.raises(BplError, match='encoding'): write('bad', path=str(dest), overwrite=1, encoding='UTF-16')
    teq(dest.read_bytes(), 'é\r\n'.encode())
    dest.write_bytes(b'\xff')
    with pytest.raises(BplError, match='IO ERROR'): read(str(dest))
    with pytest.raises(BplError, match='IO ERROR'): read(str(tmp_path/'absent'))
    prompts = []
    monkeypatch.setattr('builtins.input', lambda prompt: prompts.append(prompt) or 'bob')
    teq(bpl('"-" •nput "Name? " ⋄ "Hi ",⎕').py, 'Hi bob')
    teq(prompts, ['Name? '])
    bpl('"-" •nput "a" ⋄ ⎕←1')
    teq(capsys.readouterr().out, 'a1\n')


def test_binary_files(tmp_path):
    path, dest = tmp_path/'bytes', tmp_path/'copy'
    data = bytes(range(256))
    path.write_bytes(data)
    read, write = bpl.nget, bpl.nput
    values = read(str(path), binary=1)
    teq(values.np.dtype, np.dtype('uint8'))
    teq(values.py, list(data))
    opts = dict(path=str(dest))
    teq(write(values, **opts).py, 256)
    teq(dest.read_bytes(), data)
    with pytest.raises(BplError, match='IO ERROR'): write(values, **opts)
    opts['overwrite'] = 1
    for bad in ([256], [-1], [0.5], [float('inf')]):
        with pytest.raises(BplError, match='DOMAIN'): write(bad, **opts)
        teq(dest.read_bytes(), data)
    with pytest.raises(BplError, match='encoding'): read(str(path), binary=1, encoding='UTF-8')
    teq(write([], **opts).py, 0)
    teq(dest.read_bytes(), b'')
    teq(read(str(dest), binary=1).shape, (0,))


def test_fetch():
    "`•fetch` reads curl's output: the final response's status and headers after redirects, then the body."
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *args): pass
        def reply(self, code, body, headers=()):
            self.send_response(code)
            for name, value in headers: self.send_header(name, value)
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        def do_GET(self):
            if self.path == '/old': self.reply(302, b'moved', [('Location', '/new')])
            elif self.path == '/new': self.reply(200, b'a\r\n\r\nb', [('X-Two', '1'), ('X-Two', '2')])
            else: self.reply(404, b'none')
        def do_POST(self):
            body = self.rfile.read(int(self.headers['Content-Length']))
            self.reply(201, body + b' ' + self.headers['X-Sent'].encode())
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    url = f'http://127.0.0.1:{server.server_address[1]}'
    try:
        r = bpl.fetch(url + '/old').py
        teq((r['status'], r['body'], r['headers']['x-two']), (200, 'a\r\n\r\nb', '1, 2'))
        teq(bpl.fetch(url + '/gone').py['status'], 404)
        teq(bpl.fetch(url, body='hi', headers={'X-Sent': 'yes'}).py['body'], 'hi yes')
        teq(bpl.fetch(url + '/new', binary=1).py['body'], list(b'a\r\n\r\nb'))
    finally:
        server.shutdown()
        server.server_close()
    with pytest.raises(BplError, match='IO ERROR'): bpl.fetch(url)


def test_regex_functions():
    p = bpl.fn('•r')(r'([a-z]+)([0-9]+)')
    matches, replace = p['matches'], p['replace']
    teq(matches('ab12 cd3')['text'].py, ['ab12', 'cd3'])
    teq(replace('$2:$1', 'ab12 cd3').py, '12:ab 3:cd')
    teq(bpl('(p.matches s).position', p=p, s='é ab12').py, [2])
    teq(matches('x9')['groups'].py, [['x', '9']])


def test_distribution_functions():
    distribution = bpl.fn('•distribution')
    normal, binomial = distribution([3., 2.], 'normal'), distribution([10, 0.5], 'binomial')
    sample = normal['sample']
    x, y = sample(10000).np, binomial['sample'](10000).np
    assert x.dtype == np.float64 and np.issubdtype(y.dtype, np.integer)
    assert abs(x.mean()-3) < 0.15 and abs(x.std()-2) < 0.15
    assert np.all((0 <= y) & (y <= 10)) and abs(y.mean()-5) < 0.15
    teq(binomial['quantile']([0., 1.]).np, [0, 10])
    teq(normal['cdf'](dict(a=3., b=float('inf'))).py, dict(a=0.5, b=1.))
    teq(sample([2, 3]).shape, (2, 3))


def test_array_surface():
    a = Array([[1, 2, 3], [4, 5, 6]])
    np.testing.assert_array_equal(bpl('m.[0 0]←0 ⋄ m', m=a), [[0, 2, 3], [4, 5, 6]])
    np.testing.assert_array_equal(a, [[1, 2, 3], [4, 5, 6]])
    assert len(a) == 2
    np.testing.assert_array_equal(a + [10, 20], [[11, 12, 13], [24, 25, 26]])
    np.testing.assert_array_equal(10 - a, [[9, 8, 7], [6, 5, 4]])
    np.testing.assert_array_equal(a[1, :], [4, 5, 6])
    np.testing.assert_array_equal(a[:, [0, 2]], [[1, 3], [4, 6]])
    np.testing.assert_array_equal(list(a)[0], [1, 2, 3])
    with pytest.raises(BplError): a[2, 0]
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
    assert Array(2).bpl == '2ₓ' and Array(2.).bpl == '2'
    assert repr(Array(2)) != repr(Array(2.))
    assert [repr(Array(s)) for s in ('text', '', ['ab', 'cd'])] == ["'text'", "''", "['ab', 'cd']"]

def test_keyed_arrays():
    t = bpl('["b":[1 2] "a":["x":"hi" "n":3]]')
    assert list(t.py) == ['b', 'a'] and t.py['a'] == dict(x='hi', n=3) and t.shape == (2,)
    np.testing.assert_array_equal(t.py['b'], [1, 2])
    assert repr(t).startswith("{'b': ")
    d = dict(z=1, y=dict(k=[1, 2, 3]), e={})
    assert list(bpl('⍳⍠0 t', t=d).py) == ['z', 'y', 'e'] and bpl('t.y.k₁+t.e≡⍬:⍬', t=d).py == 3
    assert (Array(dict(a=1, b=2)) + Array(dict(b=10))).py == dict(a=1, b=12)
    assert Array({'a': 1, 1: 5}).py == {'a': 1, 1: 5}
    with pytest.raises(TypeError): Array({2: 5})
    k = Array(dict(qty=4, price=1, tax=2))
    assert k['price'].py == 1 and k['price'].is_atom and k[1].py == 1
    assert list(k[['tax', 'qty']].py) == ['tax', 'qty']
    assert k.axis_keys == (('qty', 'price', 'tax'),)
    with pytest.raises(BplError, match='INDEX'): k['missing']
    with pytest.raises(BplError, match='DOMAIN'): k[['qty', 'qty']]

def test_axis_keys_dataframe():
    import pandas as pd
    m = Array([[10, 2], [20, 4]], axis_keys=[['alice', 'bob'], ['price', 'qty']])
    assert m.axis_keys == (('alice', 'bob'), ('price', 'qty'))
    assert m['alice'].py == dict(price=10, qty=2)
    assert m[:, 'qty'].py == dict(alice=2, bob=4)
    pd.testing.assert_frame_equal(m.py, pd.DataFrame(np.array([[10, 2], [20, 4]], dtype=np.uint8), index=['alice', 'bob'], columns=['price', 'qty']))
    np.testing.assert_array_equal(m.np, [[10, 2], [20, 4]])
    h = Array(np.arange(8).reshape(2, 2, 2), axis_keys=[['aa', 'bb'], None, ['xx', 'yy']])
    pd.testing.assert_frame_equal(h.py, pd.DataFrame(np.arange(8, dtype=np.uint8).reshape(4, 2),
        index=pd.MultiIndex.from_product([['aa', 'bb'], [0, 1]]), columns=['xx', 'yy']))
    pd.testing.assert_frame_equal(Array(3).df, pd.DataFrame([[3]], index=[0], columns=[0]))
    pd.testing.assert_frame_equal(Array([3, 4]).df, pd.DataFrame(np.array([3, 4], dtype=np.uint8), index=[0, 1], columns=[0]))
    assert Array([[1, 2]]).axis_keys == (None, None)
    for keys in ([None], [['a', 'a'], None], [None, ['a']]):
        with pytest.raises(ValueError): Array([[1, 2], [3, 4]], axis_keys=keys)

def test_based_values():
    atom, unit, vector = [bpl(code) for code in ('3', '⊂3', ',3')]
    assert atom.is_atom and not unit.is_atom and not vector.is_atom
    assert atom.shape == unit.shape == () and vector.shape == (1,)
    assert isinstance(atom.py, float) and isinstance(unit.py, np.ndarray) and unit.py.shape == ()
    for value in (atom, unit, vector):
        bpl(value=value)
        assert bpl('value').is_atom == value.is_atom
        assert bpl('value').shape == value.shape
    assert Array(3).is_atom and not Array(np.array(3)).is_atom
    assert bpl('v←3 4 ⋄ 1⌷v').is_atom
    assert not bpl('[⊂1]⌷v').is_atom
    assert bpl('1⊃v').is_atom
    assert Array([3, 4])[1].is_atom and not Array([3, 4])[np.array(1)].is_atom
    assert bpl('fs←[+ ×]')[1](3, 4).py == 12
    teq(list(bpl('•ucs "a\r\nb\rc"').py), [97, 10, 98, 10, 99])


def test_words_binding_and_operators():
    np.testing.assert_array_equal(iota(3), [0, 1, 2])
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
    assert type(tally('abc').py) is int and shape(Array([1., 2.])).bpl == '[2]ₓ'
    assert (Array('abc') + 1).py == 'bcd'
    np.testing.assert_array_equal(plus.reduce[0](Array([[1, 2], [3, 4]])), [4, 6])
    np.testing.assert_array_equal(plus.reduce[1, 2](np.arange(1, 9).reshape(2, 2, 2)), [10, 26])
    np.testing.assert_array_equal(times.outer_product([1, 2], [3, 4]), [[3, 4], [6, 8]])
    assert (plus @ times)([1, 2], [3, 4]).py == 11
    assert (subtract(1) ** 3)(10).py == 7
    assert (times(2.) ** -1)(10).py == 5
    assert (floor << times(10.))(1.25).py == 12
    assert (times(10.) >> floor)(1.25).py == 12
    assert atop(floor, times(10.))(1.25).py == 12
    assert plus.under(logarithm)(2., 3.).py == pytest.approx(6.)
    np.testing.assert_array_equal(reverse.rank(1)(Array([[1, 2], [3, 4]])), [[2, 1], [4, 3]])
    np.testing.assert_array_equal(times(2).at([1, 3])([1, 2, 3, 4]), [1, 4, 3, 8])
    with pytest.raises(TypeError): plus @ Array(2)
    assert plus.over(3)(10).py == 3
    with pytest.raises(ValueError, match='DOMAIN'): plus.stencil(times)
    assert subtract.commute.reduce([1, 2, 3]).py == 0
    teq(times(2).power.each(3)([1, 2]).py, [8, 16])
    teq(subtract.left.each(2)([1, 2, 3]).py, [1, 0, -1])
    teq(plus.left(1).power.each.power(2)(3)([0, 10]).py, [6, 16])
    teq(plus.power.reduce[0](1)([[1, 2], [3, 4]]).py, [4, 6])
    teq((subtract.left.each + 10)(2)([3, 4]).py, [9, 8])
    teq((sign.valences(plus)(-3).py, sign.valences(plus)(2, 3).py), (-1, 5))


def test_math_construction():
    from basedpl import prime, prime_mode, factors, factor_spec, roots, polyval, windows
    f = plus.left(1).inverse_pair(subtract(1))
    np.testing.assert_array_equal(f.power([2, -1, 0])(10), [12, 9, 10])
    np.testing.assert_array_equal(f.history(-2)(10), [10, 9, 8])
    stop = bpl('limit←13 ⋄ {⍺≥limit}')
    np.testing.assert_array_equal(f.history(stop)(10), [10, 11, 12, 13])
    np.testing.assert_array_equal(windows(2, [1, 2, 3]), [[1, 2], [2, 3]])
    assert prime(9).py == 29 and prime_mode(1, 29).py == 1
    np.testing.assert_array_equal(factors(700), [2, 2, 5, 5, 7])
    np.testing.assert_array_equal(factor_spec(float('inf'), 700), [2, 0, 2, 1])
    np.testing.assert_array_equal(roots.undo([2, [1, 3]]), [6, -8, 2])
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
    bpl(M=keyed)
    assert bpl('⍴M').py == dict(city=2, month=3)
    renamed = bpl('("town" 1:⍴M)⍴M')
    teq(renamed.axis_names, ('town', None))
    teq(renamed.axis_keys, keyed.axis_keys)
    assert bpl('⍴("town" 1:⍴M)⍴M').py == {'town': 2, 1: 3}
    teq(bpl('(:⍴M)⍴M').axis_names, (None, None))
    teq(bpl('(⍴M)⍴(:⍴M)⍴M').axis_names, keyed.axis_names)
    assert bpl('⍴("items":2)⍴1 2').py == dict(items=2)
    for code in ['["city":2 "city":3]⍴M', '("city" "city":⍴M)⍴M', '(1 1:⍴M)⍴M']:
        with pytest.raises(BplError, match='DOMAIN'): bpl(code)
    teq(bpl('"Paris"⌷⍠"city" M').np, data[0])
    teq(bpl('1⌷M').axis_names, ('month',))
    teq(bpl('+/⍠"city" "month" M').np, 15)
    teq(bpl('⍉M').axis_names, ('month', 'city'))
    teq(bpl('2 3⍴M').axis_names, (None, None))
    teq(bpl('(⍴M)⍴M').axis_names, ('city', 'month'))
    teq(bpl(':M').axis_names, ('city', 'month'))
    teq(bpl('+/¨⊂⍠1 M').axis_names, ('city',))
    teq(bpl('⌽⍤1 M').axis_names, ('city', 'month'))
    with pytest.raises(BplError, match='INDEX'): bpl('+/⍠"missing" M')
    v = Array([1, 2], axis_names=('city',))
    teq(times.outer_product(v, v).axis_names, (None, None))
    teq(reshape([4], v).axis_names, (None,))
    teq(plus.scan(v).axis_names, ('city',))
    bpl(V=v)
    for code, names in [('2#V', ('city',)), ('1↕V', (None, None)), ('{⍵}⌺1 V', (None, None)), ('V,V', ('city',))]:
        teq(bpl(code).axis_names, names)
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
    assert pick(1, fs)(2, 3).py == 6
    assert fs.py[0](2, 3).py == 5
    fs = bpl('offset←10 ⋄ [{offset+⍵} +]')
    bpl(offset=20)
    for f in [first(fs), first(list(fs)[0]), fs.py[0], first(Array(fs.np)[0]), bpl.fn('{↑⍵}')(fs)]: assert f(3).py == 23
    assert pick(1, reverse(fs))(3).py == 23
    bpl(fs=fs)
    assert bpl('f←0⊃fs ⋄ f 3').py == 23


def test_retained_and_late_bound_functions(capsys):
    with pytest.raises(BplError) as caught: bpl.fn('{')
    assert caught.value.source == '{'
    bpl(k=2)
    f = bpl('{k+⍵}')
    assert f(3).py == 5
    bpl(k=10)
    assert f(3).py == 13
    late = bpl.fn('g')
    bpl('g←+')
    composed = late + times(2)
    assert composed(3).py == 9
    assert late.reduce([]).py == 0
    assert (bpl.fn('+') @ bpl.fn('×'))([], []).py == 0
    np.testing.assert_array_equal(bpl.fn('⌽')[0]([[1, 2], [3, 4]]), [[3, 4], [1, 2]])
    bpl('scale←2×')
    assert (bpl.fn('scale') ** -1)(6).py == 3
    bpl(fold=late.reduce)
    assert bpl('fold ⍬').py == 0
    bpl(loop=bpl.fn('loop'))
    with pytest.raises(BplError, match='LIMIT'): bpl('loop 1')
    assert bpl('1+1').py == 2
    assert bpl('count←{⍵=0?0;1+∇⍵-1} ⋄ count 500').py == 500
    bpl('g←-')
    assert composed(3).py == 3
    saved = bpl('g')
    bpl('g←+')
    assert saved(3).py == -3 and late(3).py == 3
    assert bpl('f 5', f=times(2)).py == 10
    bpl('outer←{k←100 ⋄ ⍶ ⍵}', f=f)
    assert bpl('f outer 1').py == 11
    g = bpl('{⎕←⍵ ⋄ ⍵}')
    assert (g + g)(2).py == 4 and capsys.readouterr().out == '2ₓ\n2ₓ\n'
    assert bpl('g', 'explicit', g=g).value(2).py == 2
    assert capsys.readouterr().out == '2ₓ\n'
    assert saved(3).py == -3


def test_function_inspection():
    source = '{⍝ Mean of a vector\n(+/⍵)÷≢⍵}'
    bpl('mean←'+source)
    mean = bpl.fn('mean')
    teq(mean.source, source)
    assert 'Mean of a vector' in mean.__doc__
    assert 'adds' in plus.__doc__
    teq(bpl.names(prefix='me'), ['mean'])
    assert bpl.inspect('mean')['kind'] == 'function'
    assert bpl.inspect('mean 1 2 3') is None
    assert 'Mean of a vector' in '\n'.join(bpl(']help mean', 'explicit').output)
    assert source in '\n'.join(bpl(']help mean -source', 'explicit').output)
    for command in [']help', ']help mean -other', ']help mean -source extra']:
        with pytest.raises(BplError, match='SYNTAX'): bpl(command, 'explicit')
    with pytest.raises(BplError, match='VALUE'): bpl(']help absent', 'explicit')
    bpl('changed←0 ⋄ danger←{changed+←1 ⋄ ⍵}')
    assert bpl.inspect('danger')['source'].startswith('{')
    teq(bpl('changed').py, 0)
    bpl('mean←{42}')
    teq(mean.source, '{42}')
