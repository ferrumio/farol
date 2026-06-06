//! Generates meta-refresh HTML for declared path redirects.
//!
//! Looks for `redirects.toml` at the project root:
//!
//! ```toml
//! [redirects]
//! "/old/path/" = "/new/path/"
//! "/legacy/" = "https://example.com/new"
//! ```
//!
//! For each entry, writes `<old>/index.html` with a
//! `<meta http-equiv="refresh">` tag pointing at the target plus a canonical
//! link and a visible fallback message. Targets may be relative or
//! absolute URLs.

use std::{collections::BTreeMap, path::Path};

use serde::Deserialize;

use crate::{Config, FarolError, PluginHost, Result};

pub struct RedirectsPlugin;

#[derive(Debug, Deserialize)]
struct RedirectsFile {
    #[serde(default)]
    redirects: BTreeMap<String, String>,
}

impl PluginHost for RedirectsPlugin {
    fn name(&self) -> &str {
        "redirects"
    }

    fn plugins(&self) -> Vec<String> {
        vec!["redirects".into()]
    }

    fn on_post_build(&self, site_dir: &Path, _config: &Config) -> Result<()> {
        let project_root = site_dir.parent().unwrap_or_else(|| Path::new("."));
        let redirects_path = project_root.join("redirects.toml");
        if !redirects_path.exists() {
            return Ok(());
        }

        let text = std::fs::read_to_string(&redirects_path)
            .map_err(|e| FarolError::io(&redirects_path, e))?;
        let parsed: RedirectsFile = toml::from_str(&text).map_err(|e| {
            FarolError::ConfigInvalid { message: format!("invalid redirects.toml: {e}") }
        })?;

        for (from, to) in &parsed.redirects {
            write_redirect(site_dir, from, to)?;
        }
        Ok(())
    }
}

fn write_redirect(site_dir: &Path, from: &str, to: &str) -> Result<()> {
    // Normalize the "from" path into a directory under site_dir.
    let rel = from.trim_start_matches('/').trim_end_matches('/');
    let target_dir = if rel.is_empty() { site_dir.to_path_buf() } else { site_dir.join(rel) };
    std::fs::create_dir_all(&target_dir).map_err(|e| FarolError::io(&target_dir, e))?;

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Redirecting...</title>
  <link rel="canonical" href="{to}">
  <meta http-equiv="refresh" content="0; url={to}">
  <meta name="robots" content="noindex">
</head>
<body>
  <p>This page has moved. If you are not redirected, <a href="{to}">click here</a>.</p>
</body>
</html>
"#,
        to = escape_attr(to),
    );

    let dest = target_dir.join("index.html");
    std::fs::write(&dest, html).map_err(|e| FarolError::io(&dest, e))
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}
