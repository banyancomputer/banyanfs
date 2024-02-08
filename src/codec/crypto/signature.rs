use p384::NistP384;

pub struct Signature {
    inner: ecdsa::Signature<NistP384>,
}

impl Signature {
    pub(crate) fn from_slice(slice: &[u8]) -> Result<Self, SignatureError> {
        let inner = ecdsa::Signature::from_slice(slice)?;
        Ok(Self { inner })
    }

    pub(crate) fn to_bytes(&self) -> [u8; 96] {
        let signature_bytes = self.inner.to_bytes();

        let mut signature = [0u8; 96];
        signature.copy_from_slice(&signature_bytes);

        signature
    }
}

impl From<ecdsa::Signature<NistP384>> for Signature {
    fn from(inner: ecdsa::Signature<NistP384>) -> Self {
        Self { inner }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SignatureError {
    #[error("invalid signature: {0}")]
    InvalidSignature(#[from] ecdsa::Error),
}
