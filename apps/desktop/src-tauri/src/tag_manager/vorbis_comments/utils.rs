// Generic utils for handling vorbis for multiple audio formats
use crate::tag_manager::utils::{FrameKey, TagValue};
use base64::{engine::general_purpose as b64_gp, Engine as _};
use once_cell::sync::Lazy;

use std::collections::HashMap;
use std::io::Error;

pub fn vorbis_code(key: FrameKey) -> &'static str {
    match key {
        FrameKey::AttachedPicture => "METADATA_BLOCK_PICTURE",
        FrameKey::Title => "TITLE",
        FrameKey::Artist => "ARTIST",
        FrameKey::Album => "ALBUM",
        FrameKey::AlbumArtist => "ALBUMARTIST",
        FrameKey::Year => "DATE",
        FrameKey::RecordingDate => "DATE",
        FrameKey::ReleaseDate => "ORIGINALDATE",
        FrameKey::TrackNumber => "TRACKNUMBER",
        FrameKey::TotalTracks => "TRACKTOTAL",
        FrameKey::DiscNumber => "DISCNUMBER",
        FrameKey::TotalDiscs => "DISCTOTAL",
        FrameKey::Genre => "GENRE",
        FrameKey::ContentGroup => "GROUPING",
        FrameKey::Composer => "COMPOSER",
        FrameKey::EncodedBy => "ENCODER",
        FrameKey::UnsyncedLyrics => "LYRICS",
        FrameKey::SynchronizedLyrics => "SYNCEDLYRICS",
        FrameKey::Comments => "COMMENT",
        FrameKey::Conductor => "CONDUCTOR",
        FrameKey::BeatsPerMinute => "BPM",
        FrameKey::Language => "LANGUAGE",
        FrameKey::UserDefinedURL => "URL",

        _ => "",
    }
}

fn normalize_vorbis_key(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

pub static VORBIS_REVERSE_MAP: Lazy<HashMap<&'static str, FrameKey>> = Lazy::new(|| {
    let mut map = HashMap::new();

    let mappings: [(&'static str, FrameKey); 19] = [
        ("TITLE", FrameKey::Title),
        ("ARTIST", FrameKey::Artist),
        ("ALBUM", FrameKey::Album),
        ("ALBUMARTIST", FrameKey::AlbumArtist),
        ("ALBUM ARTIST", FrameKey::AlbumArtist),
        ("DATE", FrameKey::Year),
        ("YEAR", FrameKey::Year),
        ("ORIGINALDATE", FrameKey::ReleaseDate),
        ("TRACKNUMBER", FrameKey::TrackNumber),
        ("TRACK", FrameKey::TrackNumber),
        ("GENRE", FrameKey::Genre),
        ("GROUPING", FrameKey::ContentGroup),
        ("COMPOSER", FrameKey::Composer),
        ("ENCODER", FrameKey::EncodedBy),
        ("LYRICS", FrameKey::UnsyncedLyrics),
        ("SYNCEDLYRICS", FrameKey::SynchronizedLyrics),
        ("COMMENT", FrameKey::Comments),
        ("CONDUCTOR", FrameKey::Conductor),
        ("BPM", FrameKey::BeatsPerMinute),
    ];

    for (k, v) in mappings {
        map.insert(k, v);
    }
    map
});

pub fn raw_to_tags(raw: &HashMap<String, Vec<TagValue>>) -> HashMap<FrameKey, Vec<TagValue>> {
    let mut result: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();

    for (raw_key, values) in raw.iter() {
        let norm = normalize_vorbis_key(raw_key);
        let key_opt = VORBIS_REVERSE_MAP
            .get(norm.as_str())
            .copied()
            .or(match norm.as_str() {
                "TRACKTOTAL" | "TOTALTRACKS" => Some(FrameKey::TotalTracks),
                "DISCTOTAL" | "TOTALDISCS" => Some(FrameKey::TotalDiscs),
                "DISCNUMBER" => Some(FrameKey::DiscNumber),
                "LABEL" => Some(FrameKey::Label),
                "ISRC" => Some(FrameKey::Isrc),
                "METADATA_BLOCK_PICTURE" => Some(FrameKey::AttachedPicture),

                _ => None,
            });

        let Some(frame_key) = key_opt else {
            continue;
        };

        let mut expanded: Vec<TagValue> = Vec::new();
        for v in values.iter() {
            match v {
                TagValue::Text(s)
                    if matches!(frame_key, FrameKey::Artist | FrameKey::Genre)
                        && (s.contains('\\') || s.contains('\u{0}')) =>
                {
                    expanded.push(TagValue::Text(s.clone()));
                }
                _ => expanded.push(v.clone()),
            }
        }

        result.entry(frame_key).or_default().extend(expanded);
    }

    result
}

pub fn parse_comments(data: &[u8]) -> Result<HashMap<FrameKey, Vec<TagValue>>, Error> {
    let mut offset = 0usize;

    let vendor_length = read_u32_le(data, &mut offset)? as usize;
    if offset + vendor_length > data.len() {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Vendor string exceeds buffer",
        ));
    }
    offset += vendor_length;

    let comment_count = read_u32_le(data, &mut offset)? as usize;

    let mut raw: HashMap<String, Vec<TagValue>> = HashMap::new();
    for _ in 0..comment_count {
        let comment_len = read_u32_le(data, &mut offset)? as usize;
        if offset + comment_len > data.len() {
            return Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Comment exceeds buffer",
            ));
        }

        let comment_bytes = &data[offset..offset + comment_len];
        offset += comment_len;

        let comment_str = String::from_utf8(comment_bytes.to_vec()).map_err(|_| {
            Error::new(
                std::io::ErrorKind::InvalidData,
                "Comment is not valid UTF-8",
            )
        })?;

        let Some(eq_pos) = comment_str.find('=') else {
            continue;
        };
        let (k, v_with_eq) = comment_str.split_at(eq_pos);
        let v = &v_with_eq[1..];

        let norm_key = normalize_vorbis_key(k);
        if norm_key == "METADATA_BLOCK_PICTURE" {
            let pic_data = b64_gp::STANDARD.decode(v).map_err(|_| {
                Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to decode base64 picture data",
                )
            })?;
            let pic_tag = parse_picture(&pic_data)?;
            raw.entry(norm_key).or_default().push(pic_tag);
            continue;
        }
        raw.entry(norm_key)
            .or_default()
            .push(TagValue::Text(v.to_string()));
    }

    Ok(raw_to_tags(&raw))
}
pub fn parse_picture(buf: &[u8]) -> Result<TagValue, Error> {
    let mut offset = 0;

    let read_u32 = |buf: &[u8], offset: &mut usize| -> Result<u32, Error> {
        if *offset + 4 > buf.len() {
            return Err(Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected EOF while reading picture u32",
            ));
        }
        let bytes: [u8; 4] = buf[*offset..*offset + 4].try_into().unwrap();
        *offset += 4;
        Ok(u32::from_be_bytes(bytes))
    };

    let picture_type = read_u32(buf, &mut offset)?;

    let mime_len = read_u32(buf, &mut offset)? as usize;
    if offset + mime_len > buf.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected EOF while reading picture mime",
        ));
    }
    let mime = String::from_utf8(buf[offset..offset + mime_len].to_vec());
    if mime.is_err() {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Picture mime is not valid UTF-8",
        ));
    }
    let mime = mime.unwrap();
    offset += mime_len;

    let desc_len = read_u32(buf, &mut offset)? as usize;
    if offset + desc_len > buf.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected EOF while reading picture description",
        ));
    }
    let description = String::from_utf8(buf[offset..offset + desc_len].to_vec());
    if description.is_err() {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Picture description is not valid UTF-8",
        ));
    }
    let description = description.unwrap();
    offset += desc_len;

    // useless
    let _width = read_u32(buf, &mut offset)?;
    let _height = read_u32(buf, &mut offset)?;
    let _depth = read_u32(buf, &mut offset)?;
    let _indexed_colors = read_u32(buf, &mut offset)?;

    let data_len = read_u32(buf, &mut offset)? as usize;
    if offset + data_len > buf.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected EOF while reading picture data",
        ));
    }
    let data = buf[offset..offset + data_len].to_vec();
    let pt = u8::try_from(picture_type);
    if pt.is_err() {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Picture type out of range",
        ));
    }
    let picture_type = pt.unwrap();

    Ok(TagValue::Picture {
        picture_type: Some(picture_type),
        mime,
        description: Some(description),
        data,
    })
}

pub fn build_comments(tags: &HashMap<FrameKey, Vec<TagValue>>, needs_picture: bool) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let vendor_string = "Audexis"; // cheeky boy
    let vendor_bytes = vendor_string.as_bytes();
    let vendor_length = vendor_bytes.len() as u32;
    out.extend(&vendor_length.to_le_bytes());
    out.extend(vendor_bytes);

    let mut comment_list: Vec<Vec<u8>> = Vec::new();

    for (key, values) in tags.iter() {
        let vorbis_key = vorbis_code(*key);
        for value in values.iter() {
            if let TagValue::Text(text) = value {
                let comment_str = format!("{}={}", vorbis_key, text);
                let comment_bytes = comment_str.as_bytes();
                let comment_length = comment_bytes.len() as u32;

                let mut comment_entry: Vec<u8> = Vec::new();
                comment_entry.extend(&comment_length.to_le_bytes());
                comment_entry.extend(comment_bytes);

                comment_list.push(comment_entry);
            } else if let TagValue::Comment {
                text,
                encoding: _,
                language: _,
                description: _,
            } = value
            {
                let comment_str = format!("{}={}", vorbis_key, text);
                let comment_bytes = comment_str.as_bytes();
                let comment_length = comment_bytes.len() as u32;

                let mut comment_entry: Vec<u8> = Vec::new();
                comment_entry.extend(&comment_length.to_le_bytes());
                comment_entry.extend(comment_bytes);

                comment_list.push(comment_entry);
            } else if needs_picture {
                if let TagValue::Picture {
                    picture_type: _,
                    mime: _,
                    description: _,
                    data: _,
                } = value
                {
                    let pic_comment = build_picture_comment(value);
                    if let Some(pic_bytes) = pic_comment {
                        let encoded = b64_gp::STANDARD.encode(&pic_bytes);
                        let comment_str = format!("METADATA_BLOCK_PICTURE={}", encoded);
                        let comment_bytes = comment_str.as_bytes();
                        let comment_length = comment_bytes.len() as u32;
                        let mut comment_entry: Vec<u8> = Vec::new();
                        comment_entry.extend(&comment_length.to_le_bytes());
                        comment_entry.extend(comment_bytes);
                        comment_list.push(comment_entry);
                    }
                }
            }
        }
    }

    let comment_count = comment_list.len() as u32;
    out.extend(&comment_count.to_le_bytes());

    for comment in comment_list {
        out.extend(comment);
    }

    out
}
pub fn build_picture_comment(tag: &TagValue) -> Option<Vec<u8>> {
    if let TagValue::Picture {
        picture_type,
        mime,
        description,
        data,
    } = tag
    {
        let mut out = Vec::new();

        out.extend(&(picture_type.unwrap_or(3) as u32).to_be_bytes());

        let mime_bytes = mime.as_bytes();
        out.extend(&(mime_bytes.len() as u32).to_be_bytes());
        out.extend(mime_bytes);

        let desc_bytes = description.as_deref().unwrap_or("").as_bytes();
        out.extend(&(desc_bytes.len() as u32).to_be_bytes());
        out.extend(desc_bytes);

        out.extend(&0u32.to_be_bytes());
        out.extend(&0u32.to_be_bytes());
        out.extend(&0u32.to_be_bytes());
        out.extend(&0u32.to_be_bytes());

        out.extend(&(data.len() as u32).to_be_bytes());
        out.extend(data);

        Some(out)
    } else {
        None
    }
}
pub fn build_picture_tag(tags: &HashMap<FrameKey, Vec<TagValue>>) -> Vec<Vec<u8>> {
    let mut pictures = Vec::new();

    for values in tags.values() {
        for value in values {
            if let TagValue::Picture {
                picture_type,
                mime,
                description,
                data,
            } = value
            {
                let mut out = Vec::new();

                out.extend(&(picture_type.unwrap_or(3) as u32).to_be_bytes());

                let mime_bytes = mime.as_bytes();
                out.extend(&(mime_bytes.len() as u32).to_be_bytes());
                out.extend(mime_bytes);

                let desc_bytes = description.as_deref().unwrap_or("").as_bytes();
                out.extend(&(desc_bytes.len() as u32).to_be_bytes());
                out.extend(desc_bytes);

                out.extend(&0u32.to_be_bytes());
                out.extend(&0u32.to_be_bytes());
                out.extend(&0u32.to_be_bytes());
                out.extend(&0u32.to_be_bytes());

                out.extend(&(data.len() as u32).to_be_bytes());
                out.extend(data);

                pictures.push(out);
            }
        }
    }

    pictures
}
fn read_u32_le(buf: &[u8], offset: &mut usize) -> Result<u32, Error> {
    if *offset + 4 > buf.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected EOF while reading Vorbis comment u32",
        ));
    }
    let b0 = buf[*offset] as u32;
    let b1 = buf[*offset + 1] as u32;
    let b2 = buf[*offset + 2] as u32;
    let b3 = buf[*offset + 3] as u32;

    *offset += 4;

    Ok(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
}

pub fn update_comments(
    original: &[u8],
    changes: &HashMap<FrameKey, Vec<TagValue>>,
    needs_picture: bool,
) -> Result<Vec<u8>, Error> {
    if changes.keys().any(|key| vorbis_code(*key).is_empty()) {
        return Err(Error::new(
            std::io::ErrorKind::InvalidInput,
            "Unsupported Vorbis metadata field",
        ));
    }
    fn entries(data: &[u8]) -> Result<(Vec<u8>, Vec<Vec<u8>>), Error> {
        let mut offset = 0;
        let vendor_len = read_u32_le(data, &mut offset)? as usize;
        let vendor = data
            .get(offset..offset + vendor_len)
            .ok_or_else(|| {
                Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid comment vendor length",
                )
            })?
            .to_vec();
        offset += vendor_len;
        let count = read_u32_le(data, &mut offset)?;
        let mut values = Vec::new();
        for _ in 0..count {
            let length = read_u32_le(data, &mut offset)? as usize;
            values.push(
                data.get(offset..offset + length)
                    .ok_or_else(|| {
                        Error::new(std::io::ErrorKind::InvalidData, "Invalid comment length")
                    })?
                    .to_vec(),
            );
            offset += length;
        }
        Ok((vendor, values))
    }
    let (vendor, mut comments) = entries(original)?;
    comments.retain(|comment| {
        let Some(equals) = comment.iter().position(|byte| *byte == b'=') else {
            return true;
        };
        let key = String::from_utf8_lossy(&comment[..equals]).to_ascii_uppercase();
        let mapped = raw_to_tags(&HashMap::from([(key.clone(), Vec::new())]));
        !changes
            .keys()
            .any(|changed| mapped.contains_key(changed) || vorbis_code(*changed) == key)
    });
    comments.extend(entries(&build_comments(changes, needs_picture))?.1);
    let mut result = (vendor.len() as u32).to_le_bytes().to_vec();
    result.extend(vendor);
    result.extend((comments.len() as u32).to_le_bytes());
    for comment in comments {
        result.extend((comment.len() as u32).to_le_bytes());
        result.extend(comment);
    }
    Ok(result)
}
