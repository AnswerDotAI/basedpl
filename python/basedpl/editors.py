"""Write editor highlighters, regional macOS keyboards, and glyph-page key lines.

BPL glyph lists come from its symbol metadata. Dyalog APL and BQN lists for the comparison page are defined here. `keyboards` generates the macOS bundles. Paths are relative to the repository root.

`write()` updates all generated editor files during release preparation, including `bpl.tmLanguage.json` beside this module, exported from `nbs/bpl.xml`. `icon()` rebuilds the common input-menu icon when its design changes; it needs Pillow and macOS's `iconutil`. Keyboard generation copies the icon without rebuilding it."""
import json, re, subprocess, tempfile
from pathlib import Path
from xml.etree import ElementTree as ET
from xml.sax.saxutils import escape
from . import symbols, keyboards
from ._core import _superscripts, _subscripts

VIM = Path('editors/vim/syntax/bpl.vim')
GLYPH_PAGES = Path('nbs/glyphs')
KEY_NAMES = {char: f'Shift-{label}' for char, label in keyboards.SHIFT_KEYS.items()} | {'-': 'Minus', '\\': 'Backslash', '`': 'Backtick'}
FONT, MENLO_BOLD, PURPLE = '/System/Library/Fonts/Menlo.ttc', 1, (61, 31, 107)

# Each highlight class: the Kate style that colours it in every highlighter, the BPL symbol kind it covers, and the script characters it adds.
CLASSES = {'Function': ('dsFunction', 'function', ''), 'MonadicOperator': ('dsOperator', 'monadic-operator', _superscripts),
           'DyadicOperator': ('dsExtension', 'dyadic-operator', ''), 'Argument': ('dsVariable', 'argument', ''), 'Keyword': ('dsKeyword', 'syntax', _subscripts)}
# The characters each class colours in each language. The Dyalog APL and BQN tables serve the comparison page.
BPL = {cls: ''.join(s['glyph'] for s in symbols if s['kind'] == kind) + scripts for cls, (_, kind, scripts) in CLASSES.items()}
DYALOG = {'Function': '+-×÷⌈⌊|*⍟○!?~∧∨⍲⍱<≤=≥>≠≡≢⍴,⍪⌽⊖⍉↑↓⊂⊃⊆⊇⌷⍋⍒⊤⊥⍳⍸∊⍷∪∩⊣⊢⍎⍕⌹', 'MonadicOperator': r'¨⍨/⌿\⍀⌸&⌶',
          'DyadicOperator': '∘⍤⍥⍣⍠⌺@.⍛', 'Argument': '⍺⍵∇', 'Keyword': '←→⋄:⎕⍞'}
BQN = {'Function': '+-×÷⋆√⌊⌈|¬∧∨<>≠=≤≥≡≢⊣⊢⥊∾≍⋈↑↓↕«»⌽⍉/⍋⍒⊏⊑⊐⊒∊⍷⊔!', 'MonadicOperator': '˙˜˘¨⌜⁼´˝`',
       'DyadicOperator': '∘○⊸⟜⌾⊘◶⎉⚇⍟⎊', 'Argument': '𝕨𝕩𝕗𝕘𝕤𝕣𝕎𝕏𝔽𝔾𝕊', 'Keyword': '←↩⇐⋄,‿·?:;'}
HIGHLIGHTERS = {Path('nbs/bpl.xml'): BPL, Path('nbs/dyalog.xml'): DYALOG, Path('nbs/bqn.xml'): BQN}
TEXTMATE = Path(__file__).with_name('bpl.tmLanguage.json')
SCOPES = {'Normal Text': 'source.bpl', 'Comment': 'comment.line.bpl', 'String': 'string.quoted.double.bpl',
          'Char': 'string.quoted.single.bpl', 'Number': 'constant.numeric.bpl', 'System': 'support.function.system.bpl',
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


def quarto(text, glyphs):
    "The Quarto highlighter `text`, with each class's style regenerated and its glyph list taken from `glyphs`."
    for cls, (style, *_) in CLASSES.items():
        text = _replace(text, f'<AnyChar String="[^"]*" attribute="{cls}"/>', f'<AnyChar String="{escape(glyphs[cls], {chr(34): "&quot;"})}" attribute="{cls}"/>')
        text = _replace(text, f'<itemData name="{cls}" defStyleNum="[^"]*"/>', f'<itemData name="{cls}" defStyleNum="{style}"/>')
    return text


def textmate(xml):
    r"Export BPL's line-local XML rules as a TextMate grammar."
    contexts = {c.get('name'): c for c in ET.fromstring(xml).findall('./highlighting/contexts/context')}
    def pattern(rule):
        if rule.tag == 'RegExpr': return rule.get('String')
        if rule.tag == 'AnyChar': return '[' + re.escape(rule.get('String')) + ']'
        if rule.tag in ('DetectChar', 'Detect2Chars'): return re.escape(rule.get('char') + rule.get('char1', ''))
        raise ValueError(f'unsupported XML rule: {rule.tag}')
    def convert(rule):
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
    return {'name': 'bpl', 'scopeName': 'source.bpl', 'fileTypes': ['bpl'], 'patterns': [convert(r) for r in contexts['Normal']]}


def vim(text):
    "The vim syntax `text` with each class's glyph list regenerated."
    for cls in CLASSES: text = _replace(text, f'(?m)^syntax match bpl{cls} .*$', f'syntax match bpl{cls} {vim_class(BPL[cls])}')
    return text


def key(shortcut):
    "The keys that type a glyph, from its `shortcut` in `basedpl.symbols`. For example, `' o 8'` is `Alt-o 8`."
    keys = [KEY_NAMES.get(k, k) for k in shortcut.split()]
    return ' '.join(['Alt-' + keys[0], *keys[1:]]) if keys else ''


def glyph_page(text, shortcut):
    "The glyph page `text` with the keys line after its title regenerated. A glyph typed without Alt has no keys line."
    title, rest = text.split('\n\n', 1)
    rest = re.sub(r'\AKeys: .*\n\n', '', rest)
    return f'{title}\n\n' + (f'Keys: `{key(shortcut)}`\n\n' if shortcut.strip() else '') + rest


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


def write():
    "Write the XML and TextMate highlighters, regional keyboard bundles, and glyph-page key lines."
    for path, glyphs in HIGHLIGHTERS.items(): keyboards.write_changed(path, quarto(path.read_text(), glyphs))
    keyboards.write_changed(TEXTMATE, json.dumps(textmate(Path('nbs/bpl.xml').read_text()), ensure_ascii=False, indent=2) + '\n')
    keyboards.write_changed(VIM, vim(VIM.read_text()))
    keyboards.write()
    for s in symbols:
        path = GLYPH_PAGES/f"{s['name']}.qmd"
        keyboards.write_changed(path, glyph_page(path.read_text(), s['shortcut']))
