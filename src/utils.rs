use crate::codec::Cid;

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Number of bytes that make up a raw CID within this framework. When properly formatted as a
/// multicodec CIDv1 it will be longer. For the full size representation please refer to
/// [`crate::codec::Cid`].
pub const CID_HASH_SIZE: usize = 32;

/// Quick helper that produces the library specific [`Cid`] object over the given data. Internally
/// this is simply a 32 byte BLAKE3 hash of the data wrapped around a helper struct. For formating
/// as a standard CIDv1 string, please see the [`Cid::as_base64url_multicodec`] method.
pub fn calculate_cid(data: &[u8]) -> Cid {
    let hash: [u8; CID_HASH_SIZE] = blake3::hash(data).into();
    Cid::from(hash)
}

/// Helper utility in regular builds to produce a standard RNG for cryptographic use. Implemented
/// to allow a standardized way to access an environment specific secure RNG.
///
/// Selection of RNG may change in the future which would affect this return type. Should generally
/// treat the returned type as an object implementing the `rand::CrytoRngCore` trait.
#[cfg(not(target_arch = "wasm32"))]
pub fn crypto_rng() -> ChaCha20Rng {
    ChaCha20Rng::from_entropy()
}

/// Helper utility in WASM builds to produce a standard RNG for cryptographic use. Implemented
/// to allow a standardized way to access an environment specific secure RNG.
///
/// Selection of RNG may change in the future which would affect this return type. Should generally
/// treat the returned type as an object implementing the `rand::CrytoRngCore` trait.
#[cfg(target_arch = "wasm32")]
pub fn crypto_rng() -> ChaCha20Rng {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).expect("setting the RNG seed");
    ChaCha20Rng::from_seed(seed)
}

/// Helper utility to get the current time in milliseconds since the Unix epoch. This is the finest
/// level of precision on timestamps supported by BanyanFS and matches the precision of other
/// formats.
///
/// The underlying time library is likely to be removed in favor of more standard uses to reduce
/// the dependency footprint in some of the environments we target.
pub fn current_time_ms() -> i64 {
    use time::OffsetDateTime;
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

/// Useful as a low character count way to generate informative [`std::io::Error`] error messages.
/// Maybe be removed in the future for concrete error types.
pub(crate) fn std_io_err(msg: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, msg)
}
