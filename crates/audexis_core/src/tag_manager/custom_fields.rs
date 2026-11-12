use super::traits::Formats;
use super::utils::{FrameKey, SerializableTagValue};
use std::collections::HashSet;

pub fn supported(format: &Formats) -> bool {
    matches!(
        format,
        Formats::Id3v22
            | Formats::Id3v23
            | Formats::Id3v24
            | Formats::Flac
            | Formats::Ogg
            | Formats::Itunes
    )
}

pub fn validate(key: FrameKey, values: &[SerializableTagValue]) -> Result<(), String> {
    if !matches!(key, FrameKey::UserDefinedText | FrameKey::UserDefinedURL) {
        return Ok(());
    }
    for value in values {
        let (name, text) = match (key, value) {
            (FrameKey::UserDefinedText, SerializableTagValue::UserText(entry)) => {
                (&entry.description, &entry.value)
            }
            (FrameKey::UserDefinedURL, SerializableTagValue::UserUrl(entry)) => {
                if !entry.url.is_ascii() {
                    return Err("Use a percent-encoded URL for custom URL fields".into());
                }
                (&entry.description, &entry.url)
            }
            _ => return Err("Custom metadata values do not match their field type".into()),
        };
        if name.contains('=') {
            return Err("Metadata keys cannot contain an equals sign (=).".into());
        }
        if name.contains('\0') || text.contains('\0') {
            return Err("Custom fields cannot contain NUL characters".into());
        }
    }
    Ok(())
}

pub fn supports_urls(format: &Formats) -> bool {
    matches!(format, Formats::Id3v22 | Formats::Id3v23 | Formats::Id3v24)
}

pub fn validate_for_format(
    format: &Formats,
    key: FrameKey,
    values: &[SerializableTagValue],
) -> Result<(), String> {
    validate(key, values)?;
    if !matches!(key, FrameKey::UserDefinedText | FrameKey::UserDefinedURL) {
        return Ok(());
    }
    if !supported(format) {
        return Err("This metadata format does not support custom fields".into());
    }
    if key == FrameKey::UserDefinedURL && !values.is_empty() && !supports_urls(format) {
        return Err("Use a text field for URLs in this format".into());
    }
    let mut names = HashSet::new();
    for value in values {
        let name = match value {
            SerializableTagValue::UserText(entry) => &entry.description,
            SerializableTagValue::UserUrl(entry) => &entry.description,
            _ => continue,
        };
        if supports_urls(format) && !names.insert(name) {
            return Err("Custom ID3 fields of the same type must have unique names".into());
        }
        if matches!(format, Formats::Flac | Formats::Ogg) {
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|byte| (0x20..=0x7d).contains(&byte) && byte != b'=')
            {
                return Err("FLAC/Ogg field keys must be printable ASCII without '='".into());
            }
            let mapped = super::vorbis_comments::utils::raw_to_tags(
                &std::collections::HashMap::from([(name.clone(), Vec::new())]),
            );
            if !mapped.contains_key(&FrameKey::UserDefinedText) {
                return Err(format!(
                    "{name} is a standard metadata field; edit it in Details"
                ));
            }
        }
        if *format == Formats::Itunes {
            let full = super::itunes::utils::custom_key(name);
            let (mean, native_name) = full
                .strip_prefix("----:")
                .and_then(|key| key.split_once(':'))
                .ok_or("Use ----:namespace:name for a namespaced MP4 key")?;
            if mean.is_empty() || native_name.is_empty() {
                return Err("MP4 fields need a namespace and name".into());
            }
            if super::itunes::utils::FREEFORM_REVERSE_MAP.contains_key(&(mean, native_name)) {
                return Err(format!("{name} is a standard metadata field"));
            }
        }
    }
    Ok(())
}
