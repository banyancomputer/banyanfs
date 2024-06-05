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
        let guess = mime_guess::get_mime_extensions_str(
            self.name.as_ref().map_or("", |name| name.as_str()),
        );
        if let Some(guess) = guess {
            return mime::MediaType::parse(*guess.first()?).ok();
        }
        None
    }

    fn pattern_match(&self) -> Option<mime::MediaType> {
        let magic_bytes = &self.data.get(0..34)?;

        // Taken from https://mimesniff.spec.whatwg.org/
        match magic_bytes {
            [0xFF, 0xD8, 0xFF, ..] => Some(mime::IMAGE_JPEG),
            [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] => Some(mime::IMAGE_PNG),
            [0x47, 0x49, 0x46, 0x38, 0x37, 0x61, ..] | [0x47, 0x49, 0x46, 0x38, 0x39, 0x61, ..] => {
                Some(mime::IMAGE_GIF)
            }
            [0x42, 0x4D, ..] => Some(mime::IMAGE_BMP),
            [0x3C, 0x3F, 0x78, 0x6D, 0x6C, ..] => Some(mime::TEXT_XML),
            [0x3C, 0x73, 0x76, 0x67, ..] => Some(mime::IMAGE_SVG),
            [0x77, 0x4F, 0x46, 0x46, ..] => Some(mime::FONT_WOFF),
            [0x77, 0x4F, 0x46, 0x32, ..] => Some(mime::FONT_WOFF2),
            [0x25, 0x50, 0x44, 0x46, 0x2D, ..] => Some(mime::APPLICATION_PDF),
            [0x7B, ..] => Some(mime::APPLICATION_JSON),
            [0x46, 0x4F, 0x52, 0x4D, _, _, _, _, 0x41, 0x49, 0x46, 0x46, ..] => {
                Some(mime::AUDIO_AIFF)
            }
            [0x49, 0x44, 0x33, ..] => Some(mime::AUDIO_MPEG),
            [0x4F, 0x67, 0x67, 0x53, 0x00, ..] => Some(mime::AUDIO_OGG),
            [0x4D, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, ..] => Some(mime::AUDIO_MIDI),
            [0x52, 0x49, 0x46, 0x46, _, _, _, _, 0x41, 0x56, 0x49, 0x20, ..] => {
                Some(mime::VIDEO_AVI)
            }
            [0x52, 0x49, 0x46, 0x46, _, _, _, _, 0x57, 0x41, 0x56, 0x45, ..] => {
                Some(mime::AUDIO_WAVE)
            }
            [0x1F, 0x8B, 0x08, ..] => Some(mime::APPLICATION_GZIP),
            [0x50, 0x4B, 0x03, 0x04, ..] => Some(mime::APPLICATION_ZIP),
            [0x52, 0x61, 0x72, 0x20, 0x1A, 0x07, 0x00, ..] => Some(mime::APPLICATION_RAR),
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4C, 0x50, ..] => {
                Some(mime::APPLICATION_VND_MS_FONTOBJECT)
            }
            [0x00, 0x01, 0x00, 0x00, ..] => Some(mime::FONT_TTF),
            [0x4F, 0x54, 0x54, 0x4F, ..] => Some(mime::FONT_OTF),
            [0x74, 0x74, 0x63, 0x66, ..] => Some(mime::FONT_COLLECTION),
            [0x25, 0x21, 0x50, 0x53, 0x2D, 0x41, 0x64, 0x6F, 0x62, 0x65, 0x2D, ..] => {
                Some(mime::APPLICATION_POSTSCRIPT)
            }
            [0xFE, 0xFF, 0x00, 0x00, ..]
            | [0xFF, 0xFE, 0x00, 0x00, ..]
            | [0xEF, 0xBB, 0xBF, 0x00, ..] => Some(mime::TEXT_PLAIN),
            // TODO: And the mask
            [0x3C, 0x21, 0x44, 0x4F, 0x43, 0x54, 0x59, 0x50, 0x45, 0x20, 0x48, 0x54, 0x4D, 0x4C, ..]
            | [0x3C, 0x53, 0x43, 0x52, 0x49, 0x50, 0x54, ..]
            | [0x3C, 0x49, 0x46, 0x52, 0x41, 0x4D, 0x45, ..]
            | [0x3C, 0x54, 0x41, 0x42, 0x4C, 0x45, ..]
            | [0x3C, 0x53, 0x54, 0x59, 0x4C, 0x45, ..]
            | [0x3C, 0x54, 0x49, 0x54, 0x4C, 0x45, ..]
            | [0x3C, 0x48, 0x45, 0x41, 0x44, ..]
            | [0x3C, 0x48, 0x54, 0x4D, 0x4C, ..]
            | [0x3C, 0x46, 0x4F, 0x4E, 0x54, ..]
            | [0x3C, 0x42, 0x4F, 0x44, 0x59, ..]
            | [0x3C, 0x44, 0x49, 0x56, ..]
            | [0x3C, 0x21, 0x2D, 0x2D, ..]
            | [0x3C, 0x48, 0x31, ..]
            | [0x3C, 0x42, 0x52, ..]
            | [0x3C, 0x41, ..]
            | [0x3C, 0x42, ..]
            | [0x3C, 0x50, ..] => Some(mime::TEXT_HTML),
            _ => None,
        }
    }

    fn algorithm_match(&self) -> Option<mime::MediaType> {
        if self.is_mp4() == Some(mime::AUDIO_MP4) {
            return Some(mime::AUDIO_MP4);
        }
        if self.is_mp3() {
            return Some(mime::AUDIO_MPEG);
        }
        None
    }
    fn is_mp4(&self) -> Option<mime::MediaType> {
        let length = self.data.len();
        if length < 12 {
            return None;
        }
        let box_size = u32::from_be_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]);
        if length < box_size as usize || box_size % 4 != 0 {
            return None;
        }
        if self.data[4..8] != [0x66, 0x74, 0x79, 0x70] {
            return None;
        }
        if self.data[8..11] == [0x6D, 0x70, 0x34] {
            return Some(mime::AUDIO_MP4);
        }
        let mut bytes_read = 16;
        while bytes_read < box_size as usize {
            if self.data[bytes_read..bytes_read + 3] == [0x6D, 0x70, 0x34] {
                return Some(mime::AUDIO_MP4);
            }
            bytes_read += 4;
        }
        None
    }

    fn is_mp3(&self) -> bool {
        let sequence = &self.data;
        let length = sequence.len();
        let mut s = 0;

        if !match_mp3_header(sequence, s) {
            return false;
        }

        let (version, bitrate_index, samplerate_index, pad) = parse_mp3_frame(sequence, s);
        let bitrate = if version & 0x01 != 0 {
            MimeGuesser::MP25_RATES[bitrate_index as usize]
        } else {
            MimeGuesser::MP3_RATES[bitrate_index as usize]
        };
        let sample_rate = MimeGuesser::SAMPLE_RATES[samplerate_index as usize];
        let skipped_bytes = compute_mp3_frame_size(version, bitrate, sample_rate, pad);

        if skipped_bytes < 4 || skipped_bytes > length - s {
            return false;
        }
        s += skipped_bytes;

        if !match_mp3_header(sequence, s) {
            return false;
        }

        true
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
    let samplerate_index = sequence[s + 2] & 0x0c >> 2;
    let pad = sequence[s + 2] & 0x02 >> 1;
    (version, bitrate_index, samplerate_index, pad)
}

fn compute_mp3_frame_size(version: u8, bitrate: u32, samplerate: u32, pad: u8) -> usize {
    let scale = if version == 1 { 72 } else { 144 };
    let mut size = (bitrate as usize * scale / samplerate as usize) as usize;
    if pad != 0 {
        size += 1;
    }
    size
}
