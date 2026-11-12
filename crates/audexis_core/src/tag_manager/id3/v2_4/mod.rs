use crate::tag_manager;
use crate::tag_manager::id3::utils::{id3v24_key, id3v24_raw_to_tags, id3v24_tags_to_raw};
use crate::tag_manager::id3::v2_3::utils::{
    create_header_with_version, decode_text_payload, encode_text_payload, split_encoded_text,
    to_synchsafe,
};
use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::traits::TagFormat;
use crate::tag_manager::utils::{FrameKey, TagValue, UserTextEntry, UserUrlEntry};
use std::collections::HashMap;
use std::io::Read;

#[derive(Debug, Clone)]
pub struct V2_4 {}

fn from_synchsafe(bytes: [u8; 4]) -> usize {
    ((bytes[0] as usize) << 21)
        | ((bytes[1] as usize) << 14)
        | ((bytes[2] as usize) << 7)
        | (bytes[3] as usize)
}

fn frame_from_synchsafe(bytes: [u8; 4]) -> usize {
    from_synchsafe(bytes)
}

fn frame_to_synchsafe(size: usize) -> [u8; 4] {
    to_synchsafe(size as u32)
}

fn build_frame_v24(id: &str, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(10 + payload.len());
    frame.extend_from_slice(id.as_bytes());
    frame.extend_from_slice(&frame_to_synchsafe(payload.len()));
    frame.extend_from_slice(&[0x00, 0x00]);
    frame.extend_from_slice(payload);
    frame
}

impl TagFormat for V2_4 {
    fn new() -> Self {
        Self {}
    }
    fn get_tags(
        &self,
        file_path: &std::path::Path,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        self.read_metadata(file_path)
    }
    fn write_tags(
        &self,
        file_path: &std::path::Path,
        updated: HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        self.write_metadata(file_path, updated)
    }
}

mod read;

mod write;
