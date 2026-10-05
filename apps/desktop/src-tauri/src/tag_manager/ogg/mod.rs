use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::vorbis_comments;

use super::traits::{Formats, TagFamily, TagFormat};
use super::utils::{replace_tmp, temp_path_for, FrameKey, TagValue};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use utils::{
    build_page_for_serial, read_page, write_page, PacketAssembler, PacketChunker, SerialMuxState,
    StreamClassifier, StreamKind,
};
mod utils;
#[derive(Debug, Clone)]
pub struct Ogg;
impl TagFamily for Ogg {
    fn new() -> Self {
        Self
    }
    fn get_release_class(&self, version: &Formats) -> Option<Box<dyn TagFormat>> {
        match version {
            Formats::Ogg => Some(Box::new(OggFormat::new())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct OggFormat;

impl TagFormat for OggFormat {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }
    fn get_tags(
        &self,
        file_path: &std::path::Path,
    ) -> Result<HashMap<FrameKey, Vec<TagValue>>, BackendError> {
        let file = File::open(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not open file".to_string(),
                internal_message: "Failed to open file".to_string(),
            })
        })?;

        let packet = utils::extract_comment_packet(file).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to find Vorbis/Opus comment packet".to_string(),
                internal_message: "Failed to find Vorbis/Opus comment packet".to_string(),
            })
        })?;

        let payload = if packet.len() >= 7 && &packet[0..7] == b"\x03vorbis" {
            packet[7..].to_vec()
        } else if packet.len() >= 8 && &packet[0..8] == b"OpusTags" {
            packet[8..].to_vec()
        } else {
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Unknown comment packet format".to_string(),
                internal_message: "Unknown comment packet format".to_string(),
            }));
        };

        let comment = vorbis_comments::utils::parse_comments(&payload);
        if comment.is_err() {
            println!("Error parsing comments");
            return Err(BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to parse Vorbis comments".to_string(),
                internal_message: "Failed to parse Vorbis comments".to_string(),
            }));
        }
        let comment = comment.unwrap();

        Ok(comment)
    }
    fn write_tags(
        &self,
        file_path: &std::path::Path,
        tags: std::collections::HashMap<FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        let mut merged: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
        merged.extend(tags);
        if let Ok(existing) = self.get_tags(file_path) {
            for (k, v) in existing {
                merged.entry(k).or_insert(v);
            }
        }

        let generic_payload = vorbis_comments::utils::build_comments(&merged, true);
        let new_vorbis_comment_packet = utils::make_vorbis_comment_packet(&generic_payload);
        let new_opus_tags_packet = utils::make_opus_tags_packet(&generic_payload);

        let input = File::open(file_path).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not open file".to_string(),
                internal_message: "Failed to open file".to_string(),
            })
        })?;
        let mut r = BufReader::new(input);

        let tmp_path = temp_path_for(file_path);
        let out = File::create(&tmp_path).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: tmp_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not create temporary file".to_string(),
                internal_message: "Failed to create temporary file".to_string(),
            })
        })?;
        let mut w = BufWriter::new(out);

        #[derive(Default)]
        struct OutState {
            next_seq: u32,
            bos_written: bool,
        }

        #[derive(Default)]
        struct EditState {
            kind: Option<StreamKind>,
            seen_vorbis_ident: bool,
            seen_vorbis_comment: bool,
            seen_vorbis_setup: bool,
            seen_opus_head: bool,
            seen_opus_tags: bool,
            header_packets: Vec<Vec<u8>>,
            header_ready: bool,
            header_emitted: bool,
            header_end_input_seq: Option<u32>,
            will_edit: bool,
        }

        fn write_copied_page(
            w: &mut BufWriter<File>,
            out_state: &mut HashMap<u32, OutState>,
            mut page: utils::OggPage,
        ) -> Result<(), ()> {
            let st = out_state.entry(page.bitstream_serial_number).or_default();
            page.page_sequence_number = st.next_seq;
            st.next_seq = st.next_seq.wrapping_add(1);
            if (page.header_type & 0x02) != 0 {
                st.bos_written = true;
            }
            write_page(w, &page).map_err(|_| ())
        }

        fn emit_rebuilt_headers(
            w: &mut BufWriter<File>,
            out_state: &mut HashMap<u32, OutState>,
            edit_state: &mut HashMap<u32, EditState>,
            serial: u32,
        ) -> Result<(), ()> {
            let (header_packets, already_emitted) = {
                let es = edit_state.get(&serial).ok_or(())?;
                (es.header_packets.clone(), es.header_emitted)
            };
            if already_emitted {
                return Ok(());
            }

            {
                let es = edit_state.get(&serial).ok_or(())?;
                if !es.header_ready || !es.will_edit {
                    return Err(());
                }
            }

            let st_next = out_state.entry(serial).or_default().next_seq;
            let st_bos = out_state.entry(serial).or_default().bos_written;

            let mut chunker = PacketChunker::new();
            for pkt in header_packets {
                chunker.push_packet(serial, pkt);
            }

            let mut mux = SerialMuxState {
                next_seq: st_next,
                bos_emitted: st_bos,
                eos_emitted: false,
                _prev_page_ended_mid_packet: false,
            };

            let mut first = true;
            while chunker.has_data(serial) {
                let page = build_page_for_serial(serial, 0, first, false, &mut mux, &mut chunker)
                    .ok_or(())?;
                write_page(w, &page).map_err(|_| ())?;
                first = false;
            }

            {
                let st = out_state.entry(serial).or_default();
                st.next_seq = mux.next_seq;
                st.bos_written = mux.bos_emitted;
            }

            edit_state.get_mut(&serial).ok_or(())?.header_emitted = true;
            Ok(())
        }

        let mut assembler = PacketAssembler::new();
        let mut classifier = StreamClassifier::new();

        let mut out_state: HashMap<u32, OutState> = HashMap::new();
        let mut edit_state: HashMap<u32, EditState> = HashMap::new();

        while let Some(page) = read_page(&mut r).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not read file".to_string(),
                internal_message: "Failed to read file".to_string(),
            })
        })? {
            let serial = page.bitstream_serial_number;

            for pkt in assembler.push_page(&page) {
                classifier.observe_packet(&pkt);
                let kind = classifier.kind.get(&pkt.serial).copied();
                let es = edit_state.entry(pkt.serial).or_default();

                if es.kind.is_none() {
                    es.kind = kind;
                }

                match es.kind {
                    Some(StreamKind::Vorbis) => {
                        es.will_edit = true;

                        if utils::is_vorbis_ident(&pkt.data) {
                            es.seen_vorbis_ident = true;
                            if es.header_packets.is_empty() {
                                es.header_packets.push(pkt.data.clone());
                            }
                        } else if utils::is_vorbis_comment(&pkt.data) {
                            es.seen_vorbis_comment = true;
                            es.header_packets.push(new_vorbis_comment_packet.clone());
                        } else if utils::is_vorbis_setup(&pkt.data) {
                            es.seen_vorbis_setup = true;
                            es.header_packets.push(pkt.data.clone());
                            es.header_ready = true;
                            es.header_end_input_seq = Some(page.page_sequence_number);
                        }
                    }
                    Some(StreamKind::Opus) => {
                        es.will_edit = true;

                        if utils::is_opus_head(&pkt.data) {
                            es.seen_opus_head = true;
                            if es.header_packets.is_empty() {
                                es.header_packets.push(pkt.data.clone());
                            }
                        } else if utils::is_opus_tags(&pkt.data) {
                            es.seen_opus_tags = true;
                            es.header_packets.push(new_opus_tags_packet.clone());
                            es.header_ready = true;
                            es.header_end_input_seq = Some(page.page_sequence_number);
                        }
                    }
                    _ => {}
                }
            }

            let (kind, will_edit, header_ready, header_emitted, header_end_seq) =
                match edit_state.get(&serial) {
                    Some(es) => (
                        es.kind,
                        es.will_edit,
                        es.header_ready,
                        es.header_emitted,
                        es.header_end_input_seq,
                    ),
                    None => (None, false, false, false, None),
                };

            let is_editable = matches!(kind, Some(StreamKind::Vorbis) | Some(StreamKind::Opus));

            if is_editable && will_edit {
                if !header_ready {
                    continue;
                }
                let end_seq = header_end_seq.ok_or(BackendError::WriteFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "Could not read file".to_string(),
                    internal_message: "Failed to read file".to_string(),
                }))?;
                if page.page_sequence_number <= end_seq {
                    continue;
                }

                if !header_emitted {
                    let r = emit_rebuilt_headers(&mut w, &mut out_state, &mut edit_state, serial);

                    if r.is_err() {
                        return Err(BackendError::WriteFailed(TagError {
                            path: file_path.to_str().unwrap_or("").to_string(),
                            public_message: "Could not write file".to_string(),
                            internal_message: "Failed to write file".to_string(),
                        }));
                    }
                }
            }

            write_copied_page(&mut w, &mut out_state, page).map_err(|_| {
                BackendError::WriteFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "Could not write file".to_string(),
                    internal_message: "Failed to write file".to_string(),
                })
            })?;
        }

        let serials: Vec<u32> = edit_state.keys().copied().collect();
        for serial in serials {
            let needs_emit = edit_state
                .get(&serial)
                .map(|es| es.will_edit && es.header_ready && !es.header_emitted)
                .unwrap_or(false);
            if needs_emit {
                let r = emit_rebuilt_headers(&mut w, &mut out_state, &mut edit_state, serial);
                if r.is_err() {
                    return Err(BackendError::WriteFailed(TagError {
                        path: file_path.to_str().unwrap_or("").to_string(),
                        public_message: "Could not write file".to_string(),
                        internal_message: "Failed to write file".to_string(),
                    }));
                }
            }
        }

        w.flush().map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not write file".to_string(),
                internal_message: "Failed to write file".to_string(),
            })
        })?;
        replace_tmp(&tmp_path, file_path).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Could not replace file".to_string(),
                internal_message: "Failed to replace file".to_string(),
            })
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod lyrics_tests {
    use super::*;
    use crate::tag_manager::tag_backend::{DefaultBackend, TagBackend};
    use crate::tag_manager::utils::{Changes, SerializableTagValue, TagChange};

    #[test]
    fn opus_lyrics_save_and_delete_preserve_audio_packet() {
        let path =
            std::env::temp_dir().join(format!("audexis-lyrics-{}.ogg", uuid::Uuid::new_v4()));
        let mut file = File::create(&path).unwrap();
        let audio = b"synthetic audio packet";
        for (seq, packet) in [
            b"OpusHead\x01\x02\0\0\x80\xbb\0\0\0\0\0".as_slice(),
            b"OpusTags\0\0\0\0\0\0\0\0",
            audio,
        ]
        .into_iter()
        .enumerate()
        {
            write_page(
                &mut file,
                &utils::OggPage {
                    version: 0,
                    header_type: if seq == 0 {
                        2
                    } else if seq == 2 {
                        4
                    } else {
                        0
                    },
                    granule_position: if seq == 2 { 960 } else { 0 },
                    bitstream_serial_number: 1,
                    page_sequence_number: seq as u32,
                    checksum: 0,
                    segment_table: vec![packet.len() as u8],
                    payload: packet.to_vec(),
                },
            )
            .unwrap();
        }
        drop(file);
        let backend = DefaultBackend::new();
        for delete in [false, true] {
            let tags = [
                (FrameKey::UnsyncedLyrics, "Café; 世界\nline"),
                (FrameKey::SynchronizedLyrics, "[00:01.500]Café; 世界"),
            ]
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    if delete {
                        TagChange::Delete
                    } else {
                        TagChange::Replace(vec![SerializableTagValue::Text(value.into())])
                    },
                )
            })
            .collect();
            let errors = backend.write_changes(&Changes {
                paths: vec![path.to_string_lossy().into_owned()],
                tags,
            });
            assert!(errors.is_empty(), "{errors:?}");
            let metadata = backend.read(&path).unwrap();
            assert_eq!(
                metadata.tags.contains_key(&FrameKey::UnsyncedLyrics),
                !delete
            );
            assert_eq!(
                metadata.tags.contains_key(&FrameKey::SynchronizedLyrics),
                !delete
            );
            let bytes = std::fs::read(&path).unwrap();
            assert!(bytes.windows(audio.len()).any(|window| window == audio));
        }
        std::fs::remove_file(path).unwrap();
    }
}
