use elliptic_curve::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use rand::Rng;
use winnow::binary::{le_u64, le_u8};
use winnow::error::ErrMode;
use winnow::stream::Offset;
use winnow::token::{literal, take};
use winnow::Parser;

use super::data_options::DataOptions;
use crate::codec::crypto::{
    AccessKey, AuthenticationTag, Nonce, Signature, SigningKey, VerifyingKey,
};
use crate::codec::header::BANYAN_DATA_MAGIC;
use crate::codec::meta::{BlockSize, BlockSizeError};
use crate::codec::{Cid, ParserResult, Stream};
use crate::utils::std_io_err;

use std::sync::{Arc, RwLock};

pub struct DataBlock {
    data_options: DataOptions,
    cid: Arc<RwLock<Option<Cid>>>,
    contents: Vec<Vec<u8>>,
}

impl DataBlock {
    /// The amount of payload a small encrypted block can hold.
    #[allow(clippy::identity_op)]
    pub const SMALL_ENCRYPTED_SIZE: usize =
        262_144 - (1 * Nonce::size() + AuthenticationTag::size());

    /// The amount of payload a standard encrypted block can hold.
    pub const STANDARD_ENCRYPTED_SIZE: usize =
        134_217_728 - (64 * Nonce::size() + AuthenticationTag::size());

    pub fn base_chunk_size(&self) -> usize {
        self.data_options().chunk_size() as usize
    }

    pub fn chunk_size(&self) -> usize {
        let mut base_chunk_size = self.base_chunk_size();

        // length bytes
        base_chunk_size -= 8;

        if self.data_options().encrypted() {
            base_chunk_size -= Nonce::size();
            base_chunk_size -= AuthenticationTag::size();
        }

        base_chunk_size
    }

    pub fn cid(&self) -> Result<Cid, DataBlockError> {
        let inner_cid = self.cid.read().map_err(|_| DataBlockError::LockPoisoned)?;

        match &*inner_cid {
            Some(cid) => Ok(cid.clone()),
            None => Err(DataBlockError::EncodingRequired),
        }
    }

    pub fn data_options(&self) -> DataOptions {
        self.data_options
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        rng: &mut impl CryptoRngCore,
        access_key: &AccessKey,
        signing_key: &SigningKey,
        writer: &mut W,
    ) -> std::io::Result<(usize, Vec<Cid>)> {
        // Pad our data block
        let mut extra_chunks = Vec::new();
        let needed_chunks = self.max_chunk_count() - self.contents.len();

        for _ in 0..needed_chunks {
            let mut padding = Vec::with_capacity(self.chunk_size());
            rng.fill_bytes(&mut padding);
            extra_chunks.push(padding);
        }

        if self.data_options.ecc_present() {
            unimplemented!("parity blocks are not yet supported");
        }

        if !self.data_options.encrypted() {
            unimplemented!("unencrypted data blocks are not yet supported");
        }

        let mut signed_data = Vec::new();

        signed_data.write_all(BANYAN_DATA_MAGIC).await?;
        signed_data.write_all(&[0x01]).await?;

        // todo(sstelfox): I should move all the chunk encoding, encrypting, decrypting, and
        // decoding into its own type to encapsulate that information but also to switch this in an
        // enum over encrypted and decrypted types. I want to be able to decode a single chunk
        // without all of them.
        let mut data_buffer = Vec::new();
        let mut chunk_cids = Vec::new();

        for chunk in self.contents.iter().chain(&extra_chunks) {
            if chunk.len() > self.chunk_size() {
                tracing::error!(true_length = ?chunk.len(), max_length = self.chunk_size(), "chunk too large");
                return Err(std_io_err("chunk size mismatch (chunk too large)"));
            }

            // write out the true data length
            let chunk_length = chunk.len() as u64;
            let chunk_length_bytes = chunk_length.to_le_bytes();

            // We need to prepend the length of the data, the pad the remaining space with random
            // data.
            let full_size = self.chunk_size() + 8;
            let mut payload = Vec::with_capacity(full_size);
            payload.extend_from_slice(&chunk_length_bytes);
            payload.extend_from_slice(chunk);
            payload.resize_with(full_size, || rng.gen());

            let (nonce, tag) = access_key
                .encrypt_buffer(rng, &[], &mut payload)
                .map_err(|_| std_io_err("failed to encrypt chunk"))?;

            let mut chunk_data = Vec::with_capacity(self.base_chunk_size());

            nonce.encode(&mut chunk_data).await?;
            chunk_data.write_all(&payload).await?;
            tag.encode(&mut chunk_data).await?;

            let cid = crate::utils::calculate_cid(&chunk_data);

            data_buffer.extend(chunk_data);
            chunk_cids.push(cid);
        }

        // We include a map of the chunks and their CIDs in a trailer at the end of the block. This
        // gets captured by the data block CID. We know how many entries there are, and the
        // index of each chunk matches its respective block so we can just write them straight out.
        for cid in &chunk_cids {
            cid.encode(&mut data_buffer).await?;
        }

        // The data block CID is only over the data payload, the rest of the
        // data is signed.
        let cid = crate::utils::calculate_cid(&data_buffer);
        cid.encode(&mut signed_data).await?;
        self.data_options.encode(&mut signed_data).await?;

        // This mutex is very picky, need to put it in its own scope
        {
            let mut inner_cid = self
                .cid
                .write()
                .map_err(|_| std_io_err("cid lock was poisoned"))?;
            inner_cid.replace(cid);
        }

        writer.write_all(&signed_data).await?;
        let mut written_bytes = signed_data.len();

        let signature = signing_key.sign(rng, &signed_data);
        written_bytes += signature.encode(writer).await?;

        writer.write_all(&data_buffer).await?;
        written_bytes += data_buffer.len();

        Ok((written_bytes, chunk_cids))
    }

    pub fn get_chunk_data(&self, index: usize) -> Result<&[u8], DataBlockError> {
        self.contents
            .get(index)
            .map(|chunk| chunk.as_slice())
            .ok_or(DataBlockError::ChunkIndexOutOfBounds)
    }

    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }

    pub fn max_chunk_count(&self) -> usize {
        self.data_options().block_size().chunk_count() as usize
    }

    pub fn is_full(&self) -> bool {
        self.contents.len() >= self.max_chunk_count()
    }

    pub fn parse<'a>(
        input: Stream<'a>,
        access_key: &AccessKey,
        _verifying_key: &VerifyingKey,
    ) -> ParserResult<'a, Self> {
        let (input, version) = le_u8.parse_peek(input)?;

        if version != 0x01 {
            let err = winnow::error::ParserError::from_error_kind(
                &input,
                winnow::error::ErrorKind::Verify,
            );
            return Err(winnow::error::ErrMode::Cut(err));
        }

        let (input, cid) = Cid::parse(input)?;
        let (input, data_options) = DataOptions::parse(input)?;

        if data_options.ecc_present() {
            unimplemented!("ecc encoding is not yet supported");
        }

        if !data_options.encrypted() {
            unimplemented!("unencrypted data blocks are not yet supported");
        }

        // todo(sstelfox): I'm not yet verifying the data block's signature here yet
        let (input, _signature) = Signature::parse(input)?;

        let chunk_count = data_options.block_size().chunk_count() as usize;

        let base_chunk_size = data_options.chunk_size();
        let encrypted_chunk_size = data_options.encrypted_chunk_data_size() + 8;

        let mut contents = Vec::with_capacity(chunk_count);
        for _ in 0..chunk_count {
            let chunk_start = input;

            let (input, nonce) = Nonce::parse(input)?;
            let (input, data) = take(encrypted_chunk_size).parse_peek(input)?;
            let (input, tag) = AuthenticationTag::parse(input)?;

            debug_assert!(
                input.offset_from(&chunk_start) == base_chunk_size,
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

            contents.push(plaintext_data);
        }

        let mut chunk_cids = Vec::with_capacity(chunk_count);
        let mut input = input;
        for _ in 0..chunk_count {
            let (remaining, cid) = Cid::parse(input)?;
            input = remaining;
            chunk_cids.push(cid);
        }

        // todo(sstelfox): not doing anything with the cids...

        let block = Self {
            data_options,
            cid: Arc::new(RwLock::new(Some(cid))),
            contents,
        };

        Ok((input, block))
    }

    pub fn parse_with_magic<'a>(
        input: Stream<'a>,
        access_key: &AccessKey,
        verifying_key: &VerifyingKey,
    ) -> ParserResult<'a, Self> {
        let (input, _magic) = banyan_data_magic_tag(input)?;
        Self::parse(input, access_key, verifying_key)
    }

    pub fn push_chunk(&mut self, data: Vec<u8>) -> Result<(), DataBlockError> {
        if self.is_full() {
            return Err(DataBlockError::Full);
        }

        let max_chunk_size = self.chunk_size();
        if data.len() > max_chunk_size {
            return Err(DataBlockError::Size(BlockSizeError::ChunkTooLarge(
                data.len(),
                max_chunk_size,
            )));
        }

        let mut inner_cid = self.cid.write().map_err(|_| DataBlockError::LockPoisoned)?;
        inner_cid.take();
        drop(inner_cid);

        self.contents.push(data);

        Ok(())
    }

    pub fn remaining_space(&self) -> u64 {
        let block_size = self.data_options.block_size();

        let total_chunks = block_size.chunk_count();
        let remaining_chunks = total_chunks - self.contents.len() as u64;

        remaining_chunks * self.chunk_size() as u64
    }

    pub fn small() -> Result<Self, DataBlockError> {
        let data_options = DataOptions {
            ecc_present: false,
            encrypted: true,
            block_size: BlockSize::small()?,
        };

        Ok(Self {
            data_options,
            cid: Arc::new(RwLock::new(None)),

            contents: Vec::new(),
        })
    }

    pub fn standard() -> Result<Self, DataBlockError> {
        let data_options = DataOptions {
            ecc_present: false,
            encrypted: true,
            block_size: BlockSize::small()?,
        };

        Ok(Self {
            data_options,
            cid: Arc::new(RwLock::new(None)),
            contents: Vec::new(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DataBlockError {
    #[error("requested chunk index that isn't in the block")]
    ChunkIndexOutOfBounds,

    #[error("CID's are not available until after the block has been encoded")]
    EncodingRequired,

    #[error("no space left in block")]
    Full,

    #[error("cid lock was poisoned")]
    LockPoisoned,

    #[error("block size was invalid: {0}")]
    Size(#[from] BlockSizeError),
}

fn banyan_data_magic_tag(input: Stream) -> ParserResult<&[u8]> {
    literal(BANYAN_DATA_MAGIC).parse_peek(input)
}
