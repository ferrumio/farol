use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use minijinja::{context, Environment};

use crate::{
    assets,
    cache::Cache,
    config::Config,
    error::{FarolError, Result},
    files::{self, FileKind},
    frontmatter,
    graph::{Graph, Node, Report as GraphReport},
    hash::{Hash, Hasher},
    links::{self, BrokenLink},
    markdown,
    page::Page,
    plugin::{NoOpHost, PluginHost},
    theme, toc,
    url::{output_path_for, site_url_for},
};

/// Options controlling a build invocation.
#[derive(Debug, Default, Clone)]
pub struct BuildOptions {
    /// Collect per-node timing and emit a summary via `BuildReport::graph`.
    pub timings: bool,
    /// Override cache location. `None` = `<project_root>/.farol/cache.redb`.
    pub cache_path: Option<PathBuf>,
    /// Skip cache entirely (useful for CI without persistent disks).
    pub no_cache: bool,
}

/// Outcome of a full build.
#[derive(Debug)]
pub struct BuildReport {
    pub pages: usize,
    pub assets: usize,
    pub broken_links: Vec<BrokenLink>,
    pub graph: Option<GraphReport>,
}

/// Build a site from `config` into `config.site_dir`. Short-form helper used by
/// tests and the default CLI path.
pub fn build(config: &Config, project_root: &Path) -> Result<BuildReport> {
    build_with(config, project_root, &BuildOptions::default(), &NoOpHost)
}

/// Build a site, with explicit options and a plugin host.
pub fn build_with(
    config: &Config,
    project_root: &Path,
    opts: &BuildOptions,
    host: &dyn PluginHost,
) -> Result<BuildReport> {
    // Plugins get first crack at the config.
    let config = host.on_config(config.clone())?;
    let config = &config;

    let docs_dir = project_root.join(&config.docs_dir);
    let site_dir = project_root.join(&config.site_dir);
    fs::create_dir_all(&site_dir).map_err(|e| FarolError::io(&site_dir, e))?;

    // --- pre-graph: walk and parse -----------------------------------------
    let tree = files::walk(&docs_dir)?;
    let tree = host.on_files(tree, config)?;
    let mut pages: Vec<Page> = Vec::new();
    let mut known_pages: HashMap<PathBuf, String> = HashMap::new();

    for file in tree.files.iter().filter(|f| f.kind == FileKind::Markdown) {
        let source = fs::read_to_string(&file.path).map_err(|e| FarolError::io(&file.path, e))?;
        let (fm, body) = frontmatter::split(&source, &file.path)?;

        // Build a placeholder page so plugins have metadata at on_page_markdown time.
        let url = site_url_for(&file.relative);
        let title_guess =
            fm.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| {
                file.relative.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled").to_string()
            });
        let placeholder = Page {
            relative: file.relative.clone(),
            url: url.clone(),
            output: output_path_for(&url),
            title: title_guess,
            frontmatter: fm.clone(),
            body_html: String::new(),
            toc: Vec::new(),
        };

        let body = host.on_page_markdown(body.to_string(), &placeholder, config)?;
        let parsed = markdown::parse(&body, &file.path)?;

        let title = fm
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(parsed.title.clone())
            .unwrap_or_else(|| {
                file.relative.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled").to_string()
            });

        known_pages.insert(file.relative.clone(), url.clone());

        let toc_tree = toc::build(&parsed.headings, 3);

        pages.push(Page {
            relative: file.relative.clone(),
            url,
            output: output_path_for(&site_url_for(&file.relative)),
            title,
            frontmatter: fm,
            body_html: parsed.html,
            toc: toc_tree,
        });
    }

    host.on_nav(&pages, config)?;

    // Resolve internal links before hashing: this ensures cache entries are
    // invalidated when sibling pages are renamed or added.
    let mut broken_links: Vec<BrokenLink> = Vec::new();
    for page in pages.iter_mut() {
        let (rewrites, mut broken) =
            links::resolve_in_html(&page.relative, &page.body_html, &known_pages);
        page.body_html = links::apply_rewrites(&page.body_html, &rewrites);
        broken_links.append(&mut broken);
    }

    // Plugins see resolved HTML and may mutate it.
    for page in pages.iter_mut() {
        let html = std::mem::take(&mut page.body_html);
        page.body_html = host.on_page_html(html, page, config)?;
    }

    for b in &broken_links {
        tracing::warn!(page = %b.page.display(), href = %b.href, reason = b.reason, "broken link");
    }

    // --- graph: render + write per page -----------------------------------
    let overrides = project_root.join("overrides");
    let env = theme::build_env(Some(&overrides))?;
    let env = Arc::new(env);

    // Summary used in the input hash so theme/config changes invalidate cache.
    let theme_summary = theme_summary_bytes(config);
    let nav_summary = nav_summary_bytes(&pages);

    let cache = if opts.no_cache {
        None
    } else {
        let path = opts
            .cache_path
            .clone()
            .unwrap_or_else(|| project_root.join(".farol").join("cache.redb"));
        Some(Cache::open(&path)?)
    };

    let mut graph = Graph::new();
    for page in pages.iter().cloned() {
        graph.push(RenderPageNode {
            page,
            site_dir: site_dir.clone(),
            env: env.clone(),
            config: config.clone(),
            theme_summary: theme_summary.clone(),
            nav_summary: nav_summary.clone(),
        });
    }

    let graph_report = graph.execute(cache.as_ref())?;

    // --- post-graph: assets, sitemap, robots (cheap; always regenerated) ---
    theme::copy_assets(&site_dir)?;
    let mut asset_count = 0;
    for file in tree.files.iter().filter(|f| f.kind == FileKind::Asset) {
        assets::copy_asset(&file.path, &file.relative, &site_dir, false)?;
        asset_count += 1;
    }

    write_sitemap(&site_dir, &pages, config)?;
    write_robots(&site_dir, config)?;

    host.on_post_build(&site_dir, config)?;

    Ok(BuildReport {
        pages: pages.len(),
        assets: asset_count,
        broken_links,
        graph: if opts.timings { Some(graph_report) } else { None },
    })
}

/// Node that renders a single page and writes it to disk.
struct RenderPageNode {
    page: Page,
    site_dir: PathBuf,
    env: Arc<Environment<'static>>,
    config: Config,
    theme_summary: Vec<u8>,
    nav_summary: Vec<u8>,
}

impl RenderPageNode {
    fn render_html(&self) -> Result<String> {
        let tmpl = self.env.get_template("default.html").map_err(|e| FarolError::Cache {
            message: format!("failed to load default template: {e}"),
        })?;
        tmpl.render(context! { page => self.page, config => self.config }).map_err(|e| {
            FarolError::Cache {
                message: format!("render error in {}: {e}", self.page.relative.display()),
            }
        })
    }

    fn write_html(&self, html: &str) -> Result<()> {
        let dest = self.site_dir.join(&self.page.output);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| FarolError::io(parent, e))?;
        }
        fs::write(&dest, html).map_err(|e| FarolError::io(&dest, e))
    }
}

impl Node for RenderPageNode {
    fn id(&self) -> &str {
        // `/guide/install/` - stable per-URL id regardless of docs_dir rename.
        &self.page.url
    }

    fn input_hash(&self) -> Hash {
        Hasher::new()
            .tag("render-page")
            .update(self.page.url.as_bytes())
            .update(self.page.title.as_bytes())
            .update(self.page.body_html.as_bytes())
            // TOC captured by body_html already, since heading changes flow through markdown output.
            .update(&self.theme_summary)
            .update(&self.nav_summary)
            .finish()
    }

    fn execute(&self) -> Result<Vec<u8>> {
        let html = self.render_html()?;
        self.write_html(&html)?;
        Ok(html.into_bytes())
    }

    fn restore(&self, cached: &[u8]) -> Result<()> {
        let html = std::str::from_utf8(cached).map_err(|e| FarolError::Cache {
            message: format!("invalid cached html for {}: {e}", self.page.url),
        })?;
        self.write_html(html)
    }
}

fn theme_summary_bytes(config: &Config) -> Vec<u8> {
    Hasher::new()
        .tag("theme")
        .update(config.site_name.as_bytes())
        .update(config.site_url.as_deref().unwrap_or("").as_bytes())
        .update(config.theme.name.as_bytes())
        .update(config.theme.palette.as_deref().unwrap_or("").as_bytes())
        .update(config.theme.primary.as_deref().unwrap_or("").as_bytes())
        .update(config.theme.accent.as_deref().unwrap_or("").as_bytes())
        .finish()
        .as_bytes()
        .to_vec()
}

fn nav_summary_bytes(pages: &[Page]) -> Vec<u8> {
    let mut pairs: Vec<(&str, &str)> =
        pages.iter().map(|p| (p.url.as_str(), p.title.as_str())).collect();
    pairs.sort();
    let mut h = Hasher::new().tag("nav");
    for (url, title) in pairs {
        h = h.update(url.as_bytes()).update(b"|").update(title.as_bytes()).update(b"\n");
    }
    h.finish().as_bytes().to_vec()
}

fn write_sitemap(site_dir: &Path, pages: &[Page], config: &Config) -> Result<()> {
    let base = config.site_url.clone().unwrap_or_default();
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for page in pages {
        xml.push_str("  <url><loc>");
        xml.push_str(&format!("{}{}", base.trim_end_matches('/'), page.url));
        xml.push_str("</loc></url>\n");
    }
    xml.push_str("</urlset>\n");
    let dest = site_dir.join("sitemap.xml");
    fs::write(&dest, xml).map_err(|e| FarolError::io(&dest, e))
}

fn write_robots(site_dir: &Path, config: &Config) -> Result<()> {
    let mut text = String::from("User-agent: *\nAllow: /\n");
    if let Some(url) = &config.site_url {
        text.push_str(&format!("Sitemap: {}/sitemap.xml\n", url.trim_end_matches('/')));
    }
    let dest = site_dir.join("robots.txt");
    fs::write(&dest, text).map_err(|e| FarolError::io(&dest, e))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn write(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    #[test]
    fn builds_minimal_site() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let docs = root.join("docs");
        write(&docs, "index.md", "# Home\n\n[guide](./guide/install.md).\n");
        write(&docs, "guide/install.md", "---\ntitle: Install\n---\n# Install\n\nstuff.\n");

        let cfg = Config { site_url: Some("https://example.com".into()), ..Config::default() };
        let report = build(&cfg, root).unwrap();

        assert_eq!(report.pages, 2);
        assert!(report.broken_links.is_empty());
        assert!(root.join("site/index.html").exists());
        assert!(root.join("site/guide/install/index.html").exists());
        assert!(root.join("site/sitemap.xml").exists());
        assert!(root.join("site/robots.txt").exists());
        assert!(root.join("site/assets/base.css").exists());

        let home = fs::read_to_string(root.join("site/index.html")).unwrap();
        assert!(home.contains("Home"));
        assert!(home.contains(r#"href="/guide/install/""#));
    }

    #[test]
    fn reports_broken_link() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let docs = root.join("docs");
        write(&docs, "index.md", "# Home\n\n[missing](./nope.md)\n");

        let cfg = Config::default();
        let report = build(&cfg, root).unwrap();
        assert_eq!(report.broken_links.len(), 1);
    }

    #[test]
    fn warm_rebuild_hits_cache() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let docs = root.join("docs");
        write(&docs, "index.md", "# Home\n");
        write(&docs, "a.md", "# A\n");

        let cfg = Config::default();
        let opts = BuildOptions { timings: true, ..BuildOptions::default() };

        let r1 = build_with(&cfg, root, &opts, &NoOpHost).unwrap();
        assert_eq!(r1.graph.as_ref().unwrap().cache_misses, 2);
        assert_eq!(r1.graph.as_ref().unwrap().cache_hits, 0);

        let r2 = build_with(&cfg, root, &opts, &NoOpHost).unwrap();
        assert_eq!(r2.graph.as_ref().unwrap().cache_hits, 2);
        assert_eq!(r2.graph.as_ref().unwrap().cache_misses, 0);
    }

    #[test]
    fn plugin_can_rewrite_markdown() {
        use crate::plugin::PluginHost;

        struct WaveHost;
        impl PluginHost for WaveHost {
            fn on_page_markdown(
                &self,
                markdown: String,
                _page: &Page,
                _config: &Config,
            ) -> Result<String> {
                Ok(markdown.replace(":wave:", "👋"))
            }
        }

        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let docs = root.join("docs");
        write(&docs, "index.md", "# Hi :wave:\n");

        let cfg = Config::default();
        let opts = BuildOptions { no_cache: true, ..BuildOptions::default() };
        build_with(&cfg, root, &opts, &WaveHost).unwrap();

        let html = fs::read_to_string(root.join("site/index.html")).unwrap();
        assert!(html.contains("👋"), "plugin replacement missing from output");
        assert!(!html.contains(":wave:"), "raw token leaked");
    }

    #[test]
    fn plugin_can_rewrite_html() {
        use crate::plugin::PluginHost;

        struct AttrHost;
        impl PluginHost for AttrHost {
            fn on_page_html(&self, html: String, _page: &Page, _config: &Config) -> Result<String> {
                Ok(html.replace("<p>", "<p data-plugin=\"attr\">"))
            }
        }

        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let docs = root.join("docs");
        write(&docs, "index.md", "# Hi\n\nparagraph.\n");

        let cfg = Config::default();
        let opts = BuildOptions { no_cache: true, ..BuildOptions::default() };
        build_with(&cfg, root, &opts, &AttrHost).unwrap();

        let html = fs::read_to_string(root.join("site/index.html")).unwrap();
        assert!(html.contains("data-plugin=\"attr\""));
    }

    #[test]
    fn plugin_error_propagates() {
        use crate::plugin::PluginHost;

        struct Fails;
        impl PluginHost for Fails {
            fn on_page_markdown(
                &self,
                _markdown: String,
                _page: &Page,
                _config: &Config,
            ) -> Result<String> {
                Err(FarolError::ConfigInvalid { message: "plugin said no".into() })
            }
        }

        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let docs = root.join("docs");
        write(&docs, "index.md", "# hi\n");

        let cfg = Config::default();
        let opts = BuildOptions { no_cache: true, ..BuildOptions::default() };
        let err = build_with(&cfg, root, &opts, &Fails).unwrap_err();
        match err {
            FarolError::ConfigInvalid { message } => assert!(message.contains("plugin said no")),
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn edited_page_invalidates_cache() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let docs = root.join("docs");
        write(&docs, "index.md", "# Home\n");
        write(&docs, "a.md", "# A\n");

        let cfg = Config::default();
        let opts = BuildOptions { timings: true, ..BuildOptions::default() };

        build_with(&cfg, root, &opts, &NoOpHost).unwrap();

        // Edit one file.
        write(&docs, "a.md", "# A\n\nedited.\n");
        let r = build_with(&cfg, root, &opts, &NoOpHost).unwrap();
        let g = r.graph.unwrap();
        // `a.md` rebuilds; `index.md` should still hit (nav title of A unchanged).
        assert_eq!(g.cache_misses, 1);
        assert_eq!(g.cache_hits, 1);
    }

    #[test]
    fn title_change_invalidates_dependents() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let docs = root.join("docs");
        write(&docs, "index.md", "# Home\n");
        write(&docs, "a.md", "# A\n");

        let cfg = Config::default();
        let opts = BuildOptions { timings: true, ..BuildOptions::default() };

        build_with(&cfg, root, &opts, &NoOpHost).unwrap();

        // Change a page's title - nav summary changes so every page invalidates.
        write(&docs, "a.md", "# A (renamed)\n");
        let r = build_with(&cfg, root, &opts, &NoOpHost).unwrap();
        let g = r.graph.unwrap();
        assert_eq!(g.cache_misses, 2);
        assert_eq!(g.cache_hits, 0);
    }

    #[test]
    fn theme_change_invalidates_all() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let docs = root.join("docs");
        write(&docs, "a.md", "# A\n");
        write(&docs, "b.md", "# B\n");

        let opts = BuildOptions { timings: true, ..BuildOptions::default() };

        let cfg1 = Config { site_name: "v1".into(), ..Config::default() };
        build_with(&cfg1, root, &opts, &NoOpHost).unwrap();

        let cfg2 = Config { site_name: "v2".into(), ..Config::default() };
        let r = build_with(&cfg2, root, &opts, &NoOpHost).unwrap();
        assert_eq!(r.graph.unwrap().cache_misses, 2);
    }
}
