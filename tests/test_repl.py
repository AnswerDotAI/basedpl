import os, json
from importlib.resources import files
import fcntl, pty, re, select, subprocess, termios, time

def test_terminal_symbol_entry_and_exit():
    master, slave = pty.openpty()
    termios.tcsetwinsize(slave, (24, 100))
    # The pty is bpl's controlling terminal, as a shell's is, so Ctrl-C during an evaluation sends SIGINT.
    child = subprocess.Popen(['bpl'], stdin=slave, stdout=slave, stderr=slave, env={**os.environ, 'TERM': 'xterm-256color'},
                             start_new_session=True, preexec_fn=lambda: fcntl.ioctl(0, termios.TIOCSCTTY, 0))
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
        enter('•prefs ["box":$f]\r', '["box":$f "trees":$t "fns":$t "limit":1000ₓ "edges":3ₓ "prec":∞ "width":100ₓ]\r\n')
        os.write(master, b'"\x1ba')
        read_until('⍺:_-'.encode())
        enter('_"\r', '\r\n⍶\r\n')
        enter('"£\x1b£"\r', '\r\n£#\r\n')
        enter('"\x1b`\x1b` \x1ba\x1ba \x1b6\x1b66"\r', '\r\n⋄ ⍺ ^6\r\n')
        layout = json.loads(files('basedpl').joinpath('layout.json').read_text())
        typed = {k: v for k, v in layout['option'].items() if isinstance(v, str)}
        enter('"' + ''.join('\x1b'+k for k in typed) + '"\r', '\r\n' + ''.join(typed.values()) + '\r\n')
        enter('r\x1bh1+2\x1bl2\x1b.×\r', '\r\n')  # r←1+2→2↣×
        enter('\x1bq \x1bhr\r', '\r\n6\r\n')  # explicit output: Alt-q, then Space, types ⎕
        enter('3\x1b62\r', '\r\n9\r\n')  # Alt-6, then 2, types ²
        enter('"a^b"\r', '\r\na^b\r\n')  # in a string, ^ types itself
        enter('\x1bi_1 0 1\r', '\r\n[0 2]ₓ\r\n')  # Alt-i, then _, types ⍸
        # Dead keys: Backspace cancels, a plain key types both, Alt-c starts its own, - then 1 types ⁻¹, and Space types ^
        enter('"\x1bo\x7fx\x1box\x1bo\x1bct\x1b6-1\x1b6 "\r', '\r\nx○x○⍝⁻¹^\r\n')
        enter('1 2 3\x1bl+/\r', '\r\n6\r\n')
        enter('界`assign `io\t4\r', '\r\n')  # space, Tab, Unicode byte offsets
        enter('+/界\r', '\r\n6\r\n')
        enter('2`times3+4\r', '\r\n14\r\n')  # delimiter is retained
        enter('sum`assign +`reduce\r', '\r\n')  # Enter accepts and submits
        enter('sum 界\r', '\r\n6\r\n')
        enter('`iotx\x7fa3\r', '\r\n0 1 2\r\n')  # backspace while entering a name
        data = enter('`de\t \r', 'UNSUPPORTED')  # ambiguous Tab must not choose a glyph
        assert b'`de ' in data
        enter('`iota\x1b[D\r', 'UNSUPPORTED')  # moving the cursor ends name entry
        enter('x`lar-5\r', '\r\n')  # completion replaces only the name
        enter('x\r', '\r\n¯5\r\n')
        enter('2+2\r', '\r\n4\r\n')
        data = enter('⍝ \\ `iota \r', '\r\n')
        assert b'\\ `iota ' in data  # comments and literal backslash are untouched
        enter('6+7\r', '\r\n13\r\n')
        enter('\x1b[200~`iota\x1b[201~\r', 'UNSUPPORTED')  # pasted names do not auto-expand
        enter('(2+\r', '\r\n')
        enter('\x03', '\r\n')  # Ctrl-C discards the whole unfinished expression
        enter('2+3\r', '\r\n5\r\n')
        os.write(master, '⎕←"go" ⋄ •delay ∞\r'.encode())
        read_until(b'go\r\n')  # the evaluation is running
        enter('\x03', 'INTERRUPT')  # Ctrl-C interrupts the running evaluation, and the session continues
        os.write(master, '⌽⎕\r'.encode())
        read_until(b'\x1b[?2004l')  # readline has returned the terminal to line mode
        enter('ab\r', 'ba\r\n')  # ⎕ reads a line typed at the terminal
        os.write(master, b'\x04')
        tail = read_until(b'\r\n')
        assert child.wait(timeout=5) == 0
        visible = re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', tail)
        assert visible.endswith(b'\r\n')  # EOF must not leave the shell prompt indented
    finally:
        if child.poll() is None: child.kill()
        child.wait(timeout=5)
        os.close(master)
