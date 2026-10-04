"Build the extension with Cargo, as `cargo t` builds it, and install it in the editable package. Run it after Rust changes."
import json, os, shutil, subprocess, sysconfig
from pathlib import Path

root = Path(__file__).resolve().parent.parent
cmd = ['cargo', 'build', '--features', 'python', '--lib', '--message-format=json-render-diagnostics']
out = subprocess.run(cmd, cwd=root, stdout=subprocess.PIPE, text=True, check=True).stdout
msgs = [json.loads(line) for line in out.splitlines()]
lib, = [f for m in msgs if m.get('reason') == 'compiler-artifact' and m['target']['name'] == 'basedpl'
        for f in m['filenames'] if Path(f).suffix in ('.dylib', '.so', '.dll')]
dest = root/'python'/'basedpl'/f"_core{sysconfig.get_config_var('EXT_SUFFIX')}"
# A copy beside the target, then a rename, leaves running processes with the file they loaded.
tmp = dest.with_name(dest.name + '.tmp')
shutil.copy2(lib, tmp)
os.replace(tmp, dest)
print(f'Installed {dest.relative_to(root)}')
