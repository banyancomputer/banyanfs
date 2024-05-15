use elliptic_curve::rand_core::CryptoRngCore;
use rand::Rng;
use winnow::token::take;
use winnow::Parser;
use winnow::Partial;

use super::{data_chunk::DataChunk, data_options::DataOptions};
use crate::codec::crypto::AccessKey;
use crate::codec::{Cid, ParserResult, Stream};

pub struct EncryptedDataChunk {
    contents: Box<[u8]>,
    cid: Cid,
}

impl EncryptedDataChunk {
    pub fn new(contents: Box<[u8]>, cid: Cid) -> Self {
        Self { contents, cid }
    }

    pub fn cid(&self) -> &Cid {
        &self.cid
    }

    pub fn data(&self) -> &[u8] {
        &self.contents
    }

    pub fn decrypt<'a>(
        &'a self,
        options: &DataOptions,
        access_key: &AccessKey,
    ) -> Result<DataChunk, DataChunkError> {
        let input = Partial::new(self.contents.as_ref());
        let (remaining, chunk) = DataChunk::parse(input, options, access_key)
            .map_err(|_| DataChunkError::ParseDecryptError)?;
        if !remaining.is_empty() {
            return Err(DataChunkError::ParseDecryptError);
        }
        Ok(chunk)
    }

    pub fn padding_chunk(rng: &mut impl CryptoRngCore, options: &DataOptions) -> Self {
        let mut chunk_data = vec![0; options.chunk_size()];
        rng.fill(chunk_data.as_mut_slice());

        let cid = crate::utils::calculate_cid(&chunk_data);
        Self::new(chunk_data.into_boxed_slice(), cid)
    }

    pub fn parse<'a>(input: Stream<'a>, options: &DataOptions) -> ParserResult<'a, Self> {
        if !options.encrypted() {
            unimplemented!("unencrypted data blocks are not yet supported");
        }
        let chunk_start = input;
        let (input, data) = take(options.chunk_size()).parse_peek(input)?;
        let cid = crate::utils::calculate_cid(data);

        Ok((input, Self::new(data.into(), cid)))
    }
}

pub enum DataChunkError {
    ParseDecryptError,
}
