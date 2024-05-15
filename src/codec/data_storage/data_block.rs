use elliptic_curve::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use winnow::binary::le_u8;
use winnow::token::literal;
use winnow::Parser;

use super::data_options::DataOptions;
use super::encrypted_data_chunk::EncryptedDataChunk;
use crate::codec::crypto::{AuthenticationTag, Nonce};
use crate::codec::header::BANYAN_DATA_MAGIC;
use crate::codec::meta::{BlockSize, BlockSizeError};
use crate::codec::{Cid, ParserResult, Stream};
use crate::utils::std_io_err;

use std::sync::{Arc, RwLock};

pub struct DataBlock {
    data_options: DataOptions,
    cid: Arc<RwLock<Option<Cid>>>,
    contents: Vec<EncryptedDataChunk>,
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
        writer: &mut W,
    ) -> std::io::Result<(usize, Vec<Cid>)> {
        if self.data_options.ecc_present() {
            unimplemented!("parity blocks are not yet supported");
        }

        if !self.data_options.encrypted() {
            unimplemented!("unencrypted data blocks are not yet supported");
        }

        let mut header_data = Vec::new();

        header_data.write_all(BANYAN_DATA_MAGIC).await?;
        header_data.write_all(&[0x01]).await?;

        let mut data_buffer = Vec::new();
        let mut chunk_cids = Vec::new();

        for chunk in self.contents.iter() {
            data_buffer.extend_from_slice(chunk.data());
            chunk_cids.push(chunk.cid().clone());
        }

        // Pad our data block
        let needed_chunks = self.max_chunk_count() - self.contents.len();
        for _ in 0..needed_chunks {
            let padding_chunk = EncryptedDataChunk::padding_chunk(rng, &self.data_options);
            data_buffer.extend_from_slice(padding_chunk.data());
            chunk_cids.push(padding_chunk.cid().clone());
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
        cid.encode(&mut header_data).await?;
        self.data_options.encode(&mut header_data).await?;

        // This mutex is very picky, need to put it in its own scope
        {
            let mut inner_cid = self
                .cid
                .write()
                .map_err(|_| std_io_err("cid lock was poisoned"))?;
            inner_cid.replace(cid);
        }

        writer.write_all(&header_data).await?;
        let mut written_bytes = header_data.len();

        writer.write_all(&data_buffer).await?;
        written_bytes += data_buffer.len();

        Ok((written_bytes, chunk_cids))
    }

    pub fn get_chunk(&self, index: usize) -> Result<&EncryptedDataChunk, DataBlockError> {
        self.contents
            .get(index)
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

    pub fn parse<'a>(input: Stream<'a>) -> ParserResult<'a, Self> {
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

        let chunk_count = data_options.block_size().chunk_count() as usize;
        let mut contents = Vec::with_capacity(chunk_count);
        let mut input = input;
        for _ in 0..chunk_count {
            let (remaining, chunk) = EncryptedDataChunk::parse(input, &data_options)?;
            input = remaining;
            contents.push(chunk);
        }

        let mut chunk_cids = Vec::with_capacity(chunk_count);
        for _ in 0..chunk_count {
            let (remaining, cid) = Cid::parse(input)?;
            input = remaining;
            chunk_cids.push(cid);
        }

        // todo(sstelfox): not doing anything with the cids...
        // note(jason): should compare cids from EncryptedDataChunk to these to make sure they agree

        let block = Self {
            data_options,
            cid: Arc::new(RwLock::new(Some(cid))),
            contents,
        };

        Ok((input, block))
    }

    pub fn parse_with_magic<'a>(input: Stream<'a>) -> ParserResult<'a, Self> {
        let (input, _magic) = banyan_data_magic_tag(input)?;
        Self::parse(input)
    }

    pub fn push_chunk(&mut self, chunk: EncryptedDataChunk) -> Result<(), DataBlockError> {
        if self.is_full() {
            return Err(DataBlockError::Full);
        }

        let mut inner_cid = self.cid.write().map_err(|_| DataBlockError::LockPoisoned)?;
        inner_cid.take();
        drop(inner_cid);

        self.contents.push(chunk);

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
