"""Entry point when the user runs `farol` or `python -m farol`.

Loads every installed plugin via entry_points and hands control to the
Rust CLI.
"""

from __future__ import annotations

import sys

from ._manager import PluginManager


def main() -> None:
    manager = PluginManager.default()
    try:
        from ._native import run_cli
    except ImportError as e:
        print(
            "farol: native engine is not available; did the wheel build fail?\n"
            f"  {e}",
            file=sys.stderr,
        )
        sys.exit(2)
    run_cli(sys.argv, manager)


if __name__ == "__main__":
    main()
