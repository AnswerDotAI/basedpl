"""Write the generated parts of the editor files: the glyph lists in the Quarto and vim highlighters, the macOS keyboard layout bundle, and the keys line on each glyph page. BPL's lists come from its glyph metadata, and the bundle from `layout.json`. The Quarto highlighters for Dyalog APL and BQN, used on the comparison page, take their lists from tables here. Paths are relative to the repository root.

`write` writes them all, and the release script calls it. `icon` rebuilds the bundle's input-menu icon. It needs Pillow and macOS's `iconutil`, and only runs when the icon's design changes."""
import json, re, subprocess, tempfile
from pathlib import Path
from xml.sax.saxutils import escape
from . import symbols
from ._core import _superscripts, _subscripts

LAYOUT = Path(__file__).with_name('layout.json')
VIM = Path('editors/vim/syntax/bpl.vim')
BUNDLE = Path('editors/macos/BasedPL.bundle/Contents')
KEYLAYOUT = BUNDLE/'Resources/BasedPL.keylayout'
ICON = KEYLAYOUT.with_suffix('.icns')
GLYPH_PAGES = Path('nbs/glyphs')
KEY_NAMES = {'-': 'Minus', '\\': 'Backslash', '`': 'Backtick'}
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


def vim(text):
    "The vim syntax `text` with each class's glyph list regenerated."
    for cls in CLASSES: text = _replace(text, f'(?m)^syntax match bpl{cls} .*$', f'syntax match bpl{cls} {vim_class(BPL[cls])}')
    return text


def key(shortcut):
    "The keys that type a glyph, from its `shortcut` in `basedpl.symbols`. For example, `' o *'` is `Alt-o *`."
    keys = [KEY_NAMES.get(k, k) for k in shortcut.split()]
    return ' '.join(['Alt-' + keys[0], *keys[1:]]) if keys else ''


def glyph_page(text, shortcut):
    "The glyph page `text` with the keys line after its title regenerated. A glyph typed without Alt has no keys line."
    title, rest = text.split('\n\n', 1)
    rest = re.sub(r'\AKeys: .*\n\n', '', rest)
    return f'{title}\n\n' + (f'Keys: `{key(shortcut)}`\n\n' if shortcut.strip() else '') + rest


# US key codes, with each key's character unshifted and shifted. Code 10 is the extra key on ISO keyboards.
CHARACTER_KEYS = {
    0: 'aA', 1: 'sS', 2: 'dD', 3: 'fF', 4: 'hH', 5: 'gG', 6: 'zZ', 7: 'xX', 8: 'cC', 9: 'vV', 10: '§±', 11: 'bB', 12: 'qQ', 13: 'wW',
    14: 'eE', 15: 'rR', 16: 'yY', 17: 'tT', 18: '1!', 19: '2@', 20: '3#', 21: '4$', 22: '6^', 23: '5%', 24: '=+', 25: '9(', 26: '7&',
    27: '-_', 28: '8*', 29: '0)', 30: ']}', 31: 'oO', 32: 'uU', 33: '[{', 34: 'iI', 35: 'pP', 37: 'lL', 38: 'jJ', 39: '\'"', 40: 'kK',
    41: ';:', 42: '\\|', 43: ',<', 44: '/?', 45: 'nN', 46: 'mM', 47: '.>', 50: '`~',
}
SPACE, DELETE, ESCAPE = 49, 51, 53
# Keys that type the same thing whatever the modifiers: return, tab, enter, the keypad, function keys, navigation and arrows.
FIXED_KEYS = {
    36: '\r', 48: '\t', 52: '\x03', 65: '.', 67: '*', 69: '+', 71: '\x1b', 75: '/', 76: '\x03', 78: '-', 81: '=',
    **dict(zip([82, 83, 84, 85, 86, 87, 88, 89, 91, 92], '0123456789')),
    **{code: '\x10' for code in [64, 79, 80, 96, 97, 98, 99, 100, 101, 103, 105, 106, 107, 109, 111, 113, 118, 120, 122]},
    114: '\x05', 115: '\x01', 116: '\x0b', 117: '\x7f', 119: '\x04', 121: '\x0c', 123: '\x1c', 124: '\x1d', 125: '\x1f', 126: '\x1e',
}
# The key map for each combination of modifiers. Option–Shift types the shifted character, since no glyph needs Shift.
MODIFIERS = ['', 'anyShift caps?', 'caps', 'anyOption caps?', 'anyShift anyOption caps?', 'command anyShift? anyOption? caps? anyControl?',
             'anyControl anyShift? anyOption? caps?']
PLAIN, SHIFT, CAPS, OPTION, OPTION_SHIFT, COMMAND, CONTROL = range(len(MODIFIERS))


def typed(code, index, layout):
    "What the key `code` types with the modifiers of key map `index`: a string, or `{'state': name}` for a dead key."
    lower, upper = CHARACTER_KEYS[code]
    if index == CONTROL: return chr(ord(lower) - 96) if lower.isalpha() else {'[': '\x1b', '\\': '\x1c', ']': '\x1d'}.get(lower, lower)
    if index == OPTION: return layout['option'].get(lower, '')
    return {PLAIN: lower, SHIFT: upper, CAPS: lower.upper(), OPTION_SHIFT: upper, COMMAND: lower}[index]


def attribute(text):
    "Quote `text` as an XML attribute, using numeric references for XML-special characters and controls."
    return '"' + ''.join(f'&#x{ord(c):04X};' if ord(c) < 32 or ord(c) == 127 or c in '&<>"' else c for c in text) + '"'


def when(state, value):
    "A `when` element for `value` in `state`: typing text, or moving to another state."
    return f'<when state={attribute(state)} next={attribute(value["state"])}/>' if isinstance(value, dict) else f'<when state={attribute(state)} output={attribute(value)}/>'


def keylayout(layout):
    "The text of a macOS `.keylayout` for `layout`."
    states = layout['states']
    # A character needs an action when a state gives it a meaning, or when it starts a state itself.
    actions, keymaps = {}, []
    def action(value, char=None):
        entries = {'none': value}
        if char is not None: entries |= {name: s['keys'][char] for name, s in states.items() if char in s['keys']}
        if len(entries) == 1 and not isinstance(value, dict): return None
        id = f'u{ord(char):04X}' if char is not None else f'{value["state"]}-start'
        actions[id] = entries
        return id
    for index in range(len(MODIFIERS)):
        keys = []
        for code in sorted(CHARACTER_KEYS):
            value = typed(code, index, layout)
            id = action(value, value if isinstance(value, str) and len(value) == 1 and index not in (CONTROL, COMMAND, OPTION) else None)
            keys.append(f'<key code="{code}" action="{id}"/>' if id else f'<key code="{code}" output={attribute(value)}/>')
        keys += [f'<key code="{code}" action="{name}"/>' for code, name in [(SPACE, 'space'), (DELETE, 'delete'), (ESCAPE, 'escape')]]
        keys += [f'<key code="{code}" output={attribute(text)}/>' for code, text in FIXED_KEYS.items()]
        keymaps.append(f'  <keyMap index="{index}">\n' + ''.join(f'   {k}\n' for k in keys) + '  </keyMap>\n')
    # Space types a state's terminator alone. Delete and Escape cancel a state and type nothing.
    cancel = {name: '' for name in states}
    actions['space'] = {'none': ' ', **{name: s['terminator'] for name, s in states.items()}}
    actions['delete'] = {'none': '\x08', **cancel}
    actions['escape'] = {'none': '\x1b', **cancel}
    texts = [v for a in actions.values() for v in a.values() if isinstance(v, str)] + [s['terminator'] for s in states.values()]
    maxout = max(len(t.encode('utf-16-le')) // 2 for t in texts)
    selects = ''.join(f'  <keyMapSelect mapIndex="{i}"><modifier keys="{m}"/></keyMapSelect>\n' for i, m in enumerate(MODIFIERS))
    acts = ''.join(f'  <action id="{id}">\n' + ''.join(f'   {when(s, v)}\n' for s, v in entries.items()) + '  </action>\n' for id, entries in actions.items())
    terms = ''.join(f'  <when state={attribute(name)} output={attribute(s["terminator"])}/>\n' for name, s in states.items() if s['terminator'])
    return ('<?xml version="1.1" encoding="UTF-8"?>\n'
            '<!DOCTYPE keyboard SYSTEM "file://localhost/System/Library/DTDs/KeyboardLayout.dtd">\n'
            '<!-- Generated by basedpl.editors from python/basedpl/layout.json. Do not edit. -->\n'
            f'<keyboard group="126" id="-24681" name="BasedPL" maxout="{maxout}">\n'
            ' <layouts>\n  <layout first="0" last="255" mapSet="keys" modifiers="modifiers"/>\n </layouts>\n'
            f' <modifierMap id="modifiers" defaultIndex="0">\n{selects} </modifierMap>\n'
            f' <keyMapSet id="keys">\n{"".join(keymaps)} </keyMapSet>\n'
            f' <actions>\n{acts} </actions>\n'
            f' <terminators>\n{terms} </terminators>\n'
            '</keyboard>\n')


def plist(**entries):
    "A property list holding `entries` as string keys and values."
    items = ''.join(f'\t<key>{k}</key>\n\t<string>{v}</string>\n' for k, v in entries.items())
    return ('<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n'
            f'<plist version="1.0">\n<dict>\n{items}</dict>\n</plist>\n')


def metadata():
    "The bundle's property lists and localized name, with the identifier of the bundle users already have installed."
    return {BUNDLE/'Info.plist': plist(CFBundleIdentifier='org.basedpl.keyboardlayout.basedpl', CFBundleName='BasedPL', CFBundleVersion='').encode(),
            BUNDLE/'version.plist': plist(BuildVersion='', ProjectName='BasedPL', SourceVersion='').encode(),
            BUNDLE/'Resources/en.lproj/InfoPlist.strings': '"BasedPL" = "BasedPL";\n'.encode('utf-16')}


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
        iconset = Path(tmp)/'BasedPL.iconset'
        iconset.mkdir()
        for name, size in [('icon_16x16', 16), ('icon_16x16@2x', 32), ('icon_32x32', 32)]: _image(size).save(iconset/f'{name}.png')
        subprocess.run(['iconutil', '-c', 'icns', str(iconset), '-o', str(ICON)], check=True)


def write():
    "Write the highlighters' glyph lists and styles, the keyboard layout bundle, and the keys line on each glyph page."
    for path, glyphs in HIGHLIGHTERS.items(): path.write_text(quarto(path.read_text(), glyphs))
    VIM.write_text(vim(VIM.read_text()))
    for path, data in {KEYLAYOUT: keylayout(json.loads(LAYOUT.read_text())).encode(), **metadata()}.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    for s in symbols:
        path = GLYPH_PAGES/f"{s['name']}.qmd"
        path.write_text(glyph_page(path.read_text(), s['shortcut']))
