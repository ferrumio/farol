# farol

> **docs, lit.**
> Forged in Rust. Lit for life.

Fast, plugin-first documentation generator. Rust core. Python-friendly. Apache-2.0, free forever.

**Status:** pre-alpha.

---

## Why farol

A lighthouse - a *farol* - is built of iron and lit to guide.

That is what a docs site should be: a structure that stands on its own, and a beam that points the reader home. Existing tools fail at one or both. MkDocs is Python and slow; Material for MkDocs lives downstream of an owner's roadmap; Zensical gates features behind paid tiers; Docusaurus and Starlight ship JavaScript runtimes to render static prose; Hugo is fast but hostile to extension; Marmite is lovely but blog-first and AGPL.

farol is built for docs, in Rust, and lit by Python. The iron is fast. The beam is ergonomic. Both are free, forever.

---

## Design principles

1. **Fast by default.** 10k pages under 3 seconds cold, under 200ms warm. Incremental rebuild with a persistent dependency graph.
2. **Plugins are first-class, not a bolt-on.** Every builtin feature uses the same public plugin API the community uses.
3. **Python for plugin authors, Rust for the engine.** `pip install farol-plugin-foo` is all a user should need.
4. **Free forever, legally durable.** Apache-2.0, no CLA, trademark separated. The code cannot be rug-pulled, relicensed, or quietly closed.
5. **i18n and versioning are core, not plugins.** They shape the routing graph and belong in the engine.
6. **No JS framework in the output.** Static HTML, CSS, and small islands of vanilla JS. No React. No hydration.
7. **Great default theme.** One polished theme ships with the binary. Community themes distribute as pip packages.
8. **Honest errors.** Every parse, config, or render error points at the file, line, and column.

---

## Install

```bash
pip install farol
```

A pre-built wheel with the Rust engine embedded (no Rust toolchain required on your side).

## Quick start

```bash
farol new my-docs
cd my-docs
farol serve
```

Open `http://localhost:8000`. Edit a `.md` file. The browser refreshes in milliseconds.

## Plugins in ten lines

```python
# farol_plugin_wave/__init__.py
from farol import hookimpl

@hookimpl
def on_page_markdown(markdown, page, config):
    return markdown.replace(":wave:", "👋")
```

Register in `pyproject.toml`:

```toml
[project.entry-points."farol.plugins"]
wave = "farol_plugin_wave"
```

`pip install farol-plugin-wave` and it's picked up automatically. No base class, no manifest, no setup step.

Scaffold a new plugin in one command:

```bash
farol plugin new my-plugin
```

---

## Configuration

```toml
# farol.toml
site_name = "My Docs"
site_url  = "https://docs.example.com"

[theme]
name    = "default"
palette = "slate"
primary = "indigo"

[i18n]
default   = "en"
languages = ["en", "pt-BR", "es"]

[versioning]
current  = "v2.1"
versions = ["v2.1", "v2.0", "v1.9"]

[search]
stemming = "auto"
fuzzy    = true

[plugins]
enabled  = ["search", "sitemap", "admonitions", "code-copy"]
disabled = ["rss"]
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      farol CLI                          │
│         (Rust, distributed via pip as wheel)            │
└────────────────────┬────────────────────────────────────┘
                     │
     ┌───────────────┴───────────────┐
     │                               │
┌────▼─────────┐            ┌────────▼────────────┐
│  Core (Rust) │            │  Plugin runtime     │
│              │            │  (PyO3 + pluggy)    │
│ - Parser     │   hooks    │                     │
│ - Graph      ├───────────►│  @hookimpl          │
│ - Cache      │            │  def on_page_md(…)  │
│ - Renderer   │            │                     │
│ - Search     │            │  Discovered via     │
│ - i18n       │            │  entry_points       │
│ - Versioning │            │                     │
└──────────────┘            └─────────────────────┘
```

---

## Builtin plugins

| Tier | Plugins | State |
|---|---|---|
| **Always on** | nav-builder, toc, frontmatter, link-resolver, asset-pipeline, sitemap | Not disableable |
| **On by default** | search, admonitions, code-copy, syntax-highlight, anchor-links, rss, redirects | Togglable |
| **Opt-in builtin** | mermaid, katex, mkdocs-migrate, social-cards, git-info, llm-txt, api-reference | Off by default |

Analytics, comments, and SaaS integrations are **community plugins**, not builtins. The core stays neutral.

---

## Default theme

One polished theme ships with the binary. Layouts: `default`, `landing`, `blog`, `api`. Light and dark, responsive, config-driven colors. Template engine: **MiniJinja** (Jinja2-compatible - Material for MkDocs templates port with minimal work).

Users can override individual templates in `overrides/` without forking the theme. Community themes are pip packages that may `extends = "default"` and override only what they need.

JavaScript budget for the theme: **under 15 KB gzipped**, all vanilla, zero framework. Works with JS disabled except for search and switchers.

---

## Roadmap

| Milestone | Scope |
|---|---|
| **M0** | CLI scaffold, parse MD → HTML, minimal theme, `build` and `serve` |
| **M1** | Incremental dependency graph, persistent cache, hot reload |
| **M2** | Python plugin API (PyO3 + pluggy), tier-1 and tier-2 builtins |
| **M3** | `tantivy` search (static + optional server) |
| **M4** | i18n and versioning |
| **M5** | Default theme polish, MiniJinja templates, plugin scaffolding CLI |
| **M6** | `pip install farol`, wheel distribution, `mkdocs-migrate` |
| **M7** | Dogfood - farol's own docs built with farol |

---

## Governance

- **Apache-2.0.** Patent grant included.
- **No CLA.** Contributions stay with their authors.
- **Trademark separate from code.** The name "farol" and the logo are protected; the code is free. Forks are welcome; misleading "official" forks are not.
- **RFC process** for breaking plugin-API changes, with six-month deprecation on major bumps.
- **Hook signature stability** guaranteed across minor versions.

---

## Non-goals

- Replacing full-featured SSGs like Next.js or Astro. farol is docs-shaped.
- Arbitrary React or Vue components inside content. MDX-style mixing is out.
- A hosted service. farol builds static output; host it wherever.
- Paid tiers. Ever.

---

## Origin

The word *farol* comes from *Pharos*, the lighthouse of Alexandria - the first structure built to guide travelers home. Modern lighthouses are iron towers; the mirrors that first focused their flame were polished bronze. Metal, shaped to point the way.

farol is forged in [**ferrumio**](https://github.com/ferrumio) - a foundry of Rust-powered developer tools. From the same forge as [pydynox](https://github.com/ferrumio/pydynox), farol is the piece built to light the path.

---

## License

Apache-2.0. See [LICENSE](LICENSE).
