use super::*;

#[derive(Clone)]
pub(super) struct Field {
    pub key: FrameKey,
    pub title: &'static str,
    pub value: String,
}

pub(super) fn fields(kind: Tab, item: &Value) -> Vec<Field> {
    let sources: &[(FrameKey, &str, &str)] = match kind {
        Tab::Details => &[
            (FrameKey::Title, "Title", "trackName"),
            (FrameKey::Artist, "Artist", "artistName"),
            (FrameKey::Album, "Album", "collectionName"),
            (FrameKey::Genre, "Genre", "primaryGenreName"),
            (FrameKey::Year, "Year", "releaseDate"),
            (FrameKey::TrackNumber, "Track number", "trackNumber"),
            (FrameKey::DiscNumber, "Disc number", "discNumber"),
            (FrameKey::AttachedPicture, "Cover artwork", "artworkUrl100"),
        ],
        Tab::Synced => &[(
            FrameKey::SynchronizedLyrics,
            "Synced lyrics",
            "syncedLyrics",
        )],
        _ => &[(FrameKey::UnsyncedLyrics, "Plain lyrics", "plainLyrics")],
    };
    sources
        .iter()
        .filter_map(|(key, title, source)| {
            let value = &item[*source];
            let mut value = value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_u64().map(|number| number.to_string()))?;
            if *key == FrameKey::Year {
                value = value.chars().take(4).collect();
            }
            (!value.trim().is_empty()).then_some(Field {
                key: *key,
                title,
                value,
            })
        })
        .collect()
}

pub(super) fn title(item: &Value) -> String {
    item["trackName"]
        .as_str()
        .unwrap_or("Unknown track")
        .to_owned()
}

pub(super) fn subtitle(kind: Tab, item: &Value) -> String {
    [
        "artistName",
        if kind == Tab::Details {
            "collectionName"
        } else {
            "albumName"
        },
    ]
    .into_iter()
    .filter_map(|key| item[key].as_str())
    .filter(|text| !text.trim().is_empty())
    .collect::<Vec<_>>()
    .join(" · ")
}

pub(super) fn availability(kind: Tab, item: &Value) -> String {
    let duration = if kind == Tab::Details {
        item["trackTimeMillis"].as_u64().map(|ms| ms / 1000)
    } else {
        item["duration"]
            .as_f64()
            .filter(|seconds| *seconds >= 0.)
            .map(|seconds| seconds as u64)
    };
    let mut parts = duration
        .map(crate::ui::format_time)
        .into_iter()
        .collect::<Vec<_>>();
    if kind != Tab::Details {
        for (key, label) in [
            ("plainLyrics", "Plain lyrics"),
            ("syncedLyrics", "Synced lyrics"),
        ] {
            if item[key]
                .as_str()
                .is_some_and(|text| !text.trim().is_empty())
            {
                parts.push(label.to_owned());
            }
        }
        if item["instrumental"].as_bool() == Some(true) {
            parts.push("Instrumental".into());
        }
    }
    parts.join(" · ")
}

pub(super) async fn prepare(fields: Vec<Field>) -> Result<Vec<(FrameKey, TagValue)>, String> {
    let mut changes = Vec::new();
    for field in fields {
        let value = match field.key {
            FrameKey::AttachedPicture => api::artwork(field.value).await
                .map_err(|error| format!("Could not download cover artwork: {error}. Uncheck it to apply the other fields."))?,
            FrameKey::SynchronizedLyrics => {
                metadata_editor::parse_lrc(&field.value)?;
                TagValue::Text(field.value)
            }
            _ => TagValue::Text(field.value),
        };
        changes.push((field.key, value));
    }
    Ok(changes)
}
