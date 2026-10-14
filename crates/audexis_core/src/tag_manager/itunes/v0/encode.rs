use super::*;

impl V0 {
    pub(super) fn encode_ilst(
        raw_entries: Vec<(String, TagValue)>,
        old_ilst_atoms: Vec<Atom>,
        replaced_keys: Vec<String>,
    ) -> Vec<u8> {
        let mut ilst_entries: Vec<u8> = Vec::new();
        let mut encoded_keys = replaced_keys;

        for (key, value) in &raw_entries {
            if key == "covr" {
                if let TagValue::Picture { mime, data, .. } = value {
                    let mut mime_buff: Vec<u8> = vec![0x00, 0x00, 0x00, 0x0d];
                    if mime == "image/png" {
                        mime_buff = vec![0x00, 0x00, 0x00, 0x0e];
                    }
                    let reserved = vec![0u8; 4];
                    let mut data_buff = Vec::new();
                    data_buff.extend_from_slice(&mime_buff);
                    data_buff.extend_from_slice(&reserved);
                    data_buff.extend_from_slice(data);
                    let data_atom_size = 8 + data_buff.len();
                    let data_size_buffer = (data_atom_size as u32).to_be_bytes().to_vec();
                    let mut data_atom = Vec::new();
                    data_atom.extend_from_slice(&data_size_buffer);
                    data_atom.extend_from_slice(b"data");
                    data_atom.extend_from_slice(&data_buff);
                    let key_atom_size = 8 + data_atom.len();
                    let key_size_buffer = (key_atom_size as u32).to_be_bytes().to_vec();
                    let mut key_atom = Vec::new();
                    let key_bytes = V0::parse_key("covr");
                    key_atom.extend_from_slice(&key_size_buffer);
                    key_atom.extend_from_slice(&key_bytes);
                    key_atom.extend_from_slice(&data_atom);
                    ilst_entries.extend_from_slice(&key_atom);
                    if !encoded_keys.iter().any(|k| k == "covr") {
                        encoded_keys.push("covr".to_string());
                    }
                }
            } else if key.starts_with("----:") {
                if let Some((_, rest)) = key.split_once(':')
                    && let Some((mean, name)) = rest.split_once(':')
                {
                    let mut mean_buf = Vec::new();
                    let mean_size = (8 + 4 + mean.len()) as u32;
                    mean_buf.extend_from_slice(&mean_size.to_be_bytes());
                    mean_buf.extend_from_slice(b"mean");
                    mean_buf.extend_from_slice(&[0u8; 4]);
                    mean_buf.extend_from_slice(mean.as_bytes());

                    let mut name_buf = Vec::new();
                    let name_size = (8 + 4 + name.len()) as u32;
                    name_buf.extend_from_slice(&name_size.to_be_bytes());
                    name_buf.extend_from_slice(b"name");
                    name_buf.extend_from_slice(&[0u8; 4]);
                    name_buf.extend_from_slice(name.as_bytes());

                    let emit_one = |text: &str, ilst_entries: &mut Vec<u8>| {
                        let mut data_buf = Vec::new();
                        let mut inner = Vec::new();
                        inner.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                        inner.extend_from_slice(&[0u8; 4]);
                        inner.extend_from_slice(text.as_bytes());
                        let total = 8 + inner.len();
                        data_buf.extend_from_slice(&(total as u32).to_be_bytes());
                        data_buf.extend_from_slice(b"data");
                        data_buf.extend_from_slice(&inner);

                        let mut key_atom = Vec::new();
                        let key_bytes = V0::parse_key("----");
                        let key_size = 8 + mean_buf.len() + name_buf.len() + data_buf.len();
                        key_atom.extend_from_slice(&(key_size as u32).to_be_bytes());
                        key_atom.extend_from_slice(&key_bytes);
                        key_atom.extend_from_slice(&mean_buf);
                        key_atom.extend_from_slice(&name_buf);
                        key_atom.extend_from_slice(&data_buf);
                        ilst_entries.extend_from_slice(&key_atom);
                    };

                    if let TagValue::Text(text) = value {
                        emit_one(text, &mut ilst_entries);
                    }

                    if !encoded_keys.iter().any(|k| k == key) {
                        encoded_keys.push(key.clone());
                    }
                }
            } else {
                let mut emit_key = |data_buffer: Vec<u8>| {
                    let data_atom_size = 8 + data_buffer.len();
                    let data_size_buffer = (data_atom_size as u32).to_be_bytes().to_vec();
                    let mut data_atom = Vec::new();
                    data_atom.extend_from_slice(&data_size_buffer);
                    data_atom.extend_from_slice(b"data");
                    data_atom.extend_from_slice(&data_buffer);
                    let key_atom_size = 8 + data_atom.len();
                    let key_size_buffer = (key_atom_size as u32).to_be_bytes().to_vec();
                    let mut key_atom = Vec::new();
                    let key_bytes = V0::parse_key(key);
                    key_atom.extend_from_slice(&key_size_buffer);
                    key_atom.extend_from_slice(&key_bytes);
                    key_atom.extend_from_slice(&data_atom);
                    ilst_entries.extend_from_slice(&key_atom);
                };
                if let Some(data_buffer) = V0::data_to_buffer(key, value) {
                    emit_key(data_buffer);
                    if !encoded_keys.iter().any(|k| k == key) {
                        encoded_keys.push(key.to_string());
                    }
                }
            }
        }

        for atom in old_ilst_atoms {
            if atom.atom_type == "----" {
                let mut cursor: u64 = 8;
                let end: u64 = atom.size;
                let mut mean: Option<String> = None;
                let mut name: Option<String> = None;
                while cursor + 8 <= end {
                    let sz = u32::from_be_bytes([
                        atom.buffer[cursor as usize],
                        atom.buffer[cursor as usize + 1],
                        atom.buffer[cursor as usize + 2],
                        atom.buffer[cursor as usize + 3],
                    ]) as u64;
                    if sz < 12 || cursor + sz > end {
                        break;
                    }
                    let t = &atom.buffer[(cursor + 4) as usize..(cursor + 8) as usize];
                    let t_str = String::from_utf8_lossy(t);
                    if &*t_str == "mean" {
                        mean = Some(
                            String::from_utf8_lossy(
                                &atom.buffer[(cursor + 12) as usize..(cursor + sz) as usize],
                            )
                            .to_string(),
                        );
                    } else if &*t_str == "name" {
                        name = Some(
                            String::from_utf8_lossy(
                                &atom.buffer[(cursor + 12) as usize..(cursor + sz) as usize],
                            )
                            .to_string(),
                        );
                    }
                    cursor += sz;
                }
                if let (Some(mean), Some(name)) = (mean, name) {
                    let comp = format!("----:{}:{}", mean, name);
                    if encoded_keys.iter().any(|k| k == &comp) {
                        continue;
                    }
                }
                ilst_entries.extend_from_slice(&atom.buffer);
            } else if !encoded_keys.iter().any(|k| k == &atom.atom_type) {
                ilst_entries.extend_from_slice(&atom.buffer);
            }
        }

        let ilst_size = 8 + ilst_entries.len();
        let ilst_size_buffer = (ilst_size as u32).to_be_bytes().to_vec();
        let mut ilst_atom = Vec::new();
        ilst_atom.extend_from_slice(&ilst_size_buffer);
        ilst_atom.extend_from_slice(b"ilst");
        ilst_atom.extend_from_slice(&ilst_entries);
        ilst_atom
    }

    pub(super) fn parse_key(key: &str) -> Vec<u8> {
        if key.starts_with('©') {
            let mut buff = vec![0u8; 4];

            buff[0] = 0xa9;

            let rest = key.chars().skip(1).collect::<String>();
            let bytes = rest.as_bytes();
            for (i, &b) in bytes.iter().take(3).enumerate() {
                buff[i + 1] = b;
            }

            buff
        } else {
            key.as_bytes().to_vec()
        }
    }

    pub(super) fn data_to_buffer(key: &str, value: &TagValue) -> Option<Vec<u8>> {
        if matches!(key, "trkn" | "disk") {
            let TagValue::Text(text) = value else {
                return None;
            };
            let (number, total) = parse_number_pair(text)?;
            let mut data = vec![0; 10];
            data.extend_from_slice(&number.to_be_bytes());
            data.extend_from_slice(&total.unwrap_or(0).to_be_bytes());
            if key == "trkn" {
                data.extend_from_slice(&[0, 0]);
            }
            return Some(data);
        }
        let item = get_atom_flag(key);
        if let Some(item) = item {
            if item.flag[3] == 0x01 {
                match value {
                    TagValue::Text(text) => {
                        let prefix = "\x00\x00\x00\x00".as_bytes();
                        let vale_buffer = text.as_bytes();
                        let trimmed_value = vale_buffer;
                        let mut data_buffer = Vec::new();
                        data_buffer.extend_from_slice(&item.flag);
                        data_buffer.extend_from_slice(prefix);
                        data_buffer.extend_from_slice(trimmed_value);

                        Some(data_buffer)
                    }
                    _ => None,
                }
            } else if item.flag[3] == 0x15 || item.flag[3] == 0x00 {
                match value {
                    TagValue::Text(text) => {
                        let mut data_buffer = Vec::new();
                        data_buffer.extend_from_slice(&item.flag);

                        let int_value: u64 = text.parse().ok()?;
                        let width = if key == "tmpo" {
                            2
                        } else {
                            item.size.map_or(1, |bits| (bits / 8) as usize)
                        };
                        if width == 0
                            || width > 8
                            || (width < 8 && int_value >= (1u64 << (width * 8)))
                            || (item.boolean && int_value > 1)
                        {
                            return None;
                        }
                        data_buffer.extend_from_slice(&[0; 4]);
                        data_buffer.extend_from_slice(&int_value.to_be_bytes()[8 - width..]);

                        Some(data_buffer)
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}
