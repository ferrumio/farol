use std::path::PathBuf;

use farol_core::frontmatter;

#[test]
fn no_frontmatter() {
    let (fm, body) = frontmatter::split("# hi\n", &PathBuf::from("p.md")).unwrap();
    assert!(fm.is_empty());
    assert_eq!(body, "# hi\n");
}

#[test]
fn yaml_block() {
    let input = "---\ntitle: Hello\nweight: 10\n---\n# body\n";
    let (fm, body) = frontmatter::split(input, &PathBuf::from("p.md")).unwrap();
    assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Hello"));
    assert_eq!(fm.get("weight").and_then(|v| v.as_integer()), Some(10));
    assert_eq!(body, "# body\n");
}

#[test]
fn toml_block() {
    let input = "+++\ntitle = \"Hello\"\n+++\n# body\n";
    let (fm, body) = frontmatter::split(input, &PathBuf::from("p.md")).unwrap();
    assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Hello"));
    assert_eq!(body, "# body\n");
}

#[test]
fn unterminated_frontmatter_errors() {
    let input = "---\ntitle: No closer\n";
    assert!(frontmatter::split(input, &PathBuf::from("p.md")).is_err());
}
