---
title: Home
---

# farol

> docs, lit.

Fast, plugin-first documentation generator. Rust core. Python-friendly. Apache-2.0, free forever.

## Why farol

A lighthouse — a *farol* — is built of iron and lit to guide.

That is what a docs site should be: a structure that stands on its own, and a beam that points the reader home.

- **Fast by default.** 10k pages under 3 seconds cold, under 200ms warm.
- **Plugins are first-class.** Every builtin feature uses the same public plugin API.
- **Python for plugins, Rust for the engine.** `pip install farol-plugin-foo` is all you need.
- **Free forever.** Apache-2.0, no CLA, no rug-pull.
- **No JS framework in the output.** Static HTML, CSS, vanilla JS islands.

## Quick start

```bash
pip install farol
farol new my-docs
cd my-docs
farol serve
```

## Learn more

- [Getting started](getting-started/index.md) — install, create, serve, publish
- [Configuration](reference/configuration.md) — all `farol.toml` keys
- [Writing docs](guides/writing.md) — frontmatter, admonitions, code blocks
- [Themes](guides/themes.md) — built-in themes, overrides, custom themes
- [Plugins](guides/plugins.md) — using and writing plugins
- [CLI reference](reference/cli.md) — all subcommands
