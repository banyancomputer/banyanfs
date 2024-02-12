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

const KEY_ACCESS_SETTINGS_PRIVATE_RESERVED_MASK: u8 = 0b0001_0000;

const KEY_ACCESS_SETTINGS_REALIZED_KEY_PRESENT_BIT: u8 = 0b0000_1000;

const KEY_ACCESS_SETTINGS_DATA_KEY_PRESENT_BIT: u8 = 0b0100_0100;

const KEY_ACCESS_SETTINGS_JOURNAL_KEY_PRESENT_BIT: u8 = 0b0000_0010;

const KEY_ACCESS_SETTINGS_MAINTENANCE_KEY_PRESENT_BIT: u8 = 0b0000_0001;

#[derive(Clone)]
pub enum KeyAccessSettings {
    Public {
        protected: bool,
        owner: bool,
        historical: bool,
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
        let maintenance_key_present = byte & KEY_ACCESS_SETTINGS_MAINTENANCE_KEY_PRESENT_BIT != 0;

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

        let settings = Self::Public {
            protected,
            owner,
            historical,
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
                    settings |= KEY_ACCESS_SETTINGS_MAINTENANCE_KEY_PRESENT_BIT;
                }
            }
        }

        writer.write_all(&[settings]).await?;

        Ok(1)
    }
}
