use super::*;

pub fn raw_to_tags(raw: &[(String, TagValue)]) -> HashMap<FrameKey, Vec<TagValue>> {
    let mut result: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();

    for (k, v) in raw.iter() {
        if let Some(&key) = ITUNES_REVERSE_MAP.get(k.as_str()) {
            result.entry(key).or_default().push(v.clone());
            continue;
        }
        if let Some(rest) = k.strip_prefix("----:")
            && let Some((mean, name)) = rest.split_once(':')
        {
            if let Some(&fk) = FREEFORM_REVERSE_MAP.get(&(mean, name)) {
                result.entry(fk).or_default().push(v.clone());
                continue;
            }
            {
                use FrameKey::UserDefinedText;
                if let TagValue::Text(s) = v {
                    result
                        .entry(UserDefinedText)
                        .or_default()
                        .push(TagValue::UserText(tag_manager::utils::UserTextEntry {
                            description: if mean == "com.apple.iTunes" {
                                name.to_string()
                            } else {
                                k.clone()
                            },
                            value: s.clone(),
                        }));
                }
                continue;
            }
        }
    }

    result
}

pub(crate) fn parse_number_pair(text: &str) -> Option<(u16, Option<u16>)> {
    let (number, total) = match text.split_once('/') {
        Some((number, total)) => (number, Some(total.trim().parse::<u16>().ok()?)),
        None => (text, None),
    };
    Some((number.trim().parse::<u16>().ok()?, total))
}

pub(crate) fn custom_key(name: &str) -> String {
    if name.starts_with("----:") {
        name.to_string()
    } else {
        format!("----:com.apple.iTunes:{name}")
    }
}
