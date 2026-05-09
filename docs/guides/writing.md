---
title: Writing Docs
---

# Writing Docs

## Frontmatter

Every page can start with YAML frontmatter:

```yaml
---
title: My Page Title
layout: default
---
```

| Key | Description |
|-----|-------------|
| `title` | Page title (used in nav and `<title>` tag) |
| `layout` | Template layout to use (default: `default`) |

## Admonitions

Use `:::` fenced blocks:

```markdown
:::tip
This is a helpful tip.
:::

:::warning
Be careful with this.
:::
```

Supported types: `note`, `tip`, `important`, `warning`, `caution`.

## Code blocks

Standard fenced code blocks with syntax highlighting:

````markdown
```rust
fn main() {
    println!("Hello, farol!");
}
```
````

### Filenames

Add a filename with the `title` attribute:

````markdown
```rust title="src/main.rs"
fn main() {}
```
````

### Line highlighting

Highlight specific lines:

````markdown
```python {2,4-5}
import farol

config = farol.load_config()
site = farol.build(config)
site.write()
```
````

## Tables

Standard GFM tables:

```markdown
| Column A | Column B |
|----------|----------|
| value 1  | value 2  |
```

## Links

Internal links use relative paths:

```markdown
[Getting Started](../getting-started/index.md)
```

Farol validates internal links at build time and warns on broken references.
