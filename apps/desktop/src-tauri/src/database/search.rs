use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::collections::BTreeMap;

pub(crate) fn prefix_terms(value: &str) -> String {
    value
        .split_whitespace()
        .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

pub(crate) async fn refresh_pending(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    loop {
        let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media_search_pending)")
            .fetch_one(pool)
            .await?;
        if !pending {
            return Ok(());
        }
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        let ids: Vec<i64> = sqlx::query_scalar(
            "SELECT file_id FROM media_search_pending ORDER BY file_id LIMIT 200",
        )
        .fetch_all(&mut *tx)
        .await?;
        if ids.is_empty() {
            tx.commit().await?;
            return Ok(());
        }
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT f.id, f.file_name, m.key, m.value FROM files f \
             LEFT JOIN metadata_texts m ON m.file_id = f.id AND m.key IN \
             ('title','albumArtist','artist','album','lyrics','unsyncedLyrics','synchronizedLyrics','genre') \
             WHERE f.id IN ("
        );
        let mut separated = query.separated(",");
        for id in &ids {
            separated.push_bind(id);
        }
        query.push(") ORDER BY f.id, m.key, m.ord");
        let rows: Vec<(i64, String, Option<String>, Option<String>)> =
            query.build_query_as().fetch_all(&mut *tx).await?;
        let mut documents: BTreeMap<i64, [String; 7]> = BTreeMap::new();
        for (id, name, key, value) in rows {
            let columns = documents.entry(id).or_insert_with(|| {
                let mut columns = std::array::from_fn(|_| String::new());
                columns[6] = name;
                columns
            });
            if let (Some(key), Some(value)) = (key, value) {
                let column = match key.as_str() {
                    "title" => 0,
                    "albumArtist" => 1,
                    "artist" => 2,
                    "album" => 3,
                    "lyrics" | "unsyncedLyrics" | "synchronizedLyrics" => 4,
                    "genre" => 5,
                    _ => continue,
                };
                if !columns[column].is_empty() {
                    columns[column].push('\n');
                }
                if column == 4 {
                    columns[column].push_str(&plain_lyrics(&value));
                } else {
                    columns[column].push_str(&value);
                }
            }
        }
        let mut delete = QueryBuilder::<Sqlite>::new("DELETE FROM media_search WHERE rowid IN (");
        let mut separated = delete.separated(",");
        for id in &ids {
            separated.push_bind(id);
        }
        delete.push(")").build().execute(&mut *tx).await?;
        if !documents.is_empty() {
            let mut insert = QueryBuilder::<Sqlite>::new(
                "INSERT INTO media_search (rowid,title,album_artist,artist,album,lyrics,genre,file_name) "
            );
            insert.push_values(&documents, |mut row, (id, columns)| {
                row.push_bind(id);
                for column in columns {
                    row.push_bind(column);
                }
            });
            insert.build().execute(&mut *tx).await?;
        }
        let mut delete_collections = QueryBuilder::<Sqlite>::new(
            "DELETE FROM collection_search_documents WHERE file_id IN (",
        );
        let mut separated = delete_collections.separated(",");
        for id in &ids {
            separated.push_bind(id);
        }
        delete_collections
            .push(")")
            .build()
            .execute(&mut *tx)
            .await?;

        let mut collections =
            QueryBuilder::<Sqlite>::new("WITH selected AS (SELECT id FROM files WHERE id IN (");
        let mut separated = collections.separated(",");
        for id in &ids {
            separated.push_bind(id);
        }
        collections.push(")) INSERT INTO collection_search_documents(file_id,kind,name,artist) \
            SELECT f.id, 'album', a.value, COALESCE((SELECT m.value FROM metadata_texts m \
                WHERE m.file_id=f.id AND m.key IN ('albumArtist','artist') AND TRIM(m.value) <> '' \
                ORDER BY CASE m.key WHEN 'albumArtist' THEN 0 ELSE 1 END, m.ord LIMIT 1), '') \
            FROM selected f JOIN metadata_texts a ON a.file_id=f.id AND a.key='album' WHERE TRIM(a.value) <> '' \
            UNION ALL SELECT f.id, 'artist', m.value, '' FROM selected f JOIN metadata_texts m ON m.file_id=f.id \
                WHERE m.key IN ('artist','albumArtist') AND TRIM(m.value) <> '' \
            UNION ALL SELECT f.id, 'album', '', '' FROM selected f WHERE NOT EXISTS \
                (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key='album' AND TRIM(m.value) <> '') \
            UNION ALL SELECT f.id, 'artist', '', '' FROM selected f WHERE NOT EXISTS \
                (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key IN ('artist','albumArtist') AND TRIM(m.value) <> '')");
        collections.build().execute(&mut *tx).await?;
        let mut clear =
            QueryBuilder::<Sqlite>::new("DELETE FROM media_search_pending WHERE file_id IN (");
        let mut separated = clear.separated(",");
        for id in &ids {
            separated.push_bind(id);
        }
        clear.push(")").build().execute(&mut *tx).await?;
        tx.commit().await?;
    }
}

fn timestamp(value: &str) -> bool {
    let Some((minutes, rest)) = value.split_once(':') else {
        return false;
    };
    let (seconds, fraction) = rest.split_once('.').unwrap_or((rest, ""));
    !minutes.is_empty()
        && seconds.len() == 2
        && fraction.len() <= 3
        && minutes
            .bytes()
            .chain(seconds.bytes())
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
        && seconds.parse::<u8>().is_ok_and(|s| s < 60)
}

fn plain_lyrics(value: &str) -> String {
    value
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(tag) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                if let Some((key, _)) = tag.split_once(':') {
                    if ["ar", "al", "ti", "au", "by", "offset", "re", "ve", "length"]
                        .contains(&key.to_ascii_lowercase().as_str())
                    {
                        return None;
                    }
                }
            }
            let mut output = String::new();
            let mut rest = line;
            while let Some(index) = rest.find(['[', '<']) {
                output.push_str(&rest[..index]);
                rest = &rest[index..];
                let close = if rest.starts_with('[') { ']' } else { '>' };
                if let Some(end) = rest.find(close) {
                    if timestamp(&rest[1..end]) {
                        output.push(' ');
                        rest = &rest[end + 1..];
                        continue;
                    }
                }
                output.push(rest.chars().next().unwrap());
                rest = &rest[1..];
            }
            output.push_str(rest);
            Some(output)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
