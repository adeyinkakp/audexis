use super::*;

pub(super) fn values_preserved(
    format: &Formats,
    key: FrameKey,
    expected: &[TagValue],
    actual: Option<&Vec<TagValue>>,
) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    if *format == Formats::Itunes && matches!(key, FrameKey::TrackNumber | FrameKey::DiscNumber) {
        let ([TagValue::Text(expected)], [TagValue::Text(actual)]) = (expected, actual.as_slice())
        else {
            return false;
        };
        return match (
            super::super::itunes::utils::parse_number_pair(expected),
            super::super::itunes::utils::parse_number_pair(actual),
        ) {
            (Some((number, total)), Some((written_number, written_total))) => {
                number == written_number
                    && total.is_none_or(|total| total == written_total.unwrap_or(0))
            }
            _ => false,
        };
    }
    if key == FrameKey::UserDefinedText
        && matches!(format, Formats::Flac | Formats::Ogg | Formats::Itunes)
    {
        let normalized = |values: &[TagValue]| {
            let mut entries = values
                .iter()
                .map(|value| match value {
                    TagValue::UserText(entry) => (
                        if matches!(format, Formats::Flac | Formats::Ogg) {
                            entry.description.to_ascii_uppercase()
                        } else {
                            super::super::itunes::utils::custom_key(&entry.description)
                        },
                        entry.value.clone(),
                    ),
                    _ => (String::new(), value.to_string()),
                })
                .collect::<Vec<_>>();
            entries.sort();
            entries
        };
        return normalized(expected) == normalized(actual);
    }
    if key == FrameKey::SynchronizedLyrics {
        let parse = |values: &[TagValue]| {
            values
                .iter()
                .map(|value| match value {
                    TagValue::Text(text) => super::super::id3::lyrics::parse_lrc(text),
                    _ => Err("Expected lyrics text".into()),
                })
                .collect::<Result<Vec<_>, String>>()
        };
        return matches!((parse(expected), parse(actual)), (Ok(a), Ok(b)) if a == b);
    }
    if key != FrameKey::AttachedPicture {
        return actual == expected;
    }
    if actual.len() != expected.len() {
        return false;
    }

    expected.iter().zip(actual).all(|(expected, actual)| {
        let (
            TagValue::Picture {
                mime: expected_mime,
                data: expected_data,
                picture_type: expected_type,
                description: expected_description,
            },
            TagValue::Picture {
                mime: actual_mime,
                data: actual_data,
                picture_type: actual_type,
                description: actual_description,
            },
        ) = (expected, actual)
        else {
            return false;
        };

        let is_jpeg = |mime: &str| {
            mime.eq_ignore_ascii_case("image/jpg") || mime.eq_ignore_ascii_case("image/jpeg")
        };
        let mime_matches = expected_mime.eq_ignore_ascii_case(actual_mime)
            || (is_jpeg(expected_mime) && is_jpeg(actual_mime));
        let image_matches = mime_matches && expected_data == actual_data;
        if *format == Formats::Itunes {
            return image_matches;
        }

        image_matches
            && expected_type.unwrap_or(3) == actual_type.unwrap_or(3)
            && expected_description.as_deref().unwrap_or_default()
                == actual_description.as_deref().unwrap_or_default()
    })
}

pub(super) fn to_tag_value(value: &SerializableTagValue) -> Result<TagValue, String> {
    match value {
        SerializableTagValue::Text(text) => Ok(TagValue::Text(text.clone())),
        SerializableTagValue::Picture {
            mime,
            data_base64,
            picture_type,
            description,
        } => base64::engine::general_purpose::STANDARD
            .decode(data_base64)
            .map(|data| TagValue::Picture {
                mime: mime.clone(),
                data,
                picture_type: *picture_type,
                description: description.clone(),
            })
            .map_err(|error| format!("Invalid picture data: {error}")),
        SerializableTagValue::UserText(entry) => Ok(TagValue::UserText(entry.clone())),
        SerializableTagValue::UserUrl(entry) => Ok(TagValue::UserUrl(entry.clone())),
        SerializableTagValue::Comment {
            encoding,
            language,
            description,
            text,
        } => Ok(TagValue::Comment {
            encoding: encoding.clone(),
            language: language.clone(),
            description: description.clone(),
            text: text.clone(),
        }),
    }
}
