pub use scroll::TableScroll;
mod fields;
mod header;
mod menu;
pub use fields::CustomFieldsEditor;
pub use header::TableHeader;

use crate::services::AppServices;
use audexis_core::{config::*, tag_manager::field_catalog};
use freya::prelude::*;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub struct TableSettings {
    pub table: State<SongsTableConfig>,
    pub definitions: State<Vec<CustomFieldDefinition>>,
    pub editing_fields: State<bool>,
    pub error: State<Option<String>>,
}
pub fn use_table_settings() -> TableSettings {
    let services = use_consume::<Arc<AppServices>>();
    let initial = use_hook(|| {
        services
            .user_config
            .lock()
            .expect("Settings lock poisoned")
            .clone()
    });
    TableSettings {
        table: use_state(|| {
            let mut table = initial.songs_table;
            table.normalize();
            table
        }),
        definitions: use_state(|| initial.custom_field_definitions),
        editing_fields: use_state(|| false),
        error: use_state(|| None),
    }
}
impl TableSettings {
    pub fn save(&self, services: &AppServices) {
        let table = self.table.peek().clone();
        let definitions = self.definitions.peek().clone();
        let result = services.update_config(|cfg| {
            cfg.songs_table = table;
            cfg.custom_field_definitions = definitions;
        });
        let mut error = self.error;
        error.set(
            result
                .err()
                .map(|e| format!("Could not save table settings: {e}")),
        );
    }
    pub fn title(&self, column: &SongColumn) -> String {
        column_title(column, &self.definitions.read())
    }
}
pub fn column_title(column: &SongColumn, definitions: &[CustomFieldDefinition]) -> String {
    match column {
        SongColumn::Title => "Title",
        SongColumn::Artist => "Artist",
        SongColumn::Album => "Album",
        SongColumn::Duration => "Time",
        SongColumn::Format => "Format",
        SongColumn::FileName => "File name",
        SongColumn::Path => "Path",
        SongColumn::Size => "Size",
        SongColumn::Modified => "Date modified",
        SongColumn::Plays => "Plays",
        SongColumn::Loved => "Loved",
        SongColumn::Metadata(key) => {
            return field_catalog::catalog(None)
                .into_iter()
                .find(|f| f.storage_key == *key)
                .map(|f| f.label.to_owned())
                .unwrap_or_else(|| key.clone());
        }
        SongColumn::Custom { key, .. } => {
            return definitions
                .iter()
                .find(|f| f.column() == *column)
                .map(|f| f.title.clone())
                .unwrap_or_else(|| key.clone());
        }
    }
    .into()
}
pub fn column_catalog() -> Vec<(String, SongColumn)> {
    let mut columns: Vec<_> = [
        SongColumn::Title,
        SongColumn::Artist,
        SongColumn::Album,
        SongColumn::Duration,
        SongColumn::Format,
        SongColumn::FileName,
        SongColumn::Path,
        SongColumn::Size,
        SongColumn::Modified,
        SongColumn::Plays,
        SongColumn::Loved,
    ]
    .into_iter()
    .map(|c| (column_title(&c, &[]), c))
    .collect();
    columns.extend(
        field_catalog::catalog(None)
            .into_iter()
            .filter(|f| {
                matches!(f.kind, "text" | "url")
                    && !matches!(f.storage_key.as_str(), "title" | "artist" | "album")
            })
            .map(|f| (f.label.to_string(), SongColumn::Metadata(f.storage_key))),
    );
    columns
}
mod scroll;
pub use scroll::use_table_scroll;

mod row;
pub(crate) use row::SongTableRow;
pub(crate) const ROW_HEIGHT: f32 = 46.0;
