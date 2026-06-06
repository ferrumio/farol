use farol_core::{Hash, Hasher};

#[test]
fn hex_is_64_chars() {
    let h = Hash::of(b"hello");
    assert_eq!(h.hex().len(), 64);
}

#[test]
fn same_input_same_hash() {
    assert_eq!(Hash::of(b"x"), Hash::of(b"x"));
    assert_ne!(Hash::of(b"x"), Hash::of(b"y"));
}

#[test]
fn hasher_matches_single_of() {
    let single = Hash::of(b"abc");
    let incremental = Hasher::new().update(b"a").update(b"b").update(b"c").finish();
    assert_eq!(single, incremental);
}

#[test]
fn tags_separate_streams() {
    let a = Hasher::new().tag("foo").update(b"content").finish();
    let b = Hasher::new().tag("bar").update(b"content").finish();
    assert_ne!(a, b);
}
