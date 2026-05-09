---
title: Themes
---

# Themes

## Built-in themes

Farol ships with three built-in themes:

### default

Sidebar navigation with table of contents panel. The general-purpose layout for most documentation sites.

```toml
[theme]
name = "default"
```

### api

Two-panel layout with a sticky navigation panel on the left and continuous-scroll content on the right. Designed for API references.

```toml
[theme]
name = "api"
```

### book

Linear reading layout with generous line-height, serif body text, and prev/next chapter navigation. Designed for tutorials, guides, and long-form content.

```toml
[theme]
name = "book"
```

## External themes

Point to a theme directory on your filesystem:

```toml
[theme]
path = "./my-theme"
```

### Theme structure

```
my-theme/
├── theme.toml          ← manifest
├── templates/
│   ├── default.html    ← main layout
│   └── partials/
│       ├── header.html
│       ├── footer.html
│       └── nav.html
└── assets/
    └── style.css
```

### `theme.toml`

```toml
[theme]
name = "my-theme"
version = "0.1.0"
description = "My custom theme"

[theme.layouts]
supported = ["default"]

[theme.assets]
css = ["style.css"]
shared_js = true
```

## Template context

All templates receive:

- `page` — `{ title, url, body_html, toc, frontmatter, layout }`
- `config` — full site configuration
- `nav` — navigation tree `[{ title, url, children }]`

Templates use MiniJinja (Jinja2-compatible) syntax.
