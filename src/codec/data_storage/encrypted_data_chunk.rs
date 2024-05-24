use elliptic_curve::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use rand::Rng;
use winnow::binary::le_u32;
use winnow::binary::length_take;
use winnow::error::ErrMode;
use winnow::token::take;
use winnow::Parser;

use super::{data_chunk::DataChunk, data_options::DataOptions};
use crate::codec::crypto::AccessKey;
use crate::codec::crypto::AuthenticationTag;
use crate::codec::crypto::Nonce;
use crate::codec::{Cid, ParserResult, Stream};

pub struct EncryptedDataChunk {
    nonce: Nonce,
    payload: Box<[u8]>,
    authentication_tag: AuthenticationTag,
}

impl EncryptedDataChunk {
    pub fn new(nonce: Nonce, payload: Box<[u8]>, authentication_tag: AuthenticationTag) -> Self {
        Self {
            nonce,
            payload,
            authentication_tag,
        }
    }

    pub async fn cid(&self) -> Cid {
        let mut encoded = Vec::new();

        self.nonce
            .encode(&mut encoded)
            .await
            .expect("Infalliable unless we run out of heap");
        encoded
            .write_all(&self.payload)
            .await
            .expect("Infalliable unless we run out of heap");
        self.authentication_tag
            .encode(&mut encoded)
            .await
            .expect("Infalliable unless we run out of heap");

        crate::utils::calculate_cid(&encoded)
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<(usize, Cid)> {
        let mut encoded = Vec::new();

        self.nonce.encode(&mut encoded).await?;
        encoded.write_all(&self.payload).await?;
        self.authentication_tag.encode(&mut encoded).await?;

        let cid = crate::utils::calculate_cid(&encoded);

        writer.write_all(&encoded).await?;
        Ok((encoded.len(), cid))
    }

    pub fn decrypt<'a>(
        &'a self,
        options: &DataOptions,
        access_key: &AccessKey,
    ) -> Result<DataChunk, EncryptedDataChunkError> {
        let mut plaintext_payload = Vec::from(self.payload.clone());
        if let Err(err) = access_key.decrypt_buffer(
            self.nonce.clone(),
            &[],
            &mut plaintext_payload,
            self.authentication_tag.clone(),
        ) {
            tracing::error!("failed to decrypt chunk: {err}");
            return Err(EncryptedDataChunkError::DecryptError);
        }

        let (_remaining, plaintext_data) =
            length_take(le_u32::<&[u8], ErrMode<winnow::error::ContextError>>)
                .parse_peek(plaintext_payload.as_slice())
                .map_err(|_| EncryptedDataChunkError::PlainTextLengthParseError)?;

        let chunk = DataChunk::from_slice(plaintext_data, options)
            .map_err(|_| EncryptedDataChunkError::ChunkLengthError)?;
        Ok(chunk)
    }

    pub async fn encode_padding_chunk<W: AsyncWrite + Unpin + Send>(
        rng: &mut impl CryptoRngCore,
        options: &DataOptions,
        writer: &mut W,
    ) -> std::io::Result<(usize, Cid)> {
        let mut chunk_data = vec![
            0;
            usize::try_from(options.chunk_size())
                .expect("Architectures below 32 bit are not supported")
        ];
        rng.fill(chunk_data.as_mut_slice());

        let cid = crate::utils::calculate_cid(&chunk_data);

        writer.write_all(&chunk_data).await?;
        Ok((chunk_data.len(), cid))
    }

    pub fn parse<'a>(input: Stream<'a>, options: &DataOptions) -> ParserResult<'a, Self> {
        if !options.encrypted {
            unimplemented!("unencrypted data blocks are not yet supported");
        }
        let (input, nonce) = Nonce::parse(input)?;
        let (input, payload) = take(options.chunk_payload_size()).parse_peek(input)?;
        let (input, tag) = AuthenticationTag::parse(input)?;

        Ok((input, Self::new(nonce, payload.into(), tag)))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncryptedDataChunkError {
    #[error("Error Decrypting a chunk")]
    DecryptError,
    #[error("Error parsing the length field of the chunk after decryption")]
    PlainTextLengthParseError,
    #[error("Decrypted chunk plaintext is too long for the data options currently set")]
    ChunkLengthError,
}
