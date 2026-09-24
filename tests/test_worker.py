import json, os, signal, threading
import pytest
from basedpl import apl, AplError
from basedpl._core import _check_reference
from basedpl.worker import Worker

def test_worker_bindings_calls_and_echo():
    with Worker() as w:
        a = dict(shape=[3], data=[1, 2, 3], prototype=0)
        assert w.request(dict(bindings=dict(x=a)), timeout=2) == dict(value=None, output=[], error=None)
        r = w.request(dict(code='1 ⋄ ⎕←+/x ⋄ x+1x', echo=False), timeout=2)
        assert r['output'] == [dict(kind='explicit', data={'text/plain':'6ₓ'})] and r['value']['data'] == [2, 3, 4] and r['error'] is None
        r = w.request(dict(call='-', args=[a, a], echo=False), timeout=2)
        assert r['output'] == [] and r['value']['data'] == [0, 0, 0]
        for code in ['3x', '⊂3x', '⊂⊂3x', "(2x*100x)0.5 1r3 1j2 'a'", "(1 2)'ab'(0 3⍴0x)", '0⍴⊂1 2', "0 2⍴''", '∞ ¯∞']:
            original = w.eval(code, timeout=2)
            r = w.request(dict(bindings=dict(v=original['value']), call='⊢', args=[original['value']]), timeout=2)
            assert r == original
            assert w.eval('v', timeout=2) == original
        assert w.eval('∞ ¯∞')['value']['data'] == [{'infinity': 1}, {'infinity': -1}]
        for payload in [dict(bindings={'x←99': a}), dict(bindings=dict(x=dict(shape=[1], data=[1, 2], prototype=0))),
            dict(bindings=dict(x=dict(shape=[], data=[{'rational':['1', '0']}], prototype=0))),
            dict(bindings=dict(x=dict(shape=[], data=['ab'], prototype=' '))), dict(bindings=[]),
            dict(call='+', args=[dict(shape=[], data=[True], prototype=0)]), dict(call='+'), dict(args=[a]),
            dict(code='1', call='+', args=[a]),
            dict(bindings=dict(x=dict(shape=[], data=[{'infinity': 0}], prototype=0))),
        ]: assert w.request(payload, timeout=2)['error']['kind'] == 'REQUEST ERROR'
        r = w.request(dict(call='{⎕←⍵ ⋄ 1÷0}', args=[a], echo=False), timeout=2)
        assert r['output'] == [dict(kind='explicit', data={'text/plain':'1ₓ 2ₓ 3ₓ'})] and r['error']['kind'] == 'DOMAIN ERROR'
        assert r['error']['calls'][-1]['source']['text'] == '{⎕←⍵ ⋄ 1÷0}'
        assert w.request(dict(call='{∇⍵}', args=[a]), timeout=.01)['error']['kind'] == 'TIMEOUT'
        for value in [float('inf'), float('-inf'), float('nan')]:
            with pytest.raises(ValueError): w.request(dict(bindings=dict(x=dict(shape=[], data=[value], prototype=0))))
        assert w.eval('x', timeout=2)['value'] == a

def test_worker_keyed_arrays():
    with Worker() as w:
        v = w.eval("('b':1 2),('a':'x':3)", timeout=2)['value']
        assert v['shape'] == [2] and v['axis_keys'] == [['b', 'a']]
        assert v['data'][1] == dict(shape=[1], data=[3], prototype=0, axis_keys=[['x']])
        m = w.eval("('north' 'south' ⋄ 'west' 'east'):2 2⍴⍳4", timeout=2)['value']
        assert m['shape'] == [2, 2] and m['axis_keys'] == [['north', 'south'], ['west', 'east']]
        empty = w.eval('⍬:⍬', timeout=2)['value']
        assert empty['shape'] == [0] and empty['axis_keys'] == [[]]
        m['axis_names'] = ['row', 'col']
        for value in (v, m, empty):
            assert w.request(dict(bindings=dict(k=value), call='⊢', args=[value]), timeout=2)['value'] == value
            assert w.eval('k', timeout=2)['value'] == value
        assert w.request(dict(bindings=dict(m=m), code="+/['col']m"), timeout=2)['value']['axis_names'] == ['row']
        for keys in ([], [None, None], [['a']], [['a', 'a']], [[1, 2]], ['ab']):
            bad = dict(shape=[2], data=[1, 2], prototype=0, axis_keys=keys)
            assert w.request(dict(bindings=dict(k=bad)), timeout=2)['error']['kind'] == 'REQUEST ERROR'

def test_worker_cancellation_and_reference_sessions():
    case = dict(code='a←3', expected_code='3')
    assert json.loads(_check_reference(json.dumps(case), 1))['status'] == 'pass'
    with Worker() as w:
        assert w.eval('keep←42')['value'] == 42
        result = w.eval('(+⍣{0})1', timeout=.01)
        assert result['error']['kind'] == 'TIMEOUT'
        timer = threading.Timer(.05, w.interrupt)
        timer.start()
        try: result = w.eval('{∇⍵}0', timeout=2)
        finally: timer.join()
        assert result['error']['kind'] == 'INTERRUPT'
        timer = threading.Timer(.05, lambda: os.kill(os.getpid(), signal.SIGINT))
        timer.start()
        try:
            with pytest.raises(KeyboardInterrupt): w.eval('{∇⍵}0', timeout=2)
        finally: timer.join()
        assert w.eval('keep+1')['value'] == 43
        assert w.request(dict(case=case), timeout=1)['status'] == 'pass'
        assert w.request(dict(case=dict(code='a', expected_error='VALUE ERROR')), timeout=1)['status'] == 'pass'
        assert w.request(dict(case=dict(code='1', expected=dict(shape=[1], data=[1], prototype=0))))['message'] == 'atom versus array'
        assert w.request(dict(case=dict(code='1', expected={})))['status'] == 'invalid'
        case = dict(code='∞', expected_code='∞', relative_tolerance=1e-14)
        assert w.request(dict(case=case))['status'] == 'pass'
        assert w.request(dict(case=dict(case, code='1e308')))['status'] == 'mismatch'
    apl.timeout = .01
    with pytest.raises(AplError) as e: apl('x←7 ⋄ {∇⍵}0', 'explicit')
    assert e.value.kind == 'TIMEOUT'
    assert apl('x').py == 7
