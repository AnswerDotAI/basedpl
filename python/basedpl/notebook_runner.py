"Run BPL notebooks directly, optionally saving their output."

import difflib, json
from pathlib import Path
from fastcore.script import call_parse
from fastcore.xtras import working_directory
from fastcore.nbio import write_nb
from . import bpl

LANGUAGES = {'bpl', 'hpl'}


def _outputs(events):
    "A cell's notebook outputs, from the events of its BPL run."
    return [dict(output_type='stream', name='stdout', text=e['data']['text/plain']+'\n') if e['kind'] == 'explicit'
        else dict(output_type='display_data', data=e['data'], metadata={}) for e in events]


def _shown(outputs):
    "The text that `outputs` show: each output's text, or the values of its data."
    def text(v): return (''.join(v) if isinstance(v, list) else str(v)).rstrip('\n')
    return '\n'.join(text(o['text']) if 'text' in o else '\n'.join(text(v) for v in o.get('data', {}).values()) for o in outputs)


def run_notebook(path, save=False, compare=False):
    "Run BPL cells in a cleared workspace, from the notebook's directory. Save outputs only when requested. With `compare`, give `(cell number, id, stored, new)` for each cell whose shown text changed."
    path = Path(path)
    nb = json.loads(path.read_text())
    if nb['metadata']['kernelspec']['language'] not in LANGUAGES: raise ValueError(f'{path}: expected a BPL notebook')
    bpl(']clear')
    changed = []
    with working_directory(path.parent):
        for i, cell in enumerate(nb['cells'], 1):
            if cell['cell_type'] != 'code': continue
            try: result = bpl(''.join(cell['source']), 'repl')
            except Exception as e: raise RuntimeError(f'{path}: cell {i} ({cell.get("id", "")}): {e}') from e
            outputs = _outputs(result.events)
            if compare and (old := _shown(cell.get('outputs', []))) != (new := _shown(outputs)): changed.append((i, cell.get('id', ''), old, new))
            if save: cell['outputs'] = outputs
    if save: write_nb(nb, path)
    return changed


@call_parse(pos=['path'])
def main(
    path:Path=Path('nbs'), # BPL notebook or directory to search
    save:bool=False, # Save outputs in the notebook
    compare:bool=False, # Print a diff for each cell whose outputs differ from the stored ones, and exit with status 1 if any do
):
    "Run BPL notebooks without starting Jupyter, holding the timing lock shared."
    try: from fastws.timelock import timing_lock
    except ImportError: raise SystemExit("bpl-nb runs basedpl's own notebooks and needs the dev extras: pip install 'basedpl[dev]'")
    changed = []
    with timing_lock():
        paths = sorted(path.rglob('*.ipynb')) if path.is_dir() else [path]
        for p in paths:
            if path.is_dir() and json.loads(p.read_text())['metadata'].get('kernelspec', {}).get('language') not in LANGUAGES: continue
            changed += [(p, *c) for c in run_notebook(p, save, compare)]
    for p, i, cid, old, new in changed:
        print(f'{p}: cell {i} ({cid})')
        print('\n'.join(difflib.unified_diff(old.splitlines(), new.splitlines(), 'stored', 'new', lineterm='')))
    if changed: raise SystemExit(1)


if __name__ == '__main__': main()
