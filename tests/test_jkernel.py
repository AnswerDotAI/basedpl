import asyncio, sys
from conkernelclient import run_kernel


def streamed(messages): return ''.join(m['content']['text'] for m in messages if m['msg_type']=='stream')


async def j_kernel_story():
    async with run_kernel('J', argv=[sys.executable, '-m', 'basedpl.j', '{connection_file}']) as (_, kc):
        info = (await kc.shell_request('kernel_info_request'))['content']
        assert info['language_info']['name'] == 'J' and info['banner'].startswith('J j')
        for code, expected in [('2+2', '4\n'), ('x =: 41', ''), ('x + 1', '42\n'), ('mean =: 3 : 0\n(+/ y) % # y\n)\nmean 1 2 3 4', '2.5\n')]:
            _, messages = await kc.exec_ok(code)
            assert streamed(messages) == expected
        reply, _ = await kc.exec_drain("1 + 'a'")
        assert reply['content']['ename'] == 'JError' and 'domain error' in reply['content']['evalue']
        await kc.exec_ok('spin =: 3 : 0\nn =. 0\nwhile. n < 1e9 do. n =. n + 1 end.\n)')
        running = kc.run('spin 0', timeout=10)
        async for message in running:
            if message['msg_type'] == 'execute_input': break
        await asyncio.sleep(0.5)
        await kc.interrupt()
        reply = next(m for m in [m async for m in running] if m['msg_type']=='execute_reply')
        assert 'attention interrupt' in reply['content']['evalue']
        _, messages = await kc.exec_ok('x')
        assert streamed(messages) == '41\n'


def test_j_kernel(): asyncio.run(j_kernel_story())
