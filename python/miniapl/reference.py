"""Import, capture, review and activate reference cases; search and edit their inventory."""
import base64, csv, json, re, zlib
from collections import Counter
from fractions import Fraction
from pathlib import Path


def source_definitions(text):
    "Read top-level named assignments from APL source, ignoring comments and preserving nested definitions."
    text = re.sub(r"'(?:''|[^'])*'|⍝[^\n]*", lambda m: '' if m[0].startswith('⍝') else m[0], text)+'\n'
    result, start, depth = {}, 0, 0
    for token in re.finditer(r"'(?:''|[^'])*'|[(){}\[\]⋄\n]", text):
        c = token[0]
        if c in ('⋄', '\n') and depth==0:
            code = text[start:token.start()].strip()
            if match := re.match(r'^(\w+)\s*←', code):
                result[match[1]] = '\n'.join(line.rstrip() for line in code.splitlines() if line.strip())
            start = token.end()
        elif c in ('(', '{', '['): depth += 1
        elif c in (')', '}', ']'): depth -= 1
    return result


def library_definitions(path):
    "Read named definitions from an APL library."
    return source_definitions(Path(path).read_text())


def library_dependencies(definitions, code):
    "Select transitive name references for review, ignoring strings/comments but not resolving lexical shadowing."
    selected, pending = set(), [code]
    while pending:
        clean = re.sub(r"'(?:''|[^'])*'|⍝[^\n]*", '', pending.pop())
        names = set(re.findall(r'\b[^\W\d]\w*\b', clean)) & definitions.keys() - selected
        selected.update(names)
        pending.extend(definitions[name] for name in names)
    return {name: code for name,code in definitions.items() if name in selected}


class Corpus:
    def __init__(self, directory='tests/reference/inventory'):
        self.directory = Path(directory)

    def _read(self):
        return {p: [json.loads(line) for line in p.read_text().splitlines()] for p in sorted(self.directory.glob('*.jsonl'))}

    def find(self, pattern='', status='', source='', limit=20):
        """Search full IDs/code/reasons; return 180-character previews. Index by ID for full fields; limit=None returns all matches."""
        result = {}
        for path, rows in self._read().items():
            if source and path.stem != source: continue
            for row in rows:
                if status and row['status'] != status: continue
                text = f"{row['status']}: {row['code']}"
                if reason := row.get('reason'): text += f' ⍝ {reason}'
                if not re.search(pattern, row['id']+' '+text): continue
                if limit is not None and len(result) >= limit: return result
                preview = text.replace('\n', ' ¶ ')
                result[row['id']] = preview if len(preview) <= 180 else preview[:179]+'…'
        return result

    def __getitem__(self, key):
        """Read corpus[id, *fields]; default to code/status/reason. '*' reads the full record; missing fields are omitted."""
        id, *fields = (key,) if isinstance(key, str) else key
        return self.get_many([id], *fields)[id]

    def get_many(self, ids, *fields):
        """Read a batch with the same field selection as indexing, keyed by ID."""
        ids, result = set(ids), {}
        for rows in self._read().values():
            for row in rows:
                if row['id'] in ids:
                    result[row['id']] = row if fields == ('*',) else {k: row[k] for k in fields or ('code', 'status', 'reason') if k in row}
        if missing := ids - result.keys(): raise KeyError(sorted(missing))
        return result

    def update(self, ids, remove=(), **fields):
        """Patch one ID or an iterable of IDs. None is a JSON value; use remove to delete fields."""
        return self.update_many({id: fields for id in ([ids] if isinstance(ids, str) else ids)}, remove=remove)

    def update_many(self, changes, remove=()):
        """Patch {ID: fields} against fresh files; return changed field names. Unknown IDs write nothing."""
        files = self._read()
        known = {row['id'] for rows in files.values() for row in rows}
        if missing := changes.keys() - known: raise KeyError(sorted(missing))
        if 'id' in remove or any('id' in fields for fields in changes.values()): raise ValueError('case IDs are stable')
        result = {}
        for path, rows in files.items():
            changed = False
            for row in rows:
                if row['id'] not in changes: continue
                before = row.copy()
                for key in remove: row.pop(key, None)
                row.update(changes[row['id']])
                keys = sorted(k for k in before.keys() | row.keys() if k not in before or k not in row or before[k] != row[k])
                if keys:
                    result[row['id']] = keys
                    changed = True
            if changed: path.write_text(''.join(json.dumps(row, ensure_ascii=False, allow_nan=False, separators=(',', ':'))+'\n' for row in rows))
        return result


def lisp_tokens(text):
    """Token positions, including strings and character literals; omit Lisp comments."""
    pattern = r'#\|[\s\S]*?\|#|;[^\n]*|"(?:\\[\s\S]|[^"\\])*"|#\\(?:[()\s]|[^\s()]+)|#\d+[aA]|#(?=\()|[()]|[^\s()";]+'
    return [m for m in re.finditer(pattern, text) if not m[0].startswith((';', '#|'))]


def lisp_string(token):
    return re.sub(r'\\([\s\S])', r'\1', token[1:-1])


def array(shape, data, prototype=None):
    if prototype is None: prototype = fill(data[0]) if data else 0
    return dict(shape=shape, data=data, prototype=prototype)


def fill(value):
    if isinstance(value, dict) and 'shape' in value:
        return array(value['shape'], [fill(x) for x in value['data']], fill(value['prototype']))
    return ' ' if isinstance(value, str) else 0


def item(value):
    if isinstance(value, dict) and value.get('shape') == [] and not (isinstance(value['data'][0], dict) and 'shape' in value['data'][0]):
        return value['data'][0]
    return value


def lisp_expected(text):
    """Read April's literal expectations, not arbitrary Common Lisp expressions."""
    tokens = iter(m[0] for m in lisp_tokens(text))
    def read(token=None):
        token = next(tokens) if token is None else token
        if token == '(':
            values = []
            for token in tokens:
                if token == ')': return values
                values.append(read(token))
            raise ValueError('unclosed Lisp list')
        if token.startswith('"'):
            return array([len(s := lisp_string(token))], list(s), ' ')
        if token == '#':
            values = read()
            return array([len(values)], [item(x) for x in values])
        if token.lower() == '#c':
            real, imag = read()
            return {'complex': [real, imag]} if imag else real
        if match := re.fullmatch(r'#(\d+)[aA]', token):
            rank, values = int(match[1]), read()
            if rank == 0: return array([], [item(values)])
            shape, data = [], values
            for _ in range(rank):
                shape.append(len(data))
                data = data[0] if data else []
            data = values
            for _ in range(rank - 1): data = [x for row in data for x in row]
            return array(shape, [item(x) for x in data])
        if token.startswith('#*'): return array([len(token) - 2], [int(c) for c in token[2:]])
        if token.startswith('#\\'):
            name = token[2:]
            return {'Space': ' ', 'Newline': '\n', 'Return': '\r', 'Tab': '\t'}.get(name, name) if len(name) > 1 else name
        if re.fullmatch(r'[+-]?\d+/\d+', token): return float(Fraction(token))
        if re.fullmatch(r'[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eEdDfF][+-]?\d+)?', token):
            return float(re.sub('[dDfF]', 'e', token))
        raise ValueError(f'nonliteral Lisp expectation: {token}')
    value = read()
    if next(tokens, None) is not None: raise ValueError('additional Lisp expectation forms')
    return value if isinstance(value, dict) and 'shape' in value else array([], [value])


def scope(code):
    """Only explicit exclusions; uncertain language features remain review questions."""
    clean = re.sub(r"'(?:''|[^'])*'|⍝[^\n]*", '', code)
    if '«' in clean or '»' in clean: return 'excluded', 'host-language escapes are arbitrary FFI (§5.3)'
    files = 'NGET|NPUT|NREAD|NAPPEND|NREPLACE|NINFO|NTIE|NUNTIE|NCREATE|NDELETE|NCOPY|NMOVE|NEXISTS|NPARTS|NRESIZE|NLOCK|NERASE|NRENAME|NNAMES|NNUMS|NSIZE|NXLATE|MKDIR'
    files += '|FREAD|FAPPEND|FREPLACE|FSTAC|FSTIE|FTIE|FUNTIE|FCREATE|FERASE|FCOPY|FDROP|FRENAME|FRESIZE|FPROPS|FAVAIL|FSIZE|FNAMES|FNUMS|FLOCK|FHOLD|FLIB|FHIST|FCHK'
    if re.search(r'⎕(?:NS|CS|NEW|NA|CMD|SH|SHELL|CY|SAVE|LOAD|'+files+r')\b', clean, re.I):
        return 'excluded', 'namespaces, external calls, files or workspace facilities (§5.3)'
    if re.search(r'⎕(?:IO|CT|DCT|DIV|ML)\s*←', clean, re.I):
        return 'excluded', 'configurable origin/tolerance/division/migration modes; fixed policies (§3.1, §5.3)'
    if re.search(r'^\s*[)\]]\w|⎕SE\.|⎕(?:WC|WG|WN|NQ|DQ|OFF|CLASS|THIS|TPOOL|TALLOC|TGET|TPUT|TKILL|SPAWN)\b', clean, re.I):
        return 'excluded', 'system commands, GUI or interpreter process control (§5.3)'
    if '∞' in clean: return 'excluded', 'non-finite numeric values are outside the agreed finite-number policy'
    if '⍞' in clean or re.search(r'⎕[A-Za-z]+|⍠|⍫|«|»|\$\[|\{\s*\[|⍎|⍙|⍢|→', clean):
        return 'question', 'system facilities or dialect extension: see meta/reference-questions.md'
    return 'pending', 'not yet enabled; check prerequisites and expected result'


def record(source, line, code, **extra):
    status, reason = scope(code)
    return dict(id=f'{source}:{line}', code=code, status=status, reason=reason, **extra)


def ngn_cases(root):
    text = (root/'t.apl').read_text()
    cases = []
    for line, raw in enumerate(text.splitlines(), 1):
        if not raw.strip(): continue
        code, separator, expected = re.split(r'(←→|!!)', raw, maxsplit=1)
        case = record('ngn', line, code.strip(), upstream=raw, origin=0)
        if separator == '!!': case['expected_error'] = expected.strip()
        else: case['expected_apl'] = expected.strip()
        case['reason'] = 'zero-origin/prototype/dialect review; then enable' if case['status'] == 'pending' else case['reason']
        cases.append(case)
    for path in sorted((root/'examples').glob('*.apl')):
        cases.append(record(f'ngn/examples/{path.stem}', 1, path.read_text(), expected_output=path.with_suffix('.out').read_text(), origin=0))
    return cases


def april_file(path, source):
    text = path.read_text()
    tokens, stack, forms = lisp_tokens(text), [], []
    for i, token in enumerate(tokens):
        if token[0] == '(': stack.append(i)
        elif token[0] == ')':
            start = stack.pop()
            if tokens[start+1][0] in ('is', 'is-error', 'for', 'for-printed', 'provision'): forms.append((start, i))
    assert not stack
    cases = []
    for start, end in sorted(forms):
        head, first = tokens[start+1][0], start+2
        line = text.count('\n', 0, tokens[start].start()) + 1
        raw = text[tokens[start].start():tokens[end].end()]
        if not tokens[first][0].startswith('"'):
            cases.append(dict(id=f'{source}:{line}', upstream=raw, status='question', reason='host-wrapper assertion: extract its language behavior before excluding', code=''))
            continue
        if head == 'provision':
            case = record(source, line, lisp_string(tokens[first][0]), upstream=raw, role='setup')
            if case['status'] != 'excluded':
                case.update(status='setup', reason='Upstream demo setup; retained as source material for self-contained cases.')
            cases.append(case)
            continue
        if head == 'is-error':
            case = record(source, line, lisp_string(tokens[first][0]), upstream=raw, expected_error='')
            case['reason'] = 'upstream requires an error; classify its kind against Dyalog before enabling'
            cases.append(case)
            continue
        description = lisp_string(tokens[first][0]) if head != 'is' else ''
        if head != 'is': first += 1
        if not tokens[first][0].startswith('"'): raise ValueError(f'non-string April program at {line}')
        code = lisp_string(tokens[first][0])
        expected = text[tokens[first+1].start():tokens[end].start()].strip()
        case = record(source, line, code, description=description, upstream=raw, expected_lisp=expected)
        if head == 'for-printed':
            case['expected_output'] = lisp_string(expected)
            if case['status'] == 'pending': case.update(status='question', reason='April display/exact-number policy differs; review presentation contract')
        else:
            try: case['expected'] = lisp_expected(expected)
            except (ValueError, StopIteration, TypeError) as e: case['expectation_note'] = str(e)
        if '⋆' in code:
            case['code'] = re.sub(r"'(?:''|[^'])*'|⍝[^\n]*|⋆", lambda m: '*' if m[0] == '⋆' else m[0], code)
            case['adaptation'] = 'April power alias ⋆ written with standard APL *'
        cases.append(case)
    return cases


def april_cases(root):
    cases = april_file(root/'spec.lisp', 'april')
    for path in sorted(root.glob('**/demo.lisp')):
        extra = april_file(path, 'april/'+path.relative_to(root).as_posix())
        for case in extra:
            case['requires'] = path.relative_to(root).as_posix()
            if case['status'] == 'pending': case.update(status='question', reason='library/demo definitions and setup required; see meta/reference-questions.md')
        cases.extend(extra)
    return cases


def tio_program(url):
    """Decode TIO's offline permalink payload; do not contact or execute TIO."""
    if not url.startswith('https://tio.run/##'): return None
    payload = url.split('##', 1)[1].split('#', 1)[0].replace('@', '+')
    try:
        parts = zlib.decompress(base64.b64decode(payload + '=' * (-len(payload) % 4)), -15).split(b'\xff')
    except (ValueError, zlib.error, UnicodeError): return None
    return '\n'.join(p.decode() for p in parts[1:4] if p) if len(parts) > 1 and parts[0].startswith(b'apl-dyalog') else None


def aplcart_cases(root):
    cases = []
    for filename in ('table.tsv', 'tt.tsv'):
        with (root/filename).open() as f:
            reader = csv.reader(f, delimiter='\t', quoting=csv.QUOTE_NONE)
            for line, row in enumerate(reader, 1):
                if not row or row[0] == 'SYNTAX': continue
                row += [''] * (9-len(row))
                syntax, description, cls, kind, group, category, keywords, tio, docs = row[:9]
                case = record(f'aplcart/{filename}', line, syntax, description=description, category=category, kind=kind)
                case.update(syntax=syntax, source_class=cls, group=group, keywords=keywords, tio=tio, docs=docs)
                program = tio_program(tio)
                if program:
                    case['example'] = program
                    status, reason = scope(program)
                    if status in ('excluded', 'question'): case['example_note'] = reason
                if case['status'] == 'pending': case['reason'] = 'supply/review concrete arguments and independent expected result'
                cases.append(case)
    return cases


def import_sources(links):
    """Return every core assertion/demo and catalogue row in the three local snapshots."""
    links = Path(links)
    return dict(ngn=ngn_cases(links/'ngn'), april=april_cases(links/'april'), aplcart=aplcart_cases(links/'aplcart'))




REFERENCE_ENCODER = r'''
RefElement←{0≠≡⍵:'array'(RefArray ⍵) ⋄ ' '=⊃0⍴⍵:'char'⍵ ⋄ 0≠11○⍵:'complex'(9 11○⊂⍵) ⋄ 'number'⍵}
RefArray←{(⍴⍵)(RefElement¨,⍵)(RefElement⊃0⍴⍵)}
'''


def dyalog_expected(apl, expression):
    """Capture a reviewed expression through REFERENCE_ENCODER installed in Dyalog."""
    def element(tagged):
        tag, value = tagged
        if tag == 'array': return decode(value)
        if tag == 'complex': return {'complex': value}
        return value
    def decode(value):
        shape, data, prototype = value
        return dict(shape=shape, data=[element(x) for x in data], prototype=element(prototype))
    return decode(apl.pyval('RefArray ('+expression+')'))


def capture_ngn_expectations(apl, cases):
    """Evaluate only ngn's closed literal expectations, never arbitrary upstream code."""
    from aplnb.dyalog import AplError
    captured = 0
    for case in cases:
        expression = case.get('expected_apl', '')
        clean = re.sub(r"'(?:''|[^'])*'", '', expression)
        if not expression or not re.fullmatch(r'[\d\s.EeJj¯⍴⊂(),⍬]*', clean): continue
        try: case['expected'] = dyalog_expected(apl, expression)
        except AplError as e:
            case['expectation_note'] = 'Dyalog rejected the upstream expectation: '+str(e)
            continue
        case['oracle'] = 'upstream RHS captured in Dyalog 20.0.53963.0; IO=1 CT=1E-14 DIV=0 ML=1'
        captured += 1
    return captured


def capture_aplcart_examples(apl, cases):
    """Capture closed, small calculator examples; leave other recipes pending for review."""
    from aplnb.dyalog import AplError
    captured = 0
    for case in cases:
        if case['status'] != 'pending' or not case.get('example'): continue
        lines = [line.strip() for line in case['example'].splitlines() if line.strip()]
        if not all(line.startswith('⎕←') for line in lines): continue
        expressions = [line[2:].strip() for line in lines]
        code = expressions[0] if len(expressions) == 1 else '('+' ⋄ '.join(expressions)+')'
        clean = re.sub(r"'(?:''|[^'])*'", '', code)
        if not re.fullmatch(r'[\d\s.EeJj¯+\-×÷⍴,⊂⊃↑↓⌽⊖⍉≢=≠<≤>≥⍳/⌿\\⍀()⍬⋄]*', clean): continue
        if any(float(n) > 1000 for n in re.findall(r'\d+(?:\.\d*)?(?:[eE][¯-]?\d+)?', clean.replace('¯', '-'))): continue
        try: expected = dyalog_expected(apl, code)
        except AplError as e:
            case['expectation_note'] = 'Dyalog rejected the concrete example: '+str(e)
            continue
        case.update(code=code, expected=expected, adaptation='TIO output expressions collected as one value; original recipe and example retained',
                    oracle='concrete TIO example executed in Dyalog 20.0.53963.0; IO=1 CT=1E-14 DIV=0 ML=1')
        captured += 1
    return captured


def write_cases(directory, sources, replace=False):
    """Write data artifacts; replacing reviewed records requires an explicit flag."""
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    for source, cases in sources.items():
        with (directory/f'{source}.jsonl').open('w' if replace else 'x') as f:
            for case in cases: f.write(json.dumps(case, ensure_ascii=False, separators=(',', ':'))+'\n')
    return {source: len(cases) for source, cases in sources.items()}


def scan(directory='tests/reference/inventory', source='', match='', timeout=.25, report='meta/reference-scan.json'):
    "Check pending cases with independent expectations; never change fixture metadata."
    from miniapl.worker import Worker
    rows, inventory = [], Counter()
    worker = Worker()
    try:
        for path in sorted(Path(directory).glob('*.jsonl')):
            if source and path.stem != source: continue
            for line in path.read_text().splitlines():
                case = json.loads(line)
                if match and not re.search(match, case['id']+' '+case['code']): continue
                if case['status'] != 'pending':
                    inventory[case['status']] += 1
                    continue
                if 'expected' not in case and not case.get('expected_error'):
                    inventory['needs expectation'] += 1
                    continue
                clean = re.sub(r"'(?:''|[^'])*'|⍝[^\n]*", '', case['code'])
                if '?' in clean:
                    inventory['random: review separately'] += 1
                    continue
                try: result = worker.request(dict(case=case), timeout=timeout)
                except (TimeoutError, EOFError, BrokenPipeError) as e:
                    result = dict(status='worker failure', message=str(e), diagnostics=worker.diagnostics)
                    worker.close()
                    worker = Worker()
                rows.append(dict(file=str(path), case=case, **result))
    finally: worker.close()
    result = dict(inventory=dict(inventory), results=rows)
    path = Path(report)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n')
    print(f'{len(rows)} checked: {dict(Counter(r["status"] for r in rows))}')
    print(f'Not scanned: {dict(inventory)}')
    print(f'Review: {report}')
    return result


def selected(report, source='', match='', status='pass'):
    rows = json.loads(Path(report).read_text())['results']
    return [r for r in rows if (not source or Path(r['file']).stem == source)
            and (not status or r['status'] == status)
            and (not match or re.search(match, r['case']['id']+' '+r['case']['code']+' '+r.get('message', '')))]


def review(report, source='', match='', status='pass', limit=20, details=False):
    rows = selected(report, source, match, status)
    for row in rows[:limit or None]:
        case = row['case']
        print(f'{case["id"]} [{row["status"]}] {row.get("kind", "")} {row.get("message", "")}')
        print('  '+case['code'].replace('\n', ' ⋄ '))
        if details: print(json.dumps(row, ensure_ascii=False, indent=2))
    print(f'{len(rows)} matching; {min(limit or len(rows), len(rows))} shown')


def activate(report, source='', match='', output='tests/reference'):
    "Activate reviewed passing selections only if their saved fixture records are unchanged."
    from miniapl.apltests import add
    rows = selected(report, source, match)
    if not rows:
        print('Activated 0 reviewed cases')
        return
    updates = {r['case']['id']: r['case'] for r in rows}
    corpus = Corpus(Path(rows[0]['file']).parent)
    current = corpus.get_many(updates, '*')
    for id, case in updates.items():
        if current[id] != case: raise ValueError(f'{id}: fixture changed since scan; rescan before activation')
    add(list(updates), output, corpus.directory)
    print(f'Activated {len(updates)} reviewed cases')
