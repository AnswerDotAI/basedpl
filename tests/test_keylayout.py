import json, re
import xml.etree.ElementTree as ET
from basedpl.editors import LAYOUT, CHARACTER_KEYS, PLAIN, SHIFT, OPTION, attribute, keylayout

layout = json.loads(LAYOUT.read_text())
KEYS = {c: (code, i) for code, pair in CHARACTER_KEYS.items() for i, c in zip([PLAIN, SHIFT], pair)}


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
        state, out = (w['next'], out) if 'next' in w else ('none', out + w['output'])
    return out


def expected(presses):
    "What the rules of `layout.json` type for `presses`, each `('option', key)`, `('key', char)`, `('space',)` or `('delete',)`."
    state, out = None, ''
    for kind, *key in presses:
        s = layout['states'][state] if state else None
        if s and kind == 'key' and key[0] in s['keys']: value = s['keys'][key[0]]
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
        return (KEYS[key[0]][0], OPTION) if kind == 'option' else KEYS[key[0]]
    return [press(*p) for p in presses]


def test_macos_follows_the_mapping_rules():
    assert attribute('"&<>') == '"&#x0022;&#x0026;&#x003C;&#x003E;"'
    parsed = parse(keylayout(layout))
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
