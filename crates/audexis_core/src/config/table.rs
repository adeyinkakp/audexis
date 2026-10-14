use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SongColumn {
    Title,
    Artist,
    Album,
    Duration,
    Format,
    FileName,
    Path,
    Size,
    Modified,
    Plays,
    Loved,
    Metadata(String),
    Custom { key: String, kind: CustomFieldKind },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomFieldKind {
    Text,
    Url,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomFieldDefinition {
    pub id: String,
    pub title: String,
    pub key: String,
    pub kind: CustomFieldKind,
}
impl CustomFieldDefinition {
    pub fn column(&self) -> SongColumn {
        SongColumn::Custom {
            key: self.key.clone(),
            kind: self.kind,
        }
    }
    pub fn validate(&self, others: &[Self]) -> Result<(), String> {
        if self.title.trim().is_empty() || self.key.trim().is_empty() {
            return Err("Enter a display title and metadata key.".into());
        }
        if self.key.contains(['=', '\0']) || self.title.contains('\0') {
            return Err("Metadata keys cannot contain '=' or NUL characters.".into());
        }
        if others
            .iter()
            .any(|f| f.id != self.id && f.column() == self.column())
        {
            return Err("This metadata key is already defined for that field type.".into());
        }
        Ok(())
    }
}
impl SongColumn {
    pub fn storage_key(&self, format: &str) -> Option<String> {
        match self {
            Self::Metadata(key) => Some(key.clone()),
            Self::Custom { key, kind } => {
                let (kind, key) = match format {
                    "FLAC" | "Ogg" => (CustomFieldKind::Text, key.to_uppercase()),
                    "Itunes" => (
                        CustomFieldKind::Text,
                        key.strip_prefix("----:com.apple.iTunes:")
                            .unwrap_or(key)
                            .to_string(),
                    ),
                    _ => (*kind, key.clone()),
                };
                Some(format!(
                    "custom:{}:{key}",
                    if kind == CustomFieldKind::Text {
                        "text"
                    } else {
                        "url"
                    }
                ))
            }
            _ => None,
        }
    }
    pub fn default_width(&self) -> u16 {
        match self {
            Self::Title => 240,
            Self::Artist => 180,
            Self::Album => 200,
            Self::Duration | Self::Format | Self::Plays | Self::Loved => 90,
            Self::Path => 320,
            Self::FileName => 220,
            Self::Size => 110,
            _ => 180,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableColumn {
    pub column: SongColumn,
    pub width: u16,
}
impl TableColumn {
    pub fn new(column: SongColumn) -> Self {
        Self {
            width: column.default_width(),
            column,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SongsTableConfig {
    pub columns: Vec<TableColumn>,
    pub hidden_columns: Vec<TableColumn>,
    pub sort: SongColumn,
    pub descending: bool,
}
impl Default for SongsTableConfig {
    fn default() -> Self {
        Self {
            columns: [
                SongColumn::Title,
                SongColumn::Artist,
                SongColumn::Album,
                SongColumn::Duration,
                SongColumn::Format,
            ]
            .into_iter()
            .map(TableColumn::new)
            .collect(),
            hidden_columns: Vec::new(),
            sort: SongColumn::Title,
            descending: false,
        }
    }
}
impl SongsTableConfig {
    pub fn move_column(&mut self, source: &SongColumn, target: &SongColumn) -> bool {
        let Some(from) = self.columns.iter().position(|c| &c.column == source) else {
            return false;
        };
        let Some(to) = self.columns.iter().position(|c| &c.column == target) else {
            return false;
        };
        if from == to {
            return false;
        }
        let column = self.columns.remove(from);
        self.columns.insert(to, column);
        true
    }

    pub fn normalize(&mut self) {
        if self.columns.is_empty() {
            let title = self
                .hidden_columns
                .iter()
                .position(|c| c.column == SongColumn::Title)
                .map(|index| self.hidden_columns.remove(index))
                .unwrap_or_else(|| TableColumn::new(SongColumn::Title));
            self.columns.push(title);
        }
        let mut seen = std::collections::HashSet::new();
        self.columns.retain(|c| seen.insert(c.column.clone()));
        self.hidden_columns
            .retain(|c| seen.insert(c.column.clone()));
        for col in self
            .columns
            .iter_mut()
            .chain(self.hidden_columns.iter_mut())
        {
            col.width = col.width.clamp(64, 1200);
        }
    }
    pub fn toggle(&mut self, column: SongColumn) {
        if let Some(index) = self.columns.iter().position(|c| c.column == column) {
            if self.columns.len() > 1 {
                self.hidden_columns.push(self.columns.remove(index));
            }
        } else {
            let col = self
                .hidden_columns
                .iter()
                .position(|c| c.column == column)
                .map(|index| self.hidden_columns.remove(index))
                .unwrap_or_else(|| TableColumn::new(column));
            self.columns.push(col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dragging_columns_preserves_widths_and_row_sorting() {
        let mut cfg = SongsTableConfig::default();
        cfg.columns[0].width = 333;
        cfg.sort = SongColumn::Album;
        cfg.descending = true;
        assert!(cfg.move_column(&SongColumn::Title, &SongColumn::Format));
        assert_eq!(cfg.columns.last().unwrap().column, SongColumn::Title);
        assert_eq!(cfg.columns.last().unwrap().width, 333);
        assert!(cfg.move_column(&SongColumn::Title, &SongColumn::Artist));
        assert_eq!(cfg.columns[0].column, SongColumn::Title);
        assert_eq!(cfg.sort, SongColumn::Album);
        assert!(cfg.descending);
        assert!(!cfg.move_column(&SongColumn::Title, &SongColumn::Title));
        assert!(!cfg.move_column(&SongColumn::Path, &SongColumn::Title));
        let saved: SongsTableConfig =
            serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(saved, cfg);
    }
    #[test]
    fn config_saves_to_disk_and_replaces_previous_preferences() {
        let root = std::env::temp_dir().join(format!("audexis-config-{}", uuid::Uuid::new_v4()));
        let path = root.join("settings/user_config.json");
        let mut config = super::super::UserConfig::default();
        super::super::UserConfig::save_config(&path, &config).unwrap();
        config.songs_table.columns[0].width = 420;
        config.songs_table.columns.swap(0, 2);
        config.songs_table.descending = true;
        config.custom_field_definitions.push(CustomFieldDefinition {
            id: "source".into(),
            title: "Source".into(),
            key: "SOURCE".into(),
            kind: CustomFieldKind::Url,
        });
        super::super::UserConfig::save_config(&path, &config).unwrap();
        assert_eq!(super::super::UserConfig::load_config(&path), config);
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn preferences_round_trip_and_hidden_widths_survive() {
        let mut config = super::super::UserConfig::default();
        let table = &mut config.songs_table;
        table.columns[0].width = 357;
        table.toggle(SongColumn::Title);
        table.toggle(SongColumn::Title);
        table.sort = SongColumn::Custom {
            key: "SOURCE".into(),
            kind: CustomFieldKind::Url,
        };
        table.descending = true;
        assert_eq!(table.columns.last().unwrap().width, 357);
        let restored: super::super::UserConfig =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(config, restored);
        let old: super::super::UserConfig = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(old.songs_table, SongsTableConfig::default());
    }
}
