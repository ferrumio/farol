"""farol - fast, plugin-first documentation generator.

This package exposes:
- `hookimpl`  : decorator for plugin authors
- `hookspec`  : internal use; plugins rarely touch this
- `PluginManager` : discovers and dispatches plugin hooks
- `testing` : test harness for plugin authors

The actual engine lives in the Rust extension module `farol._native`.
"""

from __future__ import annotations

from ._manager import PluginManager, hookimpl, hookspec  # noqa: F401
from . import testing  # noqa: F401

try:
    from ._native import version as _rust_version
except ImportError:
    def _rust_version() -> str:  # type: ignore[misc]
        return "0.0.0-pure-python-stub"


def version() -> str:
    """Return the farol version string."""
    return _rust_version()


__all__ = ["PluginManager", "hookimpl", "hookspec", "testing", "version"]
