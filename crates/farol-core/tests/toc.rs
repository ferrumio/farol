use farol_core::toc;

fn h(level: u8, title: &str) -> (u8, String, String) {
    (level, title.to_string(), title.to_ascii_lowercase().replace(' ', "-"))
}

#[test]
fn flat_h2() {
    let input = vec![h(2, "A"), h(2, "B")];
    let toc = toc::build(&input, 3);
    assert_eq!(toc.len(), 2);
    assert!(toc[0].children.is_empty());
}

#[test]
fn h3_nests_under_h2() {
    let input = vec![h(2, "A"), h(3, "A1"), h(3, "A2"), h(2, "B")];
    let toc = toc::build(&input, 3);
    assert_eq!(toc.len(), 2);
    assert_eq!(toc[0].children.len(), 2);
    assert_eq!(toc[0].children[0].title, "A1");
}

#[test]
fn h1_is_excluded() {
    let input = vec![h(1, "Title"), h(2, "A")];
    let toc = toc::build(&input, 3);
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "A");
}

#[test]
fn respects_max_level() {
    let input = vec![h(2, "A"), h(3, "A1"), h(4, "A1a")];
    let toc = toc::build(&input, 3);
    assert_eq!(toc[0].children.len(), 1);
    assert!(toc[0].children[0].children.is_empty());
}
