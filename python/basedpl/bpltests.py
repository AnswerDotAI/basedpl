"Read BPL test cases, convert reference inventories, and activate reviewed cases."
import json, math, re
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from ._core import _captured_literal
from .reference import Corpus, _dyalog_string

HEADER = re.compile(r'^⍝ (?:(\S*) )?—(?: (.*))?$')
SEPARATOR = '⍝ =>'
INPUT = '⍝ input:'
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
    input: str | None = None


def _unescaped(text):
    try: return re.sub(r'\\(.)?', lambda m: {'n': '\n', '\\': '\\'}[m[1]], text)
    except KeyError as e: raise ValueError(r'input and output escapes are \n and \\') from e


def _escaped(text): return text.replace('\\', '\\\\').replace('\n', '\\n')


def _comment(text):
    chars = iter(enumerate(text))
    for i,c in chars:
        if c=='"': next((j for j,d in chars if d=='"'), None)
        elif c=="'": next(chars, None); next(chars, None)
        elif c=='⍝': return text[:i].rstrip(), text[i+1:].lstrip()


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
        if body and body[-1]=='': body.pop()
        elif end<len(lines): raise ValueError(f'line {start+1}: missing blank record separator')
        output = input = None
        if body and body[-1].startswith(OUTPUT): output = _unescaped(body.pop()[len(OUTPUT):].removeprefix(' '))
        if body and body[-1].startswith(INPUT): input = _unescaped(body.pop()[len(INPUT):].removeprefix(' '))
        if any(line.startswith((INPUT, OUTPUT)) for line in body): raise ValueError(f'line {start+1}: input and output lines must end the case, input first')
        count = body.count(SEPARATOR)
        if count==1:
            split = body.index(SEPARATOR)
            code, expect = '\n'.join(body[:split]), '\n'.join(body[split+1:])
        elif count==0 and body and (pair := _comment('\n'.join(body))) and '\n' not in pair[1] and (len(body)==1 or pair[0].count('\n')==len(body)-1): code,expect = pair
        elif count==0 and len(body)==2: code,expect = body
        else: raise ValueError(f'line {start+1}: use one {SEPARATOR!r} between multiline expressions, or an inline expectation on the last line')
        if not expect: raise ValueError(f'line {start+1}: missing expectation')
        result.append(Case(code, expect, id, comment, line=start+1, section=section, output=output, input=input, **options))
    return result


def render(cases):
    "Write each case inline, as a pair of lines, or with `⍝ =>` between multiline expressions."
    result, section = [], ''
    for case in cases:
        if '\n' in case.section: raise ValueError('section must fit one line')
        if case.section != section:
            if not case.section: raise ValueError('a section needs a name')
            result.append(f'⍝⍝ {case.section}\n\n')
            section = case.section
        if re.search(r'\s', case.id) or '\n' in case.comment: raise ValueError('ID and comment must fit the header')
        for source in (case.code, case.expect):
            if any(HEADER.fullmatch(line) or line==SEPARATOR or line.startswith(('⍝⍝ ', INPUT, OUTPUT)) for line in source.split('\n')):
                raise ValueError(f'{case.id}: source contains a reserved fixture marker')
        options = ' '.join(f'{key}={getattr(case, key)}' for key in ('rtol', 'atol') if getattr(case, key))
        comment = case.comment + (f' [{options}]' if options else '')
        header = '⍝ ' + (case.id+' ' if case.id else '') + '—' + (f' {comment.lstrip()}' if comment else '')
        separator = f'\n{SEPARATOR}\n' if '\n' in case.code or '\n' in case.expect else '\n'
        last = case.code.rsplit('\n', 1)[-1]
        short = len(last)+len(case.expect)<40 if '\n' in case.code else len(case.code)+len(case.expect)+5<70
        if ('\n' not in case.expect and short and case.output is None and case.input is None
            and case.code==case.code.rstrip() and case.expect==case.expect.lstrip()
            and not case.expect.startswith('⍝') and _comment(case.code) is None): separator = '   ⍝ '
        fixtures = ''.join('' if text is None else '\n'+marker+(' '+_escaped(text) if text else '') for marker,text in ((INPUT, case.input), (OUTPUT, case.output)))
        result.append(header+'\n'+case.code+separator+case.expect+fixtures+'\n\n')
    return ''.join(result)


def literal(array):
    "Express a captured reference array in BPL, as the interpreter writes values."
    return '{}0' if array is None else _captured_literal(json.dumps(array))


# Keep related corpus phrases together rather than packing lines by width.
# chkstyle: ignore-node
_BOILERPLATE = {'reviewed independent reference expectation', 'reviewed independent upstream expectation',
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
    'fixed origin one, standard ∧ and operand aliases', 'origin one', 'no library definitions required'}
_BOILERPLATE = {s.casefold() for s in _BOILERPLATE}
_BPL_PARTS = re.compile(_dyalog_string+r"|⍝[^\n]*|\s+|[^'⍝\s]+")


def _compact(code): return ''.join(m[0] for m in _BPL_PARTS.finditer(code) if not m[0].isspace() and not m[0].startswith('⍝'))


def description(row):
    "Extract existing prose and remove recognized import bookkeeping without paraphrasing."
    title = row.get('description', '').strip()
    if not title and row['id'].startswith('ngn:'):
        original = re.split(r'←→|!!', row.get('upstream', ''), maxsplit=1)[0]
        if _compact(original)==_compact(row['code']):
            parts = re.finditer(_dyalog_string+r"|#([^\n]*)", row.get('expected_apl', ''))
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
    elif 'expected_code' in row: expect = row['expected_code']
    elif 'expected' in row: expect = literal(row['expected'])
    else: raise ValueError(f"{row['id']}: missing independent expectation")
    return Case(row['code'], expect, row['id'], description(row), row.get('relative_tolerance', 0), row.get('absolute_tolerance', 0), output=row.get('expected_output'), input=row.get('input'))


def _load(path): return parse(path.read_text())


def _check(case, timeout=2):
    from ._core import _check_reference
    return json.loads(_check_reference(json.dumps(case), timeout))


def add(ids, output='tests/reference', directory='provenance'):
    "Append reviewed cases, then mark their inventory records active."
    output = Path(output)
    existing = {case.id for path in output.glob('*.bpl') for case in _load(path) if case.id}
    if len(set(ids))!=len(ids) or existing.intersection(ids): raise ValueError('duplicate case ID in selection or destination')
    corpus = Corpus(directory)
    rows = corpus.get_many(ids, '*')
    pending = defaultdict(list)
    for id,row in rows.items():
        if row['status']=='excluded': raise ValueError(f'{id}: excluded: {row["reason"]}')
        case = convert(row)
        pending[id.split(':')[0].split('/')[0]].append(case)
    texts = {source: render(cases) for source,cases in pending.items()}
    output.mkdir(parents=True, exist_ok=True)
    for source,text in texts.items():
        path = output/f'{source}.bpl'
        previous = path.read_text() if path.exists() else ''
        if previous: text = previous.rstrip('\n')+'\n\n'+text
        path.write_text(text)
    corpus.update(rows, status='active')
    return list(rows)


def check_file(
    path, # A `.bpl` reference file
    ids=None, # Check only the cases whose id or header line is in `ids`
    timeout=2 # Seconds allowed for each case
):
    "Check the cases in `path` through the installed extension, as `tests/reference.rs` does, and return the failures."
    path = Path(path)
    fails = []
    for c in _load(path):
        if ids and c.id not in ids and c.line not in ids: continue
        case = dict(code=c.code)
        if c.expect.startswith('⍝ error: '): case['expected_error'] = c.expect.removeprefix('⍝ error: ')
        else: case['expected_code'] = c.expect
        if c.output is not None: case['expected_output'] = c.output
        if c.input is not None: case['input'] = c.input
        if c.rtol: case['relative_tolerance'] = c.rtol
        if c.atol: case['absolute_tolerance'] = c.atol
        r = _check(case, timeout)
        if r['status']!='pass': fails.append(dict(line=c.line, id=c.id, status=r['status'], message=r.get('message'), code=c.code, expect=c.expect))
    return fails


def check_page(path):
    "Run the BPL examples in `.qmd` page `path` as `tests/core.rs` does, and return the failures."
    # `_check_reference` compares values but runs each case in a fresh session, so each check replays its block from the start.
    fails, block = [], None
    for i,text in enumerate(Path(path).read_text().splitlines(), 1):
        if text=='```bpl':
            block = []
            continue
        if block is None: continue
        closing = text.startswith('```')
        source, _, expected = text.partition(' ⍝ ')
        if not closing: block.append(source)
        if expected or closing:
            r = _check(dict(code='\n'.join(block), expected_code=expected or '{}0', relative_tolerance=1e-13, absolute_tolerance=1e-13))
            error = (r.get('actual') or {}).get('error')
            if (expected and r['status']!='pass') or (not expected and error):
                fails.append(dict(line=i, code=source, expected=expected, status=r['status'], message=r.get('message')))
        if closing: block = None
    return fails
