use crate::codec::crypto::{AccessKey, AuthenticationTag, Nonce};
use crate::codec::BlockSizeError;
use crate::codec::{Cid, ParserResult, Stream};
use crate::utils::std_io_err;
use elliptic_curve::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use rand::Rng;
use winnow::binary::le_u64;
use winnow::error::ErrMode;
use winnow::stream::Offset;
use winnow::token::take;
use winnow::Parser;

use super::data_options::DataOptions;
use super::encrypted_data_chunk::EncryptedDataChunk;
use super::DataBlockError;

pub struct DataChunk {
    contents: Box<[u8]>,
}

impl DataChunk {
    pub fn from_slice(data: &[u8], options: &DataOptions) -> Result<Self, DataBlockError> {
        if data.len() > options.encrypted_chunk_data_size() {
            return Err(DataBlockError::Size(BlockSizeError::ChunkTooLarge(
                data.len(),
                options.encrypted_chunk_data_size(),
            )));
        }
        Ok(Self {
            contents: data.into(),
        })
    }

    pub fn parse<'a>(
        input: Stream<'a>,
        options: &DataOptions,
        access_key: &AccessKey,
    ) -> ParserResult<'a, Self> {
        if !options.encrypted() {
            unimplemented!("unencrypted data blocks are not yet supported");
        }
        let chunk_start = input;

        let (input, nonce) = Nonce::parse(input)?;
        let (input, data) = take(options.encrypted_chunk_data_size() + 8).parse_peek(input)?;
        let (input, tag) = AuthenticationTag::parse(input)?;

        debug_assert!(
            input.offset_from(&chunk_start) == options.chunk_size(),
            "chunk should be fully read"
        );

        let mut plaintext_data = data.to_vec();
        if let Err(err) = access_key.decrypt_buffer(nonce, &[], &mut plaintext_data, tag) {
            tracing::error!("failed to decrypt chunk: {err}");
            let err = winnow::error::ParserError::from_error_kind(
                &input,
                winnow::error::ErrorKind::Verify,
            );
            return Err(winnow::error::ErrMode::Cut(err));
        }

        let data_length = match le_u64::<&[u8], ErrMode<winnow::error::ContextError>>
            .parse_peek(plaintext_data.as_slice())
        {
            Ok((_, length)) => length,
            Err(err) => {
                tracing::error!("failed to read inner length: {err:?}");

                let empty_static: &'static [u8] = &[];
                return Err(winnow::error::ErrMode::Cut(
                    winnow::error::ParserError::from_error_kind(
                        &Stream::new(empty_static),
                        winnow::error::ErrorKind::Verify,
                    ),
                ));
            }
        };
        let plaintext_data: Vec<u8> = plaintext_data
            .drain(8..(data_length as usize + 8))
            .collect();

        Ok((
            input,
            Self {
                contents: plaintext_data.into_boxed_slice(),
            },
        ))
    }

    pub fn data(&self) -> &[u8] {
        &self.contents
    }

    pub async fn encrypt(
        &self,
        rng: &mut impl CryptoRngCore,
        options: &DataOptions,
        access_key: &AccessKey,
    ) -> std::io::Result<EncryptedDataChunk> {
        let mut ciphertext = Vec::new();

        let (_size, cid) = self
            .encode(rng, options, access_key, &mut ciphertext)
            .await?;
        Ok(EncryptedDataChunk::new(ciphertext.into_boxed_slice(), cid))
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        options: &DataOptions,
        access_key: &AccessKey,
        writer: &mut W,
    ) -> std::io::Result<(usize, Cid)> {
        if !options.encrypted() {
            unimplemented!("unencrypted data blocks are not yet supported");
        }
        if self.contents.len() > options.encrypted_chunk_data_size() {
            tracing::error!(true_length = ?self.contents.len(), max_length = options.encrypted_chunk_data_size(), "chunk too large");
            return Err(std_io_err("chunk size mismatch (chunk too large)"));
        }

        // write out the true data length
        let chunk_length = self.contents.len() as u64;
        let chunk_length_bytes = chunk_length.to_le_bytes();

        // We need to prepend the length of the data, the pad the remaining space with random
        // data.
        let full_size = options.encrypted_chunk_data_size() + chunk_length_bytes.len();
        let mut payload = Vec::with_capacity(full_size);
        payload.extend_from_slice(&chunk_length_bytes);
        payload.extend_from_slice(self.data());
        payload.resize_with(full_size, || rng.gen());

        let (nonce, tag) = access_key
            .encrypt_buffer(rng, &[], &mut payload)
            .map_err(|_| std_io_err("failed to encrypt chunk"))?;

        let mut chunk_data = Vec::with_capacity(options.chunk_size());
        nonce.encode(&mut chunk_data).await?;
        chunk_data.write_all(&payload).await?;
        tag.encode(&mut chunk_data).await?;

        debug_assert_eq!(chunk_data.len(), options.chunk_size());

        writer.write_all(&chunk_data).await?;

        let cid = crate::utils::calculate_cid(&chunk_data);
        Ok((chunk_data.len(), cid))
    }

    pub async fn encode_padding_chunk<W: AsyncWrite + Unpin + Send>(
        rng: &mut impl CryptoRngCore,
        options: &DataOptions,
        writer: &mut W,
    ) -> std::io::Result<(usize, Cid)> {
        let mut chunk_data = vec![0; options.chunk_size()];
        rng.fill(chunk_data.as_mut_slice());

        writer.write_all(&chunk_data).await?;

        let cid = crate::utils::calculate_cid(&chunk_data);
        Ok((chunk_data.len(), cid))
    }
}
