use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::{tag, take};
use nom::number::streaming::le_u8;

use crate::codec::crypto::{AccessKey, SigningKey, VerifyingKey};
use crate::codec::header::BANYAN_DATA_MAGIC;
use crate::codec::{Cid, ParserResult};

const ENCRYPTED_BIT: u8 = 0b1000_0000;

const ECC_PRESENT_BIT: u8 = 0b0100_0000;

pub struct DataBlock {
    data_options: DataOptions,
    cid: Option<Cid>,

    consumed_chunks: usize,
    finalized: bool,

    contents: Vec<Content>,
}

impl DataBlock {
    pub fn add_data(&mut self, data: Vec<u8>) -> Result<(), DataBlockError> {
        if self.finalized {
            return Err(DataBlockError::AlreadyFinalized);
        }

        let mut data_len = data.len();
        if data_len > self.remaining_space() as usize {
            return Err(DataBlockError::Full);
        }

        // When allocating chunks we need to account for the data block's metadata overhead to
        // ensure we don't over-allocate chunks.
        if self.contents.is_empty() {
            data_len += Self::storage_overhead();
        }

        let chunk_capacity = self.data_options.block_size().chunk_capacity() as usize;
        let mut needed_chunks = data_len / chunk_capacity;
        if (data_len % chunk_capacity) > 0 {
            needed_chunks += 1;
        }
        self.consumed_chunks += needed_chunks;

        let chunk = Content::create_data(data);
        self.contents.push(chunk);

        Ok(())
    }

    pub fn cid(&self) -> Result<Cid, DataBlockError> {
        match &self.cid {
            Some(cid) => Ok(cid.clone()),
            None => Err(DataBlockError::NotFinalized),
        }
    }

    pub fn data_options(&self) -> DataOptions {
        self.data_options
    }

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        _access_key: &AccessKey,
        _signing_key: &SigningKey,
        _writer: &mut W,
    ) -> std::io::Result<usize> {
        if !self.finalized {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "block must be finalized before encoding",
            ));
        }

        // todo: need to set CID

        todo!()
    }

    pub fn finalize(&mut self) {
        self.finalized = true;
    }

    pub fn parse<'a>(
        input: &'a [u8],
        _access_key: &AccessKey,
        _verifying_key: &VerifyingKey,
    ) -> ParserResult<'a, Self> {
        let (input, version) = le_u8(input)?;

        if version != 0x01 {
            let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
            return Err(nom::Err::Failure(err));
        }

        let (input, _data_options) = DataOptions::parse(input)?;

        todo!()
    }

    pub fn parse_with_magic<'a>(
        input: &'a [u8],
        access_key: &AccessKey,
        verifying_key: &VerifyingKey,
    ) -> ParserResult<'a, Self> {
        let (input, _magic) = banyan_data_magic_tag(input)?;
        Self::parse(input, access_key, verifying_key)
    }

    pub fn remaining_space(&self) -> u64 {
        if self.contents.is_empty() {
            let empty_storage = self.data_options.block_size().storage_capacity();

            // The protocol overhead for the block consumes part of the first chunk, we represent
            // that by including those bytes in the initial data chunk.
            empty_storage - Self::storage_overhead() as u64
        } else {
            let total_chunks = self.data_options.block_size().chunk_count();
            let remaining_chunks = total_chunks - self.consumed_chunks as u64;
            let chunk_capacity = self.data_options.block_size().chunk_capacity();

            remaining_chunks * chunk_capacity
        }
    }

    pub fn small() -> Result<Self, DataBlockError> {
        let data_options = DataOptions {
            ecc_present: false,
            encrypted: true,
            block_size: BlockSize::small()?,
        };

        Ok(Self {
            data_options,
            cid: None,

            consumed_chunks: 0,
            finalized: false,

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
            cid: None,

            consumed_chunks: 0,
            finalized: false,

            contents: Vec::new(),
        })
    }

    const fn storage_overhead() -> usize {
        // todo: this is missing a few things, need to walk through the protocol and properly
        // account for things. Bugs from this will be annoying but fairly obvious.
        1 + Cid::size() + DataOptions::size() + 1
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DataBlockError {
    #[error("can't add more data once the block has been finalized")]
    AlreadyFinalized,

    #[error("block must be finalized and encoded before a CID is available")]
    NotFinalized,

    #[error("no space left in block")]
    Full,

    #[error("block size was invalid: {0}")]
    Size(#[from] BlockSizeError),
}

#[derive(Clone, Copy)]
pub struct DataOptions {
    ecc_present: bool,
    encrypted: bool,
    block_size: BlockSize,
}

impl DataOptions {
    pub fn block_size(&self) -> &BlockSize {
        &self.block_size
    }

    pub fn ecc_present(&self) -> bool {
        self.ecc_present
    }

    pub fn encrypted(&self) -> bool {
        self.encrypted
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, version_byte) = take(1u8)(input)?;
        let option_byte = version_byte[0];

        let ecc_present = (option_byte & ECC_PRESENT_BIT) == ECC_PRESENT_BIT;
        let encrypted = (option_byte & ENCRYPTED_BIT) == ENCRYPTED_BIT;
        let (input, block_size) = BlockSize::parse(input)?;

        let data_options = DataOptions {
            ecc_present,
            encrypted,
            block_size,
        };

        Ok((input, data_options))
    }

    pub const fn size() -> usize {
        2
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlockSize {
    /// The power of two exponent representing the total size of the block including metadata,
    /// format overhead, and error blocks.
    total_space: u8,

    /// The power of two exponent representing the encrypted chunk size within the block. Must be
    /// the same or smaller than the total space.
    chunk_size: u8,
}

impl BlockSize {
    pub fn chunk_capacity(&self) -> u64 {
        use crate::codec::crypto::{AuthenticationTag, Nonce};
        let per_chunk_overhead =
            Nonce::size() + Cid::size() + 4 /* length */ + AuthenticationTag::size();
        2u64.pow(self.chunk_size as u32) - per_chunk_overhead as u64
    }

    pub fn chunk_count(&self) -> u64 {
        2u64.pow((self.total_space - self.chunk_size) as u32)
    }

    /// Create a new instance of a BlockSize. Not exposed intentionally to limit the block sizes in
    /// use in the wild at this point in time.
    fn new(total_space: u8, chunk_size: u8) -> Result<Self, BlockSizeError> {
        if chunk_size > total_space {
            return Err(BlockSizeError::ChunkSizeTooLarge);
        }

        Ok(Self {
            total_space,
            chunk_size,
        })
    }

    pub fn parse(input: &[u8]) -> ParserResult<Self> {
        let (input, total_space) = le_u8(input)?;
        let (input, chunk_size) = le_u8(input)?;

        let block_size = Self {
            total_space,
            chunk_size,
        };

        Ok((input, block_size))
    }

    pub const fn size(&self) -> u64 {
        2
    }

    pub fn small() -> Result<Self, BlockSizeError> {
        Self::new(18, 18)
    }

    pub fn standard() -> Result<Self, BlockSizeError> {
        Self::new(26, 20)
    }

    /// Takes into account the overhead of encryption, does not account for format overhead or
    /// overhead of error blocks.
    pub fn storage_capacity(&self) -> u64 {
        self.chunk_capacity() * self.chunk_count()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BlockSizeError {
    #[error("provided chunk size was larger than block")]
    ChunkSizeTooLarge,
}

pub(crate) enum Content {
    Data(Vec<u8>),
}

impl Content {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Data(data) => data.as_slice(),
        }
    }

    pub(crate) fn cid(&self) -> Cid {
        crate::utils::calculate_cid(self.as_bytes())
    }

    pub(crate) fn create_data(data: Vec<u8>) -> Self {
        Self::Data(data)
    }

    pub(crate) async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        match self {
            Self::Data(data) => {
                let mut written_bytes = self.cid().encode(writer).await?;

                let length_bytes = data.len().to_le_bytes();
                writer.write_all(&length_bytes).await?;
                written_bytes += length_bytes.len();

                writer.write_all(data).await?;

                Ok(written_bytes + data.len())
            }
        }
    }

    pub(crate) fn parse(_input: &[u8]) -> ParserResult<Self> {
        todo!()
    }
}

fn banyan_data_magic_tag(input: &[u8]) -> ParserResult<&[u8]> {
    tag(BANYAN_DATA_MAGIC)(input)
}
