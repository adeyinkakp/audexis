use super::v2_common::encode_text_payload;

pub(crate) fn parse_lrc(text: &str) -> Result<Vec<(u32, String)>, String> {
    let mut entries = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let error = || format!("Line {} must start with [mm:ss.xxx]", index + 1);
        let (stamp, lyric) = line
            .strip_prefix('[')
            .and_then(|s| s.split_once(']'))
            .ok_or_else(error)?;
        let (minutes, seconds) = stamp.split_once(':').ok_or_else(error)?;
        let (seconds, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
        if minutes.is_empty()
            || seconds.len() != 2
            || fraction.len() > 3
            || !minutes
                .bytes()
                .chain(seconds.bytes())
                .chain(fraction.bytes())
                .all(|b| b.is_ascii_digit())
        {
            return Err(error());
        }
        let minutes: u32 = minutes.parse().map_err(|_| error())?;
        let seconds: u32 = seconds.parse().map_err(|_| error())?;
        if seconds >= 60 || lyric.contains('\0') {
            return Err(error());
        }
        let millis = if fraction.is_empty() {
            0
        } else {
            fraction.parse::<u32>().map_err(|_| error())? * 10u32.pow(3 - fraction.len() as u32)
        };
        let time = minutes
            .checked_mul(60_000)
            .and_then(|t| t.checked_add(seconds * 1000 + millis))
            .ok_or_else(error)?;
        if entries.last().is_some_and(|(previous, _)| *previous > time) {
            return Err(format!(
                "Line {} has a timestamp earlier than the previous line",
                index + 1
            ));
        }
        entries.push((time, lyric.to_string()));
    }
    Ok(entries)
}

pub(crate) fn format_lrc(entries: &[(u32, String)]) -> String {
    entries
        .iter()
        .map(|(ms, text)| {
            format!(
                "[{:02}:{:02}.{:03}]{}",
                ms / 60_000,
                ms / 1000 % 60,
                ms % 1000,
                text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn encode(id: &str, text: &str) -> Result<Vec<u8>, String> {
    let synced = matches!(id, "SYLT" | "SLT");
    let mut payload = vec![1, b'u', b'n', b'd'];
    if synced {
        payload.extend_from_slice(&[2, 1]);
    }
    payload.extend_from_slice(&[0xff, 0xfe, 0, 0]);
    if synced {
        for (time, lyric) in parse_lrc(text)? {
            payload.extend_from_slice(&encode_text_payload(&lyric, true)[1..]);
            payload.extend_from_slice(&[0, 0]);
            payload.extend_from_slice(&time.to_be_bytes());
        }
    } else {
        if text.contains('\0') {
            return Err("Lyrics cannot contain NUL characters".into());
        }
        payload.extend_from_slice(&encode_text_payload(text, true)[1..]);
    }
    Ok(payload)
}

fn terminated(encoding: u8, bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let width = if encoding == 1 || encoding == 2 { 2 } else { 1 };
    let end = (0..bytes.len()).step_by(width).find(|&i| {
        bytes
            .get(i..i + width)
            .is_some_and(|s| s.iter().all(|b| *b == 0))
    })?;
    Some((&bytes[..end], &bytes[end + width..]))
}

fn decode_text(encoding: u8, bytes: &[u8], big_endian: bool) -> String {
    if encoding == 1 || encoding == 2 {
        let be =
            bytes.starts_with(&[0xfe, 0xff]) || (!bytes.starts_with(&[0xff, 0xfe]) && big_endian);
        let bytes = bytes
            .strip_prefix(&[0xff, 0xfe])
            .or_else(|| bytes.strip_prefix(&[0xfe, 0xff]))
            .unwrap_or(bytes);
        String::from_utf16_lossy(
            &bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| {
                    if be {
                        u16::from_be_bytes([p[0], p[1]])
                    } else {
                        u16::from_le_bytes([p[0], p[1]])
                    }
                })
                .collect::<Vec<_>>(),
        )
    } else if encoding == 0 {
        bytes.iter().map(|b| char::from(*b)).collect()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub(crate) fn decode(id: &str, payload: &[u8]) -> Option<String> {
    let synced = matches!(id, "SYLT" | "SLT");
    let encoding = *payload.first()?;
    if encoding > 3 {
        return None;
    }
    if synced && (*payload.get(4)? != 2 || *payload.get(5)? != 1) {
        return Some(
            "[Unsupported synchronized lyrics: only millisecond lyric timestamps can be edited]"
                .into(),
        );
    }
    let (description, mut rest) = terminated(encoding, payload.get(if synced { 6 } else { 4 }..)?)?;
    let be = encoding == 2 || description.starts_with(&[0xfe, 0xff]);
    if !synced {
        return Some(
            decode_text(encoding, rest, be)
                .trim_end_matches('\0')
                .to_string(),
        );
    }
    let mut entries = Vec::new();
    while !rest.is_empty() {
        let (text, tail) = terminated(encoding, rest)?;
        let time = u32::from_be_bytes(tail.get(..4)?.try_into().ok()?);
        entries.push((time, decode_text(encoding, text, be)));
        rest = &tail[4..];
    }
    Some(format_lrc(&entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes_lrc_without_losing_whitespace() {
        let entries = parse_lrc("[00:01.5] leading space\n[01:02]\n").unwrap();
        assert_eq!(
            entries,
            vec![(1500, " leading space".into()), (62000, "".into())]
        );
        assert_eq!(
            format_lrc(&entries),
            "[00:01.500] leading space\n[01:02.000]"
        );
        for invalid in [
            "hello",
            "[00:60]bad",
            "[999999999999:00]overflow",
            "[00:02]a\n[00:01]b",
            "[00:01.1234]bad",
            "[00:01]a\0b",
        ] {
            assert!(parse_lrc(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn reads_independent_latin1_and_big_endian_utf16_payloads() {
        assert_eq!(
            decode("USLT", b"\0engdescription\0caf\xe9\nline"),
            Some("café\nline".into())
        );
        assert_eq!(
            decode("USLT", b"\x01eng\xfe\xff\0\0\xfe\xff\0H\0i"),
            Some("Hi".into())
        );
        assert_eq!(
            decode("SYLT", b"\0eng\x02\x01\0hello\0\0\0\x03\xe8"),
            Some("[00:01.000]hello".into())
        );
        assert_eq!(decode("SYLT", b"\0eng\x02\x01\0hello\0\0"), None);
    }
}
