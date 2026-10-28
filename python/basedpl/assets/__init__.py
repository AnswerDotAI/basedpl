"Packaged setup assets and their repository output directory."
from importlib.resources import files
from pathlib import Path

ASSETS = files(__package__)
GENERATED = Path('python/basedpl/assets')


def kernel_spec(binary='{resource_dir}/../../../../bin/bpl'):
    "The Jupyter kernelspec for a native BPL executable."
    return dict(argv=[str(binary), '--kernel', '-f', '{connection_file}'], display_name='BasedPL', language='bpl', interrupt_mode='message')
