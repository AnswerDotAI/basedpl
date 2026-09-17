"Console entry point; argument handling and execution stay in Rust."
import sys
from ._core import run_cli

def main(): return run_cli(sys.argv[1:])
