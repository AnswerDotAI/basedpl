"Read BPL test cases, convert reference inventories, activate reviewed cases, time BPL as the speed test does, and set the budgets of basedpl's and rustymath's speed tests."
import json, math, os, re, statistics, subprocess, sys, tempfile, time, tomllib
from concurrent.futures import ThreadPoolExecutor
from collections import defaultdict
from itertools import accumulate
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
FLAGS = 'unfused', 'batched', 'mixed'


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


def _checked(worker, kind, spec, sentinel):
    "The failures that `worker` gives for a `kind` request, `reference` or `page`, with `spec`, without the case that `sentinel` picks. A reply that doesn't report that case raises `ValueError`, so a worker that checked nothing, or passed everything, never reads as a pass."
    r = worker.request({kind: spec})
    if e := r['error']: raise ValueError(e['message'])
    if sum(map(sentinel, r['failures'])) != 1: raise ValueError("the worker didn't report the case that must fail, so its result can't be trusted")
    return [f for f in r['failures'] if not sentinel(f)]


def _failures(kind, specs, sentinel, bpl=None):
    "The failures that `_checked` gives for each of `specs`, each in a fresh worker running `bpl`, all at once. The workers run while this holds the timing lock shared, as the reference test does. Every worker closes before this returns or raises, which ends any request still running, as after a Ctrl-C."
    with timing_lock():
        workers = [Worker(bpl) for _ in specs]
        pool = ThreadPoolExecutor(len(workers))
        try: return [f for job in [pool.submit(_checked, w, kind, s, sentinel) for w, s in zip(workers, specs)] for f in job.result()]
        finally:
            for w in workers: w.close()
            pool.shutdown()


def check_file(
    path, # A `.bpl` reference file
    ids:Annotated[int|str, {'nargs':'*'}]=None, # Check only these cases, each named by its id or its header's line number
    bpl:Path=None # A native `bpl` to check, such as a copy saved before a change, as `Worker`'s `bpl` does; None: the installed one
):
    "Check the cases in `path` as `tests/reference.rs` does, in a fresh worker for each core that takes every n-th case, and return the failures in the file's order."
    if isinstance(ids, (int, str)): ids = [ids]
    source = Path(path).read_text()
    cases = _reference_cases(source)
    lines = [c['line'] for c in cases if ids is None or c['id'] in ids or c['line'] in ids]
    # An id that names no case goes to every worker, whose error names it.
    unknown = [i for i in ids or () if not any(i in (c['id'], c['line']) for c in cases)]
    text = source.rstrip('\n') + '\n\n⍝ bpltests-sentinel — The checker must report this case\n0   ⍝ 1\n'
    n = max(1, min(os.cpu_count(), len(lines)))
    specs = [dict(text=text, ids=[*lines[k::n], *unknown, 'bpltests-sentinel']) for k in range(n)]
    return sorted(_failures('reference', specs, lambda f: f.get('id') == 'bpltests-sentinel', bpl), key=lambda f: f['line'])


def check_page(
    path, # A `.qmd` page
    bpl:Path=None, # A native `bpl` to check, such as a copy saved before a change, as `Worker`'s `bpl` does; None: the installed one
    unbatched:bool=False # Run each example with the `batch` pref off
):
    "Run the BPL examples in `.qmd` page `path` in a fresh worker, as `tests/core.rs` does, and return the failures."
    text = Path(path).read_text().rstrip('\n') + '\n\n```bpl\n0 ⍝ 1\n```\n'
    line = text.count('\n') - 1
    return _failures('page', [dict(text=text, batch=False if unbatched else None)], lambda f: f.get('line') == line, bpl)

_ROOT = Path(__file__).parents[2]
_SPEED = _ROOT/'tests'/'speed.txt'
# The budgets on each case line of a budget file, after its code, in order: macOS on Apple silicon, Linux on CI's x86 runner, then the browser build where the file has one.
_BUDGETS = ('macos', 'linux', 'wasm')

def _platform():
    "This machine's platform, whose native budget column it checks, as `rustymath::timing::NATIVE_COLUMN` picks it."
    import rustymath
    return _BUDGETS[rustymath.timing().column]

_PKG = _SPEED.with_name('wasm-pkg.mjs')

class Costs(list):
    "Each case's cost, as `(code, picoseconds per item, copies)` rows. Copies is `None` where one copy of the data is too quick to time."
    def __repr__(self): return '\n'.join(f'{ps:>10,.0f} ps {_or_dash(copies, ".2f"):>9} copies  {code}' for code, ps, copies in self)

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

class Scaling(list):
    "Each case's time per item in picoseconds on one thread and on `threads` threads, its speedup, and the least speedup it must reach, as `(code, one, many, speedup, least)` rows. A case whose speedup is below its least is marked `slow`."
    def __init__(self, rows=(), threads=1):
        super().__init__(rows)
        self.threads = threads
    def __repr__(self):
        rows = (f'{one:>11,.0f} ps {many:>11,.0f} ps {speedup:>6.2f}x of {least:<4g} {"slow" if speedup < least else "":4}  {code}' for code, one, many, speedup, least in self)
        return '\n'.join([f'1 thread against {self.threads}', *rows])

_SCALING_SETUP = 'pm←¿[1024 1024]⍴0 ⋄ sq←¿[512 512]⍴0 ⋄ hq←sq+⍉sq ⋄ stk←¿[16384 8 8]⍴0 ⋄ big←¿4194304⍴0 ⋄ n←≢big'
_SCALING_CASES = ['pm+.×pm', '"svd"•decompose sq', '"eigen"•decompose hq', '"qr"•decompose⍤2 stk', '*big']
# On macOS, Accelerate computes these cases. Its GESDD and SYEVD run on one thread whatever its threading setting, as NumPy's calls into the same Accelerate show, so the SVD and Hermitian eigendecomposition expect no speedup, and fail only where threads slow them. Its 1024×1024 product gained 1.34 to 1.38 times on 8 or 18 threads of an M5 Max, where one thread already ran at about 480 GFLOP/s.
_ACCELERATED_LEAST = {'pm+.×pm': 1.2, '"svd"•decompose sq': 0.9, '"eigen"•decompose hq': 0.9}

def thread_scaling(
    threads:int=None, # Threads to compare with one; None: this machine's cores, at most 8
    least:float=1.5, # The least speedup that each case must reach, apart from `_ACCELERATED_LEAST`'s on macOS
    cases:str=None, # A case's code, or a list of them; None: a 1024×1024 product, a 512×512 SVD and Hermitian eigendecomposition, QR decompositions of a stack of 16,384 8×8 matrices, and `*` on 4,194,304 floats
    setup:str=_SCALING_SETUP, # BPL that defines the cases' data and its number of items `n`
    bpl:Path=None, # A native `bpl` to time, such as a copy saved before a change, as `Worker`'s `bpl` does; None: the installed one
    timeout:float=None, # Seconds before each timing stops
):
    "Time each case with `time_cases` on one thread and on `threads` threads, and give its speedup. Speed budgets are single-thread measurements, so this checks multithreaded speed instead: in the manual `Speed` workflow and locally, not in the `Test` workflow, where a runner's cores and sharing make it noisy. On macOS, the cases that Accelerate computes need only the speedups in `_ACCELERATED_LEAST`. Raises `ValueError` with the table when any case's speedup is below its least."
    threads, cases = threads or min(8, os.cpu_count() or 1), cases or _SCALING_CASES
    one, many = (time_cases(cases, setup, threads=t, bpl=bpl, timeout=timeout) for t in (1, threads))
    least_for = lambda code: min(least, _ACCELERATED_LEAST.get(code, least)) if sys.platform == 'darwin' else least
    result = Scaling(((code, a, b, a / b, least_for(code)) for (code, a, _), (_, b, _) in zip(one, many)), threads)
    if any(speedup < floor for *_, speedup, floor in result): raise ValueError(f'\n{result!r}')
    return result

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

def _program(code, n, setup):
    "BPL that runs `setup`, then `code` once with `N` set to `n`, on one thread, and ends without displaying its result. `setup` comes first so that a `•prefs` call at its start still applies before the program is parsed, which fixes each literal's float width."
    return f'•prefs ["threads":1]\n{setup}\nN←{n}\nr←{code}\n0'

# The instructions that a case retires before the lowest value of `N` that `instructions` picks, and between successive values. A case that allocates an array at each step retires a few tens of millions more instructions over its first steps, while the allocator settles. `{↑[⍵ 1]}⍣N⊢0` counts the same from 3×10⁴ steps on, about 2×10⁸ instructions, and `{fib 10}⍣N⊢0` from 30 steps on. Each process pays a different warm-up, up to about 5×10⁶ instructions, which moves a slope over 5×10⁸ instructions by under 1%. Some cases also pay a one-off cost once `N` passes a size: `≢•ucs N⍴"ab" "cde"` retires about 5×10⁶ more instructions from about 131,350 items on.
_SETTLE, _SPAN = 5e8, 5e8

# A case that names `N` outside its strings says itself how it repeats. `_stepped` runs any other case once per step.
_NAMES_N, _QUOTED = re.compile(r'(?<!\w)N(?!\w)'), re.compile(r'"(?:[^"]|"")*"|\'.\'')

def _stepped(case, force=False):
    "BPL that makes one call of `case` per step of `N`. A case that names `N`, such as `{⍵}⍣N⊢0`, is that BPL already. Any other case runs once per step, and its result is dropped, or with `force` computed in full through an indexed write. `force` needs a result of rank 1 or more."
    if _NAMES_N.search(_QUOTED.sub('', case)): return case
    return f'{{x←{case} ⋄ x.[0]←x.[0] ⋄ ⍵}}⍣N⊢0' if force else f'{{{case} ⋄ ⍵}}⍣N⊢0'

def instructions(
    cases:Annotated[str, {'nargs':'+'}], # A BPL expression to run once per step, such as `k(⌈/⊢)⌸a`, or one that names `N` and makes its own call per step of it, such as `{⍵}⍣N⊢0`, or a list of them
    setup='', # BPL that defines what the cases use, run before each case
    force=False, # Compute each step's result in full where it would stay pending, such as large windows or a transpose, for the cases that don't name `N`
    counts:Annotated[int, {'nargs':'+'}]=None, # Two or more values of `N`; None: four values past the allocator's warm-up, from a rough count at 1,000 steps
    runs=3, # Processes at each value of `N`, of which the lowest count is used
    bpl:Path=None # A native `bpl` to count, such as a copy saved before a change; None: the installed one
):
    "The instructions that each case retires per unit of `N`, on one thread. A case that doesn't name `N` runs once per step inside `{case ⋄ ⍵}⍣N⊢0`. That dfn's own call adds about 1,900 instructions per step. For each pair of successive values of `N`, the difference between fresh `bpl` processes, divided by the difference in `N`, cancels startup and `setup`. The result is the median of those slopes. The median drops a one-off cost that starts once `N` passes some size, because such a cost falls between only one pair. Unlike timings, instruction counts don't vary with load or code layout, so they show per-call changes of a few percent. A case that names `N` must make one call per step of `N`, as `⍣` does: a map such as `f¨N⍴0` calls a dfn once on all its cells, so it gives no per-call cost."
    cases = [cases] if isinstance(cases, str) else list(cases)
    def count(code, n, runs=runs): return min(_retired(_program(code, n, setup), bpl) for _ in range(runs))
    def steps(code):
        if counts: return counts
        rough = max(1, (count(code, 1_000, 1) - count(code, 0, 1)) / 1_000)
        # Each value is at least one more than the last, for a case that retires more than `_SPAN` instructions a step, whose warm-up is then a small share of a step.
        return list(accumulate((round((_SETTLE + i*_SPAN)/rough) for i in range(4)), lambda last, n: max(n, last + 1)))
    def per_step(code):
        ns = steps(code)
        cs = [count(code, n) for n in ns]
        return statistics.median((c1-c0)/(n1-n0) for n0, n1, c0, c1 in zip(ns, ns[1:], cs, cs[1:]))
    return Instructions((case, per_step(_stepped(case, force))) for case in cases)

class ProfileDiff(list):
    "Each function's estimated instructions per step of `N` on two builds, as `(function, before, after)` rows, largest change first, below a row of each build's total."
    def __init__(self, rows=(), totals=(0, 0)):
        super().__init__(rows)
        self.totals = totals
    def __repr__(self): return '\n'.join(f'{b:>10,.0f} {a:>10,.0f} {a-b:>+10,.0f}  {f[:150]}' for f, b, a in [('total', *self.totals), *self])

# A frame in the call graph that macOS `sample` reports: the tree's drawing characters, whose width gives the frame's depth, then its samples and its symbol.
_FRAME, _THREAD = re.compile(r'^([ +!:|]*)(\d+) (\S.*?)  [\[(]'), re.compile(r'^\s*\d+ Thread_')

def _self_samples(code, bpl, seconds):
    "The samples at which each function was at the top of the main thread's stack while a fresh `bpl` process ran `code`, as macOS `sample` counts them over `seconds`, with demangled names, and the thread's total. `code` must run for longer than half a second plus `seconds`. The process is then killed."
    p = subprocess.Popen([bpl or INSTALLED_BPL, '-e', code], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    try:
        time.sleep(0.5)
        if p.poll() is not None: raise ValueError(p.stderr.read())
        with tempfile.NamedTemporaryFile('r', suffix='.txt') as report:
            s = subprocess.run(['sample', str(p.pid), str(seconds), '-file', report.name], capture_output=True, text=True)
            if s.returncode: raise ValueError(s.stdout + s.stderr)
            lines = report.read().splitlines()
    finally: p.kill(); p.wait()
    start = next(i for i, line in enumerate(lines) if 'com.apple.main-thread' in line)
    total, selfs, stack = int(lines[start].split()[0]), defaultdict(int), []
    def pop():
        _, symbol, n, below = stack.pop()
        selfs[symbol] += n - below
    for line in lines[start+1:]:
        if not line.strip() or _THREAD.match(line): break
        if not (m := _FRAME.match(line)): continue
        depth, n = len(m[1]), int(m[2])
        while stack and stack[-1][0] >= depth: pop()
        if stack: stack[-1][3] += n
        stack.append([depth, m[3], n, 0])
    while stack: pop()
    named = defaultdict(int)
    for name, n in zip(subprocess.run(['c++filt'], input='\n'.join(selfs), capture_output=True, text=True).stdout.splitlines(), selfs.values()): named[name] += n
    return named, total

def _estimates(case, setup, bpl, seconds, force):
    "Each function's estimated instructions per step of `N` for `case` on `bpl`, and the build's instructions per step, as `profile` describes."
    if sys.platform != 'darwin': raise ValueError(f'no sampling profiler on {sys.platform}')
    stepped = _stepped(case, force)
    [(_, per_step)] = instructions(stepped, setup, bpl=bpl)
    samples, total = _self_samples(_program(f'{{{stepped} ⋄ ⍵}}⍣1000000000⊢0', max(1, round(1e8/per_step)), setup), bpl, seconds)
    return {f: per_step*n/total for f, n in samples.items()}, per_step

class Profile(list):
    "Each function's estimated instructions per step of `N` on one build, as `(function, instructions)` rows, largest first, below a row of the build's total."
    def __init__(self, rows=(), total=0):
        super().__init__(rows)
        self.total = total
    def __repr__(self): return '\n'.join(f'{n:>10,.0f} {n/self.total:>6.1%}  {f[:150]}' for f, n in [('total', self.total), *self])

def profile(
    case, # A BPL expression, as `instructions` takes it, such as `k(⌈/⊢)⌸a` or `{⍵}⍣N⊢0`
    setup='', # BPL that defines what the case uses, run before it
    force=False, # Compute each step's result in full, as `instructions`' `force` does
    bpl:Path=None, # A native `bpl` to profile, such as a copy saved before a change; None: the installed one
    seconds=3, # How long macOS `sample` watches the build
    rows=20 # How many functions to show
):
    "Where `case`'s instructions per step of `N` go, by function, on macOS. A fresh process repeats the case on one thread while `sample` counts how often each function is at the top of the main thread's stack. A function's estimate is its share of those samples times the build's instructions per step from `instructions`. The shares come from time, so the estimates assume every function retires instructions at the same rate."
    estimates, total = _estimates(case, setup, bpl, seconds, force)
    return Profile(sorted(estimates.items(), key=lambda r: -r[1])[:rows], total)

def profile_diff(
    case, # A BPL expression, as `instructions` takes it, such as `k(⌈/⊢)⌸a` or `{⍵}⍣N⊢0`
    base:Path, # A native `bpl` to compare with, such as a copy saved before a change
    setup='', # BPL that defines what the case uses, run before it
    force=False, # Compute each step's result in full, as `instructions`' `force` does
    bpl:Path=None, # A native `bpl` to compare with `base`; None: the installed one
    seconds=3, # How long macOS `sample` watches each build
    rows=20 # How many functions to show
):
    "Where `case`'s instructions per step of `N` moved between two builds, by function, on macOS, from `profile`'s estimates for each build. A function that only one build calls out of line, such as one that stopped being inlined, appears in that build alone."
    (before, tb), (after, ta) = _estimates(case, setup, base, seconds, force), _estimates(case, setup, bpl, seconds, force)
    fs = sorted(before.keys() | after.keys(), key=lambda f: -abs(after.get(f, 0) - before.get(f, 0)))
    return ProfileDiff([(f, before.get(f, 0), after.get(f, 0)) for f in fs[:rows]], (tb, ta))

# basedpl's library and the rustymath library it links, built as CI's Linux runner compiles them. Apple's `clang` assembles the x86-64 source in basedpl's `psm` dependency, because this Mac has no `x86_64-linux-gnu-gcc`. Incremental compilation is off, as on CI, so that code splits into the same codegen units.
_X86_BUILD = 'cargo build --lib --release --target x86_64-unknown-linux-gnu --target-dir /tmp/aai-ws-x86 -p basedpl --message-format=json-render-diagnostics'.split()
_X86_ENV = dict(CARGO_INCREMENTAL='0', CC_x86_64_unknown_linux_gnu='clang')
# A line of `nm`'s listing: an address, or spaces for an undefined symbol, then the symbol's type letter and its name.
_NM_LINE = re.compile(r'^(?:[0-9a-f]+| +) (\S) (.+)$')
# A mangled Rust name in `objdump`'s output, in the v0 scheme or the legacy one, with Mach-O's extra leading underscore where it has one.
_MANGLED = re.compile(r'(?<![\w$.])_?(?:_R|_ZN)[\w$.]+')

def _x86_libraries():
    "The libraries that `_X86_BUILD` makes, after running it with the timing lock held shared, and a note that's empty unless basedpl failed to compile. Cargo builds rustymath first, so a failed build still gives rustymath's library."
    with timing_lock():
        p = subprocess.run(_X86_BUILD, cwd=_ROOT, capture_output=True, text=True, env=os.environ | _X86_ENV)
    built = map(json.loads, p.stdout.splitlines())
    files = [f for m in built if m['reason'] == 'compiler-artifact' and m['target']['name'] in ('basedpl', 'rustymath') for f in m['filenames'] if f.endswith('.rlib')]
    if p.returncode and not files: raise ValueError(p.stderr)
    return files, "basedpl doesn't compile, so only rustymath's library was read.\n" if p.returncode else ''

def _symbols(files):
    "Each symbol in `files`, as its `nm` type letter, its mangled name and its demangled name."
    def listing(*flags): return [_NM_LINE.match(line) for line in subprocess.run(['nm', '-p', *flags, *files], capture_output=True, text=True).stdout.splitlines()]
    return [(m[1], m[2], d[2]) for m, d in zip(listing(), listing('-C')) if m and d]

def disasm(
    pattern, # A regex that the function's demangled name matches, such as `reduce::sliding::<f64, rustymath::reduce::Max>`
    x86=False, # Read the x86-64 code that CI and Linux run, in place of this Mac's
    bpl:Path=None # A native `bpl` to read, such as a copy saved before a change; None: the installed one. With `x86`, unused
):
    "The machine code of the function whose demangled name matches `pattern`, as `objdump` shows it, with names demangled. Where functions of more than one name match, it gives their names. With `x86`, it first builds basedpl's library and the rustymath library it links for Linux x86-64, as CI's runner compiles them, holding the timing lock shared. Where basedpl doesn't compile, it reads rustymath's library alone and says so first. After a change, that build takes minutes."
    files, note = _x86_libraries() if x86 else ([Path(bpl or INSTALLED_BPL).expanduser()], '')
    symbols = _symbols(files)
    if not symbols: raise ValueError(f'nm read no symbols from {files}')
    matched = sorted({d for t, _, d in symbols if t in 'tT' and re.search(pattern, d)})
    if not matched: raise ValueError(f'{note}no function matches {pattern!r}. An inlined function has no code of its own. Search for its caller.')
    if len(matched) > 1: return Report(f'{note}{len(matched)} functions match {pattern!r}:\n' + '\n'.join(matched[:40]))
    wanted = list(dict.fromkeys(m for t, m, d in symbols if t in 'tT' and d == matched[0]))
    p = subprocess.run(['objdump', '-d', '-r', '--no-show-raw-insn', f'--disassemble-symbols={",".join(wanted)}', *files], capture_output=True, text=True)
    if not (code := re.findall(r'^[0-9a-f]+ <.+>:\n(?:.+\n)*', p.stdout, re.M)): raise ValueError(p.stderr)
    # A branch within the function shows only its target's offset.
    code = re.sub(rf'<(?:{"|".join(map(re.escape, wanted))})\+', '<+', '\n'.join(code))
    names = {m: d for _, m, d in symbols}
    return Report(note + _MANGLED.sub(lambda m: names.get(m[0], m[0]), code))

_CALLS = _SPEED.with_name('instructions.txt')

# How far a case's instructions may be from its count, as a share of the count, before `instruction_budgets` marks it.
MARGIN = 0.03

def _calls(text):
    "The setup and the `(code, count)` cases of `text`, in `instructions.txt`'s format. Lines that start with `#` are comments. The lines after `[setup]` define what the cases use, and each line after `[calls]` holds a case's code and its count, separated by a tab."
    setup, cases, section = [], [], None
    for line in text.splitlines():
        if not line.strip() or line.startswith('#'): continue
        if line in ('[setup]', '[calls]'): section = line
        elif section == '[setup]': setup.append(line)
        elif section == '[calls]':
            code, count = line.rsplit('\t', 1)
            cases.append((code, int(count)))
        else: raise ValueError(f'a line before any section: {line}')
    return '\n'.join(setup), cases

def _mark(n, count): return 'over' if n > count*(1+MARGIN) else 'stale' if n < count*(1-MARGIN) else ''

class InstructionBudgets(list):
    "Each case's instructions per unit of `N` and its count, as `(code, instructions, count)` rows. A case more than `MARGIN` above its count is `over`. One more than `MARGIN` below is `stale`: its count should come down to it."
    def __repr__(self): return '\n'.join(f'{n:>12,.0f} {count:>12,} {_mark(n, count):5}  {code}' for code, n, count in self)
    @property
    def outside(self): return [row for row in self if _mark(*row[1:])]

def instruction_budgets(
    calls:str=None, # Text in `instructions.txt`'s format; None: `tests/instructions.txt`
    bpl:Path=None # A native `bpl` to count, such as a copy saved before a change; None: the installed one
):
    "Count each case in `calls` with `instructions`, beside its count."
    setup, cases = _calls(_CALLS.read_text() if calls is None else calls)
    counted = instructions([code for code, _ in cases], setup, bpl=bpl)
    return InstructionBudgets((code, n, count) for (code, n), (_, count) in zip(counted, cases))

def _or_dash(x, spec): return '-' if x is None else format(x, spec)
def _over(cost, budget): return budget is not None and cost > budget

class SpeedCosts(list):
    "Each case's cost and the budget that limits it, as `(section, code, cost, budget, ns)` rows. A cost is in copies, except in `[widths]`, where it is the 32-bit time as a fraction of the 64-bit time. A case with no budget has `None`. `ns` is the case's time for each item in nanoseconds, or `None` where the timing doesn't give it. Times compare across a change to `rustymath::timing::COPIES`, where costs don't. A case over its budget needs study, which finds either a speed bug or something its expected cost missed. `cpu` is the CPU model that a CI log reports, or `None`. `platform` is the platform whose native budgets the costs were checked against, 'macos' or 'linux', or `None` for a browser build. `copies` holds the copy lines that a log gives, as `(items, bytes, ns per item, stored)` rows, which say whether `rustymath::timing::COPIES` held that machine's copy time or the test measured it."
    def __init__(self, rows=(), cpu=None, platform=None, copies=()):
        super().__init__(rows)
        self.cpu, self.platform, self.copies = cpu, platform, list(copies)
    def __repr__(self): return '\n'.join(f'{cost:>9.2f} {_or_dash(b, "g"):>9} {_or_dash(ns, ".3f"):>9} {"over" if _over(cost, b) else "":4}  {s} {c}' for s, c, cost, b, ns in self)

def speed_costs(
    wasm:Path=None, # A directory that `cargo wasm --out` wrote, to time in that browser build, as `tests/wasm-speed.mjs` does; None: a fresh native worker, as `tests/speed.rs` does
    native=False, # Time rustymath's own linear algebra in place of Accelerate in a native timing, as `Worker`'s `native` does
    bpl:Path=None, # A native `bpl` to time, such as a copy saved before a change, as `Worker`'s `bpl` does; None: the installed one
    speed:str=None, # Text in `speed.txt`'s format; None: `tests/speed.txt`
    prefs='', # BPL that runs before each section's header, such as `•prefs ["kinds":"static"]`
    timeout:float=None, # Seconds before the timing stops
    cases:Annotated[str, {'nargs':'*', 'opt':False}]=None # A case's code, or a list of codes, to time in every section that has it; None or empty: every case
):
    "Time every case in `speed` as the speed test does, with the budget that limits it in that build."
    speed = _SPEED.read_text() if speed is None else speed
    cases = [cases] if isinstance(cases, str) else cases or None
    if wasm is None:
        with Worker(bpl, native) as w: r = w.request(dict(costs=dict(speed=speed, cases=cases, prefs=prefs)), timeout)
        if e := r['error']: raise ValueError(e['message'])
        rows = r['costs']
    else:
        script = (f'import {{ costs }} from {json.dumps(_PKG.as_uri())}; import {{ readFileSync }} from "node:fs"; '
                  f'process.stdout.write(JSON.stringify(costs(readFileSync(0, "utf8"), {json.dumps(cases)}, {json.dumps(prefs)})))')
        with timing_lock(exclusive=True):
            p = subprocess.run(['node', '--no-liftoff', '--input-type=module', '-e', script], input=speed, capture_output=True, text=True,
                               timeout=timeout, env=os.environ | dict(BASEDPL_WASM=str(wasm)))
        if p.returncode: raise ValueError(p.stderr)
        if p.stderr: sys.stderr.write(p.stderr)
        rows = json.loads(p.stdout)
    build = _platform() if wasm is None else 'wasm'
    i = _BUDGETS.index(build)
    rows = ((c['section'], c['code'], c['cost'], float(c['budgets'][i]) if len(c['budgets']) > i else None, c['seconds']*1e9 if 'seconds' in c else None) for c in rows) # A saved `bpl` from before 23 October 2026 gives no `seconds`
    return SpeedCosts(rows, platform=None if wasm else build)

_LOGGED = re.compile(r'^(?:\S*Z )?(?P<tap># )?(?P<section>\w+) (?P<code>\S.*): (?P<cost>[\d.]+|inf|NaN|Infinity), budget (?P<budget>[\d.]+)(?:, (?P<ns>[\d.]+) ns)?$')
_COPY = re.compile(r'^(?:\S*Z )?(?:# )?copy of (?P<items>\d+) (?P<bytes>\d+)-byte items: (?P<ns>[\d.]+) ns an item, (?P<source>stored for|measured on) (?P<cpu>[^\r\n]+?)(?:: add it to rustymath::timing::COPIES)?[ \t]*$', re.M)

def logged_costs(
    log, # What a speed test prints with `<NAME>_COSTS` set, such as the log of a job in the `Test` or `Speed` workflow, or the path of a file that holds it
    platform='linux' # The platform whose native budgets the log's costs were checked against: 'linux' for CI's runner, or 'macos' for a Mac
):
    "The rows that `speed_costs` gives, read from `log`, with the CPU that its copy lines name as their `cpu`, `platform`, and the log's copy lines as `copies`. A case that the log times more than once, as after a retry, gives its lowest cost, with that timing's time for each item where the log prints one."
    if isinstance(log, Path): log = log.read_text()
    found = list(_COPY.finditer(log))
    cpu = next((m['cpu'] for m in found if m['cpu'] != 'a CPU with no name'), None)
    rows = {}
    for m in filter(None, map(_LOGGED.match, log.splitlines())):
        code = re.sub(r'\\([\\#])', r'\1', m['code']) if m['tap'] else m['code'] # Node's TAP diagnostics escape `\` and `#`
        row = (m['section'], code, float(m['cost']), float(m['budget']), m['ns'] and float(m['ns']))
        if row[:2] not in rows or row[2] < rows[row[:2]][2]: rows[row[:2]] = row
    copies = list(dict.fromkeys((int(m['items']), int(m['bytes']), float(m['ns']), m['source'] == 'stored for') for m in found))
    return SpeedCosts(rows.values(), cpu, platform, copies)

CHANGE = 0.2
class CostChanges(list):
    "The cases that `compare_costs` reports, as `(section, code, before, after, budget, change)` rows. `before` and `after` are costs, or `None` for a case that `time_cases` found too quick to time. `change` is the fractional change in the case's time for each item where both runs give one, and in its cost otherwise, so it holds across a change to `rustymath::timing::COPIES`. Cases over their budget in `after` come first, then cases whose change is more than `CHANGE`, largest first. `before` and `change` are `None` for a case that the earlier run didn't time, and `budget` is `None` for a case with no budget."
    def __repr__(self): return '\n'.join(f'{_or_dash(b, ".2f"):>9} {_or_dash(a, ".2f"):>9} {_or_dash(d, "+.0%"):>6} {_or_dash(budget, "g"):>9} {"over" if _over(a, budget) else "":4}  {s + " " if s else ""}{c}' for s, c, b, a, budget, d in self)

def _comparable(x, platform):
    "`x`, in any of `compare_costs`'s forms, as `SpeedCosts`. `time_cases`'s rows have no section or budget."
    if isinstance(x, Costs): return SpeedCosts((None, code, copies, None, ps/1e3) for code, ps, copies in x)
    return x if isinstance(x, SpeedCosts) else logged_costs(x, platform)

def compare_costs(
    before:Annotated[SpeedCosts|Costs|str|Path, {'type':Path}], # The earlier run: rows that `speed_costs`, `logged_costs`, `cargo_costs`, `pytest_costs` or `time_cases` give, or a log that `logged_costs` reads, as its text or its path
    after:Annotated[SpeedCosts|Costs|str|Path, {'type':Path}], # The later run, in any of `before`'s forms
    platform='linux' # The platform whose native budgets a log's costs were checked against, as `logged_costs` takes it
):
    "Each case over its budget in `after`, and each case whose time for each item moved by more than `CHANGE` from `before` in either direction, or whose cost did where either run doesn't give the time."
    before, after = (_comparable(x, platform) for x in (before, after))
    old = {(s, c): (cost, ns) for s, c, cost, _, ns in before}
    rows = []
    for s, c, cost, budget, ns in after:
        cost0, ns0 = old.get((s, c), (None, None))
        rows.append((s, c, cost0, cost, budget, ns/ns0 - 1 if ns and ns0 else cost/cost0 - 1 if cost0 else None))
    def moved(row): return abs(row[5] or 0)
    return CostChanges(sorted((r for r in rows if _over(r[3], r[4]) or moved(r) > CHANGE), key=lambda r: (not _over(r[3], r[4]), -moved(r))))

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
    "Text displayed as it is, such as a test runner's report or a function's machine code."
    def __repr__(self): return str(self)

def wasm_test(
    wasm, # A directory that `cargo wasm --out` wrote
    speed=False, # Run `tests/wasm-speed.mjs`, with V8's `--no-liftoff` and `BASEDPL_COSTS` as CI runs it, in place of `tests/wasm.mjs`? Its report then lists every case's cost, which `logged_costs` reads
    timeout:float=None # Seconds before the run stops
):
    "Run `tests/wasm.mjs` or `tests/wasm-speed.mjs` under `node --test` against the browser build in `wasm`, as CI runs them, and give the runner's report. Only the `node` process gets `BASEDPL_WASM` and `BASEDPL_COSTS`. It holds the machine's timing lock, exclusively for the speed test and shared for the others. A failing test raises `ValueError` with the report, starting on a line of its own so that `logged_costs` reads every row of it."
    test = _SPEED.with_name('wasm-speed.mjs' if speed else 'wasm.mjs')
    with timing_lock(exclusive=speed):
        p = subprocess.run(['node', *(['--no-liftoff'] if speed else []), '--test', str(test)], cwd=_ROOT, capture_output=True,
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

_COMMANDS = 'check_file check_page time_cases instructions instruction_budgets profile profile_diff disasm speed_costs thread_scaling cargo_costs pytest_costs compare_costs wasm_test'.split()

def main():
    "Run a function in `_COMMANDS` from the shell, as `bpl-tests NAME ARGUMENT...`, and print its result. `bpl-tests NAME -h` lists NAME's arguments."
    if len(sys.argv) < 2 or sys.argv[1] not in _COMMANDS: sys.exit(f'usage: bpl-tests NAME ARGUMENT..., where NAME is one of {", ".join(_COMMANDS)}')
    name = sys.argv.pop(1)
    sys.argv[0] = f'bpl-tests {name}' # The program name in argparse's usage line
    f = globals()[name]
    args, kwargs, _ = parse_cli(f)
    print(repr(f(*args, **kwargs)))
