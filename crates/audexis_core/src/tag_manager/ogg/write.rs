use super::*;

pub(super) fn write_tags(
    file_path: &std::path::Path,
    tags: std::collections::HashMap<FrameKey, Vec<TagValue>>,
) -> Result<(), BackendError> {
    let input = crate::library_files::open(file_path).map_err(|error| {
        BackendError::WriteFailed(TagError {
            path: file_path.to_string_lossy().into_owned(),
            public_message: "Could not open file".into(),
            internal_message: error.to_string(),
        })
    })?;
    let packet = utils::extract_comment_packet(input).map_err(|_| {
        BackendError::WriteFailed(TagError {
            path: file_path.to_string_lossy().into_owned(),
            public_message: "Could not read comments".into(),
            internal_message: "Failed to extract comment packet".into(),
        })
    })?;
    let original = packet
        .strip_prefix(b"\x03vorbis")
        .or_else(|| packet.strip_prefix(b"OpusTags"))
        .ok_or_else(|| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_string_lossy().into_owned(),
                public_message: "Unsupported comments".into(),
                internal_message: "Unknown comment packet".into(),
            })
        })?;
    let generic_payload =
        vorbis_comments::utils::update_comments(original, &tags, true).map_err(|error| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_string_lossy().into_owned(),
                public_message: "Could not update comments".into(),
                internal_message: error.to_string(),
            })
        })?;
    let new_vorbis_comment_packet = utils::make_vorbis_comment_packet(&generic_payload);
    let new_opus_tags_packet = utils::make_opus_tags_packet(&generic_payload);

    let input = crate::library_files::open(file_path).map_err(|_| {
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
            let page =
                build_page_for_serial(serial, 0, first, false, &mut mux, &mut chunker).ok_or(())?;
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
            let end_seq = header_end_seq.ok_or_else(|| {
                BackendError::WriteFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "Could not read file".to_string(),
                    internal_message: "Missing header end sequence".to_string(),
                })
            })?;
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
