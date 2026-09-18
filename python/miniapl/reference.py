"""Search and edit the reference inventory without displaying full fixtures."""
import json, re
from pathlib import Path


def library_definitions(path):
    "Read column-zero named definitions from an APL library, preserving strings and nested definitions."
    text = re.sub(r"'(?:''|[^'])*'|⍝[^\n]*", lambda m: '' if m[0].startswith('⍝') else m[0], Path(path).read_text())
    starts = list(re.finditer(r'^(\w+)\s*←', text, re.M))
    result = {}
    for i, start in enumerate(starts):
        end = starts[i+1].start() if i+1 < len(starts) else len(text)
        result[start[1]] = '\n'.join(line.rstrip() for line in text[start.start():end].strip().splitlines() if line.strip())
    return result


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
