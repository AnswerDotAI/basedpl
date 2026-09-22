"IPython help and completion for APL functions and sessions."
import re
from IPython.core.completer import context_matcher, SimpleCompletion
from . import Function, Session
from ._core import _help_command


def inspection_mime(info, detail=False):
    "Render structured APL inspection as an IPython MIME bundle."
    text = info['source' if detail else 'help']
    return {'text/plain':text, 'text/markdown':f'```apl\n{text}\n```' if detail else text}


def command_help(session, code):
    "Return inspection for a ]help command, or None for ordinary evaluation."
    if (query := _help_command(code)) is None: return None
    name, detail = query
    if (info := session.inspect(name)) is not None: return inspection_mime(info, detail)


def load_ipython_extension(ip):
    "Show APL help/source for ?/?? and complete names inside session strings."
    if getattr(ip.inspector, '_basedpl', False): return
    original = ip.inspector._get_info

    def inspect(obj, oname='', formatter=None, info=None, detail_level=0, omit_sections=()):
        if not isinstance(obj, Function):
            return original(obj, oname=oname, formatter=formatter, info=info, detail_level=detail_level, omit_sections=omit_sections)
        return inspection_mime(obj.inspect(), detail_level)

    @context_matcher()
    def complete(context):
        before = context.text_until_cursor
        match = re.search(r'''\b([a-zA-Z_]\w*)\s*(?:\.fn\s*\(|\[|\()\s*(['"])([^'"\n]*)$''', before)
        if not match or not isinstance(session := ip.user_ns.get(match[1]), Session): return {'completions':[]}
        prefix = re.search(r'[\w•∆⍙]*$', match[3])[0]
        return dict(completions=[SimpleCompletion(n, type='APL name') for n in session.complete(prefix)], matched_fragment=prefix, suppress=True)

    ip.inspector._get_info = inspect
    ip.inspector._basedpl = True
    ip.Completer.custom_matchers.append(complete)
