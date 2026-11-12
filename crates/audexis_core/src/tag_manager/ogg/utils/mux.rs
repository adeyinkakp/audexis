use super::*;

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
