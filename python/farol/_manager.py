"""Plugin discovery and dispatch.

Plugin authors declare hooks with the `@hookimpl` decorator and register
their package under the `farol.plugins` entry-point group. `PluginManager`
discovers them via `importlib.metadata.entry_points` and dispatches calls
from the Rust core.

Hooks available in v0.1:
- `on_config(config)`
- `on_files(files)`
- `on_page_markdown(markdown, page, config)`
- `on_page_html(html, page, config)`
- `on_post_build(site_dir, config)`
"""

from __future__ import annotations

import sys
from importlib.metadata import entry_points
from typing import Any

import pluggy

PROJECT_NAME = "farol"
ENTRY_POINT_GROUP = "farol.plugins"

hookspec = pluggy.HookspecMarker(PROJECT_NAME)
hookimpl = pluggy.HookimplMarker(PROJECT_NAME)


class _Specs:
    """Hookspecs: first-value-wins semantics. None means 'pass through'."""

    @hookspec(firstresult=True)
    def on_config(self, config: Any) -> Any | None:  # noqa: D401
        """Mutate or replace the resolved config once, at startup."""

    @hookspec(firstresult=True)
    def on_files(self, files: list[str]) -> list[str] | None:
        """Inspect the list of source files. v0.1: read-only."""

    @hookspec(firstresult=True)
    def on_page_markdown(self, markdown: str, page: Any, config: Any) -> str | None:
        """Rewrite markdown before parsing."""

    @hookspec(firstresult=True)
    def on_page_html(self, html: str, page: Any, config: Any) -> str | None:
        """Rewrite the rendered HTML body before the template wraps it."""

    @hookspec(firstresult=True)
    def on_post_build(self, site_dir: str, config: Any) -> None:
        """Called once after everything is written to disk."""


class PluginManager:
    """Wraps `pluggy.PluginManager` with a farol-flavored API."""

    def __init__(self) -> None:
        self._pm = pluggy.PluginManager(PROJECT_NAME)
        self._pm.add_hookspecs(_Specs)
        self._names: list[str] = []

    # ------------------------------------------------------------------
    # Construction helpers
    # ------------------------------------------------------------------
    @classmethod
    def default(cls) -> "PluginManager":
        """Discover and register every plugin under `farol.plugins`."""
        mgr = cls()
        eps = entry_points(group=ENTRY_POINT_GROUP)
        for ep in eps:
            try:
                plugin = ep.load()
            except Exception as e:  # noqa: BLE001
                print(
                    f"farol: failed to load plugin `{ep.name}`: {e}", file=sys.stderr
                )
                continue
            mgr.register(plugin, name=ep.name)
        return mgr

    @classmethod
    def null(cls) -> "PluginManager":
        """An empty manager - equivalent to the no-op host."""
        return cls()

    # ------------------------------------------------------------------
    # Registration
    # ------------------------------------------------------------------
    def register(self, plugin: Any, *, name: str | None = None) -> None:
        registered_name = self._pm.register(plugin, name=name)
        if registered_name is not None:
            self._names.append(str(registered_name))

    def plugins(self) -> list[str]:
        return list(self._names)

    # ------------------------------------------------------------------
    # Hook dispatch - one method per spec. Returns None when no plugin
    # produces a value (engine keeps the input unchanged).
    # ------------------------------------------------------------------
    def on_config(self, **kw: Any) -> Any | None:
        return self._pm.hook.on_config(**kw)

    def on_files(self, **kw: Any) -> Any | None:
        return self._pm.hook.on_files(**kw)

    def on_page_markdown(self, **kw: Any) -> Any | None:
        return self._pm.hook.on_page_markdown(**kw)

    def on_page_html(self, **kw: Any) -> Any | None:
        return self._pm.hook.on_page_html(**kw)

    def on_post_build(self, **kw: Any) -> Any | None:
        return self._pm.hook.on_post_build(**kw)


__all__ = ["PluginManager", "hookimpl", "hookspec"]
