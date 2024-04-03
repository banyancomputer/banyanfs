mod vec_stream;

pub(crate) use vec_stream::vec_to_pinned_stream;

use async_std::prelude::*;
use blake3::Hasher;
use bytes::{Bytes, BytesMut};

use crate::codec::crypto::VerifyingKey;

const FINGERPRINT_SIZE: usize = 20;

/// The API uses a truncated blake3 hash for key identification. This provides a convenient method
/// to generate matching one from a public key.
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

/// Consumes an async stream into a single Bytes object. This will consume potentially boundless
/// memory which is especially problematic since we will be handling very large files. It is
/// intended primarily for WASM targeted builds where async is significantly more limited.
pub(crate) async fn consume_stream_into_bytes<S, E>(mut stream: S) -> Result<Bytes, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::error::Error,
{
    let mut bytes_mut = BytesMut::new();

    while let Some(item) = stream.next().await {
        let bytes = item?;
        bytes_mut.extend_from_slice(&bytes);
    }

    Ok(bytes_mut.freeze())
}
