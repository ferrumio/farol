//! Build-time image pipeline.
//!
//! For each image asset we:
//! - copy the original with a content-hashed filename
//! - re-encode a WebP alternative
//! - generate a 20px blur LQIP as a base64 data-URI
//! - record dimensions
//!
//! The HTML pass then rewrites `<img src="./foo.png">` into a `<picture>`
//! element with `srcset`, explicit `width`/`height`, and a `style=
//! background-image:url(lqip)` so the layout doesn't shift and the image
//! fades in.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use base64::Engine;
use image::{imageops::FilterType, ImageReader};
use sha2::{Digest, Sha256};

use crate::error::{FarolError, Result};

/// Everything we know about a processed image.
#[derive(Debug, Clone)]
pub struct ImageMeta {
    /// Public URL of the original-format hashed copy.
    pub original_url: String,
    /// Public URL of the WebP alternative (same width, re-encoded).
    pub webp_url: Option<String>,
    /// `data:image/webp;base64,...` tiny placeholder (< 1 KB typically).
    pub lqip: String,
    pub width: u32,
    pub height: u32,
    pub mime: &'static str,
}

/// An index from the *docs-relative* asset path to its metadata.
pub type ImageIndex = HashMap<PathBuf, ImageMeta>;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
        .unwrap_or(false)
}

pub fn process(src: &Path, relative: &Path, site_dir: &Path) -> Result<ImageMeta> {
    let bytes = fs::read(src).map_err(|e| FarolError::io(src, e))?;
    let ext = relative
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let mime = mime_for(&ext);
    let hash = short_hash(&bytes);

    let stem = relative.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));

    let hashed_name = format!("{stem}.{hash}.{ext}");
    let original_rel = parent.join(&hashed_name);
    let original_dest = site_dir.join(&original_rel);
    if let Some(p) = original_dest.parent() {
        fs::create_dir_all(p).map_err(|e| FarolError::io(p, e))?;
    }
    fs::write(&original_dest, &bytes).map_err(|e| FarolError::io(&original_dest, e))?;

    // Decode to inspect dimensions and produce derivatives.
    let img = match ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format() {
        Ok(r) => match r.decode() {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(path = %src.display(), error = %e, "failed to decode image; skipping derivatives");
                return Ok(ImageMeta {
                    original_url: format!("/{}", path_to_url(&original_rel)),
                    webp_url: None,
                    lqip: String::new(),
                    width: 0,
                    height: 0,
                    mime,
                });
            }
        },
        Err(_) => {
            return Ok(ImageMeta {
                original_url: format!("/{}", path_to_url(&original_rel)),
                webp_url: None,
                lqip: String::new(),
                width: 0,
                height: 0,
                mime,
            });
        }
    };

    let width = img.width();
    let height = img.height();

    // WebP alternative (skip if already webp).
    let webp_url = if ext == "webp" {
        None
    } else {
        let webp_name = format!("{stem}.{hash}.webp");
        let webp_rel = parent.join(&webp_name);
        let webp_dest = site_dir.join(&webp_rel);
        match image::codecs::webp::WebPEncoder::new_lossless(
            fs::File::create(&webp_dest).map_err(|e| FarolError::io(&webp_dest, e))?,
        )
        .encode(img.to_rgba8().as_raw(), width, height, image::ExtendedColorType::Rgba8)
        {
            Ok(()) => Some(format!("/{}", path_to_url(&webp_rel))),
            Err(e) => {
                let _ = fs::remove_file(&webp_dest);
                tracing::warn!(error = %e, "webp encode failed");
                None
            }
        }
    };

    let lqip = build_lqip(&img);

    Ok(ImageMeta {
        original_url: format!("/{}", path_to_url(&original_rel)),
        webp_url,
        lqip,
        width,
        height,
        mime,
    })
}

fn build_lqip(img: &image::DynamicImage) -> String {
    let thumb = img.resize(20, 20, FilterType::Triangle);
    let mut buf = Vec::new();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
    if encoder
        .encode(
            thumb.to_rgba8().as_raw(),
            thumb.width(),
            thumb.height(),
            image::ExtendedColorType::Rgba8,
        )
        .is_err()
    {
        return String::new();
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
    format!("data:image/webp;base64,{b64}")
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    }
}

fn short_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(8);
    for b in &digest[..4] {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn path_to_url(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Rewrite every `<img src="...">` to a `<picture>` block using the index.
///
/// `page_relative` is the docs-relative path of the page containing the
/// HTML (used to resolve relative image paths).
pub fn rewrite_images(html: &str, page_relative: &Path, index: &ImageIndex) -> String {
    if index.is_empty() {
        return html.to_string();
    }

    let mut out = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(rel_start) = html[cursor..].find("<img ") {
        let start = cursor + rel_start;
        out.push_str(&html[cursor..start]);
        let close = match html[start..].find('>') {
            Some(i) => start + i + 1,
            None => {
                out.push_str(&html[start..]);
                return out;
            }
        };
        let tag = &html[start..close];

        // Extract src value.
        let src = match extract_attr(tag, "src") {
            Some(s) => s,
            None => {
                out.push_str(tag);
                cursor = close;
                continue;
            }
        };

        // Only rewrite if we can resolve to a known image.
        let meta = resolve_image(page_relative, &src, index);
        match meta {
            Some(m) => {
                let alt = extract_attr(tag, "alt").unwrap_or_default();
                out.push_str(&render_picture(m, &alt));
            }
            None => out.push_str(tag),
        }
        cursor = close;
    }
    out.push_str(&html[cursor..]);
    out
}

fn resolve_image<'a>(
    page_relative: &Path,
    src: &str,
    index: &'a ImageIndex,
) -> Option<&'a ImageMeta> {
    if src.contains("://") || src.starts_with('/') {
        return None;
    }
    let base = page_relative.parent().unwrap_or_else(|| Path::new(""));
    let joined = base.join(src);
    let normalized = normalize(&joined);
    index.get(&normalized)
}

fn normalize(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let needle_dq = format!(r#"{name}=""#);
    if let Some(i) = tag.find(&needle_dq) {
        let start = i + needle_dq.len();
        let end = tag[start..].find('"')? + start;
        return Some(tag[start..end].to_string());
    }
    let needle_sq = format!(r#"{name}='"#);
    if let Some(i) = tag.find(&needle_sq) {
        let start = i + needle_sq.len();
        let end = tag[start..].find('\'')? + start;
        return Some(tag[start..end].to_string());
    }
    None
}

fn render_picture(meta: &ImageMeta, alt: &str) -> String {
    let mut out = String::new();
    out.push_str(r#"<picture>"#);
    if let Some(webp) = &meta.webp_url {
        out.push_str(&format!(r#"<source srcset="{webp}" type="image/webp">"#));
    }
    let style = if !meta.lqip.is_empty() {
        format!(r#" style="background-image:url({});background-size:cover""#, meta.lqip)
    } else {
        String::new()
    };
    let dims = if meta.width > 0 && meta.height > 0 {
        format!(r#" width="{}" height="{}""#, meta.width, meta.height)
    } else {
        String::new()
    };
    out.push_str(&format!(
        r#"<img src="{}" alt="{}" loading="lazy" decoding="async"{}{}></picture>"#,
        meta.original_url,
        escape_attr(alt),
        dims,
        style,
    ));
    out
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn meta() -> ImageMeta {
        ImageMeta {
            original_url: "/img/logo.abcd.png".into(),
            webp_url: Some("/img/logo.abcd.webp".into()),
            lqip: "data:image/webp;base64,ZZ".into(),
            width: 200,
            height: 100,
            mime: "image/png",
        }
    }

    #[test]
    fn rewrites_img_with_picture() {
        let mut idx = HashMap::new();
        idx.insert(PathBuf::from("img/logo.png"), meta());
        let html = r#"<p><img src="./img/logo.png" alt="Logo"></p>"#;
        let out = rewrite_images(html, Path::new("index.md"), &idx);
        assert!(out.contains("<picture>"));
        assert!(out.contains("/img/logo.abcd.webp"));
        assert!(out.contains(r#"width="200""#));
        assert!(out.contains(r#"alt="Logo""#));
    }

    #[test]
    fn leaves_absolute_urls_alone() {
        let idx = HashMap::new();
        let html = r#"<img src="https://example.com/x.png">"#;
        assert_eq!(rewrite_images(html, Path::new("index.md"), &idx), html);
    }

    #[test]
    fn leaves_unknown_local_paths_alone() {
        let idx: ImageIndex = HashMap::new();
        let html = r#"<img src="./missing.png">"#;
        assert_eq!(rewrite_images(html, Path::new("index.md"), &idx), html);
    }
}
