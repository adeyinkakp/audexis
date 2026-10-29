use super::*;

impl V2_2 {
    pub(super) fn write_metadata(
        &self,
        file_path: &std::path::Path,
        updated: HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        use std::io::{Read, Seek, SeekFrom, Write};
        let mut file = crate::library_files::open_for_update(file_path).map_err(|_| {
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
        let mut raw: Vec<(String, Vec<u8>)> = Vec::new();
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
            raw.push((id, content.to_vec()));
            pos += 6 + size;
        }
        for key in updated.keys() {
            raw.retain(|(id, _)| id != crate::tag_manager::id3::utils::id3v22_code(*key));
        }
        let mut single_map: HashMap<FrameKey, TagValue> = HashMap::new();
        for (k, vals) in updated.clone().into_iter() {
            if vals.is_empty()
                || matches!(
                    k,
                    FrameKey::UserDefinedText | FrameKey::UserDefinedURL | FrameKey::Comments
                )
            {
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
                    .join("\0");
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
                        if matches!(k, "ULT" | "SLT" | "COM") {
                            let payload = crate::tag_manager::id3::lyrics::encode(k, &t).map_err(
                                |message| {
                                    BackendError::WriteFailed(TagError {
                                        path: file_path.to_string_lossy().into_owned(),
                                        public_message: "Invalid lyrics".into(),
                                        internal_message: message,
                                    })
                                },
                            )?;
                            raw.push((k.to_string(), payload));
                            continue;
                        }

                        let encoded = if k.starts_with('W') {
                            t.as_bytes().to_vec()
                        } else {
                            encode_text_payload(&t, false)
                        };
                        raw.push((k.to_string(), encoded));
                    }
                }
                TagValue::Picture {
                    mime,
                    data,
                    picture_type,
                    description,
                } => {
                    let encoded_description =
                        encode_text_payload(description.as_deref().unwrap_or(""), false);
                    let enc = encoded_description[0];
                    let format_code = if mime == "image/png" { b"PNG" } else { b"JPG" };
                    let pt = picture_type.unwrap_or(3);
                    let desc_bytes = &encoded_description[1..];
                    let mut payload = Vec::new();
                    payload.push(enc);
                    payload.extend_from_slice(format_code);
                    payload.push(pt);
                    payload.extend_from_slice(desc_bytes);
                    payload.push(0x00);
                    if enc == 1 {
                        payload.push(0x00);
                    }
                    payload.extend_from_slice(&data);
                    raw.push(("PIC".to_string(), payload));
                }
                TagValue::UserText(ut) => {
                    let joined = format!("{}={}", ut.description, ut.value);
                    let encoded = encode_text_payload(&joined, false);
                    raw.push((k.to_string(), encoded));
                }
                TagValue::UserUrl(uu) => {
                    let joined = format!("{}={}", uu.description, uu.url);
                    let encoded = encode_text_payload(&joined, false);
                    raw.push((k.to_string(), encoded));
                }
                _ => { /*Hnandle other types later */ }
            }
        }
        for (index, value) in updated
            .get(&FrameKey::Comments)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let payload = crate::tag_manager::id3::lyrics::encode_comment(value, index).map_err(
                |message| {
                    BackendError::WriteFailed(TagError {
                        path: file_path.to_string_lossy().into_owned(),
                        public_message: "Invalid comment".into(),
                        internal_message: message,
                    })
                },
            )?;
            raw.push(("COM".to_string(), payload));
        }
        raw.extend(crate::tag_manager::id3::v2_common::custom_frames(
            &updated, true,
        ));
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
