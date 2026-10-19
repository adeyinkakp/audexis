use crate::ui::components::media::{MediaTarget, PlaylistOccurrence};
use crate::{
    services::AppServices,
    ui::{hooks::Playback, routes::Route},
};
use audexis_core::database::media_actions::MenuState;
use freya::{
    prelude::*,
    router::{NavigationTarget, RouterContext},
};
use std::sync::Arc;
mod model;
use model::{Action, Entry, entries, tracks};
mod actions;
pub(crate) mod app;
pub(crate) mod context;
use actions::dispatch;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod native;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use native::{native_item, native_popup};
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod fallback;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use fallback::fallback_item;
