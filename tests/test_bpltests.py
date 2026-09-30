import json
from pathlib import Path
import pytest
from basedpl.reference import Corpus
from basedpl.bpltests import Case, parse, render, convert, add, description, literal, _check

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
    assert parse('⍝ —\n{\n⍵\n}1\n⍝ =>\n1') == [Case('{\n⍵\n}1', '1')]
    assert r'⍝ ⎕: 9\n2' in render(cases)
    with pytest.raises(ValueError, match='escapes'): parse('⍝ —\n1\n1\n⍝ ⎕: \\t\n\n')


def test_comment_extraction():
    corpus = Corpus(ROOT/'tests/reference/inventory')
    assert 'sin(pi/6)' in description(corpus['ngn:177', '*'])
    assert '0 <= x < n' not in description(corpus['ngn:518', '*'])
    comment = description(corpus['april/libraries/dfns/array/demo.lisp:14', '*'])
    assert 'self-contained' not in comment and 'original independent expectation' not in comment


def test_captured_literals():
    for line in (ROOT/'tests/reference/inventory/ngn.jsonl').read_text().splitlines():
        expected = json.loads(line).get('expected')
        if isinstance(expected, dict): assert _check(dict(code=literal(expected), expected=expected))['status']=='pass', literal(expected)


def test_incremental_export(tmp_path):
    inventory = tmp_path/'inventory'
    inventory.mkdir()
    rows = Corpus(ROOT/'tests/reference/inventory').get_many(['ngn:177', 'ngn:517', 'ngn:518'], '*')
    rows['ngn:177']['code'] = '1e¯10>|.5-1○π÷6'
    rows['ngn:518']['code'] = 'n←100⋄A←(n÷2)?n⋄∧/(0≤A),A<n'
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
