use futures::{AsyncWrite, AsyncWriteExt};
use p384::NistP384;
use winnow::{token::take, Parser};

use crate::codec::{ParserResult, Stream};

const SIGNATURE_SIZE: usize = 96;

pub struct Signature {
    inner: ecdsa::Signature<NistP384>,
}

impl Signature {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        let byte_ref = self.inner.to_bytes();
        writer.write_all(byte_ref.as_slice()).await?;
        Ok(byte_ref.len())
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, SignatureError> {
        let inner = ecdsa::Signature::from_slice(slice)?;
        Ok(Self { inner })
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        let (remaining, signature_bytes) = take(SIGNATURE_SIZE).parse_peek(input)?;
        let signature = match Signature::from_slice(signature_bytes) {
            Ok(signature) => signature,
            Err(_) => {
                let err = winnow::error::ParserError::from_error_kind(
                    &input,
                    winnow::error::ErrorKind::Verify,
                );
                return Err(winnow::error::ErrMode::Cut(err));
            }
        };

        Ok((remaining, signature))
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.inner.to_vec()
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
