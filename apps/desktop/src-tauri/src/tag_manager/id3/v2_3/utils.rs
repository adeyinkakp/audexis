use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub(crate) use super::super::v2_common::{
    build_frame, decode_text_payload, encode_img_payload, encode_text_payload, split_encoded_text,
    to_synchsafe,
};

pub fn ensure_header(file_path: &PathBuf) -> std::io::Result<bool> {
    let mut file = File::open(file_path)?;
    let mut header = [0u8; 10];
    file.read_exact(&mut header)?;
    Ok(&header[..3] == b"ID3" && matches!(header[3..5], [3, 0] | [4, 0]))
}

pub(crate) use super::super::v2_common::create_header as create_header_with_version;

pub(crate) fn create_header(tag_size: usize) -> [u8; 10] {
    super::super::v2_common::create_header(3, tag_size)
}
