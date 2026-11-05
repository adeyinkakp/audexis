use crate::tag_manager;
use crate::tag_manager::utils::FrameKey;
use crate::tag_manager::utils::TagValue;
use std::collections::HashMap;
use std::sync::LazyLock as Lazy;

mod codes;
pub use codes::*;
mod atom_flags;
pub use atom_flags::*;
mod freeform;
pub use freeform::*;
mod freeform_names;
pub use freeform_names::*;
mod conversion;
pub use conversion::*;
