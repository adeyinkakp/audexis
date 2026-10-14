use crate::tag_manager::traits;
use crate::tag_manager::traits::{Formats, TagFormat};

pub mod utils;
mod v0;
#[derive(Debug, Clone)]
pub struct Itunes {
    v0: v0::V0,
}

impl Itunes {
    pub fn new() -> Self {
        Self { v0: v0::V0::new() }
    }
}

impl traits::TagFamily for Itunes {
    fn new() -> Self {
        Self { v0: v0::V0::new() }
    }
    fn get_release_class(&self, version: &Formats) -> Option<Box<dyn TagFormat>> {
        match version {
            Formats::Itunes => Some(Box::new(self.v0.clone())),

            _ => None,
        }
    }
}
