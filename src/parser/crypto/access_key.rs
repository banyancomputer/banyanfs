use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{
    Key as ChaChaKey, Tag as ChaChaTag, XChaCha20Poly1305, XNonce as ChaChaNonce,
};
use ecdsa::signature::rand_core::CryptoRngCore;
use nom::bits::bits;
use nom::bytes::streaming::{tag, take};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::multi::count;
use nom::number::streaming::{le_u32, le_u8};
use nom::sequence::tuple;
use nom::AsBytes;
use nom::{IResult, Needed};
use rand::{CryptoRng, Rng};

//use crate::crypto::utils::short_symmetric_decrypt;
//use crate::crypto::{AuthenticationTag, CryptoError, Nonce, SigningKey};
use crate::crypto::{CryptoError, SigningKey};
use crate::parser::crypto::{
    AuthenticationTag, KeyId, Nonce, VerifyingKey, SYMMETRIC_KEY_LENGTH, TAG_LENGTH,
};

const ACCESS_KEY_CIPHER_TEXT_LENGTH: usize = SYMMETRIC_KEY_LENGTH + KEY_VERIFICATION_PATTERN_LENGTH;

const ACCESS_KEY_RECORD_LENGTH: usize = KeyId::size() + VerifyingKey::size() + 146;

const KEY_VERIFICATION_PATTERN_LENGTH: usize = 4;

#[derive(Clone)]
pub(crate) enum AccessKey {
    Locked {
        key_id: KeyId,
        dh_exchange_key: VerifyingKey,
        nonce: Nonce,
        cipher_text: [u8; ACCESS_KEY_CIPHER_TEXT_LENGTH],
        tag: AuthenticationTag,
    },
    Open {
        key: [u8; SYMMETRIC_KEY_LENGTH],
    },
}

impl AccessKey {
    pub(crate) fn chacha_key(&self) -> Result<&ChaChaKey, AccessKeyError<&[u8]>> {
        match self {
            Self::Locked { .. } => Err(AccessKeyError::LockedKey),
            Self::Open { key } => Ok(ChaChaKey::from_slice(key)),
        }
    }

    pub(crate) fn generate(rng: &mut impl Rng) -> Self {
        Self::Open { key: rng.gen() }
    }

    pub(crate) fn lock_for(
        &self,
        rng: &mut impl CryptoRngCore,
        verifying_key: &VerifyingKey,
    ) -> Result<Self, AccessKeyError<&[u8]>> {
        match self {
            Self::Locked { .. } => Ok(self.clone()),
            Self::Open { key } => {
                let (dh_exchange_key, shared_secret) = verifying_key.ephemeral_dh_exchange(rng);

                // Intentionally leave the last four bytes as zeros which acts as our successful
                // decryption oracle.
                let mut key_payload = [0u8; 36];
                key_payload[..32].copy_from_slice(key);

                let chacha_key = ChaChaKey::from_slice(&shared_secret);
                let cipher = XChaCha20Poly1305::new(chacha_key);

                let nonce = Nonce::generate(rng);
                let raw_tag = cipher.encrypt_in_place_detached(&nonce, &[], &mut key_payload)?;

                let mut tag_bytes = [0u8; TAG_LENGTH];
                tag_bytes.copy_from_slice(raw_tag.as_bytes());
                let tag = AuthenticationTag::from(tag_bytes);

                let key_id = verifying_key.key_id();

                Ok(Self::Locked {
                    dh_exchange_key,
                    nonce,
                    cipher_text: key_payload,
                    tag,
                    key_id,
                })
            }
        }
    }

    pub(crate) fn unlock(&self, key: &SigningKey) -> Result<Self, AccessKeyError<&[u8]>> {
        todo!()
        //let result = short_symmetric_decrypt(key, &self.nonce, &self.cipher_text, &self.tag, aad)
        //    .map_err(EncryptedPayloadError::CryptoFailure)?;

        //let mut fixed_key: [u8; 32] = [0u8; 32];
        //fixed_key.copy_from_slice(&result);

        //Ok(AccessKey::Open { key: fixed_key })
    }

    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (key_id, dh_exchange_key, nonce, cipher_text, tag)) = tuple((
            KeyId::parse,
            VerifyingKey::parse,
            Nonce::parse,
            take(ACCESS_KEY_CIPHER_TEXT_LENGTH),
            AuthenticationTag::parse,
        ))(input)?;

        let access_key = AccessKey::Locked {
            key_id,
            dh_exchange_key,
            nonce,
            cipher_text: cipher_text.try_into().unwrap(),
            tag,
        };

        Ok((input, access_key))
    }

    pub(crate) fn parse_many(input: &[u8], key_count: u8) -> IResult<&[u8], Vec<Self>> {
        let (input, keys) = match count(Self::parse, key_count as usize)(input) {
            Ok(res) => res,
            Err(nom::Err::Incomplete(Needed::Size(_))) => {
                // If there wasn't enough data for one of the records, return how much more data we
                // _actually_ need before we can keep going.
                let total_size = key_count as usize * ACCESS_KEY_RECORD_LENGTH;

                return Err(nom::Err::Incomplete(Needed::new(total_size - input.len())));
            }
            Err(err) => return Err(err),
        };

        Ok((input, keys))
    }

    //pub(crate) fn to_bytes(&self) -> Result<[u8; 148], AccessKeyError<&[u8]>> {
    //    match self {
    //        AccessKey::Locked {
    //            nonce,
    //            cipher_text,
    //            tag,
    //        } => {
    //            let mut bytes = [0u8; 148];
    //            let mut current_idx = 0;

    //            let nonce_bytes = nonce.as_bytes();
    //            let nonce_len = nonce_bytes.len();
    //            bytes[current_idx..(current_idx + nonce_len)].copy_from_slice(nonce_bytes);
    //            current_idx += nonce_len;

    //            let cipher_len = cipher_text.len();
    //            bytes[current_idx..(current_idx + cipher_len)].copy_from_slice(cipher_text);
    //            current_idx += cipher_len;

    //            let tag_bytes = tag.as_bytes();
    //            let tag_len = tag_bytes.len();
    //            bytes[current_idx..(current_idx + tag_len)].copy_from_slice(tag_bytes);

    //            Ok(bytes)
    //        }
    //        AccessKey::Open { .. } => unimplemented!(),
    //    }
    //}
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AccessKeyError<I> {
    #[error("decoding data failed: {0}")]
    FormatFailure(#[from] nom::Err<nom::error::Error<I>>),

    #[error("unspecified crypto error")]
    CryptoFailure,

    #[error("validation failed most likely due to the use of an incorrect key")]
    IncorrectKey,

    #[error("key must be unlocked before it can be used")]
    LockedKey,
}

impl<I> From<chacha20poly1305::Error> for AccessKeyError<I> {
    fn from(_: chacha20poly1305::Error) -> Self {
        AccessKeyError::CryptoFailure
    }
}
