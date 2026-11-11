import http.server, operator, threading
from fractions import Fraction
from pathlib import Path
import numpy as np, pytest
import basedpl
from fastcore.test import test_eq as teq
from basedpl import (Array, bpl, BplError, add, multiply, subtract, divide, exponent, sign, tally, indices,
    reshape, shape, floor, natural_log, reverse, transpose, fork, atop, first, pick, not_)


def test_documentation_examples():
    for path in (Path(__file__).resolve().parents[1]/'nbs').glob('*.qmd'):
        namespace = {}
        for block in path.read_text().split('```python\n')[1:]: exec(compile(block.split('```', 1)[0], str(path), 'exec'), namespace)


def test_builtin_attributes():
    for api in (basedpl, bpl):
        teq(api.add([1, 2], 10).py, [11, 12])
        teq((api.add(2)(3).py, api.multiply(2)(3).py, api.divide(2)(3).py), (5, 6, 1))
        teq((api.plus(2, 3).py, api.times(-2).py, api.downstile(2.5).py), (5, -1, 2))
        teq(getattr(api, '•distribution')([2, 0.5], 'binomial')['quantile'](1.).py, 2)
        assert {'add', 'plus', 'conjugate', 'distribution', 'π'} <= set(dir(api))
        teq(api.plu(2, 3).py, 5)
        for name in ('userfn', 'nonexistent', 'abs', 'disclose', 'diaeresis'):
            with pytest.raises(AttributeError): getattr(api, name)
    bpl('add←99 ◇ userfn←{ω+1}')
    teq(bpl.add(2, 3).py, 5)
    teq(bpl['add'].py, 99)
    teq(bpl.userfn(1).py, 2)
    assert 'userfn' in dir(bpl)
    teq(bpl.execute('add').py, 99)
    teq(bpl.execute_in({'a': 1, 'b': 2}, 'a+b').py, 3)
    teq(bpl.not_(0).py, 1)
    assert bpl.names.__func__ is type(bpl).names
    from basedpl import add, conjugate
    teq((add(2)(3).py, conjugate(2).py), (5, 2))
    assert 'add' not in vars(basedpl)


def test_python_printer():
    from basedpl import to_python, same
    for code, expected in {
        '×': 'times', '×↢2': 'multiply(2.)', '2↣-': '2. | subtract', '2-': '2. | subtract',
        '×↢2ₓ': 'multiply(2)', '÷↢1r2': 'divide(Fraction(1, 2))',
        '+/÷≢': 'add.reduce / equal_underbar_slash', '+.×': 'add @ multiply', '×⊗': 'multiply.outer_product',
        '+/⍠1': 'add.reduce.axis(1.)', '1↣+⍣[≡]': '(1. | add).power([match])', '+⌿': 'add.reduce_first', '-⍨': 'subtract.tilde_diaeresis',
        '+∘×': 'conjugate.atop(times)', '+⍥×': 'plus.over(sign)', '-↣+': 'negate.before(add)', '+↢-': 'add.after(negate)', '-⊘+': 'negate.valences(add)',
        '{ω<0}⍚[⊢ -]': 'fn("{ω<0}").agenda(bpl("[⊢ -]"))', '[1 2◇3 4]↣+': 'bpl("[1 2◇3 4]") | add',
        '{ω×2}': 'fn("{ω×2}")', '{ω×2}¨': 'fn("{ω×2}").each', '(×↢2)⁻': 'multiply(2.).undo', '⌽⍠0': 'circle_stile.axis(0.)',
    }.items(): teq(to_python(bpl(code)), expected)
    teq(eval(to_python(bpl('+/⍠1')))([[1, 2], [3, 4]]).py, [3, 7])
    selected = same[None, ..., ::-1]
    teq(eval(to_python(selected))([[1, 2], [3, 4]]).py, [[[2, 1], [4, 3]]])
    selected = same[['bb', 'aa']]
    teq(to_python(selected), "same[['bb', 'aa']]")
    teq(eval(to_python(selected))({'aa': 1, 'bb': 2}).py, {'bb': 2, 'aa': 1})
    assert '<slice(1, None, 2)>' in repr(same[1::2])
    assert '<slice("a", "b", 1)>' in repr(same['a':'b'])
    teq(to_python(bpl('×'), dyad=True), 'multiply')
    teq(to_python(bpl('-⍨'), dyad=True), 'subtract.commute')
    teq(to_python(bpl('+∘×'), dyad=True), 'conjugate.atop(multiply)')
    teq(to_python(multiply(2.) ** 3), 'multiply(2.) ** 3')
    teq(to_python((add.reduce / tally).each), '(add.reduce / equal_underbar_slash).each')
    teq(to_python(bpl.fn('{⎕←1 ◇ ω}')), 'fn("{⎕←1 ◇ ω}")')
    teq(to_python(bpl.fn('unknown')), 'fn("unknown")')
    teq(to_python(bpl.fn('(⎕←1)+')), 'fn("(⎕←1)+")')
    teq(to_python(Array([add, multiply]).fn), 'bpl("[+ ×]").fn')
    for name in ('exact', 'rational'):
        assert to_python(getattr(add, name)).endswith('.'+name)


def test_load(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    lib = tmp_path/'lib'
    lib.mkdir()
    source = lib/'defs.bpl'
    source.write_text('_half←{ω÷2}\nhalve←{_half ω}\ntwice←{⍶ ⍶ ω}\nx←7\n')
    (lib/'main.bpl').write_text('[halve:]←•load "./defs.bpl"\nq←halve 10\n')
    m = bpl('•load "lib/defs.bpl"').py
    assert sorted(m) == ['halve', 'twice', 'x']
    assert m['twice'](bpl.fn('{ω+1}'))(3).py == 5
    assert bpl('{m←•load "lib/defs.bpl" ◇ [halve: x:]←m ◇ (halve x),-m.twice 3}0').py.tolist() == [3.5, 3]
    assert 'halve' not in bpl.names()
    assert bpl('•LOAD "lib/main.bpl"').py['q'] == 5
    (lib/'a.bpl').write_text('•load "./b.bpl"')
    (lib/'b.bpl').write_text('•load "./a.bpl"')
    with pytest.raises(BplError, match='load cycle'): bpl('•load "lib/a.bpl"')
    source.write_text('x←7\n⎕←x\nx\n1÷"a"')
    with pytest.raises(BplError, match=r'defs.bpl:4') as err: bpl('•load "lib/defs.bpl"')
    assert err.value.output == ['7']
    source.write_text('⎕←9\n{∇ω}0')
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
    monkeypatch.setattr('builtins.input', lambda prompt: prompts.append(prompt) or bpl('"b","ob"').py)
    teq(bpl('"-" •nput "Name? " ◇ "Hi ",⎕').py, 'Hi bob')
    teq(prompts, ['Name? '])
    bpl('"-" •nput "a" ◇ ⎕←1')
    teq(capsys.readouterr().out, 'a1\n')


def test_binary_files(tmp_path):
    path, dest = tmp_path/'bytes', tmp_path/'copy'
    data = bytes(range(256))
    path.write_bytes(data)
    read, write = bpl.nget, bpl.nput
    values = read(str(path), binary=1)
    teq(values.np.dtype, np.dtype('int64'))
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
    threading.Thread(target=server.serve_forever, args=(0.01,), daemon=True).start()
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
    r, replace = bpl.fn('•r'), bpl.fn('•replace')
    pattern = r'([a-z]+)([0-9]+)'
    teq(r('ab12 cd3', pattern)['text'].py, ['ab12', 'cd3'])
    teq(replace('ab12 cd3', [pattern, '$2:$1']).py, '12:ab 3:cd')
    teq(bpl('(s •r p).position', p=pattern, s='é ab12').py, [2])
    teq(r('x9', pattern)['groups'].py, [['x', '9']])
    repeated = r'(?<= )([a-z]+)\1(?![a-z])'
    teq(r('z abab xx', repeated).text.py, ['abab', 'xx'])
    teq(replace('z abab xx', [repeated, '$1']).py, 'z ab x')
    with pytest.raises(BplError, match='backtrack'): r('ab'*64+'!c', r'(?i)(a|b|ab)*(?>c)')


def test_distribution_functions():
    distribution = bpl.fn('•distribution')
    normal, binomial = distribution([3., 2.], 'normal'), distribution([10, 0.5], 'binomial')
    sample = normal.sample
    x, y = sample(10000).np, binomial.sample(10000).np
    assert x.dtype == np.float64 and np.issubdtype(y.dtype, np.integer)
    assert abs(x.mean()-3) < 0.15 and abs(x.std()-2) < 0.15
    assert np.all((0 <= y) & (y <= 10)) and abs(y.mean()-5) < 0.15
    teq(binomial.quantile([0., 1.]).np, [0, 10])
    teq(normal.cdf(dict(a=3., b=float('inf'))).py, dict(a=0.5, b=1.))
    teq(sample([2, 3]).shape, (2, 3))


def test_array_surface():
    a = Array([[1, 2, 3], [4, 5, 6]])
    assert (a == None) is False and (a != None) is True
    for op in (operator.add, operator.floordiv, operator.lt, operator.matmul, operator.and_):
        with pytest.raises(TypeError): op(a, None)
    with pytest.raises(TypeError): add(a, None)
    with pytest.raises(BplError, match='LENGTH'): a + [1, 2, 3]
    assert (Array(np.ones(2, np.float32)) + .5).np.dtype == np.float32
    np.testing.assert_array_equal(bpl('m₀ ₀←0 ◇ m', m=a), [[0, 2, 3], [4, 5, 6]])
    np.testing.assert_array_equal(a, [[1, 2, 3], [4, 5, 6]])
    assert len(a) == 2
    assert Array([4, 5, 6]) in a and Array([4, 5, 7]) not in a and Array(2) in Array([1, 2, 3])
    assert 'bb' in Array(['aa', 'bb']) and 'b' not in Array(['aa', 'bb']) and 'b' in Array('abc')
    np.testing.assert_array_equal(a + [10, 20], [[11, 12, 13], [24, 25, 26]])
    np.testing.assert_array_equal(10 - a, [[9, 8, 7], [6, 5, 4]])
    np.testing.assert_array_equal(a[1, :], [4, 5, 6])
    np.testing.assert_array_equal(a[:, [0, 2]], [[1, 3], [4, 6]])
    np.testing.assert_array_equal(a[:, a[0] > 1], [[1, 2, 2], [4, 5, 5]])
    np.testing.assert_array_equal(a[[False, True], np.array([True, False])], [2, 4])
    np.testing.assert_array_equal(a[:, [False, False, False]], [[1, 1, 1], [4, 4, 4]])
    np.testing.assert_array_equal(a[:, [0, 1, 1]], [[1, 2, 2], [4, 5, 5]])
    np.testing.assert_array_equal(a[:, True], [2, 5])
    np.testing.assert_array_equal(bpl('[mask]⌷v', mask=[False, True, True], v=[10, 20, 30]), [10, 20, 20])
    np.testing.assert_array_equal(list(a)[0], [1, 2, 3])
    teq(a[2, 0].py, 0)
    np.testing.assert_array_equal(a[:, np.array([0, 3])], [[1, 0], [4, 0]])
    k = bpl('["aa":1 "bb":2]')
    np.testing.assert_array_equal(k[Array(['bb', 'aa'])], [2, 1])
    assert k['zz'] == 0
    for key in (['aa', 'zz'], Array(['aa', 'zz'])): assert k[key].py == {'aa': 1, 1: 0}
    for key, want in (('zz', dict(aa=1, bb=2, zz=3)), (['aa', 'zz'], dict(aa=3, bb=2, zz=3)), (Array(['aa', 'zz']), dict(aa=3, bb=2, zz=3))):
        written = bpl('["aa":1 "bb":2]')
        written[key] = 3
        assert written.py == want
    short = Array({'a': 1, 'b': 2, 'ba': 3, '': 4})
    names = ['b', 'a']
    teq(short[names].py, {'b': 2, 'a': 1})
    teq(basedpl.same[names](short).py, short[names].py)
    teq(short['ba'].py, 3)
    teq(short[['']].py, {'': 4})
    assert short[[]].shape == (0,)
    short[names] = 9
    teq(short.py, {'a': 9, 'b': 9, 'ba': 3, '': 4})
    nested, paths = bpl('[2 3ρι6ₓ;2 2ρι4ₓ]'), bpl('[[0 1] [1 0]]')
    np.testing.assert_array_equal(nested[paths, 0], [3, 0])
    with pytest.raises(ValueError, match='paths select cells of different shapes'): nested[paths, 1:]
    teq(bpl('[[1 2 3◇4 5 6] [7 8 9◇10 11 12]]')[paths, 1:].py, [[5, 6], [8, 9]])
    np.testing.assert_array_equal(a[1:2, :], [[4, 5, 6]])
    np.testing.assert_array_equal(a[:, ::-1], [[3, 2, 1], [6, 5, 4]])
    np.testing.assert_array_equal(a[:, -2:], [[2, 3], [5, 6]])
    np.testing.assert_array_equal(a[..., 0], [1, 4])
    assert a[:, 5:].shape == (2, 0)
    with pytest.raises(ValueError): a[::0]
    with pytest.raises(IndexError): a[:, :, 1:]
    assert (Array(7) % 3).py == 1 and (Array(7) // 3).py == 2
    assert (Array(2) ** 3).py == 8 and (Array(3) / 2).py == 1
    assert operator.index(Array(3)) == 3 and int(Array(3.5)) == 3 and float(Array(Fraction(1, 2))) == .5
    assert bool(Array([1])) and not bool(Array(0))
    with pytest.raises(ValueError): bool(a)
    with pytest.raises(TypeError): operator.index(Array(3.))
    with pytest.raises(TypeError): hash(a)
    np.testing.assert_array_equal(a > 3, [[0, 0, 0], [1, 1, 1]])
    np.testing.assert_array_equal(~(a > 3), a <= 3)
    product = a @ transpose(a)
    np.testing.assert_array_equal(product, [[14, 32], [32, 77]])
    assert product.np.dtype == np.int64
    assert add.reduce(a).np.dtype == np.int64
    assert Array(2).bpl == '2ₓ' and Array(2.).bpl == '2'
    assert repr(Array(2)) != repr(Array(2.))
    assert [repr(Array(s)) for s in ('text', '', ['ab', 'cd'])] == ["'text'", "''", "['ab', 'cd']"]

def test_pending_indexing():
    selected = (1 | add)[1::2]
    for n in (4, 7, 3):
        a = Array(list(range(n))) + 1
        teq(a[1::2].py, list(range(2, n+1, 2)))
        teq(selected(Array(list(range(n)))).py, a[1::2].py)
        a[1::2] = 0
        teq(a.py, [i+1 if i%2 == 0 else 0 for i in range(n)])
    a = bpl(r'["x":[0.5×ι100000]]')
    assert bpl(r'(•storage x.x₀).pending', x=a).py
    assert a.force() is a
    assert not bpl(r'(•storage x.x₀).pending', x=a).py



def test_python_array_value_semantics():
    a = Array([[1, 2], [3, 4]], keys=[['a', 'b'], None], names=['row', 'col'])
    held, view, f = Array(a), a[:, ::-1], a | add
    assert a.np.dtype == np.int64
    with pytest.raises(ValueError, match='copy'): np.asarray(a, copy=False)
    snapshot = np.asarray(a, dtype=np.uint8, copy=False)
    a[None, ..., 1] = [[20, 40]]
    a.keys, a.names = None, None
    teq(a.np, [[1, 20], [3, 40]])
    teq(held.np, [[1, 2], [3, 4]])
    teq(view.np, [[2, 1], [4, 3]])
    teq(snapshot, held.np)
    teq(f(0).np, held.np)
    assert held.keys == (('a', 'b'), None) and held.names == ('row', 'col')
    assert a.keys == (None, None) and a.names == (None, None)
    expanded = basedpl.same[None, ...](held)
    teq(expanded.py, held[None, ...].py)
    assert expanded.keys == (None, ('a', 'b'), None) and expanded.names == (None, 'row', 'col')
    for inf in (-np.inf, np.inf):
        teq(held[:, inf].np, [0, 0])
        teq(held[bpl('[[0] [1]]'), inf].np, [0, 0])
    nested = bpl('[[1 2 3◇4 5 6] [7 8 9◇10 11 12]]')
    paths = bpl('[[0 1] [1 0]]')
    rows = np.array([[4, 5, 6], [7, 8, 9]])
    teq(nested[paths, None].np, rows[:, None, :])
    teq(nested[None, paths, ..., None].np, rows[None, :, :, None])
    nested[paths, ..., None] = (rows * 10)[:, :, None]
    teq(nested[paths].np, rows * 10)

def test_rustymath_mixed_operators():
    import rustymath as rm
    a, b = Array([[1, 2], [3, 4]]), rm.array([[4, 3], [2, 1]])
    for op in (operator.add, operator.sub, operator.lt, operator.matmul, operator.and_):
        for x,y in ((a, b), (b, a)):
            result = op(x, y)
            assert isinstance(result, Array)
            teq(result.np, op(Array(x), Array(y)).np)

def test_matrix_stack_operator():
    import rustymath as rm
    a, b, v = np.arange(12).reshape(2, 2, 3), np.arange(24).reshape(2, 3, 4), np.arange(3)
    for x,y in ((a, b), (a, b[:, None]), (a, v), (v, b), (v, v)):
        result, expected = Array(x) @ Array(y), rm.matmul(x, y)
        teq(result.np, np.asarray(expected))
    for x,y in ((1, v), (v, 1), ([1], v)): teq(int(Array(x) @ Array(y)), 3)
    with pytest.raises(BplError): Array(v) @ Array(np.ones(2))
    left = dict(names=['batch', 'row', 'k'], keys=[['x', 'y'], ['r', 's'], None])
    right = dict(names=['batch', 'k', 'col'], keys=[['x', 'y'], None, ['a', 'b', 'c', 'd']])
    result, expected = Array(a, **left) @ Array(b, **right), rm.matmul(rm.array(a, **left), rm.array(b, **right))
    assert result.names == expected.names and result.keys == expected.keys
    teq(result.np, np.asarray(expected))

def test_own():
    fresh = bpl('0.5×ι100000')
    items = np.asarray(fresh, copy=False).ctypes.data
    moved = fresh.own()
    assert moved.ctypes.data == items and moved.flags.writeable
    with pytest.raises(ValueError): fresh.shape
    held = bpl('shared←100000ρ0.5 ◇ shared')
    twin = Array(held)
    items = np.asarray(held, copy=False).ctypes.data
    assert held.own().ctypes.data != items
    teq(twin.np, np.full(100000, .5))
    teq(bpl('+/shared').py, 50000.)
    chars = bpl('"abc"').own()
    assert chars.tolist() == ['a', 'b', 'c'] and chars.flags.writeable
    flags = bpl('0 1 0=1').own()
    assert flags.dtype == bool and flags.tolist() == [False, True, False] and flags.flags.writeable
    # A structural copy that reads its source through strides lends the source's items, without being built.
    bpl('v←0.5×ι1000000 ◇ m←1000 1000ρv')
    source, windows = np.asarray(bpl['v'], copy=False), np.lib.stride_tricks.sliding_window_view
    for code, expected in [('3↕v', windows(source, 3)), ('⌽v', source[::-1]), ('2↓v', source[2:]), ('2 2↕m', windows(source.reshape(1000, 1000), (2, 2)))]:
        copy = bpl(code)
        lent = np.asarray(copy, copy=False)
        np.testing.assert_array_equal(lent, expected)
        assert np.may_share_memory(lent, source) and bpl('(•storage x).pending', x=copy).py

def test_keyed_arrays():
    t = bpl('["b":[1 2] "a":["x":"hi" "n":3]]')
    assert list(t.py) == ['b', 'a'] and t.py['a'] == dict(x='hi', n=3) and t.shape == (2,)
    np.testing.assert_array_equal(t.py['b'], [1, 2])
    assert repr(t).startswith("{'b': ")
    d = dict(z=1, y=dict(k=[1, 2, 3]), e={})
    assert list(bpl('ι₀t', t=d).py) == ['z', 'y', 'e'] and bpl('t.y.k₁+t.e≡⍬:⍬', t=d).py == 3
    assert (Array(dict(a=1, b=2)) + Array(dict(b=10))).py == dict(a=1, b=12)
    assert Array({'a': 1, 1: 5}).py == {'a': 1, 1: 5}
    with pytest.raises(TypeError): Array({2: 5})
    k = Array(dict(qty=4, price=1, tax=2))
    assert k['price'].py == 1 and k['price'].is_atom and k[1].py == 1
    assert list(k[['tax', 'qty']].py) == ['tax', 'qty']
    assert k.keys == (('qty', 'price', 'tax'),)
    assert t.a.n.py == 3 and k.price.py == 1
    collision = Array(shape=7)
    assert collision.shape == (1,) and collision['shape'].py == 7
    with pytest.raises(AttributeError): k.missing
    assert k['missing'].py == 0
    assert k[['qty', 'qty']].keys == (None,)

def test_keys_dataframe():
    import pandas as pd
    m = Array([[10, 2], [20, 4]], keys=[['alice', 'bob'], ['price', 'qty']])
    assert m.keys == (('alice', 'bob'), ('price', 'qty'))
    assert m['alice'].py == dict(price=10, qty=2)
    assert m[:, 'qty'].py == dict(alice=2, bob=4)
    assert m['alice':'bob', 'qty'].py == dict(alice=2, bob=4)
    np.testing.assert_array_equal(m[:, 'price':'qty'], [[10, 2], [20, 4]])
    assert m['bob':'alice':-1, 'price'].keys == (('bob', 'alice'),)
    assert m[0:'alice', 'price'].py == dict(alice=10)
    with pytest.raises(KeyError): m['carol':]
    with pytest.raises(KeyError): Array([1, 2])['a':]
    pd.testing.assert_frame_equal(m.py, pd.DataFrame(np.array([[10, 2], [20, 4]], dtype=np.int64), index=['alice', 'bob'], columns=['price', 'qty']))
    np.testing.assert_array_equal(m.np, [[10, 2], [20, 4]])
    h = Array(np.arange(8).reshape(2, 2, 2), keys=[['aa', 'bb'], None, ['xx', 'yy']])
    pd.testing.assert_frame_equal(h.py, pd.DataFrame(np.arange(8).reshape(4, 2),
        index=pd.MultiIndex.from_product([['aa', 'bb'], [0, 1]]), columns=['xx', 'yy']))
    pd.testing.assert_frame_equal(Array(3).df, pd.DataFrame([[3]], index=[0], columns=[0]))
    pd.testing.assert_frame_equal(Array([3, 4]).df, pd.DataFrame(np.array([3, 4], dtype=np.int64), index=[0, 1], columns=[0]))
    assert Array([[1, 2]]).keys == (None, None)
    for keys in ([None], [['a', 'a'], None], [None, ['a']]):
        with pytest.raises(ValueError): Array([[1, 2], [3, 4]], keys=keys)

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
    assert bpl('v←3 4 ◇ 1⌷v').is_atom
    assert not bpl('[⊂1]⌷v').is_atom
    assert bpl('1⊃v').is_atom
    assert Array([3, 4])[1].is_atom and not Array([3, 4])[np.array(1)].is_atom
    assert bpl('fs←[+ ×]')[1](3, 4).py == 12
    teq(list(bpl('•ucs "a\r\nb\rc"').py), [97, 10, 98, 10, 99])


def test_words_binding_and_operators():
    f = bpl.fn('⊢')
    assert int(Array([6]) & 3) == 2
    with pytest.raises(BplError): Array([6.5]) & 3
    assert int((f & 3)(6)) == 2 and int(Array(3) | 6) == 7 and int((~f)(6)) == -7
    np.testing.assert_array_equal(indices(3), [0, 1, 2])
    np.testing.assert_array_equal(shape(reshape([2, 3], indices(6))), [2, 3])
    assert multiply(2.)(3).py == 6. and type(multiply(2.)(3).py) is float
    assert multiply(2)(3).py == 6 and type(multiply(2)(3).py) is int
    assert subtract(2.)(5).py == 3 and (2. | subtract)(5).py == -3
    assert (Fraction(1) | divide)(3).py == Fraction(1, 3)
    assert sign(-3).py == -1 and exponent(2)(3).py == 9
    assert not_(0).py == 1
    mean = add.reduce / tally
    assert mean([1, 2, 4]).py == 2
    assert fork(add.reduce, divide, tally)([1, 2, 4]).py == 2
    assert (add.reduce + 2)([1, 2]).py == 5
    assert (2 - add.reduce)([1, 2]).py == -1
    np.testing.assert_array_equal(add.scan([1, 2, 3]), [1, 3, 6])
    np.testing.assert_array_equal(subtract.scan([1, 2, 3]), [1, -1, -4])
    np.testing.assert_array_equal(add.scan(10, [1, 2, 3]), [11, 13, 16])
    assert type(tally('abc').py) is int and shape(Array([1., 2.])).bpl == '[2]ₓ'
    assert (Array('abc') + 1).py == 'bcd'
    np.testing.assert_array_equal(add.reduce_first(Array([[1, 2], [3, 4]])), [4, 6])
    np.testing.assert_array_equal(add.reduce.axis([1, 2])(np.arange(1, 9).reshape(2, 2, 2)), [10, 26])
    teq(subtract.axis(0)([10, 20])([1, 2]).py, [-9, -18])
    teq(subtract[0]([10, 20])([1, 2]).py, -9)
    teq(add.rank[1:](1)([[1, 2], [3, 4]]).py, [[3, 4]])
    np.testing.assert_array_equal(multiply.outer_product([1, 2], [3, 4]), [[3, 4], [6, 8]])
    assert (add @ multiply)([1, 2], [3, 4]).py == 11 == add.inner_product(multiply)([1, 2], [3, 4]).py
    assert subtract.determinant(multiply)([[1, 2], [3, 4]]).py == -2
    assert subtract.dot(multiply)([[1, 2], [3, 4]]).py == -2 and add.dot(multiply)([1, 2], [3, 4]).py == 11
    assert (subtract(1) ** 3)(10).py == 7
    assert (multiply(2.) ** -1)(10).py == 5
    assert (floor << multiply(10.))(1.25).py == 12
    assert (multiply(10.) >> floor)(1.25).py == 12
    for value in ([1.25, 2.75], Array([1.25, 2.75]), np.array([1.25, 2.75])): teq((value >> (multiply(10.) >> floor)).py, [12, 27])
    assert atop(floor, multiply(10.), add(1.))(1.25).py == 22
    assert add.under(natural_log)(2., 3.).py == pytest.approx(6.)
    np.testing.assert_array_equal(reverse.rank(1)(Array([[1, 2], [3, 4]])), [[2, 1], [4, 3]])
    np.testing.assert_array_equal(multiply(2).under([1, 3])([1, 2, 3, 4]), [1, 4, 3, 8])
    with pytest.raises(TypeError): add @ Array(2)
    assert add.over(3)(10).py == 3
    with pytest.raises(ValueError, match='DOMAIN'): add.agenda(multiply)
    assert subtract.commute.reduce([1, 2, 3]).py == 0 and subtract.self(3).py == 0 and subtract.commute(1, 3).py == 2
    assert subtract.tilde_diaeresis(3).py == 0 and subtract.tilde_diaeresis(1, 3).py == 2 and add.slash([1, 2, 3]).py == 6
    teq(multiply(2).power.each(3)([1, 2]).py, [8, 16])
    teq((2 | subtract).each([1, 2, 3]).py, [1, 0, -1])
    teq((1 | add).power.each.power(2)(3)([0, 10]).py, [6, 16])
    teq(add.power.reduce_first(1)([[1, 2], [3, 4]]).py, [4, 6])
    teq(((2 | subtract).each + 10)([3, 4]).py, [9, 8])
    teq((sign.valences(add)(-3).py, sign.valences(add)(2, 3).py), (-1, 5))


def test_function_trains_and_binding():
    plus, sqrt = basedpl.plus, basedpl.sqrt
    values = [1., 2., 6.]
    mean = plus.reduce.divide.tally
    teq(mean(values).py, 3.)
    assert sqrt(plus.reduce / tally)(values).py == pytest.approx(3.**.5)
    for chain, source in ((mean.sqrt, '+/÷≢√'), (subtract.divide.left, '-÷⊣')):
        teq(chain(2., 3.).py, bpl.fn(source)(2., 3.).py)
    teq(plus.reduce.divi.tall(values).py, 3.)
    teq(basedpl.conj(2).py, 2)
    teq((divide | 2)(6.).py, 3.)
    teq((2 | divide)(4.).py, .5)
    teq((Array([1, 2]) | subtract)(3).py, [-2, -1])
    teq((Array([1, 2]) | subtract).each([3, 4]).py, [[-2, -1], [-3, -2]])
    assert {'reduce', 'reduce_first', 'divide', 'tally'} <= set(dir(plus))
    with pytest.raises(AttributeError, match='reduce.*reduce_first'): plus.re
    with pytest.raises(AttributeError, match='subscript.*subtract'): basedpl.sub
    with pytest.raises(AttributeError): plus._absent


def test_bulk_catenate_reduction():
    cat = basedpl.catenate
    pieces = [Array([1, 2]), Array([3]), Array([4, 5])]
    for copies in (1, 32): teq(cat.reduce(pieces*copies).py, [1, 2, 3, 4, 5]*copies)
    teq(cat.reduce([pieces[0][:0], *pieces, pieces[0][:0]]).py, [1, 2, 3, 4, 5])
    teq((cat.reduce([]).py, cat.reduce([pieces[0]]).py), ([], [1, 2]))
    matrices = [Array([[1], [2]]), Array([[3], [4]]), Array([[5], [6]])]
    teq(cat.axis(1).reduce(matrices).py, [[1, 3, 5], [2, 4, 6]])
    teq(basedpl.catenate_first.reduce(matrices).py, [[i] for i in range(1, 7)])
    teq(cat.reduce([matrices[0], Array([3, 4]), matrices[2]]).py, [[1, 3, 5], [2, 4, 6]])
    with pytest.raises(BplError, match='LENGTH'): cat.axis(1).reduce([matrices[0], Array([[1], [2], [3]])])
    named = [Array([i], names=(name,)) for i,name in enumerate(('x', 'y', 'z'))]
    assert cat.reduce(named).names == ('x',)
    keyed = [Array([[1, 2]], keys=[None, ['a', 'b']]), Array([[30, 40]], keys=[None, ['b', 'a']]), Array([[5, 6]], keys=[None, ['a', 'b']])]
    joined = cat.axis(0).reduce(keyed)
    teq(joined.np.tolist(), [[1, 2], [40, 30], [5, 6]])
    assert joined.keys == (None, ('a', 'b'))
    converted = [Array(np.array([0.], np.float64)), Array(np.array([16777217], np.int64)), Array(np.array([1.], np.float32))]
    teq(cat.reduce(converted).py, [0., 16777216., 1.])


def test_math_construction():
    from basedpl import prime, prime_mode, factors, factor_spec, roots, polyval, windows
    f = (1 | add).undo(subtract(1))
    np.testing.assert_array_equal(f.power([2, -1, 0])(10), [12, 9, 10])
    np.testing.assert_array_equal(f.power([0, -1, -2])(10), [10, 9, 8])
    stop = bpl('limit←13 ◇ {α≥limit}')
    np.testing.assert_array_equal(f.power([stop])(10), [10, 11, 12, 13])
    np.testing.assert_array_equal(windows(2, [1, 2, 3]), [[1, 2], [2, 3]])
    assert prime(9).py == 29 and prime_mode(1, 29).py == 1
    np.testing.assert_array_equal(factors(700), [2, 2, 5, 5, 7])
    np.testing.assert_array_equal(factor_spec(float('inf'), 700), [2, 0, 2, 1])
    np.testing.assert_array_equal(roots.undo([2, [1, 3]]), [6, -8, 2])
    p = [1, 2, 3] | polyval
    assert p.derivative(2).py == 14 and p.derivative.derivative(2).py == 6
    np.testing.assert_array_equal(p.derivative([1, 2]), [[8, 0], [0, 14]])
    rosen = bpl('{[x y]←ω ◇ ⊼1-x + 100×⊼y-⊼x}')
    jvp = bpl('{[x y]←ω ◇ +/α×[(¯2×1-x) - 400×x×y-⊼x; 200×y-⊼x]}')
    r = rosen.derivative(jvp)
    assert r([-1, 2]).py == 104
    np.testing.assert_array_equal(r.derivative([-1, 2]), [396, 200])
    np.testing.assert_allclose(rosen.optimize([-1.2, 1]), [1, 1], atol=1e-6)
    found = rosen.optimize([0.5, 0.5], lower=[-np.inf, 0], upper=[0.8, np.inf], report=True).py
    np.testing.assert_allclose(found['x'], [0.8, 0.64], atol=1e-6)
    assert found['converged']
    loss = [1, 2, 3, 4] | bpl('{+/⊼α-ω}')
    np.testing.assert_allclose([loss.optimize(0), loss.optimize(0, method='powell')], [2.5, 2.5])
    np.testing.assert_allclose(bpl('2-⍨⊼').optimize(1, solve=True), 2**0.5)
    np.testing.assert_allclose(bpl('{(ω×4)-8}').optimize(0, solve=True, linear=True, precondition=bpl('{ω÷4}')), 2)


def test_names():
    data = np.arange(6).reshape(2, 3)
    m = Array(data, names=('city', 'month'))
    month = Array([10, 20, 30], names=('month',))
    teq((m + month).np, data + [10, 20, 30])
    teq((m + [10, 20]).np, data + np.array([[10], [20]]))
    teq((month + m).names, ('city', 'month'))
    teq((month + m).np, data + [10, 20, 30])
    teq((m + transpose(m)).np, data * 2)
    a = Array(data, names=('city', None))
    b = Array(data.T, names=(None, 'city'))
    teq((a + b).np, data * 2)
    product = Array(np.arange(12).reshape(4, 3), names=('product', None))
    teq((a + product).names, ('city', None, 'product'))
    teq((a + product).np, data[:, :, None] + product.np.T[None, :, :])
    teq(add.reduce.axis('month')(m).names, ('city',))
    teq(add.reduce.axis('month')(m).np, [3, 12])
    keyed = Array(m, keys=(('Paris', 'London'), ('Jan', 'Feb', 'Mar')))
    other = Array([30, 20, 10], names=('month',), keys=(('Mar', 'Feb', 'Jan'),))
    teq((keyed + other).np, data + [10, 20, 30])
    assert keyed.df.index.name == 'city' and keyed.df.columns.name == 'month'
    bpl(M=keyed)
    assert bpl('ρM').py == dict(city=2, month=3)
    renamed = bpl('("town" 1:ρM)ρM')
    teq(renamed.names, ('town', None))
    teq(renamed.keys, keyed.keys)
    assert bpl('ρ("town" 1:ρM)ρM').py == {'town': 2, 1: 3}
    teq(bpl('(:ρM)ρM').names, (None, None))
    teq(bpl('(ρM)ρ(:ρM)ρM').names, keyed.names)
    assert bpl('ρ("items":2)ρ1 2').py == dict(items=2)
    for code in ['["city":2 "city":3]ρM', '("city" "city":ρM)ρM', '(1 1:ρM)ρM']:
        with pytest.raises(BplError, match='DOMAIN'): bpl(code)
    teq(bpl('"Paris"⌷⍠"city" M').np, data[0])
    teq(bpl('1⌷M').names, ('month',))
    teq(bpl('+/⍠"city" "month" M').np, 15)
    teq(bpl('⍉M').names, ('month', 'city'))
    teq(bpl('2 3ρM').names, (None, None))
    teq(bpl('(ρM)ρM').names, ('city', 'month'))
    teq(bpl(':M').names, (None, None))
    teq(bpl('(ρM)ρ,M').names, ('city', 'month'))
    teq(bpl('+/¨⊂₁M').names, ('city',))
    teq(bpl('⌽⍤1 M').names, ('city', 'month'))
    with pytest.raises(BplError, match='INDEX'): bpl('+/⍠"missing" M')
    v = Array([1, 2], names=('city',))
    teq(multiply.outer_product(v, v).names, (None, None))
    teq(reshape([4], v).names, (None,))
    teq(add.scan(v).names, ('city',))
    bpl(V=v)
    for code, names in [('2#V', ('city',)), ('1↕V', (None, None)), ('⊢⌺1 V', (None, None)), ('V,V', ('city',))]:
        teq(bpl(code).names, names)
    with pytest.raises(ValueError): Array(data, names=('city', 'city'))
    with pytest.raises(ValueError): Array(data, names=('city',))


def test_mathematical_monads():
    from basedpl import sqrt, root, square, double, decrement, increment, pi_ratio, cis, real_imag, polar, classify, binary_encode, binary_decode
    assert sqrt(Fraction(1, 4)).py == 0.5 and root(3, -8).py == -2
    assert square(3).py == 9 and double(3).py == 6 and decrement(increment(3)).py == 3
    assert complex(cis(pi_ratio(1, 2)).py) == pytest.approx(1j)
    np.testing.assert_array_equal(real_imag(3+4j), [3., 4.])
    np.testing.assert_allclose(polar(1j), [1., np.pi/2])
    np.testing.assert_array_equal(classify('aba'), [[1, 0, 1], [0, 1, 0]])
    np.testing.assert_array_equal(binary_decode(binary_encode([2, 5])), [2, 5])


def test_function_arrays():
    fs = Array([add, multiply])
    assert pick(1, fs)(2, 3).py == 6
    assert fs.py[0](2, 3).py == 5
    fs = bpl('offset←10 ◇ [{offset+ω} +]')
    bpl(offset=20)
    for f in [first(fs), first(list(fs)[0]), fs.py[0], first(Array(fs.np)[0]), bpl.fn('{↑ω}')(fs)]: assert f(3).py == 23
    assert pick(1, reverse(fs))(3).py == 23
    bpl(fs=fs)
    assert bpl('f←0⊃fs ◇ f 3').py == 23
    assert isinstance(fs, basedpl.FunctionArray) and not callable(Array([1, 2]))
    v = Array([1, 2])
    v[0] = add
    np.testing.assert_array_equal(v(3, 4), [7, 2])
    v[0] = 0
    assert not callable(v)
    class CustomArray(Array): pass
    custom = CustomArray([1, 2])
    custom[0] = add
    assert type(custom) is CustomArray
    bpl('aggs←[+ -]')
    np.testing.assert_array_equal(bpl.fn('aggs')(3), [3, -3])


def test_retained_and_late_bound_functions(capsys):
    with pytest.raises(BplError) as caught: bpl.fn('{')
    assert caught.value.source == '{'
    bpl(k=2)
    f = bpl('{k+ω}')
    assert f(3).py == 5
    bpl(k=10)
    assert f(3).py == 13
    late = bpl.fn('g')
    bpl('g←+')
    composed = late + multiply(2)
    assert composed(3).py == 9
    assert late.reduce([]).py == 0
    assert (bpl.fn('+') @ bpl.fn('×'))([], []).py == 0
    np.testing.assert_array_equal(bpl.fn('⌽').axis(0)([[1, 2], [3, 4]]), [[3, 4], [1, 2]])
    bpl('scale←2×')
    assert (bpl.fn('scale') ** -1)(6).py == 3
    bpl(fold=late.reduce)
    assert bpl('fold ⍬').py == 0
    bpl(loop=bpl.fn('loop'))
    with pytest.raises(BplError, match='LIMIT'): bpl('loop 1')
    assert bpl('1+1').py == 2
    assert bpl('count←{ω=0?0;1+∇ω-1} ◇ count 500').py == 500
    bpl('g←-')
    assert composed(3).py == 3
    saved = bpl('g')
    bpl('g←+')
    assert saved(3).py == -3 and late(3).py == 3
    assert bpl('f 5', f=multiply(2)).py == 10
    bpl('outer←{k←100 ◇ ⍶ ω}', f=f)
    assert bpl('f outer 1').py == 11
    g = bpl('{⎕←ω ◇ ω}')
    assert (g + g)(2).py == 4 and capsys.readouterr().out == '2ₓ\n2ₓ\n'
    assert bpl('g', 'explicit', g=g).value(2).py == 2
    assert capsys.readouterr().out == '2ₓ\n'
    assert saved(3).py == -3


def test_function_inspection():
    def help_text(command): return '\n'.join(bpl(command, 'explicit').output)
    source = '{⍝ Mean of a vector\n(+/ω)÷≢ω}'
    bpl('mean←'+source)
    mean = bpl.fn('mean')
    teq(mean.source, source)
    assert 'Mean of a vector' in mean.__doc__
    assert 'adds' in add.__doc__
    teq(bpl.names(prefix='me'), ['mean'])
    assert bpl.inspect('mean')['kind'] == 'function'
    assert bpl.inspect('mean 1 2 3') is None
    assert 'Mean of a vector' in '\n'.join(bpl(']help mean', 'explicit').output)
    assert source in '\n'.join(bpl(']help mean -source', 'explicit').output)
    bpl('llms←1')
    assert '# BasedPL' in '\n'.join(bpl(']help llms', 'explicit').output)
    assert ']help indexing' in help_text(']help')
    assert '# Indexing' in help_text(']help indexing')
    assert ']help indexing' in help_text(']help indexi')
    assert ']help indexing' in help_text(r']help? ^INDEXING$')
    results = help_text(r']help? INDEX(?!ing[0-9])')
    assert results.index(']help indexing') < results.index(']help arrays')
    with pytest.raises(BplError, match='DOMAIN'): help_text(']help? (?')
    for command in [']help?', ']help ? indexing', ']help mean -other', ']help mean -source extra']:
        with pytest.raises(BplError, match='SYNTAX'): bpl(command, 'explicit')
    assert 'No matching topics' in help_text(']help nonexistent_topic_xyz')
    bpl('changed←0 ◇ danger←{changed+←1 ◇ ω}')
    assert bpl.inspect('danger')['source'].startswith('{')
    teq(bpl('changed').py, 0)
    bpl('mean←{42}')
    teq(mean.source, '{42}')
