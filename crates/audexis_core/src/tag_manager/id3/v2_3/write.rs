use super::*;

impl V2_3 {
    pub(super) fn write_metadata(
        &self,
        file_path: &std::path::Path,
        updated_tags: HashMap<FrameKey, Vec<TagValue>>,
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
            if vec_vals.is_empty()
                || matches!(
                    k,
                    FrameKey::UserDefinedText | FrameKey::UserDefinedURL | FrameKey::Comments
                )
            {
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
                    .join("\0");
                flattened.insert(k, TagValue::Text(joined));
            } else {
                flattened.insert(k, vec_vals[0].clone());
            }
        }
        let raw_updated_tags = tags_to_raw(&flattened);

        let updated_keys: Vec<&str> = updated_tags
            .keys()
            .map(|key| {
                id3v23_code(match key {
                    FrameKey::Artists => FrameKey::Artist,
                    _ => *key,
                })
            })
            .collect();
        raw_frames.retain(|(id, _)| !updated_keys.iter().any(|key| *key == id));

        for (k, v) in raw_updated_tags {
            match v {
                TagValue::Text(text) => {
                    if !text.is_empty() {
                        if matches!(k, "USLT" | "SYLT" | "COMM") {
                            let payload = crate::tag_manager::id3::lyrics::encode(k, &text)
                                .map_err(|message| {
                                    BackendError::WriteFailed(TagError {
                                        path: file_path.to_string_lossy().into_owned(),
                                        public_message: "Invalid lyrics".into(),
                                        internal_message: message,
                                    })
                                })?;
                            raw_frames.push((k.to_string(), payload));
                            continue;
                        }

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
                            let encoded_text = if k.starts_with('W') {
                                text.as_bytes().to_vec()
                            } else {
                                encode_text_payload(&text, false)
                            };
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
        raw_frames.extend(crate::tag_manager::id3::v2_common::custom_frames(
            &updated_tags,
            false,
        ));
        for (index, value) in updated_tags
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
