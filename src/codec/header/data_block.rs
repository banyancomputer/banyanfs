use elliptic_curve::rand_core::CryptoRngCore;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::bytes::streaming::{tag, take};
use nom::number::streaming::le_u8;

use crate::codec::crypto::{AccessKey, AuthenticationTag, Nonce, SigningKey, VerifyingKey};

use crate::codec::header::BANYAN_DATA_MAGIC;
use crate::codec::meta::{BlockSize, BlockSizeError};
use crate::codec::{Cid, ParserResult};
use crate::utils::std_io_err;

const ENCRYPTED_BIT: u8 = 0b1000_0000;

const ECC_PRESENT_BIT: u8 = 0b0100_0000;

use std::sync::{Arc, RwLock};

pub struct DataBlock {
    data_options: DataOptions,
    cid: Arc<RwLock<Option<Cid>>>,
    contents: Vec<Vec<u8>>,
}

impl DataBlock {
    pub fn add_chunk(&mut self, data: Vec<u8>) -> Result<(), DataBlockError> {
        if self.is_full() {
            return Err(DataBlockError::Full);
        }

        let expected_chunk_size = self.chunk_size();
        if data.len() != expected_chunk_size {
            return Err(DataBlockError::Size(BlockSizeError::ChunkSizeMismatch(
                data.len(),
                expected_chunk_size,
            )));
        }

        let mut inner_cid = self.cid.write().map_err(|_| DataBlockError::LockPoisoned)?;
        inner_cid.take();
        drop(inner_cid);

        self.contents.push(data);

        Ok(())
    }

    pub fn base_chunk_size(&self) -> usize {
        self.data_options().block_size().chunk_size() as usize
    }

    pub fn chunk_size(&self) -> usize {
        let mut base_chunk_size = self.base_chunk_size();

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
    ) -> std::io::Result<usize> {
        if !self.is_full() {
            todo!("pad the block");
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

        for chunk in self.contents.iter() {
            if chunk.len() != self.chunk_size() {
                return Err(std_io_err("chunk size mismatch"));
            }

            let mut payload = chunk.clone();
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
        for cid in chunk_cids {
            cid.encode(&mut data_buffer).await?;
        }

        // The data block CID is only over the data payload, the rest of the
        // data is signed.
        let cid = crate::utils::calculate_cid(&data_buffer);
        cid.encode(&mut signed_data).await?;
        self.data_options.encode(&mut signed_data).await?;

        let mut inner_cid = self
            .cid
            .write()
            .map_err(|_| std_io_err("cid lock was poisoned"))?;
        inner_cid.replace(cid);
        drop(inner_cid);

        writer.write_all(&signed_data).await?;
        let mut written_bytes = signed_data.len();

        let signature = signing_key.sign(rng, &signed_data);
        written_bytes += signature.encode(writer).await?;

        writer.write(&data_buffer).await?;
        written_bytes += data_buffer.len();

        Ok(written_bytes)
    }

    pub fn get_chunk_data(&self, index: usize) -> Result<&[u8], DataBlockError> {
        self.contents
            .get(index)
            .map(|chunk| chunk.as_slice())
            .ok_or(DataBlockError::ChunkIndexOutOfBounds)
    }

    pub fn is_full(&self) -> bool {
        self.data_options().block_size().chunk_count() >= self.contents.len() as u64
    }

    pub fn parse<'a>(
        input: &'a [u8],
        access_key: &AccessKey,
        _verifying_key: &VerifyingKey,
    ) -> ParserResult<'a, Self> {
        let (input, version) = le_u8(input)?;

        if version != 0x01 {
            let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
            return Err(nom::Err::Failure(err));
        }

        let (input, cid) = Cid::parse(input)?;
        let (input, data_options) = DataOptions::parse(input)?;

        if !data_options.ecc_present() {
            unimplemented!("ecc encoding is not yet supported");
        }

        if !data_options.encrypted() {
            unimplemented!("unencrypted data blocks are not yet supported");
        }

        let chunks = data_options.block_size().chunk_count() as usize;
        let chunk_size = data_options.block_size().chunk_size() as usize;
        let overhead = AuthenticationTag::size() + Nonce::size();

        // todo(sstelfox): I'm not yet verifying the data block's signature here yet, I'll have to
        // do a buffer indirection to make that work and its too late for those games.

        let mut contents = Vec::with_capacity(chunks);
        for _ in 0..chunks {
            let (input, chunk_data) = take(chunk_size)(input)?;

            let (chunk_data, nonce) = Nonce::parse(chunk_data)?;
            let (chunk_data, data) = take(chunk_size - overhead)(chunk_data)?;
            let (chunk_data, tag) = AuthenticationTag::parse(chunk_data)?;
            debug_assert!(chunk_data.is_empty(), "chunk should be fully read");

            let mut plaintext_data = data.to_vec();
            if let Err(err) = access_key.decrypt_buffer(nonce, &[], &mut plaintext_data, tag) {
                tracing::error!("failed to decrypt chunk: {err}");
                let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
                return Err(nom::Err::Failure(err));
            }

            contents.push(plaintext_data);
        }

        let block = Self {
            data_options,
            cid: Arc::new(RwLock::new(Some(cid))),
            contents,
        };

        Ok((input, block))
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

    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let mut option_byte = 0u8;

        if self.ecc_present {
            option_byte |= ECC_PRESENT_BIT;
        }

        if self.encrypted {
            option_byte |= ENCRYPTED_BIT;
        }

        writer.write_all(&[option_byte]).await?;
        let block_size = self.block_size.encode(writer).await?;

        Ok(1 + block_size)
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

fn banyan_data_magic_tag(input: &[u8]) -> ParserResult<&[u8]> {
    tag(BANYAN_DATA_MAGIC)(input)
}
