use super::data_options::{DataOptions, DataOptionsError};
use super::encrypted_data_chunk::EncryptedDataChunk;
use super::DataBlockError;
use crate::codec::crypto::AccessKey;
use crate::utils::std_io_err;

use elliptic_curve::rand_core::CryptoRngCore;
use rand::Rng;

pub struct DataChunk {
    contents: Box<[u8]>,
}

impl DataChunk {
    pub fn from_slice(data: &[u8], options: &DataOptions) -> Result<Self, DataBlockError> {
        if data.len() > options.chunk_data_size() {
            return Err(DataBlockError::OptionsError(
                DataOptionsError::ChunkTooLarge(data.len(), options.chunk_data_size()),
            ));
        }
        Ok(Self {
            contents: data.into(),
        })
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
        if !options.encrypted {
            //should probably actually be an error "can't encrypt chunk for unenrypted options" or similar
            unimplemented!("unencrypted data blocks are not yet supported");
        }
        if self.contents.len() > options.chunk_data_size() {
            tracing::error!(true_length = ?self.contents.len(), max_length = options.chunk_data_size(), "chunk too large");
            return Err(std_io_err("chunk size mismatch (chunk too large)"));
        }

        // write out the true data length
        let chunk_length = self.contents.len() as u32;
        let chunk_length_bytes = chunk_length.to_le_bytes();

        // We need to prepend the length of the data, the pad the remaining space with random
        // data.
        let full_size = options.chunk_payload_size();
        let mut payload = Vec::with_capacity(full_size);
        payload.extend_from_slice(&chunk_length_bytes);
        payload.extend_from_slice(self.data());
        payload.resize_with(full_size, || rng.gen());

        let (nonce, tag) = access_key
            .encrypt_buffer(rng, &[], &mut payload)
            .map_err(|_| std_io_err("failed to encrypt chunk"))?;

        Ok(EncryptedDataChunk::new(
            nonce,
            payload.into_boxed_slice(),
            tag,
        ))
    }
}
