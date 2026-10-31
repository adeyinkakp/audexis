use super::*;

impl V0 {
    pub(super) fn write_metadata(
        &self,
        file_path: &std::path::Path,
        updated_tags: HashMap<tag_manager::utils::FrameKey, Vec<TagValue>>,
    ) -> Result<(), BackendError> {
        let old_vec = self.get_tags(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read tags from file".to_string(),
                internal_message: "Failed to read tags from file".to_string(),
            })
        })?;
        let mut updated_entries: Vec<(String, TagValue)> = Vec::new();
        let mut updated_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

        for key in updated_tags.keys() {
            let code = tag_manager::itunes::utils::itunes_code(*key);
            if code == "----" {
                if let Some(spec) = tag_manager::itunes::utils::itunes_freeform_spec(*key) {
                    updated_keys.insert(format!("----:{}:{}", spec.mean, spec.name));
                }
            } else {
                updated_keys.insert(code.to_string());
            }
        }
        if updated_tags.contains_key(&tag_manager::utils::FrameKey::UserDefinedText) {
            for value in old_vec
                .get(&tag_manager::utils::FrameKey::UserDefinedText)
                .into_iter()
                .flatten()
            {
                if let TagValue::UserText(entry) = value {
                    updated_keys.insert(tag_manager::itunes::utils::custom_key(&entry.description));
                }
            }
        }
        let mut push_key_once = |key: &String| {
            updated_keys.insert(key.clone());
        };

        for (k, vals) in updated_tags.iter() {
            match k {
                tag_manager::utils::FrameKey::UserDefinedText => {
                    for v in vals {
                        if let TagValue::UserText(ut) = v {
                            let key = tag_manager::itunes::utils::custom_key(&ut.description);
                            updated_entries.push((key.clone(), TagValue::Text(ut.value.clone())));
                            push_key_once(&key);
                        } else if let TagValue::Text(s) = v {
                            let key = format!("----:{}:{}", "com.apple.iTunes", "TXXX");
                            updated_entries.push((key.clone(), TagValue::Text(s.clone())));
                            push_key_once(&key);
                        }
                    }
                }
                tag_manager::utils::FrameKey::UserDefinedURL => {
                    for v in vals {
                        if let TagValue::UserUrl(u) = v {
                            let key = format!(
                                "----:{}:{}",
                                "com.apple.iTunes",
                                u.description.replace(" ", "_")
                            );
                            updated_entries.push((key.clone(), TagValue::Text(u.url.clone())));
                            push_key_once(&key);
                        }
                    }
                }
                tag_manager::utils::FrameKey::AttachedPicture => {
                    let mut any = false;
                    for v in vals {
                        if let TagValue::Picture { .. } = v {
                            updated_entries.push(("covr".to_string(), v.clone()));
                            any = true;
                        }
                    }
                    if any {
                        push_key_once(&"covr".to_string());
                    }
                }
                FrameKey::TrackNumber | FrameKey::DiscNumber => {
                    if vals.is_empty() {
                        continue;
                    }
                    let parsed = match vals.as_slice() {
                        [TagValue::Text(text)] => parse_number_pair(text),
                        _ => None,
                    };
                    let (number, total) = parsed.ok_or_else(|| {
                        BackendError::WriteFailed(TagError {
                            path: file_path.to_string_lossy().into_owned(),
                            public_message: "Invalid track or disc number".into(),
                            internal_message:
                                "Expected a number or number/total between 0 and 65535".into(),
                        })
                    })?;
                    let previous_total =
                        old_vec
                            .get(k)
                            .and_then(|values| values.first())
                            .and_then(|value| {
                                if let TagValue::Text(text) = value {
                                    parse_number_pair(text).and_then(|(_, total)| total)
                                } else {
                                    None
                                }
                            });
                    let text = match total.or(previous_total) {
                        Some(total) if total > 0 => format!("{number}/{total}"),
                        _ => number.to_string(),
                    };
                    updated_entries.push((
                        tag_manager::itunes::utils::itunes_code(*k).into(),
                        TagValue::Text(text),
                    ));
                }
                key if get_atom_flag(tag_manager::itunes::utils::itunes_code(*key))
                    .is_some_and(|flag| flag.flag[3] == 1) =>
                {
                    for value in vals {
                        updated_entries.push((
                            tag_manager::itunes::utils::itunes_code(*k).to_string(),
                            value.clone(),
                        ));
                    }
                }
                other => {
                    let code = tag_manager::itunes::utils::itunes_code(*other);
                    if code == "----" {
                        if let Some(spec) = tag_manager::itunes::utils::itunes_freeform_spec(*other)
                        {
                            for first in vals.iter().filter(|v| matches!(v, TagValue::Text(_))) {
                                let key = format!("----:{}:{}", spec.mean, spec.name);
                                updated_entries.push((key.clone(), first.clone()));
                                push_key_once(&key);
                            }
                        }
                    } else {
                        let text_joined = vals
                            .iter()
                            .filter_map(|v| match v {
                                TagValue::Text(s) => Some(s.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\\");
                        let key = code.to_string();
                        if !text_joined.is_empty() {
                            updated_entries.push((key.clone(), TagValue::Text(text_joined)));
                            push_key_once(&key);
                        } else if let Some(first) = vals.first() {
                            updated_entries.push((key.clone(), first.clone()));
                            push_key_once(&key);
                        }
                    }
                }
            }
        }

        let all_entries = updated_entries;

        let buffer = crate::library_files::read(file_path).map_err(|_| {
            BackendError::ReadFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to read file".to_string(),
                internal_message: "Failed to read file".to_string(),
            })
        })?;
        atoms::validate_for_write(&buffer).map_err(|message| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_string_lossy().into_owned(),
                public_message: "Cannot safely edit this MP4 file".into(),
                internal_message: message,
            })
        })?;
        let ilst_atom = V0::ensure_ilst_atom(&buffer);

        let rebuilt_file = if let Ok(ilst_atom) = ilst_atom {
            let ilst_sub_atoms = V0::parse_atoms(&ilst_atom.buffer, 8, ilst_atom.size);
            let updated_ilst_buffer = V0::encode_ilst(
                all_entries,
                ilst_sub_atoms,
                updated_keys.iter().cloned().collect(),
            );
            V0::rebuild_file(updated_ilst_buffer, &buffer)
                .ok_or(())
                .map_err(|_| {
                    BackendError::WriteFailed(TagError {
                        path: file_path.to_str().unwrap_or("").to_string(),
                        public_message: "Failed to write tags to file".to_string(),
                        internal_message: "Failed to write tags to file".to_string(),
                    })
                })?
        } else {
            let updated_ilst_buffer = V0::encode_ilst(
                all_entries,
                Vec::new(),
                updated_keys.iter().cloned().collect(),
            );
            V0::rebuild_file_insert_ilst(updated_ilst_buffer, &buffer).ok_or_else(|| {
                BackendError::WriteFailed(TagError {
                    path: file_path.to_str().unwrap_or("").to_string(),
                    public_message: "Failed to write tags to file".to_string(),
                    internal_message: "Failed to insert ilst atom".to_string(),
                })
            })?
        };

        fs::write(file_path, &rebuilt_file).map_err(|_| {
            BackendError::WriteFailed(TagError {
                path: file_path.to_str().unwrap_or("").to_string(),
                public_message: "Failed to write tags to file".to_string(),
                internal_message: "Failed to write tags to file".to_string(),
            })
        })?;

        Ok(())
    }
}
