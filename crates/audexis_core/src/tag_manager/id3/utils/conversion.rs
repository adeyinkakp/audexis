use super::*;

pub fn raw_to_tags(raw: &HashMap<String, Vec<TagValue>>) -> HashMap<FrameKey, Vec<TagValue>> {
    let mut result: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
    for (id, values) in raw.iter() {
        if let Some(key) = ID3V23_REVERSE_MAP.get(id.as_str()) {
            let mut expanded: Vec<TagValue> = Vec::new();
            for v in values.iter() {
                match v {
                    TagValue::Text(s) if key.is_multi_valued() && s.contains('\0') => {
                        for part in s.split('\0') {
                            let seg = part.trim();
                            if !seg.is_empty() {
                                expanded.push(TagValue::Text(seg.to_string()));
                            }
                        }
                    }
                    _ => expanded.push(v.clone()),
                }
            }
            result.entry(*key).or_default().extend(expanded);
        }
    }
    result
}
pub fn tags_to_raw(tags: &HashMap<FrameKey, TagValue>) -> HashMap<&'static str, TagValue> {
    let mut out: HashMap<&'static str, TagValue> = HashMap::new();
    for (key, value) in tags.iter() {
        let id = match key {
            FrameKey::Artists => id3v23_code(FrameKey::Artist),
            _ => id3v23_code(*key),
        };
        out.insert(id, value.clone());
    }
    out
}
pub fn id3v22_raw_to_tags(
    raw: &HashMap<String, Vec<TagValue>>,
) -> HashMap<FrameKey, Vec<TagValue>> {
    let mut result: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
    for (id, values) in raw.iter() {
        if let Some(k) = ID3V22_REVERSE_MAP.get(id.as_str()) {
            result.entry(*k).or_default().extend(values.clone());
        }
    }
    result
}

pub fn id3v22_tags_to_raw(tags: &HashMap<FrameKey, TagValue>) -> HashMap<&'static str, TagValue> {
    tags.iter()
        .map(|(k, v)| (id3v22_code(*k), v.clone()))
        .collect()
}

pub fn id3v24_raw_to_tags(
    raw: &HashMap<String, Vec<TagValue>>,
) -> HashMap<FrameKey, Vec<TagValue>> {
    let mut result: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
    for (id, values) in raw.iter() {
        let key_opt = ID3V24_REVERSE_MAP.get(id.as_str()).cloned().or_else(|| {
            if id == "TYER" {
                Some(FrameKey::Year)
            } else {
                None
            }
        });
        if let Some(k) = key_opt {
            let mut expanded: Vec<TagValue> = Vec::new();
            for v in values.iter() {
                match v {
                    TagValue::Text(s) if k.is_multi_valued() && s.contains('\u{0}') => {
                        for part in s.split('\u{0}') {
                            let seg = part.trim();
                            if !seg.is_empty() {
                                expanded.push(TagValue::Text(seg.to_string()));
                            }
                        }
                    }
                    _ => expanded.push(v.clone()),
                }
            }
            result.entry(k).or_default().extend(expanded);
        }
    }
    result
}

pub fn id3v24_tags_to_raw(tags: &HashMap<FrameKey, TagValue>) -> HashMap<&'static str, TagValue> {
    tags.iter()
        .map(|(k, v)| (id3v24_code(*k), v.clone()))
        .collect()
}
