pub enum ProgressType<T> {
    Ready(usize, T),
    Advance(usize),
}
