---
title: Plugins
---

# Plugins

## Built-in plugins

All built-in plugins are enabled by default:

| Plugin | Description |
|--------|-------------|
| `syntax_highlight` | Code block syntax highlighting |
| `admonitions` | `:::note`, `:::warning`, etc. |
| `anchor_links` | Clickable heading anchors |
| `code_copy` | Copy button on code blocks |
| `search` | Client-side full-text search |
| `prev_next` | Previous/next page navigation |
| `reading_time` | Estimated reading time |
| `edit_on_git` | "Edit this page" link |
| `sitemap` | Generate `sitemap.xml` |
| `redirects` | Redirect old URLs |

### Disabling a plugin

```toml
[plugins]
search = false
reading_time = false
```

## Writing a plugin

Plugins are Python packages that implement the farol plugin protocol.

### Minimal plugin

```python title="farol_plugin_hello/__init__.py"
from farol import hookimpl

@hookimpl
def on_page_html(page, html):
    """Called after each page is rendered to HTML."""
    return html.replace("Hello", "Olá")
```

### Plugin hooks

| Hook | Timing | Description |
|------|--------|-------------|
| `on_config` | Start | Modify configuration |
| `on_page_markdown` | Before render | Transform markdown source |
| `on_page_html` | After render | Transform rendered HTML |
| `on_post_build` | End | Run after all pages built |

### Packaging

```toml title="pyproject.toml"
[project]
name = "farol-plugin-hello"
version = "0.1.0"

[project.entry-points."farol.plugins"]
hello = "farol_plugin_hello"
```

Install with:

```bash
pip install farol-plugin-hello
```
