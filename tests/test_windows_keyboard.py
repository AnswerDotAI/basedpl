import json, re
from basedpl import windows_keyboard
from basedpl.keyboards import LAYOUT


def test_windows_layers_and_dead_keys():
    text = windows_keyboard.keylayout(json.loads(LAYOUT.read_text()))
    main, dead = text.split('static DEADKEY')
    rows = {vk: values.split(', ') for vk, values in re.findall(r"\{([^,]+), [01], (.+)\},", main)}
    assert {chr(int(c, 16)) for row in rows.values() for c in row[:2] if c.startswith('0x')} >= {chr(i) for i in range(32, 127)}
    assert rows['VK_OEM_7'][:2] == ['0x0027', '0x0022']
    for vk, terminator in [("'O'", '25cb'), ('VK_OEM_5', '236d')]:
        assert 'WCH_DEAD' in rows[vk]
        assert f'0x{terminator}' in main.split('{' + vk + ',')[1].splitlines()[1]
    transitions = {(int(k, 16), int(prefix, 16)): int(result, 16)
                   for k, prefix, result in re.findall(r'DEADTRANS\(0x(\w+), 0x(\w+), 0x(\w+), 0\)', dead)}
    assert transitions[ord('g'), ord('⍭')] == ord('⍒')
    assert transitions[ord('⍭'), ord('⍭')] == ord('⍭')
    assert transitions[ord(' '), ord('○')] == ord('○')
    assert transitions[ord('-'), ord('^')] == ord('⁻')
    assert transitions[ord('1'), ord('^')] == ord('¹')
    assert transitions[ord('-'), ord('_')] == ord('₋')
    assert transitions[ord('1'), ord('_')] == ord('₁')
