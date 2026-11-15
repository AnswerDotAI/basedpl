"""nbdev's documentation inventory for Python definitions and BPL glyphs."""

from . import symbols
from ._modidx import d as _python

_settings = _python['settings']
_base = _settings['doc_host'].rstrip('/') + '/' + _settings['doc_baseurl'].strip('/')
d = dict(settings=_settings, syms=_python['syms'] | {'basedpl.glyphs': {
    f"basedpl.glyphs.{s['glyph']}": f"{_base}/glyphs/{s['name']}.html" for s in symbols}})
