use crate::filesystem::nodes::{Directory, File};

#[derive(Debug)]
pub enum Node {
    File(File),
    Directory(Directory),
}
