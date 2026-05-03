use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use minijinja::context;

use crate::{
    assets,
    config::Config,
    error::{FarolError, Result},
    files::{self, FileKind},
    frontmatter,
    links::{self, BrokenLink},
    markdown,
    page::Page,
    theme, toc,
    url::{output_path_for, site_url_for},
};

/// Outcome of a full build.
#[derive(Debug, Default)]
pub struct BuildReport {
    pub pages: usize,
    pub assets: usize,
    pub broken_links: Vec<BrokenLink>,
}

/// Build a site from `config` into `config.site_dir`.
pub fn build(config: &Config, project_root: &Path) -> Result<BuildReport> {
    let docs_dir = project_root.join(&config.docs_dir);
    let site_dir = project_root.join(&config.site_dir);
    fs::create_dir_all(&site_dir).map_err(|e| FarolError::io(&site_dir, e))?;

    let tree = files::walk(&docs_dir)?;
    let mut pages: Vec<Page> = Vec::new();
    let mut known_pages: HashMap<PathBuf, String> = HashMap::new();

    // First pass: parse each markdown into a Page (body not yet link-resolved).
    for file in tree.files.iter().filter(|f| f.kind == FileKind::Markdown) {
        let source = fs::read_to_string(&file.path).map_err(|e| FarolError::io(&file.path, e))?;
        let (fm, body) = frontmatter::split(&source, &file.path)?;
        let parsed = markdown::parse(body, &file.path)?;

        let title = fm
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or(parsed.title.clone())
            .unwrap_or_else(|| {
                file.relative.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled").to_string()
            });

        let url = site_url_for(&file.relative);
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

    // Second pass: resolve internal markdown links using the full page index.
    let mut broken_links: Vec<BrokenLink> = Vec::new();
    for page in pages.iter_mut() {
        let (rewrites, mut broken) =
            links::resolve_in_html(&page.relative, &page.body_html, &known_pages);
        page.body_html = links::apply_rewrites(&page.body_html, &rewrites);
        broken_links.append(&mut broken);
    }

    for b in &broken_links {
        tracing::warn!(page = %b.page.display(), href = %b.href, reason = b.reason, "broken link");
    }

    // Third pass: render every page through the default template.
    let overrides = project_root.join("overrides");
    let env = theme::build_env(Some(&overrides))?;
    let tmpl = env.get_template("default.html").map_err(|e| FarolError::ConfigInvalid {
        message: format!("failed to load default template: {e}"),
    })?;

    for page in &pages {
        let out = tmpl.render(context! { page => page, config => config }).map_err(|e| {
            FarolError::ConfigInvalid {
                message: format!("render error in {}: {e}", page.relative.display()),
            }
        })?;
        let dest = site_dir.join(&page.output);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| FarolError::io(parent, e))?;
        }
        fs::write(&dest, out).map_err(|e| FarolError::io(&dest, e))?;
    }

    // Copy theme assets and user assets.
    theme::copy_assets(&site_dir)?;
    let mut asset_count = 0;
    for file in tree.files.iter().filter(|f| f.kind == FileKind::Asset) {
        assets::copy_asset(&file.path, &file.relative, &site_dir, false)?;
        asset_count += 1;
    }

    // Extras: sitemap + robots.
    write_sitemap(&site_dir, &pages, config)?;
    write_robots(&site_dir, config)?;

    Ok(BuildReport { pages: pages.len(), assets: asset_count, broken_links })
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

        let install = fs::read_to_string(root.join("site/guide/install/index.html")).unwrap();
        assert!(install.contains("Install"));
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
}
