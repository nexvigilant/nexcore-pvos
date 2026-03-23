/// FNV-1a hash for audit records (zero-dep, deterministic).
///
/// Tier: T1 (pure Mapping primitive)
#[must_use]
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_deterministic() {
        let a = fnv1a_hash(b"hello");
        let b = fnv1a_hash(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn test_fnv1a_different_inputs() {
        let a = fnv1a_hash(b"hello");
        let b = fnv1a_hash(b"world");
        assert_ne!(a, b);
    }
}
