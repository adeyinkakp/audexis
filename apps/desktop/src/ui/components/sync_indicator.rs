use crate::{
    services::AppServices,
    ui::{routes::Route, theme::use_audexis_colors},
};
use audexis_core::cloud_sync::{CloudSyncStatus, SyncPhase};
use freya::prelude::*;
use std::sync::Arc;

#[derive(PartialEq)]
pub struct SyncIndicator;
impl Component for SyncIndicator {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let mut status = use_state(|| services.sync_status.borrow().clone());
        let mut progress = use_state(|| services.sync_progress.borrow().clone());
        use_future(move || {
            let mut statuses = services.sync_status.subscribe();
            let mut updates = services.sync_progress.subscribe();
            async move {
                status.set(statuses.borrow_and_update().clone());
                progress.set(updates.borrow_and_update().clone());
                loop {
                    tokio::select! {
                        changed=statuses.changed()=>{if changed.is_err(){break;}status.set(statuses.borrow_and_update().clone());},
                        changed=updates.changed()=>{if changed.is_err(){break;}progress.set(updates.borrow_and_update().clone());},
                    }
                }
            }
        });
        let state = status.read().clone();
        let current = progress.read().clone();
        if state == CloudSyncStatus::Stopped {
            return rect().into_element();
        }
        let title = match &state {
            _ if current.phase == SyncPhase::Uploading => {
                format!("Uploading · {:.0}%", current.percent())
            }
            _ if current.phase == SyncPhase::Updating => "Saving changes…".into(),
            _ if current.phase == SyncPhase::Processing && current.file_name.is_some() => {
                "Processing audio…".into()
            }
            _ if current.retry_at.is_some() => "Server busy · retrying shortly".into(),
            CloudSyncStatus::Failed(_) => "Sync needs attention".into(),
            CloudSyncStatus::Running => match current.phase {
                SyncPhase::Uploading => format!("Uploading · {:.0}%", current.percent()),
                SyncPhase::Processing => "Processing audio…".into(),
                SyncPhase::Updating => "Saving changes…".into(),
                SyncPhase::Discovering => "Finding new music…".into(),
                _ => "Syncing library…".into(),
            },
            _ if current.phase == SyncPhase::Processing && current.queued > 0 => {
                "Processing audio…".into()
            }
            _ if current.queued > 0 => format!("{} items pending", current.queued),
            _ => "Library synced".into(),
        };
        let percent = if current.phase == SyncPhase::Uploading {
            current.percent()
        } else {
            let total = current.completed.saturating_add(current.queued);
            if total > 0 {
                current.completed as f32 / total as f32 * 100.
            } else {
                100.
            }
        };
        let mut body =
            rect()
                .width(Size::fill())
                .spacing(6.)
                .child(label().text(title).font_size(12.).color(
                    if matches!(state, CloudSyncStatus::Failed(_)) {
                        colors.destructive
                    } else {
                        colors.foreground
                    },
                ));
        if let Some(name) = current.file_name {
            body = body.child(
                label()
                    .text(crate::ui::display_text(name))
                    .font_size(11.)
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis)
                    .color(colors.muted_foreground),
            );
        }
        if current.queued > 0 || current.phase == SyncPhase::Uploading {
            body = body.child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(4.))
                    .corner_radius(2.)
                    .background(colors.border)
                    .child(
                        rect()
                            .width(Size::percent(percent.clamp(0., 100.)))
                            .height(Size::fill())
                            .corner_radius(2.)
                            .background(colors.primary),
                    ),
            );
        }
        let summary = if current.completed > 0 || current.queued > 0 {
            format!("{} done · {} remaining", current.completed, current.queued)
        } else {
            "View cloud library".into()
        };
        body = body.child(
            label()
                .text(summary)
                .font_size(11.)
                .color(colors.muted_foreground),
        );
        if current.failed > 0 {
            body = body.child(
                label()
                    .text(format!("{} need attention", current.failed))
                    .font_size(11.)
                    .color(colors.destructive),
            );
        }
        Link::new(Route::CloudLibrary)
            .child(
                SideBarItem::new()
                    .padding((10., 10.))
                    .background(colors.surface_subtle)
                    .hover_background(colors.hover)
                    .focus_border_fill(colors.ring)
                    .child(body),
            )
            .into_element()
    }
}
