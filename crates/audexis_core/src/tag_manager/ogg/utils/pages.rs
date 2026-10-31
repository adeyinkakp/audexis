use super::*;

#[derive(Debug, Clone)]
pub struct OggPage {
    pub version: u8,
    pub header_type: u8,
    pub granule_position: u64,
    pub bitstream_serial_number: u32,
    pub page_sequence_number: u32,
    #[allow(dead_code)]
    pub checksum: u32,
    pub segment_table: Vec<u8>,
    pub payload: Vec<u8>,
}

pub fn extract_comment_packet(file: File) -> Result<Vec<u8>, ()> {
    let mut reader = BufReader::new(file);
    let mut curr_packet: Vec<u8> = Vec::new();
    loop {
        let mut header = [0u8; 27];
        if reader.read_exact(&mut header).is_err() {
            break;
        }

        if &header[0..4] != b"OggS" {
            break;
        }

        let page_segments = header[26];
        let mut segment_table = vec![0u8; page_segments as usize];
        reader.read_exact(&mut segment_table).map_err(|_| ())?;

        for &segment_size in &segment_table {
            let mut segment_data = vec![0u8; segment_size as usize];
            reader.read_exact(&mut segment_data).map_err(|_| ())?;
            curr_packet.extend_from_slice(&segment_data);

            if segment_size < 255 {
                let is_vorbis_comment =
                    curr_packet.len() >= 7 && &curr_packet[0..7] == b"\x03vorbis";
                let is_opus_tags = curr_packet.len() >= 8 && &curr_packet[0..8] == b"OpusTags";
                if is_vorbis_comment || is_opus_tags {
                    return Ok(curr_packet);
                }
                curr_packet.clear();
            }
        }
    }
    Err(())
}
pub fn write_page<W: Write>(w: &mut W, page: &OggPage) -> io::Result<()> {
    let page_segments = page.segment_table.len();
    if page_segments > 255 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Too many segments",
        ));
    }

    let mut header = [0u8; 27];
    header[0..4].copy_from_slice(b"OggS");
    header[4] = page.version;
    header[5] = page.header_type;
    header[6..14].copy_from_slice(&page.granule_position.to_le_bytes());
    header[14..18].copy_from_slice(&page.bitstream_serial_number.to_le_bytes());
    header[18..22].copy_from_slice(&page.page_sequence_number.to_le_bytes());
    header[22..26].copy_from_slice(&0u32.to_le_bytes());
    header[26] = page_segments as u8;

    let mut full = Vec::with_capacity(27 + page.segment_table.len() + page.payload.len());
    full.extend_from_slice(&header);
    full.extend_from_slice(&page.segment_table);
    full.extend_from_slice(&page.payload);

    let crc = ogg_crc32(&full);

    header[22..26].copy_from_slice(&crc.to_le_bytes());
    w.write_all(&header)?;
    w.write_all(&page.segment_table)?;
    w.write_all(&page.payload)?;
    Ok(())
}

pub fn ogg_crc32(data: &[u8]) -> u32 {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();

    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];

        for (i, item) in t.iter_mut().enumerate() {
            let mut r = (i as u32) << 24;
            for _ in 0..8 {
                if (r & 0x8000_0000) != 0 {
                    r = (r << 1) ^ 0x04C11DB7;
                } else {
                    r <<= 1;
                }
            }
            *item = r;
        }
        t
    });

    let mut crc: u32 = 0;
    for &b in data {
        let idx = ((crc >> 24) as u8) ^ b;
        crc = (crc << 8) ^ table[idx as usize];
    }
    crc
}
pub fn read_page<R: Read>(r: &mut R) -> io::Result<Option<OggPage>> {
    let mut header = [0u8; 27];
    match r.read(&mut header[..1]) {
        Ok(0) => return Ok(None),
        Ok(_) => r.read_exact(&mut header[1..])?,
        Err(error) => return Err(error),
    }

    if &header[0..4] != b"OggS" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Missing OggS capture pattern",
        ));
    }
    let version = header[4];
    if version != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unsupported Ogg version",
        ));
    }

    let header_type = header[5];
    let granule_position = u64::from_le_bytes(header[6..14].try_into().unwrap());
    let bitstream_serial_number = u32::from_le_bytes(header[14..18].try_into().unwrap());
    let page_sequence_number = u32::from_le_bytes(header[18..22].try_into().unwrap());
    let checksum = u32::from_le_bytes(header[22..26].try_into().unwrap());
    let page_segments = header[26] as usize;

    let mut segment_table = vec![0u8; page_segments];
    r.read_exact(&mut segment_table)?;

    let payload_len: usize = segment_table.iter().map(|&b| b as usize).sum();
    let mut payload = vec![0u8; payload_len];
    r.read_exact(&mut payload)?;

    Ok(Some(OggPage {
        version,
        header_type,
        granule_position,
        bitstream_serial_number,
        page_sequence_number,
        checksum,
        segment_table,
        payload,
    }))
}
