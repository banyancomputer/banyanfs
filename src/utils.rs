use crate::codec::Cid;

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

const CID_HASH_SIZE: usize = 32;

pub fn calculate_cid(data: &[u8]) -> Cid {
    let hash: [u8; CID_HASH_SIZE] = blake3::hash(data).into();
    Cid::from(hash)
}

#[cfg(not(taget_arch = "wasm32"))]
pub fn crypto_rng() -> ChaCha20Rng {
    ChaCha20Rng::from_entropy()
}

#[cfg(taget_arch = "wasm32")]
pub fn crypto_rng() -> ChaCha20Rng {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).expect();
    Ok(ChaCha20Rng::from_seed(seed))
}

pub fn current_time_ms() -> i64 {
    use time::OffsetDateTime;
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

pub(crate) fn std_io_err(msg: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, msg)
}
