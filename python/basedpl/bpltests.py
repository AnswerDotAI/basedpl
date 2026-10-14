"Read BPL test cases, convert reference inventories, activate reviewed cases, time BPL as the speed test does, and set the budgets of basedpl's and rustymath's speed tests."
import fcntl, json, math, os, re, subprocess, sys, time, tomllib
from contextlib import contextmanager
from collections import defaultdict
from decimal import Context, Decimal, ROUND_CEILING
from dataclasses import dataclass, field
from pathlib import Path
from ._core import _captured_literal
from .reference import Corpus, _dyalog_string
from . import BplError
from .worker import Worker

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
        elif c=="'":
            next(chars, None)
            if not text.startswith("''", i) or text.startswith("'''", i): next(chars, None)
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


def _failures(request):
    "The failures that a fresh worker gives for `request`, a `reference` or `page` request."
    with Worker() as w: r = w.request(request)
    if e := r['error']: raise ValueError(e['message'])
    return r['failures']


def check_file(
    path, # A `.bpl` reference file
    ids=None # Check only these cases, each named by its id or its header's line number
):
    "Check the cases in `path` in a fresh worker, as `tests/reference.rs` does, and return the failures."
    return _failures(dict(reference=dict(text=Path(path).read_text(), ids=ids)))


def check_page(
    path # A `.qmd` page
):
    "Run the BPL examples in `.qmd` page `path` in a fresh worker, as `tests/core.rs` does, and return the failures."
    return _failures(dict(page=dict(text=Path(path).read_text())))

_SPEED = Path(__file__).parents[2]/'tests'/'speed.txt'
_PKG = _SPEED.with_name('wasm-pkg.mjs')
_LOCK_JS = _SPEED.with_name('timing-lock.mjs')

class Costs(list):
    "Each case's cost, as `(code, picoseconds per item, copies)` rows. Copies is `None` where one copy of the data is too quick to time."
    def __repr__(self): return '\n'.join(f'{ps:>10,.0f} ps {("-" if copies is None else f"{copies:.2f}"):>9} copies  {code}' for code, ps, copies in self)

def time_cases(
    cases, # A BPL expression to time, or a list of them
    setup=None, # BPL that defines the data and its number of items `n`; None: the first `[setup]` section of `tests/speed.txt`
    prefs='', # BPL that runs before `setup`, such as `•prefs ["float":32]`
    threads=1, # Threads that each case's rustymath calls split across, for a scaling check; 0: rustymath's default
    bpl=None, # The `bpl` executable to time, such as a copy saved before a change; None: the installed one
    native=False, # Time rustymath's own linear algebra in place of Accelerate, as `Worker`'s `native` does
    timeout=None # Seconds before the worker is closed
):
    "Time `cases` in a fresh worker, as `tests/speed.rs` does: the fastest of five runs, per item and in copies of the data."
    speed = _SPEED.read_text() if setup is None else f'[setup]\n{setup}'
    cases = [cases] if isinstance(cases, str) else list(cases)
    with Worker(bpl, native) as w: r = w.request(dict(timing=dict(speed=speed, prefs=prefs, cases=cases, threads=threads)), timeout)
    if e := r['error']: raise BplError(e, r['output']) if 'display' in e else ValueError(e['message'])
    return Costs((c['code'], c['seconds']*1e12/r['n'], c['copies']) for c in r['costs'])

class SpeedCosts(list):
    "Each case's cost and the budget that limits it, as `(section, code, cost, budget)` rows. A cost is in copies, except in `[widths]`, where it is the 32-bit time as a fraction of the 64-bit time. A case with no budget has `None`. `cpu` is the CPU model that a CI log reports, or `None`."
    def __init__(self, rows=(), cpu=None):
        super().__init__(rows)
        self.cpu = cpu
    def __repr__(self):
        rows = ((s, c, cost, '-' if b is None else f'{b:g}', 'over' if b is not None and cost > b else '') for s, c, cost, b in self)
        return '\n'.join(f'{cost:>9.2f} {b:>9} {over:4}  {s} {c}' for s, c, cost, b, over in rows)

# The file whose lock keeps timings apart from builds and `pytest` on this machine, as DEV.md describes. Native timings take it
# exclusively in Rust, in `basedpl::timing`, and wasm timings through `tests/timing-lock.mjs`. Builds and `pytest` take it shared.
_TIMING_LOCK = '/tmp/aai-ws-timing.lock'

@contextmanager
def timing_lock():
    "Hold the timing lock shared, as `pytest` does, waiting up to 5 minutes while a timing holds it, then going ahead without it. Never hold the timing lock in the shared kernel process. A timing started from the kernel would wait on that hold for up to 5 minutes. All that time, every agent's kernel calls wait too."
    with open(_TIMING_LOCK, 'a') as f:
        start, waiting = time.monotonic(), False
        while time.monotonic() - start < 300:
            try:
                fcntl.flock(f, fcntl.LOCK_SH | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if not waiting: print(f'waiting for a timing to release {_TIMING_LOCK}', file=sys.stderr)
                waiting = True
                time.sleep(0.25)
        yield
def speed_costs(
    wasm=None, # A directory that `cargo wasm --out` wrote, to time in that browser build, as `tests/wasm-speed.mjs` does; None: a fresh native worker, as `tests/speed.rs` does
    bpl=None, # The `bpl` executable that a native timing runs, such as a copy saved before a change; None: the installed one
    native=False, # Time rustymath's own linear algebra in place of Accelerate in a native timing, as `Worker`'s `native` does
    speed=None, # Text in `speed.txt`'s format; None: `tests/speed.txt`
    timeout=None, # Seconds before the timing stops
    cases=None # A case's code, or a list of codes, to time in every section that has it; None: every case
):
    "Time every case in `speed` as the speed test does, with the budget that limits it in that build."
    speed = _SPEED.read_text() if speed is None else speed
    if isinstance(cases, str): cases = [cases]
    if wasm is None:
        with Worker(bpl, native) as w: r = w.request(dict(costs=dict(speed=speed, cases=cases)), timeout)
        if e := r['error']: raise ValueError(e['message'])
        rows = r['costs']
    else:
        script = (f'import {{ costs }} from {json.dumps(_PKG.as_uri())}; import {{ timingLock }} from {json.dumps(_LOCK_JS.as_uri())}; '
                  f'import {{ readFileSync }} from "node:fs"; const release = await timingLock(); '
                  f'try {{ process.stdout.write(JSON.stringify(costs(readFileSync(0, "utf8"), {json.dumps(cases)}))) }} finally {{ release() }}')
        p = subprocess.run(['node', '--no-liftoff', '--input-type=module', '-e', script], input=speed, capture_output=True, text=True,
                           timeout=timeout, env=os.environ | dict(BASEDPL_WASM=str(wasm)))
        if p.returncode: raise ValueError(p.stderr)
        if p.stderr: sys.stderr.write(p.stderr)
        rows = json.loads(p.stdout)
    i = 0 if wasm is None else 1
    return SpeedCosts((c['section'], c['code'], c['cost'], float(c['budgets'][i]) if len(c['budgets']) > i else None) for c in rows)

_LOGGED = re.compile(r'^(?:\S*Z )?(\w+) (.*): ([\d.]+|inf|NaN|Infinity), budget ([\d.]+)$')
_CPU = re.compile(r'^(?:\S*Z )?model name\s*:\s*(.+?)\s*$', re.M)

def logged_costs(
    log # What a speed test prints with `<NAME>_COSTS` set, such as the log of a job in the `Speed` workflow
):
    "The rows that `speed_costs` gives, read from `log`, with the CPU model that the log reports as their `cpu`."
    cpu = c[1] if (c := _CPU.search(log)) else None
    return SpeedCosts(((m[1], m[2], float(m[3]), float(m[4])) for m in map(_LOGGED.match, log.splitlines()) if m), cpu)

def cargo_costs(
    path='.', # A Cargo package, such as basedpl or rustymath, whose `tests/speed.rs` prints every cost when `<NAME>_COSTS` is set
    timeout=None # Seconds before the run stops
):
    "The rows that `logged_costs` reads from what the speed test of the package at `path` prints, run as `cargo test` runs it. A case over its budget still gives its row."
    path = Path(path).expanduser()
    name = tomllib.loads((path/'Cargo.toml').read_text())['package']['name']
    p = subprocess.run(['cargo', 'test', '--test', 'speed', '--', '--nocapture'], cwd=path, capture_output=True, text=True, timeout=timeout,
                       env=os.environ | {f'{name.upper()}_COSTS': '1'})
    costs = logged_costs(p.stdout + p.stderr)
    if p.returncode and not costs: raise ValueError(p.stderr)
    return costs

class Budgets(list):
    "Each budget that `set_budgets` changed, as `(section, code, build, old, new)` rows."
    def __repr__(self): return '\n'.join(f'{old:>9} -> {new:<9} {build:6}  {s} {c}' for s, c, build, old, new in self)

def _budget(section, cost):
    "The budget that `speed.txt`'s rule gives a case in `section` whose worst cost is `cost`, rounded up to two significant figures, or to two decimal places in `[widths]`."
    if section == 'widths': return Decimal(repr(cost + 0.25)).quantize(Decimal('0.01'), ROUND_CEILING)
    return Context(prec=2, rounding=ROUND_CEILING).create_decimal(repr(max(0.5, 2*cost)))

_SOURCE = '# The CI figures come from '

def _source(run, native, wasm):
    "`speed.txt`'s header line naming `Speed` run `run` and the CPU that each build's CI log reports."
    cpus = {build: sorted({r.cpu for r in runs if r.cpu}) for build, runs in (('native', native), ('wasm', wasm))}
    line = f'{_SOURCE}`Speed` run {run}.'
    if len(every := {c for v in cpus.values() for c in v}) == 1: return f'{line} Its CPU was the {every.pop()}.'
    return line + ''.join(f" Its {build} job's CPU was the {' and the '.join(v)}." for build, v in cpus.items() if v)

def set_budgets(
    native=(), # `SpeedCosts` from each run that counts for the native budgets, such as `speed_costs()`, `cargo_costs(path)` and `logged_costs(log)`
    wasm=(), # `SpeedCosts` from each run that counts for the wasm budgets
    run=None, # The `Speed` run whose logs gave CI's rows, to name in the header with the CPU each log reports; None: leave the header as it is
    path=_SPEED, # The budget file to rewrite: basedpl's `tests/speed.txt`, or rustymath's
    cases=None # A case's code, or a list of codes, whose budgets to change in every section that has it; None: every case
):
    "Rewrite each budget in `path` under the rule in its header, from the case's worst cost in the runs that count. A case with no cost, or not in `cases`, keeps its budget. With `run`, also replace the whole header line that names the CI run. Returns the budgets that changed."
    if isinstance(cases, str): cases = [cases]
    worst = {}
    for i, runs in ((1, native), (2, wasm)):
        for costs in runs:
            for s, c, cost, _ in costs:
                if cases is not None and c not in cases: continue
                if not math.isfinite(cost): raise ValueError(f'{s} {c} costs {cost}')
                worst[s, c, i] = max(cost, worst.get((s, c, i), cost))
    path = Path(path)
    lines, section, changes = path.read_text().split('\n'), None, Budgets()
    for k, line in enumerate(lines):
        if not line or line.startswith('#'): continue
        if m := re.fullmatch(r'\[([^\] ]+)\](?: .*)?', line):
            section = m[1]
            continue
        fields = line.split('\t')
        for i, build in ((1, 'native'), (2, 'wasm')):
            if (cost := worst.get((section, fields[0], i))) is None: continue
            new = f'{_budget(section, cost).normalize():f}'
            if new != fields[i]: changes.append((section, fields[0], build, fields[i], new))
            fields[i] = new
        lines[k] = '\t'.join(fields)
    if run is not None:
        k = next((k for k, line in enumerate(lines) if line.startswith(_SOURCE)), None)
        if k is None: raise ValueError(f'{path} has no line starting {_SOURCE!r}')
        lines[k] = _source(run, native, wasm)
    path.write_text('\n'.join(lines))
    return changes
