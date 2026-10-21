import platform, sys
import pytest
from basedpl.bpltests import instruction_budgets


@pytest.mark.skipif((sys.platform, platform.machine()) != ('darwin', 'arm64'), reason='the counts are set for macOS on Apple silicon')
def test_instruction_budgets():
    budgets = instruction_budgets()
    if budgets.outside: pytest.fail(f'\n{budgets!r}', pytrace=False)
