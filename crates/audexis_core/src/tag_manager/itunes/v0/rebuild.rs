use super::*;

impl V0 {
    pub(super) fn rebuild_file(ilst_buffer: Vec<u8>, file_buffer: &[u8]) -> Option<Vec<u8>> {
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
        let shift = ilst_buffer.len() as i64 - ilst_atom.size as i64;
        let co64_atoms = V0::find_co64_atom(&almost_done_file, new_moov_atom);
        let stco_atoms = V0::find_stco_atom(&almost_done_file, new_moov_atom);
        let mut rebuilt = almost_done_file;
        if let Some(atoms) = co64_atoms {
            rebuilt = V0::update_co64_offsets(
                &rebuilt,
                shift,
                moov_atom.position + moov_atom.size,
                &atoms,
            )?;
        }
        if let Some(atoms) = stco_atoms {
            rebuilt = V0::update_stco_offsets(
                &rebuilt,
                shift,
                moov_atom.position + moov_atom.size,
                &atoms,
            )?;
        }
        Some(rebuilt)
    }

    pub(super) fn rebuild_file_insert_ilst(
        ilst_atom_buffer: Vec<u8>,
        file_buffer: &[u8],
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
        let shift = moov_size_delta;
        let co64_atoms = V0::find_co64_atom(&almost_done_file, new_moov_atom);
        let stco_atoms = V0::find_stco_atom(&almost_done_file, new_moov_atom);
        let mut rebuilt = almost_done_file;
        if let Some(atoms) = co64_atoms {
            rebuilt = V0::update_co64_offsets(
                &rebuilt,
                shift,
                moov_atom.position + moov_atom.size,
                &atoms,
            )?;
        }
        if let Some(atoms) = stco_atoms {
            rebuilt = V0::update_stco_offsets(
                &rebuilt,
                shift,
                moov_atom.position + moov_atom.size,
                &atoms,
            )?;
        }
        Some(rebuilt)
    }
}
