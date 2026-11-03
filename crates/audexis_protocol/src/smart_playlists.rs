use crate::Track;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SmartPlaylist {
    #[serde(default = "live_updating_default")]
    pub live_updating: bool,
    pub match_all: bool,
    pub rules: Vec<SmartRule>,
    pub sort: SmartSort,
    pub limit: Option<u32>,
}
impl Default for SmartPlaylist {
    fn default() -> Self {
        Self {
            live_updating: true,
            match_all: true,
            rules: vec![SmartRule::default()],
            sort: SmartSort::Title,
            limit: None,
        }
    }
}
fn live_updating_default() -> bool {
    true
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SmartRule {
    pub field: SmartField,
    pub operator: SmartOperator,
    pub value: String,
}
impl Default for SmartRule {
    fn default() -> Self {
        Self {
            field: SmartField::Artist,
            operator: SmartOperator::Contains,
            value: String::new(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartField {
    Title,
    Artist,
    AlbumArtist,
    Album,
    Genre,
    Year,
    Format,
    Favorite,
    AddedDays,
    DurationSeconds,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartOperator {
    Contains,
    NotContains,
    Equals,
    NotEquals,
    StartsWith,
    GreaterThan,
    LessThan,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartSort {
    Title,
    Artist,
    Album,
    Newest,
    Oldest,
}

impl SmartField {
    pub const ALL: [Self; 10] = [
        Self::Title,
        Self::Artist,
        Self::AlbumArtist,
        Self::Album,
        Self::Genre,
        Self::Year,
        Self::Format,
        Self::Favorite,
        Self::AddedDays,
        Self::DurationSeconds,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Artist => "Artist",
            Self::AlbumArtist => "Album artist",
            Self::Album => "Album",
            Self::Genre => "Genre",
            Self::Year => "Year",
            Self::Format => "Format",
            Self::Favorite => "Favorite",
            Self::AddedDays => "Days since added",
            Self::DurationSeconds => "Duration (seconds)",
        }
    }
    pub fn operators(self) -> &'static [SmartOperator] {
        use SmartOperator::*;
        match self {
            Self::Favorite => &[Equals, NotEquals],
            Self::Year | Self::AddedDays | Self::DurationSeconds => {
                &[Equals, NotEquals, LessThan, GreaterThan]
            }
            _ => &[Contains, NotContains, Equals, NotEquals, StartsWith],
        }
    }
    fn key(self) -> Option<&'static str> {
        match self {
            Self::Title => Some("title"),
            Self::Artist => Some("artist"),
            Self::AlbumArtist => Some("albumArtist"),
            Self::Album => Some("album"),
            Self::Genre => Some("genre"),
            Self::Year => Some("year"),
            _ => None,
        }
    }
}
impl SmartOperator {
    pub fn label(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::NotContains => "does not contain",
            Self::Equals => "is",
            Self::NotEquals => "is not",
            Self::StartsWith => "starts with",
            Self::GreaterThan => "greater than",
            Self::LessThan => "less than",
        }
    }
}
impl SmartSort {
    pub const ALL: [Self; 5] = [
        Self::Title,
        Self::Artist,
        Self::Album,
        Self::Newest,
        Self::Oldest,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Artist => "Artist",
            Self::Album => "Album",
            Self::Newest => "Newest first",
            Self::Oldest => "Oldest first",
        }
    }
}
impl SmartPlaylist {
    pub fn validate(&self) -> Result<(), String> {
        if self.rules.is_empty() || self.rules.len() > 20 {
            return Err("Use between 1 and 20 rules.".into());
        }
        if self.limit.is_some_and(|n| n == 0 || n > 10000) {
            return Err("Track limit must be between 1 and 10,000.".into());
        }
        for rule in &self.rules {
            if rule.value.trim().is_empty()
                || rule.value.len() > 512
                || !rule.field.operators().contains(&rule.operator)
            {
                return Err("Every rule needs a valid comparison and value.".into());
            }
            match rule.field {
                SmartField::Favorite if !matches!(rule.value.trim(), "true" | "false") => {
                    return Err("Favorite must be true or false.".into());
                }
                SmartField::Year | SmartField::AddedDays | SmartField::DurationSeconds => {
                    if !rule
                        .value
                        .trim()
                        .parse::<f64>()
                        .is_ok_and(|v| v.is_finite() && v >= 0.0 && v <= 1e9)
                    {
                        return Err("Numeric rules need a nonnegative number.".into());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub fn select(&self, tracks: &[Track], now: i64) -> Vec<String> {
        let mut matches: Vec<_> = tracks
            .iter()
            .filter(|track| {
                !track.deleted
                    && if self.match_all {
                        self.rules.iter().all(|rule| rule.matches(track, now))
                    } else {
                        self.rules.iter().any(|rule| rule.matches(track, now))
                    }
            })
            .collect();
        matches.sort_by(|a, b| {
            let order = match self.sort {
                SmartSort::Title => a.title().to_lowercase().cmp(&b.title().to_lowercase()),
                SmartSort::Artist => a
                    .artist()
                    .to_lowercase()
                    .cmp(&b.artist().to_lowercase())
                    .then_with(|| a.title().to_lowercase().cmp(&b.title().to_lowercase())),
                SmartSort::Album => tag(a, "album")
                    .to_lowercase()
                    .cmp(&tag(b, "album").to_lowercase())
                    .then_with(|| a.title().to_lowercase().cmp(&b.title().to_lowercase())),
                SmartSort::Newest => b.added_at.cmp(&a.added_at),
                SmartSort::Oldest => a.added_at.cmp(&b.added_at),
            };
            order.then_with(|| a.id.cmp(&b.id))
        });
        matches.truncate(self.limit.unwrap_or(10000) as usize);
        matches.into_iter().map(|track| track.id.clone()).collect()
    }
}
fn tag<'a>(track: &'a Track, key: &str) -> &'a str {
    track
        .metadata
        .tags
        .get(key)
        .and_then(|v| v.first())
        .map(String::as_str)
        .unwrap_or("")
}
impl SmartRule {
    fn matches(&self, track: &Track, now: i64) -> bool {
        use SmartField::*;
        use SmartOperator::*;
        let compare_number = |value: f64| {
            let Ok(expected) = self.value.trim().parse::<f64>() else {
                return false;
            };
            match self.operator {
                Equals => value == expected,
                NotEquals => value != expected,
                GreaterThan => value > expected,
                LessThan => value < expected,
                _ => false,
            }
        };
        match self.field {
            Favorite => {
                let equal = track.metadata.loved == (self.value.trim() == "true");
                return if self.operator == NotEquals {
                    !equal
                } else {
                    equal
                };
            }
            AddedDays => {
                return track.added_at > 0
                    && compare_number((now.saturating_sub(track.added_at).max(0) / 86400) as f64);
            }
            DurationSeconds => {
                return track
                    .duration_ms
                    .is_some_and(|ms| compare_number(ms as f64 / 1000.0));
            }
            Year => {
                return track.metadata.tags.get("year").is_some_and(|values| {
                    let years: Vec<_> = values
                        .iter()
                        .filter_map(|v| v.get(..4).unwrap_or(v).parse::<f64>().ok())
                        .collect();
                    !years.is_empty()
                        && if self.operator == NotEquals {
                            years.into_iter().all(compare_number)
                        } else {
                            years.into_iter().any(compare_number)
                        }
                });
            }
            _ => {}
        }
        let values = match self.field {
            Format => vec![track.format.as_str()],
            Title if tag(track, "title").is_empty() => vec![track.file_name.as_str()],
            field => field
                .key()
                .and_then(|key| track.metadata.tags.get(key))
                .map(|v| v.iter().map(String::as_str).collect())
                .unwrap_or_default(),
        };
        let expected = self.value.trim().to_lowercase();
        let found = values.iter().any(|v| {
            let value = v.trim().to_lowercase();
            match self.operator {
                Contains | NotContains => value.contains(&expected),
                Equals | NotEquals => value == expected,
                StartsWith => value.starts_with(&expected),
                _ => false,
            }
        });
        if matches!(self.operator, NotContains | NotEquals) {
            !found
        } else {
            found
        }
    }
}
