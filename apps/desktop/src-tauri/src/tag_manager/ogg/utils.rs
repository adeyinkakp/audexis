use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::sync::OnceLock;

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
#[derive(Default)]
pub struct StreamClassifier {
    pub kind: HashMap<u32, StreamKind>,
}

impl StreamClassifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_packet(&mut self, pkt: &Packet) {
        if pkt.data.is_empty() {
            return;
        }

        let entry = self.kind.entry(pkt.serial).or_insert(StreamKind::Other);

        if *entry == StreamKind::Vorbis || *entry == StreamKind::Opus {
            return;
        }

        if is_any_vorbis_header(&pkt.data) {
            *entry = StreamKind::Vorbis;
        } else if is_opus_head(&pkt.data) || is_opus_tags(&pkt.data) {
            *entry = StreamKind::Opus;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Vorbis,
    Opus,
    Other,
}

#[derive(Default)]
pub struct PacketChunker {
    pub packets: HashMap<u32, VecDeque<Vec<u8>>>,
    pub cur: HashMap<u32, (Vec<u8>, usize)>,
    pub need_terminator: HashSet<u32>,
}

impl PacketChunker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_packet(&mut self, serial: u32, pkt: Vec<u8>) {
        self.packets.entry(serial).or_default().push_back(pkt);
    }

    pub fn has_data(&self, serial: u32) -> bool {
        self.need_terminator.contains(&serial)
            || self.cur.contains_key(&serial)
            || self
                .packets
                .get(&serial)
                .map(|q| !q.is_empty())
                .unwrap_or(false)
    }

    pub fn ensure_cur(&mut self, serial: u32) -> bool {
        if self.cur.contains_key(&serial) {
            return true;
        }
        if let Some(q) = self.packets.get_mut(&serial) {
            if let Some(pkt) = q.pop_front() {
                self.cur.insert(serial, (pkt, 0));
                return true;
            }
        }
        false
    }

    fn take_page_data(
        &mut self,
        serial: u32,
        max_segments: usize,
        max_payload: usize,
    ) -> Option<(Vec<u8>, Vec<u8>, bool, bool)> {
        if !self.has_data(serial) {
            return None;
        }

        let mut segs: Vec<u8> = Vec::new();
        let mut payload: Vec<u8> = Vec::new();

        if self.need_terminator.contains(&serial) && segs.len() < max_segments {
            segs.push(0);
            self.need_terminator.remove(&serial);
        }

        self.ensure_cur(serial);

        let begins_continued = self
            .cur
            .get(&serial)
            .map(|(pkt, off)| !pkt.is_empty() && *off > 0)
            .unwrap_or(false);

        while segs.len() < max_segments && payload.len() < max_payload {
            if !self.ensure_cur(serial) {
                break;
            }
            let (pkt, off) = self.cur.get_mut(&serial).unwrap();

            if pkt.is_empty() {
                self.cur.remove(&serial);
                continue;
            }

            let remaining = pkt.len().saturating_sub(*off);
            if remaining == 0 {
                self.cur.remove(&serial);
                continue;
            }

            let space = max_payload - payload.len();
            if space == 0 {
                break;
            }

            let take = remaining.min(255).min(space);
            payload.extend_from_slice(&pkt[*off..*off + take]);
            *off += take;
            segs.push(take as u8);

            let ended_packet = *off == pkt.len();

            if ended_packet {
                if take < 255 {
                    self.cur.remove(&serial);
                    continue;
                } else {
                    if segs.len() < max_segments {
                        segs.push(0);
                        self.cur.remove(&serial);
                        continue;
                    } else {
                        self.need_terminator.insert(serial);
                        self.cur.remove(&serial);
                        break;
                    }
                }
            } else {
                break;
            }
        }

        if segs.is_empty() {
            return None;
        }

        let ends_mid_packet = self
            .cur
            .get(&serial)
            .map(|(pkt, off)| !pkt.is_empty() && *off < pkt.len())
            .unwrap_or(false);

        Some((segs, payload, begins_continued, ends_mid_packet))
    }
}

const MAX_SEGMENTS: usize = 255;
const MAX_PAYLOAD: usize = 255 * 255;

#[derive(Default)]
pub struct SerialMuxState {
    pub next_seq: u32,
    pub bos_emitted: bool,
    pub eos_emitted: bool,
    pub _prev_page_ended_mid_packet: bool,
}

pub fn build_page_for_serial(
    serial: u32,
    granule_position: u64,
    want_bos: bool,
    want_eos: bool,
    state: &mut SerialMuxState,
    chunker: &mut PacketChunker,
) -> Option<OggPage> {
    if state.eos_emitted {
        return None;
    }

    let (segment_table, payload, begins_continued, ends_mid_packet) =
        chunker.take_page_data(serial, MAX_SEGMENTS, MAX_PAYLOAD)?;

    let mut header_type: u8 = 0;
    if begins_continued {
        header_type |= 0x01;
    }
    if want_bos && !state.bos_emitted {
        header_type |= 0x02;
    }
    if want_eos {
        header_type |= 0x04;
        state.eos_emitted = true;
    }

    let page = OggPage {
        version: 0,
        header_type,
        granule_position,
        bitstream_serial_number: serial,
        page_sequence_number: state.next_seq,
        checksum: 0,
        segment_table,
        payload,
    };

    state.next_seq += 1;
    if (page.header_type & 0x02) != 0 {
        state.bos_emitted = true;
    }
    state._prev_page_ended_mid_packet = ends_mid_packet;

    Some(page)
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub serial: u32,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct PacketAssembler {
    curr: HashMap<u32, Vec<u8>>,
}

pub fn make_vorbis_comment_packet(comment_payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(7 + comment_payload.len() + 1);
    out.push(0x03);
    out.extend_from_slice(b"vorbis");
    out.extend_from_slice(comment_payload);
    out.push(0x01);
    out
}

pub fn make_opus_tags_packet(comment_payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + comment_payload.len());
    out.extend_from_slice(b"OpusTags");
    out.extend_from_slice(comment_payload);
    out
}

impl PacketAssembler {
    pub fn new() -> Self {
        Self {
            curr: HashMap::new(),
        }
    }

    pub fn push_page(&mut self, page: &OggPage) -> Vec<Packet> {
        let serial = page.bitstream_serial_number;
        let buf = self.curr.entry(serial).or_default();

        let mut out = Vec::new();
        let mut off = 0usize;

        for &seg in &page.segment_table {
            let seg_len = seg as usize;
            let slice = &page.payload[off..off + seg_len];
            off += seg_len;

            buf.extend_from_slice(slice);

            if seg_len < 255 {
                let mut pkt = Vec::new();
                std::mem::swap(&mut pkt, buf);
                out.push(Packet { serial, data: pkt });
            }
        }

        out
    }
}

pub fn is_any_vorbis_header(p: &[u8]) -> bool {
    p.len() >= 7 && matches!(p[0], 0x01 | 0x03 | 0x05) && &p[1..7] == b"vorbis"
}

pub fn is_vorbis_ident(p: &[u8]) -> bool {
    p.len() >= 7 && p[0] == 0x01 && &p[1..7] == b"vorbis"
}
pub fn is_vorbis_comment(p: &[u8]) -> bool {
    p.len() >= 7 && p[0] == 0x03 && &p[1..7] == b"vorbis"
}
pub fn is_vorbis_setup(p: &[u8]) -> bool {
    p.len() >= 7 && p[0] == 0x05 && &p[1..7] == b"vorbis"
}

pub fn is_opus_head(p: &[u8]) -> bool {
    p.len() >= 8 && &p[0..8] == b"OpusHead"
}
pub fn is_opus_tags(p: &[u8]) -> bool {
    p.len() >= 8 && &p[0..8] == b"OpusTags"
}

pub fn read_page<R: Read>(r: &mut R) -> io::Result<Option<OggPage>> {
    let mut header = [0u8; 27];
    match r.read_exact(&mut header) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
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
