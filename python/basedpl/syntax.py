"Parse BPL source into a tree of nodes, for tools that read or rewrite source."
from dataclasses import dataclass
from . import BplError
from ._core import _parse

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
