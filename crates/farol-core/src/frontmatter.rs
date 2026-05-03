use std::path::Path;

use serde::Deserialize;

use crate::error::{FarolError, Result};

/// Parsed frontmatter values - kept permissive for now; stricter schema lives
/// in downstream crates.
pub type Frontmatter = toml::Table;

/// Split a markdown file into `(frontmatter, body)`.
///
/// Recognizes two frontmatter forms:
/// - `---` delimited YAML block
/// - `+++` delimited TOML block
///
/// Files without frontmatter return an empty table and the full body.
pub fn split<'a>(source: &'a str, path: &Path) -> Result<(Frontmatter, &'a str)> {
    if let Some(rest) = source.strip_prefix("---\n") {
        let (raw, body) = split_at_delim(rest, "\n---\n", "\n---", path, "YAML frontmatter")?;
        let parsed: serde_yaml::Value = serde_yaml::from_str(raw).map_err(|e| {
            FarolError::Frontmatter { path: path.to_path_buf(), message: e.to_string() }
        })?;
        let table = yaml_to_toml_table(parsed)
            .map_err(|msg| FarolError::Frontmatter { path: path.to_path_buf(), message: msg })?;
        Ok((table, body))
    } else if let Some(rest) = source.strip_prefix("+++\n") {
        let (raw, body) = split_at_delim(rest, "\n+++\n", "\n+++", path, "TOML frontmatter")?;
        let table: toml::Table = toml::from_str(raw).map_err(|e| FarolError::Frontmatter {
            path: path.to_path_buf(),
            message: e.message().to_string(),
        })?;
        Ok((table, body))
    } else {
        Ok((toml::Table::new(), source))
    }
}

fn split_at_delim<'a>(
    source: &'a str,
    mid_delim: &str,
    end_delim: &str,
    path: &Path,
    label: &str,
) -> Result<(&'a str, &'a str)> {
    if let Some(idx) = source.find(mid_delim) {
        let raw = &source[..idx];
        let body = &source[idx + mid_delim.len()..];
        Ok((raw, body))
    } else if source.ends_with(end_delim) {
        let idx = source.len() - end_delim.len();
        Ok((&source[..idx], ""))
    } else {
        Err(FarolError::Frontmatter {
            path: path.to_path_buf(),
            message: format!("unterminated {label}"),
        })
    }
}

fn yaml_to_toml_table(value: serde_yaml::Value) -> std::result::Result<toml::Table, String> {
    match yaml_to_toml_value(value)? {
        toml::Value::Table(t) => Ok(t),
        _ => Err("frontmatter must be a mapping".to_string()),
    }
}

fn yaml_to_toml_value(value: serde_yaml::Value) -> std::result::Result<toml::Value, String> {
    use serde_yaml::Value as Y;
    Ok(match value {
        Y::Null => toml::Value::String(String::new()),
        Y::Bool(b) => toml::Value::Boolean(b),
        Y::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                return Err("unsupported numeric frontmatter value".into());
            }
        }
        Y::String(s) => toml::Value::String(s),
        Y::Sequence(seq) => toml::Value::Array(
            seq.into_iter().map(yaml_to_toml_value).collect::<std::result::Result<_, _>>()?,
        ),
        Y::Mapping(map) => {
            let mut out = toml::Table::new();
            for (k, v) in map {
                let key = match k {
                    Y::String(s) => s,
                    Y::Bool(b) => b.to_string(),
                    Y::Number(n) => n.to_string(),
                    _ => return Err("frontmatter keys must be strings".into()),
                };
                out.insert(key, yaml_to_toml_value(v)?);
            }
            toml::Value::Table(out)
        }
        Y::Tagged(_) => return Err("tagged YAML values are not supported".into()),
    })
}

/// Convenience: deserialize the frontmatter into a typed struct.
pub fn deserialize<T: for<'de> Deserialize<'de>>(
    fm: &Frontmatter,
) -> std::result::Result<T, toml::de::Error> {
    fm.clone().try_into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn no_frontmatter() {
        let (fm, body) = split("# hi\n", &PathBuf::from("p.md")).unwrap();
        assert!(fm.is_empty());
        assert_eq!(body, "# hi\n");
    }

    #[test]
    fn yaml_block() {
        let input = "---\ntitle: Hello\nweight: 10\n---\n# body\n";
        let (fm, body) = split(input, &PathBuf::from("p.md")).unwrap();
        assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Hello"));
        assert_eq!(fm.get("weight").and_then(|v| v.as_integer()), Some(10));
        assert_eq!(body, "# body\n");
    }

    #[test]
    fn toml_block() {
        let input = "+++\ntitle = \"Hello\"\n+++\n# body\n";
        let (fm, body) = split(input, &PathBuf::from("p.md")).unwrap();
        assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Hello"));
        assert_eq!(body, "# body\n");
    }

    #[test]
    fn unterminated_frontmatter_errors() {
        let input = "---\ntitle: No closer\n";
        assert!(split(input, &PathBuf::from("p.md")).is_err());
    }
}
