use farol_core::theme::manifest::{parse, validate_version};

#[test]
fn parses_valid_manifest() {
    let toml = r#"
[theme]
name = "test"
version = "0.1.0"
min_farol_version = "0.0.3"

[theme.layouts]
supported = ["default", "landing"]

[theme.assets]
shared_js = true
css = ["base.css"]
"#;
    let m = parse(toml).unwrap();
    assert_eq!(m.theme.name, "test");
    assert_eq!(m.theme.layouts.supported, vec!["default", "landing"]);
    assert!(m.theme.assets.shared_js);
}

#[test]
fn version_check() {
    let toml = r#"
[theme]
name = "t"
version = "0.1.0"
min_farol_version = "0.0.3"

[theme.layouts]
supported = ["default"]
"#;
    let m = parse(toml).unwrap();
    assert!(validate_version(&m, "0.0.3").is_ok());
    assert!(validate_version(&m, "0.1.0").is_ok());
    assert!(validate_version(&m, "0.0.2").is_err());
}
