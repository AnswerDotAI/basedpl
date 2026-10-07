from pathlib import Path
from shutil import copyfile

src = Path(__file__).resolve().parents[1]/'python/basedpl'
for name in ('lb.js', 'input.js', 'layout.json', 'bpl.tmLanguage.json'):
    dest = Path('playground')/name
    if not dest.exists() or dest.read_bytes() != (src/name).read_bytes(): copyfile(src/name, dest)
