"""Generate regional BPL keyboards from macOS layouts.

`capture()` reads Apple's installed layouts, including disabled ones, and saves their native maps and accent transitions in `keyboards.json`. Run it on macOS when adding or refreshing a base layout. `write()` generates the bundles, shortcut tables and docs keyboard map from that data and `layout.json` on any platform. `editors.write()` calls it during release preparation.

Shortcuts follow local characters. Native Option ASCII and accent dead keys take priority. Conflicting BPL shortcuts try Option-Shift before Control-Option and Control-Option-Shift. Existing shifted BPL shortcuts keep their chords. `OVERRIDES` lists the remaining layout-specific choices. The US layout retains its existing Option accent-key bindings. Option-Shift on the local zero key types `⍬`. Option-Shift on the local equals key types `≠`, except German and Spanish use their plus key.
"""
import ctypes as ct, json
from html import escape
from pathlib import Path

LAYOUT = Path(__file__).with_name('layout.json')
DATA = LAYOUT.with_name('keyboards.json')
OUTPUT = Path('editors/macos')
ICON = OUTPUT/'BasedPL-us.bundle/Contents/Resources/BasedPL-us.icns'
SOURCES = {'us': 'US', 'uk': 'British', 'de': 'German', 'fr': 'French', 'es': 'Spanish-ISO'}
MODIFIERS = [('', 0), ('anyShift', 2), ('caps', 4), ('anyShift caps', 6), ('anyOption caps?', 8), ('anyShift anyOption caps?', 10),
             ('command anyOption? anyControl? caps?', 1), ('command anyShift anyOption? anyControl? caps?', 3),
             ('anyControl caps?', 16), ('anyControl anyShift caps?', 18), ('anyControl anyOption caps?', 24), ('anyControl anyShift anyOption caps?', 26)]
PLAIN, SHIFT, CAPS, SHIFT_CAPS, OPTION, OPTION_SHIFT, COMMAND, COMMAND_SHIFT, CONTROL, CONTROL_SHIFT, CONTROL_OPTION, CONTROL_OPTION_SHIFT = range(12)
OVERRIDES = {'de': {'7': ('d', CONTROL_OPTION), '\\': ('p', CONTROL_OPTION), '+': ('+', OPTION_SHIFT)},
             'fr': {'\\': ('p', CONTROL_OPTION)}, 'es': {'+': ('+', OPTION_SHIFT)}}
OPTION_ACCENTS = {'us': False}
SHIFT_KEYS = {'+': '=', ')': '0'}
SPACE, DELETE, ESCAPE = 49, 51, 53
KEYBOARD_PAGE = Path('nbs/keyboard.qmd')
GRID = [[50, 18, 19, 20, 21, 23, 22, 26, 28, 25, 29, 27, 24, 51],
        [48, 12, 13, 14, 15, 17, 16, 32, 34, 31, 35, 33, 30, 42],
        [57, 0, 1, 2, 3, 5, 4, 38, 40, 37, 41, 39, 36],
        [56, 6, 7, 8, 9, 11, 45, 46, 43, 47, 44, 60],
        [59, 58, 55, 49, 54, 61, 62]]
SPANS = {36: 2, 60: 3, 49: 8}
LABELS = {48: 'Tab', 57: 'Caps', 56: 'Shift', 60: 'Shift', 59: 'Ctrl', 62: 'Ctrl', 58: 'Opt', 61: 'Opt',
          55: 'Cmd', 54: 'Cmd', 49: 'Space', 51: 'Delete', 36: 'Enter'}


class MacLayouts:
    "Read native macOS keyboard maps through Carbon's Text Input Sources API."
    def __init__(self):
        self.carbon = ct.CDLL('/System/Library/Frameworks/Carbon.framework/Carbon')
        self.cf = ct.CDLL('/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation')
        for lib, name, args, result in [
            (self.carbon, 'TISCreateInputSourceList', [ct.c_void_p, ct.c_ubyte], ct.c_void_p),
            (self.carbon, 'TISGetInputSourceProperty', [ct.c_void_p, ct.c_void_p], ct.c_void_p),
            (self.cf, 'CFArrayGetCount', [ct.c_void_p], ct.c_long),
            (self.cf, 'CFArrayGetValueAtIndex', [ct.c_void_p, ct.c_long], ct.c_void_p),
            (self.cf, 'CFStringGetCString', [ct.c_void_p, ct.c_void_p, ct.c_long, ct.c_uint32], ct.c_bool),
            (self.cf, 'CFDataGetBytePtr', [ct.c_void_p], ct.c_void_p),
            (self.cf, 'CFRelease', [ct.c_void_p], None),
            (self.carbon, 'UCKeyTranslate', [ct.c_void_p, ct.c_uint16, ct.c_uint16, ct.c_uint32, ct.c_uint32, ct.c_uint32,
                                           ct.POINTER(ct.c_uint32), ct.c_uint32, ct.POINTER(ct.c_uint32), ct.POINTER(ct.c_uint16)], ct.c_int32)]:
            fn = getattr(lib, name)
            fn.argtypes, fn.restype = args, result
        id_key = ct.c_void_p.in_dll(self.carbon, 'kTISPropertyInputSourceID')
        data_key = ct.c_void_p.in_dll(self.carbon, 'kTISPropertyUnicodeKeyLayoutData')
        self.sources = self.carbon.TISCreateInputSourceList(None, True)
        self.data = {}
        for i in range(self.cf.CFArrayGetCount(self.sources)):
            source = self.cf.CFArrayGetValueAtIndex(self.sources, i)
            ident = self.carbon.TISGetInputSourceProperty(source, id_key)
            buf = ct.create_string_buffer(512)
            if not self.cf.CFStringGetCString(ident, buf, len(buf), 0x08000100): raise ValueError('cannot read input-source ID')
            name = buf.value.decode()
            if name in {f'com.apple.keylayout.{s}' for s in SOURCES.values()}:
                self.data[name] = self.cf.CFDataGetBytePtr(self.carbon.TISGetInputSourceProperty(source, data_key))

    def translate(self, source, code, modifiers=0, state=0):
        "Translate a native key into text and its pending accent state."
        dead, length, chars = ct.c_uint32(state), ct.c_uint32(), (ct.c_uint16 * 255)()
        error = self.carbon.UCKeyTranslate(self.data[source], code, 0, modifiers, 40, 0, ct.byref(dead), 255, ct.byref(length), chars)
        if error: raise ValueError(f'UCKeyTranslate: {error}')
        # The high word records the completed accent; the low word is the pending 16-bit state index.
        return bytes(chars)[:length.value * 2].decode('utf-16-le'), dead.value & 0xffff

    def capture(self, source):
        "Capture each modifier map and the accent transitions that differ from ordinary termination."
        def value(text, state): return {'state': f'native-{state}', **({'output': text} if text else {})} if state else text
        keys = {str(code): [value(*self.translate(source, code, mods)) for _, mods in MODIFIERS] for code in range(128)}
        pending = {v['state'] for values in keys.values() for v in values if isinstance(v, dict)}
        states = {}
        while pending:
            name = min(pending)
            pending.remove(name)
            number = int(name.removeprefix('native-'))
            terminator = self.translate(source, SPACE, state=number)[0]
            transitions = {}
            for code, values in keys.items():
                for index, (_, mods) in enumerate(MODIFIERS):
                    text, nextstate = self.translate(source, int(code), mods, number)
                    default = values[index]
                    expected = terminator + (default.get('output', '') if isinstance(default, dict) else default)
                    expected_state = int(default['state'].removeprefix('native-')) if isinstance(default, dict) else 0
                    if (text, nextstate) == (expected, expected_state): continue
                    transitions[f'{code}:{index}'] = value(text, nextstate)
                    if nextstate and f'native-{nextstate}' not in states and nextstate != number: pending.add(f'native-{nextstate}')
            states[name] = {'terminator': terminator, 'keys': transitions}
        return {'source': source, 'keys': keys, 'states': states}


def data_json(data, indent=0):
    "Indent dictionaries while keeping key-map arrays on one line."
    if not isinstance(data, dict): return json.dumps(data, ensure_ascii=False)
    pad = ' ' * indent
    rows = [f'{pad} {json.dumps(str(k))}: {data_json(v, indent + 1)}' for k, v in data.items()]
    return '{\n' + ',\n'.join(rows) + '\n' + pad + '}'


def capture():
    "Refresh `keyboards.json` from native macOS layouts without enabling or installing them."
    native = MacLayouts()
    try: bases = {lang: native.capture(f'com.apple.keylayout.{name}') for lang, name in SOURCES.items()}
    finally: native.cf.CFRelease(native.sources)
    DATA.write_text(data_json(bases) + '\n')
    return DATA


def bases(): return json.loads(DATA.read_text())


def character(value, base):
    "The character a native key prints, or the accent its dead key starts."
    return base['states'][value['state']]['terminator'] if isinstance(value, dict) else value


def shortcuts(layout, lang, base):
    "Assign BPL shortcuts to local keys, preserving native Option ASCII and accent keys."
    keys = {int(code): values for code, values in base['keys'].items() if int(code) <= 50}
    bindings = {char: (next(code for code, values in keys.items() if character(values[PLAIN], base) == label), index)
                for char, (label, index) in OVERRIDES.get(lang, {}).items()}
    used = set(bindings.values())
    shifted = {character(v[SHIFT], base) for v in keys.values()} - {character(v[PLAIN], base) for v in keys.values()}
    fallbacks = {OPTION: (OPTION, OPTION_SHIFT, CONTROL_OPTION, CONTROL_OPTION_SHIFT),
                 OPTION_SHIFT: (OPTION_SHIFT, CONTROL_OPTION_SHIFT), CONTROL_OPTION: (CONTROL_OPTION, CONTROL_OPTION_SHIFT),
                 CONTROL_OPTION_SHIFT: (CONTROL_OPTION_SHIFT,)}
    # Assign shifted BPL shortcuts before another shortcut can claim their chords as a fallback.
    for char in sorted(layout['option'], key=lambda c: (c not in SHIFT_KEYS, c not in shifted)):
        if char in bindings: continue
        sources = [(code, index) for code, values in keys.items() for index in (PLAIN, SHIFT, OPTION, OPTION_SHIFT)
                   if character(values[index], base) == SHIFT_KEYS.get(char, char)]
        sources.sort(key=lambda p: (p[1], p[0]))
        targets = []
        for code, index in sources:
            start = OPTION_SHIFT if char in SHIFT_KEYS else {PLAIN: OPTION, SHIFT: OPTION_SHIFT, OPTION: CONTROL_OPTION, OPTION_SHIFT: CONTROL_OPTION_SHIFT}[index]
            for dest in fallbacks[start]:
                native = keys[code][dest]
                accent = isinstance(native, dict) and OPTION_ACCENTS.get(lang, True)
                ascii_key = isinstance(native, str) and len(native) == 1 and native.isascii() and native.isprintable() and native != ' '
                if dest in (OPTION, OPTION_SHIFT) and (accent or ascii_key): continue
                targets.append((code, dest))
        target = next((p for p in targets if p not in used), None)
        if target is None: raise ValueError(f'{lang}: no available chord for {char!r}; add an override')
        bindings[char] = target
        used.add(target)
    return bindings


def attribute(text):
    "Quote an XML attribute, using numeric references for XML-special characters and controls."
    return '"' + ''.join(f'&#x{ord(c):04X};' if ord(c) < 32 or ord(c) == 127 or c in '&<>"' else c for c in text) + '"'


def when(state, value):
    "A state transition, which can type text and start another dead key."
    attrs = f'next={attribute(value["state"])} output={attribute(value.get("output", ""))}' if isinstance(value, dict) else f'output={attribute(value)}'
    return f'<when state={attribute(state)} {attrs}/>'


def keylayout(layout, lang='us'):
    "Generate a BPL `.keylayout` over the native maps for `lang`."
    base = bases()[lang]
    states = layout['states'] | base['states']
    bindings = {chord: layout['option'][char] for char, chord in shortcuts(layout, lang, base).items()}
    actions, keymaps, texts = {}, [], []
    for index in range(len(MODIFIERS)):
        keys = []
        for code, values in base['keys'].items():
            chord = (int(code), index)
            value = bindings.get(chord, values[index])
            entries = {'none': value}
            if chord not in bindings:
                native_key = f'{code}:{index}'
                entries |= {name: s['keys'][native_key] for name, s in base['states'].items() if native_key in s['keys']}
                char = character(value, base)
                if index not in (COMMAND, COMMAND_SHIFT, CONTROL, CONTROL_SHIFT, CONTROL_OPTION, CONTROL_OPTION_SHIFT):
                    entries |= {name: s['keys'][char] for name, s in layout['states'].items() if char in s['keys']}
            elif isinstance(value, dict): entries[value['state']] = states[value['state']]['terminator']
            if int(code) == SPACE: entries |= {name: s['terminator'] for name, s in layout['states'].items()}
            if int(code) in (DELETE, ESCAPE): entries |= {name: '' for name in states}
            texts += [v.get('output', '') if isinstance(v, dict) else v for v in entries.values()]
            if len(entries) == 1 and isinstance(value, str): keys.append(f'<key code="{code}" output={attribute(value)}/>')
            else:
                id = f'k{code}-m{index}'
                actions[id] = entries
                keys.append(f'<key code="{code}" action="{id}"/>')
        keymaps.append(f'  <keyMap index="{index}">\n' + ''.join(f'   {k}\n' for k in keys) + '  </keyMap>\n')
    texts += [s['terminator'] for s in states.values()]
    maxout = max(len(t.encode('utf-16-le')) // 2 for t in texts)
    selects = ''.join(f'  <keyMapSelect mapIndex="{i}"><modifier keys="{m}"/></keyMapSelect>\n' for i, (m, _) in enumerate(MODIFIERS))
    acts = ''.join(f'  <action id="{id}">\n' + ''.join(f'   {when(s, v)}\n' for s, v in entries.items()) + '  </action>\n' for id, entries in actions.items())
    terms = ''.join(f'  <when state={attribute(name)} output={attribute(s["terminator"])}/>\n' for name, s in states.items() if s['terminator'])
    keyboard_id = -24681 - list(SOURCES).index(lang)
    return ('<?xml version="1.1" encoding="UTF-8"?>\n'
            '<!DOCTYPE keyboard SYSTEM "file://localhost/System/Library/DTDs/KeyboardLayout.dtd">\n'
            '<!-- Generated by basedpl.keyboards from layout.json and keyboards.json. Do not edit. -->\n'
            f'<keyboard group="126" id="{keyboard_id}" name="BasedPL-{lang}" maxout="{maxout}">\n'
            ' <layouts>\n  <layout first="0" last="255" mapSet="keys" modifiers="modifiers"/>\n </layouts>\n'
            f' <modifierMap id="modifiers" defaultIndex="0">\n{selects} </modifierMap>\n'
            f' <keyMapSet id="keys">\n{"".join(keymaps)} </keyMapSet>\n'
            f' <actions>\n{acts} </actions>\n'
            f' <terminators>\n{terms} </terminators>\n'
            '</keyboard>\n')


def bundle(lang): return OUTPUT/f'BasedPL-{lang}.bundle/Contents'


def metadata(lang):
    "Generate the regional bundle's property lists and input-menu name."
    from plistlib import dumps
    path, name = bundle(lang), f'BasedPL-{lang}'
    return {path/'Info.plist': dumps(dict(CFBundleIdentifier=f'org.basedpl.keyboardlayout.basedpl-{lang}', CFBundleName=name, CFBundleVersion='')),
            path/'version.plist': dumps(dict(BuildVersion='', ProjectName=name, SourceVersion='')),
            path/'Resources/en.lproj/InfoPlist.strings': f'"{name}" = "{name}";\n'.encode('utf-16')}


def keyboard_help(layout, lang):
    "List shortcuts with each key's unshifted label."
    base = bases()[lang]
    modifiers = {OPTION: 'Option', OPTION_SHIFT: 'Option-Shift', CONTROL_OPTION: 'Control-Option', CONTROL_OPTION_SHIFT: 'Control-Option-Shift'}
    rows = [f'BasedPL-{lang}: {base["source"]}', '', 'Chords use the unshifted key label.',
            'Repeat a prefix chord to type its character, or release modifiers before its next key.', '', 'Chord\tOutput or prefix']
    for char, (code, index) in sorted(shortcuts(layout, lang, base).items(), key=lambda p: p[1]):
        label = character(base['keys'][str(code)][PLAIN], base)
        value = layout['option'][char]
        output = f'{value["state"]} prefix' if isinstance(value, dict) else value
        rows.append(f'{modifiers[index]}-{label}\t{output}')
    return '\n'.join(rows) + '\n'


def dead_key_table(layout):
    "List Option dead keys, their double-tap glyphs and follow-up sequences."
    def code(text):
        text = escape(text).replace('\\', '&#92;').replace('|', '&#124;').replace('`', '&#96;')
        return f'<code>{text}</code>'
    def followups(name, prefix=''):
        keys = layout['states'][name]['keys']
        digits = all(isinstance(keys.get(str(i)), str) for i in range(10))
        if digits: yield prefix + '0–9', keys['0'] + '–' + keys['9']
        for key, value in keys.items():
            if digits and key in '0123456789': continue
            if isinstance(value, dict): yield from followups(value['state'], prefix + key)
            else: yield prefix + key, value
    rows = ['| Key | Double-tap | Follow-ups |', '|---|---|---|']
    for key, value in layout['option'].items():
        if not isinstance(value, dict): continue
        groups = {}
        for sequence, result in followups(value['state']): groups.setdefault(result, []).append(sequence)
        following = '; '.join('/'.join(code(k) for k in keys) + ':' + code(result) for result, keys in groups.items())
        rows.append(f'| {code(key)} | {code(layout["states"][value["state"]]["terminator"])} | {following} |')
    return '\n'.join(rows)


def keyboard_page(layout):
    "Generate the US-based Option keyboard diagram and dead-key table for the docs."
    base = bases()['us']
    bindings = {chord: layout['option'][char] for char, chord in shortcuts(layout, 'us', base).items()}
    def glyph(value): return escape(layout['states'][value['state']]['terminator'] if isinstance(value, dict) else value)
    rows = []
    for row in GRID:
        cells = []
        for code in row:
            main, shifted = (bindings.get((code, index), '') for index in (OPTION, OPTION_SHIFT))
            label = escape(LABELS.get(code, base['keys'][str(code)][PLAIN]))
            classes = 'dead' if isinstance(main, dict) else ''
            if code in LABELS: content = f'<span class="key-name">{label}</span>'
            else:
                content = (f'<span class="key-shift">{glyph(shifted)}</span><span class="key-option">{glyph(main)}</span>'
                           f'<span class="key-label">{label}</span>')
            cells.append(f'<td class="{classes}" colspan="{SPANS.get(code, 1)}">{content}</td>')
        rows.append('<tr>' + ''.join(cells) + '</tr>')
    table = '\n'.join(rows)
    return r'''<!-- Generated by basedpl.keyboards from layout.json and keyboards.json. Do not edit. -->
# Keyboard map

Hold Option (Alt) for the large character. Add Shift for the small character above it. The small label at the bottom is the ordinary key.

```{=html}
<div class="bpl-keyboard-wrap">
<table class="bpl-keyboard" aria-label="US BPL Option keyboard">
''' + table + r'''
</table>
</div>
```

Orange borders mark dead keys. Release Option before typing the next key. Repeat the same Option chord, or press Space, to enter the dead key's own character. Follow-ups can be sequences, such as `-1` for a negative superscript or subscript.

''' + dead_key_table(layout) + r'''

This is the US-based map used in the browser and REPL, and by the native BasedPL-us macOS layout. Regional macOS layouts preserve their native Option characters and move conflicting BPL shortcuts. See [macOS installation](#macos-installation).

## macOS installation

From a BasedPL repository checkout, run `scripts/install-keyboard.sh` for the US layout. Pass `uk`, `de`, `fr` or `es` for British, German QWERTZ, French AZERTY or Spanish: for example, `scripts/install-keyboard.sh de`.

In System Settings, open Keyboard → Text Input → Edit, add `BasedPL-us` (or your chosen code), then select it from the input menu. If the layout isn't listed, log out and back in. `editors/macos/BasedPL-{code}.txt` lists each native layout's shortcuts.
'''


def write():
    "Generate regional bundles, shortcut tables, and the docs keyboard map, reusing the US icon."
    layout, image = json.loads(LAYOUT.read_text()), ICON.read_bytes()
    KEYBOARD_PAGE.write_text(keyboard_page(layout))
    for lang in SOURCES:
        name = f'BasedPL-{lang}'
        resources = bundle(lang)/'Resources'
        outputs = {resources/f'{name}.keylayout': keylayout(layout, lang).encode(), resources/f'{name}.icns': image,
                   OUTPUT/f'{name}.txt': keyboard_help(layout, lang).encode(), **metadata(lang)}
        for path, data in outputs.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
