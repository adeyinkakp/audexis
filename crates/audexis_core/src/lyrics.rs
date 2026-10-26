use crate::{
    database::Database,
    tag_manager::{
        tag_backend::{DefaultBackend, TagBackend},
        utils::FrameKey,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct LyricLine {
    pub time_ms: u64,
    pub text: String,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    pub plain: String,
}
impl Lyrics {
    pub fn active_index(&self, position: u64) -> Option<usize> {
        self.lines
            .partition_point(|line| line.time_ms <= position)
            .checked_sub(1)
    }
}
fn timestamp(value: &str) -> Option<u64> {
    let (minutes, seconds) = value.split_once(':')?;
    let (seconds, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
    if minutes.is_empty()
        || seconds.len() != 2
        || fraction.len() > 3
        || !minutes
            .bytes()
            .chain(seconds.bytes())
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let seconds: u64 = seconds.parse().ok()?;
    if seconds >= 60 {
        return None;
    }
    let millis = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().ok()? * 10u64.pow(3 - fraction.len() as u32)
    };
    minutes
        .parse::<u64>()
        .ok()?
        .checked_mul(60_000)?
        .checked_add(seconds * 1000 + millis)
}
pub fn parse_lrc(text: &str) -> Vec<LyricLine> {
    let offset = text
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("[offset:")?
                .strip_suffix(']')?
                .parse::<i64>()
                .ok()
        })
        .unwrap_or(0);
    let mut rows = Vec::new();
    for line in text.lines() {
        let mut rest = line.trim_start_matches('\u{feff}').trim_start();
        let mut times = Vec::new();
        while let Some((stamp, tail)) = rest.strip_prefix('[').and_then(|s| s.split_once(']')) {
            let Some(time) = timestamp(stamp) else {
                break;
            };
            times.push(time);
            rest = tail;
        }
        for time in times {
            let time_ms = if offset >= 0 {
                time.saturating_add(offset as u64)
            } else {
                time.saturating_sub(offset.unsigned_abs())
            };
            rows.push(LyricLine {
                time_ms,
                text: rest.to_owned(),
            });
        }
    }
    rows.sort_by_key(|line| line.time_ms);
    let mut grouped: Vec<LyricLine> = Vec::new();
    for row in rows {
        if let Some(last) = grouped
            .last_mut()
            .filter(|last| last.time_ms == row.time_ms)
        {
            last.text.push('\n');
            last.text.push_str(&row.text);
        } else {
            grouped.push(row);
        }
    }
    grouped
}
fn from_values(values: Vec<String>) -> Lyrics {
    let mut lyrics = Lyrics::default();
    for value in values {
        if value.trim().is_empty() {
            continue;
        }
        let rows = parse_lrc(&value);
        if !rows.is_empty() && lyrics.lines.is_empty() {
            lyrics.lines = rows;
        } else if lyrics.plain.is_empty() {
            lyrics.plain = value;
        }
    }
    lyrics
}
impl Database {
    pub async fn lyrics(&self, file_id: i64, path: String) -> Result<Lyrics, String> {
        if file_id > 0 {
            let values: Vec<String> = sqlx::query_scalar("SELECT value FROM metadata_texts WHERE file_id=? AND key IN ('synchronizedLyrics','lyrics','unsyncedLyrics') ORDER BY CASE key WHEN 'synchronizedLyrics' THEN 0 WHEN 'lyrics' THEN 1 ELSE 2 END,ord")
                .bind(file_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
            let lyrics = from_values(values);
            if !lyrics.lines.is_empty() || !lyrics.plain.is_empty() {
                return Ok(lyrics);
            }
        }
        if path.is_empty() || crate::library_files::is_virtual_path(std::path::Path::new(&path)) {
            return Ok(Lyrics::default());
        }
        tokio::task::spawn_blocking(move || {
            let metadata = DefaultBackend::new()
                .read(std::path::Path::new(&path))
                .map_err(|e| format!("Could not read embedded lyrics: {e:?}"))?;
            let values = [
                FrameKey::SynchronizedLyrics,
                FrameKey::Lyrics,
                FrameKey::UnsyncedLyrics,
            ]
            .into_iter()
            .flat_map(|key| {
                metadata
                    .tags
                    .get(&key)
                    .into_iter()
                    .flatten()
                    .map(ToString::to_string)
            })
            .collect();
            Ok(from_values(values))
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn loads_stored_and_unindexed_embedded_synced_lyrics() {
        use crate::tag_manager::utils::{Changes, SerializableTagValue, TagChange};
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE metadata_texts(file_id INTEGER,key TEXT,value TEXT,ord INTEGER)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO metadata_texts VALUES (1,'synchronizedLyrics','[00:01]Stored line',0),(1,'lyrics','Plain fallback',0)").execute(&pool).await.unwrap();
        let db = Database { pool };
        let stored = db.lyrics(1, "/not-needed.mp3".into()).await.unwrap();
        assert_eq!(stored.lines[0].time_ms, 1000);
        assert_eq!(stored.plain, "Plain fallback");
        let path =
            std::env::temp_dir().join(format!("audexis-synced-{}.mp3", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"ID3\x03\0\0\0\0\0\0").unwrap();
        let fixture = path.clone();
        tokio::task::spawn_blocking(move || {
            let errors = DefaultBackend::new().write_changes(&Changes {
                paths: vec![fixture.to_string_lossy().into_owned()],
                tags: std::collections::HashMap::from([(
                    FrameKey::SynchronizedLyrics,
                    TagChange::Replace(vec![SerializableTagValue::Text(
                        "[00:01.500]Embedded line".into(),
                    )]),
                )]),
            });
            assert!(errors.is_empty(), "{errors:?}");
        })
        .await
        .unwrap();
        let embedded = db
            .lyrics(0, path.to_string_lossy().into_owned())
            .await
            .unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            embedded.lines,
            vec![LyricLine {
                time_ms: 1500,
                text: "Embedded line".into()
            }]
        );
    }
    #[test]
    fn timing_offsets_repeated_timestamps_and_backward_seeking() {
        let lines = parse_lrc(
            "[ar:Artist]\n[offset:-100]\n[00:02.50][00:04]Two\n[00:01.100]One\n[00:02.500]Translation\n[00:99]Bad",
        );
        assert_eq!(
            lines.iter().map(|l| l.time_ms).collect::<Vec<_>>(),
            vec![1000, 2400, 3900]
        );
        assert_eq!(lines[1].text, "Two\nTranslation");
        let lyrics = Lyrics {
            lines,
            plain: String::new(),
        };
        assert_eq!(lyrics.active_index(999), None);
        assert_eq!(lyrics.active_index(2400), Some(1));
        assert_eq!(lyrics.active_index(1000), Some(0));
        assert_eq!(lyrics.active_index(99999), Some(2));
    }
    #[test]
    fn plain_and_synced_candidates_coexist() {
        let lyrics = from_values(vec!["[00:01]Hello".into(), "Hello world".into()]);
        assert_eq!(lyrics.lines.len(), 1);
        assert_eq!(lyrics.plain, "Hello world");
        assert!(from_values(vec!["Plain lyrics".into()]).lines.is_empty());
    }
}
