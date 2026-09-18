import json
import pytest
from miniapl.reference import Corpus, library_definitions, library_dependencies


def test_library_definitions(tmp_path):
    path = tmp_path/'library.apl'
    path.write_text("⍝ heading\nf ← { ⍝ comment\n  inner←{'⍝'}\n  inner ⍵\n}\ng←f ⍝ alias\n")
    defs = library_definitions(path)
    assert defs == {'f': "f ← {\n  inner←{'⍝'}\n  inner ⍵\n}", 'g': 'g←f'}
    assert library_dependencies(defs, "g 1 'f' ⍝ ignored") == defs
    assert library_dependencies(defs, "'g' ⍝ f") == {}


def test_corpus_review_and_updates(tmp_path):
    path = tmp_path/'ngn.jsonl'
    rows = [dict(id=f'ngn:{i}', code=code, status='pending', reason='review', expected_error='DOMAIN ERROR', upstream='retained')
            for i, code in enumerate(['⍕1', '{}0'])]
    path.write_text(''.join(json.dumps(r)+'\n' for r in rows))
    corpus = Corpus(tmp_path)
    assert list(corpus.find('⍕', status='pending')) == ['ngn:0']
    assert 'expected_error' not in corpus.get('ngn:0')
    assert corpus.update('ngn:1', remove=['expected_error'], expected=None) == {'ngn:1': ['expected', 'expected_error']}
    assert corpus.get('ngn:1', 'expected', 'upstream') == dict(expected=None, upstream='retained')
    assert corpus.update('ngn:1', expected=None) == {}
    with pytest.raises(KeyError): corpus.update_many({'ngn:0': dict(status='active'), 'missing': dict(status='active')})
    assert corpus.get('ngn:0')['status'] == 'pending'
    with pytest.raises(ValueError): corpus.update('ngn:0', expected=float('inf'))
    assert 'expected' not in corpus.get('ngn:0', '*')
    corpus.update('ngn:1', remove=['reason'], status='active')
    assert list(corpus.find(status='active')) == ['ngn:1']
