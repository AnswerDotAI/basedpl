import subprocess
import json
import sys
import pytest
from fractions import Fraction
from concurrent.futures import ThreadPoolExecutor
from threading import Thread
from miniapl import Session, AplError

def test_session_results_and_recovery():
    with Session() as s:
        r = s.eval('v←⍳5')
        assert r.value.shape == (5,) and r.value.to_python() == [1, 2, 3, 4, 5] and r.output == []
        saved = r.value
        r = s.eval('+/v')
        assert r.value.shape == () and r.value.to_python() == 15 and r.output == ['15']
        assert s.eval('3 ⋄ f←+').value is None
        assert s.eval('').value is None
        empty = s.eval('⍳0').value
        assert empty.shape == (0,) and empty.to_python() == [] and empty.prototype == 0
        with pytest.raises(AplError) as caught: s.eval('⎕←7 ⋄ 1÷0')
        assert caught.value.kind == 'DOMAIN ERROR' and caught.value.output == ['7']
        assert '÷' in caught.value.source and len(caught.value.span) == 2
        assert s.eval('2+2').value.to_python() == 4
    with pytest.raises(RuntimeError, match='closed'): s.eval('1')
    del s
    assert saved.to_python() == [1, 2, 3, 4, 5]

def test_session_thread_affinity_and_copied_arrays():
    with Session() as s, ThreadPoolExecutor(max_workers=1) as pool:
        saved = s.eval('⍳3').value
        with pytest.raises(RuntimeError, match='creating thread'): pool.submit(s.eval, '1').result()
        with pytest.raises(RuntimeError, match='creating thread'): pool.submit(s.close).result()
        assert pool.submit(saved.to_python).result() == [1, 2, 3]
        assert s.eval('2').value.to_python() == 2

def test_unsupported_wrong_thread_destruction(monkeypatch):
    # PyO3 deliberately skips native destruction here; keep the documented restriction visible.
    errors, held = [], [Session()]
    monkeypatch.setattr(sys, 'unraisablehook', errors.append)
    worker = Thread(target=lambda: held.pop())
    worker.start()
    worker.join()
    assert len(errors) == 1 and isinstance(errors[0].exc_value, RuntimeError)
    assert 'unsendable' in str(errors[0].exc_value)

def test_installed_command():
    for code, status, output in [('2×3+4', 0, '14\n'), ('¯2+1÷0', 1, '')]:
        res = subprocess.run(['miniapl', '-e', code], capture_output=True, text=True, timeout=10)
        assert (res.returncode, res.stdout) == (status, output)
        if status: assert 'DOMAIN ERROR' in res.stderr and '<expression>:1:5' in res.stderr
        else: assert not res.stderr
    res = subprocess.run(['miniapl'], input='1÷0\n2+2\n', capture_output=True, text=True, timeout=10)
    assert (res.returncode, res.stdout) == (1, '4\n')
    assert 'DOMAIN ERROR' in res.stderr

def test_installed_json_command():
    requests = '\n'.join(json.dumps(c) for c in ['v←⍳10', '+/v', '1÷0', '2+2']) + '\n'
    res = subprocess.run(['miniapl', '--json'], input=requests, capture_output=True, text=True, timeout=10)
    assert res.returncode == 0 and not res.stderr
    replies = [json.loads(line) for line in res.stdout.splitlines()]
    assert replies[1]['value'] == {'shape': [], 'data': [55], 'prototype': 0}
    assert replies[2]['error']['kind'] == 'DOMAIN ERROR'
    assert replies[3]['value']['data'] == [4]

def test_retained_definitions_and_source():
    with Session() as s:
        s.eval('add←{⍺+⍵} ⋄ avg←+/÷≢')
        assert s.eval('add/1 2 3').value.to_python() == 6
        assert s.eval('avg 2 4 9').value.to_python() == 5
        definition = 'bad←{1÷⍵}'
        s.eval(definition)
        with pytest.raises(AplError) as caught: s.eval('bad 0')
        assert caught.value.source == definition
        assert caught.value.calls == [{'source_name': '<input>', 'source': 'bad 0', 'span': (0, 3)}]
        assert s.eval('2+2').value.to_python() == 4

def test_source_file(tmp_path):
    path = tmp_path/'lesson.apl'
    path.write_text('v←⍳10\nsum←+/\nsum v\n', encoding='utf-8')
    res = subprocess.run(['miniapl', str(path)], capture_output=True, text=True, timeout=10)
    assert (res.returncode, res.stdout, res.stderr) == (0, '55\n', '')

def test_exact_values_and_recovery():
    with Session() as s:
        a = s.eval('9007199254740993x 0.5 1r3').value
        assert a.to_python() == [9007199254740993, 0.5, Fraction(1, 3)]
        assert [type(o) for o in a.data] == [int, float, Fraction] and type(a.prototype) is int
        r = s.eval('1r3+1r6')
        assert r.output == ['1r2'] and r.value.to_python() == Fraction(1, 2)
        assert type(s.eval('1x÷2').value.to_python()) is float
        assert s.eval('6x÷3x').output == ['2x']
        assert s.eval('(1x 1r3)(0.5 2x)').value.to_python() == [[1, Fraction(1, 3)], [0.5, 2]]
        empty = s.eval('0/1r3').value
        assert empty.shape == (0,) and type(empty.prototype) is int
        with pytest.raises(AplError) as caught: s.eval('⎕←1x ⋄ 1x÷0x')
        assert caught.value.output == ['1x'] and caught.value.kind == 'DOMAIN ERROR'
        assert s.eval('0.3=0.1+0.2').value.to_python() == 1
        # Native big-int transfer must not depend on Python's decimal-string conversion limit.
        huge = s.eval('1' + '0'*4500 + 'x').value
    assert huge.to_python() == 10**4500
    assert a.to_python() == [9007199254740993, 0.5, Fraction(1, 3)]

def test_exact_installed_command_and_json():
    res = subprocess.run(['miniapl', '-e', '1x÷3x ⋄ 6x÷3x ⋄ 1x÷3'], capture_output=True, text=True, timeout=10)
    assert (res.returncode, res.stdout, res.stderr) == (0, '1r3\n2x\n0.3333333333333333\n', '')
    requests = '\n'.join(json.dumps(c) for c in ['v←9007199254740993x 0.5 1r3', 'v', '0/1r3', '1r0', '1r3+1r6']) + '\n'
    res = subprocess.run(['miniapl', '--json'], input=requests, capture_output=True, text=True, timeout=10)
    assert res.returncode == 0 and not res.stderr
    replies = [json.loads(line) for line in res.stdout.splitlines()]
    assert replies[1]['value'] == {'shape': [3], 'data': [{'rational': ['9007199254740993', '1']}, 0.5, {'rational': ['1', '3']}],
                                  'prototype': {'rational': ['0', '1']}}
    assert replies[2]['value'] == {'shape': [0], 'data': [], 'prototype': {'rational': ['0', '1']}}
    assert replies[3]['error']['kind'] == 'DOMAIN ERROR'
    assert replies[4]['value']['data'] == [{'rational': ['1', '2']}]
