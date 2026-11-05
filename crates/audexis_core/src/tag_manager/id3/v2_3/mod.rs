use crate::tag_manager::id3::utils::{id3v23_code, id3v23_key, raw_to_tags, tags_to_raw};
use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::traits::TagFormat;
use crate::tag_manager::utils::{FrameKey, TagMap, TagValue, UserTextEntry, UserUrlEntry};
use std::collections::HashMap;
use std::io::Read;

use crate::tag_manager::id3::v2_3::utils::{build_frame, create_header, encode_text_payload};
use crate::tag_manager::id3::v2_3::utils::{decode_text_payload, split_encoded_text};

pub mod utils;

#[derive(Debug, Clone)]
pub struct V2_3 {}
fn from_synchsafe(bytes: [u8; 4]) -> usize {
    ((bytes[0] as usize) << 21)
        | ((bytes[1] as usize) << 14)
        | ((bytes[2] as usize) << 7)
        | (bytes[3] as usize)
}

impl TagFormat for V2_3 {
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
        updated_tags: HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        self.write_metadata(file_path, updated_tags)
    }
}

mod read;

mod write;
