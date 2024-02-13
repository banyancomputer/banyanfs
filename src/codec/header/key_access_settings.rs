use async_trait::async_trait;
use futures::{AsyncWrite, AsyncWriteExt};
use nom::error::Error as NomError;
use nom::error::ErrorKind;
use nom::number::streaming::le_u8;

use crate::codec::AsyncEncodable;

const KEY_ACCESS_SETTINGS_PROTECTED_BIT: u8 = 0b1000_0000;

const KEY_ACCESS_SETTINGS_OWNER_BIT: u8 = 0b0100_0000;

const KEY_ACCESS_SETTINGS_HISTORICAL_BIT: u8 = 0b0010_0000;

const KEY_ACCESS_SETTINGS_PUBLIC_RESERVED_MASK: u8 = 0b0001_1111;

const KEY_ACCESS_SETTINGS_PRIVATE_ONLY_MASK: u8 = 0b0000_1111;

const KEY_ACCESS_SETTINGS_PRIVATE_RESERVED_MASK: u8 = 0b0001_0000;

const KEY_ACCESS_SETTINGS_REALIZED_KEY_PRESENT_BIT: u8 = 0b0000_1000;

const KEY_ACCESS_SETTINGS_DATA_KEY_PRESENT_BIT: u8 = 0b0100_0100;

const KEY_ACCESS_SETTINGS_JOURNAL_KEY_PRESENT_BIT: u8 = 0b0000_0010;

const KEY_ACCESS_SETTINGS_MAINT_KEY_PRESENT_BIT: u8 = 0b0000_0001;

pub struct KeyAccessSettingsBuilder {
    bits: u8,
    private: bool,
}

impl KeyAccessSettingsBuilder {
    pub fn set_owner(mut self) -> Self {
        self.bits |= KEY_ACCESS_SETTINGS_OWNER_BIT;
        self
    }

    pub fn set_protected(mut self) -> Self {
        self.bits |= KEY_ACCESS_SETTINGS_PROTECTED_BIT;
        self
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

    pub fn build(self) -> KeyAccessSettings {
        if self.private {
            KeyAccessSettings::Private {
                protected: self.bits & KEY_ACCESS_SETTINGS_PROTECTED_BIT != 0,
                owner: self.bits & KEY_ACCESS_SETTINGS_OWNER_BIT != 0,
                historical: self.bits & KEY_ACCESS_SETTINGS_HISTORICAL_BIT != 0,

                realized_key_present: self.bits & KEY_ACCESS_SETTINGS_REALIZED_KEY_PRESENT_BIT != 0,
                data_key_present: self.bits & KEY_ACCESS_SETTINGS_DATA_KEY_PRESENT_BIT != 0,
                journal_key_present: self.bits & KEY_ACCESS_SETTINGS_JOURNAL_KEY_PRESENT_BIT != 0,
                maintenance_key_present: self.bits & KEY_ACCESS_SETTINGS_MAINT_KEY_PRESENT_BIT != 0,
            }
        } else {
            KeyAccessSettings::Public {
                protected: self.bits & KEY_ACCESS_SETTINGS_PROTECTED_BIT != 0,
                owner: self.bits & KEY_ACCESS_SETTINGS_OWNER_BIT != 0,
                historical: self.bits & KEY_ACCESS_SETTINGS_HISTORICAL_BIT != 0,
                extra: self.bits & KEY_ACCESS_SETTINGS_PRIVATE_ONLY_MASK,
            }
        }
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

        realized_key_present: bool,
        data_key_present: bool,
        journal_key_present: bool,
        maintenance_key_present: bool,
    },
}

impl KeyAccessSettings {
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

    pub fn parse_private(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, byte) = le_u8(input)?;

        if cfg!(feature = "strict") && byte & KEY_ACCESS_SETTINGS_PRIVATE_RESERVED_MASK != 0 {
            return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Tag)));
        }

        let protected = byte & KEY_ACCESS_SETTINGS_PROTECTED_BIT != 0;
        let owner = byte & KEY_ACCESS_SETTINGS_OWNER_BIT != 0;
        let historical = byte & KEY_ACCESS_SETTINGS_HISTORICAL_BIT != 0;

        let realized_key_present = byte & KEY_ACCESS_SETTINGS_REALIZED_KEY_PRESENT_BIT != 0;
        let data_key_present = byte & KEY_ACCESS_SETTINGS_DATA_KEY_PRESENT_BIT != 0;
        let journal_key_present = byte & KEY_ACCESS_SETTINGS_JOURNAL_KEY_PRESENT_BIT != 0;
        let maintenance_key_present = byte & KEY_ACCESS_SETTINGS_MAINT_KEY_PRESENT_BIT != 0;

        let settings = Self::Private {
            protected,
            owner,
            historical,

            realized_key_present,
            data_key_present,
            journal_key_present,
            maintenance_key_present,
        };

        Ok((input, settings))
    }

    pub fn parse_public(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, byte) = le_u8(input)?;

        if cfg!(feature = "strict") && byte & KEY_ACCESS_SETTINGS_PUBLIC_RESERVED_MASK != 0 {
            return Err(nom::Err::Failure(NomError::new(input, ErrorKind::Tag)));
        }

        let protected = byte & KEY_ACCESS_SETTINGS_PROTECTED_BIT != 0;
        let owner = byte & KEY_ACCESS_SETTINGS_OWNER_BIT != 0;
        let historical = byte & KEY_ACCESS_SETTINGS_HISTORICAL_BIT != 0;
        let extra = byte & KEY_ACCESS_SETTINGS_PRIVATE_ONLY_MASK;

        let settings = Self::Public {
            protected,
            owner,
            historical,
            extra,
        };

        Ok((input, settings))
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
                    settings |= KEY_ACCESS_SETTINGS_PROTECTED_BIT;
                }

                if *owner {
                    settings |= KEY_ACCESS_SETTINGS_OWNER_BIT;
                }

                if *historical {
                    settings |= KEY_ACCESS_SETTINGS_HISTORICAL_BIT;
                }

                settings |= *extra & KEY_ACCESS_SETTINGS_PRIVATE_ONLY_MASK;
            }
            Self::Private {
                protected,
                owner,
                historical,
                realized_key_present,
                data_key_present,
                journal_key_present,
                maintenance_key_present,
            } => {
                if *protected {
                    settings |= KEY_ACCESS_SETTINGS_PROTECTED_BIT;
                }

                if *owner {
                    settings |= KEY_ACCESS_SETTINGS_OWNER_BIT;
                }

                if *historical {
                    settings |= KEY_ACCESS_SETTINGS_HISTORICAL_BIT;
                }

                if *realized_key_present {
                    settings |= KEY_ACCESS_SETTINGS_REALIZED_KEY_PRESENT_BIT;
                }

                if *data_key_present {
                    settings |= KEY_ACCESS_SETTINGS_DATA_KEY_PRESENT_BIT;
                }

                if *journal_key_present {
                    settings |= KEY_ACCESS_SETTINGS_JOURNAL_KEY_PRESENT_BIT;
                }

                if *maintenance_key_present {
                    settings |= KEY_ACCESS_SETTINGS_MAINT_KEY_PRESENT_BIT;
                }
            }
        }

        writer.write_all(&[settings]).await?;

        Ok(1)
    }
}
