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

pub struct EncryptedDataChunk(Box<[u8]>);

impl EncryptedDataChunk {
    pub fn new(data: Box<[u8]>) -> Self {
        Self(data)
    }

    pub async fn from_parts(nonce: Nonce, data: &[u8], tag: AuthenticationTag) -> Self {
        let mut encoded = Vec::new();
        nonce
            .encode(&mut encoded)
            .await
            .expect("Can only fail if allocator is starved");
        encoded.extend_from_slice(data);
        tag.encode(&mut encoded)
            .await
            .expect("Can only fail if allocator is starved");
        Self(encoded.into_boxed_slice())
    }

    pub fn cid(&self) -> Cid {
        crate::utils::calculate_cid(&self.0)
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<(usize, Cid)> {
        let cid = self.cid();

        writer.write_all(&self.0).await?;
        Ok((self.0.len(), cid))
    }

    pub fn decrypt(
        &self,
        options: &DataOptions,
        access_key: &AccessKey,
    ) -> Result<DataChunk, EncryptedDataChunkError> {
        if self.0.len()
            != usize::try_from(options.chunk_size())
                .expect("This code assumes it is running on a 32 bit or large platform")
        {
            return Err(EncryptedDataChunkError::ChunkLengthError);
        }
        let nonce = Nonce::from_bytes(
            self.0[..Nonce::size()]
                .try_into()
                .expect("We have checked the size above"),
        );
        let authentication_tag = AuthenticationTag::from_bytes(
            &self.0[(self.0.len() - AuthenticationTag::size())..]
                .try_into()
                .expect("We have checked the size above"),
        );
        let mut plaintext_payload =
            Vec::from(&self.0[Nonce::size()..(self.0.len() - AuthenticationTag::size())]);

        if let Err(err) =
            access_key.decrypt_buffer(nonce, &[], &mut plaintext_payload, authentication_tag)
        {
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

        let (input, data) = take(options.chunk_size()).parse_peek(input)?;

        Ok((input, Self::new(data.into())))
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
