use crate::tag_manager::id3::utils::{id3v22_key, id3v22_raw_to_tags, id3v22_tags_to_raw};
use crate::tag_manager::id3::v2_3::utils::{
    create_header_with_version, decode_text_payload, encode_text_payload, split_encoded_text,
};
use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::traits::TagFormat;
use crate::tag_manager::utils::{FrameKey, TagValue, UserTextEntry, UserUrlEntry};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct V2_2 {}

impl V2_2 {
    fn frame_size(bytes: &[u8]) -> usize {
        ((bytes[0] as usize) << 16) | ((bytes[1] as usize) << 8) | (bytes[2] as usize)
    }
}

impl TagFormat for V2_2 {
    fn new() -> Self {
        Self {}
    }

    fn get_tags(
        &self,
        file_path: &PathBuf,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        let mut file = File::open(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unable to open and read file".to_string(),
                internal_message: "Unable to open and read file".to_string(),
            })
        })?;
        // TODO: Bad handling of no header
        let mut header = [0u8; 10];
        if file.read_exact(&mut header).is_err() {
            return Ok(HashMap::new());
        }
        if &header[0..3] != b"ID3" || header[3] != 2 {
            return Ok(HashMap::new());
        }
        let tag_size = ((header[6] as usize) << 21)
            | ((header[7] as usize) << 14)
            | ((header[8] as usize) << 7)
            | (header[9] as usize);
        let mut tag_data = vec![0u8; tag_size];
        file.read_exact(&mut tag_data).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read tag data".to_string(),
                internal_message: "Failed to read tag data".to_string(),
            })
        })?;
        let mut pos = 0usize;
        let mut raw: HashMap<String, Vec<TagValue>> = HashMap::new();
        while pos + 6 <= tag_data.len() {
            let id_bytes = &tag_data[pos..pos + 3];
            if id_bytes.iter().all(|b| *b == 0) {
                break;
            }
            let id = match std::str::from_utf8(id_bytes) {
                Ok(s) if s.trim().is_empty() => break,
                Ok(s) => s.to_string(),
                Err(_) => break,
            };
            let size = V2_2::frame_size(&tag_data[pos + 3..pos + 6]);
            if size == 0 || pos + 6 + size > tag_data.len() {
                break;
            }
            let content = &tag_data[pos + 6..pos + 6 + size];
            if id == "TXX" || id == "WXX" {
                if !content.is_empty() {
                    let encoding = content[0];
                    let rest = &content[1..];
                    let (desc_bytes, value_bytes) = split_encoded_text(encoding, rest);
                    let description = decode_text_payload(encoding, desc_bytes);
                    let value = decode_text_payload(encoding, value_bytes);
                    let entry = if id == "TXX" {
                        TagValue::UserText(UserTextEntry { description, value })
                    } else {
                        TagValue::UserUrl(UserUrlEntry {
                            description,
                            url: value,
                        })
                    };
                    raw.entry(id).or_default().push(entry);
                }
            } else if id.starts_with('T') || id.starts_with('W') {
                if !content.is_empty() {
                    let encoding = content[0];
                    let text = decode_text_payload(encoding, &content[1..]);
                    let key = id3v22_key(&id);
                    if (key.is_some() && key.unwrap().is_multi_valued()) && text.contains(';') {
                        for part in text.split(';') {
                            let seg = part.trim();
                            if !seg.is_empty() {
                                raw.entry(id.clone())
                                    .or_default()
                                    .push(TagValue::Text(seg.to_string()));
                            }
                        }
                    } else {
                        raw.entry(id).or_default().push(TagValue::Text(text));
                    }
                }
            } else if id == "PIC" && content.len() > 4 {
                // let encoding = content[0];
                let image_format = &content[1..4];
                let mut idx = 4;
                if idx >= content.len() {
                    break;
                }
                let picture_type = content[idx];
                idx += 1;
                if idx >= content.len() {
                    break;
                }
                let desc_end = content[idx..]
                    .iter()
                    .position(|&b| b == 0x00)
                    .map(|o| idx + o)
                    .unwrap_or(idx);
                let image_data_start = if desc_end < content.len() {
                    desc_end + 1
                } else {
                    desc_end
                };
                let description = if desc_end > idx {
                    Some(String::from_utf8_lossy(&content[idx..desc_end]).to_string())
                } else {
                    None
                };
                let image_data = &content[image_data_start..];
                let mime = match std::str::from_utf8(image_format) {
                    Ok(f) => match f {
                        "PNG" => "image/png",
                        "JPG" | "JPEG" => "image/jpeg",
                        _ => "application/octet-stream",
                    }
                    .to_string(),
                    Err(_) => "application/octet-stream".to_string(),
                };
                raw.entry(id).or_default().push(TagValue::Picture {
                    mime,
                    data: image_data.to_vec(),
                    picture_type: Some(picture_type),
                    description,
                });
            }
            pos += 6 + size;
        }
        Ok(id3v22_raw_to_tags(&raw))
    }

    fn write_tags(
        &self,
        file_path: &PathBuf,
        updated: HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        use std::io::{Read, Seek, SeekFrom, Write};
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(file_path)
            .map_err(|_| {
                BackendError::WriteFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "Failed to open file for writing".to_string(),
                    internal_message: "Failed to open file for writing".to_string(),
                })
            })?;
        let mut header = [0u8; 10];
        file.read_exact(&mut header).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read ID3 header".to_string(),
                internal_message: "Failed to read ID3 header".to_string(),
            })
        })?;
        if &header[0..3] != b"ID3" || header[3] != 2 {
            return Err(BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Not a valid ID3v2.2 file".to_string(),
                internal_message: "Not a valid ID3v2.2 file".to_string(),
            }));
        }
        let tag_size = ((header[6] as usize) << 21)
            | ((header[7] as usize) << 14)
            | ((header[8] as usize) << 7)
            | (header[9] as usize);
        let mut tag_data = vec![0u8; tag_size];
        file.read_exact(&mut tag_data).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read ID3 tag data".to_string(),
                internal_message: "Failed to read ID3 tag data".to_string(),
            })
        })?;
        let mut pos = 0usize;
        let mut raw: HashMap<String, Vec<u8>> = HashMap::new();
        while pos + 6 <= tag_data.len() {
            let id_bytes = &tag_data[pos..pos + 3];
            if id_bytes.iter().all(|b| *b == 0) {
                break;
            }
            let id = match std::str::from_utf8(id_bytes) {
                Ok(s) if s.trim().is_empty() => break,
                Ok(s) => s.to_string(),
                Err(_) => break,
            };
            let size = V2_2::frame_size(&tag_data[pos + 3..pos + 6]);
            if size == 0 || pos + 6 + size > tag_data.len() {
                break;
            }
            let content = &tag_data[pos + 6..pos + 6 + size];
            raw.insert(id, content.to_vec());
            pos += 6 + size;
        }
        let mut single_map: HashMap<FrameKey, TagValue> = HashMap::new();
        for (k, vals) in updated.clone().into_iter() {
            if vals.is_empty() {
                continue;
            }
            if matches!(vals[0], TagValue::Text(_)) && vals.len() > 1 {
                let joined = vals
                    .iter()
                    .filter_map(|v| match v {
                        TagValue::Text(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\\");
                single_map.insert(k, TagValue::Text(joined));
            } else {
                single_map.insert(k, vals[0].clone());
            }
        }
        let raw_updates = id3v22_tags_to_raw(&single_map);
        for (k, v) in raw_updates {
            match v {
                TagValue::Text(t) => {
                    if !t.is_empty() {
                        let encoded = encode_text_payload(&t, false);
                        raw.insert(k.to_string(), encoded);
                    }
                }
                TagValue::Picture {
                    mime,
                    data,
                    picture_type,
                    description,
                } => {
                    let enc: u8 = 0x00;
                    let format_code = if mime == "image/png" { b"PNG" } else { b"JPG" };
                    let pt = picture_type.unwrap_or(3);
                    let desc_bytes = description.as_deref().unwrap_or("").as_bytes();
                    let mut payload = Vec::new();
                    payload.push(enc);
                    payload.extend_from_slice(format_code);
                    payload.push(pt);
                    payload.extend_from_slice(desc_bytes);
                    payload.push(0x00);
                    payload.extend_from_slice(&data);
                    raw.insert("PIC".to_string(), payload);
                }
                TagValue::UserText(ut) => {
                    let joined = format!("{}={}", ut.description, ut.value);
                    let encoded = encode_text_payload(&joined, false);
                    raw.insert(k.to_string(), encoded);
                }
                TagValue::UserUrl(uu) => {
                    let joined = format!("{}={}", uu.description, uu.url);
                    let encoded = encode_text_payload(&joined, false);
                    raw.insert(k.to_string(), encoded);
                }
                _ => { /*Hnandle other types later */ }
            }

            for (key, value) in updated.iter() {
                if key.clone() == FrameKey::Comments {
                    if let TagValue::Comment {
                        encoding,
                        language,
                        description,
                        text,
                    } = &value[0]
                    {
                        let enc_byte = match encoding.as_str() {
                            "UTF-16" => 0x01,
                            _ => 0x00,
                        };
                        let mut payload = Vec::new();
                        payload.push(enc_byte);
                        payload.extend_from_slice(&language.as_bytes()[..3.min(language.len())]);
                        payload.extend_from_slice(description.as_bytes());
                        payload.push(0x00);
                        payload.extend_from_slice(text.as_bytes());
                        raw.insert("COM".to_string(), payload);
                    }
                }
            }
        }
        let mut frames: Vec<u8> = Vec::new();
        for (id, content) in raw {
            if id.len() != 3 {
                continue;
            }
            let size = content.len();
            if size == 0 {
                continue;
            }
            if size > 0xFFFFFF {
                continue;
            }
            let mut frame = Vec::with_capacity(6 + size);
            frame.extend_from_slice(id.as_bytes());
            frame.push(((size >> 16) & 0xFF) as u8);
            frame.push(((size >> 8) & 0xFF) as u8);
            frame.push((size & 0xFF) as u8);
            frame.extend_from_slice(&content);
            frames.extend_from_slice(&frame);
        }
        let header = create_header_with_version(2, frames.len());
        let mut audio_data = Vec::new();
        file.read_to_end(&mut audio_data).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read audio data".to_string(),
                internal_message: "Failed to read audio data".to_string(),
            })
        })?;
        file.seek(SeekFrom::Start(0)).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to seek in file".to_string(),
                internal_message: "Failed to seek in file".to_string(),
            })
        })?;
        file.set_len(0).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to set file length".to_string(),
                internal_message: "Failed to set file length".to_string(),
            })
        })?;
        file.write_all(&header).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to write header".to_string(),
                internal_message: "Failed to write header".to_string(),
            })
        })?;
        file.write_all(&frames).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to write frames".to_string(),
                internal_message: "Failed to write frames".to_string(),
            })
        })?;
        file.write_all(&audio_data).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to write audio data".to_string(),
                internal_message: "Failed to write audio data".to_string(),
            })
        })?;
        file.flush().map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to flush file".to_string(),
                internal_message: "Failed to flush file".to_string(),
            })
        })?;
        Ok(())
    }
}
