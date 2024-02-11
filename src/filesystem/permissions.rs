#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    creator_write_only: bool,
    executable: bool,
    immutable: bool,
}

impl Permissions {
    pub fn creator_write_only(&self) -> bool {
        self.creator_write_only
    }

    pub fn executable(&self) -> bool {
        self.executable
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }
}
