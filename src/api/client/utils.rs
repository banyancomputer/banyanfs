use blake3::Hasher;

use crate::codec::crypto::VerifyingKey;

const FINGERPRINT_SIZE: usize = 20;

/// The API uses a truncated blake3 hash for key identification.
///
/// todo(sstelfox): This needs to be reverted back to a standard size
pub(crate) fn api_fingerprint_key(key: &VerifyingKey) -> String {
    let compressed_point = key.to_encoded_point(true);
    let compressed_point = compressed_point.as_bytes();

    let mut hasher = Hasher::new();
    hasher.update(compressed_point);
    let mut hash_reader = hasher.finalize_xof();

    let mut output = [0u8; FINGERPRINT_SIZE];
    hash_reader.fill(&mut output);

    output.iter().fold(String::new(), |mut acc, byte| {
        acc.push_str(&format!("{:02x}", byte));
        acc
    })
}
