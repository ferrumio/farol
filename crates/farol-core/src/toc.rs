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
