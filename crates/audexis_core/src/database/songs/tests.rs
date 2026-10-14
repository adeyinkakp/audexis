use super::*;
#[tokio::test]
async fn blanks_sort_last_in_both_directions_for_table_and_queue() {
    use crate::config::CustomFieldKind;
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("src/database/migrations")
        .run(&pool)
        .await
        .unwrap();
    let db = Database { pool };
    for (id, name, value) in [
        (1, "", ""),
        (2, "Zulu", "Zulu"),
        (3, "Alpha", "Alpha"),
        (4, " \t\n ", " \t\n "),
    ] {
        sqlx::query("INSERT INTO files(id,path,file_name,status,size) VALUES(?,?,?,'ok',100)")
            .bind(id)
            .bind(format!("/{id}.flac"))
            .bind(name)
            .execute(&db.pool)
            .await
            .unwrap();
        for key in ["album", "genre", "custom:text:SOURCE", "trackNumber"] {
            let value = if key == "trackNumber" && id == 2 {
                "10"
            } else if key == "trackNumber" && id == 3 {
                "2"
            } else {
                value
            };
            sqlx::query("INSERT INTO metadata_texts(file_id,key,value) VALUES(?,?,?)")
                .bind(id)
                .bind(key)
                .bind(value)
                .execute(&db.pool)
                .await
                .unwrap();
        }
    }
    sqlx::query("INSERT INTO files(id,path,file_name,status,size) VALUES(5,'/5.flac','','ok',100)")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO metadata_texts(file_id,key,value,ord) VALUES(1,'genre',' ',1)")
        .execute(&db.pool)
        .await
        .unwrap();
    for sort in [
        SongSort::Title,
        SongSort::Album,
        SongSort::Metadata("genre".into()),
        SongSort::Metadata("trackNumber".into()),
        SongSort::Custom {
            key: "SOURCE".into(),
            kind: CustomFieldKind::Text,
        },
    ] {
        for descending in [false, true] {
            let expected = if descending {
                vec![2, 3, 1, 4, 5]
            } else {
                vec![3, 2, 1, 4, 5]
            };
            let page = db
                .songs_page("", sort.clone(), descending, 0)
                .await
                .unwrap();
            assert_eq!(
                page.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
                expected,
                "{sort:?}, descending={descending}"
            );
            let queue = db
                .songs_queue(
                    &SongQuery {
                        search: String::new(),
                        sort: sort.clone(),
                        descending,
                    },
                    1,
                )
                .await
                .unwrap();
            assert_eq!(
                queue.tracks.iter().map(|r| r.id).collect::<Vec<_>>(),
                expected
            );
            assert_eq!(queue.index, 2);
        }
    }
}
#[tokio::test]
async fn metadata_and_custom_columns_sort_like_the_complete_playback_queue() {
    use crate::config::CustomFieldKind;
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("src/database/migrations")
        .run(&pool)
        .await
        .unwrap();
    let db = Database { pool };
    let key = "Source's : key";
    let custom = SongSort::Custom {
        key: key.into(),
        kind: CustomFieldKind::Url,
    };
    let genre = SongSort::Metadata("genre".into());
    let track = SongSort::Metadata("trackNumber".into());
    let columns = vec![custom.clone(), genre.clone(), track.clone()];
    for (id, format, url, number) in [
        (1, "FLAC", "Zulu", "10"),
        (2, "id3v2.3", "alpha", "2"),
        (3, "Itunes", "Mike", "3"),
    ] {
        sqlx::query(
            "INSERT INTO files(id,path,file_name,status,size,format) VALUES(?,?,?,'ok',100,?)",
        )
        .bind(id)
        .bind(format!("/{id}.audio"))
        .bind(format!("Song {id}"))
        .bind(format)
        .execute(&db.pool)
        .await
        .unwrap();
        for (key, value, ord) in [
            (custom.storage_key(format).unwrap(), url, 0),
            ("genre".into(), "Rock", 0),
            ("genre".into(), "Pop", 1),
            ("trackNumber".into(), number, 0),
        ] {
            sqlx::query("INSERT INTO metadata_texts(file_id,key,value,ord) VALUES(?,?,?,?)")
                .bind(id)
                .bind(key)
                .bind(value)
                .bind(ord)
                .execute(&db.pool)
                .await
                .unwrap();
        }
    }
    let first = db
        .songs_page_columns("", custom.clone(), false, 0, &columns)
        .await
        .unwrap();
    assert_eq!(
        first.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![2, 3, 1]
    );
    assert_eq!(first.rows[0].column_text(&custom), "alpha");
    assert_eq!(first.rows[0].column_text(&genre), "Rock; Pop");
    assert_eq!(first.rows[0].metadata.len(), 3);
    let queue = db
        .songs_queue(
            &SongQuery {
                search: "song".into(),
                sort: custom.clone(),
                descending: false,
            },
            3,
        )
        .await
        .unwrap();
    assert_eq!(queue.index, 1);
    assert_eq!(
        queue.tracks.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![2, 3, 1]
    );
    let reversed = db
        .songs_page_columns("song", custom, true, 1, &columns)
        .await
        .unwrap();
    assert_eq!(
        reversed.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![3, 2]
    );
    let numeric = db.songs_page("", track, false, 0).await.unwrap();
    assert_eq!(
        numeric.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![2, 3, 1]
    );
    assert!(numeric.rows.iter().all(|r| r.metadata.is_empty()));
}
#[tokio::test]
async fn paging_search_and_sort_cover_the_entire_available_library() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("src/database/migrations")
        .run(&pool)
        .await
        .unwrap();
    let db = Database { pool };
    for id in 1..=102 {
        sqlx::query(
            "INSERT INTO files(id,path,file_name,status,duration_ms,size) VALUES (?,?,?,'ok',?,100)",
        )
        .bind(id)
        .bind(format!("/music/{id}.flac"))
        .bind(format!("Song {id:03}"))
        .bind(id * 1000)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE files SET missing_since=1 WHERE id=102")
        .execute(&db.pool)
        .await
        .unwrap();
    let first = db.songs_page("", SongSort::Title, false, 0).await.unwrap();
    let next = db
        .songs_page("", SongSort::Title, false, 100)
        .await
        .unwrap();
    assert_eq!(first.total, 101);
    assert_eq!(first.rows.len(), 100);
    assert_eq!(
        next.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![101]
    );
    let query = SongQuery {
        search: String::new(),
        sort: SongSort::Title,
        descending: false,
    };
    let queue = db.songs_queue(&query, 5).await.unwrap();
    assert_eq!(queue.index, 4);
    assert_eq!(queue.tracks.len(), 101);
    assert_eq!(queue.tracks[3].id, 4);
    assert_eq!(queue.tracks[100].id, 101);
    let filtered = SongQuery {
        search: "song 0".into(),
        descending: true,
        ..query.clone()
    };
    let queue = db.songs_queue(&filtered, 5).await.unwrap();
    assert_eq!(queue.tracks.len(), 99);
    assert_eq!(queue.index, 94);
    assert_eq!(queue.tracks[queue.index - 1].id, 6);
    assert_eq!(queue.tracks[queue.index + 1].id, 4);
    assert!(db.songs_queue(&filtered, 101).await.is_err());
    let reversed = db
        .songs_page("", SongSort::Duration, true, 0)
        .await
        .unwrap();
    assert_eq!(reversed.rows[0].id, 101);
    let searched = db
        .songs_page("song 101", SongSort::Artist, false, 0)
        .await
        .unwrap();
    assert_eq!(searched.total, 1);
    assert_eq!(searched.rows[0].id, 101);
    assert_eq!(
        db.songs_page("%", SongSort::Title, false, 0)
            .await
            .unwrap()
            .total,
        0
    );
}
