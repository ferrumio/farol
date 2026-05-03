use serde::Serialize;

/// A heading entry in the table of contents.
#[derive(Debug, Clone, Serialize)]
pub struct TocEntry {
    pub level: u8,
    pub title: String,
    pub slug: String,
    pub children: Vec<TocEntry>,
}

/// Build a nested TOC from a flat list of `(level, title, slug)` tuples.
/// Default includes levels 2 and 3; adjust with `max_level`.
pub fn build(flat: &[(u8, String, String)], max_level: u8) -> Vec<TocEntry> {
    let filtered: Vec<TocEntry> = flat
        .iter()
        .filter(|(lvl, _, _)| *lvl >= 2 && *lvl <= max_level)
        .map(|(lvl, title, slug)| TocEntry {
            level: *lvl,
            title: title.clone(),
            slug: slug.clone(),
            children: Vec::new(),
        })
        .collect();

    nest(&filtered)
}

fn nest(entries: &[TocEntry]) -> Vec<TocEntry> {
    let mut stack: Vec<TocEntry> = Vec::new();
    let mut roots: Vec<TocEntry> = Vec::new();

    for entry in entries {
        while let Some(top) = stack.last() {
            if top.level < entry.level {
                break;
            }
            let finished = stack.pop().unwrap();
            attach(&mut stack, &mut roots, finished);
        }
        stack.push(entry.clone());
    }
    while let Some(finished) = stack.pop() {
        attach(&mut stack, &mut roots, finished);
    }
    roots
}

fn attach(stack: &mut [TocEntry], roots: &mut Vec<TocEntry>, entry: TocEntry) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(entry);
    } else {
        roots.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(level: u8, title: &str) -> (u8, String, String) {
        (level, title.to_string(), title.to_ascii_lowercase().replace(' ', "-"))
    }

    #[test]
    fn flat_h2() {
        let input = vec![h(2, "A"), h(2, "B")];
        let toc = build(&input, 3);
        assert_eq!(toc.len(), 2);
        assert!(toc[0].children.is_empty());
    }

    #[test]
    fn h3_nests_under_h2() {
        let input = vec![h(2, "A"), h(3, "A1"), h(3, "A2"), h(2, "B")];
        let toc = build(&input, 3);
        assert_eq!(toc.len(), 2);
        assert_eq!(toc[0].children.len(), 2);
        assert_eq!(toc[0].children[0].title, "A1");
    }

    #[test]
    fn h1_is_excluded() {
        let input = vec![h(1, "Title"), h(2, "A")];
        let toc = build(&input, 3);
        assert_eq!(toc.len(), 1);
        assert_eq!(toc[0].title, "A");
    }

    #[test]
    fn respects_max_level() {
        let input = vec![h(2, "A"), h(3, "A1"), h(4, "A1a")];
        let toc = build(&input, 3);
        assert_eq!(toc[0].children.len(), 1);
        assert!(toc[0].children[0].children.is_empty());
    }
}
