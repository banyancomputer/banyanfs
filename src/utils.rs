use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub fn crypto_rng() -> ChaCha20Rng {
    ChaCha20Rng::from_entropy()
}

pub fn current_time_ms() -> u64 {
    use time::OffsetDateTime;
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as u64
}
