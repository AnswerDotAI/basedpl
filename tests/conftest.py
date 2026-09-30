import pytest
from basedpl import bpl


@pytest.fixture(autouse=True)
def cleared_workspace():
    bpl(']clear')
    bpl.timeout = None
