use super::traits::{Formats, TagFamily, TagFormat};
use super::utils::{FrameKey, TagValue};
use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::vorbis_comments::utils;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Flac;

impl TagFamily for Flac {
    fn new() -> Self {
        Self
    }
    fn get_release_class(&self, version: &Formats) -> Option<Box<dyn TagFormat>> {
        match version {
            Formats::Flac => Some(Box::new(FlacFormat::new())),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
enum FlacBlockType {
    StreamInfo,
    Padding,
    Application,
    SeekTable,
    VorbisComment,
    CueSheet,
    Picture,
    Unknown(u8),
}

impl From<u8> for FlacBlockType {
    fn from(ty: u8) -> Self {
        match ty {
            0 => FlacBlockType::StreamInfo,
            1 => FlacBlockType::Padding,
            2 => FlacBlockType::Application,
            3 => FlacBlockType::SeekTable,
            4 => FlacBlockType::VorbisComment,
            5 => FlacBlockType::CueSheet,
            6 => FlacBlockType::Picture,
            other => FlacBlockType::Unknown(other),
        }
    }
}
#[derive(Debug, Clone)]
struct FlacFormat;

impl TagFormat for FlacFormat {
    fn new() -> Self {
        Self
    }
    fn get_tags(
        &self,
        file_path: &PathBuf,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        let mut data: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
        let b = fs::read(file_path);
        if b.is_err() {
            print!("I guess");
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not read file".to_string(),
                internal_message: "Failed to read file".to_string(),
            }));
        }
        let b = b.unwrap();

        if b.len() < 4 || &b[0..4] != b"fLaC" {
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Not a FLAC file".to_string(),
                internal_message: "File does not start with fLaC signature".to_string(),
            }));
        }
        let pos = 4;
        let mut offset = pos;
        while offset < b.len() {
            if offset + 4 > b.len() {
                return Err(BackendError::ReadFailed(TagError {
                    path: file_path.to_string_lossy().to_string(),
                    public_message: "Truncated FLAC metadata block".to_string(),
                    internal_message: "Metadata block header is shorter than four bytes"
                        .to_string(),
                }));
            }
            let is_last = (b[offset] & 0x80) != 0;

            let block_type = FlacBlockType::from(b[offset] & 0x7F);
            let block_length = ((b[offset + 1] as u32) << 16)
                | ((b[offset + 2] as u32) << 8)
                | (b[offset + 3] as u32);
            offset += 4;
            if offset + block_length as usize > b.len() {
                return Err(BackendError::ReadFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "Bad flac block length".to_string(),
                    internal_message: "Bad flac block length".to_string(),
                }));
            }
            let block_data = &b[offset..offset + block_length as usize];

            offset += block_length as usize;
            if let FlacBlockType::VorbisComment = block_type {
                let tags = utils::parse_comments(block_data);
                if tags.is_err() {
                    return Err(BackendError::ReadFailed(TagError {
                        path: file_path.to_str().unwrap_or("").to_string(),
                        public_message: "Failed to parse vorbis comments".to_string(),
                        internal_message: "Failed to parse vorbis comments".to_string(),
                    }));
                }
                let tags = tags.unwrap();
                data.extend(tags.into_iter());
            } else if FlacBlockType::Picture == block_type {
                let pic = utils::parse_picture(block_data);
                if pic.is_err() {
                    return Err(BackendError::ReadFailed(TagError {
                        path: file_path.to_str().unwrap_or("").to_string(),
                        public_message: "Failed to parse picture block".to_string(),
                        internal_message: "Failed to parse picture block".to_string(),
                    }));
                }
                let pic: TagValue = pic.unwrap();
                data.entry(FrameKey::AttachedPicture).or_default().push(pic);
            }

            if is_last {
                break;
            }
        }

        Ok(data)
    }
    fn write_tags(
        &self,
        file_path: &PathBuf,
        updated_tags: HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        let b = fs::read(&file_path);
        if b.is_err() {
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not read file".to_string(),
                internal_message: "Failed to read file".to_string(),
            }));
        }
        let old_tags = self.get_tags(file_path);
        if old_tags.is_err() {
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not read tags".to_string(),
                internal_message: "Failed to read tags".to_string(),
            }));
        }
        let old_tags = old_tags.unwrap();
        let b = b.unwrap();

        if b.len() < 4 || &b[0..4] != b"fLaC" {
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Not a FLAC file".to_string(),
                internal_message: "File does not start with fLaC header".to_string(),
            }));
        }
        let mut tags: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();

        tags.extend(updated_tags.into_iter());

        old_tags.into_iter().for_each(|(k, v)| {
            if !tags.contains_key(&k) {
                tags.insert(k, v);
            }
        });
        let payload = utils::build_comments(&tags, false);

        let mut all_blocks: Vec<FlacBlock> = vec![];
        let mut seen_vorbis = false;
        let mut offset = 4;

        while offset < b.len() {
            if offset + 4 > b.len() {
                return Err(BackendError::ReadFailed(TagError {
                    path: file_path.to_string_lossy().to_string(),
                    public_message: "Truncated FLAC metadata block".to_string(),
                    internal_message: "Metadata block header is shorter than four bytes"
                        .to_string(),
                }));
            }
            let is_last = (b[offset] & 0x80) != 0;
            let block_type_raw = b[offset] & 0x7F;
            let block_type = FlacBlockType::from(block_type_raw);

            let block_length = ((b[offset + 1] as u32) << 16)
                | ((b[offset + 2] as u32) << 8)
                | (b[offset + 3] as u32);

            offset += 4;
            if offset + block_length as usize > b.len() {
                return Err(BackendError::ReadFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "Invalid block length".to_string(),
                    internal_message: "Block length exceeds file size".to_string(),
                }));
            }
            let block_data = &b[offset..offset + block_length as usize];
            offset += block_length as usize;

            match block_type {
                FlacBlockType::VorbisComment => {
                    seen_vorbis = true;
                    all_blocks.push(FlacBlock {
                        block_type: FlacBlockType::VorbisComment,

                        data: payload.clone(),
                    });
                }
                FlacBlockType::Picture => {}
                _ => {
                    all_blocks.push(FlacBlock {
                        block_type,

                        data: block_data.to_vec(),
                    });
                }
            }

            if is_last {
                break;
            }
        }

        if !seen_vorbis {
            let index = all_blocks
                .iter()
                .position(|b| b.block_type == FlacBlockType::StreamInfo);

            if index.is_none() {
                return Err(BackendError::ReadFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "Not a valid FLAC file".to_string(),
                    internal_message: "No STREAMINFO block found in FLAC file".to_string(),
                }));
            }
            let index = index.unwrap();
            all_blocks.insert(
                index + 1,
                FlacBlock {
                    block_type: FlacBlockType::VorbisComment,

                    data: payload.clone(),
                },
            );
        }
        for payload in utils::build_picture_tag(&tags) {
            all_blocks.push(FlacBlock {
                block_type: FlacBlockType::Picture,

                data: payload,
            });
        }

        let audio_start = offset;
        let audio_data = &b[audio_start..];

        let mut out: Vec<u8> = Vec::new();
        out.extend(b"fLaC");
        for (i, block) in all_blocks.iter().enumerate() {
            let block_length = block.data.len() as u32;
            let is_last = if i == all_blocks.len() - 1 {
                0x80
            } else {
                0x00
            };
            let block_type_byte = match block.block_type {
                FlacBlockType::StreamInfo => 0,
                FlacBlockType::Padding => 1,
                FlacBlockType::Application => 2,
                FlacBlockType::SeekTable => 3,
                FlacBlockType::VorbisComment => 4,
                FlacBlockType::CueSheet => 5,
                FlacBlockType::Picture => 6,
                FlacBlockType::Unknown(t) => t,
            };
            out.push(is_last | block_type_byte);
            out.push(((block_length >> 16) & 0xFF) as u8);
            out.push(((block_length >> 8) & 0xFF) as u8);
            out.push((block_length & 0xFF) as u8);
            out.extend(&block.data);
        }

        out.extend(audio_data);
        let write_result = fs::write(file_path, out);

        if write_result.is_err() {
            println!("err");
            return Err(BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not write tags".to_string(),
                internal_message: "Failed to write tags".to_string(),
            }));
        }
        Ok(())
    }
}

struct FlacBlock {
    block_type: FlacBlockType,

    data: Vec<u8>,
}
