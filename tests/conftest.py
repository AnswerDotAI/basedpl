import pytest
from basedpl import apl


@pytest.fixture(autouse=True)
def cleared_workspace():
    apl(']clear')
    apl.timeout = None
