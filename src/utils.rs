use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

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
