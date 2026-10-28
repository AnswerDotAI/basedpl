"""Write editor highlighters, glyph-entry templates, regional macOS keyboards, and glyph-page kinds, keys, hanzi and ranks.

BPL glyph lists come from its symbol metadata. Dyalog APL and BQN lists for the comparison page are defined here. `keyboards` generates the macOS bundles. Paths are relative to the repository root.

`write()` updates all generated editor files during release preparation, including the BPL and Hanzi TextMate grammars beside this module, exported from the Quarto XML, and the JetBrains bundle and templates. The Hanzi grammar includes BPL and adds its aliases from the same symbol metadata. `icon()` rebuilds the common input-menu icon when its design changes; it needs Pillow and macOS's `iconutil`. Keyboard generation copies the icon without rebuilding it."""
import json, re, subprocess, tempfile
from pathlib import Path
from xml.etree import ElementTree as ET
from . import symbols, keyboards, windows_keyboard
from ._core import _superscripts, _subscripts, _number_marks
from .assets import kernel_spec

VIM = Path('editors/vim/syntax/bpl.vim')
JETBRAINS = Path('editors/jetbrains')
GLYPH_PAGES = Path('nbs/glyphs')
KEY_NAMES = {char: f'Shift-{label}' for char, label in keyboards.SHIFT_KEYS.items()} | {'-': 'Minus', '\\': 'Backslash', '`': 'Backtick'}
FONT, MENLO_BOLD, PURPLE = '/System/Library/Fonts/Menlo.ttc', 1, (61, 31, 107)

# Each highlight class: the Kate style that colours it in every highlighter, the BPL symbol kind it covers, and the script characters it adds.
CLASSES = {'Function': ('dsFunction', 'function', ''), 'MonadicOperator': ('dsOperator', 'monadic-operator', _superscripts),
           'DyadicOperator': ('dsExtension', 'dyadic-operator', ''), 'Argument': ('dsVariable', 'argument', ''), 'Keyword': ('dsKeyword', 'syntax', _subscripts)}
# The characters each class colours in each language. The Dyalog APL and BQN tables serve the comparison page.
SPELLINGS = {s['glyph']: ''.join(dict.fromkeys(s['glyph'] + ''.join(s['hanzi']))) for s in symbols}
BPL = {cls: ''.join(s['glyph'] for s in symbols if s['kind'] == kind) + scripts for cls, (_, kind, scripts) in CLASSES.items()}
HANZI = {cls: ''.join(SPELLINGS[s['glyph']][1:] for s in symbols if s['kind'] == kind) for cls, (_, kind, _) in CLASSES.items()}
TOKEN_PATTERNS = {'System': r'•[A-Za-z_∆⍙][A-Za-z0-9_∆⍙]*', 'Empty': r'⍬{mark}?', 'Mark': r'{mark}',
                  'Number': r'(?:¯?∞|¯?(\d+(\.\d+)?|\.\d+)([eEₑ]¯?\d+)?([jJⱼ]¯?(\d+(\.\d+)?|\.\d+)([eEₑ]¯?\d+)?|[rᵣ]¯?\d+)?){mark}?|[ₑⱼᵣ]'}
DYALOG = {'Function': '+-×÷⌈⌊|*⍟○!?~∧∨⍲⍱<≤=≥>≠≡≢⍴,⍪⌽⊖⍉↑↓⊂⊃⊆⊇⌷⍋⍒⊤⊥⍳⍸∊⍷∪∩⊣⊢⍎⍕⌹', 'MonadicOperator': r'¨⍨/⌿\⍀⌸&⌶',
          'DyadicOperator': '∘⍤⍥⍣⍠⌺@.⍛', 'Argument': '⍺⍵∇', 'Keyword': '←→⋄:⎕⍞'}
BQN = {'Function': '+-×÷⋆√⌊⌈|¬∧∨<>≠=≤≥≡≢⊣⊢⥊∾≍⋈↑↓↕«»⌽⍉/⍋⍒⊏⊑⊐⊒∊⍷⊔!', 'MonadicOperator': '˙˜˘¨⌜⁼´˝`',
       'DyadicOperator': '∘○⊸⟜⌾⊘◶⎉⚇⍟⎊', 'Argument': '𝕨𝕩𝕗𝕘𝕤𝕣𝕎𝕏𝔽𝔾𝕊', 'Keyword': '←↩⇐⋄,‿·?:;'}
HIGHLIGHTERS = {Path('nbs/bpl.xml'): BPL, Path('nbs/dyalog.xml'): DYALOG, Path('nbs/bqn.xml'): BQN}
TEXTMATE = Path(__file__).parent
SCOPES = {'Normal Text': 'source.bpl', 'Name': 'variable.other.bpl', 'Comment': 'comment.line.bpl', 'String': 'string.quoted.double.bpl',
          'Char': 'string.quoted.single.bpl', 'Number': 'constant.numeric.bpl',
          'Function': 'support.function.bpl', 'MonadicOperator': 'keyword.operator.monadic.bpl',
          'DyadicOperator': 'keyword.operator.dyadic.bpl', 'Argument': 'variable.parameter.bpl', 'Keyword': 'keyword.control.bpl'}


def _replace(text, pattern, line):
    "`text` with its one match of `pattern` replaced by `line`."
    new, n = re.subn(pattern, lambda _: line, text)
    if n != 1: raise ValueError(f'expected one match of {pattern}, found {n}')
    return new


def vim_class(chars):
    "A vim pattern matching any of `chars`, between delimiters that aren't among them."
    body = chars.replace('\\', '\\\\').replace(']', '\\]').replace('^', '\\^')
    if '-' in body: body = body.replace('-', '') + '-'
    delimiter = next(c for c in '/#%' if c not in chars)
    return f'{delimiter}[{body}]{delimiter}'


def token_patterns(spellings):
    "Lexical rules with `spellings` for each BPL glyph."
    marks = '(?:' + '|'.join(re.escape(glyph) for glyph, _ in _number_marks) + ')'
    patterns = {cls: ''.join('[' + re.escape(spellings[c]) + ']' if c in '•¯∞⍬' else c for c in pattern)
        .replace(r'\.', '[' + spellings['.'] + ']').format(mark=marks) for cls, pattern in TOKEN_PATTERNS.items()}
    name = r'[^\s\p{Cc}0-9' + re.escape(''.join(spellings.values()) + _superscripts + _subscripts + ''.join(glyph for glyph, _ in _number_marks) + r'''()[]{}'";$_ₑⱼᵣ''') + ']'
    return patterns | {'Name': rf'(?:_\d+|_\S|_|{name})+'}


def _xml(root):
    ET.indent(root, space='  ')
    return '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE language>\n' + ET.tostring(root, encoding='unicode') + '\n'


def quarto(text, glyphs):
    "Regenerate glyph classes and numeric rules in a Quarto XML grammar."
    root = ET.fromstring(text)
    normal = root.find('./highlighting/contexts/context[@name="Normal"]')
    for cls, (style, *_) in CLASSES.items():
        normal.find(f'AnyChar[@attribute="{cls}"]').set('String', glyphs[cls])
        root.find(f'./highlighting/itemDatas/itemData[@name="{cls}"]').set('defStyleNum', style)
    if glyphs is BPL:
        patterns = token_patterns({glyph: glyph for glyph in SPELLINGS})
        normal.find('RegExpr[@attribute="Name"]').set('String', patterns['Name'])
        for cls, attr, marker in [('System', 'Function', '•'), ('Number', 'Number', '∞'), ('Empty', 'Number', '⍬')]:
            next(r for r in normal.findall(f'RegExpr[@attribute="{attr}"]') if marker in r.get('String', '')).set('String', patterns[cls])
        marked = normal.find('RegExpr[@attribute="MonadicOperator"]')
        if marked is None:
            marked = ET.Element('RegExpr', attribute='MonadicOperator')
            normal.insert(list(normal).index(normal.find('AnyChar[@attribute="MonadicOperator"]')), marked)
        marked.set('String', patterns['Mark'])
        normal.find('RegExpr[@context="Mark"]').set('String', r'[\])](?=' + patterns['Mark'] + ')')
        marked = root.find('./highlighting/contexts/context[@name="Mark"]')[0]
        marked.tag = 'RegExpr'
        marked.set('String', patterns['Mark'])
        normal.find('AnyChar[@attribute="Comment"]').set('String', '⍝')
    return _xml(root)


def hanzi_grammar(xml):
    "Hanzi-sensitive rules followed by an include of the BPL grammar."
    root, patterns = ET.fromstring(xml), token_patterns(SPELLINGS)
    root.set('name', 'BPL (Hanzi)')
    root.set('extensions', '*.hpl')
    contexts = root.find('./highlighting/contexts')
    normal = contexts.find("context[@name='Normal']")
    contexts.clear()
    contexts.append(normal)
    for rule in list(normal):
        attr, value = rule.get('attribute'), rule.get('String', '')
        if attr == 'Comment':
            rule.set('String', SPELLINGS['⍝'][1:])
            rule.set('context', 'Comment##BPL')
        elif attr == 'Name': rule.set('String', patterns['Name'])
        elif rule.tag == 'RegExpr' and '•' in value: rule.set('String', patterns['System'])
        elif rule.tag == 'RegExpr' and any(c in value for c in '∞⍬'): rule.set('String', patterns['Empty' if '⍬' in value else 'Number'])
        elif rule.tag == 'AnyChar' and attr in HANZI: rule.set('String', HANZI[attr])
        else: normal.remove(rule)
    ET.SubElement(normal, 'IncludeRules', context='Normal##BPL')
    return _xml(root)


def textmate(xml, language='bpl', base=None):
    r"Export line-local XML rules as a TextMate grammar; `base` supplies included BPL contexts."
    contexts = {c.get('name'): c for c in ET.fromstring(xml).findall('./highlighting/contexts/context')}
    if base: contexts |= {c.get('name') + '##BPL': c for c in ET.fromstring(base).findall('./highlighting/contexts/context')}
    def pattern(rule):
        if rule.tag == 'RegExpr': return rule.get('String')
        if rule.tag == 'AnyChar': return '[' + re.escape(rule.get('String')) + ']'
        if rule.tag in ('DetectChar', 'Detect2Chars'): return re.escape(rule.get('char') + rule.get('char1', ''))
        raise ValueError(f'unsupported XML rule: {rule.tag}')
    def convert(rule):
        if rule.tag == 'IncludeRules': return {'include': 'source.bpl'}
        scope, target = SCOPES[rule.get('attribute')], rule.get('context', '#stay')
        if target == '#stay': return {'match': pattern(rule), 'name': scope}
        context = contexts[target]
        if context.get('lineEndContext') != '#pop': raise ValueError(f'{target} must end at the line boundary')
        pops = [r for r in context if r.get('context') == '#pop']
        result = {'begin': pattern(rule), 'beginCaptures': {'0': {'name': scope}},
                  'end': '|'.join([*(pattern(r) for r in pops), '$']), 'contentName': SCOPES[context.get('attribute')],
                  'patterns': [convert(r) for r in context if r not in pops], 'applyEndPatternLast': True}
        if pops: result['endCaptures'] = {'0': {'name': SCOPES[pops[0].get('attribute')]}}
        return result
    return dict(name=language, displayName='BPL (Hanzi)' if language == 'hpl' else 'BPL', scopeName='source.' + language,
        fileTypes=[language], patterns=[convert(r) for r in contexts['Normal']])


def vim(text):
    "The vim syntax with glyph classes and numeric rules regenerated."
    patterns = {cls: pattern.replace('(?:', '%(').replace(r'\p{Cc}', r'\x00-\x1f\x7f-\x9f').replace('/', r'\/')
        for cls, pattern in token_patterns({glyph: glyph for glyph in SPELLINGS}).items()}
    text = _replace(text, r'(?m)^syntax match bplName .*$', rf'syntax match bplName /\v{patterns["Name"]}/')
    for cls in CLASSES:
        glyphs = vim_class(BPL[cls])
        if cls == 'MonadicOperator': glyphs = glyphs[0] + r'\v' + patterns['Mark'] + '|' + glyphs[1:]
        text = _replace(text, f'(?m)^syntax match bpl{cls} .*$', f'syntax match bpl{cls} {glyphs}')
    for cls, marker, pattern in [('System', '•', patterns['System']),
        ('Number', '∞', patterns['Number'] + '|' + patterns['Empty']), ('Comment', '⍝', r'⍝.*$')]:
        text = _replace(text, f'(?m)^syntax match bpl{cls} .*{marker}.*$', f'syntax match bpl{cls} /\\v{pattern}/')
    return text


def jetbrains():
    "Glyph-name and alias live templates, available in every file."
    root = ET.Element('templateSet', group='BPL')
    names = {name: s['glyph'] for s in symbols for name in [s['name'], s['monad'], s['dyad']] if name}
    for name, glyph in sorted(names.items()):
        template = ET.SubElement(root, 'template', name=name, value=glyph, description=glyph, shortcut='TAB')
        ET.SubElement(ET.SubElement(template, 'context'), 'option', name='OTHER', value='true')
    ET.indent(root, space='  ')
    return ET.tostring(root, encoding='unicode') + '\n'


def key(shortcut):
    "The keys that type a glyph, from its `shortcut` in `basedpl.symbols`. For example, `' o 8'` is `Alt-o 8`."
    keys = [KEY_NAMES.get(k, k) for k in shortcut.split()]
    return ' '.join(['Alt-' + keys[0], *keys[1:]]) if keys else ''


def keys(s):
    "The keys that type glyph row `s` from `basedpl.symbols`, such as ``Keys: `Alt-m` ``. A glyph typed without Alt has none."
    alt = key(s['shortcut'])
    return f'Keys: `{alt}`' if alt else ''


def hanzi(s):
    "The hanzi that spell glyph row `s` from `basedpl.symbols`, such as ``Hanzi: `乘/号` (`乘/號`)``. The traditional spelling follows in brackets where it differs from the simplified one."
    simplified, traditional = ('/'.join(h) for h in s['hanzi'])
    return f'Hanzi: `{simplified}`' + (f' (`{traditional}`)' if traditional != simplified else '')


def ranks(s):
    "The ranks of glyph row `s` from `basedpl.symbols`, such as ``Ranks: `1` monadic, `1 0` dyadic``. Only functions have ranks."
    forms = [f"`{s['monad_rank']}` monadic" if s['monad_rank'] else '',
             f"`{s['dyad_ranks']}` dyadic" if s['dyad_ranks'] else '']
    return 'Ranks: ' + ', '.join(filter(None, forms)) if any(forms) else ''


def kind(s):
    "The kind of glyph row `s` from `basedpl.symbols`, as its page names it, such as `Monadic operator`."
    return s['kind'].replace('-', ' ').capitalize()


def glyph_page(text, s):
    "The page `text` of glyph row `s` with the line after its title regenerated. The line gives the glyph's kind, its keys if it's typed with Alt, its hanzi and its ranks."
    title, rest = text.split('\n\n', 1)
    kinds = '|'.join({kind(r) for r in symbols})
    rest = re.sub(rf'\A(?:{kinds})(?:\. .*)?\n\n', '', rest)
    line = '. '.join(filter(None, [kind(s), keys(s), hanzi(s), ranks(s)]))
    return f'{title}\n\n{line}\n\n{rest}'


def glyph_pages():
    "Regenerate the line after each glyph page's title."
    for s in symbols:
        path = GLYPH_PAGES/f"{s['name']}.qmd"
        keyboards.write_changed(path, glyph_page(path.read_text(), s))


def _image(size):
    "The input-menu icon at `size` pixels: a white `⍺²` in Menlo Bold on a dark purple rounded square, which reads on light and dark menu bars."
    from PIL import Image, ImageDraw, ImageFont
    big = size * 8
    square = Image.new('RGBA', (big, big))
    ImageDraw.Draw(square).rounded_rectangle((0, 0, big - 1, big - 1), radius=big // 5, fill=PURPLE)
    image = square.resize((size, size), Image.LANCZOS)
    draw, font = ImageDraw.Draw(image), ImageFont.truetype(FONT, round(size * 0.62), index=MENLO_BOLD)
    left, top, right, bottom = draw.textbbox((0, 0), '⍺²', font=font)
    draw.text(((size - right - left) / 2, (size - bottom - top) / 2), '⍺²', font=font, fill='white')
    return image


def icon():
    "Rebuild the bundle's input-menu icon with macOS's `iconutil`, from the image at 16 and 32 pixels."
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp)/'BasedPL-us.iconset'
        iconset.mkdir()
        for name, size in [('icon_16x16', 16), ('icon_16x16@2x', 32), ('icon_32x32', 32)]: _image(size).save(iconset/f'{name}.png')
        subprocess.run(['iconutil', '-c', 'icns', str(iconset), '-o', str(keyboards.ICON)], check=True)


def browser():
    "Write the self-contained editor module, `wasm/bpl-editor.js`, from the shared browser input sources. The module loads its font, `SAX2B.woff2`, from beside its own URL. `cargo wasm` copies both files into the npm package and the playground."
    configuration = dict(comments=dict(lineComment='⍝'), brackets=[['(', ')'], ['[', ']'], ['{', '}']],
        autoClosingPairs=[dict(open=a, close=b, notIn=['string', 'comment']) for a, b in [('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ("'", "'")]],
        surroundingPairs=[dict(open=a, close=b) for a, b in [('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ("'", "'")]])
    data = dict(grammar=textmate(Path('nbs/bpl.xml').read_text()), configuration=configuration,
        editorOptions=dict(fontFamily='SAX2B, monospace', unicodeHighlight=dict(ambiguousCharacters=False)))
    rows = ['// Generated by basedpl.editors.browser.',
        *(f'export const {name} = {json.dumps(value, ensure_ascii=False)};' for name, value in data.items()),
        f'const bar = {TEXTMATE.joinpath("lb.js").read_text()}({json.dumps(symbols, ensure_ascii=False)}, '
        f'{TEXTMATE.joinpath("input.js").read_text()}, {TEXTMATE.joinpath("layout.json").read_text()});',
        'const font = new URL("SAX2B.woff2", import.meta.url);', r'''export function installFont(doc = document) {
    if (doc.getElementById('bpl-font')) return;
    const style = doc.createElement('style');
    style.id = 'bpl-font';
    style.textContent = `@font-face{font-family:SAX2B;src:url("${font}") format("woff2")}`;
    doc.head.append(style);
}
export function attach(monaco, container) {
    installFont(container.ownerDocument);
    return bar.attach(monaco, container);
}''']
    keyboards.write_changed(Path('wasm/bpl-editor.js'), '\n'.join(rows) + '\n')


def write():
    "Write highlighters, glyph-entry templates, regional keyboard bundles, and glyph-page kinds, keys, hanzi and ranks."
    keyboards.write_changed(Path('wheel/data/share/jupyter/kernels/bpl/kernel.json'), json.dumps(kernel_spec(), indent=2) + '\n')
    for path, glyphs in HIGHLIGHTERS.items(): keyboards.write_changed(path, quarto(path.read_text(), glyphs))
    base = Path('nbs/bpl.xml').read_text()
    hanzi = hanzi_grammar(base)
    keyboards.write_changed(Path('nbs/hpl.xml'), hanzi)
    for lang, xml in [('bpl', base), ('hpl', hanzi)]:
        grammar = json.dumps(textmate(xml, lang, base), ensure_ascii=False, indent=2) + '\n'
        for folder in [TEXTMATE, JETBRAINS/'BPL.tmbundle/Syntaxes']:
            keyboards.write_changed(folder/f'{lang}.tmLanguage.json', grammar)
    browser()
    keyboards.write_changed(JETBRAINS/'BPL.xml', jetbrains())
    keyboards.write_changed(VIM, vim(VIM.read_text()))
    keyboards.write()
    windows_keyboard.write()
    glyph_pages()

if __name__ == '__main__': write()
