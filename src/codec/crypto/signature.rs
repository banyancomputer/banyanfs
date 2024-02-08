use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::IResult;
use p384::NistP384;

use crate::codec::AsyncEncodable;

const SIGNATURE_SIZE: usize = 96;

pub struct Signature {
    inner: ecdsa::Signature<NistP384>,
}

impl Signature {
    pub fn from_slice(slice: &[u8]) -> Result<Self, SignatureError> {
        let inner = ecdsa::Signature::from_slice(slice)?;
        Ok(Self { inner })
    }

    pub fn parse(_input: &[u8]) -> IResult<&[u8], Self> {
        todo!()
    }

    pub fn to_bytes(&self) -> [u8; SIGNATURE_SIZE] {
        let signature_bytes = self.inner.to_bytes();

        let mut signature = [0u8; SIGNATURE_SIZE];
        signature.copy_from_slice(&signature_bytes);

        signature
    }
}

#[async_trait]
impl AsyncEncodable for Signature {
    async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
        start_pos: usize,
    ) -> std::io::Result<usize> {
        let byte_ref = self.inner.to_bytes();
        writer.write_all(byte_ref.as_slice()).await?;
        Ok(start_pos + byte_ref.len())
    }
}

impl From<ecdsa::Signature<NistP384>> for Signature {
    fn from(inner: ecdsa::Signature<NistP384>) -> Self {
        Self { inner }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    #[error("invalid signature: {0}")]
    InvalidSignature(#[from] ecdsa::Error),
}
