use super::*;

impl V0 {
    pub(super) fn find_co64_atom(buffer: &[u8], moov_atom: &Atom) -> Option<Vec<Atom>> {
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

    pub(super) fn find_stco_atom(buffer: &[u8], moov_atom: &Atom) -> Option<Vec<Atom>> {
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

    pub(super) fn update_co64_offsets(
        file_buffer: &[u8],
        shift: i64,
        cutoff: u64,
        co64_sub_atoms: &Vec<Atom>,
    ) -> Option<Vec<u8>> {
        let mut buff = file_buffer.to_owned();

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

                let new_offset = if old_offset >= cutoff {
                    old_offset.checked_add_signed(shift)?
                } else {
                    old_offset
                };
                let new_bytes = new_offset.to_be_bytes();
                co64_buffer[pos..pos + 8].copy_from_slice(&new_bytes);
            }

            buff.splice(start..end, co64_buffer);
        }

        Some(buff)
    }

    pub(super) fn update_stco_offsets(
        file_buffer: &[u8],
        shift: i64,
        cutoff: u64,
        stco_sub_atoms: &Vec<Atom>,
    ) -> Option<Vec<u8>> {
        let mut buff = file_buffer.to_owned();

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

                let new_offset = if u64::from(old_offset) >= cutoff {
                    u32::try_from(i64::from(old_offset).checked_add(shift)?).ok()?
                } else {
                    old_offset
                };
                let new_bytes = new_offset.to_be_bytes();
                stco_buffer[pos..pos + 4].copy_from_slice(&new_bytes);
            }

            buff.splice(start..end, stco_buffer);
        }

        Some(buff)
    }
}
