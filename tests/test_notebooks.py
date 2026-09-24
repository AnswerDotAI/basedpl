import json, pytest
from basedpl.notebook_runner import run_notebook


def test_notebook_execution_and_optional_save(tmp_path):
    path = tmp_path/'lesson.ipynb'
    cells = [dict(cell_type='code', id=str(i), source=s, execution_count=7, outputs=[])
        for i,s in enumerate(['x←21', ['x×', '2']])]
    path.write_text(json.dumps(dict(metadata=dict(kernelspec=dict(language='apl')), cells=cells)))
    original = path.read_bytes()
    assert run_notebook(path) == 2
    assert path.read_bytes() == original
    run_notebook(path, save=True)
    saved = json.loads(path.read_text())['cells']
    assert saved[0]['outputs'] == []
    assert saved[1]['outputs'][0]['data']['text/plain'].strip() == '42'
    assert saved[1]['execution_count'] == 7
    cells[0]['source'] = 'x'
    path.write_text(json.dumps(dict(metadata=dict(kernelspec=dict(language='apl')), cells=cells)))
    with pytest.raises(RuntimeError, match=r'cell 1.*VALUE ERROR'): run_notebook(path)
