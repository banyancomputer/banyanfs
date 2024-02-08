use sha2::Digest;

mod signing_key;
pub(crate) mod utils;

use crate::codec::crypto::AccessKey;
pub(crate) use signing_key::SigningKey;

pub fn full_key_walkthrough() {
    let mut rng = utils::cs_rng();

    let access_key = AccessKey::generate(&mut rng);
    let signing_key_one = SigningKey::generate(&mut rng);
    let locked_for_one = access_key
        .lock_for(&mut rng, &signing_key_one.verifying_key())
        .unwrap();
    let unlocked_access_key = locked_for_one.unlock(&signing_key_one).unwrap();

    let original_key = access_key.chacha_key().unwrap().to_vec();
    let recovered_key = unlocked_access_key.chacha_key().unwrap().to_vec();
    tracing::info!("original_key={original_key:02x?}, recovered_key={recovered_key:02x?}");

    // blake3 hashing
    let data_to_hash = b"some data to hash";
    tracing::info!("data_to_hash({})={data_to_hash:02x?}", data_to_hash.len());

    let mut hasher = blake3::Hasher::new();
    hasher.update(data_to_hash);
    let final_hash = hasher.finalize();
    let hash = final_hash.to_vec();
    tracing::info!("blake3_hash({})={hash:02x?}", hash.len());
}
