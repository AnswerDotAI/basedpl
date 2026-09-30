((symbols, layout) => {
    // US physical keys, before macOS Option or another layout transforms event.key.
    const punctuation = {Backquote: '`~', Minus: '-_', Equal: '=+', BracketLeft: '[{', BracketRight: ']}',
        Backslash: '\\|', Semicolon: ';:', Quote: "'\"", Comma: ',<', Period: '.>', Slash: '/?'};
    function usKey({code, shiftKey}) {
        if (/^Key[A-Z]$/.test(code)) return shiftKey ? code.slice(3) : code.slice(3).toLowerCase();
        if (/^Digit[0-9]$/.test(code)) return shiftKey ? ')!@#$%^&*('[Number(code[5])] : code[5];
        return punctuation[code]?.[Number(shiftKey)];
    }

    // Composed input from `layout.json`, with the same rules as the REPL and the macOS layout. `pending` is the dead-key state
    // that the next key completes.
    let pending = null;
    const act = action => typeof action === 'string' ? (pending = null, action) : (pending = action.state, '');
    // What a key types: `{text, stop}`, where `stop` means the key itself does nothing more, or nothing for an ordinary key.
    // `option` is whether an Option chord counts here, and `code` whether a plain dead key such as `^` starts a sequence.
    function press(ev, option, code) {
        const key = usKey(ev), plain = !ev.altKey && !ev.ctrlKey && !ev.metaKey;
        if (pending) {
            const state = layout.states[pending];
            pending = null;
            if (plain && ev.key === ' ') return {text: state.terminator, stop: true};
            if (ev.key === 'Backspace' || ev.key === 'Escape') return {text: '', stop: true};
            if (plain && key in state.keys) return {text: act(state.keys[key]), stop: true};
            // Any other key types the terminator, then acts as if nothing were pending.
            const rest = press(ev, option, code);
            return {text: state.terminator + (rest?.text ?? ''), stop: rest?.stop ?? false};
        }
        if (option && ev.altKey && !ev.shiftKey && !ev.ctrlKey && !ev.metaKey && key in layout.option) return {text: act(layout.option[key]), stop: true};
        if (plain && code && key in layout.plain) return {text: act(layout.plain[key]), stop: true};
    }
    const reset = () => { pending = null; };

    // At each level (exact, prefix, prefixes of hyphen-separated parts) a name outranks a search word, as in `editor.rs`.
    function matches(query) {
        query = query.toLowerCase();
        const letters = word => word.replaceAll('-', '');
        let best = Infinity, found = [];
        for (const {glyph, name, monad, dyad, aliases} of symbols) {
            let rank = Infinity;
            [name, monad, dyad, ...aliases.split(' ')].filter(Boolean).forEach((word, i) => {
                const level = letters(word) === query ? 0 : letters(word).startsWith(query) ? 1 : 2;
                if (level === 2) {
                    let rest = query;
                    for (const part of word.split('-')) {
                        let n = 0;
                        while (n < part.length && part[n] === rest[n]) n++;
                        if (!n) break;
                        rest = rest.slice(n);
                        if (!rest) break;
                    }
                    if (rest) return;
                }
                rank = Math.min(rank, 2 * level + (i > 0));
            });
            if (rank < best) { best = rank; found = []; }
            if (rank < Infinity && rank === best) found.push([glyph, name]);
        }
        // A name that is a prefix of every other match wins: `om gives omega, `omu gives omega-underbar.
        const shortest = found.find(([, a]) => found.every(([, b]) => letters(b).startsWith(letters(a))));
        return shortest && found.length > 1 ? [shortest] : found;
    }

    function inCode(text, python = false) {
        let quote = '', comment = false;
        for (let i = 0; i < text.length; i++) {
            const c = text[i];
            if (comment) { if (c === '\n') comment = false; }
            else if (quote) {
                if (python && c === '\\') i++;
                else if (text.startsWith(quote, i)) { i += quote.length - 1; quote = ''; }
            } else if (c === "'" && !python) {
                if (i + 2 >= text.length) return false;
                i += 2;
            } else if (c === "'" || c === '"') {
                quote = python && text.startsWith(c.repeat(3), i) ? c.repeat(3) : c;
                i += quote.length - 1;
            } else if (c === (python ? '#' : '⍝')) comment = true;
        }
        return !quote && !comment;
    }

    function aplStart(e) {
        if (e.apl) return 0;
        const header = /^%%apl[^\S\n]*(?:\r?\n|$)/.exec(e.text);
        if (header) return e.pos >= header[0].length ? header[0].length : -1;
        const start = e.text.lastIndexOf('\n', e.pos - 1) + 1;
        const line = /^[ \t]*(?:[\p{ID_Start}_][\p{ID_Continue}]*[ \t]*=[ \t]*)?%apl[ \t]+/u.exec(e.text.slice(start, e.pos));
        return line && inCode(e.text.slice(0, start), true) ? start + line[0].length : -1;
    }

    function entry(e) {
        const body = aplStart(e), start = e.text.lastIndexOf('`', e.pos - 1);
        if (body < 0 || start < body || !e.empty || !inCode(e.text.slice(body, start))) return;
        const query = e.text.slice(start + 1, e.pos);
        if (/^[a-z]*$/i.test(query)) return {start, query, found: matches(query)};
    }

    return {matches, inCode, aplStart, entry, press, reset};
})
