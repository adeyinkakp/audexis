use super::get_files::FilesResponse;
use crate::database::search::prefix_terms;
use crate::{
    database::types::{DatabaseMediaFile, DatabaseMediaMetadata},
    AppState,
};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

#[derive(Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchInput {
    query: String,
    favorites_only: bool,
    exact_album: Option<String>,
    exact_artist: Option<String>,
    title: String,
    artist: String,
    album: String,
    file_name: String,
    genre: String,
    format: String,
    folder: String,
    min_duration_ms: Option<i64>,
    max_duration_ms: Option<i64>,
    offset: i64,
}

fn escape_like(value: &str) -> String {
    value
        .trim()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn pattern(value: &str) -> String {
    format!("%{}%", escape_like(value))
}

fn contains(query: &mut QueryBuilder<'_, Sqlite>, expression: &str, value: &str) {
    query
        .push(expression)
        .push(" LIKE ")
        .push_bind(pattern(value))
        .push(" ESCAPE '\\'");
}
#[tauri::command]
pub async fn search_media(
    state: tauri::State<'_, AppState>,
    input: SearchInput,
) -> Result<FilesResponse, String> {
    search_media_with_pool(&state.db.pool, input).await
}

async fn search_media_with_pool(
    pool: &SqlitePool,
    input: SearchInput,
) -> Result<FilesResponse, String> {
    let terms: Vec<_> = input.query.split_whitespace().collect();
    if terms.len() > 12
        || [
            &input.query,
            &input.title,
            &input.artist,
            &input.album,
            &input.file_name,
            &input.genre,
            &input.format,
            &input.folder,
        ]
        .iter()
        .any(|value| value.len() > 500)
    {
        return Err("Use up to 12 search words and 500 characters per field".into());
    }
    if input.offset < 0
        || input.min_duration_ms.is_some_and(|value| value < 0)
        || input.max_duration_ms.is_some_and(|value| value < 0)
        || matches!((input.min_duration_ms, input.max_duration_ms), (Some(min), Some(max)) if min > max)
    {
        return Err("Check the duration range".into());
    }
    if input.exact_album.as_ref().is_some_and(|v| v.len() > 500)
        || input.exact_artist.as_ref().is_some_and(|v| v.len() > 500)
    {
        return Err("Collection name is too long".into());
    }
    let mut matches = Vec::new();
    if !terms.is_empty() {
        matches.push(format!(
            "{{title album_artist artist album lyrics}} : ({})",
            prefix_terms(&input.query)
        ));
    }
    for (column, value) in [
        ("title", &input.title),
        ("artist", &input.artist),
        ("album", &input.album),
        ("genre", &input.genre),
        ("file_name", &input.file_name),
    ] {
        if !value.trim().is_empty() {
            matches.push(format!("{column} : ({})", prefix_terms(value)));
        }
    }
    let has_match = !matches.is_empty();
    if has_match {
        crate::database::search::refresh_pending(pool)
            .await
            .map_err(|error| error.to_string())?;
    }
    let mut query = QueryBuilder::<Sqlite>::new("SELECT f.* FROM files f");
    if has_match {
        query.push(" JOIN media_search ON media_search.rowid = f.id");
    }
    query.push(" WHERE f.missing_since IS NULL");
    if has_match {
        query
            .push(" AND media_search MATCH ")
            .push_bind(matches.join(" AND "));
    }
    if input.favorites_only {
        query.push(
            " AND EXISTS (SELECT 1 FROM media_info i WHERE i.file_id = f.id AND i.loved = 1)",
        );
    }
    if let Some(album) = &input.exact_album {
        if album.is_empty() {
            query.push(" AND NOT EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key='album' AND TRIM(m.value) <> '')");
        } else {
            query.push(" AND EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key='album' AND m.value = ").push_bind(album.clone()).push(" COLLATE METADATA_NOCASE)");
        }
    }
    if let Some(artist) = &input.exact_artist {
        if artist.is_empty() {
            query.push(" AND NOT EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key IN ('artist','albumArtist') AND TRIM(m.value) <> '')");
        } else {
            query.push(" AND EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id=f.id AND m.key IN ('artist','albumArtist') AND m.value = ").push_bind(artist.clone()).push(" COLLATE METADATA_NOCASE)");
        }
    }
    for (expression, value) in [("f.format", &input.format), ("f.path", &input.folder)] {
        if !value.trim().is_empty() {
            query.push(" AND ");
            contains(&mut query, expression, value);
        }
    }
    if let Some(min) = input.min_duration_ms {
        query.push(" AND f.duration_ms >= ").push_bind(min);
    }
    if let Some(max) = input.max_duration_ms {
        query.push(" AND f.duration_ms <= ").push_bind(max);
    }
    query.push(" ORDER BY ");
    if has_match {
        query.push("bm25(media_search, 10.0, 8.0, 8.0, 6.0, 1.0, 2.0, 2.0), ");
    }
    query
        .push("f.file_name COLLATE NOCASE, f.id LIMIT 51 OFFSET ")
        .push_bind(input.offset);
    let mut tx = pool.begin().await.map_err(|error| error.to_string())?;
    let mut files = query
        .build_query_as::<DatabaseMediaFile>()
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    let next_cursor = (files.len() > 50).then_some(input.offset + 50);
    files.truncate(50);
    let mut metadata = Vec::new();
    if !files.is_empty() {
        let mut tags = QueryBuilder::<Sqlite>::new(
            "SELECT file_id, key, value, ord FROM metadata_texts WHERE file_id IN (",
        );
        let mut separated = tags.separated(",");
        for file in &files {
            separated.push_bind(file.id);
        }
        tags.push(") ORDER BY file_id, ord");
        metadata = tags
            .build_query_as::<DatabaseMediaMetadata>()
            .fetch_all(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
    }
    tx.commit().await.map_err(|error| error.to_string())?;
    Ok(FilesResponse {
        files,
        metadata,
        next_cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::{search_media_with_pool, FilesResponse, SearchInput};
    use crate::database::Database;
    use sqlx::SqlitePool;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    const LIBRARY_SIZE: i64 = 100_000;
    const MATCH_EVERY: i64 = 4;
    const EXPECTED_MATCH_COUNT: i64 = LIBRARY_SIZE / MATCH_EVERY;
    const SEARCH_TIME_BUDGET: Duration = Duration::from_secs(2);
    const SEARCH_QUERY: &str = "kp here";
    const BENCHMARK_RUNS: usize = 9;

    struct TestDirectory(PathBuf);

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    async fn seed_fake_library(pool: &SqlitePool) {
        let mut transaction = pool.begin().await.expect("test transaction should begin");

        for index in 0..LIBRARY_SIZE {
            let result = sqlx::query(
                "INSERT INTO files \
                (path, file_name, last_validated, status, duration_ms, format, modified_at, size) \
                VALUES (?1, ?2, 1, 'ok', ?3, 'mp3', ?4, ?5)",
            )
            .bind(format!("/fake/library/track-{index:05}.mp3"))
            .bind(format!("track-{index:05}.mp3"))
            .bind(180_000_i64 + index)
            .bind(index)
            .bind(4_000_000_i64 + index)
            .execute(&mut *transaction)
            .await
            .expect("fake file should be inserted");

            let title = if index % MATCH_EVERY == 0 {
                format!("Kp was here {index:05}")
            } else {
                format!("Library Track {index:05}")
            };
            sqlx::query(
                "INSERT INTO metadata_texts (file_id, key, value, ord) \
                VALUES (?1, 'title', ?2, 0)",
            )
            .bind(result.last_insert_rowid())
            .bind(title)
            .execute(&mut *transaction)
            .await
            .expect("fake title should be inserted");
        }

        transaction
            .commit()
            .await
            .expect("test transaction should commit");
    }

    fn median(timings: &mut [Duration]) -> Duration {
        timings.sort_unstable();
        timings[timings.len() / 2]
    }

    fn search_input() -> SearchInput {
        SearchInput {
            query: SEARCH_QUERY.to_string(),
            ..SearchInput::default()
        }
    }

    fn assert_search_response(response: &FilesResponse) {
        assert_eq!(response.files.len(), 50);
        assert_eq!(response.metadata.len(), 50);
        assert_eq!(response.next_cursor, Some(50));
        assert!(response
            .metadata
            .iter()
            .all(|metadata| metadata.value.contains("Kp was here")));
    }

    async fn benchmark_full_search(pool: &SqlitePool, runs: usize) -> Vec<Duration> {
        let mut timings = Vec::with_capacity(runs);
        for _ in 0..runs {
            let started = Instant::now();
            let response = search_media_with_pool(pool, search_input())
                .await
                .expect("search should succeed");
            let elapsed = started.elapsed();

            assert_search_response(&response);
            assert!(
                elapsed < SEARCH_TIME_BUDGET,
                "{LIBRARY_SIZE}-file search took {elapsed:?}, exceeding \
                 {SEARCH_TIME_BUDGET:?} for query {SEARCH_QUERY:?}"
            );
            timings.push(elapsed);
        }
        timings
    }

    #[test]
    fn profile_search_100k_files() {
        const FILTER: &str = r#"
           FROM files f
           WHERE 1=1
             AND (f.file_name LIKE ?1 ESCAPE '\' OR f.path LIKE ?1 ESCAPE '\' OR f.format LIKE ?1 ESCAPE '\'
                  OR EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id = f.id AND m.value LIKE ?1 ESCAPE '\'))
             AND (f.file_name LIKE ?2 ESCAPE '\' OR f.path LIKE ?2 ESCAPE '\' OR f.format LIKE ?2 ESCAPE '\'
                  OR EXISTS (SELECT 1 FROM metadata_texts m WHERE m.file_id = f.id AND m.value LIKE ?2 ESCAPE '\'))
       "#;
        tauri::async_runtime::block_on(async {
            let test_directory = TestDirectory(
                std::env::temp_dir().join(format!("audexis-profile-test-{}", uuid::Uuid::new_v4())),
            );
            let database = Database::init(&test_directory.0.join("library.db"))
                .await
                .expect("test database should initialize");
            seed_fake_library(&database.pool).await;
            let pool = &database.pool;

            let (p1, p2) = ("%kp%", "%here%");
            let filter_count_sql = format!("SELECT COUNT(*) {FILTER}");
            let filter_sort_sql =
                format!("SELECT f.* {FILTER} ORDER BY f.file_name COLLATE NOCASE, f.id LIMIT 51");

            let started = Instant::now();
            let first = search_media_with_pool(pool, search_input())
                .await
                .expect("initial indexing and search should succeed");
            eprintln!("Initial indexing + search: {:?}", started.elapsed());
            let second = search_media_with_pool(
                pool,
                SearchInput {
                    offset: 50,
                    ..search_input()
                },
            )
            .await
            .expect("second page should succeed");
            assert_eq!(second.files.len(), 50);
            assert_eq!(second.next_cursor, Some(100));
            assert!(second
                .files
                .iter()
                .all(|file| !first.files.iter().any(|previous| previous.id == file.id)));

            let mut filter_only = Vec::with_capacity(BENCHMARK_RUNS);
            for _ in 0..BENCHMARK_RUNS {
                let started = Instant::now();
                let count: i64 = sqlx::query_scalar(&filter_count_sql)
                    .bind(p1)
                    .bind(p2)
                    .fetch_one(pool)
                    .await
                    .expect("filter-only query should succeed");
                filter_only.push(started.elapsed());
                assert_eq!(count, EXPECTED_MATCH_COUNT);
            }

            let mut filter_sort = Vec::with_capacity(BENCHMARK_RUNS);
            for _ in 0..BENCHMARK_RUNS {
                let started = Instant::now();
                let rows = sqlx::query(&filter_sort_sql)
                    .bind(p1)
                    .bind(p2)
                    .fetch_all(pool)
                    .await
                    .expect("filter+sort query should succeed");
                filter_sort.push(started.elapsed());
                assert_eq!(rows.len(), 51);
            }

            let mut full = benchmark_full_search(pool, BENCHMARK_RUNS).await;

            let a = median(&mut filter_only);
            let b = median(&mut filter_sort);
            let c = median(&mut full);
            eprintln!(" search profile ({LIBRARY_SIZE} files, query {SEARCH_QUERY:?}, median of {BENCHMARK_RUNS}) ");
            eprintln!("A. filter only (COUNT):          {a:?}");
            eprintln!("B. filter + name sort, LIMIT 51: {b:?}");
            eprintln!(
                "C. full search (ranking + tags): {c:?}   (additional full-search work ~{:?})",
                c.saturating_sub(b)
            );

            database.pool.close().await;
        });
    }
}
