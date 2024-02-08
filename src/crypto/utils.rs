use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub(crate) fn cs_rng() -> ChaCha20Rng {
    ChaCha20Rng::from_entropy()
}
