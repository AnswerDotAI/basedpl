import json, re
import xml.etree.ElementTree as ET
from basedpl.keyboards import LAYOUT, PLAIN, SHIFT, OPTION, OPTION_SHIFT, CONTROL_OPTION, attribute, keylayout, bases, shortcuts

layout = json.loads(LAYOUT.read_text())
KEYS = {char: (int(code), i) for code, values in bases()['us']['keys'].items() if int(code) <= 50
        for i, char in enumerate(values[:2]) if isinstance(char, str) and len(char) == 1 and char.isprintable()}


def parse(text):
    "The key maps, actions and terminators of a `.keylayout`. ElementTree reads XML 1.0, so control characters become private-use ones."
    text = re.sub(r'&#x00([01][0-9A-F]|7F);', lambda m: f'&#x{0xE000 + int(m[1], 16):04X};', text.replace('version="1.1"', 'version="1.0"', 1))
    root = ET.fromstring(text)
    maps = {int(m.get('index')): {int(k.get('code')): k.attrib for k in m} for m in root.iter('keyMap')}
    actions = {a.get('id'): {w.get('state'): w.attrib for w in a} for a in root.iter('action')}
    return maps, actions, {w.get('state'): w.get('output') for w in root.find('terminators')}


def simulate(parsed, presses):
    "What macOS types for `presses`, each a key code and key-map index."
    maps, actions, terminators = parsed
    state, out = 'none', ''
    for code, index in presses:
        key = maps[index][code]
        if 'output' in key: out, state = out + terminators.get(state, '') * (state != 'none') + key['output'], 'none'; continue
        entries = actions[key['action']]
        if state not in entries: out, state = out + terminators.get(state, ''), 'none'
        w = entries[state]
        state, out = w.get('next', 'none'), out + w.get('output', '')
    return out


def expected(presses):
    "What the rules of `layout.json` type for `presses`, each `('option', key)`, `('key', char)`, `('space',)` or `('delete',)`."
    state, out = None, ''
    for kind, *key in presses:
        s = layout['states'][state] if state else None
        if s and kind == 'option' and layout['option'].get(key[0]) == {'state': state}: value = s['terminator']
        elif s and kind == 'key' and key[0] in s['keys']: value = s['keys'][key[0]]
        else:
            if s: out, state = out + s['terminator'] * (kind != 'delete'), None
            if s and kind in ('space', 'delete'): continue
            special = {'space': ' ', 'delete': '\x08'}
            value = special[kind] if kind in special else layout['option'].get(key[0], '') if kind == 'option' else key[0]
        state, out = (value['state'], out) if isinstance(value, dict) else (None, out + value)
    return out


def codes(presses):
    "The key code and key-map index of each press."
    def press(kind, *key):
        if kind in ('space', 'delete'): return {'space': 49, 'delete': 51}[kind], PLAIN
        code, index = KEYS[key[0]]
        return (code, {PLAIN: OPTION, SHIFT: OPTION_SHIFT}[index]) if kind == 'option' else (code, index)
    return [press(*p) for p in presses]


def test_macos_follows_the_mapping_rules():
    assert attribute('"&<>') == '"&#x0022;&#x0026;&#x003C;&#x003E;"'
    parsed = parse(keylayout(layout))
    assert simulate(parsed, [(24, OPTION_SHIFT)]) == '≠'
    starts = {v['state']: [('option', k)] for k, v in layout['option'].items() if isinstance(v, dict)}
    while len(starts) < len(layout['states']):
        starts |= {v['state']: starts[n] + [('key', k)] for n in list(starts) for k, v in layout['states'][n]['keys'].items() if isinstance(v, dict)}
    sequences = [[('option', k)] for k in layout['option']] + [[('key', '^'), ('key', '2')]]
    for name, start in starts.items():
        sequences += [start + [('key', k)] for k in layout['states'][name]['keys']]
        sequences += [start + [('space',), ('key', 'x')], start + [('key', 'x')], start + [('delete',), ('key', 'a')], start + [('option', 'h')]]
    for presses in sequences: assert simulate(parsed, codes(presses)) == expected(presses), presses
    assert expected([('option', 'o'), ('key', '-')]) == '⊖'
    assert expected([('option', '6'), ('key', '-'), ('key', '1')]) == '⁻¹'
    assert expected([('option', 'o'), ('space',), ('key', '|')]) == '○|'
    assert expected([('option', 'q'), ('key', 'x')]) == '⎕x'


def test_regional_layouts_preserve_native_typing_and_bpl():
    native = bases()
    for lang, base in native.items():
        parsed = parse(keylayout(layout, lang))
        assert simulate(parsed, [(29, OPTION_SHIFT)]) == '⍬', lang
        for char, chord in shortcuts(layout, lang, base).items():
            value = layout['option'][char]
            presses = [tuple(chord)] + ([(49, PLAIN)] if isinstance(value, dict) else [])
            wanted = layout['states'][value['state']]['terminator'] if isinstance(value, dict) else value
            assert simulate(parsed, presses) == wanted, (lang, char)
            if isinstance(value, dict): assert simulate(parsed, [chord, chord, (49, PLAIN)]) == wanted + ' ', (lang, char)
    cases = [('uk', [(20, SHIFT)], '£'), ('uk', [(20, OPTION), (20, OPTION_SHIFT)], '#√'),
             ('de', [(30, OPTION_SHIFT)], '≠'),
             ('es', [(30, OPTION_SHIFT)], '≠'),
             ('de', [(33, PLAIN), (39, SHIFT)], 'üÄ'),
             ('de', [(23, OPTION), (22, OPTION)], '[]'), ('de', [(10, PLAIN), (0, PLAIN)], 'â'),
             ('de', [(10, PLAIN), (24, PLAIN), (0, PLAIN)], '^á'),
             ('de', [(23, OPTION_SHIFT), (18, PLAIN), (22, CONTROL_OPTION), (18, PLAIN)], '₁¹'),
             ('de', [(50, OPTION_SHIFT)], '≥'),
             ('de', [(6, OPTION)], '•'), ('fr', [(12, OPTION), (49, PLAIN)], '⍺'),
             ('fr', [(18, SHIFT), (20, PLAIN)], '1"'), ('fr', [(23, OPTION_SHIFT), (27, OPTION_SHIFT)], '[]'),
             ('fr', [(33, PLAIN), (14, PLAIN)], 'ê'), ('es', [(41, SHIFT), (39, PLAIN), (0, PLAIN)], 'Ñá')]
    for lang, presses, wanted in cases: assert simulate(parse(keylayout(layout, lang)), presses) == wanted, lang
