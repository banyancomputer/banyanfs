use crate::prelude::nodes::NodeName;

#[derive(Default)]
pub struct MimeGuesser {
    name: Option<String>,
    data: Vec<u8>,
}

impl MimeGuesser {
    pub fn with_name(mut self, name: NodeName) -> Self {
        match name {
            NodeName::Named(name) => self.name = Some(name.clone()),
            NodeName::Root => {}
        }
        self
    }

    pub fn with_data(mut self, data: &[u8]) -> Self {
        self.data.extend_from_slice(data);
        self
    }

    pub fn guess_mime_type(&self) -> Option<mime::MediaType> {
        self.pattern_match()
            .or_else(|| self.algorithm_match())
            .or_else(|| self.extension_match())
    }

    fn extension_match(&self) -> Option<mime::MediaType> {
        if let Some(name) = self.name.as_ref() {
            let guess = mime_guess::from_path(name);
            if !guess.is_empty() {
                return mime::MediaType::parse(guess.first()?.as_ref()).ok();
            }
        }
        None
    }

    fn pattern_match(&self) -> Option<mime::MediaType> {
        let magic_bytes = &self.data[..];

        // Taken from https://mimesniff.spec.whatwg.org/
        match magic_bytes {
            &[0xFF, 0xD8, 0xFF, ..] => Some(mime::IMAGE_JPEG),
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => Some(mime::IMAGE_PNG),
            &[b'G', b'I', b'F', b'8', b'7', b'a', ..]
            | &[b'G', b'I', b'F', b'8', b'9', b'a', ..] => Some(mime::IMAGE_GIF),
            &[b'B', b'M', ..] => Some(mime::IMAGE_BMP),
            &[b'<', b'?', b'x', b'm', b'l', ..] => Some(mime::TEXT_XML),
            &[b'<', b's', b'v', b'g', ..] => Some(mime::IMAGE_SVG),
            &[b'w', b'O', b'F', b'F', ..] => Some(mime::FONT_WOFF),
            &[b'w', b'O', b'F', b'2', ..] => Some(mime::FONT_WOFF2),
            &[b'%', b'P', b'D', b'F', b'-', ..] => Some(mime::APPLICATION_PDF),
            &[b'{', ..] => Some(mime::APPLICATION_JSON),
            &[b'F', b'O', b'R', b'M', _, _, _, _, b'A', b'I', b'F', b'F', ..] => {
                Some(mime::AUDIO_AIFF)
            }
            &[b'I', b'D', b'3', ..] => Some(mime::AUDIO_MPEG),
            &[b'O', b'g', b'g', b'S', 0, ..] => Some(mime::AUDIO_OGG),
            &[b'M', b'T', b'h', b'd', 0, 0, 0, 0x06, ..] => Some(mime::AUDIO_MIDI),
            &[b'R', b'I', b'F', b'F', _, _, _, _, b'A', b'V', b'I', b' ', ..] => {
                Some(mime::VIDEO_AVI)
            }
            &[b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'A', b'V', b'E', ..] => {
                Some(mime::AUDIO_WAVE)
            }
            &[0x1F, 0x8B, 0x08, ..] => Some(mime::APPLICATION_GZIP),
            &[b'P', b'K', 0x03, 0x04, ..] => Some(mime::APPLICATION_ZIP),
            &[b'R', b'a', b'r', b' ', 0x1A, 0x07, 0, ..] => Some(mime::APPLICATION_RAR),
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b'L', b'P'] => {
                Some(mime::APPLICATION_VND_MS_FONTOBJECT)
            }
            &[0, 0x01, 0, 0, ..] => Some(mime::FONT_TTF),
            &[b'O', b'T', b'T', b'O', ..] => Some(mime::FONT_OTF),
            &[b't', b't', b'c', b'f', ..] => Some(mime::FONT_COLLECTION),
            &[b'%', b'!', b'P', b'S', b'-', b'A', b'd', b'o', b'b', b'e', b'-', ..] => {
                Some(mime::APPLICATION_POSTSCRIPT)
            }
            &[0xFE, 0xFF, 0, 0, ..] | &[0xFF, 0xFE, 0, 0, ..] | &[0xEF, 0xBB, 0xBF, 0, ..] => {
                Some(mime::TEXT_PLAIN)
            }
            [b'<', ..] => {
                match &magic_bytes[1..]
                    .iter()
                    .map(|&b| b.to_ascii_uppercase())
                    .collect::<Vec<_>>()
                    .as_slice()
                {
                    [b'!', b'D', b'O', b'C', b'T', b'Y', b'P', b'E', b' ', b'H', b'T', b'M', b'L', tt, ..]
                        if is_whitespace_or_tag_terminator(*tt) =>
                    {
                        Some(mime::TEXT_HTML)
                    }
                    [b'H', b'T', b'M', b'L', tt, ..] if is_whitespace_or_tag_terminator(*tt) => {
                        Some(mime::TEXT_HTML)
                    }
                    [b'H', b'E', b'A', b'D', tt, ..] if is_whitespace_or_tag_terminator(*tt) => {
                        Some(mime::TEXT_HTML)
                    }
                    [b'S', b'C', b'R', b'I', b'P', b'T', tt, ..]
                        if is_whitespace_or_tag_terminator(*tt) =>
                    {
                        Some(mime::TEXT_HTML)
                    }
                    [b'I', b'F', b'R', b'A', b'M', b'E', tt, ..]
                        if is_whitespace_or_tag_terminator(*tt) =>
                    {
                        Some(mime::TEXT_HTML)
                    }
                    [b'H', b'1', tt, ..] if is_whitespace_or_tag_terminator(*tt) => {
                        Some(mime::TEXT_HTML)
                    }
                    [b'D', b'I', b'V', tt, ..] if is_whitespace_or_tag_terminator(*tt) => {
                        Some(mime::TEXT_HTML)
                    }
                    [b'F', b'O', b'N', b'T', tt, ..] if is_whitespace_or_tag_terminator(*tt) => {
                        Some(mime::TEXT_HTML)
                    }
                    [b'T', b'A', b'B', b'L', b'E', tt, ..]
                        if is_whitespace_or_tag_terminator(*tt) =>
                    {
                        Some(mime::TEXT_HTML)
                    }
                    [b'A', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'S', b'T', b'Y', b'L', b'E', tt, ..]
                        if is_whitespace_or_tag_terminator(*tt) =>
                    {
                        Some(mime::TEXT_HTML)
                    }
                    [b'T', b'I', b'T', b'L', b'E', tt, ..]
                        if is_whitespace_or_tag_terminator(*tt) =>
                    {
                        Some(mime::TEXT_HTML)
                    }
                    [b'B', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'B', b'O', b'D', b'Y', tt, ..] if is_whitespace_or_tag_terminator(*tt) => {
                        Some(mime::TEXT_HTML)
                    }
                    [b'B', b'R', tt, ..] if is_whitespace_or_tag_terminator(*tt) => {
                        Some(mime::TEXT_HTML)
                    }
                    [b'P', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn algorithm_match(&self) -> Option<mime::MediaType> {
        if self.is_mp4() {
            return Some(mime::VIDEO_MP4);
        }
        if self.is_webm() {
            return Some(mime::VIDEO_WEBM);
        }

        None
    }

    fn is_mp4(&self) -> bool {
        let data = &self.data;
        if data.len() < 12 {
            return false;
        }
        let box_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        if data.len() < box_size as usize || box_size % 4 != 0 {
            return false;
        }

        data.get(4..8) == Some(b"ftyp")
            && (data.get(8..11) == Some(b"mp4")
                || data[16..]
                    .chunks_exact(4)
                    .any(|chunk| chunk.starts_with(b"mp4")))
    }
    fn is_webm(&self) -> bool {
        let data = &self.data;
        if data.len() < 4 || data[..4] != [0x1A, 0x45, 0xDF, 0xA3] {
            return false;
        }

        let skip_first_bytes = 4;
        let chunk_size = 2;
        let magic_bytes_delim = [0x42, 0x82];
        for (chunk_idx, chunk) in data[skip_first_bytes..].chunks(chunk_size).enumerate() {
            // went over 4 + 2 * 17 = 38 bytes
            if chunk_idx >= 17 {
                break;
            }

            if chunk != magic_bytes_delim {
                continue;
            }

            let offset = skip_first_bytes + chunk_idx * chunk_size + magic_bytes_delim.len();
            if let Some((_, number_size)) = data.get(offset..).map(|d| parse_vint(d, 0)) {
                let start = offset + number_size;
                let end = start + 4;
                if data.get(start..end) == Some(b"webm") {
                    return true;
                }
            }
        }

        false
    }
}

fn parse_vint(data: &[u8], offset: usize) -> (usize, usize) {
    let mut mask = 128;
    let max_vint_length = 8;
    let mut number_size = 1;

    while number_size < max_vint_length
        && data.get(offset).is_none()
        && (data.get(offset).unwrap() & mask == 0)
    {
        mask >>= 1;
        number_size += 1;
    }

    let mut parsed_number = data.get(offset).map_or(0, |&b| (b & !mask) as usize);

    for &b in data.get(offset + 1..offset + number_size).unwrap_or(&[]) {
        parsed_number = (parsed_number << 8) | b as usize;
    }

    (parsed_number, number_size)
}

fn is_whitespace_or_tag_terminator(byte: u8) -> bool {
    byte == b' ' || byte == b'>'
}
