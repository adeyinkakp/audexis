use super::*;

impl V2_2 {
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
            if matches!(id.as_str(), "ULT" | "SLT") {
                if let Some(text) = crate::tag_manager::id3::lyrics::decode(&id, content) {
                    raw.entry(id).or_default().push(TagValue::Text(text));
                }
            } else if id == "COM" {
                if let Some(text) = crate::tag_manager::id3::lyrics::decode("USLT", content) {
                    raw.entry(id).or_default().push(TagValue::Text(text));
                }
            } else if id == "TXX" || id == "WXX" {
                if !content.is_empty() {
                    let encoding = content[0];
                    let rest = &content[1..];
                    let (desc_bytes, value_bytes) = split_encoded_text(encoding, rest);
                    let description = decode_text_payload(encoding, desc_bytes);
                    let value =
                        decode_text_payload(if id == "WXX" { 0 } else { encoding }, value_bytes);
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
            } else if id.starts_with('W') {
                raw.entry(id)
                    .or_default()
                    .push(TagValue::Text(decode_text_payload(0, content)));
            } else if id.starts_with('T') {
                if !content.is_empty() {
                    let encoding = content[0];
                    let text = decode_text_payload(encoding, &content[1..]);
                    let key = id3v22_key(&id);
                    if (key.is_some() && key.unwrap().is_multi_valued()) && text.contains('\0') {
                        for part in text.split('\0') {
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
                let (description_bytes, image_data) =
                    split_encoded_text(content[0], &content[idx..]);
                let description_text = decode_text_payload(content[0], description_bytes);
                let description = (!description_text.is_empty()).then_some(description_text);
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
}
