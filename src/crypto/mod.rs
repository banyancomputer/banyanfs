use chacha20poly1305::aead::generic_array::GenericArray;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::XChaCha20Poly1305;
use ecdsa::signature::{DigestVerifier, RandomizedDigestSigner};
use elliptic_curve::sec1::ToEncodedPoint;
use p384::ecdh::EphemeralSecret;
use p384::{NistP384, PublicKey};
use rand::Rng;
use sha2::Digest;

mod signing_key;
pub(crate) mod utils;

pub(crate) use signing_key::SigningKey;

pub fn full_key_walkthrough() {
    let mut rng = utils::cs_rng();

    let key_contents = b"deadbeefdeadbeefdeadbeefdeadbeef";
    let authenticated_data = b"visible but verified";

    //let key = AccessKey::generate(&mut rng);
    //let nonce = Nonce::generate(&mut rng);

    //let payload = key
    //    .encrypt(nonce, key_contents, authenticated_data)
    //    .unwrap();

    //// NOTE: Encryption boundary, need to transfer authenticated_data, nonce, cipher_text, tag. It is expected remote side already has key

    //let access_key = payload.decrypt(&key, authenticated_data).unwrap();
    //tracing::info!(
    //    "received_key_contents({})={:02x?}",
    //    access_key.len(),
    //    access_key.as_bytes(),
    //);

    // lets get to our ECDSA keys... and do some basic operations on them
    let p384_signing_key = ecdsa::SigningKey::<NistP384>::random(&mut rng);

    let p384_signing_key_bytes = p384_signing_key.to_bytes().to_vec();
    tracing::info!(
        "p384_signing_key({})={p384_signing_key_bytes:02x?}",
        p384_signing_key_bytes.len()
    );

    // Could be either of the following:
    //let p384_verifying_key = ecdsa::VerifyingKey::from(&p384_signing_key);
    let p384_verifying_key = p384_signing_key.verifying_key();

    // encode the public key to our standardized format
    let p384_verifying_key_bytes = p384_verifying_key
        .to_encoded_point(true)
        .as_bytes()
        .to_vec();

    tracing::info!(
        "p384_verifying_key_compressed_bytes({})={p384_verifying_key_bytes:02x?}",
        p384_verifying_key_bytes.len()
    );

    // signing
    let mut digest = sha2::Sha384::new();
    digest.update(&key_contents);
    let signature: ecdsa::Signature<NistP384> =
        p384_signing_key.sign_digest_with_rng(&mut rng, digest);
    let signature_bytes = signature.to_vec();

    tracing::info!(
        "sha_384_p384_signature_bytes({})={signature_bytes:02x?}",
        signature_bytes.len()
    );

    // verifying
    let signature: ecdsa::Signature<NistP384> =
        ecdsa::Signature::from_slice(&signature_bytes).unwrap();
    let mut digest = sha2::Sha384::new();
    digest.update(&key_contents);
    p384_verifying_key
        .verify_digest(digest, &signature)
        .unwrap();
    tracing::info!("signature verified");

    // ECDH key exchange
    let eph_secret: EphemeralSecret = EphemeralSecret::random(&mut rng);
    let eph_pub_bytes = eph_secret
        .public_key()
        .to_encoded_point(true)
        .as_bytes()
        .to_vec();

    tracing::info!(
        "ephemeral_pub_exchange_key({})={eph_pub_bytes:02x?}",
        eph_pub_bytes.len()
    );

    // the fixed pub key here is the intended recipient of the encrypted data
    let fixed_pubkey = PublicKey::from_sec1_bytes(&p384_verifying_key_bytes).unwrap();
    let eph_secret = eph_secret.diffie_hellman(&fixed_pubkey);
    let eph_hkdf = eph_secret.extract::<sha2::Sha384>(None);
    let mut secret_bytes: [u8; 32] = [0; 32];
    eph_hkdf.expand(&[], &mut secret_bytes).unwrap();

    tracing::info!(
        "initial_ecdh_secret_bytes({})={secret_bytes:02x?}",
        secret_bytes.len()
    );

    let fs_key: [u8; 32] = rng.gen();
    let nonce: [u8; 24] = rng.gen();

    let mut fs_key_payload = [0u8; 4].to_vec();
    fs_key_payload.extend(&fs_key);

    // encrypt our symmetric key with the ephemeral key for our recipient
    let key_payload = Payload {
        msg: &fs_key_payload,
        aad: &[],
    };
    let encrypted_fs_key_blob = XChaCha20Poly1305::new(GenericArray::from_slice(&secret_bytes))
        .encrypt(GenericArray::from_slice(&nonce), key_payload)
        .unwrap();

    tracing::info!(
        "encrypted_fs_key_blob({})={encrypted_fs_key_blob:02x?}, nonce({})={nonce:02x?}",
        encrypted_fs_key_blob.len(),
        nonce.len(),
    );

    let eph_pubkey = PublicKey::from_sec1_bytes(&eph_pub_bytes).unwrap();
    let recovered_eph_secret = elliptic_curve::ecdh::diffie_hellman(
        p384_signing_key.as_nonzero_scalar(),
        eph_pubkey.as_affine(),
    );
    let eph_hkdf = recovered_eph_secret.extract::<sha2::Sha384>(None);
    let mut recovered_secret_bytes: [u8; 32] = [0; 32];
    eph_hkdf.expand(&[], &mut recovered_secret_bytes).unwrap();

    tracing::info!(
        "recovered_ecdh_secret_bytes({})={recovered_secret_bytes:02x?}",
        recovered_secret_bytes.len()
    );

    // blake3 hashing
    let data_to_hash = b"some data to hash";
    tracing::info!("data_to_hash({})={data_to_hash:02x?}", data_to_hash.len());

    let mut hasher = blake3::Hasher::new();
    hasher.update(data_to_hash);
    let final_hash = hasher.finalize();
    let hash = final_hash.to_vec();
    tracing::info!("blake3_hash({})={hash:02x?}", hash.len());
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CryptoError {
    /// I would love this to be more descriptive, but the underlying library deliberately opaques
    /// the failure reason to avoid potential side-channel leakage.
    #[error("failed to perform symmetric decryption")]
    DecryptionFailure,

    #[error("failed to perform symmetric encryption")]
    EncryptionFailure,
}
