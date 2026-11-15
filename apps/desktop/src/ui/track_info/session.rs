use super::*;

#[derive(Clone, Default)]
pub struct Session {
    pub(super) files: Vec<EditFile>,
    pub(super) busy: bool,
    pub(super) cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
    pub(super) progress: String,
    pub(super) message: String,
    pub(super) selected: usize,
    pub(super) revision: u64,
}
impl Session {
    pub(super) fn includes_file(&self, index: usize, tab: Tab) -> bool {
        tab != Tab::Custom || index == self.selected
    }
    pub(super) fn scoped_file(&self, index: usize, tab: Tab) -> EditFile {
        let file = &self.files[index];
        let mut tags = file.original.tags.clone();
        for key in file.changed_keys_matching(|key| tab.includes(key)) {
            if let Some(values) = file.tags.get(&key) {
                tags.insert(key, values.clone());
            } else {
                tags.remove(&key);
            }
        }
        EditFile {
            id: file.id,
            original: file.original.clone(),
            tags: tags.into(),
        }
    }
    pub(super) fn discard(&mut self, tab: Tab) {
        for (index, file) in self.files.iter_mut().enumerate() {
            if tab == Tab::Custom && index != self.selected {
                continue;
            }
            file.tags.retain(|key, _| !tab.includes(*key));
            file.tags.extend(
                file.original
                    .tags
                    .iter()
                    .filter(|(key, _)| tab.includes(**key))
                    .map(|(key, values)| (*key, values.clone())),
            );
        }
        self.revision += 1;
        self.message.clear();
    }
    pub(super) fn accept(&mut self, mut saved: EditFile, tab: Tab) {
        if let Some(old) = self.files.iter_mut().find(|f| f.id == saved.id) {
            for key in old.changed_keys().filter(|key| !tab.includes(*key)) {
                if let Some(values) = old.tags.get(&key) {
                    saved.tags.insert(key, values.clone());
                } else {
                    saved.tags.remove(&key);
                }
            }
            *old = saved;
        }
    }

    pub(super) fn dirty(&self) -> bool {
        self.files.iter().any(EditFile::dirty)
    }
    pub(super) fn values(&self, key: FrameKey) -> Vec<TagValue> {
        self.files
            .first()
            .and_then(|f| f.tags.get(&key))
            .cloned()
            .unwrap_or_default()
    }
    pub(super) fn mixed(&self, key: FrameKey) -> bool {
        let first = self
            .files
            .first()
            .and_then(|file| file.tags.get(&key))
            .map(Vec::as_slice)
            .unwrap_or_default();
        self.files
            .iter()
            .any(|file| file.tags.get(&key).map(Vec::as_slice).unwrap_or_default() != first)
    }
    pub(super) fn common_values(&self, key: FrameKey) -> Vec<TagValue> {
        if self.mixed(key) {
            Vec::new()
        } else {
            self.values(key)
        }
    }
    pub(super) fn set_values(&mut self, key: FrameKey, values: Vec<TagValue>) {
        for file in &mut self.files {
            file.tags.insert(key, values.clone());
        }
    }
    pub(super) fn text(&self, key: FrameKey) -> String {
        self.values(key).first().map(text_value).unwrap_or_default()
    }
}
pub(super) fn text_value(value: &TagValue) -> String {
    match value {
        TagValue::Text(s) => crate::ui::display_text(s.clone()),
        TagValue::Comment { text, .. } => crate::ui::display_text(text.clone()),
        _ => String::new(),
    }
}
