"Parse BPL source into a tree of nodes, for tools that read or rewrite source."
from dataclasses import dataclass
from . import BplError, symbols
from ._core import _parse

_separators = '⁝' + ''.join(next(s['hanzi'] for s in symbols if s['glyph'] == '⁝'))
_unseparate = str.maketrans('', '', _separators)

def edit(text, edits):
    "`text` with each `(span, new)` pair in `edits` replaced. Spans are character offsets into `text`. They must lie in `text` and not overlap."
    out, pos = [], 0
    for (a, b), new in sorted(edits):
        if a < pos or b > len(text): raise ValueError(f'edit {a}:{b} overlaps an earlier edit or lies outside the text')
        out += [text[pos:a], new]
        pos = b
    return ''.join(out) + text[pos:]

@dataclass(frozen=True)
class Node:
    "A parse tree node: its kind, its character span in the parsed text, its own source text, its child nodes, and the `BplError` of an `Error` node."
    kind:str
    span:tuple
    text:str
    children:tuple
    error:BplError|None = None

    def walk(self):
        "This node, then each descendant, in source order."
        yield self
        for child in self.children: yield from child.walk()

    def edit(self, edits):
        "This node's text with each `(span, new)` pair in `edits` replaced. Spans are offsets in the parsed text, like `Node.span`. They must lie in this node and not overlap."
        start = self.span[0]
        return edit(self.text, (((a-start, b-start), new) for (a, b), new in edits))

    def _lines(self, depth):
        yield f'{"  "*depth}{self.kind} {self.span[0]}:{self.span[1]} {self.text!r}'
        for child in self.children: yield from child._lines(depth+1)

    def __repr__(self): return '\n'.join(self._lines(0))

def parse(text):
    "The parse tree of BPL `text`, as a `Program` node. Lines that don't parse become `Error` nodes."
    def node(kind, start, end, children, error):
        return Node(kind, (start, end), text[start:end], tuple(node(*c) for c in children), error and BplError(error, []))
    return node(*_parse(text))

def linear(text):
    "The full linear form of BPL `text`: the ordinary APL that BPL reads. Each strand of literals is in brackets, and each run of several tokens is in parentheses. A bracket list whose items hold several tokens has ` ; ` between its items. A literal that touches the name before it, as the `1` in `x1` does, is in parentheses."
    def form(n, before=None):
        if n.kind in ('Comment', 'Error'): return n.text
        gap = text[before.span[1]:n.span[0]] if before else None
        separated = bool(gap) and not gap.strip(_separators)
        if n.kind in ('Name', 'System', 'Literal'):
            own = n.text if n.kind != 'Literal' or n.text.startswith(('"', "'")) else n.text.translate(_unseparate)
            return f'({own})' if separated or (n.kind == 'Literal' and before and before.kind == 'Name' and gap == '') else own
        cells = n.children
        gaps = zip([n.span[0], *(c.span[1] for c in cells)], [*(c.span[0] for c in cells), n.span[1]])
        clear = [((a, b), text[a:b].translate(_unseparate)) for a,b in gaps if any(c in _separators for c in text[a:b])]
        if n.kind == 'ArrayLiteral' and n.text.startswith('[') and any(len(c.children) > 1 for c in cells) and all(text[a.span[1]:b.span[0]].isspace() for a, b in zip(cells, cells[1:])):
            return n.edit([((cells[0].span[0], cells[-1].span[1]), ' ; '.join(map(form, cells))), *clear])
        own = n.edit([(c.span, form(c, b)) for b, c in zip([None, *cells], cells)] + clear)
        if n.kind == 'Strand': return f'[{own}]'
        return f'({own})' if n.kind == 'Run' or (n.kind == 'Group' and not n.text.startswith('(')) else own
    return form(parse(text))
