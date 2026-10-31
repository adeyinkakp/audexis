use super::*;

fn picture(mime: &str, picture_type: Option<u8>, description: Option<&str>) -> TagValue {
    TagValue::Picture {
        mime: mime.to_string(),
        data: vec![1, 2, 3, 4],
        picture_type,
        description: description.map(str::to_string),
    }
}

#[test]
fn artwork_verification_normalizes_empty_optional_fields() {
    let expected = vec![picture("image/jpeg", Some(3), Some(""))];
    let actual = vec![picture("image/jpg", None, None)];

    assert!(values_preserved(
        &Formats::Id3v23,
        FrameKey::AttachedPicture,
        &expected,
        Some(&actual),
    ));
}

#[test]
fn itunes_artwork_verification_ignores_unsupported_fields() {
    let expected = vec![picture("image/png", Some(4), Some("Back cover"))];
    let actual = vec![picture("image/png", None, None)];

    assert!(values_preserved(
        &Formats::Itunes,
        FrameKey::AttachedPicture,
        &expected,
        Some(&actual),
    ));
}

#[test]
fn artwork_verification_still_rejects_changed_image_data() {
    let expected = vec![picture("image/png", Some(3), None)];
    let mut actual = vec![picture("image/png", Some(3), None)];
    if let TagValue::Picture { data, .. } = &mut actual[0] {
        data.push(5);
    }

    assert!(!values_preserved(
        &Formats::Flac,
        FrameKey::AttachedPicture,
        &expected,
        Some(&actual),
    ));
}
