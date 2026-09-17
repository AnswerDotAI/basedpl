import json, os, signal, threading
import pytest
from miniapl import Session, AplError
from miniapl._core import _check_reference
from miniapl.worker import Worker

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
    with Session() as s:
        with pytest.raises(AplError) as e: s.eval('x←7 ⋄ {∇⍵}0', timeout=.01)
        assert e.value.kind == 'TIMEOUT'
        assert s.eval('x').value.to_python() == 7
