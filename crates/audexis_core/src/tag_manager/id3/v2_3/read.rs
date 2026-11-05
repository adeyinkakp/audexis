use super::*;

impl V2_3 {
    pub(super) fn read_metadata(
        &self,
        file_path: &std::path::Path,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        let has_header = utils::ensure_header(file_path);

        match has_header {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                return Err(BackendError::ReadFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "File does not contain ID3 tag".to_string(),
                    internal_message: "File does not contain ID3 tag".to_string(),
                }));
            }
        }
        let mut file = crate::library_files::open(file_path).map_err(|_| {
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

            if matches!(frame_id.as_str(), "USLT" | "SYLT") {
                if let Some(text) = crate::tag_manager::id3::lyrics::decode(&frame_id, content) {
                    tags.entry(frame_id).or_default().push(TagValue::Text(text));
                }
            } else if frame_id == "COMM" {
                if let Some(text) = crate::tag_manager::id3::lyrics::decode("USLT", content) {
                    tags.entry(frame_id).or_default().push(TagValue::Text(text));
                }
            } else if frame_id == "TXXX" || frame_id == "WXXX" {
                if !content.is_empty() {
                    let encoding = content[0];
                    let rest = &content[1..];
                    let (desc_bytes, value_bytes) = split_encoded_text(encoding, rest);
                    let description = decode_text_payload(encoding, desc_bytes);
                    let value = decode_text_payload(
                        if frame_id == "WXXX" { 0 } else { encoding },
                        value_bytes,
                    );
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
            } else if frame_id.starts_with('W') {
                tags.entry(frame_id)
                    .or_default()
                    .push(TagValue::Text(decode_text_payload(0, content)));
            } else if frame_id.starts_with('T') {
                if !content.is_empty() {
                    let encoding = content[0];
                    let raw_string = decode_text_payload(encoding, &content[1..]);

                    let key = id3v23_key(&frame_id);

                    if (key.is_some() && key.unwrap().is_multi_valued())
                        && raw_string.contains('\0')
                    {
                        for part in raw_string.split('\0').map(|s| s.trim()) {
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
                    let (description_bytes, image_data) =
                        split_encoded_text(content[0], &content[description_start..]);
                    let description_text = decode_text_payload(content[0], description_bytes);
                    let description = (!description_text.is_empty()).then_some(description_text);
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
}
