mod write;
use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::vorbis_comments;

use super::traits::{Formats, TagFamily, TagFormat};
use super::utils::{FrameKey, TagValue, replace_tmp, temp_path_for};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use utils::{
    PacketAssembler, PacketChunker, SerialMuxState, StreamClassifier, StreamKind,
    build_page_for_serial, read_page, write_page,
};
mod utils;
#[derive(Debug, Clone)]
pub struct Ogg;
impl TagFamily for Ogg {
    fn new() -> Self {
        Self
    }
    fn get_release_class(&self, version: &Formats) -> Option<Box<dyn TagFormat>> {
        match version {
            Formats::Ogg => Some(Box::new(OggFormat::new())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct OggFormat;

impl TagFormat for OggFormat {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }
    fn get_tags(
        &self,
        file_path: &std::path::Path,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        let file = crate::library_files::open(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not open file".to_string(),
                internal_message: "Failed to open file".to_string(),
            })
        })?;

        let packet = utils::extract_comment_packet(file).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to find Vorbis/Opus comment packet".to_string(),
                internal_message: "Failed to find Vorbis/Opus comment packet".to_string(),
            })
        })?;

        let payload = if packet.len() >= 7 && &packet[0..7] == b"\x03vorbis" {
            packet[7..].to_vec()
        } else if packet.len() >= 8 && &packet[0..8] == b"OpusTags" {
            packet[8..].to_vec()
        } else {
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unknown comment packet format".to_string(),
                internal_message: "Unknown comment packet format".to_string(),
            }));
        };

        let comment = vorbis_comments::utils::parse_comments(&payload);
        if comment.is_err() {
            println!("Error parsing comments");
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to parse Vorbis comments".to_string(),
                internal_message: "Failed to parse Vorbis comments".to_string(),
            }));
        }
        let comment = comment.unwrap();

        Ok(comment)
    }
    fn write_tags(
        &self,
        file_path: &std::path::Path,
        tags: HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        write::write_tags(file_path, tags)
    }
}

#[cfg(test)]
mod metadata_tests;
