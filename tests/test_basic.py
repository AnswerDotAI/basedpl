import subprocess, json, sys, os, signal, threading
from fractions import Fraction
from concurrent.futures import ThreadPoolExecutor
import numpy as np, pytest
from basedpl import bpl, BplError, Array


def test_native_calls_and_explicit_output():
    a = bpl('1r3 2ₓ')
    bpl['x'] = a
    r = bpl('⎕←x ⋄ x', 'explicit')
    np.testing.assert_array_equal(r.value, a)
    assert r.output == ['1ᵣ3 2ₓ']
    assert bpl('1 ⋄ ⎕←2 ⋄ 3').py == 3
    assert bpl('1 ⋄ ⎕←2 ⋄ 3', 'explicit').output == ['2']
    assert bpl('+/x', x=a).py == Fraction(7, 3)


def test_clear_restores_starting_workspace():
    bpl('•prefs ["box":$t]')
    bpl(x=1)
    bpl(']clear')
    with pytest.raises(BplError, match='VALUE'): bpl('x')
    assert bpl('1 2', 'repl').output == ['1 2']
    with pytest.raises(BplError, match='SYNTAX'): bpl(']clear x')


def test_bindings_functions_and_output(capsys):
    assert bpl(x=np.arange(1, 6), source=9, timeout=8) is None
    assert bpl('source+timeout').py == 17
    assert bpl('+/x').py == 15 and type(bpl('+/x')) is Array
    mean = bpl.fn('{(+/⍵)÷≢⍵}')
    assert mean([1, 2, 3]).py == 2 and type(mean([1, 2, 3]).py) is int
    assert mean([1, 2, 4]).py == Fraction(7, 3)
    added = bpl.fn('+')([1, 2, 3], 10)
    np.testing.assert_array_equal(added, [11, 12, 13])
    assert added.np.dtype == np.uint8
    f = bpl.fn('foo')
    bpl('foo←+')
    assert f(3).py == 3
    bpl('foo←-')
    assert f(3).py == -3
    bpl['x'] = [[1, 2], [3, 4]]
    np.testing.assert_array_equal(bpl['x'], [[1, 2], [3, 4]])
    assert bpl('x←7').py == 7 and bpl('3 ⋄ f←+') is None and bpl('') is None
    assert bpl('silent←{a←7} ⋄ silent 0').py == 7 and bpl('{}0') is None
    assert capsys.readouterr().out == ''
    assert bpl('1 ⋄ ⎕←2 ⋄ ⍎"3 ⋄ ⎕←4 ⋄ 5"').py == 5
    assert capsys.readouterr().out == '2\n4\n'
    r = bpl('⎕←x ⋄ x+1ₓ', 'explicit', x=9)
    assert r.value.py == 10 and r.output == ['9ₓ'] and capsys.readouterr().out == ''
    assert bpl.fn('{⎕←⍵}')(3).py == 3 and capsys.readouterr().out == '3ₓ\n'
    assert bpl(']Display 1 2', 'explicit').output and capsys.readouterr().out == ''
    with pytest.raises(TypeError): f()
    with pytest.raises(TypeError): f(1, 2, 3)
    with pytest.raises(ValueError): bpl['x←99'] = 1
    assert bpl['x'].py == 9
    r = bpl('1 ⋄ ⎕←2 ⋄ x←3', 'repl')
    assert r.value.py == 3 and r.output == ['1', '2'] and capsys.readouterr().out == ''


def test_numpy_inputs_and_copies():
    original = np.arange(12).reshape(3, 4)[:, ::2]
    bpl(m=original)
    original[:] = 99
    np.testing.assert_array_equal(bpl('+/m'), [2, 10, 18])
    saved = bpl['m'].np
    saved[:] = 77
    np.testing.assert_array_equal(bpl('+/m'), [2, 10, 18])
    for a in [np.array(True), np.array(3, dtype=np.float32), np.array([1., 2j]), np.array([2**64-1], dtype=np.uint64).astype(object),
        np.empty((0, 3), dtype=int), np.empty((2, 0)), np.array([np.inf, -np.inf]), np.array([['a', 'b'], ['c', 'd']]), np.array([[1, Fraction(2, 3)]], dtype=object),
        np.arange(6, dtype=np.int8).reshape(2, 3).T, np.array([0.5, -0.], dtype=np.float32), np.array([2**63-1], dtype=np.uint64), np.array([1., np.nan])]:
        result = bpl('x', x=a)
        np.testing.assert_array_equal(result, a)
        assert np.shape(result) == a.shape
    assert bpl('x', x=np.arange(3, dtype=np.int8)).np.dtype == np.uint8 and bpl('x', x=np.ones(2, np.float32)).np.dtype == np.float64
    assert bpl('x', x=np.float32(1.5)).py == 1.5
    assert bpl('x', x=np.int64(3)).py == 3
    assert bpl('x+1', x=float('inf')).py == float('inf')
    with pytest.raises(ValueError): bpl(x=np.array([2**64-1], dtype=np.uint64))
    for a in [np.array([b'a']), np.array(['2020'], dtype='datetime64[Y]'), object()]:
        with pytest.raises(TypeError): bpl(x=a)
    np.testing.assert_array_equal(saved, np.full((3, 2), 77))


def test_exact_nested_and_character_values():
    for value in [42, 2**100, 0.5, Fraction(1, 3), 1+2j, 'hello', 'a', '']:
        result = bpl('x', x=value).py
        assert result == value and type(result) is type(value)
    mixed = bpl('x', x=[2**100, 0.5, Fraction(1, 3), 1+2j]).np
    assert mixed.dtype == object and [type(o) for o in mixed] == [int, float, Fraction, complex]
    huge = 10**4500
    assert bpl('x', x=huge).py == huge and bpl('10ₓ*4500ₓ').py == huge
    assert bpl('x', x=Fraction(huge, 3)).py == Fraction(huge, 3)
    assert bpl('x', x=['a', 'b']).py == 'ab'
    strings = bpl('x', x=['ab', 'cd']).np
    assert strings.dtype == object and strings.tolist() == ['ab', 'cd']
    ragged = bpl('x', x=[[1, 2], [3]]).np
    assert ragged.shape == (2,) and ragged.dtype == object
    np.testing.assert_array_equal(ragged[0], [1, 2])
    np.testing.assert_array_equal(ragged[1], [3])
    nested = bpl('[[1 2] [3 4]]').np
    assert nested.shape == (2,) and nested.dtype == object
    np.testing.assert_array_equal(bpl('x', x=nested).np[1], [3, 4])
    boxed = bpl('⊂1 2')
    assert boxed.shape == () and boxed.np.dtype == object and bpl('≡x', x=boxed).py == 2
    np.testing.assert_array_equal(boxed.np.item(), [1, 2])
    assert bpl('0 3⍴""').shape == (0, 3) and bpl('0⍴⊂1 2').np.dtype == object
    assert bpl('x', x=[[], []]).shape == (2, 0)
    assert type(bpl('1j2×1j¯2').py) is float and bpl('+1j2').py == 1-2j
    assert bpl('0#1j2').np.dtype == np.float64
    empty = bpl('0⍴⊂2 3⍴1ₓ')
    assert bpl('x≡0⍴⊂2 3⍴1ₓ', x=empty).py == 1
    assert Array(a := [1, 2]).shape == (2,)
    a.append(a)
    with pytest.raises(ValueError, match='cyclic'): Array(a)
    with pytest.raises(ValueError, match='copy'): np.array(empty, copy=False)


def test_errors_capture_output_and_recover(capsys):
    for call in [bpl, lambda src: bpl(src, 'explicit')]:
        with pytest.raises(BplError) as caught: call('x←7 ⋄ ⎕←1ₓ ⋄ 1÷"a"')
        e = caught.value
        assert e.kind == 'DOMAIN' and e.output == ['1ₓ'] and e.source[slice(*e.span)] == '÷'
        assert ' --> <input>:1:' in str(e)
        assert capsys.readouterr().out == ('1ₓ\n' if call is bpl else '')
        assert bpl('x+1').py == 8
    definition = 'bad←{1÷⍵}'
    bpl(definition)
    with pytest.raises(BplError) as caught: bpl.fn('bad')('a')
    assert caught.value.source == definition and caught.value.calls[-1]['source']['text'] == 'bad'
    bpl.timeout = .01
    with pytest.raises(BplError) as caught: bpl('{∇⍵}0', 'explicit')
    assert caught.value.kind == 'TIMEOUT' and capsys.readouterr().out == ''
    assert bpl('x').py == 7


def test_interrupts_and_threads(capsys):
    with ThreadPoolExecutor(max_workers=4) as pool:
        bpl(x=42)
        assert pool.submit(bpl, 'x').result().py == 42
        assert sorted(pool.map(lambda _: bpl('x+←1ₓ ⋄ x').py, range(8))) == list(range(43, 51))
        bpl(x=42)
        ctrl_c = lambda: os.kill(os.getpid(), signal.SIGINT)
        for call, interrupt, error in [(bpl, bpl.interrupt, BplError), (bpl, ctrl_c, KeyboardInterrupt), (lambda src: bpl(src, 'explicit'), ctrl_c, KeyboardInterrupt)]:
            timer = threading.Timer(.05, interrupt)
            timer.start()
            try:
                with pytest.raises(error) as caught: call('⎕←7 ⋄ {∇⍵}0')
            finally: timer.join()
            assert caught.value.output == ['7'] and capsys.readouterr().out == ('7\n' if call is bpl else '')
            assert bpl('x').py == 42


def test_cross_thread_array_lifetime():
    with ThreadPoolExecutor(max_workers=1) as pool:
        def create():
            return bpl('⊂1r3 2ₓ')
        a = pool.submit(create).result()
        assert bpl('+/⊃x', x=a).py == Fraction(7, 3)
        held = [a]
        del a
        pool.submit(held.pop).result()


def test_without_numpy():
    code = "from basedpl import bpl; import sys; assert bpl('1+2').py == 3.; assert 'numpy' not in sys.modules"
    subprocess.run([sys.executable, '-c', code], check=True, timeout=10)


def test_installed_command(tmp_path):
    for code, status, output in [('2×3+4', 0, '14\n'), ('¯2+1÷"a"', 1, '')]:
        res = subprocess.run(['bpl', '-e', code], capture_output=True, text=True, timeout=10)
        assert (res.returncode, res.stdout) == (status, output)
        if status: assert 'DOMAIN ERROR' in res.stderr and '<expression>:1:5' in res.stderr
        else: assert not res.stderr
    path = tmp_path/'lesson.bpl'
    path.write_text('#!/usr/bin/env bpl\nv←⍳10\nsum←+/\nsum v\n', encoding='utf-8')
    res = subprocess.run(['bpl', str(path)], capture_output=True, text=True, timeout=10)
    assert (res.returncode, res.stdout, res.stderr) == (0, '45\n', '')


def test_installed_worker_command():
    codes = ['v←9007199254740993ₓ 0.5 1r3', 'v', '0#1r3', '1r0', '1r3+1r6', '2ₓ*100ₓ', '1j2 3j4']
    res = subprocess.run(['bpl', '--worker'], input=''.join(json.dumps(dict(code=c))+'\n' for c in codes), capture_output=True, text=True, timeout=10)
    assert res.returncode == 0 and not res.stderr
    replies = [json.loads(line)['result'] for line in res.stdout.splitlines()]
    assert replies[1]['value'] == dict(shape=[3], data=[9007199254740993, 0.5, {'rational': ['1', '3']}], prototype=0)
    assert replies[2]['value'] == dict(shape=[0], data=[], prototype=0)
    assert replies[3]['error']['kind'] == 'DOMAIN'
    assert replies[4]['value'] == {'rational': ['1', '2']}
    assert replies[5]['value'] == 2**100
    assert replies[6]['value']['data'] == [{'complex': [1, 2]}, {'complex': [3, 4]}]
