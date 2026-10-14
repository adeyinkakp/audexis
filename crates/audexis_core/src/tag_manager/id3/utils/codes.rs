use super::*;

pub fn id3v22_code(key: FrameKey) -> &'static str {
    match key {
        FrameKey::Title => "TT2",
        FrameKey::Artist => "TP1",
        FrameKey::Album => "TAL",
        FrameKey::Year => "TYE",
        FrameKey::TrackNumber => "TRK",
        FrameKey::DiscNumber => "TPA",
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
        FrameKey::DiscNumber => "TPOS",
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
        "TPA" => Some(FrameKey::DiscNumber),
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
        "TPOS" => Some(FrameKey::DiscNumber),
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
