"""Search and edit the repository's JSONL acceptance corpus without displaying full fixtures."""
import json, re
from pathlib import Path


class Corpus:
    def __init__(self, directory='tests/reference'):
        self.directory = Path(directory)

    def _read(self):
        return {p: [json.loads(line) for line in p.read_text().splitlines()] for p in sorted(self.directory.glob('*.jsonl'))}

    def find(self, pattern='', status='', source='', limit=20):
        """Regex-search IDs, code and reasons; return concise entries keyed by ID. None means no limit."""
        result = {}
        for path, rows in self._read().items():
            if source and path.stem != source: continue
            for row in rows:
                if status and row['status'] != status: continue
                text = f"{row['status']}: {row['code']} ⍝ {row['reason']}"
                if not re.search(pattern, row['id']+' '+text): continue
                if limit is not None and len(result) >= limit: return result
                result[row['id']] = text
        return result

    def get(self, id, *fields):
        """Read selected fields; default to code/status/reason. '*' reads the full record; missing fields are omitted."""
        return self.get_many([id], *fields)[id]

    def get_many(self, ids, *fields):
        """Read a batch with get's field selection, keyed by ID."""
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
            if changed: path.write_text(''.join(json.dumps(row, ensure_ascii=False, separators=(',', ':'))+'\n' for row in rows))
        return result
