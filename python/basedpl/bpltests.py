"Read BPL test cases, convert reference inventories, activate reviewed cases, time BPL as the speed test does, and set the budgets of basedpl's and rustymath's speed tests."
import difflib, json, math, os, re, statistics, subprocess, sys, tempfile, time, tomllib
from concurrent.futures import ThreadPoolExecutor
from collections import defaultdict
from itertools import accumulate
from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import Annotated
from fastws import store
from fastws.cargo import run_tests
from fastws.timelock import timing_lock
from fastws.procs import JOB_ENV, bounded, kill_group
from fastcore.script import parse_cli
from fastcore.basics import listify
from fastcore.meta import delegates
from ._core import _captured_literal, _reference_file, _retry_pauses, _budget, _budget_lines, _budget_columns as _BUDGETS
from .reference import Corpus, _dyalog_string
from . import BplError, _value
from .worker import LIMIT, Limit, Worker, environment, fresh, resolved, _commented, _installed
from .syntax import linear

SEPARATOR = '⍝ =>'
INPUT = '⍝ input:'
OUTPUT = '⍝ ⎕:'
ERROR = '⍝ error: '

# The header options that set a flag on a case, as Rust's `reference::FLAGS` names them.
FLAGS = 'unfused', 'batched', 'mixed', 'compiled'


@dataclass
class Case:
    code: str
    expect: str | None = None
    id: str = ''
    comment: str = ''
    rtol: float = 0
    atol: float = 0
    unfused: bool = False
    batched: bool = False
    mixed: bool = False
    compiled: bool = False
    line: int = field(default=0, compare=False, repr=False)
    section: str = ''
    output: str | None = None
    input: str | None = None
    prefs: str | None = None
    budgets: tuple = ()


def _escaped(text): return text.replace('\\', '\\\\').replace('\n', '\\n')


def parse(text):
    "The records of `text`, as the shared native BPL parser reads them. Malformed files raise `ValueError`."
    return [Case(code=c['code'], expect=c.get('expected_code', ERROR+c['expected_error'] if 'expected_error' in c else None), id=c['id'], comment=c['comment'],
                 rtol=c.get('relative_tolerance', 0), atol=c.get('absolute_tolerance', 0), **{key: c.get(key, False) for key in FLAGS}, line=c['line'],
                 section=c['section'], output=c.get('expected_output'), input=c.get('input'), prefs=c.get('prefs'), budgets=tuple(c.get('budgets', ()))) for c in _reference_file(text)['cases']]


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
        if case.expect is None or case.budgets: raise ValueError('edit timed files in place, not through render')
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


_STALL = 60 # Seconds that a check's worker may start no case or example before it counts as stuck, as a deadlock leaves it: a case runs at most a few evaluations of `reference::TIMEOUT`, 2 s each

def _checked(worker, kind, spec, sentinel, bpl, limit):
    "The failures that `worker` gives for a `kind` request, `reference` or `page`, with `spec`, without the case that `sentinel` picks. A reply that doesn't report that case raises `ValueError`, so a worker that checked nothing, or passed everything, never reads as a pass. A reference worker notes each case on stderr as it starts it (`case LINE`), and a page worker each example (`example LINE`). Where an abort, such as a stack overflow, ends the worker, or the worker starts nothing for `_STALL` seconds, the last noted case fails as `crashed` or `timeout`, and the cases after it run in a fresh worker of `bpl` while `limit` has time left. A stuck worker is closed, so no worker outlives the timing lock's hold (`_failures`). Each request gets what's left of `limit`: where it runs out, the worker stops and gives the next case or example as `unchecked`, and the sentinel goes unchecked too."
    try: r = worker.request({kind: spec}, timeout=limit.left(), stall=_STALL)
    except (EOFError, TimeoutError) as e:
        started = re.findall(r'^case (\d+)$', worker.diagnostics, re.M)
        if kind != 'reference' or not started: raise
        line, stuck = int(started[-1]), isinstance(e, TimeoutError)
        tail = worker.diagnostics.rsplit(f'case {line}\n', 1)[-1].strip()
        rest = [i for i in spec['ids'] if not isinstance(i, int) or i > line]
        try: limit.left()
        except TimeoutError: rest = None
        message = (str(e) if stuck else 'the worker aborted') + (f': {tail}' if tail else '') + ('; the call reached its limit, so the cases after it are unchecked' if rest is None else '')
        failed = dict(id='', line=line, section='', comment='', code='', expected_code='', status='timeout' if stuck else 'crashed', message=message, actual=None)
        if rest is None: return [failed]
        with Worker(bpl) as w: return [failed, *_checked(w, kind, dict(spec, ids=rest), sentinel, bpl, limit)]
    if e := r['error']: raise ValueError(e['message'])
    stopped = any(f.get('status') == 'unchecked' for f in r['failures'])
    if not stopped and sum(map(sentinel, r['failures'])) != 1: raise ValueError("the worker didn't report the case that must fail, so its result can't be trusted")
    return [f for f in r['failures'] if not sentinel(f)]


def _failures(kind, specs, sentinel, build=None, timeout=LIMIT):
    "The failures that `_checked` gives for each of `specs`, each in a fresh worker, all at once, every one running the same build, resolved once. The workers run while this holds the timing lock shared, as the reference test does, and share `timeout`'s limit (`worker.Limit`), counted while this holds the lock. Every worker closes before this returns or raises, which ends any request still running, as after a Ctrl-C. Each failure holds its code's linear form (`syntax.linear`) as `linear`."
    bpl, limit = resolved(build), Limit.of(timeout)
    with timing_lock(), limit.running():
        workers = [Worker(bpl) for _ in specs]
        pool = ThreadPoolExecutor(len(workers))
        try: return [f | {'linear': linear(f['code'])} for job in [pool.submit(_checked, w, kind, s, sentinel, bpl, limit) for w, s in zip(workers, specs)] for f in job.result()]
        finally:
            for w in workers: w.close()
            pool.shutdown()


def check_file(
    path, # A `.bpl` reference file
    ids:Annotated[int|str, {'nargs':'*'}]=None, # Check only these cases: one, or a list, each named by its id or by its header's line number, as an `int` or a string of digits
    build:str=None, # The build to check, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build
    timeout:float=LIMIT, # Seconds of work that the check gets, or a `Limit` to share (`worker.Limit`); None: no limit
):
    "Check the cases in `path` as `tests/reference.rs` does, and their route directives as the route test does, in a fresh worker for each core that takes every n-th case, and return the failures in the file's order."
    ids = None if ids is None else [int(i) if isinstance(i, str) and i.isdigit() else i for i in ([ids] if isinstance(ids, (int, str)) else ids)]
    source = Path(path).read_text()
    cases = _reference_file(source)['cases']
    lines = [c['line'] for c in cases if ids is None or c['id'] in ids or c['line'] in ids]
    # An id that names no case goes to every worker, whose error names it.
    unknown = [i for i in ids or () if not any(i in (c['id'], c['line']) for c in cases)]
    text = source.rstrip('\n') + '\n\n⍝ bpltests-sentinel — The checker must report this case\n0   ⍝ 1\n'
    n = max(1, min(os.cpu_count(), len(lines)))
    specs = [dict(text=text, ids=[*lines[k::n], *unknown, 'bpltests-sentinel']) for k in range(n)]
    return sorted(_failures('reference', specs, lambda f: f.get('id') == 'bpltests-sentinel', build, timeout), key=lambda f: f['line'])


def check_page(
    path, # A `.qmd` page
    build:str=None, # The build to check, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build,
    unbatched:bool=False, # Run each example with the `batch` pref off
    timeout:float=LIMIT # Seconds of work that the check gets, or a `Limit` to share (`worker.Limit`); None: no limit
):
    "Run the BPL examples in `.qmd` page `path` in a fresh worker, as `tests/core.rs` does, and return the failures."
    text = Path(path).read_text().rstrip('\n') + '\n\n```bpl\n0 ⍝ 1\n```\n'
    line = text.count('\n') - 1
    return _failures('page', [dict(text=text, batch=False if unbatched else None)], lambda f: f.get('line') == line, build, timeout)

_ROOT = Path(__file__).parents[2]
_SPEED = _ROOT/'tests'/'speed'

def _platform():
    "This machine's platform, whose native budget column its speed tests compare costs with, as `rustymath::timing::NATIVE_COLUMN` picks it, or `None` on a machine with no budgets, such as Linux on Arm."
    import rustymath
    column = rustymath.timing().column
    return None if column is None else _BUDGETS[column]

_PKG = _SPEED.parent/'wasm-pkg.mjs'

class Costs(list):
    "Each case's cost, as `(code, picoseconds per item, copies)` rows. Copies is `None` where one copy of the data is too quick to time. Each row displays its cost, then its code and the code's linear form (`syntax.linear`)."
    def __repr__(self): return '\n'.join(f'{ps:>10,.0f} ps {_or_dash(copies, ".2f"):>9} copies  {_commented(code)}' for code, ps, copies in self)

class ByBuild(list):
    "Each build's rows, as `time_cases` or `instructions` gives them for one build, in the order of `builds`, where `None` is the installed `bpl`. The display shows each case's value on each build, in `unit`, and its change from the first build to the last, then the case and its linear form (`syntax.linear`). `compare_costs(*rows)` compares two builds' `Costs`."
    def __init__(self, rows=(), builds=(), unit=''):
        super().__init__(rows)
        self.builds, self.unit = builds, unit
    def __repr__(self):
        head = ''.join(f'{Path(b).name if b else "installed":>13}' for b in self.builds) + f' {"change":>7}'
        w = 13 - len(self.unit)
        return '\n'.join([head, *(''.join(f'{r[1]:>{w},.0f}{self.unit}' for r in rs) + f' {rs[-1][1]/rs[0][1]-1:>+7.1%}  {_commented(rs[0][0])}' for rs in zip(*self))])

@delegates(environment)
def time_cases(
    cases:Annotated[str, {'nargs':'+'}], # A BPL expression to time, or a list of them
    setup:str=None, # BPL that defines the data and its number of items `n`; None: the setup in tests/speed/arrays.bpl
    prefs='', # BPL that runs before `setup`, such as `•prefs ["float":32]`
    section:str=None, # A speed file's section, as `file:section` such as `models.bpl:static`: the file's setup in place of `setup`, and the section's preferences after `prefs`, as the speed test runs them
    threads=1, # Threads that each case's rustymath calls split across, for a scaling check; 0: rustymath's default
    build:Annotated[str, {'nargs':'+'}]=None, # The build to time, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build; or a list of builds to compare
    rounds=2, # The fresh workers that each build gets, taking turns with any other builds' in alternating order
    timeout:float=LIMIT, # Seconds of work that the call gets across its builds and rounds, or a `Limit` to share (`worker.Limit`), counted from when each worker holds the timing lock; None: no limit
    **kwargs
):
    "Time `cases` in fresh workers, as `tests/speed.rs` does: in each worker the fastest of five runs, per item and in copies of the data, and of those the fastest over `rounds` workers, since a large case's time varies between processes by more than within one. Given several builds, it gives `ByBuild`, holding each case's fastest time on each build, and resolves each build once, as the comparison starts."
    if section:
        name, _, part = section.partition(':')
        speed = _reference_file((_SPEED/name).read_text())
        header = next((c.get('prefs', '') for c in speed['cases'] if c['section'] == part), None)
        if header is None: raise ValueError(f'tests/speed/{name} has no section {part!r}')
        setup, prefs = speed['setup'], '\n'.join([prefs, header])
    setup = _reference_file((_SPEED/'arrays.bpl').read_text())['setup'] if setup is None else setup
    cases = listify(cases)
    limit = Limit.of(timeout)
    def timed(build):
        with Worker(build, **kwargs) as w:
            r = w.request(dict(timing=dict(setup=setup, prefs=prefs, cases=cases, threads=threads)), limit.left())
            limit.spend(w.ran)
        if e := r['error']:
            limit.reached(e)
            raise BplError(e, r['output']) if 'display' in e else ValueError(e['message'])
        return Costs((c['code'], c['seconds']*1e12/r['n'], c['copies']) for c in r['costs'])
    builds = listify(build) or [None]
    runs, paths = [[] for _ in builds], [resolved(b) for b in builds]
    for r in range(rounds):
        for i in (range(len(builds)) if r % 2 == 0 else reversed(range(len(builds)))): runs[i].append(timed(paths[i]))
    fastest = [Costs(min(rows, key=lambda row: row[1]) for rows in zip(*each)) for each in runs]
    return fastest[0] if len(builds) == 1 else ByBuild(fastest, builds, ' ps')

class Scaling(list):
    "Each case's time per item in picoseconds on one thread and on `threads` threads, its speedup, and the least speedup it must reach, as `(code, one, many, speedup, least)` rows. A case whose speedup is below its least is marked `slow`."
    def __init__(self, rows=(), threads=1):
        super().__init__(rows)
        self.threads = threads
    def __repr__(self):
        rows = (f'{one:>11,.0f} ps {many:>11,.0f} ps {speedup:>6.2f}x of {least:<4g} {"slow" if speedup < least else "":4}  {code}' for code, one, many, speedup, least in self)
        return '\n'.join([f'1 thread against {self.threads}', *rows])

_SCALING_SETUP = 'pm←¿[1024 1024]⍴0 ⋄ sq←¿[512 512]⍴0 ⋄ hq←sq+⍉sq ⋄ stk←¿[16384 8 8]⍴0 ⋄ img←¿[256 256 16]⍴0 ⋄ cw←[64516 144]⍴3 3↕img ⋄ ck←¿[144 32]⍴0 ⋄ big←¿4194304⍴0 ⋄ n←≢big'
_SCALING_THREADS = 4
# Each case's serial fraction s: the share of its one-thread time that doesn't split across threads. On t threads it should then speed up 1/(s + (1 − s)/t) times, by Amdahl's law. A product, a stack's cells and an elementwise function split into independent pieces. A product of windows splits its rows into one piece for each thread, which gathers its own windows a block at a time. A copy of a view splits its items across threads, but memory bandwidth limits its gain. Its serial fraction comes from measurement: a copy of a convolution's windows gained 1.6 to 1.9 times on 4 threads of an M5 Max (28 October 2026). A serial fraction of 0.5 matches the lower gain. A 512×512 SVD or Hermitian eigendecomposition spends about half its time in matrix products and half in its one-stage reduction. The products split. The reduction's matrix-vector passes over the trailing matrix are mostly too short to split.
_SCALING = {'pm+.×pm': 0, '"svd"•decompose sq': 0.5, '"eigen"•decompose hq': 0.5, '"qr"•decompose⍤2 stk': 0, 'cw+.×ck': 0, '3 3↕img': 0.5, '*big': 0}
# On macOS, Accelerate computes these cases, and their serial fractions come from measurement. Its GESDD and SYEVD run on one thread whatever its threading setting, as NumPy's calls into the same Accelerate show. Its 1024×1024 product ran at about 480 GFLOP/s on one thread of an M5 Max and gained 1.35 times on 4 threads (27 October 2026), a serial fraction of 0.65. The product of windows gained 1.88 times on 4 threads (28 October 2026), a serial fraction of 0.38.
_ACCELERATED = {'pm+.×pm': 0.65, '"svd"•decompose sq': 1, '"eigen"•decompose hq': 1, 'cw+.×ck': 0.38}

def _physical_cores():
    "This machine's physical cores: the distinct sets of hardware threads that share a core, as Linux lists them, or else its logical CPUs. On Apple silicon, which runs one hardware thread on each core, the logical CPUs are the cores."
    return len({p.read_text() for p in Path('/sys/devices/system/cpu').glob('cpu[0-9]*/topology/thread_siblings_list')}) or os.cpu_count()

def thread_scaling(
    cases:str=None, # Codes of the cases in `_SCALING` to time, or one code; None: all of them, a 1024×1024 product, a 512×512 SVD and Hermitian eigendecomposition, QR decompositions of a stack of 16,384 8×8 matrices, a product of a 3 by 3 convolution's windows, a copy of those windows, and `*` on 4,194,304 floats
    setup:str=_SCALING_SETUP, # BPL that defines the cases' data and its number of items `n`
    build:str=None, # The build to time, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build
    timeout:float=LIMIT, # Seconds of work that the check gets across its timings and retries, or a `Limit` to share (`worker.Limit`); None: no limit
):
    "Time each case with `time_cases` on one thread and on t threads, the smaller of `_SCALING_THREADS` and this machine's physical cores (`_physical_cores`), and give its speedup, both on one build. Speed budgets are single-thread measurements, so this checks multithreaded speed instead: in the manual `Speed` workflow and locally, not in the `Test` workflow, where a runner's cores and sharing make it noisy. Each case must reach half its expected gain, 1 + (S − 1)/2, where S = 1/(s + (1 − s)/t) for its serial fraction s in `_SCALING`, or in `_ACCELERATED` on macOS. A case expected not to split, with s of 1, must reach 0.9. As the speed tests do, it times a case below its least again after the rest, after each pause in `rustymath::timing::RETRY_PAUSES`, and prints each retry. Raises `ValueError` with the table when a case is below its least every time."
    serial = _SCALING | (_ACCELERATED if sys.platform == 'darwin' else {})
    threads = min(_SCALING_THREADS, _physical_cores())
    least = lambda s: 1 + (1 / (s + (1 - s) / threads) - 1) / 2 if s < 1 else 0.9
    build, timeout = resolved(build), Limit.of(timeout)
    def timed(codes):
        one, many = (time_cases(codes, setup, threads=t, build=build, timeout=timeout) for t in (1, threads))
        return {code: (code, a, b, a / b, least(serial[code])) for (code, a, _), (_, b, _) in zip(one, many)}
    rows = timed(listify(cases) or list(serial))
    for pause in _retry_pauses:
        slow = {code: speedup for code, _, _, speedup, floor in rows.values() if speedup < floor}
        if not slow: break
        time.sleep(pause)
        rows |= timed(list(slow))
        for code, before in slow.items():
            *_, after, floor = rows[code]
            print(f'{code}: {before:.2f}x, then {after:.2f}x after a pause of {pause} s, against {floor:g}')
    result = Scaling(rows.values(), threads)
    if any(speedup < floor for *_, speedup, floor in result): raise ValueError(f'\n{result!r}')
    return result

class Instructions(list):
    "Each case's instructions per unit of `N`, as `(code, instructions)` rows."
    def __repr__(self): return '\n'.join(f'{n:>12,.0f}  {code}' for code, n in self)

# The line of each platform's counter that gives the instructions a process retired: `/usr/bin/time -l`'s on macOS, `perf stat -x,`'s on Linux.
_RETIRED = dict(darwin=re.compile(r'^\s*(\d+)\s+instructions retired$', re.M), linux=re.compile(r'^(\d+),[^,]*,instructions', re.M))

# The environment of the processes that count a case's instructions or profile its functions, with the settings that `worker.environment` takes. Loops run their steps as ordinary code (`settings::Loops`), so that a count measures a call's interpretation, as `tests/instructions.txt` records it. With `replays`, loops replay their recordings, as real runs do.
def _counting(replays=False, **kwargs): return environment(**kwargs) | ({} if replays else dict(BASEDPL_LOOPS='ordinary'))

def _retired(code, bpl, replays, settings, limit):
    "The instructions that a fresh `bpl` process retires running `code`, in the environment that `_counting` gives for `replays` and `settings`, killed once `limit` (`worker.Limit`) runs out."
    if sys.platform not in _RETIRED: raise ValueError(f'no instruction counter on {sys.platform}')
    with tempfile.NamedTemporaryFile('r') as report:
        counter = ['/usr/bin/time', '-l', '-o', report.name] if sys.platform == 'darwin' else ['perf', 'stat', '-x,', '-e', 'instructions:u', '-o', report.name, '--']
        p = bounded([*counter, resolved(bpl), '-e', code], limit.left(), env=_counting(replays, **settings))
        if p.returncode: raise ValueError(p.stdout + p.stderr)
        return int(_RETIRED[sys.platform].search(report.read())[1])

def _program(code, n, setup):
    "BPL that runs `setup`, then `code` once with `N` set to `n`, on one thread with the random generator seeded at 0, and ends without displaying its result. `setup` comes first so that a `•prefs` call at its start still applies before the program is parsed, which fixes each literal's float width."
    return f'•prefs ["threads":1 "seed":0]\n{setup}\nN←{n}\nr←{code}\n0'

# The instructions that a case retires before the lowest value of `N` that `instructions` picks, and between successive values. A case that allocates an array at each step retires a few tens of millions more instructions over its first steps, while the allocator settles. `{↑[⍵ 1]}⍣N 0` counts the same from 3×10⁴ steps on, about 2×10⁸ instructions, and `{fib 10ₓ}⍣N 0` from 30 steps on. Each process pays a different warm-up, up to about 5×10⁶ instructions, which moves a slope over 5×10⁸ instructions by under 1%. Some cases also pay a one-off cost once `N` passes a size: `≢•ucs N⍴"ab" "cde"` retires about 5×10⁶ more instructions from about 131,350 items on.
_SETTLE, _SPAN = 5e8, 5e8

# A case that names `N` outside its strings says itself how it repeats. `_stepped` runs any other case once per step.
_NAMES_N, _QUOTED = re.compile(r'(?<!\w)N(?!\w)'), re.compile(r'"(?:[^"]|"")*"|\'.\'')

def _stepped(case, force=False):
    "BPL that makes one call of `case` per step of `N`. A case that names `N`, such as `{⍵}⍣N 0`, is that BPL already. Any other case runs once per step, and its result is dropped, or with `force` computed in full through an indexed write where the result is an array with a first item."
    if _NAMES_N.search(_QUOTED.sub('', case)): return case
    return f'{{x←{case} ⋄ (×≢⍴x)∧×≢x?x.[0]←x.[0] ⋄ ⍵;⍵}}⍣N 0' if force else f'{{{case} ⋄ ⍵}}⍣N 0'

def _locked(pool, calls, limit=None):
    "The results of `calls`, each a function and its arguments, run at once on `pool` as one batch, holding the machine's timing lock shared for that batch alone, and spending `limit` (`worker.Limit`) while it runs. A timing that waits for the lock runs between batches. An interrupt or error cancels the calls that haven't started; those running end within `limit` (`fastws.procs.bounded`)."
    with timing_lock(), (limit or Limit(None)).running():
        futures = [pool.submit(*call) for call in calls]
        try: return [f.result() for f in futures]
        except BaseException:
            for f in futures: f.cancel()
            raise

@delegates(environment)
def instructions(
    cases:Annotated[str, {'nargs':'+'}], # A BPL expression to run once per step, such as `k(⌈/⊢)⌸a`, or one that names `N` and makes its own call per step of it, such as `{⍵}⍣N 0`, or a list of them
    setup='', # BPL that defines what the cases use, run before each case
    force=False, # Compute each step's result in full where it would stay pending, such as large windows or a transpose, for the cases that don't name `N`
    counts:Annotated[int, {'nargs':'+'}]=None, # Two or more values of `N`; None: four values past the allocator's warm-up, from a rough count at 1,000 steps
    runs=3, # Processes at each value of `N`, of which the lowest count is used
    replays=False, # Count loops as real runs make them, replaying their recordings, in place of running every step as ordinary code
    build:Annotated[str, {'nargs':'+'}]=None, # The build to count, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build; or a list of builds to compare
    timeout:float=LIMIT, # Seconds of work that the count gets, or a `Limit` to share (`worker.Limit`), spent while its processes run; None: no limit
    **kwargs
):
    "The instructions that each case retires per unit of `N`, on one thread. A case that doesn't name `N` runs once per step inside `{case ⋄ ⍵}⍣N 0`. That dfn's own call adds about 1,900 instructions per step. For each pair of successive values of `N`, the difference between fresh `bpl` processes, divided by the difference in `N`, cancels startup and `setup`. The result is the median of those slopes. The median drops a one-off cost that starts once `N` passes some size, because such a cost falls between only one pair. Unlike timings, instruction counts don't vary with load or code layout, so they show per-call changes of a few percent, and its processes run at once, one for each core. A case that names `N` must make one call per step of `N`, as `⍣` does: a map such as `f¨N⍴0` calls a dfn once on all its cells, so it gives no per-call cost. Given several builds, it gives `ByBuild`, counting each case at the same values of `N` on every build. Each build is resolved once, as the count starts."
    cases = listify(cases)
    builds = listify(build) or [None]
    stepped = [_stepped(case, force) for case in cases]
    def slope(ns, cs): return statistics.median((c1-c0)/(n1-n0) for n0, n1, c0, c1 in zip(ns, ns[1:], cs, cs[1:]))
    paths, limit = [resolved(b) for b in builds], Limit.of(timeout)
    with ThreadPoolExecutor(os.cpu_count()) as pool:
        def lowest(jobs, runs=runs):
            "The lowest count of `runs` processes for each `(code, n, build)` in `jobs`, from processes that run at once as one batch (`_locked`)."
            got = _locked(pool, [(_retired, _program(code, n, setup), build, replays, kwargs, limit) for code, n, build in jobs for _ in range(runs)], limit)
            return [min(got[i:i+runs]) for i in range(0, len(got), runs)]
        if counts: ns = [counts]*len(cases)
        else:
            rough = lowest([(code, n, paths[0]) for code in stepped for n in (1_000, 0)])
            # Each value is at least one more than the last, for a case that retires more than `_SPAN` instructions a step, whose warm-up is then a small share of a step.
            ns = [list(accumulate((round((_SETTLE + i*_SPAN)/max(1, (c1-c0)/1_000)) for i in range(4)), lambda last, n: max(n, last + 1)))
                  for c1, c0 in zip(rough[::2], rough[1::2])]
        each = []
        for build in paths:
            cs = iter(lowest([(code, n, build) for code, steps in zip(stepped, ns) for n in steps]))
            each.append(Instructions((case, slope(steps, [next(cs) for _ in steps])) for case, steps in zip(cases, ns)))
    return each[0] if len(builds) == 1 else ByBuild(each, builds)

class FunctionCounts(list):
    "Instructions by function: `(function, count)` rows for one build, largest first, or `(function, before, after)` rows for two, largest change first. The rows of `totals`, in the same form, show above them. One build's rows show each count's share of the first total."
    def __init__(self, rows=(), totals=()):
        super().__init__(rows)
        self.totals = totals
    def __repr__(self):
        def line(f, *ns): return (f'{ns[0]:>10,.0f} {ns[0]/self.totals[0][1]:>6.1%}' if len(ns) == 1 else f'{ns[0]:>10,.0f} {ns[1]:>10,.0f} {ns[1]-ns[0]:>+10,.0f}') + f'  {f[:150]}'
        return '\n'.join(line(*row) for row in [*self.totals, *self])

# A frame in the call graph that macOS `sample` reports: the tree's drawing characters, whose width gives the frame's depth, then its samples and its symbol.
_FRAME, _THREAD = re.compile(r'^([ +!:|]*)(\d+) (\S.*?)  [\[(]'), re.compile(r'^\s*\d+ Thread_')

def _self_samples(code, bpl, seconds, replays, within, settings):
    "The samples at which each function was at the top of the main thread's stack while a fresh `bpl` process ran `code` in the environment that `_counting` gives for `replays` and `settings`, as macOS `sample` counts them over `seconds`, with demangled names, and the thread's total. `code` must run for longer than half a second plus `seconds`. The process is then killed. It raises `TimeoutError` when `sample` runs past `within` seconds, as it does while macOS waits for an answer to a Developer Tools authorization prompt."
    p = subprocess.Popen([resolved(bpl), '-e', code], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, env=_counting(replays, **settings))
    try:
        time.sleep(0.5)
        if p.poll() is not None: raise ValueError(p.stderr.read())
        with tempfile.NamedTemporaryFile('r', suffix='.txt') as report:
            try: s = subprocess.run(['sample', str(p.pid), str(seconds), '-file', report.name], capture_output=True, text=True, timeout=within)
            except subprocess.TimeoutExpired: raise TimeoutError(rf"`sample` didn't finish in the {within:.1f} s left of the call's limit. macOS may be waiting for an answer to a Developer Tools authorization prompt.") from None
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

def _estimates(case, setup, build, seconds, force, replays, limit, settings):
    "Each function's estimated instructions per step of `N` for `case` on `build`, and the build's instructions per step, as `profile` describes, under the settings that `worker.environment` takes."
    if sys.platform != 'darwin': raise ValueError(f'no sampling profiler on {sys.platform}')
    stepped = _stepped(case, force)
    bpl = resolved(build)
    [(_, per_step)] = instructions(stepped, setup, build=bpl, replays=replays, timeout=limit, **settings)
    with timing_lock(), limit.running(): samples, total = _self_samples(_program(f'({{{stepped} ⋄ ⍵}}⍣1000000000)0', max(1, round(1e8/per_step)), setup), bpl, seconds, replays, limit.left(), settings)
    return {f: per_step*n/total for f, n in samples.items()}, per_step

@delegates(environment)
def profile(
    case, # A BPL expression, as `instructions` takes it, such as `k(⌈/⊢)⌸a` or `{⍵}⍣N 0`
    setup='', # BPL that defines what the case uses, run before it
    force=False, # Compute each step's result in full, as `instructions`' `force` does
    replays=False, # Count loops as real runs make them, as `instructions`' `replays` does
    build:str=None, # The build to profile, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build,
    seconds=3, # How long macOS `sample` watches the build
    timeout:float=LIMIT, # Seconds of work that the count and sampling get, or a `Limit` to share (`worker.Limit`); None: no limit
    rows=20, # How many functions to show
    **kwargs
):
    "Where `case`'s instructions per step of `N` go, by function, on macOS. A fresh process repeats the case on one thread while `sample` counts how often each function is at the top of the main thread's stack. A function's estimate is its share of those samples times the build's instructions per step from `instructions`. The shares come from time, so the estimates assume every function retires instructions at the same rate."
    estimates, total = _estimates(case, setup, build, seconds, force, replays, Limit.of(timeout), kwargs)
    return FunctionCounts(sorted(estimates.items(), key=lambda r: -r[1])[:rows], [('total', total)])

@delegates(environment)
def profile_diff(
    case, # A BPL expression, as `instructions` takes it, such as `k(⌈/⊢)⌸a` or `{⍵}⍣N 0`
    base:str, # The build to compare with, resolved as `build` is, such as the one installed before a change
    setup='', # BPL that defines what the case uses, run before it
    force=False, # Compute each step's result in full, as `instructions`' `force` does
    replays=False, # Count loops as real runs make them, as `instructions`' `replays` does
    build:str=None, # The build to compare with `base`, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build,
    seconds=3, # How long macOS `sample` watches each build
    timeout:float=LIMIT, # Seconds of work that both builds' counts and sampling get, or a `Limit` to share (`worker.Limit`); None: no limit
    rows=20, # How many functions to show
    **kwargs
):
    "Where `case`'s instructions per step of `N` moved between two builds, by function, on macOS, from `profile`'s estimates for each build. A function that only one build calls out of line, such as one that stopped being inlined, appears in that build alone."
    limit = Limit.of(timeout)
    (before, tb), (after, ta) = _estimates(case, setup, base, seconds, force, replays, limit, kwargs), _estimates(case, setup, build, seconds, force, replays, limit, kwargs)
    fs = sorted(before.keys() | after.keys(), key=lambda f: -abs(after.get(f, 0) - before.get(f, 0)))
    return FunctionCounts([(f, before.get(f, 0), after.get(f, 0)) for f in fs[:rows]], [('total', tb, ta)])

# The script that `call_profile` runs in `lldb`'s own Python, 3.9 on macOS. `run` reads its arguments from a JSON file. It launches `bpl` with `-e code` and lets `skip` calls of the function that `pattern` names run. It then single-steps the next call to its return, the first instruction where the stack pointer is back where it was and the program counter at the address in the link register. An instruction counts under its function's name, or under `caller > function` where the function's caller, read as the program counter enters the function, matches the allocator's pattern. An instruction in the allocator also names, after a tab, the first function on the stack outside the allocator and Rust's standard library, the one whose code allocated. `call_profile` then puts what the allocator calls, such as the zeroing of a freed block and a read of the clock, in the allocator's row. It writes JSON to `out`: how many instructions each function retired, or the number of calls where the process ended first, or that the call passed `limit`.
_LLDB_STEPS = r'''
import collections, json, lldb, re

def run(path):
    a = json.load(open(path))
    debugger = lldb.debugger
    debugger.SetAsync(False)
    debugger.HandleCommand('settings set target.skip-prologue false')
    target = debugger.CreateTarget(a['bpl'])
    point = target.BreakpointCreateByRegex(a['pattern'])
    process, hits, result = target.LaunchSimple(['-e', a['code']], None, None), 0, {}
    while process.GetState() == lldb.eStateStopped:
        thread = process.GetSelectedThread()
        if thread.GetStopReason() == lldb.eStopReasonBreakpoint and (hits := hits + 1) > a['skip']: break
        process.Continue()
    else: result['hits'] = hits
    if not result:
        target.BreakpointDelete(point.GetID())
        frame = thread.GetFrameAtIndex(0)
        sp, back = frame.GetSP(), frame.FindRegister('lr').GetValueAsUnsigned()
        allocator, counts, n, current = re.compile(a['allocator']), collections.Counter(), 0, None
        while n < a['limit']:
            frame = thread.GetFrameAtIndex(0)
            if frame.GetPC() == back and frame.GetSP() == sp: break
            symbol = frame.GetSymbol()
            here = symbol.GetName() if symbol.IsValid() else hex(frame.GetPC())
            if here != current:
                current, caller = here, thread.GetFrameAtIndex(1).GetSymbol()
                name = f'{caller.GetName()} > {here}' if caller.IsValid() and allocator.search(caller.GetName()) and not allocator.search(here) else here
                if allocator.search(name):
                    frames = (thread.GetFrameAtIndex(i).GetFunctionName() or '' for i in range(1, thread.GetNumFrames()))
                    name += '\t' + next((f for f in frames if f and not allocator.search(f) and not re.match(r'<?(?:alloc|core|std)::', f)), '?')
            counts[name] += 1
            n += 1
            thread.StepInstruction(False)
        result = dict(counts=counts) if n < a['limit'] else dict(over=True)
    process.Kill()
    json.dump(result, open(a['out'], 'w'))
'''

def _counted_call(code, pattern, skip, build, within, settings, limit=100_000, replays=False):
    "How many instructions each function retired in the call of the function that `pattern` names after `skip` earlier calls, in a fresh `bpl` process running `code` in the environment that `_counting` gives for `replays` and `settings`, as `_LLDB_STEPS` counts them. `lldb` spends `within` (`worker.Limit`) while it runs, and raises `TimeoutError` once it's spent, as it is while macOS waits for an answer to a Developer Tools authorization prompt."
    bpl = str(resolved(build))
    with tempfile.TemporaryDirectory() as d:
        steps, args, out = Path(d)/'steps.py', Path(d)/'args.json', Path(d)/'out.json'
        steps.write_text(_LLDB_STEPS)
        args.write_text(json.dumps(dict(bpl=bpl, code=code, pattern=pattern, skip=skip, out=str(out), limit=limit, allocator=_ALLOCATOR.pattern)))
        try:
            with within.running(): p = bounded(['lldb', '--batch', '--no-lldbinit', '-o', f'command script import {steps}', '-o', f'script steps.run({str(args)!r})'], within.left(), env=_counting(replays, **settings))
        except TimeoutError as e:
            e.add_note('macOS may be waiting for an answer to a Developer Tools authorization prompt.')
            raise
        if not out.exists(): raise ValueError(p.stdout + p.stderr)
        result = json.loads(out.read_text())
    if 'hits' in result: raise ValueError(f'`{pattern}` stopped {bpl} at {result["hits"]} calls, and `skip` lets {skip} run first. A function that is always inlined never stops it.')
    if 'over' in result: raise ValueError(f'the call retired more than {limit:,} instructions, in {bpl}')
    return result['counts']

# The functions of macOS's allocator and of Rust's allocation shims. A function that one of them calls matches too, through the caller in its `caller > function` name. `call_profile` gives them one row, because the instructions they retire depend on the heap's state.
_ALLOCATOR = re.compile(r'malloc|^_*free|mfm|xzm|szone|nanov2|__rust_(?:alloc|dealloc|realloc|no_alloc)')

@delegates(environment)
def call_profile(
    case, # A BPL expression, as `instructions` takes it, such as `v_4ₓ₄` or `{⍵}⍣N 0`
    function, # A regex that the demangled name of the function to count matches, such as `binder::derived`. A function that is always inlined has no code of its own and never stops the process
    setup='', # BPL that defines what the case uses, run before it
    base:str=None, # The build to compare with, resolved as `build` is, such as the one installed before a change; None: count one build
    build:str=None, # The build to count, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build,
    skip=3, # Calls of `function` to let run first, while the case warms up
    force=False, # Compute each step's result in full, as `instructions`' `force` does
    timeout:float=LIMIT, # Seconds of work that the counts get, or a `Limit` to share (`worker.Limit`). `lldb` steps about 450 instructions a second, so a call of more than about 13,000 needs a larger one; None: no limit
    replays=False, # Count loops as real runs make them, as `instructions`' `replays` does
    allocations=False, # Split the allocator's row by the first function outside the allocator and Rust's standard library that called it
    rows=20, # How many functions to show
    **kwargs
):
    "Every instruction that one call of `function` retires, with the calls it makes, counted by function, on macOS on Apple silicon. `lldb` runs `case` once per step in a fresh process, lets `skip` calls of `function` run, and single-steps the next one to its return. The counts are exact, where `profile`'s estimates come from time, but they cover one call. At the 450 or so instructions a second that `lldb` steps, the largest call it counts, 100,000 instructions, takes about four minutes. The allocator's functions, with the functions they call, share the row `allocator`, which the second total leaves out. With `allocations`, they have a row for each function that called the allocator, named `allocator ← function`."
    if sys.platform != 'darwin' or os.uname().machine != 'arm64': raise ValueError('`call_profile` runs only on macOS on Apple silicon')
    code, timeout = _program(_stepped(case, force), 1000, setup), Limit.of(timeout)
    merged, allocated = [defaultdict(int) for _ in range(2 if base else 1)], [0, 0]
    for k, (b, m) in enumerate(zip([base, build] if base else [build], merged)):
        for f, n in _counted_call(code, function, skip, b, timeout, kwargs, replays=replays).items():
            name, _, site = f.partition('\t')
            if _ALLOCATOR.search(name): allocated[k], name = allocated[k] + n, f'allocator ← {site}' if allocations else 'allocator'
            m[name] += n
    totals = [('total', *(sum(m.values()) for m in merged)), ('outside the allocator', *(sum(m.values()) - a for m, a in zip(merged, allocated)))]
    fs = sorted(set().union(*merged), key=lambda f: -abs(merged[-1].get(f, 0) - merged[0].get(f, 0)) if base else -merged[0].get(f, 0))
    return FunctionCounts([(f, *(m.get(f, 0) for m in merged)) for f in fs[:rows]], totals)


# basedpl's library and the rustymath library it links, built as CI's Linux runner compiles them. Apple's `clang` assembles the x86-64 source in basedpl's `psm` dependency, because this Mac has no `x86_64-linux-gnu-gcc`. Incremental compilation is off, as on CI, so that code splits into the same codegen units.
_X86_BUILD = 'cargo build --lib --release --target x86_64-unknown-linux-gnu --target-dir /tmp/aai-ws-x86 -p basedpl'.split()
_X86_ENV = dict(CARGO_INCREMENTAL='0', CC_x86_64_unknown_linux_gnu='clang')
# A line of `nm`'s listing: an address, or spaces for an undefined symbol, then the symbol's type letter and its name.
_NM_LINE = re.compile(r'^(?:[0-9a-f]+| +) (\S) (.+)$')
# A mangled Rust name in `objdump`'s output, in the v0 scheme or the legacy one, with Mach-O's extra leading underscore where it has one.
_MANGLED = re.compile(r'(?<![\w$.])_?(?:_R|_ZN)[\w$.]+')

def _x86_libraries():
    "The libraries that `_X86_BUILD` makes, and a note that's empty unless basedpl failed to compile. Cargo builds rustymath first, so a failed build still gives rustymath's library."
    p = subprocess.run([*_X86_BUILD, '--message-format=json-render-diagnostics'], cwd=_ROOT, capture_output=True, text=True, env=os.environ | _X86_ENV)
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
    build:str=None, # The build to read, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build. With `x86`, unused
):
    "The machine code of the function whose demangled name matches `pattern`, as `objdump` shows it, with names demangled. Where functions of more than one name match, it gives their names. With `x86`, it first builds basedpl's library and the rustymath library it links for Linux x86-64, as CI's runner compiles them, holding the timing lock shared. Where basedpl doesn't compile, it reads rustymath's library alone and says so first. After a change, that build takes minutes."
    files, note = _x86_libraries() if x86 else ([resolved(build)], '')
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

def jit_disasm(
    case, # BPL whose compiled kernels to show, such as `+/ux>127ₓ₁`
    setup:str='', # BPL that defines what the case uses, run before it
    prefs:str='', # BPL that runs first, such as `•prefs ["kinds":"static"]`
    build:str=None, # The build to run, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build
    timeout:float=LIMIT, # Seconds that the run gets, or a `Limit` to share (`worker.Limit`); None: no limit
):
    "The machine code of each kernel that the JIT compiles while `prefs`, `setup` and `case` run, after a line that describes the kernel, as `BASEDPL_JIT_DISASM` prints them. A stage compiles on its first run where that run repays the compile (`Blocks::repays`), from about 2.7 million items for two elementwise steps, and a recording's stages compile when it's reused."
    p = bounded([resolved(build), '-e', '\n'.join(c for c in (prefs, setup, case) if c)], Limit.of(timeout).left(), env={**os.environ, 'BASEDPL_JIT_DISASM': '1'})
    if p.returncode: raise ValueError(p.stderr)
    return Report(p.stderr)

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
    build:str=None, # The build to count, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build
    timeout:float=LIMIT, # Seconds of work that the counts get, or a `Limit` to share (`worker.Limit`); None: no limit
):
    "Count each case in `calls` with `instructions`, beside its count."
    setup, cases = _calls(_CALLS.read_text() if calls is None else calls)
    counted = instructions([code for code, _ in cases], setup, build=build, timeout=timeout)
    return InstructionBudgets((code, n, count) for (code, n), (_, count) in zip(counted, cases))

def _or_dash(x, spec): return '-' if x is None else format(x, spec)
def _over(cost, budget): return budget is not None and cost > budget

class SpeedCosts(list):
    "Each case's cost and the budget that limits it, as `(section, code, cost, budget, ns)` rows. A cost is in copies, except in `[widths]`, where it is the 32-bit time as a fraction of the 64-bit time. A case with no budget has `None`. `ns` is the case's time for each item in nanoseconds, or `None` where the timing doesn't give it. Times compare across a change to `rustymath::timing::COPIES`, where costs don't. A case over its budget needs study, which finds either a speed bug or something its expected cost missed. `cpu` is the CPU model that a CI log reports, or `None`. `platform` is the platform whose native budgets the costs were checked against, 'macos' or 'linux', or `None` for a browser build or a machine with no budgets. `copies` holds the copy lines that a log gives, as `(items, bytes, ns per item, stored)` rows, which say whether `rustymath::timing::COPIES` held that machine's copy time or the test measured it. `footprints` maps the `(section, code)` of each case whose footprint passes one core's cache (`rustymath::timing::footprint`) to that footprint in bytes and the time for each item, in nanoseconds, of a copy of it measured beside the case. A case's `ns` divided by that time is its cost in copies of its footprint. `holds` maps the `(section, code)` of each case whose budget a `held` or `busy` line raises to `(hold, rule, stale)`: `'held'` for a gap in the code or `'busy'` for CI's busy hosts, the rule budget that its footer gives, and whether a gap's hold has outlived the gap, as `rustymath::timing::Budget::stale` decides. The display marks each case `over`, `stale`, `held` or `busy`, in that order of precedence."
    def __init__(self, rows=(), cpu=None, platform=None, copies=(), footprints=None, holds=None):
        super().__init__(rows)
        self.cpu, self.platform, self.copies, self.footprints, self.holds = cpu, platform, list(copies), dict(footprints or {}), dict(holds or {})
    def __repr__(self):
        fp = {k: f' {b/2**20:8.1f} MiB {ns:6.3f} ns' for k, (b, ns) in self.footprints.items()}
        pad = ' ' * max(map(len, fp.values()), default=0)
        def mark(s, c, cost, b):
            hold, _, stale = self.holds.get((s, c), ('', None, False))
            return 'over' if _over(cost, b) else 'stale' if stale else hold
        return '\n'.join(f'{cost:>9.2f} {_or_dash(b, "g"):>9} {_or_dash(ns, ".3f"):>9} {mark(s, c, cost, b):5}{fp.get((s, c), pad)}  {s} {c}' for s, c, cost, b, ns in self)

def _node():
    "The Node binary for basedpl's browser and editor tests, of the major version that `.node-version` names for CI: `node` on `PATH` where it has that version or where there's no `.node-version`, else Homebrew's `node@N`. Raises `ValueError` where neither runs."
    pin = _ROOT/'.node-version'
    want = pin.read_text().strip().lstrip('v').split('.')[0] if pin.exists() else None
    for node in ['node', *([f'/opt/homebrew/opt/node@{want}/bin/node', f'/usr/local/opt/node@{want}/bin/node'] if want else [])]:
        try: ran = subprocess.run([node, '--version'], capture_output=True, text=True)
        except FileNotFoundError: continue
        if not ran.returncode and want in (None, ran.stdout.lstrip('v').split('.')[0]): return node
    raise ValueError(f'`.node-version` names Node {want}, which this machine lacks: `brew install node@{want}` installs it' if want else "`node` on `PATH` doesn't run")

def _wasm_env(wasm):
    "The environment in which `tests/wasm-pkg.mjs` loads the browser build that `cargo wasm --out` wrote to `wasm`."
    return os.environ | dict(BASEDPL_WASM=str(Path(wasm).expanduser()))

@delegates(environment)
def speed_costs(
    wasm:Path=None, # A directory that `cargo wasm --out` wrote; None: a fresh native worker
    build:str=None, # The build to time, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build,
    speed:str=None, # One BPL speed file's text; None: tests/speed/*.bpl
    prefs='', # BPL run before each section's preferences and setup
    timeout:float=LIMIT, # Seconds of work that the timing gets, or a `Limit` to share (`worker.Limit`), counted from when it holds the timing lock; None: no limit
    cases:Annotated[str, {'nargs':'*', 'opt':False}]=None, # Codes or file-qualified labels to time; None: all
    **kwargs
):
    "Time the BPL speed records, with their budgets, through the shared native/browser runner."
    files = [dict(name=p.name, text=p.read_text()) for p in sorted(_SPEED.glob('*.bpl'))] if speed is None else [dict(name='<input>', text=speed)]
    cases, limit = listify(cases) or None, Limit.of(timeout)
    if wasm is None:
        with Worker(build, **kwargs) as w:
            r = w.request(dict(costs=dict(files=files, cases=cases, prefs=prefs)), limit.left())
            limit.spend(w.ran)
        if e := r['error']:
            limit.reached(e)
            raise ValueError(e['message'])
        rows = r['costs']
    else:
        script = (f'import {{ costs }} from {json.dumps(_PKG.as_uri())}; import {{ readFileSync }} from "node:fs"; '
                  f'process.stdout.write(JSON.stringify(costs(JSON.parse(readFileSync(0, "utf8")), {json.dumps(cases)}, {json.dumps(prefs)})))')
        with timing_lock(exclusive=True), limit.running():
            p = bounded([_node(), '--no-liftoff', '--input-type=module', '-e', script], limit.left(), input=json.dumps(files), env=_wasm_env(wasm))
        if p.returncode: raise ValueError(p.stderr)
        if p.stderr: sys.stderr.write(p.stderr)
        rows = json.loads(p.stdout)
    footprints = {(c['section'], c['label']): (f['bytes'], f['seconds']*1e9) for c in rows if (f := c.get('footprint'))}
    holds = {(c['section'], c['label']): (c['hold'], c['rule'], c['stale']) for c in rows if c['hold']}
    rows = ((c['section'], c['label'], c['cost'], c['budget'], c['seconds']*1e9) for c in rows)
    return SpeedCosts(rows, platform=None if wasm else _platform(), footprints=footprints, holds=holds)


_LOGGED = re.compile(r'^(?:\S*Z )?(?P<tap># )?(?P<section>\w+) (?P<code>\S.*): (?P<cost>[\d.]+|inf|NaN|Infinity), budget (?P<budget>[\d.]+|-)(?: (?P<hold>held|busy) from (?P<rule>[\d.]+))?(?:, (?P<ns>[\d.]+) ns(?:, footprint (?P<mib>[\d.]+) MiB copied at (?P<copy>[\d.]+) ns)?)?(?P<stale>: stale)?$')
_COPY = re.compile(r'^(?:\S*Z )?(?:# )?copy of (?P<items>\d+) (?P<bytes>\d+)-byte items: (?P<ns>[\d.]+) ns an item, (?P<source>stored for|measured on) (?P<cpu>[^\r\n]+?)(?:: add it to rustymath::timing::COPIES)?[ \t]*$', re.M)

def logged_costs(
    log, # What a speed test prints with `<NAME>_COSTS` set, such as the log of a job in the `Test` or `Speed` workflow, or the path of a file that holds it, as a `Path` or a one-line str
    platform='linux' # The platform whose native budgets the log's costs were checked against: 'linux' for CI's runner, 'macos' for a Mac, or `None` for a machine with no budgets, such as Linux on Arm, whose log's budgets don't apply
):
    "The rows that `speed_costs` gives, read from `log`, with the CPU that its copy lines name as their `cpu`, `platform`, the log's copy lines as `copies`, the footprints that its case lines give as `footprints`, and the holds that they give as `holds`, where a hold is stale if the log marks the case `stale`. A case that the log times more than once, as after a retry, gives its lowest cost, with that timing's time for each item and footprint where the log prints them."
    if isinstance(log, Path) or '\n' not in log: log = Path(log).expanduser().read_text()
    found = list(_COPY.finditer(log))
    cpu = next((m['cpu'] for m in found if m['cpu'] != 'a CPU with no name'), None)
    rows, footprints, held, stale = {}, {}, {}, set()
    for m in filter(None, map(_LOGGED.match, log.splitlines())):
        code = re.sub(r'\\([\\#])', r'\1', m['code']) if m['tap'] else m['code'] # Node's TAP diagnostics escape `\` and `#`
        row = (m['section'], code, float(m['cost']), None if platform is None or m['budget'] == '-' else float(m['budget']), m['ns'] and float(m['ns']))
        if platform is not None and m['hold']: held[row[:2]] = (m['hold'], float(m['rule']))
        if m['stale']: stale.add(row[:2])
        if row[:2] not in rows or row[2] < rows[row[:2]][2]:
            rows[row[:2]] = row
            if m['mib']: footprints[row[:2]] = (round(float(m['mib']) * 2**20), float(m['copy']))
            else: footprints.pop(row[:2], None)
    copies = list(dict.fromkeys((int(m['items']), int(m['bytes']), float(m['ns']), m['source'] == 'stored for') for m in found))
    return SpeedCosts(rows.values(), cpu, platform, copies, footprints, {k: (*hold, k in stale) for k, hold in held.items()})

CHANGE = 0.2
class CostChanges(list):
    "The cases that `compare_costs` reports, as `(section, code, before, after, budget, change)` rows. `before` and `after` are costs, or `None` for a case that `time_cases` found too quick to time. `change` is the fractional change in the case's time for each item where both runs give one, and in its cost otherwise, so it holds across a change to `rustymath::timing::COPIES`. Cases over their budget in `after` come first, then cases whose change is more than `CHANGE`, largest first, then cases that only one run timed, as when a case's code changed between the runs. `before` is `None` for a case that only the later run timed, and `after` for one that only the earlier run timed, and `change` is `None` for both. `budget` is `None` for a case with no budget."
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
    "Each case over its budget in `after`, each case whose time for each item moved by more than `CHANGE` from `before` in either direction, or whose cost did where either run doesn't give the time, and each case that only one of the runs timed."
    before, after = (_comparable(x, platform) for x in (before, after))
    old = {(s, c): (cost, ns) for s, c, cost, _, ns in before}
    rows, only = [], []
    for s, c, cost, budget, ns in after:
        if (s, c) not in old: only.append((s, c, None, cost, budget, None)); continue
        cost0, ns0 = old.pop((s, c))
        rows.append((s, c, cost0, cost, budget, ns/ns0 - 1 if ns and ns0 else cost/cost0 - 1 if cost0 else None))
    only += [(s, c, cost0, None, None, None) for (s, c), (cost0, _) in old.items()]
    def moved(row): return abs(row[5] or 0)
    def over(row): return _over(row[3], row[4])
    changed = [r for r in rows if over(r) or moved(r) > CHANGE] + [r for r in only if over(r)]
    return CostChanges(sorted(changed, key=lambda r: (not over(r), -moved(r))) + [r for r in only if not over(r)])

class Bisection(list):
    "The builds that `bisect` measured, oldest first, as `(build, verdict, measure)` rows, where `build` is a `store.builds` record and `verdict` is `good` or `bad`. `before` and `after` are the adjacent builds where the verdict changes, or None where the endpoints don't differ. `edits` holds the journal's records of edits to `after`'s input files, from the build before `before` up to `after`, or None where the process has no journal (`EXHASH_JOURNAL`), as a shell has none."
    def __init__(self, rows=(), before=None, after=None, edits=()):
        super().__init__(rows)
        self.before, self.after, self.edits = before, after, edits
    def __repr__(self):
        def at(t, spec='%m-%d %H:%M:%S'): return time.strftime(spec, time.localtime(t))
        def names(files): return ', '.join(sorted(Path(f).name for f in files)) or 'none'
        def shown(m): return f'{m:,.0f}' if isinstance(m, float) else str(m).replace('\n', ' ¶ ')[:120]
        verdict = {b['id']: v for b, v, _ in self}
        rows = [f"    {b['id'][:8]}  {at(b['time'])}  {v:4}  {shown(m)}" for b, v, m in self]
        if self.after is None: return '\n'.join(['no change between the endpoints:', *rows])
        b, a = self.before, self.after
        lines = [f"first {verdict[a['id']]} build {a['id'][:8]} ({at(a['time'])}), after {verdict[b['id']]} build {b['id'][:8]} ({at(b['time'])})",
                 f"  inputs changed: {names(a['changes'] or [])}"]
        lines += [f"  {x['id'][:8]} inputs possibly modified since build started: {names(x['edited'])}" for x in (b, a) if x['edited']]
        lines.append('  no journal in this process: EXHASH_JOURNAL is unset' if self.edits is None else '  journal edits to its inputs:' if self.edits else '  no journal edits to its inputs')
        lines += [f"    {at(e['time'], '%H:%M:%S')}  {(e['author'] or '-')[:8]:8}  {os.path.relpath(e['path'], _ROOT.parent)}  +{e['added']} -{e['removed']}" for e in self.edits or ()]
        return '\n'.join([*lines, '  measured:', *rows])

def bisect(
    case, # What to measure on each build: a BPL expression, or for `by='check'` a reference case's id or header line in `path`
    good:str, # A kept build where `case` passes: a prefix of its id, as `store.builds` lists them
    bad:str=None, # A kept build where it fails, older or newer than `good`: a prefix of its id; None or `installed`: the installed build
    by:Annotated[str, {'choices':['check', 'value', 'instructions', 'time']}]='value', # How a build is judged: `check` (the reference case passes), `value` (the expression prints what it prints on `good`), `instructions` or `time` (its cost is nearer `good`'s than `bad`'s)
    setup:str='', # BPL that defines what `case` uses, run before it, for `value`, `instructions` and `time`
    prefs:str='', # BPL that runs first in each worker, such as `•prefs ["kinds":"static"]`, for `value` and `time`
    path:Path=_ROOT/'tests'/'reference'/'core.bpl', # The reference file, for `check`
    timeout:float=LIMIT, # Seconds of work that every measurement together gets, or a `Limit` to share (`worker.Limit`); None: no limit
):
    "Find the adjacent kept builds between `good` and `bad` where `case`'s verdict changes. Each step measures, in fresh workers, the build halfway between the nearest good and bad builds so far. The result names both builds, the input files that changed between them, the edits to those files that every session's journal recorded (the directories beside `EXHASH_JOURNAL`) and each measurement."
    kept = store.builds(_ROOT, None)
    def index(prefix):
        found = [i for i, b in enumerate(kept) if b['id'].startswith(prefix)]
        if len(found) != 1: raise ValueError(f'{len(found)} kept builds have ids starting {prefix!r}')
        return found[0]
    limit = Limit.of(timeout)
    def printed(b):
        try: return fresh(f'{setup} ⋄ {case}' if setup else case, prefs=prefs, build=b, timeout=limit)
        except (BplError, EOFError, TimeoutError) as e:
            limit.left()
            return f'{type(e).__name__}: {e}'
    measure = dict(check=lambda b: '; '.join(f"{f['status']}: {f['message']}" for f in check_file(path, [case], build=b, timeout=limit)),
                   value=printed,
                   instructions=lambda b: instructions(case, setup, build=b, timeout=limit)[0][1],
                   time=lambda b: time_cases(case, setup or None, prefs, build=b, timeout=limit)[0][1])[by]
    g, b = index(good), index(kept.installed if bad in (None, 'installed') else bad)
    measured = {g: measure(kept[g]['id']), b: measure(kept[b]['id'])}
    is_bad = dict(check=bool, value=lambda m: m != measured[g]).get(by, lambda m: abs(m - measured[b]) < abs(m - measured[g]))
    lo, hi = sorted((g, b))
    found = not is_bad(measured[g]) and is_bad(measured[b])
    while found and hi - lo > 1:
        mid = (lo + hi) // 2
        measured[mid] = measure(kept[mid]['id'])
        if is_bad(measured[mid]) == is_bad(measured[max(g, b)]): hi = mid
        else: lo = mid
    edits = None
    if found and (log := os.environ.get('EXHASH_JOURNAL')):
        from exhash import journal
        sessions = [d for d in Path(log).parent.iterdir() if d.is_dir()]
        edits = sorted((e for d in sessions for e in journal(d, since=kept[max(lo-1, 0)]['time']) if e['time'] <= kept[hi]['time'] and e['path'] in kept[hi]['inputs']), key=lambda e: e['time'])
    rows = [(kept[i], 'bad' if is_bad(m) else 'good', m) for i, m in sorted(measured.items())]
    return Bisection(rows, kept[lo] if found else None, kept[hi] if found else None, edits)

_LONG = ('listing', 'site') # Columns that `ExplainDiff` shows on lines of their own and never pairs rows by
_VARIES = ('site', 'microseconds') # Columns that vary between runs of one build, which a `~` row never reports

class ExplainDiff(list):
    "The rows that `explain_diff` reports, as `(record, sign, row, moved)` rows in `•explain`'s order of records. `sign` is `-` for a row that only `base` gave, `+` for a row that only `build` gave, and `~` for a row that both gave with different counts or listings. `row` maps each column to the row's value, from `build` except in a `-` row. `moved` maps each column that differs in a `~` row to its values in `base` and `build`. A row's `site`, where `•explain` filled it, and the `listing` of a program that only one build gave show on lines below the row; a moved listing shows as a diff."
    def __repr__(self):
        def cell(k, v): return f'{k} {v!r}' if isinstance(v, str) else f'{k} {v}'
        lines = []
        for record, sign, row, moved in self:
            cells = [f'{k} {moved[k][0]!r}→{moved[k][1]!r}' if k in moved else cell(k, v) for k, v in row.items() if k not in _LONG and (v != '' or k in moved)]
            lines.append(f'{sign} {record:13} {", ".join(cells)}')
            if 'listing' in moved: lines += ['    ' + l for l in list(difflib.unified_diff(*(m.splitlines() for m in moved['listing']), lineterm='', n=1))[2:]]
            elif sign != '~' and row.get('listing'): lines += ['    ' + l for l in row['listing'].splitlines()]
            if row.get('site'): lines.append(f'    site {row["site"]}')
        return '\n'.join(lines) or 'no difference'

def _explained(case, setup, prefs, build, sites, limit):
    "`•explain`'s records of `case` on `build`, as a route check captures them (the worker's `route` request), spending `limit`: each record's name to its rows, each row a dict from column to value. A build older than that request runs `case` as `•explain` text instead (`_explained_by_text`)."
    with Worker(resolved(build)) as w:
        r = w.request(dict(route=dict(code=case, setup=setup, prefs=prefs, sites=sites)), limit.left())
        limit.spend(w.ran)
        if (r['error'] or {}).get('kind') == 'REQUEST': r = _explained_by_text(w, case, setup, prefs, sites, limit)
    limit.reached(r['error'])
    if r['error']: raise BplError(r['error'], r['output'])
    return {name: [{k: v.item() if hasattr(v, 'item') else v for k, v in zip(t, vals)} for vals in zip(*t.values())] for name, t in _value(r['value']).items()}

def _explained_by_text(w, case, setup, prefs, sites, limit):
    "The reply of `w`, a build older than the worker's `route` request, to `•explain` of a second run of `case`, after `prefs`, `setup` and an explained first run, each evaluation spending `limit`. A build older than `•explain`'s options runs without `sites`."
    def run(code):
        r = w.eval(code, limit.left())
        limit.spend(w.ran)
        return r
    text, opts = '"' + case.replace('"', '""') + '"', '["sites":$t]' if sites else ''
    for code in filter(None, [prefs, setup, f'(•explain {text}) ⋄ 0']):
        if (r := run(code))['error']: return r
    r = run(f'1↓{opts}•explain {text}')
    return run(f'1↓•explain {text}') if r['error'] and opts else r

def _paired(old, new):
    "One record's `ExplainDiff` rows, as `(sign, row, moved)`: rows of `old` and `new` paired first by every column but `_LONG`'s, then by their text columns alone."
    out, old, new = [], list(old), list(new)
    cols = [k for k in new[0] if k in old[0]] if old and new else []
    for text_only in (False, True):
        def key(r): return [(k, r[k]) for k in cols if k not in _LONG and (isinstance(r[k], str) or not text_only)]
        for b in list(new):
            a = next((a for a in old if key(a) == key(b)), None)
            if a is None: continue
            old.remove(a); new.remove(b)
            if moved := {k: (a[k], b[k]) for k in cols if k not in _VARIES and a[k] != b[k]}: out.append(('~', b, moved))
    return [('-', a, {}) for a in old] + [('+', b, {}) for b in new] + out

def explain_diff(
    case, # BPL code that `•explain` runs, such as `{+/⍵×⍵}∂ v`
    base:str, # The build to compare with, resolved as `build` is, such as the one installed before a change
    setup='', # BPL that defines what the case uses, run first in each worker
    prefs='', # BPL that runs before `setup`, such as `•prefs ["kinds":"static"]`
    build:str=None, # The build to compare with `base`, resolved once (`worker.resolved`): a path to a `bpl`, a prefix of the id of a build that `cargo develop` kept, or None or `installed` for the installed build
    sites=False, # Fill each `forced` and `stopped` row's `site` column, as `•explain`'s `"sites"` option does, on builds that have the option
    timeout:float=LIMIT # Seconds of work that both captures get, or a `Limit` to share (`worker.Limit`); None: no limit
):
    "How `•explain`'s records of `case` differ between two builds: each row that only one build gave, and each row that both gave whose counts or program listing moved. Each build captures `case` as the route test does (the worker's `route` request), in a fresh worker: `prefs` and `setup` run, then an explained warm-up run, then the run whose records it compares."
    limit = Limit.of(timeout)
    old, new = (_explained(case, setup, prefs, b, sites, limit) for b in (base, build))
    return ExplainDiff((name, *r) for name in [*new, *(n for n in old if n not in new)] for r in _paired(old.get(name, []), new.get(name, [])))

def _cost_env(path, **kwargs):
    name = tomllib.loads((Path(path).expanduser()/'Cargo.toml').read_text())['package']['name']
    return environment(**kwargs) | {f'{name.upper()}_COSTS': '1'}

def _cost_rows(text, returncode):
    costs = logged_costs(text, _platform())
    if returncode and not costs: raise ValueError(text)
    return costs

def _run_costs(cmd, path, timeout, env=None, **kwargs):
    "The rows that `logged_costs` reads from what `cmd` prints, run in the package at `path` with `<NAME>_COSTS` set, `env`'s variables and the settings that `worker.environment` takes. A case over its budget still gives its row."
    path = Path(path).expanduser()
    limit = Limit.of(timeout)
    with limit.running(): p = bounded(cmd, limit.left(), cwd=path, env=_cost_env(path, **kwargs) | (env or {}))
    return _cost_rows(p.stdout + p.stderr, p.returncode)

@delegates(environment)
def cargo_costs(
    path='.', # A Cargo package, such as basedpl or rustymath, whose `tests/speed.rs` prints every cost when `<NAME>_COSTS` is set
    build:str=None, # A copy of the speed test's executable to run in place of the one that `cargo test` builds, such as one saved before a change
    timeout:float=LIMIT, # Seconds of work that the run gets, or a `Limit` to share (`worker.Limit`); None: no limit
    **kwargs
):
    "The speed test's logged cost rows. Build once outside the execution budget, then run that binary directly; `build` runs a saved executable instead. Time inside the binary, including lock waits, spends `timeout`. A case over its budget still gives its row."
    if build: return _run_costs([Path(build).expanduser()], path, timeout, **kwargs)
    limit = Limit.of(timeout)
    left = limit.left()
    try: result = run_tests(path, '--test', 'speed', timeout=left, env=_cost_env(path, **kwargs))
    except TimeoutError:
        limit.spend(left)
        raise
    limit.spend(result.seconds)
    return _cost_rows(result.text, result.returncode)

def _python_path(path, build):
    "The environment variables that put `build`'s Python package before the installed one on `PYTHONPATH`, for the package at `path`, or none where `build` is None. `build` is a directory that holds a copy of the package, or a prefix of the id of a build that `cargo develop` kept (`fastws.store`)."
    if _installed(build): return {}
    d = Path(build).expanduser()
    if not d.exists(): d = store.resolve(Path(path).expanduser(), str(build))/'python'
    return dict(PYTHONPATH=os.pathsep.join(filter(None, [str(d), os.environ.get('PYTHONPATH')])))

def pytest_costs(
    path='.', # A package, such as rustymath, whose `tests/test_speed.py` prints every cost when `<NAME>_COSTS` is set
    build:str=None, # The build of the package to time: a prefix of the id of a build that `cargo develop` kept (`fastws.store`), or a directory that holds a copy of its Python package; None or `installed`: the installed one
    native=False, # Time rustymath's own linear algebra in place of Accelerate, as `Worker`'s `native` does
    timeout:float=LIMIT # Seconds of work that the run gets, or a `Limit` to share (`worker.Limit`); None: no limit
):
    "The rows that `logged_costs` reads from what the Python speed test of the package at `path` prints, run with `pytest`. A case over its budget still gives its row."
    return _run_costs([sys.executable, '-m', 'pytest', '-q', 'tests/test_speed.py'], path, timeout, env=_python_path(path, build), native=native)

def run_pytest(
    tests:Annotated[str, {'nargs':'+'}], # A test's `pytest` node ID, such as `tests/test_structure.py::test_edge_modes_pad_take_and_windows`, or a list of them
    path='.', # A package, such as rustymath, whose tests the node IDs name
    build:str=None, # The build of the package to test: a prefix of the id of a build that `cargo develop` kept (`fastws.store`), or a directory that holds a copy of its Python package; None or `installed`: the installed one
    timeout:float=LIMIT # Seconds of work that the run gets, or a `Limit` to share (`worker.Limit`); None: no limit
):
    "What `pytest -q` prints for `tests`, run in the package at `path`."
    limit = Limit.of(timeout)
    with limit.running(): p = bounded([sys.executable, '-m', 'pytest', '-q', *listify(tests)], limit.left(), cwd=Path(path).expanduser(), env=os.environ | _python_path(path, build))
    return Report(p.stdout + p.stderr)

class Report(str):
    "Text displayed as it is, such as a test runner's report or a function's machine code."
    def __repr__(self): return str(self)

def wasm_test(
    wasm, # A directory that `cargo wasm --out` wrote
    speed=False, # Run `tests/wasm-speed.mjs`, with V8's `--no-liftoff` and `BASEDPL_COSTS` as CI runs it, in place of `tests/wasm.mjs`? Its report then lists every case's cost, which `logged_costs` reads
    timeout:float=LIMIT # Seconds of work that the run gets, or a `Limit` to share (`worker.Limit`), counted while it holds the timing lock; None: no limit
):
    "Run `tests/wasm.mjs` or `tests/wasm-speed.mjs` under `node --test` against the browser build in `wasm`, as CI runs them, and give the runner's report. Only the `node` process gets `BASEDPL_WASM` and `BASEDPL_COSTS`. It holds the machine's timing lock, exclusively for the speed test and shared for the others. A failing test raises `ValueError` with the report, starting on a line of its own so that `logged_costs` reads every row of it."
    test, limit = _SPEED.with_name('wasm-speed.mjs' if speed else 'wasm.mjs'), Limit.of(timeout)
    with timing_lock(exclusive=speed), limit.running():
        p = bounded([_node(), *(['--no-liftoff'] if speed else []), '--test', str(test)], limit.left(), cwd=_ROOT, env=_wasm_env(wasm) | (dict(BASEDPL_COSTS='1') if speed else {}))
    report = Report(p.stdout + p.stderr)
    if p.returncode: raise ValueError(f'\n{report}')
    return report

class StackUse(list):
    "Each recursion shape's worker stack per level and below its first level, in KB, as `(shape, per_level, base)` rows."
    def __repr__(self): return '\n'.join(f'{per:6.2f} KB a level {base:7.1f} KB base  {shape}' for shape, per, base in self)

def wasm_stack(
    wasm, # A directory that `cargo wasm --out` wrote
    timeout:float=LIMIT, # Seconds of work that every trial together gets, or a `Limit` to share (`worker.Limit`), counted while the trials run; None: no limit
):
    "The worker stack, in KB, that each recursion shape of `tests/wasm.mjs` takes per level and below its first level, under V8's TurboFan alone (`--no-liftoff`). The browser's depth limit, `MAX_CALL_DEPTH` in `src/eval.rs`, is set from these. For each shape, it bisects to 1 KB the least stack at which the shape's recursion reaches the test's depth, and half that depth. Each trial runs `tests/wasm.mjs depths` in a fresh `node` process. Its worker compiles the module, as the test's worker does. The bisections run at once, one for each core, in rounds that each run one trial for every unfinished bisection as one batch (`_locked`)."
    node, test, env, most, limit = _node(), _SPEED.with_name('wasm.mjs'), _wasm_env(wasm), 2048, Limit.of(timeout)
    def depths(kb, *shape):
        "What `tests/wasm.mjs depths` gives with `kb` KB of worker stack, for every shape or for `shape`, a name and a depth: each shape's depth, the depth it reached, and what deeper recursion gave. `None` where the stack is too small for the worker to load the module."
        p = bounded([node, '--no-liftoff', test, 'depths', str(kb), *map(str, shape)], limit.left(), cwd=_ROOT, env=env)
        # A worker whose stack is too small to load the module overflows before it posts results. Node then exits with a RangeError, printed as `RangeError [Error]` where an internal module overflowed, or exits 0 with no output.
        if 'Maximum call stack size exceeded' in p.stderr or not (p.returncode or p.stdout.strip()): return None
        if p.returncode: raise ValueError(p.stderr)
        return json.loads(p.stdout)
    with timing_lock(), limit.running(): full = depths(most)
    if short := [name for name, (n, reached, *_) in full.items() if reached != n]: raise ValueError(f'With {most} KB of stack, {", ".join(short)} fall short of their depths')
    jobs = [(name, d) for name, (n, *_) in full.items() for d in (n//2, n)]
    # Each bisection searches for the least worker stack at which its shape reaches its depth, from the 192 KB that Node keeps in reserve.
    bounds = {job: (192, most) for job in jobs}
    with ThreadPoolExecutor(os.cpu_count()) as pool:
        while trials := {job: (lo+hi)//2 for job, (lo, hi) in bounds.items() if hi - lo > 1}:
            for (job, mid), got in zip(trials.items(), _locked(pool, [(depths, mid, *job) for job, mid in trials.items()], limit)):
                lo, hi = bounds[job]
                bounds[job] = (lo, mid) if got and got[job[0]][1] == job[1] else (mid, hi)
    kb = {job: hi for job, (lo, hi) in bounds.items()}
    per = {name: (kb[name, n] - kb[name, n//2])/(n - n//2) for name, (n, *_) in full.items()}
    return StackUse((name, per[name], kb[name, n] - per[name]*n) for name, (n, *_) in full.items())

def _timed_build(name, cmd, env):
    "The seconds that the build `cmd` takes in basedpl's root with `env` added. A failed build raises `ValueError` with its output."
    start = time.monotonic()
    p = subprocess.run(cmd, cwd=_ROOT, env=os.environ | env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if p.returncode: raise ValueError(f'{name} failed:\n{p.stdout}')
    return time.monotonic() - start

def target_builds(
    out:Path, # The directory for the browser package, which `wasm_test` then tests
):
    "Build what CI builds and this Mac's daily builds leave out: basedpl's and rustymath's libraries for Linux x86-64, as `disasm`'s `x86` builds them, and the browser package, with `cargo wasm --out`. The two builds run at once. It gives each build's time, or raises `ValueError` naming each build that failed."
    builds = {'Linux x86-64 libraries': (_X86_BUILD, _X86_ENV), f'browser package in {out}': (['cargo', 'wasm', '--out', str(Path(out).expanduser())], {})}
    with ThreadPoolExecutor(len(builds)) as pool: running = {name: pool.submit(_timed_build, name, cmd, env) for name, (cmd, env) in builds.items()}
    rows, failed = [], []
    for name, f in running.items():
        try: rows.append(f'{name}: {f.result():.0f} s')
        except ValueError as e: failed.append(str(e))
    if failed: raise ValueError('\n\n'.join([*failed, *rows]))
    return Report('\n'.join(rows))

class Budgets(list):
    "Each budget that `set_budgets` changed, as `(section, code, build, old, new)` rows."
    def __repr__(self): return '\n'.join(f'{old:>9} -> {new:<9} {build:6}  {s} {c}' for s, c, build, old, new in self)

def set_budgets(
    expected, # Expected costs keyed by case code, in copies or, for widths, as a 32/64-bit ratio
    path=_SPEED, # A BPL file/directory, or Rustymath's speed.txt or speed_python.txt
    builds=None, # macos, linux or wasm, or a list; None: every column present
    sections=None, # Section names to update; None: all
    estimate=False, # True for costs that no run has measured on the selected builds, such as estimates for Linux or wasm, which keep budgets at twice the cost
):
    "Replace selected budget footers with the budget for each expected cost, by the rule that `rustymath::timing::budget` holds, leaving source, prose and `held` and `busy` lines unchanged; return the changed budgets. A held column's footer keeps this rule budget, and its line gives the cost it is held at."
    if isinstance(builds, str): builds = [builds]
    if isinstance(sections, str): sections = [sections]
    for code, cost in expected.items():
        if not math.isfinite(cost) or cost < 0: raise ValueError(f'{code} should cost {cost}')
    path = Path(path)
    if path.is_dir(): return Budgets(row for p in sorted(path.glob('*.bpl')) for row in set_budgets(expected, p, builds, sections, estimate))
    data, records = path.read_bytes(), []
    if path.suffix == '.bpl':
        records = [(c['section'], c['code'], c['budgets'], c['budget_span']) for c in _reference_file(data.decode())['cases'] if 'budgets' in c]
    else: records = [case for case in _budget_lines(data.decode()) if case[2]]
    width = min(len(_BUDGETS), max((len(row[2]) for row in records), default=0))
    if builds and (unknown := set(builds) - set(_BUDGETS[:width])): raise ValueError(f'{path.name} has no budget column {", ".join(sorted(unknown))}')
    columns = [i for i, build in enumerate(_BUDGETS[:width]) if builds is None or build in builds]
    changes, edits = Budgets(), []
    for section, code, fields, span in records:
        if (cost := expected.get(code)) is None or sections is not None and section not in sections: continue
        fields = [*fields, *['']*(width-len(fields))]
        new = _budget(section, cost, estimate)
        for i in columns:
            if new != fields[i]: changes.append((section, code, _BUDGETS[i], fields[i], new))
            fields[i] = new
        prefix = '⍝⍝: ' if path.suffix == '.bpl' else code+'\t'
        edits.append((*span, (prefix+'\t'.join(fields)).encode()))
    for start, end, replacement in reversed(edits): data = data[:start]+replacement+data[end:]
    if edits: path.write_bytes(data)
    return changes

_RUSTYMATH = _ROOT.parent/'rustymath'

@dataclass
class _Step:
    "One command that `prerelease` runs in the repository `cwd`, once the steps that `after` names have passed, with `env` added to its environment. It fails after `timeout` seconds. A `timing` step times code and runs alone after every other step."
    name: str
    cmd: list
    cwd: Path
    after: tuple = ()
    env: dict = field(default_factory=dict)
    timeout: float = 1800
    timing: bool = False

def _cargo_test(root, *args):
    "The command that runs `cargo test` with `args` in `root` through fastws's `run_tests`, and exits 1 when a test fails."
    return [sys.executable, '-c', f"import sys; from fastws.cargo import run_tests; r = run_tests({', '.join(map(repr, (str(root), *args)))}); print(r.text); sys.exit(not r.ok)"]

def _steps(out, node):
    "The steps of `prerelease`. Its browser package goes in `out/'wasm'`. `node` is the Node binary that the editor tests run under (`_node`)."
    B, R, wasm = _ROOT, _RUSTYMATH, str(out/'wasm')
    dev, rdev = 'basedpl: cargo develop', 'rustymath: cargo develop'
    return [
        _Step(dev, ['cargo', 'develop'], B, timeout=1200),
        _Step(rdev, ['cargo', 'develop'], R, timeout=1200),
        *(_Step(f'{r.name}: checked tests', _cargo_test(r, '--profile', 'checked', '--target-dir', str(r/'target'/'prerelease')), r, timeout=3600) for r in (B, R)),
        _Step('target builds', ['bpl-tests', 'target_builds', wasm], B, (dev,)),
        _Step('basedpl: pytest', ['pytest', '-q'], B, (dev,), timeout=1200),
        _Step('rustymath: pytest', ['pytest', '-q', '--ignore', 'tests/test_speed.py'], R, (rdev,), timeout=1200),
        *(_Step(f'basedpl: {t} test', [node, f'tests/{t}.mjs'], B, (dev,), timeout=300) for t in ('input', 'highlighting')),
        _Step('basedpl: browser tests', ['bpl-tests', 'wasm_test', wasm], B, ('target builds',), timeout=900),
        _Step('basedpl: BPL notebooks', ['bpl-nb', '--save'], B, (dev,)),
        _Step('basedpl: nbdev', ['nbdev-prepare'], B, (dev, 'basedpl: BPL notebooks')),
        _Step('rustymath: nbdev', ['nbdev-prepare'], R, (rdev,)),
        _Step('basedpl: cargo test', _cargo_test(B), B, (dev,), dict(BASEDPL_COSTS='1'), 3600, True),
        _Step('rustymath: cargo test', _cargo_test(R), R, (rdev,), dict(RUSTYMATH_COSTS='1'), 3600, True),
        _Step('rustymath: Python speed test', ['pytest', '-q', 'tests/test_speed.py'], R, (rdev,), dict(RUSTYMATH_COSTS='1'), 600, True),
        _Step('basedpl: thread scaling', ['bpl-tests', 'thread_scaling'], B, (dev,), timing=True),
        _Step('basedpl: browser speed test', ['bpl-tests', 'wasm_test', wasm, '--speed'], B, ('target builds',), timeout=900, timing=True)]

def _preflight():
    "What `prerelease` reads before it starts: notes for its summary, the problems that stop it, and the Node binary that `_node` chooses."
    version = lambda root: tomllib.loads((root/'Cargo.toml').read_text())['workspace']['package']['version']
    rm = version(_RUSTYMATH)
    notes, problems, node = [f'basedpl {version(_ROOT)}, rustymath {rm}'], [], 'node'
    need = tomllib.loads((_ROOT/'Cargo.toml').read_text())['workspace']['dependencies']['rustymath']['version']
    if re.search(rf'>=\s*{re.escape(rm)}\b', need): notes.append(f'basedpl needs rustymath {rm}, which is unreleased, so rustymath ships first')
    if not (_ROOT/'node_modules').is_dir(): problems.append('`tests/highlighting.mjs` reads `shiki` from `node_modules`, which `npm ci` installs')
    try: node = _node()
    except ValueError as e: problems.append(str(e))
    notes.append(f'Node: {node}')
    return notes, problems, node

def _run(steps, logs, jobs, keep_going):
    "Run `steps`, each once its inputs passed, logging output in `logs`. At most `jobs` run at once (None: no limit); timing steps run alone after the rest. Give `(name, status, seconds, log)` rows. Own each step's process group unless already in a managed job. Clean owned groups on every exit, even after a leader exits. A nested failure ends the containing job; otherwise `keep_going` permits independent steps after a failure."
    if unknown := {a for s in steps for a in s.after} - {s.name for s in steps}: raise ValueError(f'No step is named {", ".join(sorted(unknown))}')
    rows, running, pending = {}, {}, list(steps)
    owner = not os.environ.get(JOB_ENV)
    def end(name, status):
        p, f, start, _ = running.pop(name)
        if owner: kill_group(p)
        elif p.poll() is None: p.kill(); p.wait()
        f.close()
        rows[name] = (name, status, time.monotonic()-start, f.name)
    try:
        while pending or running:
            for name, (p, _, start, s) in list(running.items()):
                if p.poll() is not None: end(name, 'failed' if p.returncode else 'ok')
                elif time.monotonic()-start > s.timeout: end(name, 'timed out')
            failed = [r for r in rows.values() if r[1] != 'ok']
            if failed and not owner: raise RuntimeError(f'{failed[0][0]} {failed[0][1]}; stopping the containing job')
            if failed and not keep_going: break
            for s in [s for s in pending if any(rows[a][1] != 'ok' for a in s.after if a in rows)]:
                rows[s.name] = (s.name, 'not run', 0, '')
                pending.remove(s)
            for s in list(pending):
                if not all(a in rows for a in s.after) or (s.timing and (running or any(not t.timing for t in pending))): continue
                if not s.timing and jobs and len(running) >= jobs: continue
                f = open(logs/f"{re.sub(r'\W+', '-', s.name).strip('-')}.log", 'w')
                try: p = subprocess.Popen(s.cmd, cwd=s.cwd, env=os.environ | s.env | {JOB_ENV: '1'}, stdout=f, stderr=subprocess.STDOUT, start_new_session=owner)
                except BaseException: f.close(); raise
                running[s.name] = (p, f, time.monotonic(), s)
                pending.remove(s)
            if not running: break
            time.sleep(0.5)
    finally:
        for name in list(running): end(name, 'stopped')
    rows |= {s.name: (s.name, 'not run', 0, '') for s in pending}
    return [rows[s.name] for s in steps]

class Prerelease(list):
    "Each step of a `prerelease` run, as `(name, status, seconds, log)` rows, and the run's notes."
    def __init__(self, rows=(), notes=()):
        super().__init__(rows)
        self.notes = notes
    def __repr__(self): return '\n'.join([*(f'{status:9} {seconds:5.0f} s  {name}  {log}' for name, status, seconds, log in self), *self.notes])

def prerelease(
    keep_going:bool=False, # After a failure, run every step whose inputs passed and report every failure, as CI does? Otherwise the first failure ends the run
    jobs:int=None, # The most steps that run at once before the timings; None: every step that's ready
    out:Path=None, # The directory for the run's logs and browser package; None: a new directory under the system's temporary directory
    dry_run:bool=False, # List the steps in their order without running them?
):
    "Run basedpl's and rustymath's checks and saves before a release, as DEV.md's Release section describes. Gives each step's status, time and log. A step that doesn't pass raises `ValueError` with that summary and the end of the step's log."
    notes, problems, node = _preflight()
    out = Path(out).expanduser() if out else Path(tempfile.gettempdir())/time.strftime('prerelease-%Y%m%d-%H%M%S')
    steps, cleans = _steps(out, node), [_Step(f'{r.name}: clean', ['nbdev-clean'], r, timeout=300) for r in (_ROOT, _RUSTYMATH)]
    if dry_run:
        listed = lambda ss: [f'  {s.name}: {" ".join(s.cmd)}' + (f'  (after {", ".join(s.after)})' if s.after else '') for s in ss]
        return Report('\n'.join([*notes, *(f'Problem: {p}' for p in problems), 'In parallel, each after the steps it reads:', *listed(s for s in steps if not s.timing),
                                 'Then one at a time, with nothing else running:', *listed(s for s in steps if s.timing), 'Then in both repositories, even after a failure:', *listed(cleans)]))
    if problems: raise ValueError('\n'.join(problems))
    out.mkdir(parents=True, exist_ok=True)
    rows = _run(steps, out, jobs, keep_going) + _run(cleans, out, None, True)
    result = Prerelease(rows, notes)
    if bad := [r for r in rows if r[1] != 'ok']:
        name, _, _, log = next((r for r in bad if r[3]), bad[0])
        raise ValueError(f'\n{result!r}' + (f'\n\nThe end of the log of {name}, {log}:\n' + '\n'.join(Path(log).read_text().splitlines()[-40:]) if log else ''))
    return result

_ENGINE = 'graph fusion plan pervasive agreement program jit array number element error settings explain keyed'.split()
_USE = re.compile(r'\buse\s+(crate::|super::)?([^;]*);')
_RUST_PATH = re.compile(r'\b(crate|super)::((?:\w+::)*\w+)')

def _use_paths(tree, prefix=()):
    "The paths that a Rust `use` tree names, each a tuple of segments, without aliases."
    tree = tree.strip()
    if tree.startswith('{'):
        paths, depth, start = [], 0, 1
        for i, ch in enumerate(tree):
            depth += (ch == '{') - (ch == '}')
            if (depth == 1 and ch == ',') or depth == 0:
                if tree[start:i].strip(): paths += _use_paths(tree[start:i], prefix)
                start = i + 1
        return paths
    head, brace, rest = tree.partition('::{')
    if brace: return _use_paths('{' + rest, prefix + tuple(head.split('::')))
    return [prefix + tuple(re.sub(r'\s+as\s+\w+$', '', tree).split('::'))]

def module_edges(
    engine:Annotated[str, {'nargs':'+'}]=None, # Top-level modules that count as the engine; None: those of meta/crate-split-2026-10-31.md
    root:Path=_ROOT, # The crate whose `src` to scan
):
    "Count the references from the engine's modules into the crate's other modules, and find the largest cycle of references between top-level modules. A text scan of `crate::` and `super::` paths and `use` trees, with `lib.rs`'s re-exports resolved: it counts paths, not distinct items."
    engine, src = set(engine or _ENGINE), Path(root).expanduser()/'src'
    texts = {'::'.join(p.relative_to(src).with_suffix('').parts): re.sub(r'(?m)//.*$', '', p.read_text()) for p in src.rglob('*.rs')}
    lib = texts.pop('lib')
    tops = {m.split('::')[0] for m in texts}
    def module(path): return None if not path or path[0] not in tops else '::'.join(path[:2]) if '::'.join(path[:2]) in texts else path[0]
    reexported = {p[-1]: m for _, tree in _USE.findall(lib) for p in _use_paths(tree) if (m := module(p))}
    refs = defaultdict(lambda: defaultdict(int))
    for m, text in texts.items():
        paths = [(kind, p) for kind, tree in _USE.findall(text) if kind for p in _use_paths(tree)]
        paths += [(kind, tuple(path.split('::'))) for kind, path in _RUST_PATH.findall(_USE.sub('', text))]
        for kind, path in paths:
            if kind.startswith('super'): path = tuple(m.split('::')[:-1]) + path
            target = reexported.get(path[0]) or module(path)
            if target and target != m: refs[m][target] += 1
    top = lambda m: m.split('::')[0]
    graph, lines = defaultdict(set), defaultdict(int)
    for m, targets in refs.items(): graph[top(m)] |= {top(t) for t in targets} - {top(m)}
    for m, text in texts.items(): lines[top(m)] += len(text.splitlines())
    index, low, stack, cycles = {}, {}, [], []
    def visit(v):
        index[v] = low[v] = len(index); stack.append(v)
        for w in graph[v]:
            if w not in index: visit(w); low[v] = min(low[v], low[w])
            elif w in stack: low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            cycle = [stack.pop()]
            while cycle[-1] != v: cycle.append(stack.pop())
            cycles.append(cycle)
    for v in sorted(tops - index.keys()): visit(v)
    cycle = max(cycles, key=len)
    into = defaultdict(lambda: defaultdict(int))
    for m, targets in refs.items():
        for t, n in targets.items():
            if top(m) in engine and top(t) not in engine: into[t][m] += n
    rows = [f"{sum(n for c in into.values() for n in c.values())} references from {len({m for c in into.values() for m in c})} of the engine's modules into others:"]
    for t, c in sorted(into.items(), key=lambda tc: -sum(tc[1].values())):
        rows.append(f'{sum(c.values()):5} {t}: ' + ', '.join(f'{m} {n}' for m, n in sorted(c.items(), key=lambda mn: -mn[1])))
    rows.append(f'Largest cycle: {len(cycle)} of {len(tops)} top-level modules, {sum(lines[m] for m in cycle):,} of {sum(lines.values()):,} lines: {" ".join(sorted(cycle))}')
    return Report('\n'.join(rows))

_CRATES = {'basedpl': ('src', 'py', 'cli', 'wasm', 'xtask'), 'rustymath': ('src', 'py', 'buffer')}
_SKIPPED_DIRS = ['target', 'links', 'node_modules', '_proc', '_docs', 'meta']
_FN = re.compile(r'^(\s*)(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?fn\s+(\w+)')
_IMPL = re.compile(r'^(\s*)(?:unsafe\s+)?(impl|trait|pub(?:\([^)]*\))?\s+trait)\b')
# Attributes through which a test runner, Python or JavaScript reaches a function by another route than its name.
_REACHED = re.compile(r'#\[(?:test|bench|pyfunction|pymethods|pymodule|pyclass|wasm_bindgen|no_mangle|export_name|unsafe\((?:no_mangle|export_name))')

def _sources(base):
    "The Rust, Python, JavaScript and TOML files under `base`, outside build output, linked checkouts, notebooks' copies and notes."
    for top, dirs, files in os.walk(base):
        dirs[:] = [d for d in dirs if d not in _SKIPPED_DIRS and not d.startswith('.')]
        yield from (Path(top)/f for f in files if Path(f).suffix in ('.rs', '.py', '.js', '.mjs', '.toml'))

def _code(line):
    "`line` without a trailing `//` comment outside a string."
    i = line.find('//')
    while i >= 0 and line[:i].count('"') % 2: i = line.find('//', i + 2)
    return line if i < 0 else line[:i]

def _test_lines(lines):
    "The numbers, from 1, of `lines` inside an item marked `#[test]` or `#[cfg(test)]`, its attributes included."
    marked, marking, inside, depth, opened = set(), False, False, 0, False
    for n, line in enumerate(lines, 1):
        s, code = line.lstrip(), _code(line)
        if not inside:
            if s.startswith('#[') and (marking or re.match(r'#\[(?:cfg\()?test\)?\]', s)): marked.add(n); marking = True; continue
            if not marking or not s: continue
            inside, depth, opened, marking = True, 0, False, False
        marked.add(n)
        depth += code.count('{') - code.count('}')
        opened = opened or '{' in code
        if (opened and depth <= 0) or (not opened and code.rstrip().endswith(';')): inside = False
    return marked

def _is_test(path):
    "Whether `path` is a test file: under a `tests` directory, or a Python file named `test_*.py`."
    return 'tests' in path.parts or path.name.startswith('test_')

def dead_functions(
    root:Path=_ROOT, # basedpl's checkout, with rustymath's beside it
    tested:bool=False, # Also report functions that only tests name: references from test files, `#[test]` functions and `#[cfg(test)]` modules don't count
):
    "Functions whose name appears nowhere else: each `fn` in basedpl's and rustymath's crates whose name no other place in either repository's Rust, Python, JavaScript or TOML files uses. A text scan by name, so a name shared with anything else counts as used. It leaves out `main`, tests and their helpers, Python's dunder methods, a trait implementation's methods, and functions that Python, JavaScript or the linker reach through an attribute."
    roots = {'basedpl': Path(root).expanduser(), 'rustymath': Path(root).expanduser().parent/'rustymath'}
    defs, words = [], defaultdict(int)
    for repo, base in roots.items():
        for path in _sources(base):
            if tested and _is_test(path.relative_to(base)): continue
            lines = path.read_text().splitlines()
            tests = _test_lines(lines) if path.suffix == '.rs' else set()
            counted = [_code(l) for n, l in enumerate(lines, 1) if not (tested and n in tests)] if path.suffix == '.rs' else lines
            for w in re.findall(r'\b[A-Za-z_]\w*\b', '\n'.join(counted)): words[w] += 1
            if path.suffix != '.rs' or path.relative_to(base).parts[0] not in _CRATES[repo]: continue
            blocks, attrs = [], ''
            for n, line in enumerate(lines, 1):
                if not line.strip() or n in tests: continue
                indent = len(line) - len(line.lstrip())
                while blocks and blocks[-1][0] >= indent and not line.lstrip().startswith(('#', 'where', '+', ')')): blocks.pop()
                if line.lstrip().startswith('#['): attrs += line; continue
                if m := _IMPL.match(line):
                    header = ' '.join(lines[n-1:n+4]).split('{')[0]
                    reached = m[2] == 'impl' and (re.search(r'\sfor\s', header) or _REACHED.search(attrs))
                    blocks.append((indent, bool(reached)))
                elif (m := _FN.match(line)) and m[2] != 'main' and not m[2].startswith('__') and not _REACHED.search(attrs) and not (blocks and blocks[-1][1]):
                    defs.append((path.relative_to(base.parent), n, m[2]))
                attrs = ''
    counts = defaultdict(int)
    for _, _, name in defs: counts[name] += 1
    dead = [(p, n, name) for p, n, name in defs if words[name] <= counts[name]]
    reach = 'nothing outside tests names' if tested else 'nothing else names'
    return Report('\n'.join([f'{len(dead)} function{"s" * (len(dead) != 1)} that {reach}:'] + [f'{p}:{n} {name}' for p, n, name in sorted(dead)]))



_COMMANDS = 'check_file check_page time_cases instructions instruction_budgets profile profile_diff call_profile disasm jit_disasm speed_costs thread_scaling cargo_costs pytest_costs run_pytest compare_costs bisect explain_diff wasm_test wasm_stack target_builds prerelease module_edges dead_functions'.split()

def main():
    "Run a function in `_COMMANDS` from the shell, as `bpl-tests NAME ARGUMENT...`, and print its result. `bpl-tests NAME -h` lists NAME's arguments."
    if len(sys.argv) < 2 or sys.argv[1] not in _COMMANDS: sys.exit(f'usage: bpl-tests NAME ARGUMENT..., where NAME is one of {", ".join(_COMMANDS)}')
    name = sys.argv.pop(1)
    sys.argv[0] = f'bpl-tests {name}' # The program name in argparse's usage line
    f = globals()[name]
    args, kwargs, _ = parse_cli(f)
    print(repr(f(*args, **kwargs)))
