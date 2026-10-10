import json
from basedpl import windows_keyboard
from basedpl.keyboards import LAYOUT


def test_windows_layers_and_dead_keys():
    text = windows_keyboard.keylayout(json.loads(LAYOUT.read_text())).replace('\r', '')
    main, *blocks = text.split('\nDEADKEY ')
    rows = {r[1]: r[3:] for line in main.split('LAYOUT\n')[1].splitlines() if (r := line.split())}
    assert {chr(int(c, 16)) for row in rows.values() for c in row[:2]} >= {chr(i) for i in range(32, 127)}
    assert rows['OEM_7'][:2] == ['0027', '0022']
    assert rows['0'][-1] == '236C'
    assert rows['OEM_PLUS'][-1] == '2260'
    assert rows['OEM_5'][-2:] == ['236D@', '233D']
    dead = {b.splitlines()[0]: dict(line.split()[:2] for line in b.split('\n\n')[0].splitlines()[1:]) for b in blocks}
    assert dead['236D']['0067'] == '2352'
    assert dead['236D']['236D'] == '236D'
    assert dead['25CB']['0020'] == '25CB'
    assert dead['25CB']['005C'] == '2349'
    assert dead['005E']['002D'] == '207B'
    assert dead['005E']['0031'] == '00B9'
    assert dead['005F']['002D'] == '208B'
    assert dead['005F']['0031'] == '2081'
