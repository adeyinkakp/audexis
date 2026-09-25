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
        0x00 => String::from_utf8_lossy(bytes).to_string(),
        0x01 => {
            let bytes = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
            String::from_utf16_lossy(
                &bytes
                    .chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect::<Vec<_>>(),
            )
        }
        _ => String::from_utf8_lossy(bytes).to_string(),
    }
}

pub(crate) fn split_encoded_text(encoding: u8, bytes: &[u8]) -> (&[u8], &[u8]) {
    if encoding == 0x01 {
        let start = usize::from(bytes.starts_with(&[0xFF, 0xFE])) * 2;
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
    payload.push(0x00);
    payload.extend_from_slice(mime_type.as_bytes());
    payload.push(0x00);
    payload.push(picture_type);
    payload.extend_from_slice(description.as_bytes());
    payload.push(0x00);
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
