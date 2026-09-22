import subprocess, json, sys, os, signal, threading, weakref, gc
from fractions import Fraction
from concurrent.futures import ThreadPoolExecutor
import numpy as np, pytest
from basedpl import Session, AplError, Array


def test_native_calls_and_explicit_output():
    with Session() as s:
        a = s('1r3 2x')
        s['x'] = a
        r = s.eval('⎕←x ⋄ x')
        np.testing.assert_array_equal(r.value, a)
        assert r.output == ['1r3 2ₓ']
        assert s('1 ⋄ ⎕←2 ⋄ 3').py == 3
        assert s.eval('1 ⋄ ⎕←2 ⋄ 3').output == ['2']
    with Session() as s: assert s('+/x', x=a).py == Fraction(7, 3)


def test_bindings_functions_and_output(capsys):
    with Session() as apl:
        assert apl(x=np.arange(1, 6), source=9, timeout=8) is None
        assert apl('source+timeout').py == 17
        assert apl('+/x').py == 15 and type(apl('+/x')) is Array
        mean = apl.fn('{(+/⍵)÷≢⍵}')
        assert mean([1, 2, 3]).py == 2 and type(mean([1, 2, 3]).py) is int
        assert mean([1, 2, 4]).py == Fraction(7, 3)
        added = apl.fn('+')([1, 2, 3], 10)
        np.testing.assert_array_equal(added, [11, 12, 13])
        assert added.np.dtype == np.int64
        f = apl.fn('foo')
        apl('foo←+')
        assert f(3).py == 3
        apl('foo←-')
        assert f(3).py == -3
        apl['x'] = [[1, 2], [3, 4]]
        np.testing.assert_array_equal(apl['x'], [[1, 2], [3, 4]])
        assert apl('x←7').py == 7 and apl('3 ⋄ f←+') is None and apl('') is None
        assert apl('silent←{a←7} ⋄ silent 0').py == 7 and apl('{}0') is None
        assert capsys.readouterr().out == ''
        assert apl('1 ⋄ ⎕←2 ⋄ ⍎\'3 ⋄ ⎕←4 ⋄ 5\'').py == 5
        assert capsys.readouterr().out == '2\n4\n'
        r = apl.eval('⎕←x ⋄ x+1x', x=9)
        assert r.value.py == 10 and r.output == ['9ₓ'] and capsys.readouterr().out == ''
        assert apl.fn('{⎕←⍵}')(3).py == 3 and capsys.readouterr().out == '3ₓ\n'
        assert apl.eval(']Display 1 2').output and capsys.readouterr().out == ''
        with pytest.raises(TypeError): f()
        with pytest.raises(TypeError): f(1, 2, 3)
        with pytest.raises(ValueError): apl['x←99'] = 1
        assert apl['x'].py == 9
        r = apl.run('1 ⋄ ⎕←2 ⋄ x←3')
        assert r.value.py == 3 and r.output == ['1', '2'] and capsys.readouterr().out == ''
    with pytest.raises(RuntimeError, match='closed'): apl('1')
    apl.close()


def test_numpy_inputs_and_copies():
    with Session() as apl:
        original = np.arange(12).reshape(3, 4)[:, ::2]
        apl(m=original)
        original[:] = 99
        np.testing.assert_array_equal(apl('+/m'), [2, 10, 18])
        saved = apl['m'].np
        saved[:] = 77
        np.testing.assert_array_equal(apl('+/m'), [2, 10, 18])
        for a in [np.array(True), np.array(3, dtype=np.float32), np.array([1., 2j]), np.array([2**64-1], dtype=np.uint64),
            np.empty((0, 3), dtype=int), np.empty((2, 0)), np.array([np.inf, -np.inf]), np.array([['a', 'b'], ['c', 'd']]), np.array([[1, Fraction(2, 3)]], dtype=object)]:
            result = apl('x', x=a)
            np.testing.assert_array_equal(result, a)
            assert np.shape(result) == a.shape
        assert apl('x', x=np.float32(1.5)).py == 1.5
        assert apl('x', x=np.int64(3)).py == 3
        assert apl('x+1', x=float('inf')).py == float('inf')
        for a in [complex(np.inf, 0), complex(0, np.nan), [float('nan')]]:
            with pytest.raises(ValueError): apl(x=a)
        for a in [np.array([b'a']), np.array(['2020'], dtype='datetime64[Y]'), object()]:
            with pytest.raises(TypeError): apl(x=a)
    np.testing.assert_array_equal(saved, np.full((3, 2), 77))


def test_exact_nested_and_character_values():
    with Session() as apl:
        for value in [42, 2**100, 0.5, Fraction(1, 3), 1+2j, 'hello', 'a', '']:
            result = apl('x', x=value).py
            assert result == value and type(result) is type(value)
        mixed = apl('x', x=[2**100, 0.5, Fraction(1, 3), 1+2j]).np
        assert mixed.dtype == object and [type(o) for o in mixed] == [int, float, Fraction, complex]
        huge = 10**4500
        assert apl('x', x=huge).py == huge and apl('10x*4500x').py == huge
        assert apl('x', x=Fraction(huge, 3)).py == Fraction(huge, 3)
        assert apl('x', x=['a', 'b']).py == 'ab'
        strings = apl('x', x=['ab', 'cd']).np
        assert strings.dtype == object and strings.tolist() == ['ab', 'cd']
        ragged = apl('x', x=[[1, 2], [3]]).np
        assert ragged.shape == (2,) and ragged.dtype == object
        np.testing.assert_array_equal(ragged[0], [1, 2])
        np.testing.assert_array_equal(ragged[1], [3])
        nested = apl('(1 2⋄ 3 4)').np
        assert nested.shape == (2,) and nested.dtype == object
        np.testing.assert_array_equal(apl('x', x=nested).np[1], [3, 4])
        boxed = apl('⊂1 2')
        assert boxed.shape == () and boxed.np.dtype == object and apl('≡x', x=boxed).py == 2
        np.testing.assert_array_equal(boxed.np.item(), [1, 2])
        assert apl("0 3⍴''").shape == (0, 3) and apl('0⍴⊂1 2').np.dtype == object
        assert apl('x', x=[[], []]).shape == (2, 0)
        assert type(apl('1j2×1j¯2').py) is float and apl('+1j2').py == 1-2j
        assert apl('0/1j2').np.dtype == np.float64
        empty = apl('0⍴⊂2 3⍴1x')
        assert apl('x≡0⍴⊂2 3⍴1x', x=empty).py == 1
        assert Array(a := [1, 2]).shape == (2,)
        a.append(a)
        with pytest.raises(ValueError, match='cyclic'): Array(a)
        with pytest.raises(ValueError, match='copy'): np.array(empty, copy=False)


def test_errors_capture_output_and_recover(capsys):
    with Session() as apl:
        for call in [apl, apl.eval]:
            with pytest.raises(AplError) as caught: call('x←7 ⋄ ⎕←1x ⋄ 1÷0')
            e = caught.value
            assert e.kind == 'DOMAIN ERROR' and e.output == ['1ₓ'] and '÷' in e.source and len(e.span) == 2
            assert ' --> <input>:1:' in str(e)
            assert capsys.readouterr().out == ('1ₓ\n' if call is apl else '')
            assert apl('x+1').py == 8
        definition = 'bad←{1÷⍵}'
        apl(definition)
        with pytest.raises(AplError) as caught: apl.fn('bad')(0)
        assert caught.value.source == definition and caught.value.calls[-1]['source']['text'] == 'bad'
        apl.timeout = .01
        with pytest.raises(AplError) as caught: apl.eval('{∇⍵}0')
        assert caught.value.kind == 'TIMEOUT' and capsys.readouterr().out == ''
        assert apl('x').py == 7


def test_interrupts_threads_and_cleanup(capsys):
    with Session() as apl, ThreadPoolExecutor(max_workers=4) as pool:
        apl(x=42)
        assert pool.submit(apl, 'x').result().py == 42
        assert sorted(pool.map(lambda _: apl('x+←1x ⋄ x').py, range(8))) == list(range(43, 51))
        apl(x=42)
        ctrl_c = lambda: os.kill(os.getpid(), signal.SIGINT)
        for call, interrupt, error in [(apl, apl.interrupt, AplError), (apl, ctrl_c, KeyboardInterrupt), (apl.eval, ctrl_c, KeyboardInterrupt)]:
            timer = threading.Timer(.05, interrupt)
            timer.start()
            try:
                with pytest.raises(error) as caught: call('⎕←7 ⋄ {∇⍵}0')
            finally: timer.join()
            assert caught.value.output == ['7'] and capsys.readouterr().out == ('7\n' if call is apl else '')
            assert apl('x').py == 42
    apl = Session()
    native, ref = apl._worker, weakref.ref(apl)
    del apl
    gc.collect()
    assert ref() is None
    with pytest.raises(RuntimeError, match='closed'): native.request(code='1')
    with Session() as apl:
        timer = threading.Timer(.05, apl.close)
        timer.start()
        try:
            with pytest.raises(AplError) as caught: apl('{∇⍵}0')
        finally: timer.join()
        assert caught.value.kind == 'INTERRUPT'


def test_cross_thread_array_lifetime():
    with ThreadPoolExecutor(max_workers=1) as pool:
        def create():
            with Session() as apl: return apl('⊂1r3 2x')
        a = pool.submit(create).result()
        with Session() as apl: assert apl('+/⊃x', x=a).py == Fraction(7, 3)
        held = [a]
        del a
        pool.submit(held.pop).result()


def test_without_numpy():
    code = "from basedpl import Session; import sys; s=Session(); assert s('1+2').py == 3.; s.close(); assert 'numpy' not in sys.modules"
    subprocess.run([sys.executable, '-c', code], check=True, timeout=10)


def test_installed_command(tmp_path):
    for code, status, output in [('2×3+4', 0, '14\n'), ('¯2+1÷0', 1, '')]:
        res = subprocess.run(['bapl', '-e', code], capture_output=True, text=True, timeout=10)
        assert (res.returncode, res.stdout) == (status, output)
        if status: assert 'DOMAIN ERROR' in res.stderr and '<expression>:1:5' in res.stderr
        else: assert not res.stderr
    path = tmp_path/'lesson.apl'
    path.write_text('v←⍳10\nsum←+/\nsum v\n', encoding='utf-8')
    res = subprocess.run(['bapl', str(path)], capture_output=True, text=True, timeout=10)
    assert (res.returncode, res.stdout, res.stderr) == (0, '55\n', '')


def test_installed_json_command():
    codes = ['v←9007199254740993x 0.5 1r3', 'v', '0/1r3', '1r0', '1r3+1r6', '2x*100x', '1j2 3j4']
    res = subprocess.run(['bapl', '--json'], input='\n'.join(json.dumps(c) for c in codes)+'\n', capture_output=True, text=True, timeout=10)
    assert res.returncode == 0 and not res.stderr
    replies = [json.loads(line) for line in res.stdout.splitlines()]
    assert replies[1]['value'] == dict(shape=[3], data=[9007199254740993, 0.5, {'rational': ['1', '3']}], prototype=0)
    assert replies[2]['value'] == dict(shape=[0], data=[], prototype=0)
    assert replies[3]['error']['kind'] == 'DOMAIN ERROR'
    assert replies[4]['value'] == {'rational': ['1', '2']}
    assert replies[5]['value'] == 2**100
    assert replies[6]['value']['data'] == [{'complex': [1, 2]}, {'complex': [3, 4]}]
