use super::*;

// string to frame key
pub struct TagValuesWrapper(pub Vec<TagValue>);
pub struct SerializableTagValuesWrapper(pub Vec<SerializableTagValue>);

impl From<TagValuesWrapper> for Vec<SerializableTagValue> {
    fn from(values: TagValuesWrapper) -> Vec<SerializableTagValue> {
        let e: Vec<SerializableTagValue> = values
            .0
            .into_iter()
            .map(|value| match value {
                TagValue::Text(s) => SerializableTagValue::Text(s),
                TagValue::Picture {
                    mime,
                    data,
                    picture_type,
                    description,
                } => {
                    use base64::{Engine as _, engine::general_purpose::STANDARD};

                    let data_base64 = STANDARD.encode(data);

                    SerializableTagValue::Picture {
                        mime,
                        data_base64,
                        picture_type,
                        description,
                    }
                }
                TagValue::UserText(entry) => SerializableTagValue::UserText(entry),
                TagValue::UserUrl(entry) => SerializableTagValue::UserUrl(entry),
                TagValue::Comment {
                    encoding,
                    language,
                    description,
                    text,
                } => SerializableTagValue::Comment {
                    encoding,
                    language,
                    description,
                    text,
                },
            })
            .collect();
        e
    }
}

impl From<SerializableTagValuesWrapper> for Vec<TagValue> {
    fn from(values: SerializableTagValuesWrapper) -> Vec<TagValue> {
        values
            .0
            .iter()
            .map(|value| match value {
                SerializableTagValue::Text(s) => TagValue::Text(s.clone()),
                SerializableTagValue::Picture {
                    mime,
                    data_base64,
                    picture_type,
                    description,
                } => {
                    use base64::{Engine as _, engine::general_purpose::STANDARD};
                    let mime = mime.clone();

                    let data = STANDARD.decode(data_base64);
                    let data = data.unwrap_or_default();
                    let picture_type = *picture_type;
                    let description = description.clone();
                    TagValue::Picture {
                        mime,
                        data,
                        picture_type,
                        description,
                    }
                }
                SerializableTagValue::UserText(entry) => TagValue::UserText(entry.clone()),
                SerializableTagValue::UserUrl(entry) => TagValue::UserUrl(entry.clone()),
                SerializableTagValue::Comment {
                    encoding,
                    language,
                    description,
                    text,
                } => TagValue::Comment {
                    encoding: encoding.to_owned(),
                    language: language.to_owned(),
                    description: description.to_owned(),
                    text: text.to_owned(),
                },
            })
            .collect()
    }
}

/// Internal File struct to file that can be sent via ipc
impl From<MetadataFile> for SerializableFile {
    fn from(file: MetadataFile) -> Self {
        let mut tags: HashMap<String, Vec<SerializableTagValue>> = HashMap::new();
        for (k, vals) in file.tags.into_iter() {
            let key = k.to_string();
            let mut out_vals: Vec<SerializableTagValue> = Vec::new();

            for v in vals {
                match v {
                    TagValue::Text(s) => out_vals.push(SerializableTagValue::Text(s)),
                    TagValue::Picture {
                        mime,
                        data,
                        picture_type,
                        description,
                    } => out_vals.push(SerializableTagValue::Picture {
                        mime,
                        data_base64: general_purpose::STANDARD.encode(data),
                        picture_type,
                        description,
                    }),
                    TagValue::UserText(item) => out_vals.push(SerializableTagValue::UserText(item)),
                    TagValue::UserUrl(item) => out_vals.push(SerializableTagValue::UserUrl(item)),
                    TagValue::Comment {
                        encoding,
                        language,
                        description,
                        text,
                    } => out_vals.push(SerializableTagValue::Comment {
                        encoding,
                        language,
                        description,
                        text,
                    }),
                }
            }
            tags.insert(key, out_vals);
        }

        SerializableFile {
            path: file.path.display().to_string(),
            tag_format: file.tag_format.to_string(),
            file_name: file
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            tag_formats: file
                .tag_formats
                .into_iter()
                .map(|f| f.to_string())
                .collect(),
            tags,
            freeforms: file
                .freeforms
                .into_iter()
                .map(|ff| SerializableFreeform {
                    mean: ff.mean,
                    name: ff.name,
                    value: ff.value,
                })
                .collect(),
        }
    }
}
