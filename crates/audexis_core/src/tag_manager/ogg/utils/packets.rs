use super::*;

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
