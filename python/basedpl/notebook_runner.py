"Run bAsedPL notebooks directly, optionally saving their output."

import json
from pathlib import Path
from fastcore.script import call_parse
from . import apl


def run_notebook(path, save=False):
    "Run APL cells in a cleared workspace. Save outputs only when requested."
    path = Path(path)
    nb = json.loads(path.read_text())
    if nb['metadata']['kernelspec']['language'] != 'apl': raise ValueError(f'{path}: expected an APL notebook')
    count = 0
    apl(']clear')
    for i, cell in enumerate(nb['cells'], 1):
        if cell['cell_type'] != 'code': continue
        try: result = apl(''.join(cell['source']), 'repl')
        except Exception as e: raise RuntimeError(f'{path}: cell {i} ({cell.get("id", "")}): {e}') from e
        if save:
            cell['outputs'] = [dict(output_type='stream', name='stdout', text=e['data']['text/plain']+'\n') if e['kind'] == 'explicit'
                else dict(output_type='display_data', data=e['data'], metadata={}) for e in result.events]
        count += 1
    if save: path.write_text(json.dumps(nb, ensure_ascii=False, indent=1)+'\n')
    return count


@call_parse
def main(
    path:Path, # APL notebook or directory to search
    save:bool=False, # Save outputs in the notebook
):
    "Run bAsedPL notebooks without starting Jupyter."
    paths = sorted(path.rglob('*.ipynb')) if path.is_dir() else [path]
    for p in paths:
        if path.is_dir() and json.loads(p.read_text())['metadata'].get('kernelspec', {}).get('language') != 'apl': continue
        print(f'{p}: {run_notebook(p, save)} cells passed')


if __name__ == '__main__': main()
