use crate::tag_manager::traits;
use crate::tag_manager::traits::{Formats, TagFormat};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub mod utils;
mod v1;
mod v2_2;
mod v2_3;
mod v2_4;
pub(crate) mod v2_common;

#[derive(Debug, Clone)]
pub struct Id3 {
    v1: v1::V1,
    v2_2: v2_2::V2_2,
    v2_3: v2_3::V2_3,
    v2_4: v2_4::V2_4,
}

impl Id3 {
    pub fn new() -> Self {
        Self {
            v1: v1::V1::new(),
            v2_2: v2_2::V2_2::new(),
            v2_3: v2_3::V2_3::new(),
            v2_4: v2_4::V2_4::new(),
        }
    }
}

pub(crate) fn ensure_v23_header(path: &Path) -> std::io::Result<bool> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut signature = [0_u8; 3];
    let read = file.read(&mut signature)?;
    if read == signature.len() && &signature == b"ID3" {
        return Ok(false);
    }

    let original_len = file.metadata()?.len();
    file.set_len(original_len + 10)?;
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut remaining = original_len;
    while remaining > 0 {
        let size = remaining.min(buffer.len() as u64) as usize;
        let source = remaining - size as u64;
        file.seek(SeekFrom::Start(source))?;
        file.read_exact(&mut buffer[..size])?;
        file.seek(SeekFrom::Start(source + 10))?;
        file.write_all(&buffer[..size])?;
        remaining = source;
    }
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&v2_common::create_header(3, 0))?;
    file.sync_all()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::ensure_v23_header;
    use std::fs;

    #[test]
    fn inserts_header_without_changing_audio_bytes() {
        let path = std::env::temp_dir().join(format!(
            "audexis-id3-header-{}-{}.mp3",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let audio = b"\xff\xfb\x90\x64audio-data";
        fs::write(&path, audio).unwrap();
        assert!(ensure_v23_header(&path).unwrap());
        let written = fs::read(&path).unwrap();
        assert_eq!(&written[..5], b"ID3\x03\x00");
        assert_eq!(&written[10..], audio);
        assert!(!ensure_v23_header(&path).unwrap());
        assert_eq!(fs::read(&path).unwrap(), written);
        fs::remove_file(path).unwrap();
    }
}

impl traits::TagFamily for Id3 {
    fn new() -> Self {
        Self {
            v1: v1::V1::new(),
            v2_2: v2_2::V2_2::new(),
            v2_3: v2_3::V2_3::new(),
            v2_4: v2_4::V2_4::new(),
        }
    }
    fn get_release_class(&self, version: &Formats) -> Option<Box<dyn TagFormat>> {
        match version {
            Formats::Id3v10 | Formats::Id3v11 => Some(Box::new(self.v1.clone())),
            Formats::Id3v22 => Some(Box::new(self.v2_2.clone())),
            Formats::Id3v23 => Some(Box::new(self.v2_3.clone())),
            Formats::Id3v24 => Some(Box::new(self.v2_4.clone())),
            _ => None,
        }
    }
}
