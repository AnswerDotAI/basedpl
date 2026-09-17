import json
import pytest
from miniapl.reference import Corpus


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
