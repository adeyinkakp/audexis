use super::*;

impl V0 {
    pub(super) fn read_metadata(
        &self,
        file_path: &std::path::Path,
    ) -> Result<HashMap<tag_manager::utils::FrameKey, Vec<TagValue>>, BackendError> {
        let Some(ilst_atom) = Self::read_ilst_atom(file_path)? else {
            return Ok(HashMap::new());
        };
        Self::tags_from_ilst(&ilst_atom)
    }

    pub(super) fn tags_from_ilst(
        ilst_atom: &Atom,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        let mut raw_entries = Vec::new();
        for atom in V0::parse_atoms(&ilst_atom.buffer, 8, ilst_atom.size) {
            let children = V0::parse_atoms(&atom.buffer, 8, atom.size);
            let key = if atom.atom_type == "----" {
                let component = |kind: &str| {
                    children
                        .iter()
                        .find(|child| child.atom_type == kind)
                        .and_then(|child| child.buffer.get(12..))
                        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                };
                let (Some(mean), Some(name)) = (component("mean"), component("name")) else {
                    continue;
                };
                format!("----:{mean}:{name}")
            } else {
                atom.atom_type.clone()
            };
            for data in children.iter().filter(|child| child.atom_type == "data") {
                let Some(payload) = data.buffer.get(16..) else {
                    continue;
                };
                let kind = u32::from_be_bytes(data.buffer[8..12].try_into().unwrap()) & 0x00ff_ffff;
                let custom_freeform = key
                    .strip_prefix("----:")
                    .and_then(|rest| rest.split_once(':'))
                    .is_some_and(|parts| !FREEFORM_REVERSE_MAP.contains_key(&parts));

                if custom_freeform && kind != 1 {
                    continue;
                }
                let value = match atom.atom_type.as_str() {
                    "trkn" | "disk" if payload.len() >= 6 => {
                        let number = u16::from_be_bytes([payload[2], payload[3]]);
                        let total = u16::from_be_bytes([payload[4], payload[5]]);
                        TagValue::Text(if total > 0 {
                            format!("{number}/{total}")
                        } else {
                            number.to_string()
                        })
                    }
                    "covr" => TagValue::Picture {
                        mime: if kind == 14 || payload.starts_with(b"\x89PNG") {
                            "image/png"
                        } else {
                            "image/jpeg"
                        }
                        .into(),
                        data: payload.to_vec(),
                        picture_type: None,
                        description: None,
                    },
                    _ if kind == 1 => TagValue::Text(String::from_utf8_lossy(payload).into_owned()),
                    _ if kind == 21 && !payload.is_empty() && payload.len() <= 8 => {
                        let value = payload
                            .iter()
                            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte));
                        TagValue::Text(value.to_string())
                    }
                    _ => continue,
                };
                raw_entries.push((key.clone(), value));
            }
        }
        let vec_map = raw_to_tags(&raw_entries);
        Ok(vec_map)
    }
}
