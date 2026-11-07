import json
from pathlib import Path
import pytest
from basedpl.reference import Corpus
from basedpl.bpltests import Case, parse, render, convert, add, description, literal, set_budgets, set_counts, check_file, SpeedCosts, _speed_batches
from basedpl.worker import Worker

ROOT = Path(__file__).resolve().parents[1]


def test_cost_records_keep_values_and_route_reports():
    row = dict(section='copies', label='v', cost=1.234567, seconds=1.234567e-9)
    violation = 'required route fusion was not taken'
    costs = SpeedCosts.from_records([row, dict(row, label='v+w', budget=2, obligations=[violation])])
    assert costs[0][:4] == ('copies', 'v', 1.234567, None) and costs[0][4] == pytest.approx(1.234567)
    assert costs[1][3] == 2 and costs.holds == {} and costs.footprints == {}
    assert costs.obligations == {('copies', 'v+w'): [violation]}
    assert f'route obligation [copies] v+w: {violation}' in repr(costs)



def test_speed_batches_keep_files_sections_and_all_rows():
    text = '⍝⍝ copies\n-v\n⍝⍝: 1\n\n⍝⍝ static\n-v\n⍝⍝: 2\n\n⌽v\n⍝⍝: 3\n\n⍝ —\n1+1   ⍝ 2\n'
    text += '\n+/v\n⍝⍝: - - -\tinstructions macos 10 linux 10\n'
    files = [dict(name=name, text=text) for name in ('one.bpl', 'two.bpl')]
    batches = list(_speed_batches(files, size=1))
    assert [b['file'] for b in batches] == ['one.bpl', 'one.bpl', 'two.bpl', 'two.bpl']
    for b in batches:
        assert b['codes'] == (['-v'] if len(b['rows']) == 2 else ['⌽v'])
        assert b['rows'] == [[b['file'], section, b['codes'][0]] for section in
            (['copies', 'static'] if len(b['rows']) == 2 else ['static'])]
    files = [dict(name=p.name, text=p.read_text()) for p in sorted((ROOT/'tests/speed').glob('*.bpl'))]
    batches = list(_speed_batches(files))
    expected = sorted((f['name'], c.section, c.code) for f in files for c in parse(f['text']) if any(b != '-' for b in c.budgets))
    assert sorted(tuple(r) for b in batches for r in b['rows']) == expected
    assert all(len(b['codes']) <= 16 and len(b['codes']) == len(set(b['codes'])) for b in batches)



def test_records_and_boundaries():
    row = dict(id='curated:1', code='1+2', expected_code='3')
    assert convert(row).expect == '3'
    cases = [Case('1+2', '3', 'ngn:1', 'addition'),
        Case("f←{\n\n⍵+1\n}\nf 2\n", '(\n3\n)', comment='multiline', rtol=1e-14, atol=1e-15),
        Case('"unfinished', '⍝ error: SYNTAX ERROR'), Case('', '⍝ error: SYNTAX ERROR')]
    text = render(cases)
    for ending in ['', '\n', '\n\n']: assert parse(text.rstrip('\n')+ending) == cases
    assert text.count('\n⍝ =>\n') == 1
    assert parse(text)[1].line == 4
    assert '1+2   ⍝ 3' in text
    quoted = Case('"a""⍝b" \'⍝\'', '"a""⍝b" \'⍝\'')
    assert parse(render([quoted])) == [quoted]
    assert parse('⍝ —\n'+'1'*40+' ⍝ 1') == [Case('1'*40, '1')]
    assert '1'*63+'   ⍝ 1' in render([Case('1'*63, '1')])
    assert '\n'+'1'*64+'\n1\n' in render([Case('1'*64, '1')])
    commented = Case('1 ⍝ note', '1')
    assert parse(render([commented])) == [commented]
    for bad in ['⍝  —\n1\n2\n3\n\n', '⍝  —\n1\n⍝ =>\n2\n⍝ =>\n3\n\n', '⍝ —\n1\n2\n⍝ —\n3\n3']:
        with pytest.raises(ValueError): parse(bad)
    with pytest.raises(ValueError, match='duplicate'): parse(render([cases[0], cases[0]]))
    with pytest.raises(ValueError, match='use one'): render([Case('1\n⍝ =>\n2', '2')])
    with pytest.raises(ValueError, match='tolerance'): parse('⍝  — [rtol=-1]\n1\n1\n\n')
    cases = [Case('⎕←9 ⋄ ⎕←2 ⋄ 7', '7', section='Agenda', output='9\n2'),
        Case('1', '1', section='Silence', output=''), Case("⎕←'\\'", "'\\'", section='Silence', output='\\')]
    for ending in ['', '\n', '\n\n']: assert parse(render(cases).rstrip('\n')+ending) == cases
    multiline = Case('{\n⍵\n}1', '1')
    assert parse('⍝ —\n{\n⍵\n}1\n⍝ =>\n1') == parse('⍝ —\n{\n⍵\n}1   ⍝ 1') == [multiline]
    assert '}1   ⍝ 1' in render([multiline])
    assert r'⍝ ⎕: 9\n2' in render(cases)
    with pytest.raises(ValueError, match='escapes'): parse('⍝ —\n1\n1\n⍝ ⎕: \\t\n\n')


def test_comment_extraction():
    corpus = Corpus(ROOT/'provenance')
    assert 'sin(pi/6)' in description(corpus['ngn:177', '*'])
    assert '0 <= x < n' not in description(corpus['ngn:518', '*'])
    comment = description(corpus['april/libraries/dfns/array/demo.lisp:14', '*'])
    assert 'self-contained' not in comment and 'original independent expectation' not in comment


def test_captured_literals():
    with Worker() as w:
        for line in (ROOT/'provenance/ngn.jsonl').read_text().splitlines():
            expected = json.loads(line).get('expected')
            if isinstance(expected, dict): assert w.request(dict(case=dict(code=literal(expected), expected=expected)), 2)['status']=='pass', literal(expected)


def test_incremental_export(tmp_path):
    inventory = tmp_path/'inventory'
    inventory.mkdir()
    rows = Corpus(ROOT/'provenance').get_many(['ngn:177', 'ngn:517', 'ngn:518'], '*')
    rows['ngn:177']['code'] = '1e¯10>|.5-1○π÷6'
    rows['ngn:518']['code'] = 'n←100⋄A←(n÷2)¿n⋄∧/(0≤A),A<n'
    for row in rows.values(): row['status'] = 'pending'
    (inventory/'ngn.jsonl').write_text(''.join(json.dumps(row)+'\n' for row in rows.values()))
    assert add(['ngn:177', 'ngn:517'], tmp_path, inventory) == ['ngn:177', 'ngn:517']
    path = tmp_path/'ngn.bpl'
    path.write_text(path.read_text().rstrip('\n'))
    assert add(['ngn:518'], tmp_path, inventory) == ['ngn:518']
    assert [case.id for case in parse((tmp_path/'ngn.bpl').read_text())] == ['ngn:177', 'ngn:517', 'ngn:518']
    saved = (tmp_path/'ngn.bpl').read_bytes()
    with pytest.raises(ValueError, match='duplicate'): add(['ngn:518'], tmp_path, inventory)
    assert (tmp_path/'ngn.bpl').read_bytes() == saved
    assert all(row['status']=='active' for row in Corpus(inventory).get_many(rows, '*').values())


def test_set_budgets(tmp_path):
    path = tmp_path/'speed.bpl'
    path.write_text('⍝⍝ setup\nn←4\n⍝⍝ copies\n⍝ Negation\n-v\n⍝⍝: 2.8\t2.8\t3.4\n\n⌽v\n⍝⍝: 3\t3\t4.8\n\n⍝⍝ widths •prefs ["float":32]\n-v\n⍝⍝: 0.83\t0.83\t0.79\n')
    changes = set_budgets({'-v': 1.37, '⌽v': 0.1}, path=path, sections='copies')
    assert '⍝ Negation\n-v\n⍝⍝: 2.8\t2.8\t2.8\n\n⌽v\n⍝⍝: 0.5\t0.5\t0.5\n' in path.read_text()
    assert len(changes) == 4
    set_budgets({'-v': 0.5}, path=path, builds='linux', sections='widths')
    assert '⍝⍝ widths •prefs ["float":32]\n-v\n⍝⍝: 0.83\t0.75\t0.79\n' in path.read_text()
    rm = tmp_path/'rustymath.txt'
    rm.write_text('[float64]\nneg\t1\t1\n')
    set_budgets({'neg': 1.2}, path=rm)
    assert rm.read_text() == '[float64]\nneg\t2.4\t2.4\n'
    with pytest.raises(ValueError): set_budgets({'neg': 1}, path=rm, builds='wasm')


def test_set_counts(tmp_path):
    path = tmp_path/'speed.bpl'
    path.write_text('⍝⍝ setup\nn←4\n⍝⍝ copies\n-v\n⍝⍝: 2.8\t2.8\t3.4\tinstructions macos 1.5\n\n⍝⍝ steps\nfib 6ₓ\n⍝⍝: -\t-\t-\n')
    changes = set_counts('copies -v: instructions 2.125, allocated 8.000\nsteps fib 6ₓ: instructions 115000.5, allocated 0.000\n', path=path, platform='linux')
    assert len(changes) == 4
    assert '-v\n⍝⍝: 2.8\t2.8\t3.4\tinstructions macos 1.5 linux 2.125\tallocated linux 8\n' in path.read_text()
    assert 'fib 6ₓ\n⍝⍝: -\t-\t-\tinstructions linux 115000.5\tallocated linux 0\n' in path.read_text()
    set_budgets({'-v': 1.37}, path=path, sections='copies')
    assert '-v\n⍝⍝: 2.8\t2.8\t2.8\tinstructions macos 1.5 linux 2.125\tallocated linux 8\n' in path.read_text()
    rm = tmp_path/'rustymath.txt'
    rm.write_text('[float64]\nneg\t1\t1\n')
    set_counts('float64 neg: 1.10, budget 2.4, 0.200 ns, instructions 3.500, allocated 0.000\n', path=rm, platform='linux')
    assert rm.read_text() == '[float64]\nneg\t1\t1\tinstructions linux 3.5\tallocated linux 0\n'


def test_checker_counts_runs_by_mode(tmp_path):
    path = tmp_path/'checked.bpl'
    path.write_text(r'''⍝⍝ cases
⍝ wrong — A case whose expectation every run misses
1+1   ⍝ 3

⍝ right — A case that every run passes
1+1   ⍝ 2
''')
    report = check_file(tmp_path, timeout=10)
    [wrong] = report['failures']
    statuses = {m['mode']: m['status'] for m in wrong['modes']}
    # The second call checks only that it agrees with the first, so a wrong expectation passes it.
    assert wrong['id'] == 'wrong' and statuses.pop("on a dfn's second call") == 'pass', wrong
    assert set(statuses.values()) <= {'mismatch', 'not applicable'}, wrong
    accounting = report['accounting']
    assert accounting['with the batch pref on'] == {'planned': 2, 'checked': 2, 'failed': 1, 'not applicable': 0}, accounting
    # Neither case reads the batch pref, so the run with it off would repeat the run with it on.
    assert accounting['with the batch pref off'] == {'planned': 0, 'checked': 0, 'failed': 0, 'not applicable': 2}, accounting
    with pytest.raises(ValueError, match='match no case'): check_file(path, ids=['absent'])
    selected = check_file(path, ids='right', timeout=10)
    assert not selected['failures'] and selected['accounting']['with the batch pref on'] == {'planned': 1, 'checked': 1, 'failed': 0, 'not applicable': 0}

