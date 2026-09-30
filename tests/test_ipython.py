from IPython.core.interactiveshell import InteractiveShell
from IPython.core.completer import provisionalcompleter
from basedpl import bpl, fn, plus
from basedpl.ipython import load_ipython_extension


def test_ipython_inspection_and_completion():
    shell = InteractiveShell.instance()
    load_ipython_extension(shell)
    source = '{⍝ Double the argument\n⎕←99 ⋄ 2×⍵}'
    bpl('twice←'+source)
    f = fn('twice')
    shell.user_ns.update(bpl=bpl, f=f, fn=fn)
    help = shell.inspector._get_info(f, detail_level=0)
    assert 'Double the argument' in help['text/plain']
    assert 'Calls:' in help['text/plain']
    source_help = shell.inspector._get_info(f, detail_level=1)
    assert source in source_help['text/plain']
    assert 'class Function' not in source_help['text/plain']
    assert 'adds' in shell.inspector._get_info(plus)['text/plain']
    for line in ['bpl["tw', 'bpl.fn("tw', 'bpl("tw', 'fn("tw']:
        with provisionalcompleter(): matches = list(shell.Completer.completions(line, len(line)))
        assert 'twice' in [m.text for m in matches]
    assert shell.inspector._get_info(1)['text/plain']
    del shell.user_ns['bpl'], shell.user_ns['f'], shell.user_ns['fn']
