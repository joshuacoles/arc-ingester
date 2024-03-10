use std::collections::HashMap;
use std::fs::ReadDir;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use clap::Parser;
use itertools::Itertools;
use sqlx::FromRow;
use rayon::prelude::*;
use serde_json::Value;
use uuid::Uuid;

#[derive(Parser, Debug)]
struct Cli {
    /// Root of the Arc iCloud directory
    #[arg(long, short, env = "ARC_ROOT")]
    root: PathBuf,

    /// psql connection string
    #[arg(long, short, env = "ARC_DB")]
    conn: String,
}

#[derive(Debug, FromRow)]
struct DateHash {
    date: String,
    sha256: String,
}

#[derive(Debug, FromRow)]
struct TimelineItemUpdatedCheckRow {
    item_id: Uuid,
    last_saved: DateTime<Utc>,
}

#[derive(Debug)]
struct UpdatedFile {
    date: String,
    sha256: String,
    json: FileContents,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FileContents {
    timeline_items: Vec<TimelineItem>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineItem {
    item_id: Uuid,
    place: Option<Place>,
    end_date: DateTime<Utc>,
    last_saved: DateTime<Utc>,

    #[serde(flatten)]
    rest: HashMap<String, Value>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Place {
    place_id: Uuid,
    last_saved: DateTime<Utc>,

    #[serde(flatten)]
    rest: HashMap<String, Value>,
}

async fn find_updated(db: &sqlx::PgPool, files: ReadDir) -> Vec<UpdatedFile> {
    let date_hashes: Vec<DateHash> = sqlx::query_as("SELECT date, sha256 FROM raw_files")
        .fetch_all(db)
        .await
        .unwrap();

    let date_hashes: Arc<HashMap<String, String>> = Arc::new(date_hashes.into_iter()
        .map(|dh| (dh.date, dh.sha256))
        .collect());

    let date_hashes = &date_hashes;

    // Find the files that need to be refreshed, based on the sha256 hash of the file compared to
    // the hash stored in the database.
    let need_refresh = files.into_iter()
        .map(|f| f.unwrap().path())
        .map(|path| async move {
            // Extract the date from the file name
            let date = {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                let i = file_name.find('.').unwrap();
                &file_name[..i]
            };

            let bytes = tokio::fs::read(&path).await.unwrap();
            let current_hash = sha256::digest(&bytes);
            let existing_hash = date_hashes.get(date);

            if let Some(existing_hash) = existing_hash {
                if current_hash == *existing_hash {
                    return None;
                }
            }

            return Some((date.to_string(), current_hash, bytes));
        });

    let need_refresh = futures::future::join_all(need_refresh).await.into_iter()
        .filter_map(|x| x)
        .collect::<Vec<_>>();

    let decompressed = need_refresh.par_iter().map(|(date, new_hash, bytes)| {
        let mut decoder = flate2::bufread::GzDecoder::new(&bytes[..]);
        let mut string = String::new();
        decoder.read_to_string(&mut string).unwrap();

        UpdatedFile {
            date: date.clone(),
            sha256: new_hash.clone(),
            json: serde_json::from_str(&string).unwrap(),
        }
    }).collect::<Vec<_>>();

    decompressed
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let daily_exports = cli.root.join("Export/JSON/Daily");
    let files = std::fs::read_dir(daily_exports).unwrap();

    let db = sqlx::PgPool::connect(&cli.conn)
        .await
        .unwrap();

    sqlx::migrate!()
        .run(&db)
        .await
        .unwrap();

    let need_refresh = find_updated(&db, files).await;

    // Refresh the database with the new files
    for updated_file in &need_refresh {
        sqlx::query("INSERT INTO raw_files (date, sha256, json) VALUES ($1, $2, $3 :: jsonb) ON CONFLICT (date) DO UPDATE SET sha256 = $2, json = $3 :: jsonb")
            .bind(&updated_file.date)
            .bind(&updated_file.sha256)
            .bind(&serde_json::to_value(&updated_file.json).unwrap())
            .execute(&db)
            .await
            .unwrap();
    }

    // Take all the changed files' timeline items, and group them by item_id, then take the latest one.
    // If we are needing to update the database it will be with this one.
    let possibly_timeline_items = need_refresh.into_iter()
        .flat_map(|f| f.json.timeline_items)
        .group_by(|item| item.item_id.clone())
        .into_iter()
        .map(|(_, items)| items.max_by_key(|item| item.last_saved).unwrap())
        .collect_vec();

    let possibly_timeline_item_ids = possibly_timeline_items
        .iter()
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();

    let existing_last_saved_at_map = sqlx::query_as("SELECT item_id, end_date, last_saved FROM timeline_item where item_id = ANY($1)")
        .bind(&possibly_timeline_item_ids)
        .fetch_all(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|row: TimelineItemUpdatedCheckRow| (row.item_id, row.last_saved))
        .collect::<HashMap<_, _>>();

    for updated_timeline_item in possibly_timeline_items {
        // The contents of duplicated entries appears to be identical, so we only check to see if
        // the last_saved date is newer than the one we have in the database.
        if let Some(existing_last_saved_at) = existing_last_saved_at_map.get(&updated_timeline_item.item_id) {
            if updated_timeline_item.last_saved <= *existing_last_saved_at {
                continue;
            }
        }

        // We have a new or updated timeline item, we need to insert its associated place first if
        // it exists.
        // ASSUMPTION: This assumes that all place changes as associated with the timeline item
        // changes. Is this true?
        if let Some(place) = &updated_timeline_item.place {
            sqlx::query("INSERT INTO place (place_id, json, last_saved, server_last_updated) VALUES ($1, $2 :: jsonb, $3, now()) ON CONFLICT (place_id) DO UPDATE SET json = $2 :: jsonb, last_saved = $3, server_last_updated = now()")
                .bind(&place.place_id)
                .bind(&serde_json::to_value(&place).unwrap())
                .bind(&place.last_saved)
                .execute(&db).await.unwrap();
        }

        // Then we can insert/update the timeline item.
        sqlx::query("INSERT INTO timeline_item (item_id, json, place_id, end_date, last_saved, server_last_updated) VALUES ($1, $2 :: jsonb, $3, $4, $5, now()) ON CONFLICT (item_id) DO UPDATE SET json = $2 :: jsonb, place_id = $3, end_date = $4, last_saved = $5, server_last_updated = now()")
            .bind(&updated_timeline_item.item_id)
            .bind(&serde_json::to_value(&updated_timeline_item).unwrap())
            .bind(&updated_timeline_item.place.map(|place| place.place_id))
            .bind(&updated_timeline_item.end_date)
            .bind(&updated_timeline_item.last_saved)
            .execute(&db).await.unwrap();
    }
}
