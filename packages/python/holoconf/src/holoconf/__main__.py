"""Entry point for running holoconf as a module: python -m holoconf"""

import sys

from holoconf.cli import main

if __name__ == "__main__":
    sys.exit(main())
