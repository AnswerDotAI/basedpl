import json
from pathlib import Path
import pytest
from basedpl.reference import Corpus
from basedpl.bpltests import Case, parse, render, convert, add, description, literal, set_budgets, logged_costs, SpeedCosts
from basedpl.worker import Worker

ROOT = Path(__file__).resolve().parents[1]


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
    with pytest.raises(ValueError, match='reserved'): render([Case('1\n⍝ =>\n2', '2')])
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
    path = tmp_path/'speed.txt'
    path.write_text('# `⌽v` has no CI figure yet.\n[setup]\nn←4\n[copies]\n-v\t2.8\t3.4\n⌽v\t3\t4.8\n[widths] •prefs ["float":32]\n-v\t0.83\t0.79\n')
    mac = SpeedCosts([('copies', '-v', 1.37, 3.4), ('widths', '-v', 0.55, 0.83)])
    stamp = '2026-10-12T00:58:18.5848621Z '
    ci = logged_costs(f'{stamp}model name\t: CPU A\n{stamp}copies -v: 2.04, budget 3.4\n{stamp}widths -v: 0.60, budget 0.83\n')
    ci_wasm = logged_costs(f'{stamp}model name\t: CPU B\n{stamp}copies ⌽v: 0.10, budget 4.8\n')
    changes = set_budgets(native=[mac, ci], wasm=[ci_wasm], path=path)
    assert path.read_text() == ('# `⌽v` has no CI figure yet.\n'
                                '[setup]\nn←4\n[copies]\n-v\t4.1\t3.4\n⌽v\t3\t0.5\n[widths] •prefs ["float":32]\n-v\t0.85\t0.79\n')
    assert len(changes) == 3
    set_budgets(native=[SpeedCosts([('copies', '-v', 3, 3.4), ('widths', '-v', 0.7, 0.85)])], path=path, cases='-v', sections='widths')
    assert '[copies]\n-v\t4.1\t3.4\n' in path.read_text() and '[widths] •prefs ["float":32]\n-v\t0.95\t0.79\n' in path.read_text()
