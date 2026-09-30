use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::traits::TagFormat;
use crate::tag_manager::utils::{FrameKey, TagValue};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const TAG_SIZE: usize = 128;
const TAG_ID: &[u8; 3] = b"TAG";

#[derive(Debug, Clone)]
pub struct V1 {}

impl V1 {
    fn read_tail(path: &Path) -> std::io::Result<Option<[u8; TAG_SIZE]>> {
        let mut f = File::open(path)?;
        let len = f.metadata()?.len();
        if len < TAG_SIZE as u64 {
            return Ok(None);
        }
        f.seek(SeekFrom::End(-(TAG_SIZE as i64)))?;
        let mut buf = [0u8; TAG_SIZE];
        f.read_exact(&mut buf)?;
        if &buf[0..3] != TAG_ID {
            return Ok(None);
        }
        Ok(Some(buf))
    }

    fn trim_text(bytes: &[u8]) -> String {
        let s = bytes
            .iter()
            .take_while(|b| **b != 0)
            .cloned()
            .collect::<Vec<u8>>();
        let s = String::from_utf8_lossy(&s).to_string();
        s.trim_end().to_string()
    }

    fn write_tag(
        path: &Path,
        fields: &HashMap<FrameKey, TagValue>,
        existing: Option<[u8; TAG_SIZE]>,
        is_v11: bool,
    ) -> Result<(), ()> {
        let mut tag = [0u8; TAG_SIZE];
        tag[0..3].copy_from_slice(TAG_ID);
        let get_text = |k: FrameKey| -> String {
            if let Some(TagValue::Text(s)) = fields.get(&k) {
                s.clone()
            } else {
                String::new()
            }
        };

        Self::write_field(&mut tag[3..33], &get_text(FrameKey::Title));

        Self::write_field(&mut tag[33..63], &get_text(FrameKey::Artist));

        Self::write_field(&mut tag[63..93], &get_text(FrameKey::Album));

        Self::write_field(&mut tag[93..97], &get_text(FrameKey::Year));

        let comment = get_text(FrameKey::Comments);
        if is_v11 {
            Self::write_field(&mut tag[97..125], &comment);
            tag[125] = 0;

            if let Some(TagValue::Text(tn)) = fields.get(&FrameKey::TrackNumber) {
                if let Ok(num) = tn.parse::<u8>() {
                    tag[126] = num;
                }
            }
        } else {
            Self::write_field(&mut tag[97..127], &comment);
        }
        tag[127] = 255;
        if let Some(TagValue::Text(g)) = fields.get(&FrameKey::Genre) {
            if let Ok(num) = g.parse::<u8>() {
                tag[127] = num;
            }
        }

        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|_| ())?;
        if existing.is_some() {
            f.seek(SeekFrom::End(-(TAG_SIZE as i64))).map_err(|_| ())?;
            f.write_all(&tag).map_err(|_| ())?;
        } else {
            f.seek(SeekFrom::End(0)).map_err(|_| ())?;
            f.write_all(&tag).map_err(|_| ())?;
        }
        Ok(())
    }

    fn write_field(dest: &mut [u8], value: &str) {
        for b in dest.iter_mut() {
            *b = 0;
        }
        let bytes = value.as_bytes();
        let len = bytes.len().min(dest.len());
        dest[..len].copy_from_slice(&bytes[..len]);
    }
}

impl TagFormat for V1 {
    fn new() -> Self {
        Self {}
    }

    fn get_tags(
        &self,
        file_path: &std::path::Path,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        let mut map: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
        if let Some(buf) = Self::read_tail(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read ID3v1 tag".to_string(),
                internal_message: "Failed to read ID3v1 tag".to_string(),
            })
        })? {
            let is_v11 = buf[125] == 0;
            let title = Self::trim_text(&buf[3..33]);
            let artist = Self::trim_text(&buf[33..63]);
            let album = Self::trim_text(&buf[63..93]);
            let year = Self::trim_text(&buf[93..97]);
            let comment_slice = if is_v11 { &buf[97..125] } else { &buf[97..127] };
            let comment = Self::trim_text(comment_slice);
            let track = if is_v11 {
                let t = buf[126];
                if t != 0 {
                    Some(t)
                } else {
                    None
                }
            } else {
                None
            };
            let genre_byte = buf[127];

            if !title.is_empty() {
                map.entry(FrameKey::Title)
                    .or_default()
                    .push(TagValue::Text(title));
            }
            if !artist.is_empty() {
                map.entry(FrameKey::Artist)
                    .or_default()
                    .push(TagValue::Text(artist));
            }
            if !album.is_empty() {
                map.entry(FrameKey::Album)
                    .or_default()
                    .push(TagValue::Text(album));
            }
            if !year.is_empty() {
                map.entry(FrameKey::Year)
                    .or_default()
                    .push(TagValue::Text(year));
            }
            if !comment.is_empty() {
                map.entry(FrameKey::Comments)
                    .or_default()
                    .push(TagValue::Text(comment));
            }
            if let Some(tn) = track {
                map.entry(FrameKey::TrackNumber)
                    .or_default()
                    .push(TagValue::Text(tn.to_string()));
            }
            if genre_byte != 255 {
                map.entry(FrameKey::Genre)
                    .or_default()
                    .push(TagValue::Text(genre_byte.to_string()));
            }
        }
        Ok(map)
    }

    fn write_tags(
        &self,
        file_path: &std::path::Path,
        updated_tags: HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        let supported_keys = [
            FrameKey::Title,
            FrameKey::Artist,
            FrameKey::Album,
            FrameKey::Year,
            FrameKey::Comments,
            FrameKey::TrackNumber,
            FrameKey::Genre,
        ];

        let mut touches_supported = false;
        for k in supported_keys.iter() {
            if updated_tags.contains_key(k) {
                touches_supported = true;
                break;
            }
        }

        let existing_buf = Self::read_tail(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read ID3v1 tag".to_string(),
                internal_message: "Failed to read ID3v1 tag".to_string(),
            })
        })?;
        let existing_map = self.get_tags(file_path).unwrap_or_default();

        if !touches_supported {
            return Ok(());
        }

        let mut merged: HashMap<FrameKey, TagValue> = HashMap::new();
        for k in supported_keys.iter() {
            if let Some(vs) = updated_tags.get(k) {
                if let Some(TagValue::Text(s)) = vs.first() {
                    merged.insert(*k, TagValue::Text(s.clone()));
                }
            } else if let Some(vs) = existing_map.get(k) {
                if let Some(TagValue::Text(s)) = vs.first() {
                    merged.insert(*k, TagValue::Text(s.clone()));
                }
            }
        }

        let has_any = merged.iter().any(|(_, v)| match v {
            TagValue::Text(s) => !s.is_empty(),
            _ => false,
        });
        if !has_any {
            return Ok(());
        }

        let wants_track = merged.contains_key(&FrameKey::TrackNumber)
            && match merged.get(&FrameKey::TrackNumber) {
                Some(TagValue::Text(s)) => !s.is_empty(),
                _ => false,
            };
        let is_existing_v11 = existing_buf.map(|b| b[125] == 0).unwrap_or(false);
        let is_v11 = wants_track || is_existing_v11;

        Self::write_tag(file_path, &merged, existing_buf, is_v11).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to write ID3v1 tag".to_string(),
                internal_message: "Failed to write ID3v1 tag".to_string(),
            })
        })
    }
}
