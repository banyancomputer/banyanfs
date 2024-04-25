use futures::AsyncWrite;

use crate::codec::crypto::{Fingerprint, KeyId};
use crate::codec::{ParserResult, Stream};

/// Actors are unique parties for accessing a specific drive and are uniquely defined by the
/// fingerprint from the Actor's public key (via a [`VerifyingKey`]).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct ActorId(Fingerprint);

impl ActorId {
    pub async fn encode<W: AsyncWrite + Unpin + Send>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<usize> {
        self.0.encode(writer).await
    }

    /// Access the actor's [`KeyId`]. This is intended for filtering available keys down to some
    /// reasonable possible candidates and should not be used to uniquely identify the actor as a
    /// whole. For unique actor identification, the full [`ActorId`] should be used.
    pub fn key_id(&self) -> KeyId {
        self.0.key_id()
    }

    pub fn parse(input: Stream) -> ParserResult<Self> {
        Fingerprint::parse(input)
            .map(|(remaining, fingerprint)| (remaining, Self::from(fingerprint)))
    }

    pub const fn size() -> usize {
        Fingerprint::size()
    }
}

impl From<Fingerprint> for ActorId {
    fn from(fingerprint: Fingerprint) -> Self {
        Self(fingerprint)
    }
}

#[cfg(test)]
mod tests {
    use elliptic_curve::rand_core::CryptoRngCore;

    use super::*;

    impl ActorId {
        pub(crate) fn arbitrary(rng: &mut impl CryptoRngCore) -> Self {
            Self(Fingerprint::arbitrary(rng))
        }
    }
}
