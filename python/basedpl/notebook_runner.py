"Run BPL notebooks directly, optionally saving their output."

import json
from pathlib import Path
from fastcore.script import call_parse
from fastcore.xtras import working_directory
from . import bpl


def run_notebook(path, save=False):
    "Run BPL cells in a cleared workspace, from the notebook's directory. Save outputs only when requested."
    path = Path(path)
    nb = json.loads(path.read_text())
    if nb['metadata']['kernelspec']['language'] != 'bpl': raise ValueError(f'{path}: expected a BPL notebook')
    count = 0
    bpl(']clear')
    with working_directory(path.parent):
        for i, cell in enumerate(nb['cells'], 1):
            if cell['cell_type'] != 'code': continue
            try: result = bpl(''.join(cell['source']), 'repl')
            except Exception as e: raise RuntimeError(f'{path}: cell {i} ({cell.get("id", "")}): {e}') from e
            if save:
                cell['outputs'] = [dict(output_type='stream', name='stdout', text=e['data']['text/plain']+'\n') if e['kind'] == 'explicit'
                    else dict(output_type='display_data', data=e['data'], metadata={}) for e in result.events]
            count += 1
    if save: path.write_text(json.dumps(nb, ensure_ascii=False, indent=1)+'\n')
    return count


@call_parse(pos=['path'])
def main(
    path:Path=Path('nbs'), # BPL notebook or directory to search
    save:bool=False, # Save outputs in the notebook
):
    "Run BPL notebooks without starting Jupyter."
    paths = sorted(path.rglob('*.ipynb')) if path.is_dir() else [path]
    for p in paths:
        if path.is_dir() and json.loads(p.read_text())['metadata'].get('kernelspec', {}).get('language') != 'bpl': continue
        print(f'{p}: {run_notebook(p, save)} cells passed')


if __name__ == '__main__': main()
