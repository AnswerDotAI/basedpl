"Prepare the docs for a release. Check the glyph reference, write the highlighters' glyph lists, the docs' font stylesheet and the macOS keyboard layout, then clean and export the notebooks and render README.md."
import re, runpy, subprocess, sys
from pathlib import Path
from xml.sax.saxutils import escape
from basedpl import symbols
from basedpl._core import _scripts  # Superscripts and subscripts, which `symbols` leaves out

SECTIONS = {'Functions': 'function', 'Operators': 'operator', 'Syntax and literals': 'syntax'}
KEY_NAMES = {'-': 'Minus', '\\': 'Backslash', '`': 'Backtick'}


def glyphs(kind): return ''.join(s['glyph'] for s in symbols if s['kind'] == kind) + (_scripts if kind == 'operator' else '')


def key(shortcut):
    "The key column's text for a symbol's `shortcut`: `' a'` is `Alt-a`, and `' c t'` is `Alt-c t`, Option-C then T."
    keys = [KEY_NAMES.get(k, k) for k in shortcut.split()]
    return ' '.join(['Alt-' + keys[0], *keys[1:]]) if keys else ''


def check(path='nbs/glyphs.qmd'):
    "A message for each place where the tables in `path` disagree with `basedpl.symbols` about a glyph, name, key or kind."
    names, problems, listed, kind = {s['name']: s for s in symbols}, [], set(), None
    for line in Path(path).read_text().splitlines():
        if line.startswith('## '): kind = SECTIONS.get(line[3:])
        if not (kind and line.startswith('| `')): continue
        cells = [c.strip().replace('\\|', '|') for c in re.split(r'(?<!\\)\|', line)[1:-1]]
        keys = cells[1].split(', ') if cells[1] else []
        for i, (form, name, page) in enumerate(re.findall(r'`([^`]+)` \[([\w-]+)\]\(glyphs/([\w-]+)\.qmd\)', cells[0])):
            if name != page or not Path(f'nbs/glyphs/{page}.qmd').exists(): problems.append(f'`{form}` [{name}]: links to missing or different page {page}')
            s = names.get(name)
            if s is None:
                if kind != 'syntax': problems.append(f'`{form}` [{name}]: no {kind} has this name')
                continue
            if s['kind'] == kind: listed.add(name)
            if s['glyph'] not in form: problems.append(f'`{form}` [{name}]: the glyph is {s["glyph"]}')
            given = keys[i] if i < len(keys) else ''
            if given != key(s['shortcut']): problems.append(f'`{form}` [{name}]: the key is {key(s["shortcut"]) or "none"}, not {given or "none"}')
            forms = [c.replace('—', '') for c in cells[2:4]]
            if kind == s['kind'] == 'function' and forms != [s['monad'], s['dyad']]:
                problems.append(f'`{form}` [{name}]: the monad and dyad are {s["monad"] or "—"} and {s["dyad"] or "—"}, not {" and ".join(cells[2:4])}')
    for s in symbols:
        if s['name'] not in listed: problems.append(f'{s["glyph"]} [{s["name"]}]: missing from the {s["kind"]} table')
    return problems


def replace(path, pattern, text):
    "Replace the one match of `pattern` in the file at `path` with `text`, and say whether that changed the file."
    old = Path(path).read_text()
    new, n = re.subn(pattern, lambda _: text, old)
    if n != 1: sys.exit(f'{path}: expected one match of {pattern}, found {n}')
    if new != old: Path(path).write_text(new)
    return new != old


def vim_class(chars):
    "A vim pattern matching any of `chars`, between delimiters that aren't among them."
    body = chars.replace('\\', '\\\\').replace(']', '\\]').replace('^', '\\^')
    if '-' in body: body = body.replace('-', '') + '-'
    delimiter = next(c for c in '/#%' if c not in chars)
    return f'{delimiter}[{body}]{delimiter}'


def highlight():
    "Write the function and operator glyph lists of the Quarto and vim highlighters, and return the files that changed."
    changed = set()
    for kind, attribute in [('function', 'Function'), ('operator', 'Operator')]:
        text = f'<AnyChar String="{escape(glyphs(kind), {chr(34): "&quot;"})}" attribute="{attribute}"/>'
        if replace('nbs/bpl.xml', f'<AnyChar String="[^"]*" attribute="{attribute}"/>', text): changed.add('nbs/bpl.xml')
        group = 'bpl' + attribute
        if replace('editors/vim/syntax/bpl.vim', f'(?m)^syntax match {group} .*$', f'syntax match {group} {vim_class(glyphs(kind))}'): changed.add('editors/vim/syntax/bpl.vim')
    return sorted(changed)


def fonts():
    "Copy the package's font stylesheet to `nbs/fonts.css`, and say whether that changed the file."
    text, dest = Path('python/basedpl/fonts.css').read_text(), Path('nbs/fonts.css')
    changed = not dest.exists() or dest.read_text() != text
    if changed: dest.write_text(text)
    return changed


def main():
    problems = check()
    if problems: sys.exit('nbs/glyphs.qmd disagrees with basedpl.symbols:\n' + '\n'.join(problems))
    for path in highlight(): print(f'wrote {path}')
    if fonts(): print('wrote nbs/fonts.css')
    for path in runpy.run_path(str(Path(__file__).with_name('keylayout.py')))['write'](): print(f'wrote {path}')
    for command in ['nbdev-clean', 'nbdev-export', 'nbdev-readme']: subprocess.run([command], check=True)


if __name__ == '__main__': main()
