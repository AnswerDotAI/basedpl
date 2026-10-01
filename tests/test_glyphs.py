import re
from pathlib import Path
from basedpl import symbols

GLYPHS = Path(__file__).resolve().parents[1]/'nbs/glyphs.qmd'
SECTIONS = {'Functions': {'function'}, 'Operators': {'monadic-operator', 'dyadic-operator'},
            'Syntax and literals': {'argument', 'literal', 'comment', 'system', 'syntax'}}
KEY_NAMES = {'-': 'Minus', '\\': 'Backslash', '`': 'Backtick'}


def key(shortcut):
    "The key column's text for a symbol's `shortcut`: `' a'` is `Alt-a`, and `' c t'` is `Alt-c t`, Option-C then T."
    keys = [KEY_NAMES.get(k, k) for k in shortcut.split()]
    return ' '.join(['Alt-' + keys[0], *keys[1:]]) if keys else ''


def check(path):
    "A message for each place where the tables in `path` disagree with `basedpl.symbols` about a glyph, name, key or kind."
    names, problems, listed, kinds = {s['name']: s for s in symbols}, [], set(), None
    for line in path.read_text().splitlines():
        if line.startswith('## '): kinds = SECTIONS.get(line[3:])
        if not (kinds and line.startswith('| `')): continue
        cells = [c.strip().replace('\\|', '|') for c in re.split(r'(?<!\\)\|', line)[1:-1]]
        keys = cells[1].split(', ') if cells[1] else []
        for i, (form, name, page) in enumerate(re.findall(r'`([^`]+)` \[([\w-]+)\]\(glyphs/([\w-]+)\.qmd\)', cells[0])):
            if name != page or not (path.parent/'glyphs'/f'{page}.qmd').exists(): problems.append(f'`{form}` [{name}]: links to missing or different page {page}')
            s = names.get(name)
            if s is None:
                if 'syntax' not in kinds: problems.append(f'`{form}` [{name}]: no symbol has this name')
                continue
            if s['kind'] in kinds: listed.add(name)
            if s['glyph'] not in form: problems.append(f'`{form}` [{name}]: the glyph is {s["glyph"]}')
            given = keys[i] if i < len(keys) else ''
            if given != key(s['shortcut']): problems.append(f'`{form}` [{name}]: the key is {key(s["shortcut"]) or "none"}, not {given or "none"}')
            forms = [c.replace('—', '') for c in cells[2:4]]
            if s['kind'] == 'function' and 'function' in kinds and forms != [s['monad'], s['dyad']]:
                problems.append(f'`{form}` [{name}]: the monad and dyad are {s["monad"] or "—"} and {s["dyad"] or "—"}, not {" and ".join(cells[2:4])}')
    for s in symbols:
        if s['name'] not in listed: problems.append(f'{s["glyph"]} [{s["name"]}]: missing from the {s["kind"]} table')
    return problems


def test_glyph_reference_matches_symbols(): assert check(GLYPHS) == []
