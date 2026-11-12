use crate::tag_manager::tag_backend::BackendError;
use crate::tag_manager::traits::Formats;

use crate::tag_manager::tag_backend::TagError;
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
mod keys;
pub use keys::*;
mod values;
pub use values::*;
mod conversions;
mod key_names;
pub use conversions::*;
mod files;
pub use files::*;
