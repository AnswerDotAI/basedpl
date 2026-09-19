import os, json
from importlib.resources import files
import pty, re, select, subprocess, termios, time

def test_terminal_symbol_entry_and_exit():
    master, slave = pty.openpty()
    termios.tcsetwinsize(slave, (24, 100))
    child = subprocess.Popen(['basedpl'], stdin=slave, stdout=slave, stderr=slave, env={**os.environ, 'TERM': 'xterm-256color'})
    os.close(slave)
    pending = b''

    def read_until(expected):
        nonlocal pending
        data, deadline = pending, time.monotonic() + 5
        while expected not in data:
            assert time.monotonic() < deadline, data.decode(errors='replace')
            if not select.select([master], [], [], max(0, deadline-time.monotonic()))[0]: continue
            chunk = os.read(master, 65536)
            assert chunk, data
            data += chunk
            if b'\x1b[6n' in chunk: os.write(master, b'\x1b[1;1R')  # terminal cursor-position query
        end = data.index(expected) + len(expected)
        result, pending = data[:end], data[end:]
        return result

    def enter(code, expected):
        os.write(master, code.encode())
        data = read_until(expected.encode())
        read_until(b'\x1b[?2004h')  # next readline is in raw mode and ready for input
        return data

    try:
        read_until(b'\x1b[?2004h')
        enter('1 2\r', '│1 2│\r\n└~──┘\r\n')
        enter(']box off\r', 'OFF -style=max -trains=tree -fns=on\r\n')
        keys = json.loads(files('basedpl').joinpath('keyboard.json').read_text())
        enter("'" + ''.join('\x1b'+k for k in keys) + "'\r", '\r\n' + ''.join(keys.values()) + '\r\n')
        enter('r\x1bh1+2\x1bl2\x1bu×\r', '\r\n')  # r←1+2→2∘×
        enter('\x1b]\x1bhr\r', '\r\n6\r\n')  # explicit output via Alt-]
        enter('1 2 3\x1bl+/\r', '\r\n6\r\n')
        enter('界`assign `io\t4\r', '\r\n')  # space, Tab, Unicode byte offsets
        enter('+/界\r', '\r\n10\r\n')
        enter('2`times3+4\r', '\r\n14\r\n')  # delimiter is retained
        enter('sum`assign +`reduce\r', '\r\n')  # Enter accepts and submits
        enter('sum 界\r', '\r\n10\r\n')
        enter('`iotx\x7fa3\r', '\r\n1 2 3\r\n')  # backspace while entering a name
        data = enter('`sca\t \r', 'UNSUPPORTED')  # ambiguous Tab must not choose scan
        assert b'`sca ' in data
        enter('2+2\r', '\r\n4\r\n')
        data = enter('⍝ \\ `iota \r', '\r\n')
        assert b'\\ `iota ' in data  # comments and literal backslash are untouched
        enter('6+7\r', '\r\n13\r\n')
        enter('\x1b[200~`iota\x1b[201~\r', 'UNSUPPORTED')  # pasted names do not auto-expand
        enter('(2+\r', '\r\n')
        enter('\x03', '\r\n')  # Ctrl-C discards the whole unfinished expression
        enter('2+3\r', '\r\n5\r\n')
        os.write(master, b'\x04')
        tail = read_until(b'\r\n')
        assert child.wait(timeout=5) == 0
        visible = re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', tail)
        assert visible.endswith(b'\r\n')  # EOF must not leave the shell prompt indented
    finally:
        if child.poll() is None: child.kill()
        child.wait(timeout=5)
        os.close(master)
