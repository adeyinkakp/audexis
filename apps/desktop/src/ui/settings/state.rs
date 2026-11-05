#[derive(Clone, PartialEq)]
pub(super) struct Preferences {
    pub(super) theme: audexis_core::config::ThemePreference,
    pub(super) density: audexis_core::config::Density,
    pub(super) reduce_motion: bool,
    pub(super) folders: Vec<String>,
}
#[derive(Clone)]
pub(super) struct Draft {
    pub(super) value: Preferences,
    pub(super) saved: Preferences,
    pub(super) loaded: bool,
    pub(super) busy: bool,
    pub(super) error: Option<String>,
}
impl Draft {
    pub(super) fn dirty(&self) -> bool {
        self.value != self.saved
    }
}
