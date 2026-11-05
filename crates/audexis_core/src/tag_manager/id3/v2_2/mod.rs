use crate::tag_manager::id3::utils::{id3v22_key, id3v22_raw_to_tags, id3v22_tags_to_raw};
use crate::tag_manager::id3::v2_3::utils::{
    create_header_with_version, decode_text_payload, encode_text_payload, split_encoded_text,
};
use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::traits::TagFormat;
use crate::tag_manager::utils::{FrameKey, TagValue, UserTextEntry, UserUrlEntry};
use std::collections::HashMap;
use std::io::Read;

#[derive(Debug, Clone)]
pub struct V2_2 {}

impl V2_2 {
    fn frame_size(bytes: &[u8]) -> usize {
        ((bytes[0] as usize) << 16) | ((bytes[1] as usize) << 8) | (bytes[2] as usize)
    }
}

impl TagFormat for V2_2 {
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
