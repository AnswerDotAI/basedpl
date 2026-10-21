"Read BPL test cases, convert reference inventories, activate reviewed cases, time BPL as the speed test does, and set the budgets of basedpl's and rustymath's speed tests."
import json, math, os, re, subprocess, sys, tempfile, tomllib
from collections import defaultdict
from decimal import Context, Decimal, ROUND_CEILING
from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import Annotated
from fastws.cargo import timing_lock
from fastcore.script import parse_cli
from ._core import _captured_literal, _reference_cases
from .reference import Corpus, _dyalog_string
from . import BplError
from .worker import INSTALLED_BPL, NATIVE, Worker

SEPARATOR = '⍝ =>'
INPUT = '⍝ input:'
OUTPUT = '⍝ ⎕:'
ERROR = '⍝ error: '

# The header options that set a flag on a case, as Rust's `reference::FLAGS` names them.
FLAGS = 'unfused', 'batched', 'mixed', 'serial'


@dataclass
class Case:
    code: str
    expect: str
    id: str = ''
    comment: str = ''
    rtol: float = 0
    atol: float = 0
    unfused: bool = False
    batched: bool = False
    mixed: bool = False
    serial: bool = False
    line: int = field(default=0, compare=False, repr=False)
    section: str = ''
    output: str | None = None
    input: str | None = None
    prefs: str | None = None


def _escaped(text): return text.replace('\\', '\\\\').replace('\n', '\\n')


def parse(text):
    "The cases of `text`, a `.bpl` reference file, as Rust's `reference::cases` reads them. A malformed file raises `ValueError`."
    return [Case(code=c['code'], expect=c['expected_code'] if 'expected_code' in c else ERROR+c['expected_error'], id=c['id'], comment=c['comment'],
                 rtol=c.get('relative_tolerance', 0), atol=c.get('absolute_tolerance', 0), **{key: c.get(key, False) for key in FLAGS}, line=c['line'],
                 section=c['section'], output=c.get('expected_output'), input=c.get('input'), prefs=c.get('prefs')) for c in _reference_cases(text)]


def _reads_back(record, case):
    "Whether `parse` reads `record`, the text of one case without its section heading, as `case`."
    try: [read] = parse(record)
    except ValueError: return False
    return replace(read, section=case.section, prefs=case.prefs) == case


def render(cases):
    "Write each case inline, as a pair of lines, or with `⍝ =>` between multiline expressions. Raises `ValueError` where `parse` wouldn't read the result back as `cases`."
    if len({case.prefs for case in cases}) > 1: raise ValueError('cases from files with different `•prefs` headers')
    result, section = [cases[0].prefs+'\n\n'] if cases and cases[0].prefs else [], ''
    for case in cases:
        if case.section != section:
            result.append(f'⍝⍝ {case.section}\n\n')
            section = case.section
        options = ' '.join([f'{key}={getattr(case, key)}' for key in ('rtol', 'atol') if getattr(case, key)] + [key for key in FLAGS if getattr(case, key)])
        comment = case.comment + (f' [{options}]' if options else '')
        header = '⍝ ' + (case.id+' ' if case.id else '') + '—' + (f' {comment.lstrip()}' if comment else '')
        fixtures = ''.join('' if text is None else '\n'+marker+(' '+_escaped(text) if text else '') for marker,text in ((INPUT, case.input), (OUTPUT, case.output)))
        def record(separator): return header+'\n'+case.code+separator+case.expect+fixtures+'\n\n'
        separator = f'\n{SEPARATOR}\n' if '\n' in case.code or '\n' in case.expect else '\n'
        last = case.code.rsplit('\n', 1)[-1]
        short = len(last)+len(case.expect)<40 if '\n' in case.code else len(case.code)+len(case.expect)+5<70
        if short and case.output is None and case.input is None and not case.expect.startswith('⍝') and _reads_back(record('   ⍝ '), case): separator = '   ⍝ '
        result.append(record(separator))
    text = ''.join(result)
    if (read := parse(text)) != list(cases):
        case = next((c for c, r in zip(cases, read) if c != r), cases[-1])
        raise ValueError(f'{case.id or case.code}: the rendered text reads back as a different case')
    return text


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
    if error := row.get('expected_error'): expect = ERROR+error
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


def _failures(request, bpl=None):
    "The failures that a fresh worker running `bpl` gives for `request`, a `reference` or `page` request. The request also carries one case that must fail, and a reply that doesn't report it raises `ValueError`, so a worker that checked nothing, or passed everything, never reads as a pass."
    (kind, spec), = request.items()
    if kind == 'reference':
        text = spec['text'].rstrip('\n') + '\n\n⍝ bpltests-sentinel — The checker must report this case\n0   ⍝ 1\n'
        spec = {**spec, 'text': text, 'ids': spec['ids'] and [*spec['ids'], 'bpltests-sentinel']}
        sentinel = lambda f: f.get('id') == 'bpltests-sentinel'
    else:
        text = spec['text'].rstrip('\n') + '\n\n```bpl\n0 ⍝ 1\n```\n'
        spec, line = {**spec, 'text': text}, text.count('\n') - 1
        sentinel = lambda f: f.get('line') == line
    with Worker(bpl) as w: r = w.request({kind: spec})
    if e := r['error']: raise ValueError(e['message'])
    if sum(map(sentinel, r['failures'])) != 1: raise ValueError("the worker didn't report the case that must fail, so its result can't be trusted")
    return [f for f in r['failures'] if not sentinel(f)]


def check_file(
    path, # A `.bpl` reference file
    ids:Annotated[int|str, {'nargs':'*'}]=None, # Check only these cases, each named by its id or its header's line number
    bpl:Path=None # A native `bpl` to check, such as a copy saved before a change, as `Worker`'s `bpl` does; None: the installed one
):
    "Check the cases in `path` in a fresh worker, as `tests/reference.rs` does, and return the failures."
    if isinstance(ids, (int, str)): ids = [ids]
    return _failures(dict(reference=dict(text=Path(path).read_text(), ids=ids)), bpl)


def check_page(
    path, # A `.qmd` page
    bpl:Path=None, # A native `bpl` to check, such as a copy saved before a change, as `Worker`'s `bpl` does; None: the installed one
    unbatched:bool=False # Run each example with the `batch` pref off
):
    "Run the BPL examples in `.qmd` page `path` in a fresh worker, as `tests/core.rs` does, and return the failures."
    return _failures(dict(page=dict(text=Path(path).read_text(), batch=False if unbatched else None)), bpl)

_SPEED = Path(__file__).parents[2]/'tests'/'speed.txt'
# The budgets on each case line of a budget file, after its code, in order: macOS on Apple silicon, Linux on CI's x86 runner, then the browser build where the file has one.
_BUDGETS = ('macos', 'linux', 'wasm')

def _platform():
    "This machine's platform, whose native budget column it checks, as `rustymath::timing::NATIVE_COLUMN` picks it."
    import rustymath
    return _BUDGETS[rustymath.timing().column]

_PKG = _SPEED.with_name('wasm-pkg.mjs')

class Costs(list):
    "Each case's cost, as `(code, picoseconds per item, copies)` rows. Copies is `None` where one copy of the data is too quick to time."
    def __repr__(self): return '\n'.join(f'{ps:>10,.0f} ps {("-" if copies is None else f"{copies:.2f}"):>9} copies  {code}' for code, ps, copies in self)

def time_cases(
    cases:Annotated[str, {'nargs':'+'}], # A BPL expression to time, or a list of them
    setup:str=None, # BPL that defines the data and its number of items `n`; None: the first `[setup]` section of `tests/speed.txt`
    prefs='', # BPL that runs before `setup`, such as `•prefs ["float":32]`
    threads=1, # Threads that each case's rustymath calls split across, for a scaling check; 0: rustymath's default
    native=False, # Time rustymath's own linear algebra in place of Accelerate, as `Worker`'s `native` does
    bpl:Path=None, # A native `bpl` to time, such as a copy saved before a change, as `Worker`'s `bpl` does; None: the installed one
    timeout:float=None # Seconds before the worker is closed
):
    "Time `cases` in a fresh worker, as `tests/speed.rs` does: the fastest of five runs, per item and in copies of the data."
    speed = _SPEED.read_text() if setup is None else f'[setup]\n{setup}'
    cases = [cases] if isinstance(cases, str) else list(cases)
    with Worker(bpl, native) as w: r = w.request(dict(timing=dict(speed=speed, prefs=prefs, cases=cases, threads=threads)), timeout)
    if e := r['error']: raise BplError(e, r['output']) if 'display' in e else ValueError(e['message'])
    return Costs((c['code'], c['seconds']*1e12/r['n'], c['copies']) for c in r['costs'])

class Instructions(list):
    "Each case's instructions per unit of `N`, as `(code, instructions)` rows."
    def __repr__(self): return '\n'.join(f'{n:>12,.0f}  {code}' for code, n in self)

# The line of each platform's counter that gives the instructions a process retired: `/usr/bin/time -l`'s on macOS, `perf stat -x,`'s on Linux.
_RETIRED = dict(darwin=re.compile(r'^\s*(\d+)\s+instructions retired$', re.M), linux=re.compile(r'^(\d+),[^,]*,instructions', re.M))

def _retired(code, bpl):
    "The instructions that a fresh `bpl` process retires running `code`."
    if sys.platform not in _RETIRED: raise ValueError(f'no instruction counter on {sys.platform}')
    with tempfile.NamedTemporaryFile('r') as report:
        counter = ['/usr/bin/time', '-l', '-o', report.name] if sys.platform == 'darwin' else ['perf', 'stat', '-x,', '-e', 'instructions:u', '-o', report.name, '--']
        p = subprocess.run([*counter, bpl or INSTALLED_BPL, '-e', code], capture_output=True, text=True)
        if p.returncode: raise ValueError(p.stdout + p.stderr)
        return int(_RETIRED[sys.platform].search(report.read())[1])

def instructions(
    cases:Annotated[str, {'nargs':'+'}], # A BPL expression that makes one call per step of `N`, such as `{⍵}⍣N⊢0`, or a list of them
    setup='', # BPL that defines what the cases use, run before each case
    counts:Annotated[int, {'nargs':2}]=(1_000, 101_000), # The two values of `N`
    runs=3, # Processes at each value of `N`, of which the lowest count is used
    bpl:Path=None # A native `bpl` to count, such as a copy saved before a change; None: the installed one
):
    "The instructions that each case retires per unit of `N`, on one thread: the difference between fresh `bpl` processes at the two `counts`, divided by the difference in `N`, which cancels startup and `setup`. Unlike timings, instruction counts don't vary with load or code layout, so they show per-call changes of a few percent. A case must make one call per step of `N`, as `⍣` does: a map such as `f¨N⍴0` calls a dfn once on all its cells, so it gives no per-call cost."
    cases = [cases] if isinstance(cases, str) else list(cases)
    def count(code, n): return min(_retired(f'•prefs ["threads":1]\nN←{n}\n{setup}\nr←{code}\n0', bpl) for _ in range(runs))
    lo, hi = counts
    return Instructions((code, (count(code, hi) - count(code, lo)) / (hi - lo)) for code in cases)

class SpeedCosts(list):
    "Each case's cost and the budget that limits it, as `(section, code, cost, budget)` rows. A cost is in copies, except in `[widths]`, where it is the 32-bit time as a fraction of the 64-bit time. A case with no budget has `None`. A case over its budget needs study, which finds either a speed bug or something its expected cost missed. `cpu` is the CPU model that a CI log reports, or `None`. `platform` is the platform whose native budgets the costs were checked against, 'macos' or 'linux', or `None` for a browser build."
    def __init__(self, rows=(), cpu=None, platform=None):
        super().__init__(rows)
        self.cpu, self.platform = cpu, platform
    def __repr__(self):
        rows = ((s, c, cost, '-' if b is None else f'{b:g}', 'over' if b is not None and cost > b else '') for s, c, cost, b in self)
        return '\n'.join(f'{cost:>9.2f} {b:>9} {over:4}  {s} {c}' for s, c, cost, b, over in rows)

def speed_costs(
    wasm:Path=None, # A directory that `cargo wasm --out` wrote, to time in that browser build, as `tests/wasm-speed.mjs` does; None: a fresh native worker, as `tests/speed.rs` does
    native=False, # Time rustymath's own linear algebra in place of Accelerate in a native timing, as `Worker`'s `native` does
    bpl:Path=None, # A native `bpl` to time, such as a copy saved before a change, as `Worker`'s `bpl` does; None: the installed one
    speed:str=None, # Text in `speed.txt`'s format; None: `tests/speed.txt`
    timeout:float=None, # Seconds before the timing stops
    cases:Annotated[str, {'nargs':'*', 'opt':False}]=None # A case's code, or a list of codes, to time in every section that has it; None or empty: every case
):
    "Time every case in `speed` as the speed test does, with the budget that limits it in that build."
    speed = _SPEED.read_text() if speed is None else speed
    cases = [cases] if isinstance(cases, str) else cases or None
    if wasm is None:
        with Worker(bpl, native) as w: r = w.request(dict(costs=dict(speed=speed, cases=cases)), timeout)
        if e := r['error']: raise ValueError(e['message'])
        rows = r['costs']
    else:
        script = (f'import {{ costs }} from {json.dumps(_PKG.as_uri())}; import {{ readFileSync }} from "node:fs"; '
                  f'process.stdout.write(JSON.stringify(costs(readFileSync(0, "utf8"), {json.dumps(cases)})))')
        with timing_lock(exclusive=True):
            p = subprocess.run(['node', '--no-liftoff', '--input-type=module', '-e', script], input=speed, capture_output=True, text=True,
                               timeout=timeout, env=os.environ | dict(BASEDPL_WASM=str(wasm)))
        if p.returncode: raise ValueError(p.stderr)
        if p.stderr: sys.stderr.write(p.stderr)
        rows = json.loads(p.stdout)
    build = _platform() if wasm is None else 'wasm'
    i = _BUDGETS.index(build)
    rows = ((c['section'], c['code'], c['cost'], float(c['budgets'][i]) if len(c['budgets']) > i else None) for c in rows)
    return SpeedCosts(rows, platform=None if wasm else build)

_LOGGED = re.compile(r'^(?:\S*Z )?(?P<tap># )?(?P<section>\w+) (?P<code>\S.*): (?P<cost>[\d.]+|inf|NaN|Infinity), budget (?P<budget>[\d.]+)$')
_CPU = re.compile(r'^(?:\S*Z )?model name\s*:\s*(.+?)\s*$', re.M)

def logged_costs(
    log, # What a speed test prints with `<NAME>_COSTS` set, such as the log of a job in the `Test` or `Speed` workflow
    platform='linux' # The platform whose native budgets the log's costs were checked against: 'linux' for CI's runner, or 'macos' for a Mac
):
    "The rows that `speed_costs` gives, read from `log`, with the CPU model that the log reports as their `cpu`, and `platform`. A case that the log times more than once, as after a retry, gives its lowest cost."
    cpu = c[1] if (c := _CPU.search(log)) else None
    rows = {}
    for m in filter(None, map(_LOGGED.match, log.splitlines())):
        code = re.sub(r'\\([\\#])', r'\1', m['code']) if m['tap'] else m['code'] # Node's TAP diagnostics escape `\` and `#`
        row = (m['section'], code, float(m['cost']), float(m['budget']))
        if row[:2] not in rows or row[2] < rows[row[:2]][2]: rows[row[:2]] = row
    return SpeedCosts(rows.values(), cpu, platform)

def _run_costs(cmd, path, timeout, native=False, env=None):
    "The rows that `logged_costs` reads from what `cmd` prints, run in the package at `path` with `<NAME>_COSTS` set. A case over its budget still gives its row."
    path = Path(path).expanduser()
    name = tomllib.loads((path/'Cargo.toml').read_text())['package']['name']
    p = subprocess.run(cmd, cwd=path, capture_output=True, text=True, timeout=timeout, env=os.environ | {f'{name.upper()}_COSTS': '1'} | (NATIVE if native else {}) | (env or {}))
    costs = logged_costs(p.stdout + p.stderr, _platform())
    if p.returncode and not costs: raise ValueError(p.stdout + p.stderr)
    return costs

def cargo_costs(
    path='.', # A Cargo package, such as basedpl or rustymath, whose `tests/speed.rs` prints every cost when `<NAME>_COSTS` is set
    native=False, # Time rustymath's own linear algebra in place of Accelerate, as `Worker`'s `native` does
    timeout:float=None # Seconds before the run stops
):
    "The rows that `logged_costs` reads from what the speed test of the package at `path` prints, run as `cargo test` runs it. A case over its budget still gives its row."
    return _run_costs(['cargo', 'test', '--test', 'speed'], path, timeout, native)

def pytest_costs(
    path='.', # A package, such as rustymath, whose `tests/test_speed.py` prints every cost when `<NAME>_COSTS` is set
    saved:Path=None, # A directory that holds a copy of the package's Python package, saved before a change, to time in place of the installed one
    native=False, # Time rustymath's own linear algebra in place of Accelerate, as `Worker`'s `native` does
    timeout:float=None # Seconds before the run stops
):
    "The rows that `logged_costs` reads from what the Python speed test of the package at `path` prints, run with `pytest`. A case over its budget still gives its row."
    env = dict(PYTHONPATH=os.pathsep.join(filter(None, [str(Path(saved).expanduser()), os.environ.get('PYTHONPATH')]))) if saved else {}
    return _run_costs([sys.executable, '-m', 'pytest', '-q', 'tests/test_speed.py'], path, timeout, native, env)

class Report(str):
    "What a test runner printed, displayed as its text."
    def __repr__(self): return str(self)

def wasm_test(
    wasm, # A directory that `cargo wasm --out` wrote
    speed=False, # Run `tests/wasm-speed.mjs`, with V8's `--no-liftoff` and `BASEDPL_COSTS` as CI runs it, in place of `tests/wasm.mjs`? Its report then lists every case's cost, which `logged_costs` reads
    timeout:float=None # Seconds before the run stops
):
    "Run `tests/wasm.mjs` or `tests/wasm-speed.mjs` under `node --test` against the browser build in `wasm`, as CI runs them, and give the runner's report. Only the `node` process gets `BASEDPL_WASM` and `BASEDPL_COSTS`. It holds the machine's timing lock, exclusively for the speed test and shared for the others. A failing test raises `ValueError` with the report, starting on a line of its own so that `logged_costs` reads every row of it."
    test = _SPEED.with_name('wasm-speed.mjs' if speed else 'wasm.mjs')
    with timing_lock(exclusive=speed):
        p = subprocess.run(['node', *(['--no-liftoff'] if speed else []), '--test', str(test)], cwd=_SPEED.parents[1], capture_output=True,
                           text=True, timeout=timeout, env=os.environ | dict(BASEDPL_WASM=str(Path(wasm).expanduser())) | (dict(BASEDPL_COSTS='1') if speed else {}))
    report = Report(p.stdout + p.stderr)
    if p.returncode: raise ValueError(f'\n{report}')
    return report

class Budgets(list):
    "Each budget that `set_budgets` changed, as `(section, code, build, old, new)` rows."
    def __repr__(self): return '\n'.join(f'{old:>9} -> {new:<9} {build:6}  {s} {c}' for s, c, build, old, new in self)

def _budget(section, expected):
    "The budget that `speed.txt`'s rule gives a case in `section` that should cost `expected`: twice it in copies, rounded up to two significant figures and at least 0.5, or 0.25 more than it in `[widths]`, rounded up to two decimal places."
    if section == 'widths': return Decimal(repr(expected + 0.25)).quantize(Decimal('0.01'), ROUND_CEILING)
    return Context(prec=2, rounding=ROUND_CEILING).create_decimal(repr(max(0.5, 2*expected)))

def set_budgets(
    expected, # Each case's expected cost, from a first-principles analysis for an initial budget or from a fuller understanding of the case later, as a dict from its code to the cost: in copies, or in `[widths]` as a fraction of the 64-bit time
    path=_SPEED, # The budget file to rewrite: basedpl's `tests/speed.txt`, or rustymath's `tests/speed.txt` or `tests/speed_python.txt`
    builds=None, # The budget columns to set, 'macos', 'linux' or 'wasm', or a list of them; None: every column the file has. Name a column only for a platform whose expected cost differs, as a comment above the case says
    sections=None # A section's name, or a list of names, whose cases to set; None: every section
):
    "Set the budgets of each case in `expected` under the rule in `path`'s header. A case outside `expected` or `sections`, and a column outside `builds`, keeps its budget. Returns the budgets that changed."
    if isinstance(builds, str): builds = [builds]
    if isinstance(sections, str): sections = [sections]
    for code, cost in expected.items():
        if not math.isfinite(cost) or cost < 0: raise ValueError(f'{code} should cost {cost}')
    path = Path(path)
    lines, section, changes = path.read_text().split('\n'), None, Budgets()
    width = min(len(_BUDGETS), max((l.count('\t') for l in lines if not l.startswith('#')), default=0))
    if builds and (unknown := set(builds) - set(_BUDGETS[:width])): raise ValueError(f'{path.name} has no budget column {", ".join(sorted(unknown))}')
    columns = [i for i, build in enumerate(_BUDGETS[:width], 1) if builds is None or build in builds]
    for k, line in enumerate(lines):
        if not line or line.startswith('#'): continue
        if m := re.fullmatch(r'\[([^\] ]+)\](?: .*)?', line):
            section = m[1]
            continue
        fields = line.split('\t')
        if section == 'setup' or (cost := expected.get(fields[0])) is None or sections is not None and section not in sections: continue
        fields += [''] * (width + 1 - len(fields))
        new = f'{_budget(section, cost).normalize():f}'
        for i in columns:
            if new != fields[i]: changes.append((section, fields[0], _BUDGETS[i-1], fields[i], new))
            fields[i] = new
        lines[k] = '\t'.join(fields)
    path.write_text('\n'.join(lines))
    return changes

_COMMANDS = 'check_file check_page time_cases instructions speed_costs cargo_costs pytest_costs wasm_test'.split()

def main():
    "Run a function in `_COMMANDS` from the shell, as `bpl-tests NAME ARGUMENT...`, and print its result. `bpl-tests NAME -h` lists NAME's arguments."
    if len(sys.argv) < 2 or sys.argv[1] not in _COMMANDS: sys.exit(f'usage: bpl-tests NAME ARGUMENT..., where NAME is one of {", ".join(_COMMANDS)}')
    name = sys.argv.pop(1)
    sys.argv[0] = f'bpl-tests {name}' # The program name in argparse's usage line
    f = globals()[name]
    args, kwargs, _ = parse_cli(f)
    print(repr(f(*args, **kwargs)))
