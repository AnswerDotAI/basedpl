"Read APL test cases, convert reference inventories, and activate reviewed cases."
import argparse, json, math, re
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from .reference import Corpus

HEADER = re.compile(r'^⍝ (?:(\S*) )?—(?: (.*))?$')
SEPARATOR = '⍝ =>'
OUTPUT = '⍝ ⎕:'
OPTIONS = re.compile(r'(?:^| )\[((?:r|a)tol=.*)\]$')


@dataclass
class Case:
    code: str
    expect: str
    id: str = ''
    comment: str = ''
    rtol: float = 0
    atol: float = 0
    line: int = field(default=0, compare=False, repr=False)
    section: str = ''
    output: str | None = None


def _output(text):
    try: return re.sub(r'\\(.)?', lambda m: {'n': '\n', '\\': '\\'}[m[1]], text)
    except KeyError as e: raise ValueError(r'output escapes are \n and \\') from e


def parse(text):
    "Read header-delimited records. Blank lines inside either expression are preserved."
    if not text: return []
    lines = text.split('\n')
    if lines[-1]=='': lines.pop()
    starts = [i for i,line in enumerate(lines) if HEADER.fullmatch(line) or line.startswith('⍝⍝ ')]
    if not starts or starts[0]!=0: raise ValueError('expected a case or section header on line 1')
    result, ids, section = [], set(), ''
    for start,end in zip(starts, starts[1:]+[len(lines)]):
        if lines[start].startswith('⍝⍝ '):
            if any(lines[start+1:end]): raise ValueError(f'line {start+1}: expected a case after section heading')
            section = lines[start][3:]
            continue
        id, comment = HEADER.fullmatch(lines[start]).groups()
        id = id or ''
        if id and id in ids: raise ValueError(f'line {start+1}: duplicate ID {id}')
        ids.add(id)
        comment, options = comment or '', {}
        if match := OPTIONS.search(comment):
            for option in match[1].split():
                key, value = option.split('=', 1)
                value = float(value)
                if key not in ('rtol', 'atol') or key in options or not math.isfinite(value) or value<0:
                    raise ValueError(f'line {start+1}: invalid tolerance {option}')
                options[key] = value
            comment = comment[:match.start()]
        body = lines[start+1:end]
        if not body or body[-1]!='': raise ValueError(f'line {start+1}: missing blank record separator')
        body.pop()
        output = None
        if body and body[-1].startswith(OUTPUT): output = _output(body.pop()[len(OUTPUT):].removeprefix(' '))
        if any(line.startswith(OUTPUT) for line in body): raise ValueError(f'line {start+1}: output expectation must be last')
        count = body.count(SEPARATOR)
        if count==1:
            split = body.index(SEPARATOR)
            code, expect = '\n'.join(body[:split]), '\n'.join(body[split+1:])
        elif count==0 and len(body)==2: code,expect = body
        else: raise ValueError(f'line {start+1}: use one {SEPARATOR!r} between multiline expressions')
        if not expect: raise ValueError(f'line {start+1}: missing expectation')
        result.append(Case(code, expect, id, comment, line=start+1, section=section, output=output, **options))
    return result


def render(cases):
    "Write compact pairs or explicitly separated multiline expressions."
    result, section = [], ''
    for case in cases:
        if '\n' in case.section: raise ValueError('section must fit one line')
        if case.section != section:
            if not case.section: raise ValueError('a section needs a name')
            result.append(f'⍝⍝ {case.section}\n\n')
            section = case.section
        if re.search(r'\s', case.id) or '\n' in case.comment: raise ValueError('ID and comment must fit the header')
        for source in (case.code, case.expect):
            if any(HEADER.fullmatch(line) or line==SEPARATOR or line.startswith(('⍝⍝ ', OUTPUT)) for line in source.split('\n')):
                raise ValueError(f'{case.id}: source contains a reserved fixture marker')
        options = ' '.join(f'{key}={getattr(case, key)}' for key in ('rtol', 'atol') if getattr(case, key))
        comment = case.comment + (f' [{options}]' if options else '')
        header = '⍝ ' + (case.id+' ' if case.id else '') + '—' + (f' {comment.lstrip()}' if comment else '')
        separator = f'\n{SEPARATOR}\n' if '\n' in case.code or '\n' in case.expect else '\n'
        output = '' if case.output is None else '\n'+OUTPUT+(' '+case.output.replace('\\', '\\\\').replace('\n', '\\n') if case.output else '')
        result.append(header+'\n'+case.code+separator+case.expect+output+'\n\n')
    return ''.join(result)


def _number(value):
    text = str(value)
    if text.endswith('.0'): text = text[:-2]
    return text.replace('-', '¯').replace('e+', 'e')


def _chars(text):
    if all(c.isprintable() for c in text): return "'"+text.replace("'", "''")+"'"
    codes = ' '.join(str(ord(c)) for c in text)
    return '•UCS '+codes


def _element(value):
    if isinstance(value, str): return _chars(value)
    if not isinstance(value, dict): return _number(value)
    if 'complex' in value: return 'j'.join(_number(x) for x in value['complex'])
    if 'infinity' in value: return '¯∞' if value['infinity']<0 else '∞'
    return '⊂('+literal(value)+')'


def literal(array):
    "Express a captured reference array in APL, including empty shape and recursive fill."
    if array is None: return '{}0'
    shape, data, prototype = array['shape'], array['data'], array['prototype']
    if not shape: return _element(data[0])
    dims = ' '.join(map(str, shape))
    if not data:
        if shape==[0] and prototype==0: return '⍬'
        if shape==[0] and prototype==' ': return "''"
        return dims+'⍴'+_element(prototype)
    if all(isinstance(x, str) for x in data): values = _chars(''.join(data))
    else:
        def item(x):
            if isinstance(x, dict) and 'shape' in x: return '('+literal(x)+')'
            text = _element(x)
            return '('+text+')' if text.startswith('•UCS ') else text
        values = ' '.join(item(x) for x in data)
    if shape==[len(data)] and len(data)>1: return values
    if len(data)==1: values = _element(data[0])
    return dims+'⍴'+values


_BOILERPLATE = {
    'reviewed independent reference expectation', 'reviewed independent upstream expectation',
    'shared Rust reference checker passes', 'independent upstream expectation passes',
    'Reviewed concrete example checked in Dyalog and through the Rust reference worker',
    'Alternate recipe independently checked in Dyalog and through the Rust reference worker',
    'Small concrete arrays/functions chosen for the APLcart recipe',
    'Supply small inputs appropriate to the documented recipe', 'Small concrete arrays/functions',
    'Small concrete operands', 'Concrete operands for the original recipe', 'Concrete operands',
    'Concrete TIO example', 'original recipe and example retained', 'original TIO example retained',
    'Concrete TIO output expressions collected as one value', 'TIO output expressions collected as one value',
    'displayed values saved once in statement order and returned together', 'comments removed',
    'Concrete operands for catalogue syntax form', 'named derived functions are also called',
    'one-based index/axis operands', 'generated data evaluated at IO=1',
    'Evaluate the original expression at origin 1', 'original origin-0 expectation retained',
    'Remove executable shebang and comments', 'Return the final value instead of explicit output',
    'Translate zero-origin ranges and subscripts to fixed origin one',
    'Origin-one port checked against Dyalog through the Rust reference worker',
    'Fixed origin 1 bounds', '?0 follows Dyalog uniform-real semantics',
    'Deterministic shape/range/permutation/error assertion despite random operands',
    'not yet enabled', 'check prerequisites and expected result',
    'self-contained April library', 'self-contained April array library example',
    'self-contained April array library', 'self-contained pure glyph example',
    'self-contained April library with cross-library dependencies and demo setup',
    'original independent expectation', 'original independent result preserved',
    'operand aliases standardised', 'fixed origin one and standard operand aliases',
    'fixed origin one, standard ∧ and operand aliases', 'origin one', 'no library definitions required',
}
_BOILERPLATE = {s.casefold() for s in _BOILERPLATE}
_APL_PARTS = re.compile(r"'(?:''|[^'])*'|⍝[^\n]*|\s+|[^'⍝\s]+")


def _compact(code):
    return ''.join(m[0] for m in _APL_PARTS.finditer(code) if not m[0].isspace() and not m[0].startswith('⍝'))


def description(row):
    "Extract existing prose and remove recognized import bookkeeping without paraphrasing."
    title = row.get('description', '').strip()
    if not title and row['id'].startswith('ngn:'):
        original = re.split(r'←→|!!', row.get('upstream', ''), maxsplit=1)[0]
        if _compact(original)==_compact(row['code']):
            parts = re.finditer(r"'(?:''|[^'])*'|#([^\n]*)", row.get('expected_apl', ''))
            title = next((m[1] for m in parts if m[1] is not None), '')
    if not title and '/examples/' in row['id']:
        comments = []
        for line in row.get('original_code', '').splitlines():
            if line.startswith('⍝'): comments.append(line[1:].strip())
            elif line.strip() and not line.startswith('#!'): break
        title = ' '.join(comments)
    notes = [title] if title else []
    for key in ('reason', 'adaptation'):
        for clause in re.split(r';\s*|(?<=[a-z])\.\s+(?=[A-Z])', row.get(key, '')):
            clause = ' '.join(clause.split()).rstrip('.')
            if clause and clause.casefold() not in _BOILERPLATE: notes.append(clause)
    result, seen = [], set()
    for note in notes:
        note = ' '.join(note.split()).rstrip('.')
        if note.casefold() not in seen: result.append(note)
        seen.add(note.casefold())
    return '; '.join(result)


def convert(row):
    "Convert a prepared reference record without evaluating its program or changing its status."
    if error := row.get('expected_error'): expect = '⍝ error: '+error
    elif 'expected' in row: expect = literal(row['expected'])
    else: raise ValueError(f"{row['id']}: missing independent expectation")
    return Case(row['code'], expect, row['id'], description(row), row.get('relative_tolerance', 0), row.get('absolute_tolerance', 0))


_RUST_STRING = re.compile(r'r(#{0,16})"([\s\S]*?)"\1|"(?:\\[\s\S]|[^"\\])*"')
_RUST_SPACE = re.compile(r'(?:\s+|//[^\n]*(?:\n|$))*')


def _rust_string(text, pos):
    pos = _RUST_SPACE.match(text, pos).end()
    match = _RUST_STRING.match(text, pos)
    if not match: raise ValueError(f'expected a literal Rust string at character {pos}')
    if match[1] is not None: return match[2], match.end()
    def unescape(m):
        value = m[0][1:]
        if value.startswith('u{'): return chr(int(value[2:-1].replace('_', ''), 16))
        if value.startswith('x'): return chr(int(value[1:], 16))
        if value.startswith('\n'): return ''
        return {'n':'\n', 'r':'\r', 't':'\t', '0':'\0', '\\':'\\', '"':'"', "'":"'"}[value]
    value = re.sub(r'\\(?:u\{[\da-fA-F_]+\}|x[\da-fA-F]{2}|\n\s*|.)', unescape, match[0][1:-1])
    return value, match.end()


def native_cases(path):
    "Extract standalone literal equiv! tables and fails lists; leave stateful/API/constructor tests in Rust."
    text = Path(path).read_text()
    functions = list(re.finditer(r'^fn (\w+)\(', text, re.M))
    result = []
    for match in re.finditer(r'\bequiv!\s*\{|\bfails\(\s*(\w+),\s*&\[', text):
        title = next(m[1].replace('_', ' ') for m in reversed(functions) if m.start()<match.start())
        kind, pos, batch = match[1], match.end(), []
        end = ']' if kind else '}'
        while True:
            pos = _RUST_SPACE.match(text, pos).end()
            if text[pos]==end:
                result.extend(batch)
                break
            if not _RUST_STRING.match(text, pos): break
            code, pos = _rust_string(text, pos)
            if kind: expect = '⍝ error: '+kind.upper()+(' ERROR' if kind not in ('Unsupported', 'Interrupt', 'Timeout') else '')
            else:
                pos = _RUST_SPACE.match(text, pos).end()
                if text[pos:pos+2]!='=>': raise ValueError(f'{path}: expected => at character {pos}')
                pos = _RUST_SPACE.match(text, pos+2).end()
                if not _RUST_STRING.match(text, pos): break
                expect,pos = _rust_string(text, pos)
            batch.append(Case(code, expect, comment=title))
            pos = _RUST_SPACE.match(text, pos).end()
            if text[pos]==',': pos += 1
    return result


def _load(path):
    with path.open(newline='') as f: return parse(f.read())


def add(ids, output='tests/reference', directory='tests/reference/inventory'):
    "Check and append reviewed cases, then mark their inventory records active."
    from ._core import _check_reference
    output = Path(output)
    existing = {case.id for path in output.glob('*.apl') for case in _load(path) if case.id}
    if len(set(ids))!=len(ids) or existing.intersection(ids): raise ValueError('duplicate case ID in selection or destination')
    corpus = Corpus(directory)
    rows = corpus.get_many(ids, '*')
    pending = defaultdict(list)
    for id,row in rows.items():
        if row['status']=='excluded': raise ValueError(f'{id}: excluded: {row["reason"]}')
        case = convert(row)
        result = json.loads(_check_reference(json.dumps(row), 2))
        if result['status']!='pass': raise ValueError(f'{id}: {result}')
        if not row.get('expected_error'):
            result = json.loads(_check_reference(json.dumps(dict(code=case.expect, expected=row['expected'])), 2))
            if result['status']!='pass': raise ValueError(f'{id}: converted expectation: {result}')
        pending[id.split(':')[0].split('/')[0]].append(case)
    texts = {source: render(cases) for source,cases in pending.items()}
    output.mkdir(parents=True, exist_ok=True)
    for source,text in texts.items():
        with (output/f'{source}.apl').open('a', newline='') as f: f.write(text)
    corpus.update(rows, status='active')
    return list(rows)


def preview(directory='tests/reference/inventory', output='meta/apl-preview', native='tests/core.rs', replace=False):
    "Convert active references and standalone native tables into a separate preview directory."
    output = Path(output)
    batches, inventory = {}, Counter()
    for path in sorted(Path(directory).glob('*.jsonl')):
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        inventory.update(row['status'] for row in rows)
        batches[path.stem] = [convert(row) for row in rows if row['status']=='active']
    if native: batches['core'] = native_cases(native)
    texts = {name: render(cases) for name,cases in batches.items()}
    for name,text in texts.items():
        if parse(text)!=batches[name]: raise ValueError(f'{name}: conversion does not round-trip')
        if not replace and (output/f'{name}.apl').exists(): raise FileExistsError(output/f'{name}.apl')
    output.mkdir(parents=True, exist_ok=True)
    for name,text in texts.items():
        with (output/f'{name}.apl').open('w' if replace else 'x', newline='') as f: f.write(text)
    return dict(inventory=dict(inventory), files={name: dict(cases=len(cases), described=sum(bool(c.comment) for c in cases),
                multiline=sum('\n' in c.code or '\n' in c.expect for c in cases)) for name,cases in batches.items()})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['preview', 'add'])
    parser.add_argument('ids', nargs='*')
    parser.add_argument('--directory', default='tests/reference/inventory')
    parser.add_argument('--output', help='defaults to tests/reference for add, meta/apl-preview for preview')
    parser.add_argument('--native', default='tests/core.rs')
    parser.add_argument('--replace', action='store_true')
    args = parser.parse_args()
    if args.action=='add': result = add(args.ids, args.output or 'tests/reference', args.directory)
    else: result = preview(args.directory, args.output or 'meta/apl-preview', args.native, args.replace)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__=='__main__': main()
