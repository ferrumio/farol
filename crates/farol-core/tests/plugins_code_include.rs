use farol_core::plugins::core::code_include::CodeBlockAttrs;

#[test]
fn parses_bare_lang() {
    let a = CodeBlockAttrs::parse("python");
    assert_eq!(a.lang, "python");
    assert!(a.file.is_none());
}

#[test]
fn parses_file_and_title() {
    let a = CodeBlockAttrs::parse(r#"python file="./examples/x.py" title="my example""#);
    assert_eq!(a.lang, "python");
    assert_eq!(a.file.as_deref(), Some("./examples/x.py"));
    assert_eq!(a.title.as_deref(), Some("my example"));
}

#[test]
fn parses_lines() {
    let a = CodeBlockAttrs::parse(r#"py lines="10-25,40""#);
    assert_eq!(a.lines, Some(vec![(10, 25), (40, 40)]));
}

#[test]
fn parses_linenums_start() {
    let a = CodeBlockAttrs::parse(r#"py linenums="start=42""#);
    assert!(a.linenums);
    assert_eq!(a.linenums_start, 42);
}

#[test]
fn effective_title_auto_from_file() {
    let a = CodeBlockAttrs::parse(r#"py file="./examples/foo.py""#);
    assert_eq!(a.effective_title().as_deref(), Some("foo.py"));
}

#[test]
fn hl_lines_parsed() {
    let a = CodeBlockAttrs::parse(r#"py hl_lines="1 3-5""#);
    assert_eq!(a.hl_lines, vec![1, 3, 4, 5]);
}
