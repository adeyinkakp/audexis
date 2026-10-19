use super::*;
use crate::tag_manager::tag_backend::{DefaultBackend, TagBackend};
use crate::tag_manager::utils::{Changes, SerializableTagValue, TagChange};

#[test]
fn opus_metadata_save_and_delete_preserve_audio_packet() {
    metadata_round_trip(true);
}

#[test]
fn vorbis_metadata_save_and_delete_preserve_audio_packet() {
    metadata_round_trip(false);
}

fn metadata_round_trip(opus: bool) {
    let path = std::env::temp_dir().join(format!("audexis-lyrics-{}.ogg", uuid::Uuid::new_v4()));
    let mut file = File::create(&path).unwrap();
    let audio = b"synthetic audio packet";
    let mut packets = if opus {
        vec![
            b"OpusHead\x01\x02\0\0\x80\xbb\0\0\0\0\0".to_vec(),
            b"OpusTags\0\0\0\0\0\0\0\0".to_vec(),
        ]
    } else {
        vec![
            b"\x01vorbis\0\0\0\0\x02\x80\xbb\0\0".to_vec(),
            utils::make_vorbis_comment_packet(&vorbis_comments::utils::build_comments(
                &HashMap::new(),
                true,
            )),
            b"\x05vorbis setup".to_vec(),
        ]
    };
    let custom = b"CUSTOM_FIELD=Keep / this; exactly";
    let vendor = b"Original encoder";
    let mut comments = (vendor.len() as u32).to_le_bytes().to_vec();
    comments.extend(vendor);
    comments.extend(1u32.to_le_bytes());
    comments.extend((custom.len() as u32).to_le_bytes());
    comments.extend(custom);
    packets[1] = if opus {
        utils::make_opus_tags_packet(&comments)
    } else {
        utils::make_vorbis_comment_packet(&comments)
    };
    packets.push(audio.to_vec());
    let last_page = packets.len() - 1;
    for (seq, packet) in packets.into_iter().enumerate() {
        write_page(
            &mut file,
            &utils::OggPage {
                version: 0,
                header_type: if seq == 0 {
                    2
                } else if seq == last_page {
                    4
                } else {
                    0
                },
                granule_position: if seq == last_page { 960 } else { 0 },
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
            (FrameKey::Title, "Café / 世界; title"),
            (FrameKey::Artist, "AC/DC"),
            (FrameKey::Genre, "R&B; Soul"),
            (FrameKey::DiscNumber, "2"),
            (FrameKey::TotalDiscs, "3"),
            (FrameKey::TrackNumber, "4"),
            (FrameKey::TotalTracks, "12"),
            (FrameKey::Comments, "Keep this comment"),
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
        for (key, value) in [
            (FrameKey::Title, "Café / 世界; title"),
            (FrameKey::Artist, "AC/DC"),
            (FrameKey::Genre, "R&B; Soul"),
            (FrameKey::DiscNumber, "2"),
            (FrameKey::TotalDiscs, "3"),
            (FrameKey::TrackNumber, "4"),
            (FrameKey::TotalTracks, "12"),
            (FrameKey::Comments, "Keep this comment"),
        ] {
            if delete {
                assert!(!metadata.tags.contains_key(&key));
            } else {
                assert_eq!(
                    metadata.tags.get(&key),
                    Some(&vec![TagValue::Text(value.into())])
                );
            }
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.windows(audio.len()).any(|window| window == audio));
        assert!(bytes.windows(custom.len()).any(|window| window == custom));
        assert!(bytes.windows(vendor.len()).any(|window| window == vendor));
    }
    std::fs::remove_file(path).unwrap();
}
