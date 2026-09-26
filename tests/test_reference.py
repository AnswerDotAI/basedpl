import json, pytest
from basedpl.reference import Corpus, source_definitions, library_definitions, library_dependencies, lisp_expected, write_cases, scan, selected, activate
from basedpl.reference import april_file


def test_library_definitions(tmp_path):
    path = tmp_path/'library.apl'
    path.write_text("⍝ heading\nf ← { ⍝ comment\n  inner←{'⍝'}\n  inner ⍵\n}\ng←f ⍝ alias\n")
    defs = library_definitions(path)
    assert defs == {'f': "f ← {\n  inner←{'⍝'}\n  inner ⍵\n}", 'g': 'g←f'}
    assert library_dependencies(defs, 'g 1 "f" ⍝ ignored') == defs
    assert library_dependencies(defs, "'g' ⍝ f") == {}
    assert source_definitions('f←{"[⋄}"⍝ }\n ⍵}\nf 2\ng←f ⋄ g 3') == {'f': 'f←{"[⋄}"\n ⍵}', 'g': 'g←f'}
    demo = tmp_path/'demo.lisp'
    demo.write_text('(provision "f←{⍵+1}") (is "f 4" 5)')
    assert [row['status'] for row in april_file(demo, 'april/demo')] == ['setup', 'pending']


def test_scan_and_activate(tmp_path):
    inventory, output, report = tmp_path/'inventory', tmp_path/'cases', tmp_path/'scan.json'
    expected = lisp_expected('#(3 5 7)')
    row = dict(id='ngn:1', code='1 2 3+2 3 4', status='pending', reason='vector addition', expected=expected)
    assert write_cases(inventory, dict(ngn=[row])) == {'ngn': 1}
    result = scan(inventory, report=report)
    assert result['results'][0]['status'] == 'pass'
    assert selected(report, source='ngn')[0]['case'] == row
    activate(report, output=output)
    assert Corpus(inventory)['ngn:1']['status'] == 'active'
    assert '1 2 3+2 3 4   ⍝ 3 5 7' in (output/'ngn.apl').read_text()


def test_corpus_review_and_updates(tmp_path):
    path = tmp_path/'ngn.jsonl'
    rows = [dict(id=f'ngn:{i}', code=code, status='pending', reason='review', expected_error='DOMAIN ERROR', upstream='retained')
        for i, code in enumerate(['⍕1', '{}0'])]
    path.write_text(''.join(json.dumps(r)+'\n' for r in rows))
    corpus = Corpus(tmp_path)
    assert list(corpus.find('⍕', status='pending')) == ['ngn:0']
    assert 'expected_error' not in corpus['ngn:0']
    assert corpus['ngn:0', '*'] == rows[0]
    with pytest.raises(KeyError): corpus['missing']
    assert corpus.update('ngn:1', remove=['expected_error'], expected=None) == {'ngn:1': ['expected', 'expected_error']}
    assert corpus['ngn:1', 'expected', 'upstream'] == dict(expected=None, upstream='retained')
    assert corpus.update('ngn:1', expected=None) == {}
    with pytest.raises(KeyError): corpus.update_many({'ngn:0': dict(status='active'), 'missing': dict(status='active')})
    assert corpus['ngn:0']['status'] == 'pending'
    with pytest.raises(ValueError): corpus.update('ngn:0', expected=float('inf'))
    assert 'expected' not in corpus['ngn:0', '*']
    corpus.update('ngn:1', remove=['reason'], status='active')
    assert list(corpus.find(status='active')) == ['ngn:1']
    corpus.update('ngn:1', code='definition\n'+('x'*200)+' needle')
    preview = corpus.find('needle')['ngn:1']
    assert len(preview) == 180 and '\n' not in preview and preview.endswith('…')
    assert corpus['ngn:1', 'code']['code'].endswith('needle')
