import json, os, signal, threading
import pytest
from miniapl import Session, AplError
from miniapl._core import _check_reference
from miniapl.worker import Worker

def test_worker_bindings_calls_and_echo():
    with Worker() as w:
        a = dict(shape=[3], data=[1, 2, 3], prototype=0)
        assert w.request(dict(bindings=dict(x=a)), timeout=2) == dict(value=None, output=[], error=None)
        r = w.request(dict(code='1 ⋄ ⎕←+/x ⋄ x+1x', echo=False), timeout=2)
        assert r['output'] == ['6x'] and r['value']['data'] == [2, 3, 4] and r['error'] is None
        r = w.request(dict(call='-', args=[a, a], echo=False), timeout=2)
        assert r['output'] == [] and r['value']['data'] == [0, 0, 0]
        for code in ["(2x*100x)0.5 1r3 1j2 'a'", "(1 2)'ab'(0 3⍴0x)", '0⍴⊂1 2', "0 2⍴''"]:
            original = w.eval(code, timeout=2)
            r = w.request(dict(bindings=dict(v=original['value']), call='⊢', args=[original['value']]), timeout=2)
            assert r == original
            assert w.eval('v', timeout=2) == original
        for payload in [
            dict(bindings={'x←99': a}), dict(bindings=dict(x=dict(shape=[1], data=[1, 2], prototype=0))),
            dict(bindings=dict(x=dict(shape=[], data=[{'rational':['1', '0']}], prototype=0))),
            dict(bindings=dict(x=dict(shape=[], data=['ab'], prototype=' '))), dict(bindings=[]),
            dict(call='+', args=[dict(shape=[], data=[True], prototype=0)]), dict(call='+'), dict(args=[a]),
            dict(code='1', call='+', args=[a]),
        ]:
            assert w.request(payload, timeout=2)['error']['kind'] == 'REQUEST ERROR'
        r = w.request(dict(call='{⎕←⍵ ⋄ 1÷0}', args=[a], echo=False), timeout=2)
        assert r['output'] == ['1x 2x 3x'] and r['error']['kind'] == 'DOMAIN ERROR'
        assert r['error']['calls'][-1]['source']['text'] == '{⎕←⍵ ⋄ 1÷0}'
        assert w.request(dict(call='{∇⍵}', args=[a]), timeout=.01)['error']['kind'] == 'TIMEOUT'
        assert w.eval('x', timeout=2)['value'] == a

def test_worker_cancellation_and_reference_sessions():
    case = dict(code='a←3', expected=dict(shape=[], data=[3], prototype=0))
    assert json.loads(_check_reference(json.dumps(case), 1))['status'] == 'pass'
    with Worker() as w:
        assert w.eval('keep←42')['value']['data'] == [42]
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
        assert w.eval('keep+1')['value']['data'] == [43]
        assert w.request(dict(case=case), timeout=1)['status'] == 'pass'
        assert w.request(dict(case=dict(code='a', expected_error='VALUE ERROR')), timeout=1)['status'] == 'pass'
        assert w.request(dict(case=dict(code='1', expected=dict(shape=[1], data=[1], prototype=0))))['message'].startswith('shape:')
        assert w.request(dict(case=dict(code='1', expected={})))['status'] == 'invalid'
    with Session(timeout=.01) as s:
        with pytest.raises(AplError) as e: s.eval('x←7 ⋄ {∇⍵}0')
        assert e.value.kind == 'TIMEOUT'
        assert s('x') == 7
