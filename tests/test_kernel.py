import asyncio
from conkernelclient import run_kernel


def displayed(messages):
    return [(m['msg_type'], m['content']['text'] if m['msg_type']=='stream' else m['content']['data']['text/plain'])
        for m in messages if m['msg_type'] in ('stream', 'execute_result')]


async def kernel_story():
    async with run_kernel('apl') as (_, kc):
        info = await kc.shell_request('kernel_info_request')
        assert info['content']['implementation'] == 'basedpl' and info['content']['language_info']['name'] == 'apl'
        await kc.exec_ok(']box off', silent=True)
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
        reply, messages = await kc.exec_ok('v', user_expressions={'total': '+/v', 'bad': '1÷0'})
        assert displayed(messages) == [('execute_result', '4 5')]
        expressions = reply['content']['user_expressions']
        assert expressions['total']['data']['text/plain'] == '9' and expressions['bad']['ename'] == 'DOMAIN ERROR'
        for code, status in [('f←{', 'incomplete'), (')', 'invalid'), ('v←99', 'complete')]:
            assert (await kc.shell_request('is_complete_request', code=code))['content']['status'] == status
        _, messages = await kc.exec_ok('v')
        assert displayed(messages) == [('execute_result', '4 5')]
        for code, start, matches in [('⍳3 ⋄ `iot', 5, ['⍳']), ('mea', 0, ['mean']), ('"`iot', 5, [])]:
            result = (await kc.shell_request('complete_request', code=code, cursor_pos=len(code)))['content']
            assert (result['cursor_start'], result['cursor_end'], result['matches']) == (start, len(code), matches)
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
        result = (await kc.shell_request('inspect_request', code='•A', cursor_pos=2, detail_level=0))['content']
        assert result['found'] and 'Alphabet' in result['data']['text/plain']
        result = (await kc.shell_request('inspect_request', code='⍵', cursor_pos=1, detail_level=0))['content']
        assert result['found']
        _, messages = await kc.exec_ok(']help ?')
        assert 'Roll' in displayed(messages)[0][1]
        _, messages = await kc.exec_ok('?1')
        assert displayed(messages) == [('execute_result', '0')]
        _, messages = await kc.exec_ok('??')
        text = displayed(messages)[0][1]
        assert '?' in text and 'Roll' not in text
        result = (await kc.shell_request('inspect_request', code='"inspectme"', cursor_pos=5, detail_level=0))['content']
        assert not result['found']
        result = (await kc.shell_request('complete_request', code='•sr', cursor_pos=3))['content']
        assert result['matches'] == ['•src']
        reply, messages = await kc.exec_drain('⎕←7 ⋄ 1÷0')
        assert reply['content']['ename'] == 'DOMAIN ERROR' and displayed(messages) == [('stream', '7\n')]
        assert '1÷0' in '\n'.join(reply['content']['traceback'])

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
        _, messages = await kc.exec_ok('⎕←"SVG" ⋄ •svg ⍬')
        rich = [m['content']['data'] for m in messages if m['msg_type']=='execute_result']
        assert 'http://www.w3.org/2000/svg' in rich[0]['image/svg+xml']
        assert displayed(messages)[0] == ('stream', 'SVG\n')


def test_jupyter_kernel(): asyncio.run(kernel_story())
