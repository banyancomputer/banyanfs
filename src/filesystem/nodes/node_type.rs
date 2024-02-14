use crate::filesystem::nodes::{Directory, File};

#[derive(Debug)]
pub enum NodeType {
    File(File),
    Directory(Directory),
}
