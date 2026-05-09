---
title: CLI Reference
---

# CLI Reference

## `farol new`

Create a new documentation project.

```bash
farol new <name>
```

Creates a directory with a starter `farol.toml` and `docs/index.md`.

## `farol build`

Build the static site.

```bash
farol build [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--config` | `farol.toml` | Path to config file |
| `--clean` | `false` | Remove `site/` before building |

Output goes to `site/` by default.

## `farol serve`

Start a development server with live reload.

```bash
farol serve [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--port` | `8000` | Port to listen on |
| `--host` | `127.0.0.1` | Bind address |
| `--open` | `true` | Open browser on start |
| `--config` | `farol.toml` | Path to config file |

## `farol clean`

Remove the `site/` output directory.

```bash
farol clean
```
