// Based on Adám Brudzewsky's APL language bar: https://abrudz.github.io/lb
// MIT License, Copyright (c) 2011-2020 Nikolay G. Nikolov and Adam Brudzevski.
// BPL name completion, editor adapters, dark mode and overlay layout by Jeremy Howard.
((symbols, input, layout) => {
    function attach(monaco, container, magics = false) {
        const d = container.ownerDocument, w = d.defaultView, events = new AbortController();
        const on = (target, name, fn, capture = false) => target.addEventListener(name, fn, {capture, signal: events.signal});
        const {inCode, bplStart, entry, press, reset} = input(symbols, layout);
        const shortcuts = new Map(symbols.map(({glyph, shortcut}) => [glyph, shortcut]));
        let leftAlt = false, rightAlt = false;

        function textareaRect(t) {
            const mirror = d.createElement('div'), caret = d.createElement('span'), css = getComputedStyle(t), rect = t.getBoundingClientRect();
            for (const p of ['font', 'lineHeight', 'letterSpacing', 'padding', 'border', 'boxSizing', 'width', 'tabSize']) mirror.style[p] = css[p];
            Object.assign(mirror.style, {position: 'fixed', visibility: 'hidden', whiteSpace: 'pre-wrap', overflowWrap: 'break-word',
                left: `${rect.left - t.scrollLeft}px`, top: `${rect.top - t.scrollTop}px`});
            mirror.textContent = t.value.slice(0, t.selectionStart);
            caret.textContent = '\u200b';
            mirror.append(caret);
            d.body.append(mirror);
            const result = caret.getBoundingClientRect();
            mirror.remove();
            return result;
        }

        function editor(target) {
            if (target.closest?.('.text_cell, .jp-MarkdownCell')) return;
            const m = monaco?.editor.getEditors().find(e => e.hasTextFocus());
            if (m) {
                const model = m.getModel(), p = m.getPosition(), selection = m.getSelection();
                if (!model || !p || !selection) return;
                const valid = () => m.getDomNode()?.isConnected && m.getModel() === model && !m.getOption(monaco.editor.EditorOption.readOnly)
                    && (model.getLanguageId() === 'bpl' || magics && bplStart({text: model.getValue(), pos: model.getOffsetAt(m.getPosition())}) >= 0);
                if (!valid()) return;
                const e = {id: model, valid, text: model.getValue(), pos: model.getOffsetAt(p), empty: selection.isEmpty(), bpl: model.getLanguageId() === 'bpl',
                    rect: () => {
                        const r = m.getDomNode().getBoundingClientRect(), c = m.getScrolledVisiblePosition(p);
                        return {left: r.left + c.left, bottom: r.top + c.top + c.height};
                    },
                    insert: (text, from) => {
                        if (!valid()) return;
                        const selection = m.getSelection();
                        from ??= model.getOffsetAt(selection.getStartPosition());
                        const a = model.getPositionAt(from), b = selection.getEndPosition();
                        m.pushUndoStop();
                        m.executeEdits('aplnb', [{range: {startLineNumber: a.lineNumber, startColumn: a.column,
                            endLineNumber: b.lineNumber, endColumn: b.column}, text}], () => {
                            const end = model.getPositionAt(from + text.length);
                            return [new monaco.Selection(end.lineNumber, end.column, end.lineNumber, end.column)];
                        });
                        m.pushUndoStop();
                        m.focus();
                    }};
                return bplStart(e) >= 0 ? e : undefined;
            }
            if (!magics) return;
            const view = target.closest?.('.cm-editor')?.querySelector('.cm-content')?.cmTile?.view;
            if (view) {
                if (view.state.readOnly) return;
                const sel = view.state.selection.main;
                const e = {id: view, valid: () => view.dom.isConnected && !view.state.readOnly
                    && bplStart({text: view.state.doc.toString(), pos: view.state.selection.main.head}) >= 0,
                    text: view.state.doc.toString(), pos: sel.head, empty: sel.empty,
                    rect: () => view.coordsAtPos(sel.head),
                    insert: (text, from) => {
                        if (!e.valid()) return;
                        const sel = view.state.selection.main;
                        from ??= sel.from;
                        view.dispatch({changes: {from, to: sel.to, insert: text}, selection: {anchor: from + text.length}, userEvent: 'input.complete'});
                        view.focus();
                    }};
                return bplStart(e) >= 0 ? e : undefined;
            }
            if (target.tagName !== 'TEXTAREA' || target.readOnly || target.disabled) return;
            const e = {id: target, valid: () => target.isConnected && !target.readOnly && !target.disabled
                && bplStart({text: target.value, pos: target.selectionStart, bpl: target.hasAttribute('data-bpl')}) >= 0,
                text: target.value, pos: target.selectionStart, empty: target.selectionStart === target.selectionEnd, bpl: target.hasAttribute('data-bpl'),
                rect: () => textareaRect(target),
                insert: (text, from = target.selectionStart) => {
                    if (!e.valid()) return;
                    target.focus();
                    target.selectionStart = from;
                    d.execCommand('insertText', false, text);
                }};
            return bplStart(e) >= 0 ? e : undefined;
        }

        const host = d.createElement('div');
        host.innerHTML = `<div class="ngn_lb" aria-label="BPL symbols"><button type="button" class="ngn_x" style="float:right" title="Close symbol bar">×</button></div>
            <div class="bpl_choices" role="group" aria-label="BPL symbol completions" hidden></div>
            <style>
            .ngn_lb,.bpl_choices{background:var(--bpl-bar-background,#eee);color:var(--bpl-bar-color,#111);font:15px var(--bpl-font,SAX2B),monospace}
            .ngn_lb{border-bottom:1px solid #999;padding:2px}
            .ngn_lb button,.bpl_choices button{font:inherit;color:inherit;background:none;border:0;cursor:pointer;padding:2px 4px}
            .ngn_lb button:hover,.bpl_choices button:hover{background:#777;color:white}
            .bpl_choices{position:fixed;z-index:2147483647;max-height:240px;max-width:calc(100vw - 16px);overflow:auto;border:1px solid #888;border-radius:4px;box-shadow:0 3px 12px #0003;padding:4px}
            .bpl_choices button{display:block;width:100%;text-align:left;white-space:nowrap}
            .bpl_choices small{display:block;padding:4px}
            .ngn_lb .bpl_dead::after{content:'▾';font-size:9px;margin-left:1px}
            .bpl_choices.bpl_menu{max-width:min(320px,calc(100vw - 16px))}
            .bpl_menu button{display:inline-block;width:auto;min-width:28px;text-align:center}
            </style>`;
        container.append(host);
        const bar = host.querySelector('.ngn_lb'), tip = host.querySelector('.bpl_choices');
        bar.querySelector('.ngn_x').onclick = () => { bar.hidden = true; };
        let active, lastEditor, choice, keyInput = false;
        const names = new Map(symbols.map(({glyph, name, monad, dyad}) =>
            [glyph, [name, monad, dyad].filter(Boolean).join(' ')]));
        function stateGlyphs(state) {
            const {terminator, keys} = layout.states[state];
            return [terminator, ...Object.values(keys).flatMap(value => typeof value === 'string' ? [value] : stateGlyphs(value.state))];
        }
        const groups = new Map(Object.values(layout.option).filter(value => value.state).map(({state}) =>
            [layout.states[state].terminator, [...new Set(stateGlyphs(state))]]));
        const variants = new Set([...groups.values()].flatMap(glyphs => glyphs.slice(1)));
        function button(glyph, name) {
            const b = d.createElement('button');
            b.type = 'button'; b.textContent = glyph; b.title = (name || glyph) + (shortcuts.get(glyph) || ''); b.dataset.glyph = glyph;
            return b;
        }
        // Related glyphs stay together; arrows meet at the boundary between menus and plain buttons.
        const order = [...new Set([
            ...'^_¯¨○⎕≡⍭↑↓←↦↢↣⇄∨∧⍺⍵∆∇⋄∊⍳⊂⊃∩∪⊥⊤⊣⊢×÷⌊⌈≤≥≠√π', ...groups.keys(), ...names.keys()
        ])];
        for (const glyph of [...order.filter(g => groups.has(g)), ...order.filter(g => !groups.has(g))]) {
            if (!groups.has(glyph) && (variants.has(glyph) || /^[\x20-\x7e]+$/.test(glyph))) continue;
            const b = button(glyph, names.get(glyph));
            if (groups.has(glyph)) {
                b.textContent = {'^': '²', '_': '₂'}[glyph] || glyph;
                b.classList.add('bpl_dead');
                b.setAttribute('aria-expanded', 'false');
                b.title += ' (variants)';
            }
            bar.append(b);
        }
        function cancel() {
            choice?.anchor?.setAttribute('aria-expanded', 'false');
            active = undefined; choice = undefined; tip.hidden = true;
        }
        function show(e, item, anchor) {
            tip.classList.toggle('bpl_menu', !!anchor);
            tip.setAttribute('aria-label', anchor ? 'BPL symbol variants' : 'BPL symbol completions');
            anchor?.setAttribute('aria-expanded', 'true');
            if (choice?.item.query !== item.query || choice?.anchor !== anchor) {
                tip.replaceChildren();
                for (const [glyph, name] of item.found) {
                    const b = button(glyph, name);
                    if (!anchor) b.textContent = `${glyph} ${b.title}`;
                    tip.append(b);
                }
                if (!item.found.length) {
                    const note = d.createElement('small'); note.textContent = 'Unknown symbol'; tip.append(note);
                }
            }
            choice = {e, item, anchor};
            tip.hidden = false;
            const r = anchor ? anchor.getBoundingClientRect() : e.rect();
            tip.style.left = `${Math.max(4, Math.min(r.left, w.innerWidth - tip.offsetWidth - 8))}px`;
            tip.style.top = `${Math.max(4, Math.min(r.bottom + 4, w.innerHeight - tip.offsetHeight - 8))}px`;
        }
        function refresh(target) {
            const e = editor(target), item = e && entry(e);
            if (item && active?.id === e.id && active.start === item.start) show(e, item);
            else cancel();
        }
        on(bar, 'mousedown', ev => {
            ev.preventDefault();
            const b = ev.target.closest('button');
            const opened = choice?.anchor === b;
            cancel();
            if (!lastEditor?.valid()) lastEditor = undefined;
            if (b?.dataset.glyph && lastEditor) {
                const glyph = b.dataset.glyph, group = groups.get(glyph);
                if (group) {
                    if (!opened) show(lastEditor, {query: glyph, found: group.map(g => [g, names.get(g)])}, b);
                } else lastEditor.insert(glyph);
            }
        });
        on(tip, 'mousedown', ev => {
            ev.preventDefault();
            const b = ev.target.closest('button');
            if (b && choice?.e.valid()) choice.e.insert(b.dataset.glyph, choice.item.start);
            cancel();
        });
        on(d, 'focusin', ev => { if (!host.contains(ev.target)) lastEditor = editor(ev.target); cancel(); });
        on(d, 'pointerdown', ev => { if (!host.contains(ev.target)) cancel(); }, true);
        on(d, 'pointerup', ev => { if (!host.contains(ev.target)) lastEditor = editor(ev.target); });
        for (const event of ['paste', 'cut', 'compositionstart', 'focusout']) on(d, event, cancel, true);
        on(d, 'input', ev => {
            if (!keyInput || ev.inputType !== 'insertText' && ev.inputType !== 'deleteContentBackward') cancel();
            keyInput = false;
            w.requestAnimationFrame(() => { if (!events.signal.aborted) { lastEditor = editor(ev.target); refresh(ev.target); } });
        });
        on(w, 'blur', () => { leftAlt = rightAlt = false; lastEditor = undefined; cancel(); reset(); });
        on(w, 'focusin', reset, true);
        on(w, 'keyup', ev => {
            if (ev.code === 'AltLeft') leftAlt = false;
            if (ev.code === 'AltRight') rightAlt = false;
            keyInput = false; lastEditor = editor(ev.target); if (active) refresh(ev.target);
        }, true);
        on(w, 'keydown', ev => {
            if (ev.key === 'Escape' && choice?.anchor) {
                cancel(); ev.preventDefault(); ev.stopImmediatePropagation(); return;
            }
            keyInput = false;
            if (ev.code === 'AltLeft') leftAlt = true;
            if (ev.code === 'AltRight') rightAlt = true;
            if (['Shift', 'Control', 'Alt', 'Meta'].includes(ev.key)) return;
            const e = editor(ev.target);
            if (!e || ev.isComposing || ev.defaultPrevented) { lastEditor = undefined; cancel(); reset(); return; }
            if (lastEditor && lastEditor.id !== e.id) { cancel(); reset(); }
            lastEditor = e;
            const body = bplStart(e);
            const pressed = press(ev, leftAlt && !rightAlt && !ev.getModifierState('AltGraph') && body >= 0);
            if (pressed) {
                if (pressed.text) e.insert(pressed.text);
                if (pressed.stop) { cancel(); ev.preventDefault(); ev.stopImmediatePropagation(); return; }
            }
            const item = entry(e), plain = !ev.ctrlKey && !ev.altKey && !ev.metaKey;
            const tab = ev.key === 'Tab' && plain && !ev.shiftKey, enter = ev.key === 'Enter';
            const typed = active?.id === e.id && active.start === item?.start;
            const delimiter = plain && ev.key.length === 1 && !/[a-z]/i.test(ev.key);
            if (item && (tab || typed && (enter || delimiter))) {
                if (item.found.length === 1) {
                    e.insert(item.found[0][0], item.start);
                    cancel();
                } else if (tab) {
                    active = {id: e.id, start: item.start};
                    show(e, item);
                }
                if (tab) { ev.preventDefault(); ev.stopImmediatePropagation(); return; }
            }
            if (ev.key === '`' && plain) {
                const updated = editor(ev.target), body = updated ? bplStart(updated) : -1;
                if (updated && body >= 0 && updated.empty && inCode(updated.text.slice(body, updated.pos))) active = {id: updated.id, start: updated.pos};
                else cancel();
            } else if (!(typed && plain && (/^[a-z]$/i.test(ev.key) || ev.key === 'Backspace'))) cancel();
            keyInput = plain && (ev.key.length === 1 || ev.key === 'Backspace');
        }, true);
        return {dispose() { events.abort(); reset(); lastEditor = undefined; cancel(); host.remove(); }};
    }

    function page(monaco) {
        const d = document;
        if (d.querySelector('.bpl_page_bar') || d.querySelector('meta[name=generator][content^=quarto]') && !d.querySelector('[data-bpl]')) return;
        const container = d.createElement('div');
        container.className = 'bpl_page_bar';
        container.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:2147483647';
        d.body.append(container);
        const input = attach(monaco, container, true), bar = container.querySelector('.ngn_lb');
        bar.insertAdjacentHTML('afterbegin', '<button class="ngn_o" style="float:right" title="Toggle overlay/push-down"></button>');
        let overlay = false;
        const padding = d.body.style.paddingTop, toggle = bar.querySelector('.ngn_o');
        try { overlay = localStorage.getItem('ngn_lb_overlay') === '1'; } catch {}
        function reflow() {
            toggle.textContent = overlay ? '▼' : '▲';
            d.body.style.paddingTop = overlay || bar.hidden ? padding : `${bar.offsetHeight}px`;
        }
        toggle.onclick = () => {
            overlay = !overlay;
            try { localStorage.setItem('ngn_lb_overlay', overlay ? '1' : '0'); } catch {}
            reflow();
        };
        const observer = new ResizeObserver(reflow);
        observer.observe(bar); reflow();
        return {dispose() { input.dispose(); observer.disconnect(); container.remove(); d.body.style.paddingTop = padding; }};
    }
    return {attach, page};
})
