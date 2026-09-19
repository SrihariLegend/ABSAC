//! Streaming FNV-1a hashing used for identity fingerprints and
//! checker-issued artifact digests. `fmt::Write` payloads are hashed in
//! place instead of being materialized as `String`s.

#[derive(Clone, Copy, Debug)]
pub struct Fnv1a64(u64);

impl Fnv1a64 {
    pub fn new() -> Self {
        Self(0xcbf29ce484222325)
    }

    #[inline]
    pub fn write_u8(&mut self, value: u8) {
        self.0 ^= u64::from(value);
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }

    #[inline]
    pub fn write_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.write_u8(byte);
        }
    }

    #[inline]
    pub fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }

    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_u8(*byte);
        }
    }

    pub fn finish(self) -> u64 {
        self.0
    }
}

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Write for Fnv1a64 {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.write_bytes(s.as_bytes());
        Ok(())
    }
}

impl std::hash::Hasher for Fnv1a64 {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes);
    }
}

/// Digest the `Debug` form of a value without allocating a `String`.
pub fn debug_digest<T: std::fmt::Debug>(value: &T) -> u64 {
    use std::fmt::Write as _;
    let mut hash = Fnv1a64::new();
    let _ = write!(hash, "{value:?}");
    hash.finish()
}

/// Digest a hashable value directly (no formatting).
pub fn hash_digest<T: std::hash::Hash>(value: &T) -> u64 {
    let mut hash = Fnv1a64::new();
    value.hash(&mut hash);
    hash.finish()
}

/// FxHash-style integer hasher for internal maps. This is deliberately
/// not the digest hasher: artifact identity keeps FNV, while lookup
/// tables use a multiply-xor word hash with no per-byte dependency
/// chain.
#[derive(Clone, Copy, Debug, Default)]
pub struct FastHasher(u64);

impl FastHasher {
    #[inline]
    fn mix(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x517cc1b727220a95);
    }
}

impl std::hash::Hasher for FastHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = 0u64;
            for (index, byte) in chunk.iter().enumerate() {
                word |= u64::from(*byte) << (index * 8);
            }
            self.mix(word);
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.mix(u64::from(value));
    }

    fn write_u32(&mut self, value: u32) {
        self.mix(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.mix(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.mix(value as u64);
    }
}

/// `HashMap` with the fast integer hasher. Internal maps keyed by
/// ids/terms dominate solver and semantics profiles; SipHash is not
/// needed for these deterministic, non-adversarial keys.
pub type FastHashMap<K, V> =
    std::collections::HashMap<K, V, std::hash::BuildHasherDefault<FastHasher>>;

/// `HashSet` counterpart of [`FastHashMap`].
pub type FastHashSet<T> = std::collections::HashSet<T, std::hash::BuildHasherDefault<FastHasher>>;
