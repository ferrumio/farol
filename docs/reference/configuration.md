---
title: Configuration
---

# Configuration

All configuration lives in `farol.toml` at the project root.

## Site

```toml
site_name = "My Project"
site_url = "https://example.com"
site_description = "Project documentation"
repo_url = "https://github.com/org/repo"
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `site_name` | string | required | Site title shown in header and meta tags |
| `site_url` | string | `""` | Canonical URL for the site |
| `site_description` | string | `""` | Meta description |
| `repo_url` | string | `""` | Link to source repository |

## Theme

```toml
[theme]
name = "default"    # "default", "api", or "book"
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme.name` | string | `"default"` | Built-in theme name |
| `theme.path` | string | — | Path to external theme directory |

### Built-in themes

- **default** — Sidebar navigation with table of contents. Best for general docs.
- **api** — Two-panel layout with sticky navigation. Best for API references.
- **book** — Linear reading layout with prev/next navigation. Best for tutorials and guides.

## Navigation

```toml
[[nav]]
title = "Home"
path = "index.md"

[[nav]]
title = "Guide"
path = "guide/"
children = [
  { title = "Getting Started", path = "guide/start.md" },
  { title = "Advanced", path = "guide/advanced.md" },
]
```

Navigation is auto-generated from the file tree if not specified.

## Plugins

```toml
[plugins]
syntax_highlight = true
search = true
admonitions = true
```

All built-in plugins are enabled by default. Set to `false` to disable.
