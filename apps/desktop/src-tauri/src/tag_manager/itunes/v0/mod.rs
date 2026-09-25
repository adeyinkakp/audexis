use crate::tag_manager;
use crate::tag_manager::itunes::utils::{
    get_atom_flag, itunes_key, raw_to_tags, FREEFORM_REVERSE_MAP,
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
    fn encode_ilst(raw_entries: Vec<(String, TagValue)>, old_ilst_atoms: Vec<Atom>) -> Vec<u8> {
        let mut ilst_entries: Vec<u8> = Vec::new();
        let mut encoded_keys: Vec<String> = Vec::new();

        for (key, value) in &raw_entries {
            if key == "covr" {
                match value {
                    TagValue::Picture { mime, data, .. } => {
                        let mut mime_buff: Vec<u8> = vec![0x00, 0x00, 0x00, 0x0d];
                        if mime == "image/png" {
                            mime_buff = vec![0x00, 0x00, 0x00, 0x0e];
                        }
                        let reserved = vec![0u8; 4];
                        let mut data_buff = Vec::new();
                        data_buff.extend_from_slice(&mime_buff);
                        data_buff.extend_from_slice(&reserved);
                        data_buff.extend_from_slice(&data);
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

                    _ => {}
                }
            } else if key.starts_with("----:") {
                if let Some((_, rest)) = key.split_once(':') {
                    if let Some((mean, name)) = rest.split_once(':') {
                        let mut mean_buf = Vec::new();
                        let mean_size = (8 + 4 + mean.as_bytes().len()) as u32;
                        mean_buf.extend_from_slice(&mean_size.to_be_bytes());
                        mean_buf.extend_from_slice(b"mean");
                        mean_buf.extend_from_slice(&[0u8; 4]);
                        mean_buf.extend_from_slice(mean.as_bytes());

                        let mut name_buf = Vec::new();
                        let name_size = (8 + 4 + name.as_bytes().len()) as u32;
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
                    let key_bytes = V0::parse_key(&key);
                    key_atom.extend_from_slice(&key_size_buffer);
                    key_atom.extend_from_slice(&key_bytes);
                    key_atom.extend_from_slice(&data_atom);
                    ilst_entries.extend_from_slice(&key_atom);
                };
                if let Some(data_buffer) = V0::data_to_buffer(&key, &value) {
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
                    if sz < 8 || cursor + sz > end {
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
    fn parse_key(key: &str) -> Vec<u8> {
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
    fn data_to_buffer(key: &str, value: &TagValue) -> Option<Vec<u8>> {
        let item = get_atom_flag(key);
        if item.is_none() {
            return None;
        } else {
            let item = item.unwrap();
            if item.flag[3] == 0x01 {
                match value {
                    TagValue::Text(text) => {
                        let prefix = "\x00\x00\x00\x00".as_bytes();
                        let vale_buffer = text.as_bytes();
                        let allowed_bytes = 255 - prefix.len();
                        let trimmed_value = if vale_buffer.len() > allowed_bytes {
                            &vale_buffer[0..allowed_bytes]
                        } else {
                            vale_buffer
                        };
                        let mut data_buffer = Vec::new();
                        data_buffer.extend_from_slice(&item.flag);
                        data_buffer.extend_from_slice(prefix);
                        data_buffer.extend_from_slice(trimmed_value);

                        Some(data_buffer)
                    }
                    _ => {
                        return None;
                    }
                }
            } else if item.flag[3] == 0x15 || item.flag[3] == 0x00 {
                match value {
                    TagValue::Text(text) => {
                        let mut data_buffer = Vec::new();
                        data_buffer.extend_from_slice(&item.flag);

                        let int_value: u32 = text.parse().unwrap_or(0);

                        if item.size.is_some() {
                            let mut value_buffer = vec![0u8; 8];
                            let int_bytes = int_value.to_be_bytes();
                            value_buffer[4..].copy_from_slice(&int_bytes);
                            data_buffer.extend_from_slice(&value_buffer);
                        } else {
                            let mut value_buffer = vec![0u8; 5];
                            value_buffer[4] = if int_value == 1 { 1 } else { 0 };
                            data_buffer.extend_from_slice(&value_buffer);
                        }

                        Some(data_buffer)
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
    }

    fn rebuild_file(ilst_buffer: Vec<u8>, file_buffer: &Vec<u8>) -> Option<Vec<u8>> {
        let top_level_atoms = V0::parse_atoms(file_buffer, 0, file_buffer.len() as u64);

        let moov_atom = top_level_atoms
            .iter()
            .find(|atom| atom.atom_type == "moov")?;

        let moov_sub_atoms = V0::parse_atoms(&moov_atom.buffer, 8, moov_atom.size);

        let udta_atom = moov_sub_atoms
            .iter()
            .find(|atom| atom.atom_type == "udta")?;

        let udta_sub_atoms = V0::parse_atoms(&udta_atom.buffer, 8, udta_atom.size);
        let meta_atom = udta_sub_atoms
            .iter()
            .find(|atom| atom.atom_type == "meta")?;

        let meta_sub_atoms = V0::parse_atoms(&meta_atom.buffer, 12, meta_atom.size);
        let ilst_atom = meta_sub_atoms
            .iter()
            .find(|atom| atom.atom_type == "ilst")?;

        let new_meta_buffer = {
            let mut buffer = Vec::new();
            let new_meta_size = (meta_atom.size - ilst_atom.size + ilst_buffer.len() as u64) as u32;
            buffer.extend_from_slice(&new_meta_size.to_be_bytes());

            let ilst_offset_in_meta = (ilst_atom.position) as usize;
            buffer.extend_from_slice(&meta_atom.buffer[4..ilst_offset_in_meta]);
            buffer.extend_from_slice(&ilst_buffer);
            buffer.extend_from_slice(
                &meta_atom.buffer[(ilst_offset_in_meta + ilst_atom.size as usize)..],
            );
            buffer
        };

        let new_udta_buffer = {
            let mut buffer = Vec::new();
            let new_udta_size =
                (udta_atom.size - meta_atom.size + new_meta_buffer.len() as u64) as u32;
            buffer.extend_from_slice(&new_udta_size.to_be_bytes());

            let meta_offset_in_udta = (meta_atom.position) as usize;
            buffer.extend_from_slice(&udta_atom.buffer[4..meta_offset_in_udta]);
            buffer.extend_from_slice(&new_meta_buffer);
            buffer.extend_from_slice(
                &udta_atom.buffer[(meta_offset_in_udta + meta_atom.size as usize)..],
            );
            buffer
        };

        let new_moov_buffer = {
            let mut buffer = Vec::new();
            let new_moov_size =
                (moov_atom.size - udta_atom.size + new_udta_buffer.len() as u64) as u32;
            buffer.extend_from_slice(&new_moov_size.to_be_bytes());

            let udta_offset_in_moov = (udta_atom.position) as usize;
            buffer.extend_from_slice(&moov_atom.buffer[4..udta_offset_in_moov]);
            buffer.extend_from_slice(&new_udta_buffer);
            buffer.extend_from_slice(
                &moov_atom.buffer[(udta_offset_in_moov + udta_atom.size as usize)..],
            );
            buffer
        };
        let almost_done_file = {
            let mut buffer = Vec::new();
            buffer.extend_from_slice(&file_buffer[0..moov_atom.position as usize]);
            buffer.extend_from_slice(&new_moov_buffer);
            buffer
                .extend_from_slice(&file_buffer[(moov_atom.position + moov_atom.size) as usize..]);
            buffer
        };

        let new_top_level_atoms =
            V0::parse_atoms(&almost_done_file, 0, almost_done_file.len() as u64);
        let new_moov_atom = new_top_level_atoms
            .iter()
            .find(|atom| atom.atom_type == "moov")?;
        let mdat_atom = new_top_level_atoms.iter().find(|a| a.atom_type == "mdat");
        let should_update_offsets = mdat_atom
            .map(|mdat| new_moov_atom.position < mdat.position)
            .unwrap_or(false);
        if !should_update_offsets {
            return Some(almost_done_file);
        }

        let shift = ilst_buffer.len() as i64 - ilst_atom.size as i64;
        if let Some(co64_atoms) = V0::find_co64_atom(&almost_done_file, &new_moov_atom) {
            Some(V0::update_co64_offsets(
                &almost_done_file,
                shift,
                &co64_atoms,
            ))
        } else if let Some(stco_atoms) = V0::find_stco_atom(&almost_done_file, &new_moov_atom) {
            Some(V0::update_stco_offsets(
                &almost_done_file,
                shift,
                &stco_atoms,
            ))
        } else {
            Some(almost_done_file)
        }
    }

    fn rebuild_file_insert_ilst(
        ilst_atom_buffer: Vec<u8>,
        file_buffer: &Vec<u8>,
    ) -> Option<Vec<u8>> {
        let top_level_atoms = V0::parse_atoms(file_buffer, 0, file_buffer.len() as u64);

        let moov_atom = top_level_atoms.iter().find(|a| a.atom_type == "moov")?;

        let moov_sub_atoms = V0::parse_atoms(&moov_atom.buffer, 8, moov_atom.size);
        let udta_opt = moov_sub_atoms.iter().find(|a| a.atom_type == "udta");

        let (new_moov_buffer, moov_size_delta): (Vec<u8>, i64) = if let Some(udta_atom) = udta_opt {
            let udta_sub_atoms = V0::parse_atoms(&udta_atom.buffer, 8, udta_atom.size);
            if let Some(meta_atom) = udta_sub_atoms.iter().find(|a| a.atom_type == "meta") {
                let new_meta_buffer = {
                    let mut buf = Vec::new();
                    let new_size = (meta_atom.size + ilst_atom_buffer.len() as u64) as u32;
                    buf.extend_from_slice(&new_size.to_be_bytes());
                    buf.extend_from_slice(&meta_atom.buffer[4..]);
                    buf.extend_from_slice(&ilst_atom_buffer);
                    buf
                };

                let new_udta_buffer = {
                    let mut buf = Vec::new();
                    let new_size =
                        (udta_atom.size - meta_atom.size + new_meta_buffer.len() as u64) as u32;
                    buf.extend_from_slice(&new_size.to_be_bytes());
                    let meta_off = meta_atom.position as usize;
                    buf.extend_from_slice(&udta_atom.buffer[4..meta_off]);
                    buf.extend_from_slice(&new_meta_buffer);
                    buf.extend_from_slice(
                        &udta_atom.buffer[(meta_off + meta_atom.size as usize)..],
                    );
                    buf
                };

                let new_moov_buffer = {
                    let mut buf = Vec::new();
                    let new_size =
                        (moov_atom.size - udta_atom.size + new_udta_buffer.len() as u64) as u32;
                    buf.extend_from_slice(&new_size.to_be_bytes());
                    let udta_off = udta_atom.position as usize;
                    buf.extend_from_slice(&moov_atom.buffer[4..udta_off]);
                    buf.extend_from_slice(&new_udta_buffer);
                    buf.extend_from_slice(
                        &moov_atom.buffer[(udta_off + udta_atom.size as usize)..],
                    );
                    buf
                };
                let moov_delta = (new_moov_buffer.len() as i64) - (moov_atom.size as i64);
                (new_moov_buffer, moov_delta)
            } else {
                let meta_buffer = {
                    let mut buf = Vec::new();
                    let size = (12 + ilst_atom_buffer.len()) as u32;
                    buf.extend_from_slice(&size.to_be_bytes());
                    buf.extend_from_slice(b"meta");
                    buf.extend_from_slice(&[0u8; 4]);
                    buf.extend_from_slice(&ilst_atom_buffer);
                    buf
                };
                let new_udta_buffer = {
                    let mut buf = Vec::new();
                    let new_size = (udta_atom.size + meta_buffer.len() as u64) as u32;
                    buf.extend_from_slice(&new_size.to_be_bytes());
                    buf.extend_from_slice(&udta_atom.buffer[4..]);
                    buf.extend_from_slice(&meta_buffer);
                    buf
                };
                let new_moov_buffer = {
                    let mut buf = Vec::new();
                    let new_size =
                        (moov_atom.size - udta_atom.size + new_udta_buffer.len() as u64) as u32;
                    buf.extend_from_slice(&new_size.to_be_bytes());
                    let udta_off = udta_atom.position as usize;
                    buf.extend_from_slice(&moov_atom.buffer[4..udta_off]);
                    buf.extend_from_slice(&new_udta_buffer);
                    buf.extend_from_slice(
                        &moov_atom.buffer[(udta_off + udta_atom.size as usize)..],
                    );
                    buf
                };
                let moov_delta = (new_moov_buffer.len() as i64) - (moov_atom.size as i64);
                (new_moov_buffer, moov_delta)
            }
        } else {
            let meta_buffer = {
                let mut buf = Vec::new();
                let size = (12 + ilst_atom_buffer.len()) as u32;
                buf.extend_from_slice(&size.to_be_bytes());
                buf.extend_from_slice(b"meta");
                buf.extend_from_slice(&[0u8; 4]);
                buf.extend_from_slice(&ilst_atom_buffer);
                buf
            };
            let udta_buffer = {
                let mut buf = Vec::new();
                let size = (8 + meta_buffer.len()) as u32;
                buf.extend_from_slice(&size.to_be_bytes());
                buf.extend_from_slice(b"udta");
                buf.extend_from_slice(&meta_buffer);
                buf
            };
            let new_moov_buffer = {
                let mut buf = Vec::new();
                let new_size = (moov_atom.size + udta_buffer.len() as u64) as u32;
                buf.extend_from_slice(&new_size.to_be_bytes());
                buf.extend_from_slice(&moov_atom.buffer[4..]);
                buf.extend_from_slice(&udta_buffer);
                buf
            };
            (new_moov_buffer, (udta_buffer.len() as i64))
        };

        let almost_done_file = {
            let mut buffer = Vec::new();
            buffer.extend_from_slice(&file_buffer[0..moov_atom.position as usize]);
            buffer.extend_from_slice(&new_moov_buffer);
            buffer
                .extend_from_slice(&file_buffer[(moov_atom.position + moov_atom.size) as usize..]);
            buffer
        };

        let new_top_level_atoms =
            V0::parse_atoms(&almost_done_file, 0, almost_done_file.len() as u64);
        let new_moov_atom = new_top_level_atoms.iter().find(|a| a.atom_type == "moov")?;
        let mdat_atom = new_top_level_atoms.iter().find(|a| a.atom_type == "mdat");
        let should_update_offsets = mdat_atom
            .map(|mdat| new_moov_atom.position < mdat.position)
            .unwrap_or(false);
        if !should_update_offsets {
            return Some(almost_done_file);
        }

        let shift = moov_size_delta;
        if let Some(co64_atoms) = V0::find_co64_atom(&almost_done_file, &new_moov_atom) {
            Some(V0::update_co64_offsets(
                &almost_done_file,
                shift,
                &co64_atoms,
            ))
        } else if let Some(stco_atoms) = V0::find_stco_atom(&almost_done_file, &new_moov_atom) {
            Some(V0::update_stco_offsets(
                &almost_done_file,
                shift,
                &stco_atoms,
            ))
        } else {
            Some(almost_done_file)
        }
    }

    fn find_co64_atom(buffer: &Vec<u8>, moov_atom: &Atom) -> Option<Vec<Atom>> {
        let moov_end = moov_atom.position + moov_atom.size;
        let trak_atoms: Vec<Atom> = V0::parse_atoms(buffer, moov_atom.position + 8, moov_end)
            .into_iter()
            .filter(|a| a.atom_type == "trak")
            .collect();

        let mut co64_atoms: Vec<Atom> = Vec::new();

        for trak in trak_atoms {
            let trak_end = trak.position + trak.size;
            let mdia = V0::parse_atoms(buffer, trak.position + 8, trak_end)
                .into_iter()
                .find(|a| a.atom_type == "mdia");
            let Some(mdia) = mdia else { continue };
            let mdia_end = mdia.position + mdia.size;

            let minf = V0::parse_atoms(buffer, mdia.position + 8, mdia_end)
                .into_iter()
                .find(|a| a.atom_type == "minf");
            let Some(minf) = minf else { continue };
            let minf_end = minf.position + minf.size;

            let stbl = V0::parse_atoms(buffer, minf.position + 8, minf_end)
                .into_iter()
                .find(|a| a.atom_type == "stbl");
            let Some(stbl) = stbl else { continue };
            let stbl_end = stbl.position + stbl.size;

            let co64 = V0::parse_atoms(buffer, stbl.position + 8, stbl_end)
                .into_iter()
                .find(|a| a.atom_type == "co64");

            if let Some(co64) = co64 {
                co64_atoms.push(co64);
            }
        }

        if co64_atoms.is_empty() {
            None
        } else {
            Some(co64_atoms)
        }
    }
    fn find_stco_atom(buffer: &Vec<u8>, moov_atom: &Atom) -> Option<Vec<Atom>> {
        let moov_end = moov_atom.position + moov_atom.size;
        let trak_atoms: Vec<Atom> = V0::parse_atoms(buffer, moov_atom.position + 8, moov_end)
            .into_iter()
            .filter(|a| a.atom_type == "trak")
            .collect();

        let mut stco_atoms: Vec<Atom> = Vec::new();

        for trak in trak_atoms {
            let trak_end = trak.position + trak.size;
            let mdia = V0::parse_atoms(buffer, trak.position + 8, trak_end)
                .into_iter()
                .find(|a| a.atom_type == "mdia");
            let Some(mdia) = mdia else { continue };
            let mdia_end = mdia.position + mdia.size;

            let minf = V0::parse_atoms(buffer, mdia.position + 8, mdia_end)
                .into_iter()
                .find(|a| a.atom_type == "minf");
            let Some(minf) = minf else { continue };
            let minf_end = minf.position + minf.size;

            let stbl = V0::parse_atoms(buffer, minf.position + 8, minf_end)
                .into_iter()
                .find(|a| a.atom_type == "stbl");
            let Some(stbl) = stbl else { continue };
            let stbl_end = stbl.position + stbl.size;

            let stco = V0::parse_atoms(buffer, stbl.position + 8, stbl_end)
                .into_iter()
                .find(|a| a.atom_type == "stco");

            if let Some(stco) = stco {
                stco_atoms.push(stco);
            }
        }

        if stco_atoms.is_empty() {
            None
        } else {
            Some(stco_atoms)
        }
    }
    fn update_co64_offsets(
        file_buffer: &Vec<u8>,
        shift: i64,
        co64_sub_atoms: &Vec<Atom>,
    ) -> Vec<u8> {
        let mut buff = file_buffer.clone();

        for atom in co64_sub_atoms {
            if atom.size == 0 {
                continue;
            }

            let start = atom.position as usize;
            let end = start + atom.size as usize;
            if end > buff.len() {
                continue;
            }

            let mut co64_buffer = buff[start..end].to_vec();

            if co64_buffer.len() < 16 {
                continue;
            }
            let entry_count_bytes: [u8; 4] = co64_buffer[12..16].try_into().unwrap();
            let entry_count = u32::from_be_bytes(entry_count_bytes);

            for i in 0..entry_count {
                let pos = 16 + (i as usize) * 8;
                if pos + 8 > co64_buffer.len() {
                    break;
                }

                let old_offset_bytes: [u8; 8] = co64_buffer[pos..pos + 8].try_into().unwrap();
                let old_offset = u64::from_be_bytes(old_offset_bytes);

                let new_offset = (old_offset as i128 + shift as i128) as u64;
                let new_bytes = new_offset.to_be_bytes();
                co64_buffer[pos..pos + 8].copy_from_slice(&new_bytes);
            }

            buff.splice(start..end, co64_buffer);
        }

        buff
    }
    fn update_stco_offsets(
        file_buffer: &Vec<u8>,
        shift: i64,
        stco_sub_atoms: &Vec<Atom>,
    ) -> Vec<u8> {
        let mut buff = file_buffer.clone();

        for atom in stco_sub_atoms {
            if atom.size == 0 {
                continue;
            }

            let start = atom.position as usize;
            let end = start + atom.size as usize;
            if end > buff.len() {
                continue;
            }

            let mut stco_buffer = buff[start..end].to_vec();

            if stco_buffer.len() < 16 {
                continue;
            }
            let entry_count_bytes: [u8; 4] = stco_buffer[12..16].try_into().unwrap();
            let entry_count = u32::from_be_bytes(entry_count_bytes);

            for i in 0..entry_count {
                let pos = 16 + (i as usize) * 4;
                if pos + 4 > stco_buffer.len() {
                    break;
                }

                let old_offset_bytes: [u8; 4] = stco_buffer[pos..pos + 4].try_into().unwrap();
                let old_offset = u32::from_be_bytes(old_offset_bytes);

                let new_offset = old_offset as i64 + shift;
                let new_bytes = (new_offset as u32).to_be_bytes();
                stco_buffer[pos..pos + 4].copy_from_slice(&new_bytes);
            }

            buff.splice(start..end, stco_buffer);
        }

        buff
    }
}

impl TagFormat for V0 {
    fn new() -> Self {
        Self {}
    }

    fn get_tags(
        &self,
        file_path: &std::path::PathBuf,
    ) -> Result<HashMap<tag_manager::utils::FrameKey, Vec<TagValue>>, BackendError> {
        let buffer = fs::read(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                internal_message: "Failed to open file".to_string(),
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unable to read file.".to_string(),
            })
        })?;
        let ilst_atom = V0::ensure_ilst_atom(&buffer);
        if ilst_atom.is_err() {
            return Err(BackendError::ReadFailed(TagError {
                internal_message: "Failed to find 'ilst' atom".to_string(),
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "The file is missing required metadata.".to_string(),
            }));
        }
        let ilst_atom = ilst_atom.unwrap();
        let ilst_sub_atoms = V0::parse_atoms(&ilst_atom.buffer, 8, ilst_atom.size);
        let mut raw_entries: Vec<(String, TagValue)> = Vec::new();
        for atom in &ilst_sub_atoms {
            let data_start = atom.position + 16;
            let data_size = atom.size - 16;

            if atom.atom_type == "covr" {
                let mut mime_type = "image/jpeg";
                let magic_number = &ilst_atom.buffer[(data_start + 8) as usize
                    ..(data_start + 24).min(data_start + data_size) as usize];
                if magic_number.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                    mime_type = "image/png";
                } else if magic_number.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    mime_type = "image/jpeg";
                }
                let pic = tag_manager::utils::PictureData {
                    mime: mime_type.to_string(),
                    data: ilst_atom.buffer
                        [(data_start + 8) as usize..(data_start + data_size) as usize]
                        .to_vec(),
                    picture_type: None,
                    description: None,
                };
                raw_entries.push((
                    atom.atom_type.clone(),
                    TagValue::Picture {
                        mime: pic.mime.clone(),
                        data: pic.data.clone(),
                        picture_type: pic.picture_type,
                        description: pic.description.clone(),
                    },
                ));
            } else if atom.atom_type == "----" {
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
                    if sz < 8 || cursor + sz > end {
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
                        "data" => {
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
                if let (Some(mean), Some(name), Some(val)) = (mean, name, value_text) {
                    let key = format!("----:{}:{}", mean, name);
                    raw_entries.push((key, TagValue::Text(val)));
                }
            } else {
                let item = get_atom_flag(&atom.atom_type);
                if let Some(item) = item {
                    if item.flag[3] == 0x15 {
                        let value = if item.size.is_some() {
                            u32::from_be_bytes([
                                ilst_atom.buffer[(data_start + 8) as usize],
                                ilst_atom.buffer[(data_start + 8) as usize + 1],
                                ilst_atom.buffer[(data_start + 8) as usize + 2],
                                ilst_atom.buffer[(data_start + 8) as usize + 3],
                            ])
                        } else {
                            ilst_atom.buffer[(data_start + 8) as usize] as u32
                        };
                        raw_entries
                            .push((atom.atom_type.clone(), TagValue::Text(value.to_string())));
                        continue;
                    }
                }

                if data_size > ilst_atom.size {
                    continue;
                }
                let text = String::from_utf8_lossy(
                    &ilst_atom.buffer[data_start as usize..(data_start + data_size) as usize],
                )
                .to_string()
                .chars()
                .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
                .collect::<String>();
                let key = itunes_key(&atom.atom_type);

                if let Some(_) = key {
                    for part in text.split(';').map(|s| s.trim()) {
                        let seg = part.trim();
                        if !seg.is_empty() {
                            raw_entries
                                .push((atom.atom_type.clone(), TagValue::Text(seg.to_string())));
                        }
                    }
                } else {
                    raw_entries.push((atom.atom_type.clone(), TagValue::Text(text)));
                }
            }
        }
        let vec_map = raw_to_tags(&raw_entries);
        return Ok(vec_map);
    }
    fn write_tags(
        &self,
        file_path: &std::path::PathBuf,
        updated_tags: HashMap<tag_manager::utils::FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        let mut updated_entries: Vec<(String, TagValue)> = Vec::new();
        let mut updated_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

        let mut push_key_once = |key: &String| {
            updated_keys.insert(key.clone());
        };

        for (k, vals) in updated_tags.iter() {
            match k {
                tag_manager::utils::FrameKey::UserDefinedText => {
                    for v in vals {
                        if let TagValue::UserText(ut) = v {
                            let key = format!(
                                "----:{}:{}",
                                "com.apple.iTunes",
                                ut.description.replace(" ", "_")
                            );
                            updated_entries.push((key.clone(), TagValue::Text(ut.value.clone())));
                            push_key_once(&key);
                        } else if let TagValue::Text(s) = v {
                            let key = format!("----:{}:{}", "com.apple.iTunes", "TXXX");
                            updated_entries.push((key.clone(), TagValue::Text(s.clone())));
                            push_key_once(&key);
                        }
                    }
                }
                tag_manager::utils::FrameKey::UserDefinedURL => {
                    for v in vals {
                        if let TagValue::UserUrl(u) = v {
                            let key = format!(
                                "----:{}:{}",
                                "com.apple.iTunes",
                                u.description.replace(" ", "_")
                            );
                            updated_entries.push((key.clone(), TagValue::Text(u.url.clone())));
                            push_key_once(&key);
                        }
                    }
                }
                tag_manager::utils::FrameKey::AttachedPicture => {
                    let mut any = false;
                    for v in vals {
                        if let TagValue::Picture { .. } = v {
                            updated_entries.push(("covr".to_string(), v.clone()));
                            any = true;
                        }
                    }
                    if any {
                        push_key_once(&"covr".to_string());
                    }
                }
                tag_manager::utils::FrameKey::Artists => {
                    let joined = vals
                        .iter()
                        .filter_map(|v| match v {
                            TagValue::Text(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\\");
                    if !joined.is_empty() {
                        let key = "©ART".to_string();
                        updated_entries.push((key.clone(), TagValue::Text(joined)));
                        push_key_once(&key);
                    }
                }
                other => {
                    let code = tag_manager::itunes::utils::itunes_code(*other);
                    if code == "----" {
                        if let Some(spec) = tag_manager::itunes::utils::itunes_freeform_spec(*other)
                        {
                            if let Some(first) =
                                vals.iter().find(|v| matches!(v, TagValue::Text(_)))
                            {
                                let key = format!("----:{}:{}", spec.mean, spec.name);
                                updated_entries.push((key.clone(), first.clone()));
                                push_key_once(&key);
                            }
                        }
                    } else {
                        let text_joined = vals
                            .iter()
                            .filter_map(|v| match v {
                                TagValue::Text(s) => Some(s.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\\");
                        let key = code.to_string();
                        if !text_joined.is_empty() {
                            updated_entries.push((key.clone(), TagValue::Text(text_joined)));
                            push_key_once(&key);
                        } else if let Some(first) = vals.first() {
                            updated_entries.push((key.clone(), first.clone()));
                            push_key_once(&key);
                        }
                    }
                }
            }
        }

        let old_vec = self.get_tags(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read tags from file".to_string(),
                internal_message: "Failed to read tags from file".to_string(),
            })
        })?;
        let mut old_entries: Vec<(String, TagValue)> = Vec::new();
        for (k, vals) in old_vec.iter() {
            match k {
                FrameKey::UserDefinedText => {
                    for v in vals {
                        if let TagValue::UserText(ut) = v {
                            let key = format!(
                                "----:{}:{}",
                                "com.apple.iTunes",
                                ut.description.replace(" ", "_")
                            );
                            if !updated_keys.contains(&key) {
                                old_entries.push((key, TagValue::Text(ut.value.clone())));
                            }
                        }
                    }
                }
                FrameKey::UserDefinedURL => {
                    for v in vals {
                        if let TagValue::UserUrl(u) = v {
                            let key = format!(
                                "----:{}:{}",
                                "com.apple.iTunes",
                                u.description.replace(" ", "_")
                            );
                            if !updated_keys.contains(&key) {
                                old_entries.push((key, TagValue::Text(u.url.clone())));
                            }
                        }
                    }
                }
                FrameKey::AttachedPicture => {
                    let key = "covr".to_string();
                    if !updated_keys.contains(&key) {
                        for v in vals {
                            if let TagValue::Picture { .. } = v {
                                old_entries.push((key.clone(), v.clone()));
                            }
                        }
                    }
                }
                other => {
                    let code = tag_manager::itunes::utils::itunes_code(*other);
                    if code == "----" {
                        if let Some(spec) = tag_manager::itunes::utils::itunes_freeform_spec(*other)
                        {
                            let key = format!("----:{}:{}", spec.mean, spec.name);
                            if !updated_keys.contains(&key) {
                                if let Some(first) = vals.first() {
                                    old_entries.push((key, first.clone()));
                                }
                            }
                        }
                    } else {
                        let key = code.to_string();
                        if !updated_keys.contains(&key) {
                            if let Some(first) = vals.first() {
                                old_entries.push((key, first.clone()));
                            }
                        }
                    }
                }
            }
        }

        let mut all_entries = updated_entries;
        all_entries.extend(old_entries);

        let buffer = fs::read(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read file".to_string(),
                internal_message: "Failed to read file".to_string(),
            })
        })?;
        let ilst_atom = V0::ensure_ilst_atom(&buffer);

        let rebuilt_file = if let Ok(ilst_atom) = ilst_atom {
            let ilst_sub_atoms = V0::parse_atoms(&ilst_atom.buffer, 8, ilst_atom.size);
            let updated_ilst_buffer = V0::encode_ilst(all_entries, ilst_sub_atoms);
            V0::rebuild_file(updated_ilst_buffer, &buffer)
                .ok_or(())
                .map_err(|_| {
                    BackendError::WriteFailed(TagError {
                        path: file_path.to_str().unwrap_or("").to_string(),
                        public_message: "Failed to write tags to file".to_string(),
                        internal_message: "Failed to write tags to file".to_string(),
                    })
                })?
        } else {
            let updated_ilst_buffer = V0::encode_ilst(all_entries, Vec::new());
            V0::rebuild_file_insert_ilst(updated_ilst_buffer, &buffer).ok_or(
                BackendError::WriteFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "Failed to write tags to file".to_string(),
                    internal_message: "Failed to insert ilst atom".to_string(),
                }),
            )?
        };

        fs::write(file_path, &rebuilt_file).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to write tags to file".to_string(),
                internal_message: "Failed to write tags to file".to_string(),
            })
        })?;

        Ok(())
    }

    fn get_freeforms(
        &self,
        file_path: &std::path::PathBuf,
    ) -> Result<Vec<FreeformTag>, BackendError> {
        let buffer = std::fs::read(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                internal_message: "Unable to open and read file".to_string(),
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unable to read file.".to_string(),
            })
        })?;
        let ilst_atom = V0::ensure_ilst_atom(&buffer).map_err(|_| {
            BackendError::ReadFailed(TagError {
                internal_message: "Failed to find 'ilst' atom".to_string(),
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "The file is missing required metadata.".to_string(),
            })
        })?;
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
                if sz < 8 || cursor + sz > end {
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
                    "data" => {
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
            if let (Some(mean), Some(name), Some(val)) = (mean, name, value_text) {
                if FREEFORM_REVERSE_MAP
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
        }
        Ok(out)
    }
}
