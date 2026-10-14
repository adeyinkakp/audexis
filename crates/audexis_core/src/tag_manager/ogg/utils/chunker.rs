use super::*;

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
        if let Some(q) = self.packets.get_mut(&serial)
            && let Some(pkt) = q.pop_front()
        {
            self.cur.insert(serial, (pkt, 0));
            return true;
        }
        false
    }

    pub(super) fn take_page_data(
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
