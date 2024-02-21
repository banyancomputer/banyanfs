use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::number::streaming::le_u8;

use crate::codec::{AsyncEncodable, ParserResult};

const PROTECTED_BIT: u8 = 0b1000_0000;

const OWNER_BIT: u8 = 0b0100_0000;

const HISTORICAL_BIT: u8 = 0b0010_0000;

const FILESYSTEM_KEY_PRESENT_BIT: u8 = 0b0000_0100;

const DATA_KEY_PRESENT_BIT: u8 = 0b0000_0010;

const MAINTENANCE_KEY_PRESENT_BIT: u8 = 0b0000_0001;

const PUBLIC_RESERVED_MASK: u8 = 0b0001_1111;

const PRIVATE_ONLY_MASK: u8 = 0b0000_0111;

const PRIVATE_RESERVED_MASK: u8 = PUBLIC_RESERVED_MASK ^ PRIVATE_ONLY_MASK;

pub struct KeyAccessSettingsBuilder {
    bits: u8,
    private: bool,
}

impl KeyAccessSettingsBuilder {
    pub fn build(self) -> KeyAccessSettings {
        if self.private {
            KeyAccessSettings::Private {
                protected: self.bits & PROTECTED_BIT != 0,
                owner: self.bits & OWNER_BIT != 0,
                historical: self.bits & HISTORICAL_BIT != 0,

                filesystem_key_present: self.bits & FILESYSTEM_KEY_PRESENT_BIT != 0,
                data_key_present: self.bits & DATA_KEY_PRESENT_BIT != 0,
                maintenance_key_present: self.bits & MAINTENANCE_KEY_PRESENT_BIT != 0,
            }
        } else {
            KeyAccessSettings::Public {
                protected: self.bits & PROTECTED_BIT != 0,
                owner: self.bits & OWNER_BIT != 0,
                historical: self.bits & HISTORICAL_BIT != 0,
                extra: self.bits & PRIVATE_ONLY_MASK,
            }
        }
    }

    pub fn private() -> Self {
        Self {
            bits: 0,
            private: true,
        }
    }

    pub fn public() -> Self {
        Self {
            bits: 0,
            private: false,
        }
    }

    pub fn set_owner(mut self) -> Self {
        self.bits |= OWNER_BIT;
        self
    }

    pub fn set_protected(mut self) -> Self {
        self.bits |= PROTECTED_BIT;
        self
    }

    pub fn with_all_access(mut self) -> Self {
        self.bits |= FILESYSTEM_KEY_PRESENT_BIT;
        self.bits |= DATA_KEY_PRESENT_BIT;
        self.bits |= MAINTENANCE_KEY_PRESENT_BIT;

        self
    }
}

#[derive(Clone, Debug)]
pub enum KeyAccessSettings {
    Public {
        protected: bool,
        owner: bool,
        historical: bool,

        extra: u8,
    },
    Private {
        protected: bool,
        owner: bool,
        historical: bool,

        filesystem_key_present: bool,
        data_key_present: bool,
        maintenance_key_present: bool,
    },
}

impl KeyAccessSettings {
    pub fn has_data_key(&self) -> bool {
        match self {
            KeyAccessSettings::Public { .. } => false,
            KeyAccessSettings::Private {
                data_key_present, ..
            } => *data_key_present,
        }
    }

    pub fn has_filesystem_key(&self) -> bool {
        match self {
            KeyAccessSettings::Public { .. } => false,
            KeyAccessSettings::Private {
                filesystem_key_present,
                ..
            } => *filesystem_key_present,
        }
    }

    pub fn has_maintenance_key(&self) -> bool {
        match self {
            KeyAccessSettings::Public { .. } => false,
            KeyAccessSettings::Private {
                maintenance_key_present,
                ..
            } => *maintenance_key_present,
        }
    }

    pub fn is_historical(&self) -> bool {
        match self {
            KeyAccessSettings::Public { historical, .. } => *historical,
            KeyAccessSettings::Private { historical, .. } => *historical,
        }
    }

    pub fn is_owner(&self) -> bool {
        match self {
            KeyAccessSettings::Public { owner, .. } => *owner,
            KeyAccessSettings::Private { owner, .. } => *owner,
        }
    }

    pub fn is_protected(&self) -> bool {
        match self {
            KeyAccessSettings::Public { protected, .. } => *protected,
            KeyAccessSettings::Private { protected, .. } => *protected,
        }
    }

    pub fn parse_private(input: &[u8]) -> ParserResult<Self> {
        let (input, byte) = le_u8(input)?;

        if cfg!(feature = "strict") && byte & PRIVATE_RESERVED_MASK != 0 {
            let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
            return Err(nom::Err::Failure(err));
        }

        let protected = byte & PROTECTED_BIT != 0;
        let owner = byte & OWNER_BIT != 0;
        let historical = byte & HISTORICAL_BIT != 0;

        let filesystem_key_present = byte & FILESYSTEM_KEY_PRESENT_BIT != 0;
        let data_key_present = byte & DATA_KEY_PRESENT_BIT != 0;
        let maintenance_key_present = byte & MAINTENANCE_KEY_PRESENT_BIT != 0;

        let settings = Self::Private {
            protected,
            owner,
            historical,

            filesystem_key_present,
            data_key_present,
            maintenance_key_present,
        };

        Ok((input, settings))
    }

    pub fn parse_public(input: &[u8]) -> ParserResult<Self> {
        let (input, byte) = le_u8(input)?;

        if cfg!(feature = "strict") && byte & PUBLIC_RESERVED_MASK != 0 {
            let err = nom::error::make_error(input, nom::error::ErrorKind::Verify);
            return Err(nom::Err::Failure(err));
        }

        let protected = byte & PROTECTED_BIT != 0;
        let owner = byte & OWNER_BIT != 0;
        let historical = byte & HISTORICAL_BIT != 0;
        let extra = byte & PRIVATE_ONLY_MASK;

        let settings = Self::Public {
            protected,
            owner,
            historical,
            extra,
        };

        Ok((input, settings))
    }

    pub const fn size() -> usize {
        1
    }
}

#[async_trait]
impl AsyncEncodable for KeyAccessSettings {
    async fn encode<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut settings: u8 = 0x00;

        match self {
            Self::Public {
                protected,
                owner,
                historical,
                extra,
            } => {
                if *protected {
                    settings |= PROTECTED_BIT;
                }

                if *owner {
                    settings |= OWNER_BIT;
                }

                if *historical {
                    settings |= HISTORICAL_BIT;
                }

                settings |= *extra & PRIVATE_ONLY_MASK;
            }
            Self::Private {
                protected,
                owner,
                historical,
                filesystem_key_present,
                data_key_present,
                maintenance_key_present,
            } => {
                if *protected {
                    settings |= PROTECTED_BIT;
                }

                if *owner {
                    settings |= OWNER_BIT;
                }

                if *historical {
                    settings |= HISTORICAL_BIT;
                }

                if *filesystem_key_present {
                    settings |= FILESYSTEM_KEY_PRESENT_BIT;
                }

                if *data_key_present {
                    settings |= DATA_KEY_PRESENT_BIT;
                }

                if *maintenance_key_present {
                    settings |= MAINTENANCE_KEY_PRESENT_BIT;
                }
            }
        }

        writer.write_all(&[settings]).await?;

        Ok(1)
    }
}
