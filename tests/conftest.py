import pytest
from basedpl import bpl
from fastws.timelock import timing_lock


@pytest.fixture(autouse=True, scope='session')
def shared_timing_lock():
    "Hold the timing lock shared for the whole run, as builds do. A timing then never shares the machine with the tests."
    with timing_lock(): yield


@pytest.fixture(autouse=True)
def cleared_workspace():
    bpl(']clear')
    bpl.timeout = None
