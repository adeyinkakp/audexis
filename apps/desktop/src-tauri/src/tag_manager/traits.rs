use serde::Serialize;

use crate::tag_manager::tag_backend::BackendError;
use crate::tag_manager::utils::{FrameKey, FreeformTag, TagValue};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::path::Path;
use std::{fmt, write};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Formats {
    Id3v22,
    Id3v23,
    Id3v24,
    Id3v11,
    Id3v10,
    Itunes,
    Flac,
    Ogg,

    Riff,
    Unknown,
}
impl Display for Formats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Formats::Id3v10 => "id3v1.0",
            Formats::Id3v11 => "id3v1.1",
            Formats::Id3v22 => "id3v2.2",
            Formats::Id3v23 => "id3v2.3",
            Formats::Id3v24 => "id3v2.4",
            Formats::Itunes => "Itunes",
            Formats::Ogg => "Ogg",
            Formats::Flac => "FLAC",
            Formats::Riff => "RIFF",
            Formats::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}
pub trait TagFamily {
    fn new() -> Self
    where
        Self: Sized;
    ///  Returns a vector of supported tag formats for this tag family.
    fn get_release_class(&self, version: &Formats) -> Option<Box<dyn TagFormat>>;
}

pub trait TagFormat: Debug {
    fn new() -> Self
    where
        Self: Sized;
    ///  Reads the tags from the specified file path and returns a `HashMap` where the keys are `FrameKey` and the values are vectors of `TagValue`. If an error occurs during the reading process, it returns a `BackendError`.
    fn get_tags(&self, file_path: &Path) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError>;
    fn get_freeforms(&self, _file_path: &Path) -> Result<Vec<FreeformTag>, BackendError> {
        Ok(vec![])
    }
    ///  Writes the provided tags to the specified file path. The `updated_tags` parameter is a `HashMap` where the keys are `FrameKey` and the values are vectors of `TagValue`. If an error occurs during the writing process, it returns a `BackendError`.
    fn write_tags(
        &self,
        file_path: &Path,
        updated_tags: HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError>;
}

impl Display for dyn TagFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
