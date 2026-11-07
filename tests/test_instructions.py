import platform, sys
from pathlib import Path
import pytest
from basedpl.bpltests import instruction_budgets, _perf_counts


@pytest.mark.skipif((sys.platform, platform.machine()) != ('darwin', 'arm64'), reason='the counts are set for macOS on Apple silicon')
def test_instruction_budgets():
    budgets = instruction_budgets(calls=Path(__file__).with_name('instructions.txt').read_text())
    if budgets.outside: pytest.fail(f'\n{budgets!r}', pytrace=False)


def test_perf_callchains():
    samples, total = _perf_counts(r"""    7851c26b4e74 malloc
    5e0882fd6179 <alloc::raw_vec::RawVecInner>::try_allocate_in


    7851c26b4e74 malloc
    5e0882fd6179 <alloc::raw_vec::RawVecInner>::try_allocate_in

    7851c26b4e74 [unknown]
    5e0882fd6179 <alloc::raw_vec::RawVecInner>::try_allocate_in
""")
    assert samples == {'malloc':2, '[unknown]':1} and total == 3
    assert _perf_counts('7d4d6e4a13d4 kernel') == ({'kernel':1}, 1)
    assert _perf_counts('1 leaf\n 2 caller', callers=1) == ({'leaf\tcaller':1}, 1)
    for bad in ('', 'not-a-frame\n    7851c26b4e74 malloc', '1 [unknown]\n 2 [unknown]'):
        with pytest.raises(ValueError): _perf_counts(bad)
