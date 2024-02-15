#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeName(NodeNameInner);

impl NodeName {
    pub fn as_str(&self) -> &str {
        match &self.0 {
            NodeNameInner::Root => "{:root:}",
            NodeNameInner::Named(name) => name,
        }
    }

    pub(crate) fn named(name: String) -> Result<Self, NodeNameError> {
        if name.is_empty() {
            return Err(NodeNameError::Empty);
        }

        let byte_length = name.as_bytes().len();
        if byte_length > 255 {
            return Err(NodeNameError::TooLong(byte_length));
        }

        // some reserved names
        match name.as_str() {
            "." | ".." => return Err(NodeNameError::ReservedDirectoryTraversal),
            "{:root:}" => return Err(NodeNameError::ReservedRoot),
            _ => {}
        }

        // todo: extra validation, reserved names and characters etc..

        Ok(Self(NodeNameInner::Named(name)))
    }

    pub fn is_root(&self) -> bool {
        matches!(self.0, NodeNameInner::Root)
    }

    pub(crate) fn root() -> Self {
        Self(NodeNameInner::Root)
    }
}

impl std::convert::TryFrom<&str> for NodeName {
    type Error = NodeNameError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::named(value.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NodeNameError {
    #[error("name can't be empty")]
    Empty,

    #[error("name can't be '{{:root:}}' as it's reserved in the protocol")]
    ReservedRoot,

    #[error("both '.' nor '..' are directory traversal commands and can not be used as names")]
    ReservedDirectoryTraversal,

    #[error("name can be a maximum of 255 bytes, name was {0} bytes")]
    TooLong(usize),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum NodeNameInner {
    Root,
    Named(String),
}
