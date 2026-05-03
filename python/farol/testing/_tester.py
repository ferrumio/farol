"""Pytest-style fixtures for plugin authors.

A `PluginTester` builds a throwaway site in a temporary directory, registers
a user-provided hook, runs a single-page build, and returns the rendered
HTML. No live filesystem state leaks between tests.
"""

from __future__ import annotations

import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from .._manager import PluginManager, hookimpl


@dataclass
class PluginTestResult:
    """Outcome of a single `PluginTester.build_page` call."""

    html: str
    """The final HTML written to `site/index.html`."""

    pages: int
    """Number of pages rendered (always 1 for `build_page`)."""

    broken_links: int
    """Number of broken internal links found."""


class PluginTester:
    """Run one-page builds with your hooks wired in. Disposable per test."""

    def __init__(self) -> None:
        self._hooks: list[Callable] = []
        self._markdown = "# Hello world\n"

    # ------------------------------------------------------------------
    def with_hook(self, fn: Callable) -> "PluginTester":
        """Register a hook function. Decorate with `@hookimpl` if you
        haven't already; PluginTester wraps it if needed."""
        self._hooks.append(fn)
        return self

    def with_markdown(self, markdown: str) -> "PluginTester":
        self._markdown = markdown
        return self

    # ------------------------------------------------------------------
    def build_page(self, markdown: str | None = None) -> PluginTestResult:
        """Build a site containing a single `index.md` and return the HTML."""
        if markdown is not None:
            self._markdown = markdown

        manager = PluginManager()
        for fn in self._hooks:
            manager.register(_HookModule(fn))

        try:
            from .._native import build as _build  # type: ignore
        except ImportError as e:  # pragma: no cover - native missing
            raise RuntimeError(
                "farol native engine is not available; install the wheel"
            ) from e

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "docs").mkdir()
            (root / "docs" / "index.md").write_text(self._markdown)
            (root / "farol.toml").write_text('site_name = "Test"\n')

            report = _build(str(root / "farol.toml"), manager)
            html = (root / "site" / "index.html").read_text()

        return PluginTestResult(
            html=html,
            pages=report["pages"],
            broken_links=report["broken_links"],
        )


class _HookModule:
    """Wrap a bare function so pluggy can register it as a plugin."""

    def __init__(self, fn: Callable) -> None:
        # pluggy needs the function attribute to carry the hookimpl marker.
        if not getattr(fn, "farol_impl", None):
            fn = hookimpl(fn)  # type: ignore[assignment]
        setattr(self, fn.__name__, fn)
