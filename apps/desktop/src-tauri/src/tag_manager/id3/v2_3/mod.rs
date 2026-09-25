use crate::tag_manager::id3::utils::{id3v23_key, raw_to_tags, tags_to_raw};
use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::traits::TagFormat;
use crate::tag_manager::utils::{FrameKey, TagMap, TagValue, UserTextEntry, UserUrlEntry};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Read;

use crate::tag_manager::id3::v2_3::utils::{build_frame, create_header, encode_text_payload};
use crate::tag_manager::id3::v2_3::utils::{decode_text_payload, split_encoded_text};
use std::path::PathBuf;

pub mod utils;

#[derive(Debug, Clone)]
pub struct V2_3 {}
fn from_synchsafe(bytes: [u8; 4]) -> usize {
    ((bytes[0] as usize) << 21)
        | ((bytes[1] as usize) << 14)
        | ((bytes[2] as usize) << 7)
        | (bytes[3] as usize)
}

impl TagFormat for V2_3 {
    fn new() -> Self {
        Self {}
    }

    fn get_tags(
        &self,
        file_path: &PathBuf,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        let has_header = utils::ensure_header(&file_path);

        match has_header {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                return Err(BackendError::ReadFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "File does not contain ID3 tag".to_string(),
                    internal_message: "File does not contain ID3 tag".to_string(),
                }))
            }
        }
        let mut file = File::open(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unable to open and read file".to_string(),
                internal_message: "Unable to open and read file".to_string(),
            })
        })?;

        let mut header = [0u8; 10];
        file.read_exact(&mut header).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unable to read ID3 header".to_string(),
                internal_message: "Unable to read ID3 header".to_string(),
            })
        })?;

        if &header[0..3] != b"ID3" {
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "File does not contain ID3 tag".to_string(),
                internal_message: "File does not contain ID3 tag".to_string(),
            }));
        }

        let tag_size = from_synchsafe([header[6], header[7], header[8], header[9]]);
        let mut tag_data = vec![0u8; tag_size];
        file.read_exact(&mut tag_data).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read tag data".to_string(),
                internal_message: "Failed to read tag data".to_string(),
            })
        })?;

        let mut tags: TagMap = HashMap::new();
        let mut pos = 0;

        while pos + 10 <= tag_data.len() {
            let id = &tag_data[pos..pos + 4];
            let frame_id = match std::str::from_utf8(id) {
                Ok(s) if s.trim().is_empty() => break,
                Ok(s) => s.to_string(),
                Err(_) => break,
            };

            let size = u32::from_be_bytes([
                tag_data[pos + 4],
                tag_data[pos + 5],
                tag_data[pos + 6],
                tag_data[pos + 7],
            ]) as usize;

            let _flags = &tag_data[pos + 8..pos + 10];

            if pos + 10 + size > tag_data.len() {
                break;
            }

            let content = &tag_data[pos + 10..pos + 10 + size];

            if frame_id == "TXXX" || frame_id == "WXXX" {
                if !content.is_empty() {
                    let encoding = content[0];
                    let rest = &content[1..];
                    let (desc_bytes, value_bytes) = split_encoded_text(encoding, rest);
                    let description = decode_text_payload(encoding, desc_bytes);
                    let value = decode_text_payload(encoding, value_bytes);
                    let entry = if frame_id == "TXXX" {
                        TagValue::UserText(UserTextEntry { description, value })
                    } else {
                        TagValue::UserUrl(UserUrlEntry {
                            description,
                            url: value,
                        })
                    };
                    tags.entry(frame_id).or_default().push(entry);
                }
            } else if frame_id.starts_with('T') || frame_id.starts_with("W") {
                if !content.is_empty() {
                    let encoding = content[0];
                    let raw_string = decode_text_payload(encoding, &content[1..]);

                    let key = id3v23_key(&frame_id);

                    if (key.is_some() && key.unwrap().is_multi_valued()) && raw_string.contains(';')
                    {
                        for part in raw_string.split(';').map(|s| s.trim()) {
                            let seg = part.trim();
                            if !seg.is_empty() {
                                tags.entry(frame_id.clone())
                                    .or_default()
                                    .push(TagValue::Text(seg.to_string()));
                            }
                        }
                    } else {
                        tags.entry(frame_id)
                            .or_default()
                            .push(TagValue::Text(raw_string));
                    }
                }
            } else if frame_id == "APIC" && !content.is_empty() {
                let _encoding = content[0];
                if let Some(mime_null_rel) = content[1..].iter().position(|&b| b == 0x00) {
                    let mime_start = 1usize;
                    let mime_end = 1 + mime_null_rel;
                    let mime_type =
                        String::from_utf8_lossy(&content[mime_start..mime_end]).to_string();

                    let picture_type_index = mime_end + 1;
                    if picture_type_index >= content.len() {
                        pos += 10 + size;
                        continue;
                    }
                    let picture_type = content[picture_type_index];
                    let description_start = picture_type_index + 1;
                    let description_end = content[description_start..]
                        .iter()
                        .position(|&b| b == 0x00)
                        .map_or(content.len(), |p| description_start + p);
                    let description = if description_end > description_start {
                        Some(
                            String::from_utf8_lossy(&content[description_start..description_end])
                                .to_string(),
                        )
                    } else {
                        None
                    };
                    let image_data =
                        if description_end < content.len() && description_end + 1 < content.len() {
                            &content[description_end + 1..]
                        } else {
                            &[]
                        };

                    tags.entry(frame_id).or_default().push(TagValue::Picture {
                        mime: mime_type,
                        data: image_data.to_vec(),
                        picture_type: Some(picture_type),
                        description,
                    });
                }
            }

            pos += 10 + size;
        }

        let tags = raw_to_tags(&tags);
        Ok(tags)
    }

    fn write_tags(
        &self,
        file_path: &PathBuf,
        updated_tags: HashMap<FrameKey, Vec<TagValue>>,
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
        if &header[0..3] != b"ID3" || header[3] != 3 {
            return Err(BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unsupported ID3 version".to_string(),
                internal_message: "Unsupported ID3 version".to_string(),
            }));
        }

        let tag_size = from_synchsafe([header[6], header[7], header[8], header[9]]);
        let mut tag_data = vec![0u8; tag_size];
        file.read_exact(&mut tag_data).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read ID3 tag data".to_string(),
                internal_message: "Failed to read ID3 tag data".to_string(),
            })
        })?;

        let mut pos: usize = 0;
        let mut raw_frames: Vec<(String, Vec<u8>)> = Vec::new();
        while pos + 10 <= tag_data.len() {
            let id_bytes = &tag_data[pos..pos + 4];
            if id_bytes == [0, 0, 0, 0] {
                break;
            }
            let frame_id = match std::str::from_utf8(id_bytes) {
                Ok(s) if s.trim().is_empty() => break,
                Ok(s) => s.to_string(),
                Err(_) => break,
            };

            let size = u32::from_be_bytes([
                tag_data[pos + 4],
                tag_data[pos + 5],
                tag_data[pos + 6],
                tag_data[pos + 7],
            ]) as usize;

            if size == 0 {
                pos += 10;
                continue;
            }
            if pos + 10 + size > tag_data.len() {
                break;
            }

            let content = &tag_data[pos + 10..pos + 10 + size];
            raw_frames.push((frame_id, content.to_vec()));

            pos += 10 + size;
        }

        let mut pictures: Vec<TagValue> = Vec::new();
        let mut flattened: HashMap<FrameKey, TagValue> = HashMap::new();
        for (k, vec_vals) in updated_tags.clone().into_iter() {
            if vec_vals.is_empty() {
                continue;
            }
            if k == FrameKey::AttachedPicture {
                for v in vec_vals {
                    if let TagValue::Picture { .. } = v {
                        pictures.push(v);
                    }
                }
                continue;
            }
            if matches!(vec_vals[0], TagValue::Text(_)) && vec_vals.len() > 1 {
                let joined = vec_vals
                    .iter()
                    .filter_map(|v| match v {
                        TagValue::Text(s) => Some(s.clone()),
                        TagValue::UserUrl(v) => Some(format!("{}={}", v.description, v.url)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\\");
                flattened.insert(k, TagValue::Text(joined));
            } else {
                flattened.insert(k, vec_vals[0].clone());
            }
        }
        let raw_updated_tags = tags_to_raw(&flattened);
        let mut updated_keys: Vec<String> =
            raw_updated_tags.keys().map(|k| k.to_string()).collect();
        if !pictures.is_empty() {
            updated_keys.push("APIC".to_string());
        }
        raw_frames.retain(|(id, _)| !updated_keys.iter().any(|k| k == id));

        for (k, v) in raw_updated_tags {
            match v {
                TagValue::Text(text) => {
                    if !text.is_empty() {
                        if k == "TXXX" || k == "WXXX" {
                            let (desc, val) = match text.split_once('=') {
                                Some((d, v)) => (d.to_string(), v.to_string()),
                                None => (String::new(), text.clone()),
                            };
                            let mut payload = Vec::new();
                            payload.push(0x00);
                            payload.extend_from_slice(desc.as_bytes());
                            payload.push(0x00);
                            payload.extend_from_slice(val.as_bytes());
                            raw_frames.push((k.to_string(), payload));
                        } else {
                            let encoded_text = encode_text_payload(&text, false);
                            raw_frames.push((k.to_string(), encoded_text));
                        }
                    }
                }
                TagValue::Picture {
                    mime,
                    data,
                    picture_type,
                    description,
                } => {
                    let pt = picture_type.unwrap_or(3);
                    let desc = description.as_deref().unwrap_or("");
                    let encoded_data = utils::encode_img_payload(&mime, pt, desc, &data);
                    raw_frames.push((k.to_string(), encoded_data));
                }
                TagValue::UserText(ut) => {
                    let mut payload = Vec::new();
                    payload.push(0x00);
                    payload.extend_from_slice(ut.description.as_bytes());
                    payload.push(0x00);
                    payload.extend_from_slice(ut.value.as_bytes());
                    raw_frames.push((k.to_string(), payload));
                }
                TagValue::UserUrl(uu) => {
                    let mut payload = Vec::new();
                    payload.push(0x00);
                    payload.extend_from_slice(uu.description.as_bytes());
                    payload.push(0x00);
                    payload.extend_from_slice(uu.url.as_bytes());
                    raw_frames.push((k.to_string(), payload));
                }
                _ => { /*Handle other types later */ }
            }
        }
        let comments = updated_tags.get(&FrameKey::Comments);
        comments.map(|vals| {
            for val in vals {
                if let TagValue::Comment {
                    encoding: _,
                    language,
                    description,
                    text,
                } = val
                {
                    let mut payload = Vec::new();
                    payload.push(0x00);
                    let lang_bytes = language.as_bytes();
                    if lang_bytes.len() >= 3 {
                        payload.extend_from_slice(&lang_bytes[0..3]);
                    } else {
                        payload.extend_from_slice(lang_bytes);
                        for _ in 0..(3 - lang_bytes.len()) {
                            payload.push(0x00);
                        }
                    }
                    payload.extend_from_slice(description.as_bytes());
                    payload.push(0x00);
                    payload.extend_from_slice(text.as_bytes());
                    raw_frames.push(("COMM".to_string(), payload));
                }
            }
        });
        for v in pictures.into_iter() {
            if let TagValue::Picture {
                mime,
                data,
                picture_type,
                description,
            } = v
            {
                let pt = picture_type.unwrap_or(3);
                let desc = description.as_deref().unwrap_or("");
                let encoded_data = utils::encode_img_payload(&mime, pt, desc, &data);
                raw_frames.push(("APIC".to_string(), encoded_data));
            }
        }
        println!(
            "Updated raw tags: {:?}",
            raw_frames.iter().map(|(k, _)| k).collect::<Vec<_>>()
        );

        let frames = raw_frames
            .iter()
            .map(|(id, content)| build_frame(id, content))
            .collect::<Vec<_>>();
        let total_frame_size: usize = frames.iter().map(|f| f.len()).sum();
        let header = create_header(total_frame_size);

        let mut id3_tag = Vec::with_capacity(10 + total_frame_size);
        id3_tag.extend_from_slice(&header);
        for frame in frames {
            id3_tag.extend_from_slice(&frame);
        }

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
                public_message: "Failed to seek audio data".to_string(),
                internal_message: "Failed to seek audio data".to_string(),
            })
        })?;
        file.set_len(0).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to set file length".to_string(),
                internal_message: "Failed to set file length".to_string(),
            })
        })?;
        file.write_all(&id3_tag).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to write ID3 tag".to_string(),
                internal_message: "Failed to write ID3 tag".to_string(),
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
