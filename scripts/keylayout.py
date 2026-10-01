"Write the macOS keyboard layout bundle from `python/basedpl/layout.json`. Pass `--icon` to also rebuild its input-menu icon."
import argparse, json, subprocess, tempfile
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

LAYOUT, BUNDLE = Path('python/basedpl/layout.json'), Path('editors/macos/BasedPL.bundle/Contents')
OUTPUT = BUNDLE/'Resources/BasedPL.keylayout'
ICON = OUTPUT.with_suffix('.icns')
FONT, MENLO_BOLD, PURPLE = '/System/Library/Fonts/Menlo.ttc', 1, (61, 31, 107)

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
            '<!-- Generated by scripts/keylayout.py from python/basedpl/layout.json. Do not edit. -->\n'
            f'<keyboard group="126" id="-24681" name="BasedPL" maxout="{maxout}">\n'
            ' <layouts>\n  <layout first="0" last="255" mapSet="keys" modifiers="modifiers"/>\n </layouts>\n'
            f' <modifierMap id="modifiers" defaultIndex="0">\n{selects} </modifierMap>\n'
            f' <keyMapSet id="keys">\n{"".join(keymaps)} </keyMapSet>\n'
            f' <actions>\n{acts} </actions>\n'
            f' <terminators>\n{terms} </terminators>\n'
            '</keyboard>\n')


def icon(size):
    "The input-menu icon at `size` pixels: a white `⍺²` in Menlo Bold on a dark purple rounded square, which reads on light and dark menu bars."
    big = size * 8
    square = Image.new('RGBA', (big, big))
    ImageDraw.Draw(square).rounded_rectangle((0, 0, big - 1, big - 1), radius=big // 5, fill=PURPLE)
    image = square.resize((size, size), Image.LANCZOS)
    draw, font = ImageDraw.Draw(image), ImageFont.truetype(FONT, round(size * 0.62), index=MENLO_BOLD)
    left, top, right, bottom = draw.textbbox((0, 0), '⍺²', font=font)
    draw.text(((size - right - left) / 2, (size - bottom - top) / 2), '⍺²', font=font, fill='white')
    return image


def icns():
    "The `.icns` file's bytes, built by macOS's `iconutil` from the icon at 16 and 32 pixels."
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp)/'BasedPL.iconset'
        iconset.mkdir()
        for name, size in [('icon_16x16', 16), ('icon_16x16@2x', 32), ('icon_32x32', 32)]: icon(size).save(iconset/f'{name}.png')
        subprocess.run(['iconutil', '-c', 'icns', str(iconset), '-o', f'{tmp}/icon.icns'], check=True)
        return Path(f'{tmp}/icon.icns').read_bytes()


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

def write(with_icon=False):
    "Write the bundle's layout and metadata, optionally rebuild its icon, and return the paths that changed."
    outputs = {OUTPUT: keylayout(json.loads(LAYOUT.read_text())).encode(), **metadata()}
    if with_icon: outputs[ICON] = icns()
    changed = [path for path, data in outputs.items() if not path.exists() or path.read_bytes() != data]
    for path in changed:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(outputs[path])
    return changed


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--icon', action='store_true', help='also rebuild the input-menu icon')
    for path in write(with_icon=parser.parse_args().icon): print(f'wrote {path}')
