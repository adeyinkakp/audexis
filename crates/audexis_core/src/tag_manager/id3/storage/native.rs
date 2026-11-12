use super::{Container, Layout, invalid};
use crate::library_files;
use crate::tag_manager::utils::{FrameKey, TagValue};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub(in crate::tag_manager::id3) fn read(
    path: &Path,
) -> io::Result<HashMap<FrameKey, Vec<TagValue>>> {
    let mut tags = HashMap::new();
    let mut file = library_files::open(path)?;
    let Some(layout) = Layout::open(&mut file)? else {
        return Ok(tags);
    };
    if layout.id3.is_some() {
        return Ok(tags);
    }
    for (name, chunk) in layout.native {
        file.seek(SeekFrom::Start(chunk.start + 8))?;
        if matches!(layout.container, Container::Wave) {
            read_info(&mut file, chunk.size, &mut tags)?;
        } else {
            let key = match &name {
                b"NAME" => FrameKey::Title,
                b"AUTH" => FrameKey::Artist,
                b"ANNO" => FrameKey::Comments,
                _ => continue,
            };
            read_text(&mut file, chunk.size, key, &mut tags)?;
        }
    }
    Ok(tags)
}

fn read_info(
    file: &mut File,
    size: u64,
    tags: &mut HashMap<FrameKey, Vec<TagValue>>,
) -> io::Result<()> {
    if size < 4 {
        return Ok(());
    }
    let mut name = [0; 4];
    file.read_exact(&mut name)?;
    if &name != b"INFO" {
        return Ok(());
    }

    let end = file.stream_position()? + size - 4;
    while end - file.stream_position()? >= 8 {
        let mut header = [0; 8];
        file.read_exact(&mut header)?;
        let size = Container::Wave.size(header[4..].try_into().unwrap());
        let data_end = file.stream_position()? + size;
        if data_end > end {
            return Err(invalid("Truncated WAV INFO tag"));
        }
        let key = match &header[..4] {
            b"INAM" => Some(FrameKey::Title),
            b"IART" => Some(FrameKey::Artist),
            b"IPRD" => Some(FrameKey::Album),
            b"ITRK" | b"IPRT" => Some(FrameKey::TrackNumber),
            b"IGNR" => Some(FrameKey::Genre),
            b"ICRD" => Some(FrameKey::RecordingDate),
            b"ICMT" => Some(FrameKey::Comments),
            _ => None,
        };
        if let Some(key) = key {
            read_text(file, size, key, tags)?;
        }
        file.seek(SeekFrom::Start((data_end + size % 2).min(end)))?;
    }
    Ok(())
}

fn read_text(
    file: &mut File,
    size: u64,
    key: FrameKey,
    tags: &mut HashMap<FrameKey, Vec<TagValue>>,
) -> io::Result<()> {
    if size > 1024 * 1024 {
        return Ok(());
    }
    let mut text = vec![0; size as usize];
    file.read_exact(&mut text)?;
    let text = text.split(|byte| *byte == 0).next().unwrap_or_default();
    let decoded = match std::str::from_utf8(text) {
        Ok(text) => text.to_owned(),
        Err(_) => text.iter().map(|byte| char::from(*byte)).collect(),
    };
    let decoded = decoded.trim();
    if !decoded.is_empty() {
        let values = tags.entry(key).or_default();
        if values.is_empty() || key == FrameKey::Comments {
            values.push(TagValue::Text(decoded.to_owned()));
        }
    }
    Ok(())
}
