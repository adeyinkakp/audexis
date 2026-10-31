#[derive(Debug, Clone)]
pub(super) struct Atom {
    pub(super) atom_type: String,
    pub(super) size: u64,
    pub(super) position: u64,
    pub(super) buffer: Vec<u8>,
}

pub(super) fn parse_atoms(buffer: &[u8], start: u64, end: u64) -> Vec<Atom> {
    let mut position = start;
    let end = end.min(buffer.len() as u64);
    let mut atoms = Vec::new();

    while position + 8 <= end {
        let offset = position as usize;
        let atom_size = u32::from_be_bytes(buffer[offset..offset + 4].try_into().unwrap()) as u64;
        if atom_size < 8 || atom_size > end - position {
            break;
        }
        let atom_type = if buffer[offset + 4] == 0xa9 {
            format!(
                "©{}",
                String::from_utf8_lossy(&buffer[offset + 5..offset + 8])
            )
        } else {
            String::from_utf8_lossy(&buffer[offset + 4..offset + 8]).to_string()
        };
        atoms.push(Atom {
            atom_type,
            size: atom_size,
            position,
            buffer: buffer[offset..offset + atom_size as usize].to_vec(),
        });
        position += atom_size;
    }
    atoms
}

pub(super) fn find_ilst(buffer: &[u8]) -> Option<Atom> {
    let moov = parse_atoms(buffer, 0, buffer.len() as u64)
        .into_iter()
        .find(|atom| atom.atom_type == "moov")?;
    let udta = parse_atoms(&moov.buffer, 8, moov.size)
        .into_iter()
        .find(|atom| atom.atom_type == "udta")?;
    let meta = parse_atoms(&udta.buffer, 8, udta.size)
        .into_iter()
        .find(|atom| atom.atom_type == "meta")?;
    parse_atoms(&meta.buffer, 12, meta.size)
        .into_iter()
        .find(|atom| atom.atom_type == "ilst")
}

pub(super) fn validate_for_write(buffer: &[u8]) -> Result<(), String> {
    fn walk(bytes: &[u8], metadata_entries: bool, depth: usize) -> Result<(), String> {
        if depth > 32 {
            return Err("MP4 atom nesting is too deep".into());
        }
        let mut pos = 0;
        while pos < bytes.len() {
            let header = bytes.get(pos..pos + 8).ok_or("Truncated MP4 atom header")?;
            let size = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
            if size < 8 {
                return Err("Unsupported MP4 atom size".into());
            }
            let atom = bytes.get(pos..pos + size).ok_or("Truncated MP4 atom")?;
            let kind = &header[4..8];
            if kind == b"meta" {
                walk(
                    atom.get(12..).ok_or("Truncated MP4 meta atom")?,
                    false,
                    depth + 1,
                )?;
            } else if metadata_entries
                || matches!(
                    kind,
                    b"moov" | b"udta" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"ilst"
                )
            {
                walk(&atom[8..], kind == b"ilst", depth + 1)?;
            }
            if matches!(kind, b"stco" | b"co64") {
                let count = u32::from_be_bytes(
                    atom.get(12..16)
                        .ok_or("Truncated chunk table")?
                        .try_into()
                        .unwrap(),
                ) as usize;
                let width = if kind == b"stco" { 4 } else { 8 };
                if count > (size.saturating_sub(16)) / width {
                    return Err("Truncated chunk table entries".into());
                }
            }
            pos += size;
        }
        Ok(())
    }
    walk(buffer, false, 0)
}

pub(super) fn read_ilst<R: std::io::Read + std::io::Seek>(
    file: &mut R,
) -> std::io::Result<Option<Atom>> {
    use std::io::{Error, ErrorKind, SeekFrom};
    fn walk<R: std::io::Read + std::io::Seek>(
        file: &mut R,
        mut pos: u64,
        end: u64,
        depth: usize,
    ) -> std::io::Result<Option<Atom>> {
        let invalid = |message| Error::new(ErrorKind::InvalidData, message);
        if depth > 8 {
            return Err(invalid("MP4 metadata nesting is too deep"));
        }
        while pos < end {
            if end - pos < 8 {
                return Err(invalid("Truncated MP4 atom header"));
            }
            file.seek(SeekFrom::Start(pos))?;
            let mut header = [0; 8];
            file.read_exact(&mut header)?;
            let size32 = u32::from_be_bytes(header[..4].try_into().unwrap());
            let (size, header_size) = match size32 {
                0 => (end - pos, 8),
                1 => {
                    if end - pos < 16 {
                        return Err(invalid("Truncated extended MP4 atom"));
                    }
                    let mut extended = [0; 8];
                    file.read_exact(&mut extended)?;
                    (u64::from_be_bytes(extended), 16)
                }
                _ => (u64::from(size32), 8),
            };
            if size < header_size || size > end - pos {
                return Err(invalid("Invalid MP4 atom size"));
            }
            let kind = &header[4..];
            if kind == b"ilst" {
                let payload = size - header_size;
                if payload > 64 * 1024 * 1024 {
                    return Err(invalid("MP4 metadata exceeds 64 MB"));
                }
                let normalized_size = payload + 8;
                let mut buffer = vec![0; normalized_size as usize];
                buffer[..4].copy_from_slice(&(normalized_size as u32).to_be_bytes());
                buffer[4..8].copy_from_slice(b"ilst");
                file.read_exact(&mut buffer[8..])?;
                return Ok(Some(Atom {
                    atom_type: "ilst".into(),
                    size: normalized_size,
                    position: 0,
                    buffer,
                }));
            }
            if matches!(kind, b"moov" | b"udta" | b"meta") {
                let skip = if kind == b"meta" { 4 } else { 0 };
                if size < header_size + skip {
                    return Err(invalid("Truncated MP4 metadata container"));
                }
                if let Some(atom) = walk(file, pos + header_size + skip, pos + size, depth + 1)? {
                    return Ok(Some(atom));
                }
            }
            pos += size;
        }
        Ok(None)
    }
    let end = file.seek(SeekFrom::End(0))?;
    walk(file, 0, end, 0)
}
