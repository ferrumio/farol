use std::collections::HashSet;

use farol_core::slug::{slugify, unique_slug};

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
    let mut seen = HashSet::new();
    assert_eq!(unique_slug("Intro", &mut seen), "intro");
    assert_eq!(unique_slug("Intro", &mut seen), "intro-1");
    assert_eq!(unique_slug("Intro", &mut seen), "intro-2");
}
