use super::*;

impl V2_4 {
    pub(super) fn read_metadata(
        &self,
        file_path: &std::path::Path,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        let mut file = crate::library_files::open(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unable to open and read file".to_string(),
                internal_message: "Unable to open and read file".to_string(),
            })
        })?;
        let mut header = [0u8; 10];
        if file.read_exact(&mut header).is_err() {
            return Ok(HashMap::new());
        }
        if &header[0..3] != b"ID3" || header[3] != 4 {
            return Ok(HashMap::new());
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
        let mut pos = 0usize;
        let mut raw: HashMap<String, Vec<TagValue>> = HashMap::new();
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
            let _flags = &tag_data[pos + 8..pos + 10];
            if size == 0 || pos + 10 + size > tag_data.len() {
                break;
            }
            let content = &tag_data[pos + 10..pos + 10 + size];
            if matches!(id.as_str(), "USLT" | "SYLT") {
                if let Some(text) = crate::tag_manager::id3::lyrics::decode(&id, content) {
                    raw.entry(id).or_default().push(TagValue::Text(text));
                }
            } else if id == "COMM" {
                if let Some(text) = crate::tag_manager::id3::lyrics::decode("USLT", content) {
                    raw.entry(id).or_default().push(TagValue::Text(text));
                }
            } else if id == "TXXX" || id == "WXXX" {
                if !content.is_empty() {
                    let encoding = content[0];
                    let rest = &content[1..];
                    let (desc_bytes, value_bytes) = split_encoded_text(encoding, rest);
                    let description = decode_text_payload(encoding, desc_bytes);
                    let value =
                        decode_text_payload(if id == "WXXX" { 0 } else { encoding }, value_bytes);
                    let entry = if id == "TXXX" {
                        TagValue::UserText(UserTextEntry { description, value })
                    } else {
                        TagValue::UserUrl(UserUrlEntry {
                            description,
                            url: value,
                        })
                    };
                    raw.entry(id).or_default().push(entry);
                }
            } else if id.starts_with('W') {
                raw.entry(id)
                    .or_default()
                    .push(TagValue::Text(decode_text_payload(0, content)));
            } else if id.starts_with('T') {
                if !content.is_empty() {
                    let encoding = content[0];
                    let text = decode_text_payload(encoding, &content[1..]);
                    let key = id3v24_key(&id);

                    if (key.is_some() && key.unwrap().is_multi_valued()) && text.contains('\u{0}') {
                        for part in text.split('\u{0}') {
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
            } else if id == "APIC" && !content.is_empty() {
                let _encoding = content[0];
                if let Some(mime_null_rel) = content[1..].iter().position(|&b| b == 0x00) {
                    let mime_start = 1usize;
                    let mime_end = 1 + mime_null_rel;
                    let mime_type =
                        String::from_utf8_lossy(&content[mime_start..mime_end]).to_string();
                    let pic_type_index = mime_end + 1;
                    if pic_type_index >= content.len() {
                        break;
                    }
                    let picture_type = content[pic_type_index];

                    let description_start = pic_type_index + 1;
                    let (description_bytes, image_data) =
                        split_encoded_text(content[0], &content[description_start..]);
                    let description_text = decode_text_payload(content[0], description_bytes);
                    let description = (!description_text.is_empty()).then_some(description_text);
                    raw.entry(id).or_default().push(TagValue::Picture {
                        mime: mime_type,
                        data: image_data.to_vec(),
                        picture_type: Some(picture_type),
                        description,
                    });
                }
            }
            pos += 10 + size;
        }
        Ok(id3v24_raw_to_tags(&raw))
    }
}
