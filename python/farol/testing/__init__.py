"""Testing utilities for farol plugin authors.

Example:

    from farol.testing import PluginTester
    from my_plugin import on_page_markdown

    def test_wave():
        result = PluginTester().with_hook(on_page_markdown).build_page("# :wave:")
        assert "👋" in result.html
"""

from ._tester import PluginTester, PluginTestResult

__all__ = ["PluginTester", "PluginTestResult"]
