/// GitHub-compatible slug generation for headings.
///
/// Lowercases, strips everything except alphanumerics / spaces / hyphens, and
/// replaces runs of whitespace with a single hyphen.
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_sep = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            last_was_sep = false;
        } else if ch == '-' || ch == '_' {
            out.push(ch);
            last_was_sep = false;
        } else if ch.is_whitespace() && !last_was_sep && !out.is_empty() {
            out.push('-');
            last_was_sep = true;
        }
        // everything else is dropped
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Return a slug unique within `seen`, appending `-1`, `-2`, ... as needed.
pub fn unique_slug(text: &str, seen: &mut std::collections::HashSet<String>) -> String {
    let base = slugify(text);
    if seen.insert(base.clone()) {
        return base;
    }
    for n in 1.. {
        let candidate = format!("{base}-{n}");
        if seen.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn punctuation_dropped() {
        assert_eq!(slugify("It's a Test!"), "its-a-test");
    }

    #[test]
    fn unicode_lowercased() {
        assert_eq!(slugify("Olá Mundo"), "olá-mundo");
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(slugify("  many   spaces  "), "many-spaces");
    }

    #[test]
    fn unique_on_collisions() {
        let mut seen = std::collections::HashSet::new();
        assert_eq!(unique_slug("Intro", &mut seen), "intro");
        assert_eq!(unique_slug("Intro", &mut seen), "intro-1");
        assert_eq!(unique_slug("Intro", &mut seen), "intro-2");
    }
}
