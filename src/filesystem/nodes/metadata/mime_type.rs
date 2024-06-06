use crate::prelude::nodes::NodeName;

#[derive(Default)]
pub struct MimeGuesser {
    name: Option<String>,
    data: Vec<u8>,
}

impl MimeGuesser {
    const MP3_RATES: [u32; 15] = [
        0, 32000, 40000, 48000, 56000, 64000, 80000, 96000, 112000, 128000, 160000, 192000, 224000,
        256000, 320000,
    ];

    const MP25_RATES: [u32; 15] = [
        0, 8000, 16000, 24000, 32000, 40000, 48000, 56000, 64000, 80000, 96000, 112000, 128000,
        144000, 160000,
    ];

    const SAMPLE_RATES: [u32; 3] = [44100, 48000, 32000];

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
                .collect::<Vec<_>>().as_slice() {
                    [b'!', b'D', b'O', b'C', b'T', b'Y', b'P', b'E', b' ', b'H', b'T', b'M', b'L', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'H', b'T', b'M', b'L', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'H', b'E', b'A', b'D', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'S', b'C', b'R', b'I', b'P', b'T', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'I', b'F', b'R', b'A', b'M', b'E', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'H', b'1', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'D', b'I', b'V', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'F', b'O', b'N', b'T', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'T', b'A', b'B', b'L', b'E', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'A', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'S', b'T', b'Y', b'L', b'E', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'T', b'I', b'T', b'L', b'E', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'B', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'B', b'O', b'D', b'Y', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'B', b'R', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    [b'P', tt, ..] if is_whitespace_or_tag_terminator(*tt) => Some(mime::TEXT_HTML),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn algorithm_match(&self) -> Option<mime::MediaType> {
        if self.is_mp4() {
            return Some(mime::AUDIO_MP4);
        }
        if self.is_mp3() {
            return Some(mime::AUDIO_MPEG);
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
        if &data[4..8] != b"ftyp" {
            return false;
        }
        if &data[8..11] == b"mp4" {
            return true;
        }
        data[16..]
            .chunks_exact(4)
            .take_while(|chunk| &chunk[..3] != b"mp4")
            .last()
            .map_or(false, |chunk| &chunk[..3] == b"mp4")
    }

    fn is_mp3(&self) -> bool {
        let data = &self.data;
        let mut offset = 0;

        if !match_mp3_header(data, offset) {
            return false;
        }

        let (version, bitrate_index, sample_rate_index, pad) = parse_mp3_frame(data, offset);
        let bitrate = if version & 0x01 != 0 {
            Self::MP25_RATES[bitrate_index as usize]
        } else {
            Self::MP3_RATES[bitrate_index as usize]
        };
        let sample_rate = Self::SAMPLE_RATES[sample_rate_index as usize];
        let skipped_bytes = compute_mp3_frame_size(version, bitrate, sample_rate, pad);

        if skipped_bytes < 4 || skipped_bytes > data.len() - offset {
            return false;
        }
        offset += skipped_bytes;

        match_mp3_header(data, offset)
    }
}

fn match_mp3_header(sequence: &[u8], s: usize) -> bool {
    let length = sequence.len();
    if length - s < 4 {
        return false;
    }

    sequence[s] == 0xff
        && sequence[s + 1] & 0xe0 == 0xe0
        && (sequence[s + 1] & 0x06 >> 1) != 0
        && (sequence[s + 2] & 0xf0 >> 4) != 15
        && (sequence[s + 2] & 0x0c >> 2) != 3
        && (4 - (sequence[s + 1] & 0x06 >> 1)) == 3
}

fn parse_mp3_frame(sequence: &[u8], s: usize) -> (u8, u8, u8, u8) {
    let version = sequence[s + 1] & 0x18 >> 3;
    let bitrate_index = sequence[s + 2] & 0xf0 >> 4;
    let sample_rate_index = sequence[s + 2] & 0x0c >> 2;
    let pad = sequence[s + 2] & 0x02 >> 1;
    (version, bitrate_index, sample_rate_index, pad)
}

fn compute_mp3_frame_size(version: u8, bitrate: u32, sample_rate: u32, pad: u8) -> usize {
    let scale = if version == 1 { 72 } else { 144 };
    let mut size = bitrate * scale / sample_rate;
    if pad != 0 {
        size += 1;
    }
    size as usize
}
fn is_whitespace_or_tag_terminator(byte: u8) -> bool {
    byte == b' ' || byte == b'>'
}
