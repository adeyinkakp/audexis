use crate::tag_manager::utils::FrameKey;
use crate::tag_manager::utils::TagValue;
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub fn id3v22_code(key: FrameKey) -> &'static str {
    match key {
        FrameKey::Title => "TT2",
        FrameKey::Artist => "TP1",
        FrameKey::Album => "TAL",
        FrameKey::Year => "TYE",
        FrameKey::TrackNumber => "TRK",
        FrameKey::Genre => "TCO",
        FrameKey::AlbumArtist => "TP2",
        FrameKey::ContentGroup => "TT1",
        FrameKey::Composer => "TCM",
        FrameKey::EncodedBy => "TEN",
        FrameKey::SoftwareEncoder => "TSS",
        FrameKey::Length => "TLE",
        FrameKey::BeatsPerMinute => "TBP",
        FrameKey::Language => "TLA",
        FrameKey::FileType => "TFT",
        FrameKey::Time => "TIM",

        FrameKey::RecordingDate => "TRD",
        FrameKey::ReleaseDate => "TOR",

        FrameKey::UnsyncedLyrics => "ULT",
        FrameKey::SynchronizedLyrics => "SLT",
        FrameKey::Comments => "COM",
        FrameKey::AttachedPicture => "PIC",

        FrameKey::CommercialURL => "WCM",
        FrameKey::CopyrightURL => "WCP",
        FrameKey::AudioFileURL => "WAF",
        FrameKey::ArtistURL => "WAR",
        FrameKey::UserDefinedURL => "WXX",

        FrameKey::RelativeVolumeAdjustment => "RVA",

        FrameKey::EventTimingCodes => "ETC",
        FrameKey::TempoCodes => "STC",
        FrameKey::MusicCDIdentifier => "MCI",
        FrameKey::PlayCount => "CNT",
        FrameKey::AudioSeekPointIndex => "MLL",

        FrameKey::MediaType => "TMT",

        FrameKey::GeneralObject => "GEO",
        FrameKey::AudioEncryption => "CRA",
        FrameKey::RecommendedBufferSize => "BUF",
        FrameKey::EncryptionMethod => "CRM",

        FrameKey::Private
        | FrameKey::GroupIdRegistration
        | FrameKey::CommercialFrame
        | FrameKey::SignatureFrame
        | FrameKey::AudioEncodingMethod => "TXX",

        FrameKey::RadioStationURL | FrameKey::PaymentURL | FrameKey::BitmapImageURL => "WXX",

        FrameKey::UserDefinedText => "TXX",
        _ => "TXX",
    }
}
pub fn id3v23_code(key: FrameKey) -> &'static str {
    match key {
        FrameKey::Title => "TIT2",
        FrameKey::Artist => "TPE1",
        FrameKey::Album => "TALB",
        FrameKey::Year => "TYER",
        FrameKey::TrackNumber => "TRCK",
        FrameKey::Genre => "TCON",
        FrameKey::AlbumArtist => "TPE2",
        FrameKey::ContentGroup => "TIT1",
        FrameKey::Composer => "TCOM",
        FrameKey::EncodedBy => "TENC",
        FrameKey::SoftwareEncoder => "TSSE",
        FrameKey::Length => "TLEN",
        FrameKey::BeatsPerMinute => "TBPM",
        FrameKey::Language => "TLAN",
        FrameKey::FileType => "TFLT",
        FrameKey::Time => "TIME",
        FrameKey::RecordingDate => "TRDA",
        FrameKey::ReleaseDate => "TORY",
        FrameKey::UnsyncedLyrics => "USLT",
        FrameKey::SynchronizedLyrics => "SYLT",
        FrameKey::Comments => "COMM",
        FrameKey::AttachedPicture => "APIC",
        FrameKey::CommercialURL => "WCOM",
        FrameKey::CopyrightURL => "WCOP",
        FrameKey::AudioFileURL => "WOAF",
        FrameKey::ArtistURL => "WOAR",
        FrameKey::UserDefinedURL => "WXXX",
        FrameKey::Private => "PRIV",
        FrameKey::GeneralObject => "GEOB",
        FrameKey::GroupIdRegistration => "GRID",
        FrameKey::EncryptionMethod => "ENCR",
        FrameKey::AudioEncryption => "AENC",
        FrameKey::CommercialFrame => "COMR",
        FrameKey::RelativeVolumeAdjustment => "RVAD",
        FrameKey::TempoCodes => "SYTC",
        FrameKey::EventTimingCodes => "ETCO",
        FrameKey::PlayCount => "PCNT",
        FrameKey::AudioSeekPointIndex => "MLLT",
        FrameKey::MediaType => "TMED",
        FrameKey::RadioStationURL | FrameKey::PaymentURL | FrameKey::BitmapImageURL => "WXXX",
        FrameKey::Sequence
        | FrameKey::SignatureFrame
        | FrameKey::AudioEncodingMethod
        | FrameKey::RecommendedBufferSize => "TXXX",
        FrameKey::UserDefinedText => "TXXX",
        _ => "TXXX",
    }
}

pub fn id3v24_code(key: FrameKey) -> &'static str {
    match key {
        FrameKey::RecordingDate => "TDRC",
        FrameKey::ReleaseDate => "TDOR",

        FrameKey::RelativeVolumeAdjustment => "RVA2",
        FrameKey::AudioSeekPointIndex => "ASPI",

        other => id3v23_code(other),
    }
}
pub fn id3v22_key(code: &str) -> Option<FrameKey> {
    match code {
        "TT2" => Some(FrameKey::Title),
        "TP1" => Some(FrameKey::Artist),
        "TAL" => Some(FrameKey::Album),
        "TYE" => Some(FrameKey::Year),
        "TRK" => Some(FrameKey::TrackNumber),
        "TCO" => Some(FrameKey::Genre),
        "TP2" => Some(FrameKey::AlbumArtist),
        "TIT" => Some(FrameKey::ContentGroup),
        "TCM" => Some(FrameKey::Composer),
        "TEN" => Some(FrameKey::EncodedBy),
        "ULT" => Some(FrameKey::UnsyncedLyrics),
        "TLE" => Some(FrameKey::Length),
        "TP3" => Some(FrameKey::Conductor),
        "PIC" => Some(FrameKey::AttachedPicture),
        "WXX" => Some(FrameKey::UserDefinedURL),
        "COM" => Some(FrameKey::Comments),
        "RVA" => Some(FrameKey::RelativeVolumeAdjustment),
        "WCM" => Some(FrameKey::CommercialURL),
        "WCP" => Some(FrameKey::CopyrightURL),
        "WAF" => Some(FrameKey::AudioFileURL),
        "WAR" => Some(FrameKey::ArtistURL),
        "TXX" => Some(FrameKey::UserDefinedText),
        "SLT" => Some(FrameKey::SynchronizedLyrics),
        "MCI" => Some(FrameKey::MusicCDIdentifier),
        "ETC" => Some(FrameKey::EventTimingCodes),
        "CNT" => Some(FrameKey::PlayCount),
        "TSS" => Some(FrameKey::SoftwareEncoder),
        "TBP" => Some(FrameKey::BeatsPerMinute),
        "TLA" => Some(FrameKey::Language),
        "TFT" => Some(FrameKey::FileType),
        "TIM" => Some(FrameKey::Time),
        "TRD" => Some(FrameKey::RecordingDate),
        _ => None,
    }
}
pub fn id3v23_key(code: &str) -> Option<FrameKey> {
    match code {
        "TIT2" => Some(FrameKey::Title),
        "TPE1" => Some(FrameKey::Artist),
        "TALB" => Some(FrameKey::Album),
        "TYER" => Some(FrameKey::Year),
        "TRCK" => Some(FrameKey::TrackNumber),
        "TCON" => Some(FrameKey::Genre),
        "TPE2" => Some(FrameKey::AlbumArtist),
        "TIT1" => Some(FrameKey::ContentGroup),
        "TCOM" => Some(FrameKey::Composer),
        "TENC" => Some(FrameKey::EncodedBy),
        "USLT" => Some(FrameKey::UnsyncedLyrics),
        "TLEN" => Some(FrameKey::Length),
        "TPE3" => Some(FrameKey::Conductor),
        "APIC" => Some(FrameKey::AttachedPicture),
        "WXXX" => Some(FrameKey::UserDefinedURL),
        "COMM" => Some(FrameKey::Comments),
        "PRIV" => Some(FrameKey::Private),
        "RVA2" => Some(FrameKey::RelativeVolumeAdjustment),
        "ENCR" => Some(FrameKey::EncryptionMethod),
        "GRID" => Some(FrameKey::GroupIdRegistration),
        "GEOB" => Some(FrameKey::GeneralObject),
        "WCOM" => Some(FrameKey::CommercialURL),
        "WCOP" => Some(FrameKey::CopyrightURL),
        "WOAF" => Some(FrameKey::AudioFileURL),
        "WOAR" => Some(FrameKey::ArtistURL),
        "WORS" => Some(FrameKey::RadioStationURL),
        "WPAY" => Some(FrameKey::PaymentURL),
        "WBMP" => Some(FrameKey::BitmapImageURL),
        "TXXX" => Some(FrameKey::UserDefinedText),
        "SYLT" => Some(FrameKey::SynchronizedLyrics),
        "SYTC" => Some(FrameKey::TempoCodes),
        "MCDI" => Some(FrameKey::MusicCDIdentifier),
        "ETCO" => Some(FrameKey::EventTimingCodes),
        "SEQU" => Some(FrameKey::Sequence),
        "PCNT" => Some(FrameKey::PlayCount),
        "ASPI" => Some(FrameKey::AudioSeekPointIndex),
        "STIK" => Some(FrameKey::MediaType),
        "COMR" => Some(FrameKey::CommercialFrame),
        "AENC" => Some(FrameKey::AudioEncryption),
        "SIGN" => Some(FrameKey::SignatureFrame),
        "TSSE" => Some(FrameKey::SoftwareEncoder),
        "CART" => Some(FrameKey::AudioEncodingMethod),
        "RBUF" => Some(FrameKey::RecommendedBufferSize),
        "TBPM" => Some(FrameKey::BeatsPerMinute),
        "TLAN" => Some(FrameKey::Language),
        "TFLT" => Some(FrameKey::FileType),
        "TIME" => Some(FrameKey::Time),
        "TDRC" => Some(FrameKey::RecordingDate),
        "TDOR" => Some(FrameKey::ReleaseDate),
        _ => None,
    }
}
pub fn id3v24_key(code: &str) -> Option<FrameKey> {
    match code {
        "TDRC" => Some(FrameKey::RecordingDate),
        "TDOR" => Some(FrameKey::ReleaseDate),
        other => id3v23_key(other),
    }
}

pub static ID3V23_REVERSE_MAP: Lazy<HashMap<&'static str, FrameKey>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for key in [
        FrameKey::Title,
        FrameKey::Artist,
        FrameKey::Album,
        FrameKey::Year,
        FrameKey::TrackNumber,
        FrameKey::Genre,
        FrameKey::AlbumArtist,
        FrameKey::ContentGroup,
        FrameKey::Composer,
        FrameKey::EncodedBy,
        FrameKey::UnsyncedLyrics,
        FrameKey::Length,
        FrameKey::Conductor,
        FrameKey::AttachedPicture,
        FrameKey::UserDefinedURL,
        FrameKey::Comments,
        FrameKey::Private,
        FrameKey::RelativeVolumeAdjustment,
        FrameKey::EncryptionMethod,
        FrameKey::GroupIdRegistration,
        FrameKey::GeneralObject,
        FrameKey::CommercialURL,
        FrameKey::CopyrightURL,
        FrameKey::AudioFileURL,
        FrameKey::ArtistURL,
        FrameKey::RadioStationURL,
        FrameKey::PaymentURL,
        FrameKey::BitmapImageURL,
        FrameKey::UserDefinedText,
        FrameKey::SynchronizedLyrics,
        FrameKey::TempoCodes,
        FrameKey::MusicCDIdentifier,
        FrameKey::EventTimingCodes,
        FrameKey::Sequence,
        FrameKey::PlayCount,
        FrameKey::AudioSeekPointIndex,
        FrameKey::MediaType,
        FrameKey::CommercialFrame,
        FrameKey::AudioEncryption,
        FrameKey::SignatureFrame,
        FrameKey::SoftwareEncoder,
        FrameKey::AudioEncodingMethod,
        FrameKey::RecommendedBufferSize,
        FrameKey::BeatsPerMinute,
        FrameKey::Language,
        FrameKey::FileType,
        FrameKey::Time,
        FrameKey::RecordingDate,
        FrameKey::ReleaseDate,
    ] {
        map.insert(id3v23_code(key), key);
    }
    map
});

pub static ID3V22_REVERSE_MAP: Lazy<HashMap<&'static str, FrameKey>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for key in [
        FrameKey::Title,
        FrameKey::Artist,
        FrameKey::Album,
        FrameKey::Year,
        FrameKey::TrackNumber,
        FrameKey::Genre,
        FrameKey::AlbumArtist,
        FrameKey::ContentGroup,
        FrameKey::Composer,
        FrameKey::EncodedBy,
        FrameKey::UnsyncedLyrics,
        FrameKey::Length,
        FrameKey::Conductor,
        FrameKey::AttachedPicture,
        FrameKey::UserDefinedURL,
        FrameKey::Comments,
        FrameKey::Private,
        FrameKey::RelativeVolumeAdjustment,
        FrameKey::EncryptionMethod,
        FrameKey::GroupIdRegistration,
        FrameKey::GeneralObject,
        FrameKey::CommercialURL,
        FrameKey::CopyrightURL,
        FrameKey::AudioFileURL,
        FrameKey::ArtistURL,
        FrameKey::RadioStationURL,
        FrameKey::PaymentURL,
        FrameKey::BitmapImageURL,
        FrameKey::UserDefinedText,
        FrameKey::SynchronizedLyrics,
        FrameKey::TempoCodes,
        FrameKey::MusicCDIdentifier,
        FrameKey::EventTimingCodes,
        FrameKey::Sequence,
        FrameKey::PlayCount,
        FrameKey::AudioSeekPointIndex,
        FrameKey::MediaType,
        FrameKey::CommercialFrame,
        FrameKey::AudioEncryption,
        FrameKey::SignatureFrame,
        FrameKey::SoftwareEncoder,
        FrameKey::AudioEncodingMethod,
        FrameKey::RecommendedBufferSize,
        FrameKey::BeatsPerMinute,
        FrameKey::Language,
        FrameKey::FileType,
        FrameKey::Time,
        FrameKey::RecordingDate,
        FrameKey::ReleaseDate,
    ] {
        map.insert(id3v22_code(key), key);
    }
    map
});

pub static ID3V24_REVERSE_MAP: Lazy<HashMap<&'static str, FrameKey>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for key in [
        FrameKey::Title,
        FrameKey::Artist,
        FrameKey::Album,
        FrameKey::Year,
        FrameKey::TrackNumber,
        FrameKey::Genre,
        FrameKey::AlbumArtist,
        FrameKey::ContentGroup,
        FrameKey::Composer,
        FrameKey::EncodedBy,
        FrameKey::UnsyncedLyrics,
        FrameKey::Length,
        FrameKey::Conductor,
        FrameKey::AttachedPicture,
        FrameKey::UserDefinedURL,
        FrameKey::Comments,
        FrameKey::Private,
        FrameKey::RelativeVolumeAdjustment,
        FrameKey::EncryptionMethod,
        FrameKey::GroupIdRegistration,
        FrameKey::GeneralObject,
        FrameKey::CommercialURL,
        FrameKey::CopyrightURL,
        FrameKey::AudioFileURL,
        FrameKey::ArtistURL,
        FrameKey::RadioStationURL,
        FrameKey::PaymentURL,
        FrameKey::BitmapImageURL,
        FrameKey::UserDefinedText,
        FrameKey::SynchronizedLyrics,
        FrameKey::TempoCodes,
        FrameKey::MusicCDIdentifier,
        FrameKey::EventTimingCodes,
        FrameKey::Sequence,
        FrameKey::PlayCount,
        FrameKey::AudioSeekPointIndex,
        FrameKey::MediaType,
        FrameKey::CommercialFrame,
        FrameKey::AudioEncryption,
        FrameKey::SignatureFrame,
        FrameKey::SoftwareEncoder,
        FrameKey::AudioEncodingMethod,
        FrameKey::RecommendedBufferSize,
        FrameKey::BeatsPerMinute,
        FrameKey::Language,
        FrameKey::FileType,
        FrameKey::Time,
        FrameKey::RecordingDate,
        FrameKey::ReleaseDate,
    ] {
        map.insert(id3v24_code(key), key);
    }
    map
});

pub fn raw_to_tags(raw: &HashMap<String, Vec<TagValue>>) -> HashMap<FrameKey, Vec<TagValue>> {
    let mut result: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
    for (id, values) in raw.iter() {
        if let Some(key) = ID3V23_REVERSE_MAP.get(id.as_str()) {
            let mut expanded: Vec<TagValue> = Vec::new();
            for v in values.iter() {
                match v {
                    TagValue::Text(s)
                        if matches!(key, FrameKey::Artist | FrameKey::Genre) && s.contains(';') =>
                    {
                        for part in s.split(';').map(|s| s.trim()) {
                            let seg = part.trim();
                            if !seg.is_empty() {
                                expanded.push(TagValue::Text(seg.to_string()));
                            }
                        }
                    }
                    _ => expanded.push(v.clone()),
                }
            }
            result.entry(*key).or_insert_with(Vec::new).extend(expanded);
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
