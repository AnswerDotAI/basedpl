import asyncio
from conkernelclient import run_kernel
from basedpl import symbols


def displayed(messages):
    return [(m['msg_type'], m['content']['text'] if m['msg_type']=='stream' else m['content']['data']['text/plain'])
        for m in messages if m['msg_type'] in ('stream', 'execute_result')]


async def kernel_story():
    async with run_kernel('bpl') as (_, kc):
        info = await kc.shell_request('kernel_info_request')
        assert info['content']['implementation'] == 'basedpl' and info['content']['language_info']['name'] == 'bpl'
        assert 'eval' in info['content']['supported_features']
        await kc.exec_ok('•prefs ["box":$f]', silent=True)
        for code, expected in [
            ('v←⍳3 ⋄ mean←+/÷≢', []),
            ('mean v', [('execute_result', '1')]),
            ('1 ⋄ ⎕←2 ⋄ 3', [('execute_result', '1'), ('stream', '2\n'), ('execute_result', '3')]),
            ('silent←{a←7} ⋄ silent 0', []),
            ('⍎"1 ⋄ ⎕←2 ⋄ 3"', [('execute_result', '1'), ('stream', '2\n'), ('execute_result', '3')]),
        ]:
            _, messages = await kc.exec_ok(code)
            assert displayed(messages) == expected
        reply, messages = await kc.exec_ok('⎕←8 ⋄ v←4 5', silent=True)
        assert not displayed(messages)
        assert reply['content']['execution_count'] == 5
        reply, messages = await kc.exec_ok('v', user_expressions={'total': '+/v', 'bad': '1÷"a"'})
        assert displayed(messages) == [('execute_result', '4 5')]
        expressions = reply['content']['user_expressions']
        assert expressions['total']['data']['text/plain'] == '9' and expressions['bad']['ename'] == 'DOMAIN ERROR'
        for code, status in [('f←{', 'incomplete'), ('s←"one', 'incomplete'), (')', 'invalid'), ('v←99', 'complete')]:
            assert (await kc.shell_request('is_complete_request', code=code))['content']['status'] == status
        _, messages = await kc.exec_ok('v')
        assert displayed(messages) == [('execute_result', '4 5')]
        async def complete(code): return (await kc.shell_request('complete_request', code=code, cursor_pos=len(code)))['content']
        for code, start, matches in [('⍳3 ⋄ `iot', 5, ['⍳']), ('mea', 0, ['mean']), ('"`iot', 5, [])]:
            result = await complete(code)
            assert (result['cursor_start'], result['cursor_end'], result['matches']) == (start, len(code), matches)
        # Backtick names match by name or prefix, in any case, with every name ranked alike. Of tied names, one that prefixes all the others wins.
        names = {'io': '⎕', 'iot': '⍳', 'RHO': '⍴', 'exponent': '*', 'power': '⍣', 'scan': '\\', 'scanfirst': '⍀', 'alpha': '⍺', 'alphaunderbar': '⍶',
                 'omegaunderbar': '⍹', 'replicate': '#', 'om': '⍵', 'omu': '⍹', 'sca': '\\', 'larr': '←', 'leftarr': '←', 'grup': '⍋'}
        for name, glyph in names.items(): assert (await complete('`' + name))['matches'] == [glyph], name
        for s in symbols:
            for word in filter(None, [s['name'], s['monad'], s['dyad']]):
                assert (await complete('`' + word.replace('-', '')))['matches'] == [s['glyph']], word
        for name in ['nosuchsymbol', 'lg', 'lrr']: assert (await complete('`' + name))['matches'] == [], name
        # An ambiguous prefix lists every glyph it matches, by glyph name or valence name alike.
        assert (await complete('`lar'))['matches'] == ['←', '⍺']
        assert len((await complete('`do'))['matches']) > 1 and len((await complete('`de'))['matches']) > 1
        # A backtick in a string or comment starts no name.
        for code in ['"`io', '"can\'t `io', '"a""`io', '⍝ `io']: assert (await complete(code))['matches'] == [], code
        for code in ['한+`io', '"text" `io', "''' `io", "'a' `io", '⍝ comment\n`io', '注 comment\n`io']:
            assert (await complete(code))['matches'] == ['⎕'], code
        await kc.exec_ok('•prefs ["hanzi_aliases":$t]', silent=True)
        assert (await complete('注 `io'))['matches'] == []
        await kc.exec_ok('•prefs ["hanzi_aliases":$f]', silent=True)
        definition = '{⍝ Sum without running during inspection\n⎕←999 ⋄ +/⍵}'
        await kc.exec_ok('inspectme←'+definition, silent=True)
        for detail in (0,1):
            result = (await kc.shell_request('inspect_request', code='inspectme 1 2', cursor_pos=5, detail_level=detail))['content']
            assert result['found'] and ('+/⍵' if detail else 'Sum without running') in result['data']['text/plain']
            _, messages = await kc.exec_ok(']help inspectme'+(' -source' if detail else ''))
            assert not any(m['msg_type']=='stream' for m in messages)
            assert ('+/⍵' if detail else 'Sum without running') in displayed(messages)[0][1]
        result = (await kc.shell_request('inspect_request', code='+', cursor_pos=1, detail_level=0))['content']
        assert result['found'] and 'adds' in result['data']['text/plain']
        result = (await kc.shell_request('inspect_request', code='$a', cursor_pos=2, detail_level=0))['content']
        assert result['found'] and 'Latin alphabet' in result['data']['text/plain']
        result = (await kc.shell_request('inspect_request', code='⍵', cursor_pos=1, detail_level=0))['content']
        assert result['found']
        _, messages = await kc.exec_ok(']help ¿')
        assert 'Roll' in displayed(messages)[0][1]
        _, messages = await kc.exec_ok(']help ?')
        assert '(dfns.ipynb)' in displayed(messages)[0][1]
        _, messages = await kc.exec_ok(']help ⌽')
        assert '(glyphs/circle-bar.qmd)' in displayed(messages)[0][1]
        _, messages = await kc.exec_ok('¿1')
        assert displayed(messages) == [('execute_result', '0')]
        _, messages = await kc.exec_ok('¿¿')
        text = displayed(messages)[0][1]
        assert '¿' in text and 'Roll' not in text
        result = (await kc.shell_request('inspect_request', code='"inspectme"', cursor_pos=5, detail_level=0))['content']
        assert not result['found']
        assert (await complete('•sr'))['matches'] == ['•src']
        reply, messages = await kc.exec_drain('⎕←7 ⋄ 1÷"a"')
        assert reply['content']['ename'] == 'DOMAIN ERROR' and displayed(messages) == [('stream', '7\n')]
        assert '1÷"a"' in '\n'.join(reply['content']['traceback'])

        # Receiving output before interruption proves output isn't buffered until completion.
        running = kc.run('⎕←9 ⋄ {∇⍵}0', timeout=10)
        async for message in running:
            if message['msg_type'] == 'stream':
                assert message['content']['text'] == '9\n'
                break
        await kc.interrupt()
        remaining = [message async for message in running]
        reply = next(m for m in remaining if m['msg_type']=='execute_reply')
        assert reply['content']['ename'] == 'KeyboardInterrupt'
        _, messages = await kc.exec_ok('mean v')
        assert displayed(messages) == [('execute_result', '4.5')]
        lines = '\n'.join(map(str, range(12)))
        for box in ('$t', '$f'):
            await kc.exec_ok(f'•prefs ["box":{box} "limit":8 "edges":2 "width":∞]', silent=True)
            source = f'text←"abcdefghijklmnop" ⋄ lines←"{lines}" ⋄ text ⋄ ["short" text] ⋄ lines ⋄ [lines "a\tb"]'
            _, messages = await kc.exec_ok(source)
            text, nested, multiline, nested_lines = [text for _, text in displayed(messages)]
            assert text.startswith('"abcd') and text.endswith('op"') and '…' in text
            assert '"short"' in nested and text in nested
            assert multiline.startswith('"0\n1\n2\n3') and multiline.endswith('\n10\n11"') and '\n⋮\n' in multiline
            assert r'"a\tb"' in nested_lines and '"0' in nested_lines and '⋮' in nested_lines
            reply, _ = await kc.exec_ok('', user_expressions={'preview': 'text'}, silent=True)
            assert reply['content']['user_expressions']['preview']['data']['text/plain'] == text
        _, messages = await kc.exec_ok('⎕←text ⋄ "-"•nput text')
        assert ''.join(text for _, text in displayed(messages)) == '"abcdefghijklmnop"\nabcdefghijklmnop'
        await kc.exec_ok('•prefs ["limit":1000 "edges":3]', silent=True)
        _, messages = await kc.exec_ok('⎕←"SVG" ⋄ •svg ⍬')
        rich = [m['content']['data'] for m in messages if m['msg_type']=='execute_result']
        assert 'http://www.w3.org/2000/svg' in rich[0]['image/svg+xml']
        assert displayed(messages)[0] == ('stream', '"SVG"\n')
        messages = [m async for m in kc.run('⎕←"Name?" ⋄ ⌽⎕', on_stdin=lambda msg: 'Jo', timeout=10)]
        assert displayed(messages) == [('stream', '"Name?"\n'), ('execute_result', '"oJ"')]
        reply, _ = await kc.exec_drain('⎕', allow_stdin=False)
        assert reply['content']['ename'] == 'IO ERROR'
        reply, _ = await kc.exec_drain('•nget "-"', allow_stdin=True)
        assert reply['content']['ename'] == 'IO ERROR'

        # Ctrl-C while `⎕` waits for the notebook's reply interrupts the evaluation.
        asked = asyncio.Event()
        async def unanswered(msg):
            asked.set()
            await asyncio.Event().wait()
        running = kc.run('⎕', on_stdin=unanswered, timeout=10)
        await asyncio.wait_for(asked.wait(), 10)
        await kc.interrupt()
        reply = next(m for m in [m async for m in running] if m['msg_type']=='execute_reply')
        assert reply['content']['ename'] == 'KeyboardInterrupt'

        # Native namespace inspection and expression MIME share the real session, without a Python-style function catalog.
        await kc.exec_ok('huge←⍳1000000 ⋄ reduce←/', silent=True)
        reply = await kc.control_request('debug_request', type='request', seq=1, command='inspectVariables', arguments={})
        variables = {v['name']: v for v in reply['content']['body']['variables']}
        assert variables['v']['value'] == '4 5' and len(variables['huge']['value']) <= 201
        for name, kind in [('mean', 'function'), ('reduce', 'operator')]:
            assert variables[name]['type'] == kind and variables[name]['presentationHint']['kind'] == 'method'
        exprs = ['+/v', 'mean', 'reduce', '"Hello"', '1÷"a"', '•svg ⍬', '1 ⋄ assigned←2']
        _, messages = await kc.exec_ok('', metadata={'eval': exprs}, store_history=False, stop_on_error=False)
        values = next(m['content']['data']['application/json'] for m in messages if m['msg_type']=='execute_result')
        assert values['+/v']['data']['text/plain'] == '9' and values['"Hello"']['data']['text/plain'] == '"Hello"'
        assert values['1 ⋄ assigned←2']['data']['text/plain'] == '2'
        assert values['1÷"a"']['ename'] == 'DOMAIN ERROR' and 'image/svg+xml' in values['•svg ⍬']['data']
        for name, kind in [('mean', 'function'), ('reduce', 'operator')]:
            assert values[name]['metadata']['type'] == kind and values[name]['data']['text/plain']


def test_jupyter_kernel(): asyncio.run(kernel_story())
