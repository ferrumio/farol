---
title: Deploy
---

# Deploy

## GitHub Pages

Farol includes a GitHub Actions workflow for automatic deployment to GitHub Pages on every push to `main`.

### Setup

1. Go to your repository **Settings → Pages**
2. Under "Build and deployment", select **GitHub Actions** as the source
3. Add the workflow file:

```yaml title=".github/workflows/docs.yml"
name: docs

on:
  push:
    branches: [main]
    paths:
      - "docs/**"
      - "farol.toml"
      - ".github/workflows/docs.yml"
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v6
        with:
          python-version: "3.12"
      - run: pip install farol
      - run: farol build
      - uses: actions/upload-pages-artifact@v3
        with:
          path: site

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

That's it. Every push to `main` that changes docs will rebuild and deploy automatically.

## Netlify

```bash
# Build command
farol build

# Publish directory
site
```

## Any static host

`farol build` outputs plain static files to `site/`. Upload that directory to any host: Vercel, Cloudflare Pages, S3, or your own server.
