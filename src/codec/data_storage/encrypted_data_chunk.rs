use elliptic_curve::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use rand::Rng;
use winnow::binary::le_u64;
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
    data: Box<[u8]>,
    authentication_tag: AuthenticationTag,
}

impl EncryptedDataChunk {
    pub fn new(nonce: Nonce, data: Box<[u8]>, authentication_tag: AuthenticationTag) -> Self {
        Self {
            nonce,
            data,
            authentication_tag,
        }
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<(usize, Cid)> {
        let mut encoded = Vec::new();

        self.nonce.encode(&mut encoded).await?;
        encoded.write_all(&self.data).await?;
        self.authentication_tag.encode(&mut encoded).await?;

        let cid = crate::utils::calculate_cid(&encoded);

        writer.write_all(&encoded).await?;
        Ok((encoded.len(), cid))
    }

    pub fn decrypt<'a>(
        &'a self,
        options: &DataOptions,
        access_key: &AccessKey,
    ) -> Result<DataChunk, DataChunkError> {
        let mut plaintext_data = Vec::from(self.data.clone());
        if let Err(err) = access_key.decrypt_buffer(
            self.nonce.clone(),
            &[],
            &mut plaintext_data,
            self.authentication_tag.clone(),
        ) {
            tracing::error!("failed to decrypt chunk: {err}");
            return Err(DataChunkError::DecryptError);
        }

        let data_length = match le_u64::<&[u8], ErrMode<winnow::error::ContextError>>
            .parse_peek(plaintext_data.as_slice())
        {
            Ok((_, length)) => length,
            Err(err) => {
                tracing::error!("failed to read inner length: {err:?}");
                return Err(DataChunkError::PlainTextLengthParseError);
            }
        };
        let plaintext_data: Vec<u8> = plaintext_data
            .drain(8..(data_length as usize + 8))
            .collect();
        let chunk = DataChunk::from_slice(&plaintext_data, options)
            .map_err(|_| DataChunkError::ChunkLengthError)?;
        Ok(chunk)
    }

    pub async fn encode_padding_chunk<W: AsyncWrite + Unpin + Send>(
        rng: &mut impl CryptoRngCore,
        options: &DataOptions,
        writer: &mut W,
    ) -> std::io::Result<(usize, Cid)> {
        let mut chunk_data = vec![0; options.chunk_size()];
        rng.fill(chunk_data.as_mut_slice());

        let cid = crate::utils::calculate_cid(&chunk_data);

        writer.write_all(&chunk_data).await?;
        Ok((chunk_data.len(), cid))
    }

    pub fn parse<'a>(input: Stream<'a>, options: &DataOptions) -> ParserResult<'a, Self> {
        if !options.encrypted() {
            unimplemented!("unencrypted data blocks are not yet supported");
        }
        let (input, nonce) = Nonce::parse(input)?;
        let (input, data) = take(options.encrypted_chunk_data_size() + 8).parse_peek(input)?;
        let (input, tag) = AuthenticationTag::parse(input)?;

        Ok((input, Self::new(nonce, data.into(), tag)))
    }
}

pub enum DataChunkError {
    DecryptError,
    PlainTextLengthParseError,
    ChunkLengthError,
}
