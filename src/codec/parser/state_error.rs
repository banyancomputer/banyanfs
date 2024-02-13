pub trait StateError {
    fn needed_data(&self) -> Option<usize>;

    fn needs_more_data(&self) -> bool;
}
