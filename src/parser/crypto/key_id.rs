use std::ops::Deref;

#[derive(Clone, Copy)]
pub(crate) struct KeyId(u32);

impl Deref for KeyId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u32> for KeyId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}
