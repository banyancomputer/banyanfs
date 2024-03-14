pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::new(), |acc, &b| format!("{acc}{:02x}", b))
}
