use super::*;

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
    payload: Vec<u8>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("audexis-tag-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("track.mp3");
        // fake post-tag bytes test preservation not for audio
        let payload: Vec<u8> = (0..2048).map(|i| (i % 251) as u8).collect();
        let mut contents = b"ID3\x03\x00\x00\x00\x00\x00\x00".to_vec();
        contents.extend_from_slice(&payload);
        fs::write(&path, contents).unwrap();
        Self {
            directory,
            path,
            payload,
        }
    }

    fn write(&self, tags: HashMap<FrameKey, TagChange>) -> Vec<BackendError> {
        DefaultBackend::new().write_changes(&Changes {
            paths: vec![self.path.to_string_lossy().into_owned()],
            tags,
        })
    }

    fn assert_payload_preserved(&self) {
        let bytes = fs::read(&self.path).unwrap();
        let tag_size = bytes[6..10]
            .iter()
            .fold(0usize, |size, byte| (size << 7) | usize::from(*byte));
        assert_eq!(&bytes[10 + tag_size..], self.payload);
        assert_eq!(
            fs::read_dir(&self.directory).unwrap().count(),
            1,
            "temporary write files should be cleaned up"
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn text(value: &str) -> TagChange {
    TagChange::Replace(vec![SerializableTagValue::Text(value.into())])
}

#[test]
fn text_updates_and_deletes_round_trip_without_changing_unedited_tags_or_payload() {
    let fixture = Fixture::new();
    let errors = fixture.write(HashMap::from([
        (FrameKey::Title, text("no where")),
        (FrameKey::Artist, text("yb")),
    ]));
    assert!(errors.is_empty(), "{errors:?}");
    let errors = fixture.write(HashMap::from([(FrameKey::Title, text("Updated title"))]));
    assert!(errors.is_empty(), "{errors:?}");
    let metadata = DefaultBackend::new().read(&fixture.path).unwrap();
    assert_eq!(
        metadata.tags[&FrameKey::Title],
        vec![TagValue::Text("Updated title".into())]
    );
    assert_eq!(
        metadata.tags[&FrameKey::Artist],
        vec![TagValue::Text("yb".into())]
    );
    fixture.assert_payload_preserved();
    let errors = fixture.write(HashMap::from([(FrameKey::Title, TagChange::Delete)]));
    assert!(errors.is_empty(), "{errors:?}");
    let metadata = DefaultBackend::new().read(&fixture.path).unwrap();
    assert!(!metadata.tags.contains_key(&FrameKey::Title));
    assert_eq!(
        metadata.tags[&FrameKey::Artist],
        vec![TagValue::Text("yb".into())]
    );
    fixture.assert_payload_preserved();
}

#[test]
fn artwork_round_trips_and_can_be_removed_without_changing_payload() {
    let fixture = Fixture::new();
    let image = b"\x89PNG\r\n\x1a\nsynthetic artwork bytes";
    let errors = fixture.write(HashMap::from([(
        FrameKey::AttachedPicture,
        TagChange::Replace(vec![SerializableTagValue::Picture {
            mime: "image/png".into(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(image),
            picture_type: Some(3),
            description: Some("Cover".into()),
        }]),
    )]));
    assert!(errors.is_empty(), "{errors:?}");
    let metadata = DefaultBackend::new().read(&fixture.path).unwrap();
    match &metadata.tags[&FrameKey::AttachedPicture][0] {
        TagValue::Picture { data, mime, .. } => {
            assert_eq!(data, image);
            assert_eq!(mime, "image/png");
        }
        _ => panic!("Expected artwork"),
    }
    fixture.assert_payload_preserved();
    let errors = fixture.write(HashMap::from([(
        FrameKey::AttachedPicture,
        TagChange::Delete,
    )]));
    assert!(errors.is_empty(), "{errors:?}");
    assert!(!DefaultBackend::new()
        .read(&fixture.path)
        .unwrap()
        .tags
        .contains_key(&FrameKey::AttachedPicture));
    fixture.assert_payload_preserved();
}

#[test]
fn invalid_artwork_leaves_original_file_unchanged() {
    let fixture = Fixture::new();
    let before = fs::read(&fixture.path).unwrap();
    let errors = fixture.write(HashMap::from([(
        FrameKey::AttachedPicture,
        TagChange::Replace(vec![SerializableTagValue::Picture {
            mime: "image/png".into(),
            data_base64: "not valid base64!".into(),
            picture_type: Some(3),
            description: None,
        }]),
    )]));
    assert_eq!(errors.len(), 1);
    assert_eq!(fs::read(&fixture.path).unwrap(), before);
    fixture.assert_payload_preserved();
}
