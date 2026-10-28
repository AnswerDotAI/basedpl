import json, os, subprocess, sys, sysconfig
from pathlib import Path
import pytest
from basedpl.assets import ASSETS


@pytest.mark.skipif(sys.platform not in ('darwin', 'linux'), reason='the installer supports macOS and Linux')
def test_install_for_user(tmp_path):
    env = os.environ | dict(HOME=str(tmp_path), JUPYTER_DATA_DIR=str(tmp_path/'jupyter'), XDG_DATA_HOME=str(tmp_path/'xdg'))
    command = Path(sysconfig.get_path('scripts'))/'basedpl-install'
    for _ in range(2): subprocess.run([command, 'de'], env=env, cwd=tmp_path, check=True, capture_output=True, text=True)
    spec = json.loads((tmp_path/'jupyter/kernels/bpl/kernel.json').read_text())
    assert spec['argv'] == [str(Path(sysconfig.get_path('scripts'))/'bpl'), '--kernel', '-f', '{connection_file}']
    fonts = tmp_path/'Library/Fonts' if sys.platform == 'darwin' else tmp_path/'xdg/fonts/basedpl'
    assert (fonts/'SAX2B.ttf').is_file() and (fonts/'BQN386B.ttf').is_file()
    for source in (ASSETS/'fonts').iterdir(): assert (fonts/source.name).read_bytes() == source.read_bytes()
    if sys.platform == 'darwin':
        source = ASSETS/'macos/BasedPL-de.bundle/Contents/Resources/BasedPL-de.keylayout'
        assert (tmp_path/'Library/Keyboard Layouts/BasedPL-de.bundle/Contents/Resources'/source.name).read_bytes() == source.read_bytes()
