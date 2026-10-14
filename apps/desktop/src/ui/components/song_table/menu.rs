use super::{TableSettings, column_catalog};
use crate::services::AppServices;
use audexis_core::config::{SongColumn, SongsTableConfig};
use freya::prelude::*;
use std::sync::Arc;

#[derive(Clone, Debug)]
enum Action {
    Toggle(SongColumn),
    Sort(SongColumn, bool),
    Move(SongColumn, isize),
    Width(SongColumn),
    Reset,
    Fields,
}
struct Entry {
    title: String,
    enabled: bool,
    checked: Option<bool>,
    action: Option<Action>,
    children: Vec<Entry>,
}
impl Entry {
    fn item(title: impl Into<String>, action: Action) -> Self {
        Self {
            title: title.into(),
            enabled: true,
            checked: None,
            action: Some(action),
            children: vec![],
        }
    }
    fn group(title: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            title: title.into(),
            enabled: !children.is_empty(),
            checked: None,
            action: None,
            children,
        }
    }
    fn check(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
    fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    fn separator() -> Self {
        Self::group("", vec![])
    }
}
fn entries(column: Option<SongColumn>, settings: &TableSettings) -> Vec<Entry> {
    let table = settings.table.peek();
    let mut result = Vec::new();
    if let Some(column) = column {
        let index = table
            .columns
            .iter()
            .position(|c| c.column == column)
            .unwrap_or(0);
        result.extend([
            Entry::item("Sort ascending", Action::Sort(column.clone(), false))
                .check(table.sort == column && !table.descending),
            Entry::item("Sort descending", Action::Sort(column.clone(), true))
                .check(table.sort == column && table.descending),
            Entry::separator(),
            Entry::item("Move column left", Action::Move(column.clone(), -1)).enabled(index > 0),
            Entry::item("Move column right", Action::Move(column.clone(), 1))
                .enabled(index + 1 < table.columns.len()),
            Entry::item("Reset column width", Action::Width(column.clone())),
            Entry::item("Hide column", Action::Toggle(column)).enabled(table.columns.len() > 1),
            Entry::separator(),
        ]);
    }
    let toggle = |title: String, column: SongColumn| {
        let visible = table.columns.iter().any(|c| c.column == column);
        Entry::item(title, Action::Toggle(column))
            .check(visible)
            .enabled(!visible || table.columns.len() > 1)
    };
    let mut catalog = column_catalog();
    let mut metadata = catalog.split_off(11);
    result.push(Entry::group(
        "Columns",
        catalog
            .into_iter()
            .map(|(title, col)| toggle(title, col))
            .collect(),
    ));
    metadata.sort_by(|a, b| a.0.cmp(&b.0));
    result.push(Entry::group(
        "Metadata fields",
        metadata
            .into_iter()
            .map(|(title, column)| toggle(title, column))
            .collect(),
    ));
    result.push(Entry::group(
        "Custom fields",
        settings
            .definitions
            .peek()
            .iter()
            .map(|field| toggle(field.title.clone(), field.column()))
            .collect(),
    ));
    result.push(Entry::item("Manage custom fields…", Action::Fields));
    result.push(Entry::separator());
    result.push(Entry::item("Reset table layout and sorting", Action::Reset));
    result
}
fn dispatch(action: Action, mut settings: TableSettings, services: &AppServices) {
    if matches!(action, Action::Fields) {
        settings.editing_fields.set(true);
        return;
    }
    {
        let mut table = settings.table.write();
        match action {
            Action::Toggle(col) => table.toggle(col),
            Action::Sort(col, desc) => {
                table.sort = col;
                table.descending = desc;
            }
            Action::Move(col, delta) => {
                if let Some(index) = table.columns.iter().position(|c| c.column == col) {
                    let next = index as isize + delta;
                    if next >= 0 && next < table.columns.len() as isize {
                        table.columns.swap(index, next as usize);
                    }
                }
            }
            Action::Width(col) => {
                if let Some(entry) = table.columns.iter_mut().find(|c| c.column == col) {
                    entry.width = col.default_width();
                }
            }
            Action::Reset => *table = SongsTableConfig::default(),
            Action::Fields => unreachable!(),
        }
    }
    settings.save(services);
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn open(
    _event: &Event<PressEventData>,
    column: Option<SongColumn>,
    settings: TableSettings,
    services: Arc<AppServices>,
) {
    let entries = entries(column, &settings);
    let (tx, rx) = tokio::sync::oneshot::channel();
    Platform::get().with_window(None, move |window| {
        let _ = tx.send(popup(window, entries));
    });
    spawn(async move {
        match rx.await {
            Ok(Ok(Some(action))) => dispatch(action, settings, &services),
            Ok(Ok(None)) => {}
            result => {
                let mut error = settings.error;
                error.set(Some(format!("Could not open column menu: {result:?}")));
            }
        }
    });
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn popup(
    window: &freya::winit::window::Window,
    entries: Vec<Entry>,
) -> Result<Option<Action>, String> {
    use muda::ContextMenu as _;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    fn item(
        entry: Entry,
        actions: &mut Vec<(muda::MenuId, Action)>,
    ) -> Result<Box<dyn muda::IsMenuItem>, String> {
        if entry.title.is_empty() {
            return Ok(Box::new(muda::PredefinedMenuItem::separator()));
        }
        if !entry.children.is_empty() {
            let group = muda::Submenu::new(entry.title, entry.enabled);
            for child in entry.children {
                group
                    .append(item(child, actions)?.as_ref())
                    .map_err(|e| e.to_string())?;
            }
            return Ok(Box::new(group));
        }
        let item: Box<dyn muda::IsMenuItem> = if let Some(checked) = entry.checked {
            Box::new(muda::CheckMenuItem::new(
                entry.title,
                entry.enabled,
                checked,
                None,
            ))
        } else {
            Box::new(muda::MenuItem::new(entry.title, entry.enabled, None))
        };
        if entry.enabled {
            if let Some(action) = entry.action {
                actions.push((item.id().clone(), action));
            }
        }
        Ok(item)
    }
    let menu = muda::Menu::new();
    let mut actions = Vec::new();
    for entry in entries {
        menu.append(item(entry, &mut actions)?.as_ref())
            .map_err(|e| e.to_string())?;
    }
    let handle = window.window_handle().map_err(|e| e.to_string())?;
    let selected = unsafe {
        match handle.as_raw() {
            #[cfg(target_os = "macos")]
            RawWindowHandle::AppKit(handle) => {
                menu.show_context_menu_for_nsview(handle.ns_view.as_ptr(), None)
            }
            #[cfg(target_os = "windows")]
            RawWindowHandle::Win32(handle) => {
                menu.show_context_menu_for_hwnd(handle.hwnd.get(), None)
            }
            _ => return Err("Unsupported native window handle".into()),
        }
    };
    let action = muda::MenuEvent::receiver().try_iter().find_map(|event| {
        actions
            .iter()
            .find(|(id, _)| *id == event.id)
            .map(|(_, action)| action.clone())
    });
    Ok(if selected { action } else { None })
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn open(
    event: &Event<PressEventData>,
    column: Option<SongColumn>,
    settings: TableSettings,
    services: Arc<AppServices>,
) {
    fn item(entry: Entry, settings: TableSettings, services: Arc<AppServices>) -> Element {
        if entry.title.is_empty() {
            return rect()
                .height(Size::px(1.0))
                .width(Size::fill())
                .background((100, 100, 100))
                .into_element();
        }
        if !entry.children.is_empty() {
            return SubMenu::new()
                .label(entry.title)
                .children(
                    entry
                        .children
                        .into_iter()
                        .map(|child| item(child, settings.clone(), services.clone())),
                )
                .into_element();
        }
        let title = format!(
            "{}{}",
            if entry.checked == Some(true) {
                "✓ "
            } else {
                ""
            },
            entry.title
        );
        if !entry.enabled {
            return rect()
                .padding(8.0)
                .opacity(0.45)
                .child(title)
                .into_element();
        }
        MenuButton::new()
            .child(title)
            .on_press(move |_| {
                ContextMenu::close();
                if let Some(action) = entry.action.clone() {
                    dispatch(action, settings.clone(), &services);
                }
            })
            .into_element()
    }
    let menu = Menu::new().children(
        entries(column, &settings)
            .into_iter()
            .map(|entry| item(entry, settings.clone(), services.clone())),
    );
    ContextMenu::open_from_event(event, menu);
}
