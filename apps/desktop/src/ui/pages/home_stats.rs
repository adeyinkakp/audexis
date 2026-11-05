use crate::ui::theme::use_audexis_colors;
use freya::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(PartialEq)]
pub(super) struct HomeStats {
    pub tracks: i64,
    pub size: i64,
    pub albums: i64,
    pub last_scanned: Option<i64>,
}

impl Component for HomeStats {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let values = [
            ("Tracks", count(self.tracks)),
            ("Library size", bytes(self.size)),
            ("Albums", count(self.albums)),
            ("Last scanned", scanned(self.last_scanned)),
        ];
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .spacing(12.)
            .children(values.into_iter().map(|(title, value)| {
                rect()
                    .width(Size::flex(1.))
                    .padding((12., 16.))
                    .spacing(6.)
                    .corner_radius(16.)
                    .background(colors.surface_subtle)
                    .border(Border::new().width(1.).fill(colors.border_subtle))
                    .child(
                        label()
                            .text(title)
                            .font_size(12.)
                            .color(colors.muted_foreground),
                    )
                    .child(
                        label()
                            .text(value)
                            .font_size(14.)
                            .font_weight(FontWeight::BOLD),
                    )
                    .into_element()
            }))
    }
}

fn count(value: i64) -> String {
    let digits = value.max(0).to_string();
    let mut result = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            result.push(',');
        }
        result.push(digit);
    }
    result
}

fn bytes(value: i64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = value.max(0) as f64;
    let mut unit = 0;
    while size >= 1024. && unit < units.len() - 1 {
        size /= 1024.;
        unit += 1;
    }
    if unit > 1 {
        format!("{size:.1} {}", units[unit])
    } else {
        format!("{size:.0} {}", units[unit])
    }
}

fn scanned(timestamp: Option<i64>) -> String {
    let Some(timestamp) = timestamp.filter(|value| *value > 0) else {
        return "Never".into();
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let seconds = now.saturating_sub(timestamp as u64);
    if seconds < 60 {
        return "Just now".into();
    }
    let (value, unit) = if seconds < 3600 {
        (seconds / 60, "minute")
    } else if seconds < 86400 {
        (seconds / 3600, "hour")
    } else if seconds < 604800 {
        (seconds / 86400, "day")
    } else if seconds < 2592000 {
        (seconds / 604800, "week")
    } else if seconds < 31536000 {
        (seconds / 2592000, "month")
    } else {
        (seconds / 31536000, "year")
    };
    format!("{value} {unit}{} ago", if value == 1 { "" } else { "s" })
}
