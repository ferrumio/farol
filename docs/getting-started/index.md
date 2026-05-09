---
title: Getting Started
---

# Getting Started

## Install

```bash
pip install farol
```

A pre-built wheel with the Rust engine embedded. No Rust toolchain required.

## Create a site

```bash
farol new my-docs
cd my-docs
```

This creates:

```
my-docs/
├── docs/
│   └── index.md
└── farol.toml
```

## Serve locally

```bash
farol serve
```

Opens a dev server at `http://localhost:8000` with live reload. Edit any markdown file and the browser updates instantly.

## Build for production

```bash
farol build
```

Generates static HTML in `site/`. Deploy anywhere that serves static files.

## Project structure

```
my-project/
├── docs/           ← your markdown files
│   ├── index.md
│   └── guide.md
├── farol.toml      ← configuration
└── site/           ← generated output (gitignored)
```

## Next steps

- [Configuration reference](../reference/configuration.md) — customize your site
- [Writing docs](../guides/writing.md) — learn the markdown extensions
- [Deploy](../guides/deploy.md) — publish to GitHub Pages, Netlify, etc.
