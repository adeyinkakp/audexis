use super::*;

#[derive(Clone, Default)]
pub struct Session {
    pub(super) files: Vec<EditFile>,
    pub(super) busy: bool,
    pub(super) message: String,
    pub(super) selected: usize,
    pub(super) revision: u64,
}
impl Session {
    pub(super) fn scoped(&self, tab: Tab) -> Vec<EditFile> {
        self.files
            .iter()
            .enumerate()
            .filter(|(index, _)| tab != Tab::Custom || *index == self.selected)
            .map(|(_, file)| {
                let mut scoped = file.clone();
                scoped.tags.retain(|key, _| tab.includes(*key));
                scoped.tags.extend(
                    file.original
                        .tags
                        .iter()
                        .filter(|(key, _)| !tab.includes(**key))
                        .map(|(key, values)| (*key, values.clone())),
                );
                scoped
            })
            .collect()
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
            for key in old.changes().keys().filter(|key| !tab.includes(**key)) {
                if let Some(values) = old.tags.get(key) {
                    saved.tags.insert(*key, values.clone());
                } else {
                    saved.tags.remove(key);
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
        let first = self.values(key);
        self.files
            .iter()
            .any(|f| f.tags.get(&key).cloned().unwrap_or_default() != first)
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
