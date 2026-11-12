use audexis_protocol::{ListeningPeriod, ListeningStats};
use sqlx::SqliteConnection;
use std::collections::BTreeMap;

pub(super) async fn read(
    connection: &mut SqliteConnection,
    period: Option<ListeningPeriod>,
) -> Result<BTreeMap<i64, ListeningStats>, sqlx::Error> {
    let (start, end) = match period {
        Some(period) => period.bounds().map_err(sqlx::Error::Protocol)?,
        None => ("0000-01-01".into(), "9999-01-01".into()),
    };
    let mut stats = BTreeMap::<i64, ListeningStats>::new();
    let counts: Vec<(i64, i64)> = if period.is_some() {
        sqlx::query_as(
            "SELECT file_id,COUNT(*) FROM counted_plays
             WHERE date(played_at,'unixepoch','localtime')>=?
               AND date(played_at,'unixepoch','localtime')<?
             GROUP BY file_id",
        )
        .bind(&start)
        .bind(&end)
        .fetch_all(&mut *connection)
        .await?
    } else {
        sqlx::query_as("SELECT file_id,plays FROM media_info")
            .fetch_all(&mut *connection)
            .await?
    };
    for (file, plays) in counts {
        stats.entry(file).or_default().plays = plays;
    }
    let times: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT file_id,SUM(heard_us) FROM listening_time_daily
         WHERE day>=? AND day<? GROUP BY file_id",
    )
    .bind(start)
    .bind(end)
    .fetch_all(connection)
    .await?;
    for (file, listened_us) in times {
        stats.entry(file).or_default().listened_us = listened_us;
    }
    Ok(stats)
}
