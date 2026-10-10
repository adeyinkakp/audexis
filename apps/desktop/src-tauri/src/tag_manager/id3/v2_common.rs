fn is_latin1(text: &str) -> bool {
    text.chars().all(|character| (character as u32) <= 0xFF)
}

pub(crate) fn to_synchsafe(size: u32) -> [u8; 4] {
    [
        ((size >> 21) & 0x7F) as u8,
        ((size >> 14) & 0x7F) as u8,
        ((size >> 7) & 0x7F) as u8,
        (size & 0x7F) as u8,
    ]
}

pub(crate) fn encode_text_payload(text: &str, prefer_utf16: bool) -> Vec<u8> {
    if !prefer_utf16 && is_latin1(text) {
        let mut output = Vec::with_capacity(1 + text.len());
        output.push(0x00);
        output.extend(text.chars().map(|character| character as u8));
        output
    } else {
        let mut output = Vec::with_capacity(3 + text.len() * 2);
        output.extend_from_slice(&[0x01, 0xFF, 0xFE]);
        for code_unit in text.encode_utf16() {
            output.extend_from_slice(&code_unit.to_le_bytes());
        }
        output
    }
}

pub(crate) fn decode_text_payload(encoding: u8, bytes: &[u8]) -> String {
    match encoding {
        0x00 => bytes.iter().map(|byte| char::from(*byte)).collect(),
        0x01 | 0x02 => {
            let big_endian = encoding == 2 || bytes.starts_with(&[0xFE, 0xFF]);
            let bytes = bytes
                .strip_prefix(&[0xFF, 0xFE])
                .or_else(|| bytes.strip_prefix(&[0xFE, 0xFF]))
                .unwrap_or(bytes);
            let (code_units, _) = bytes.as_chunks::<2>();
            String::from_utf16_lossy(
                &code_units
                    .iter()
                    .map(|chunk| {
                        if big_endian {
                            u16::from_be_bytes(*chunk)
                        } else {
                            u16::from_le_bytes(*chunk)
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        }
        _ => String::from_utf8_lossy(bytes).to_string(),
    }
}

pub(crate) fn split_encoded_text(encoding: u8, bytes: &[u8]) -> (&[u8], &[u8]) {
    if matches!(encoding, 0x01 | 0x02) {
        let start =
            usize::from(bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF])) * 2;
        let end = (start..bytes.len().saturating_sub(1))
            .step_by(2)
            .find(|&index| bytes[index..index + 2] == [0, 0])
            .unwrap_or(bytes.len());
        let value_start = (end + 2).min(bytes.len());
        (&bytes[..end], &bytes[value_start..])
    } else {
        let end = bytes
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(bytes.len());
        let value_start = (end + 1).min(bytes.len());
        (&bytes[..end], &bytes[value_start..])
    }
}

pub(crate) fn encode_img_payload(
    mime_type: &str,
    picture_type: u8,
    description: &str,
    image_data: &[u8],
) -> Vec<u8> {
    let mut payload =
        Vec::with_capacity(3 + mime_type.len() + description.len() + image_data.len());
    let encoded_description = encode_text_payload(description, false);
    payload.push(encoded_description[0]);
    payload.extend_from_slice(mime_type.as_bytes());
    payload.push(0x00);
    payload.push(picture_type);
    payload.extend_from_slice(&encoded_description[1..]);
    payload.push(0x00);
    if encoded_description[0] == 1 {
        payload.push(0x00);
    }
    payload.extend_from_slice(image_data);
    payload
}

pub(crate) fn build_frame(id: &str, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(10 + payload.len());
    frame.extend_from_slice(id.as_bytes());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&[0x00, 0x00]);
    frame.extend_from_slice(payload);
    frame
}

pub(crate) fn create_header(version: u8, tag_size: usize) -> [u8; 10] {
    let mut header = [0u8; 10];
    header[0..3].copy_from_slice(b"ID3");
    header[3] = version;
    header[6..10].copy_from_slice(&to_synchsafe(tag_size as u32));
    header
}

pub(crate) fn validate_for_write(path: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let mut file = crate::utils::library_files::open(path).map_err(|error| error.to_string())?;
    let mut header = [0; 10];
    file.read_exact(&mut header)
        .map_err(|error| error.to_string())?;
    if header[5] != 0 || header[6..].iter().any(|byte| byte & 0x80 != 0) {
        return Err("Editing ID3 tags with header flags is not supported safely".into());
    }
    let size = header[6..]
        .iter()
        .fold(0usize, |size, byte| (size << 7) | usize::from(*byte));
    if size as u64 + 10 > file.metadata().map_err(|error| error.to_string())?.len() {
        return Err("Truncated ID3 tag".into());
    }
    let mut data = vec![0; size];
    file.read_exact(&mut data)
        .map_err(|error| error.to_string())?;
    let (id_len, header_len) = if header[3] == 2 { (3, 6) } else { (4, 10) };
    let mut pos = 0;
    let mut ids = std::collections::HashSet::new();
    while pos < data.len() {
        if data[pos..].iter().all(|byte| *byte == 0) {
            break;
        }
        if pos + header_len > data.len() {
            return Err("Truncated ID3 frame header".into());
        }
        let id = &data[pos..pos + id_len];
        if !id
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            return Err("Invalid ID3 frame identifier".into());
        }
        if header[3] == 2 && !ids.insert(id.to_vec()) && id != b"TXX" && id != b"WXX" {
            return Err("Editing duplicate ID3v2.2 frames is not supported safely".into());
        }
        if header_len == 10 && data[pos + 8..pos + 10] != [0, 0] {
            return Err("Editing flagged ID3 frames is not supported safely".into());
        }
        let size_bytes = &data[pos + id_len..pos + if header_len == 10 { 8 } else { 6 }];
        if header[3] == 4 && size_bytes.iter().any(|byte| byte & 0x80 != 0) {
            return Err("Invalid ID3v2.4 frame size".into());
        }
        let shift = if header[3] == 4 { 7 } else { 8 };
        let size = size_bytes
            .iter()
            .fold(0usize, |size, byte| (size << shift) | usize::from(*byte));
        pos += header_len;
        if size > data.len() - pos {
            return Err("Truncated ID3 frame".into());
        }
        pos += size;
    }
    Ok(())
}

pub(crate) fn custom_frames(
    tags: &std::collections::HashMap<
        crate::tag_manager::utils::FrameKey,
        Vec<crate::tag_manager::utils::TagValue>,
    >,
    v22: bool,
) -> Vec<(String, Vec<u8>)> {
    use crate::tag_manager::utils::{FrameKey, TagValue};
    let mut frames = Vec::new();
    for (key, code) in [
        (FrameKey::UserDefinedText, if v22 { "TXX" } else { "TXXX" }),
        (FrameKey::UserDefinedURL, if v22 { "WXX" } else { "WXXX" }),
    ] {
        for value in tags.get(&key).into_iter().flatten() {
            let (description, value, url) = match value {
                TagValue::UserText(entry) => (&entry.description, &entry.value, false),
                TagValue::UserUrl(entry) => (&entry.description, &entry.url, true),
                _ => continue,
            };
            let mut payload = encode_text_payload(description, true);
            payload.extend_from_slice(&[0, 0]);
            if url {
                payload.extend(value.chars().map(|character| character as u8));
            } else {
                payload.extend_from_slice(&encode_text_payload(value, true)[1..]);
            }
            frames.push((code.to_string(), payload));
        }
    }
    frames
}
