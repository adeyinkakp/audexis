use crate::tag_manager;
use crate::tag_manager::itunes::utils::{
    FREEFORM_REVERSE_MAP, get_atom_flag, parse_number_pair, raw_to_tags,
};
use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::traits::TagFormat;
use crate::tag_manager::utils::{FrameKey, FreeformTag, TagValue};
use std::collections::HashMap;
use std::fs;

mod atoms;
use atoms::Atom;

#[derive(Debug, Clone)]
pub struct V0 {}
impl V0 {
    fn parse_atoms(buffer: &[u8], start: u64, end: u64) -> Vec<Atom> {
        atoms::parse_atoms(buffer, start, end)
    }

    fn ensure_ilst_atom(buffer: &[u8]) -> Result<Atom, ()> {
        atoms::find_ilst(buffer).ok_or(())
    }
}

impl TagFormat for V0 {
    fn new() -> Self {
        Self {}
    }

    fn get_tags(
        &self,
        file_path: &std::path::Path,
    ) -> Result<HashMap<tag_manager::utils::FrameKey, Vec<TagValue>>, BackendError> {
        self.read_metadata(file_path)
    }
    fn write_tags(
        &self,
        file_path: &std::path::Path,
        updated_tags: HashMap<tag_manager::utils::FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        self.write_metadata(file_path, updated_tags)
    }

    fn get_freeforms(&self, file_path: &std::path::Path) -> Result<Vec<FreeformTag>, BackendError> {
        let buffer = crate::library_files::read(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                internal_message: "Unable to open and read file".to_string(),
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unable to read file.".to_string(),
            })
        })?;
        let Ok(ilst_atom) = V0::ensure_ilst_atom(&buffer) else {
            return Ok(Vec::new());
        };
        let ilst_sub_atoms = V0::parse_atoms(&ilst_atom.buffer, 8, ilst_atom.size);
        let mut out: Vec<FreeformTag> = Vec::new();
        for atom in &ilst_sub_atoms {
            if atom.atom_type != "----" {
                continue;
            }
            let mut cursor = atom.position + 8;
            let end = atom.position + atom.size;
            let mut mean: Option<String> = None;
            let mut name: Option<String> = None;
            let mut value_text: Option<String> = None;
            while cursor + 8 <= end {
                let sz = u32::from_be_bytes([
                    ilst_atom.buffer[cursor as usize],
                    ilst_atom.buffer[cursor as usize + 1],
                    ilst_atom.buffer[cursor as usize + 2],
                    ilst_atom.buffer[cursor as usize + 3],
                ]) as u64;
                if sz < 12 || cursor + sz > end {
                    break;
                }
                let t = &ilst_atom.buffer[(cursor + 4) as usize..(cursor + 8) as usize];
                let t_str = String::from_utf8_lossy(t);
                match &*t_str {
                    "mean" => {
                        let s = String::from_utf8_lossy(
                            &ilst_atom.buffer[(cursor + 12) as usize..(cursor + sz) as usize],
                        )
                        .to_string();
                        mean = Some(s);
                    }
                    "name" => {
                        let s = String::from_utf8_lossy(
                            &ilst_atom.buffer[(cursor + 12) as usize..(cursor + sz) as usize],
                        )
                        .to_string();
                        name = Some(s);
                    }
                    "data" if sz >= 16 => {
                        let s = String::from_utf8_lossy(
                            &ilst_atom.buffer[(cursor + 16) as usize..(cursor + sz) as usize],
                        )
                        .to_string();
                        value_text = Some(s);
                    }
                    _ => {}
                }
                cursor += sz;
            }
            if let (Some(mean), Some(name), Some(val)) = (mean, name, value_text)
                && FREEFORM_REVERSE_MAP
                    .get(&(mean.as_str(), name.as_str()))
                    .is_none()
            {
                out.push(FreeformTag {
                    mean,
                    name,
                    value: val,
                });
            }
        }
        Ok(out)
    }
}

mod encode;

mod rebuild;

mod offsets;

mod read;

mod write;
