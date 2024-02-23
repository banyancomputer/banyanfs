const HASH_SIZE: usize = 32;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Hash([u8; HASH_SIZE]);

impl std::fmt::Debug for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hsh_str: String = self
            .0
            .iter()
            .fold(String::new(), |acc, &b| format!("{acc}{:02x}", b));

        write!(f, "{{0x{hsh_str}}}")
    }
}

impl From<[u8; HASH_SIZE]> for Hash {
    fn from(bytes: [u8; HASH_SIZE]) -> Self {
        Self(bytes)
    }
}
