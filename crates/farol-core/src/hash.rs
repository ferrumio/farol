use sha2::{Digest, Sha256};

/// A 32-byte content hash, rendered as 64 hex chars when displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Hash a single byte slice.
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(hasher.finalize().into())
    }

    /// Hex representation. Useful as a stable key or cache lookup.
    pub fn hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for b in self.0 {
            out.push_str(&format!("{b:02x}"));
        }
        out
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hex())
    }
}

/// Incrementally build a Hash from multiple updates. Useful when combining
/// many inputs (content + config + dependency hashes).
pub struct Hasher(Sha256);

impl Default for Hasher {
    fn default() -> Self {
        Self(Sha256::new())
    }
}

impl Hasher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(mut self, bytes: &[u8]) -> Self {
        self.0.update(bytes);
        self
    }

    /// Mix in a domain tag to prevent collisions between unrelated streams.
    pub fn tag(mut self, tag: &str) -> Self {
        self.0.update(tag.as_bytes());
        self.0.update([0u8]);
        self
    }

    pub fn finish(self) -> Hash {
        Hash(self.0.finalize().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
