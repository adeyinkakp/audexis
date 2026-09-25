use crate::tag_manager::traits::{Formats, TagFamily};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
mod flac;
mod id3;
mod itunes;
mod ogg;
pub mod tag_backend;
pub mod traits;
pub mod utils;

mod vorbis_comments;

#[derive(Debug, Clone)]
pub struct TagManager {
    id3: id3::Id3,
    itunes: itunes::Itunes,
    flac: flac::Flac,
    ogg: ogg::Ogg,
}

impl TagManager {
    pub fn new() -> Self {
        Self {
            id3: id3::Id3::new(),
            itunes: itunes::Itunes::new(),
            flac: flac::Flac::new(),
            ogg: ogg::Ogg::new(),
        }
    }

    pub fn get_release_class(&self, version: &Formats) -> Option<Box<dyn traits::TagFormat>> {
        match version {
            Formats::Id3v10 | Formats::Id3v11 => self.id3.get_release_class(version),
            Formats::Id3v22 => self.id3.get_release_class(&Formats::Id3v22),
            Formats::Id3v23 => self.id3.get_release_class(&Formats::Id3v23),
            Formats::Id3v24 => self.id3.get_release_class(&Formats::Id3v24),
            Formats::Itunes => self.itunes.get_release_class(&Formats::Itunes),
            Formats::Flac => self.flac.get_release_class(&Formats::Flac),
            Formats::Ogg => self.ogg.get_release_class(&Formats::Ogg),

            _ => None,
        }
    }
    pub fn detect_tag_format(&self, file_path: &PathBuf) -> Formats {
        detect_formats(file_path, None)
            .into_iter()
            .next()
            .unwrap_or(Formats::Unknown)
    }
}

pub(crate) fn detect_formats(file_path: &PathBuf, primary: Option<Formats>) -> Vec<Formats> {
    let mut file = match File::open(file_path) {
        Ok(file) => file,
        Err(_) => return primary.into_iter().collect(),
    };
    let mut header = vec![0; 4096];
    let length = match file.read(&mut header) {
        Ok(length) => length,
        Err(_) => return primary.into_iter().collect(),
    };
    header.truncate(length);

    let mut formats = primary.into_iter().collect::<Vec<_>>();
    let add = |formats: &mut Vec<Formats>, format| {
        if !formats.contains(&format) {
            formats.push(format);
        }
    };
    match header.get(0..4) {
        Some(b"fLaC") => add(&mut formats, Formats::Flac),
        Some(b"OggS") => add(&mut formats, Formats::Ogg),
        Some(b"RIFF") => add(&mut formats, Formats::Riff),
        _ => {}
    }
    let known_brand = |brand: &[u8]| {
        matches!(
            brand,
            b"M4A " | b"M4B " | b"M4V " | b"mp41" | b"mp42" | b"isom" | b"iso2" | b"qt  "
        )
    };
    let mut offset = 0;
    while offset + 8 <= header.len() {
        let size = u32::from_be_bytes(header[offset..offset + 4].try_into().unwrap()) as usize;
        if &header[offset + 4..offset + 8] == b"ftyp" && size >= 16 && offset + 16 <= header.len() {
            let end = (offset + size).min(header.len());
            if known_brand(&header[offset + 8..offset + 12])
                || header[offset + 16..end].chunks_exact(4).any(known_brand)
            {
                add(&mut formats, Formats::Itunes);
            }
            break;
        }
        offset = if size >= 8 {
            offset.saturating_add(size)
        } else {
            offset + 1
        };
    }
    if let Some(version) = header.get(3..5).filter(|_| header.starts_with(b"ID3")) {
        match version {
            [2, 0] => add(&mut formats, Formats::Id3v22),
            [3, 0] => add(&mut formats, Formats::Id3v23),
            [4, 0] => add(&mut formats, Formats::Id3v24),
            _ => {}
        }
    }
    if file
        .metadata()
        .map(|meta| meta.len() >= 128)
        .unwrap_or(false)
        && file.seek(SeekFrom::End(-128)).is_ok()
    {
        let mut tail = [0; 128];
        if file.read_exact(&mut tail).is_ok() && &tail[..3] == b"TAG" {
            add(
                &mut formats,
                if tail[125] == 0 {
                    Formats::Id3v11
                } else {
                    Formats::Id3v10
                },
            );
        }
    }
    if formats.is_empty()
        && matches!(
            file_path.extension().and_then(|e| e.to_str()),
            Some("mp3" | "mp2" | "mp1")
        )
    {
        formats.push(Formats::Id3v23);
    }
    formats
}
