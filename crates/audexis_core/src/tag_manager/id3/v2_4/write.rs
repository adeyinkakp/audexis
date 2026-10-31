use super::*;

impl V2_4 {
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
                public_message: "Failed to read tag header".to_string(),
                internal_message: "Failed to read tag header".to_string(),
            })
        })?;
        if &header[0..3] != b"ID3" || header[3] != 4 {
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
                public_message: "Failed to read tag data".to_string(),
                internal_message: "Failed to read tag data".to_string(),
            })
        })?;
        let mut pos = 0usize;
        let mut raw_frames: Vec<(String, Vec<u8>)> = Vec::new();
        while pos + 10 <= tag_data.len() {
            let id_bytes = &tag_data[pos..pos + 4];
            if id_bytes.iter().all(|b| *b == 0) {
                break;
            }
            let id = match std::str::from_utf8(id_bytes) {
                Ok(s) if s.trim().is_empty() => break,
                Ok(s) => s.to_string(),
                Err(_) => break,
            };
            let size = frame_from_synchsafe([
                tag_data[pos + 4],
                tag_data[pos + 5],
                tag_data[pos + 6],
                tag_data[pos + 7],
            ]);
            if size == 0 || pos + 10 + size > tag_data.len() {
                break;
            }
            let content = &tag_data[pos + 10..pos + 10 + size];
            raw_frames.push((id, content.to_vec()));
            pos += 10 + size;
        }
        let mut pictures: Vec<(FrameKey, Vec<TagValue>)> = Vec::new();
        let mut non_picture: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
        for (k, vals) in updated.clone().into_iter() {
            if k == FrameKey::AttachedPicture {
                pictures.push((k, vals));
            } else {
                non_picture.insert(k, vals);
            }
        }

        let mut flattened: HashMap<FrameKey, TagValue> = HashMap::new();
        for (k, vals) in non_picture.into_iter() {
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
                    .join("\u{0}");
                flattened.insert(k, TagValue::Text(joined));
            } else {
                flattened.insert(k, vals[0].clone());
            }
        }
        let raw_updates = id3v24_tags_to_raw(&flattened);
        let mut updated_keys: Vec<String> = updated
            .keys()
            .map(|k| crate::tag_manager::id3::utils::id3v24_code(*k).to_string())
            .collect();
        if !pictures.is_empty() {
            updated_keys.push("APIC".to_string());
        }
        raw_frames.retain(|(id, _)| !updated_keys.iter().any(|k| k == id));

        for (k, v) in raw_updates {
            match v {
                TagValue::Text(t) => {
                    if !t.is_empty() {
                        if matches!(k, "USLT" | "SYLT" | "COMM") {
                            let payload = crate::tag_manager::id3::lyrics::encode(k, &t).map_err(
                                |message| {
                                    BackendError::WriteFailed(TagError {
                                        path: file_path.to_string_lossy().into_owned(),
                                        public_message: "Invalid lyrics".into(),
                                        internal_message: message,
                                    })
                                },
                            )?;
                            raw_frames.push((k.to_string(), payload));
                            continue;
                        }

                        if k == "TXXX" || k == "WXXX" {
                            let (desc, val) = match t.split_once('=') {
                                Some((d, v)) => (d.to_string(), v.to_string()),
                                None => (String::new(), t.clone()),
                            };
                            let mut payload = Vec::new();
                            payload.push(0x00);
                            payload.extend_from_slice(desc.as_bytes());
                            payload.push(0x00);
                            payload.extend_from_slice(val.as_bytes());
                            raw_frames.push((k.to_string(), payload));
                        } else {
                            let encoded = if k.starts_with('W') {
                                t.as_bytes().to_vec()
                            } else {
                                encode_text_payload(&t, false)
                            };
                            raw_frames.push((k.to_string(), encoded));
                        }
                    }
                }
                TagValue::Picture {
                    mime,
                    data,
                    picture_type,
                    description,
                } => {
                    let encoded = tag_manager::id3::v2_3::utils::encode_img_payload(
                        &mime,
                        picture_type.unwrap_or(3),
                        description.as_deref().unwrap_or(""),
                        &data,
                    );
                    raw_frames.push((k.to_string(), encoded));
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
            raw_frames.push(("COMM".to_string(), payload));
        }
        for (_k, vals) in pictures.into_iter() {
            for v in vals {
                if let TagValue::Picture {
                    mime,
                    data,
                    picture_type,
                    description,
                } = v
                {
                    let encoded = tag_manager::id3::v2_3::utils::encode_img_payload(
                        &mime,
                        picture_type.unwrap_or(3),
                        description.as_deref().unwrap_or(""),
                        &data,
                    );
                    raw_frames.push(("APIC".to_string(), encoded));
                }
            }
        }
        raw_frames.extend(crate::tag_manager::id3::v2_common::custom_frames(
            &updated, false,
        ));
        let frames_vec = raw_frames
            .iter()
            .map(|(id, content)| build_frame_v24(id, content))
            .collect::<Vec<_>>();
        let total_size: usize = frames_vec.iter().map(|f| f.len()).sum();
        let header_new = create_header_with_version(4, total_size);
        let mut id3_tag = Vec::with_capacity(10 + total_size);
        id3_tag.extend_from_slice(&header_new);
        for fr in frames_vec {
            id3_tag.extend_from_slice(&fr);
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
                public_message: "Failed to seek to start of file".to_string(),
                internal_message: "Failed to seek to start of file".to_string(),
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
