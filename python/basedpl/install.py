"Install the BPL kernel, fonts and macOS keyboard for the current user."
import json, os, shutil, sys, sysconfig
from pathlib import Path
from fastcore.script import call_parse
from jupyter_core.paths import jupyter_data_dir
from .assets import ASSETS, kernel_spec


@call_parse(pos=['code'])
def main(
    code:str='us', # macOS keyboard layout code
):
    "Install the BPL kernel and fonts, plus the selected macOS keyboard."
    if sys.platform not in ('darwin', 'linux'): raise SystemExit('basedpl-install supports macOS and Linux.')
    keyboard = ASSETS/'macos'/f'BasedPL-{code}.bundle'
    if not keyboard.is_dir(): raise SystemExit(f'Unknown keyboard layout: {code}')
    binary = Path(sysconfig.get_path('scripts'))/'bpl'
    if not binary.is_file(): raise SystemExit(f'BPL executable not found: {binary}')
    mac = sys.platform == 'darwin'
    data = Path(os.environ.get('XDG_DATA_HOME') or Path.home()/'.local/share')
    if not mac and not data.is_absolute(): raise SystemExit('XDG_DATA_HOME must be an absolute path.')
    fonts = Path.home()/'Library/Fonts' if mac else data/'fonts/basedpl'
    kernel = Path(jupyter_data_dir())/'kernels/bpl'
    kernel.mkdir(parents=True, exist_ok=True)
    (kernel/'kernel.json').write_text(json.dumps(kernel_spec(binary), indent=2) + '\n')
    fonts.mkdir(parents=True, exist_ok=True)
    for source in (ASSETS/'fonts').iterdir(): shutil.copy2(source, fonts/source.name)
    print(f'Installed BasedPL kernel: {kernel}')
    print(f'Installed SAX2B and BQN386B: {fonts}')
    if mac:
        target = Path.home()/'Library/Keyboard Layouts'/keyboard.name
        shutil.copytree(keyboard, target, dirs_exist_ok=True)
        print(f'Installed BasedPL-{code}. In System Settings, open Keyboard > Text Input > Edit, add BasedPL-{code}, then select it.')
        print('Log out and back in if the layout is not listed or the old layout remains active.')
    else: print('No native Linux keyboard layout is provided; use the REPL or browser glyph entry.')


if __name__ == '__main__': main()
