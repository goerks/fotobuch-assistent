"""Einstiegspunkt für die gepackte .exe (PyInstaller)."""
import sys

from fotobuch.cli import main

if __name__ == "__main__":
    sys.exit(main())
