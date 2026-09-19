import json
from pathlib import Path
import pytest
from miniapl._core import _check_reference
from miniapl.reference import Corpus
from miniapl.apltests import Case, parse, render, convert, native_cases, add, description

ROOT = Path(__file__).resolve().parents[1]


def test_records_and_boundaries():
    cases = [Case('1+2', '3', 'ngn:1', 'addition'),
             Case("f←{\n\n⍵+1\n}\nf 2\n", '(\n3\n)', comment='multiline', rtol=1e-14, atol=1e-15),
             Case("'unfinished", '⍝ error: SYNTAX ERROR'), Case('', '⍝ error: SYNTAX ERROR')]
    text = render(cases)
    for ending in ['', '\n', '\n\n']: assert parse(text.rstrip('\n')+ending) == cases
    assert text.count('\n⍝ =>\n') == 1
    assert parse(text)[1].line == 4
    assert '1+2   ⍝ 3' in text
    quoted = Case("'a''⍝b'", "'a''⍝b'")
    assert parse(render([quoted])) == [quoted]
    assert parse('⍝ —\n'+'1'*40+' ⍝ 1') == [Case('1'*40, '1')]
    assert '\n'+'1'*40+'\n1\n' in render([Case('1'*40, '1')])
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
    assert parse('⍝ —\n{\n⍵\n}1\n⍝ =>\n1') == [Case('{\n⍵\n}1', '1')]
    assert r'⍝ ⎕: 9\n2' in render(cases)
    with pytest.raises(ValueError, match='escapes'): parse('⍝ —\n1\n1\n⍝ ⎕: \\t\n\n')


def test_reference_roundtrip():
    paths = list((ROOT/'tests/reference/inventory').glob('*.jsonl'))
    assert paths
    for path in paths:
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        rows = [row for row in rows if row['status']=='active']
        cases = [convert(row) for row in rows]
        assert parse(render(cases)) == cases
        saved = parse((path.parent.parent/f'{path.stem}.apl').read_text())
        assert saved and parse(render(saved)) == saved
        for row,case in zip(rows, cases):
            assert case.id == row['id'] and case.code == row['code']
            assert case.rtol == row.get('relative_tolerance', 0)
            assert case.atol == row.get('absolute_tolerance', 0)
            if row.get('expected_error'):
                assert case.expect == '⍝ error: '+row['expected_error']
                continue
            check = dict(code=case.expect, expected=row['expected'])
            result = json.loads(_check_reference(json.dumps(check), 2))
            assert result['status']=='pass', (row['id'], case.expect, result)


def test_comment_extraction():
    corpus = Corpus(ROOT/'tests/reference/inventory')
    assert 'sin(pi/6)' in description(corpus['ngn:177', '*'])
    assert '0 <= x < n' not in description(corpus['ngn:518', '*'])
    comment = description(corpus['april/libraries/dfns/array/demo.lisp:14', '*'])
    assert 'self-contained' not in comment and 'original independent expectation' not in comment


def test_native_and_incremental_export(tmp_path):
    native = tmp_path/'core.rs'
    native.write_text('fn sums() { equiv! { "+/2 3⍴⍳6" => "6 15" } fails(Syntax, &["(\\n"]); }')
    cases = native_cases(native)
    assert cases and all(not case.id for case in cases)
    assert parse(render(cases)) == cases
    assert any(case.expect=='⍝ error: SYNTAX ERROR' and '\n' in case.code for case in cases)
    assert any(case.code=='+/2 3⍴⍳6' and case.expect=='6 15' for case in cases)
    inventory = tmp_path/'inventory'
    inventory.mkdir()
    rows = Corpus(ROOT/'tests/reference/inventory').get_many(['ngn:177', 'ngn:517', 'ngn:518'], '*')
    rows['ngn:177']['code'] = '1e¯10>|.5-1○π÷6'
    for row in rows.values(): row['status'] = 'pending'
    (inventory/'ngn.jsonl').write_text(''.join(json.dumps(row)+'\n' for row in rows.values()))
    assert add(['ngn:177', 'ngn:517'], tmp_path, inventory) == ['ngn:177', 'ngn:517']
    path = tmp_path/'ngn.apl'
    path.write_text(path.read_text().rstrip('\n'))
    assert add(['ngn:518'], tmp_path, inventory) == ['ngn:518']
    assert [case.id for case in parse((tmp_path/'ngn.apl').read_text())] == ['ngn:177', 'ngn:517', 'ngn:518']
    saved = (tmp_path/'ngn.apl').read_bytes()
    with pytest.raises(ValueError, match='duplicate'): add(['ngn:518'], tmp_path, inventory)
    assert (tmp_path/'ngn.apl').read_bytes() == saved
    assert all(row['status']=='active' for row in Corpus(inventory).get_many(rows, '*').values())
    bad = rows['ngn:177']
    bad.update(id='ngn:bad', code='0', status='pending')
    (inventory/'ngn.jsonl').write_text(json.dumps(bad)+'\n')
    with pytest.raises(ValueError, match='mismatch'): add(['ngn:bad'], tmp_path, inventory)
    assert (tmp_path/'ngn.apl').read_bytes() == saved
