use super::{
    field_catalog::{is_text, FIELDS},
    traits::Formats,
    utils::{FrameKey, TagValue, UserTextEntry},
};
use std::collections::HashMap;

pub fn fallback_name(format: &Formats, key: FrameKey) -> Option<String> {
    if !is_text(key) {
        return None;
    }
    let native = match format {
        Formats::Id3v22 | Formats::Id3v23 | Formats::Id3v24 => {
            let (code, reverse) = match format {
                Formats::Id3v22 => (
                    super::id3::utils::id3v22_code(key),
                    &*super::id3::utils::ID3V22_REVERSE_MAP,
                ),
                Formats::Id3v23 => (
                    super::id3::utils::id3v23_code(key),
                    &*super::id3::utils::ID3V23_REVERSE_MAP,
                ),
                _ => (
                    super::id3::utils::id3v24_code(key),
                    &*super::id3::utils::ID3V24_REVERSE_MAP,
                ),
            };
            reverse.get(code) == Some(&key)
                && (code.starts_with('T')
                    || code.starts_with('W')
                    || matches!(
                        key,
                        FrameKey::Comments
                            | FrameKey::UnsyncedLyrics
                            | FrameKey::SynchronizedLyrics
                    ))
        }
        Formats::Flac | Formats::Ogg => {
            !super::vorbis_comments::utils::vorbis_code(key).is_empty()
                && key != FrameKey::RecordingDate
        }
        Formats::Itunes => {
            let code = super::itunes::utils::itunes_code(key);
            super::itunes::utils::itunes_freeform_spec(key).is_some()
                || (code != "----"
                    && super::itunes::utils::ITUNES_REVERSE_MAP.get(code) == Some(&key))
        }
        _ => return None,
    };
    (!native).then(|| key.to_string().to_ascii_uppercase())
}

pub fn expose_regular_fields(format: &Formats, tags: &mut HashMap<FrameKey, Vec<TagValue>>) {
    let custom = tags
        .get(&FrameKey::UserDefinedText)
        .cloned()
        .unwrap_or_default();
    for &(key, _) in FIELDS {
        let Some(name) = fallback_name(format, key) else {
            continue;
        };
        let values = custom
            .iter()
            .filter_map(|value| match value {
                TagValue::UserText(entry) if entry.description == name => {
                    Some(TagValue::Text(entry.value.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if !values.is_empty() {
            tags.entry(key).or_insert(values);
        }
    }
}

pub fn prepare_changes(
    format: &Formats,
    changes: &HashMap<FrameKey, Vec<TagValue>>,
    existing: &HashMap<FrameKey, Vec<TagValue>>,
) -> Result<HashMap<FrameKey, Vec<TagValue>>, String> {
    let mut native = changes.clone();
    for (&key, values) in changes {
        let Some(name) = fallback_name(format, key) else {
            continue;
        };
        if values
            .iter()
            .any(|value| !matches!(value, TagValue::Text(_)))
        {
            return Err(format!("{} needs text values", key));
        }
        native.remove(&key);
        let custom = native.entry(FrameKey::UserDefinedText).or_insert_with(|| {
            existing
                .get(&FrameKey::UserDefinedText)
                .cloned()
                .unwrap_or_default()
        });
        custom.retain(
            |value| !matches!(value, TagValue::UserText(entry) if entry.description == name),
        );
        custom.extend(values.iter().filter_map(|value| match value {
            TagValue::Text(value) => Some(TagValue::UserText(UserTextEntry {
                description: name.clone(),
                value: value.clone(),
            })),
            _ => None,
        }));
    }
    Ok(native)
}
