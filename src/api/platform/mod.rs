//! API endpoints, helpers, and models for the core Banyan platform.

mod models;

pub mod account;
pub mod blocks;
pub mod drive_keys;
pub mod drives;
pub mod metadata;
pub mod snapshots;

pub use models::*;

/// This is the public key used by the Banyan platform to access a filesystem's metadata. This is
/// granted maintenance level privileges within the format which only allowed for knowledge of the
/// other keys being present. It does not grant a view of the filesystem structure, attributes, or
/// the data contents themselves.
///
/// When the CRDT and journaling system is in place, this will also grant access to knowledge about
/// which blocks of data are associated with an individual update and which ones can be safely
/// discarded.
///
/// This distinction allows deniability at the filesystem level of who can access the data (which
/// Banyan will learn by being a minimal privilege party) while still providing the minimal
/// information required to host and maintain versioned data over time.
pub const PLATFORM_MAINTENANCE_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAE00jlH0iG12Lq1tuCwom4ma2dwZ/56Oxf
Yl0LDsPoeNDWOuFDBtjvJRQllusFrIpEKJY+nRLq+Px+dlqtKlL4yD/0IVRcqYt/
9mdrZWJ4KqrEUuRnYtNiPeCrfiKRqfjA
-----END PUBLIC KEY-----"#;
