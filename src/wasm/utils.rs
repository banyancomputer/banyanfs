use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub(crate) fn chacha_rng() -> Result<ChaCha20Rng, getrandom::Error> {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed)?;
    Ok(ChaCha20Rng::from_seed(seed))
}
